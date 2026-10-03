//! An AI player in a web worker. It loads the model file at the address in the `model` parameter
//! of the worker's URL, and answers each [`Request`] of the page with Monte Carlo tree search on
//! the network of the model.

use burn::backend::{Flex, flex::FlexDevice};
use cgt_ai_core::{
    mcts::{Node, SearchConfig, run_mcts},
    openings::{OpeningTable, decide_swap},
    protocol::{Budget, Request, Response},
    quelhas::Quelhas,
    ruleset::Ruleset,
};
use cgt_ai_model::{ModelFile, NetEvaluator};
use rand::{SeedableRng, rngs::SmallRng};
use std::time::Duration;
// `std::time::Instant` panics in a browser
use web_time::Instant;

/// Simulations a search with a time limit runs between looks at the clock
const ROUND: u32 = 8;

/// A loaded model of some game.
pub trait Player {
    fn handle(&mut self, request: Request) -> Response;
}

struct Engine<R: Ruleset> {
    evaluator: NetEvaluator<R, Flex>,
    openings: Option<OpeningTable>,
    rng: SmallRng,
}

impl<R: Ruleset> Engine<R> {
    fn new(rules: R, file: ModelFile, seed: u64) -> Result<Engine<R>, String> {
        let openings = file.header.openings.clone();
        let device = FlexDevice;
        let net = file.load_net(&device)?;
        Ok(Engine {
            evaluator: NetEvaluator { rules, net, device },
            openings,
            rng: SmallRng::seed_from_u64(seed),
        })
    }

    const fn rules(&self) -> &R {
        &self.evaluator.rules
    }

    fn after(&self, history: &[usize]) -> Result<R::State, String> {
        history
            .iter()
            .try_fold(self.rules().initial_state(), |state, &action| {
                if self.rules().legal_actions(&state).contains(&action) {
                    Ok(self.rules().apply(&state, action))
                } else {
                    Err(format!("action {action} is not legal"))
                }
            })
    }

    fn opening(&mut self) -> Result<Response, String> {
        let openings = self
            .openings
            .as_ref()
            .ok_or("the model has no opening table")?;
        let (action, value) = openings
            .choose(&mut self.rng, 3)
            .ok_or("the opening table is empty")?;
        // The table holds values for the second player, and after the pie rule the first player
        // ends up on the losing side of them
        Ok(Response::Move {
            action,
            value: -value.abs(),
        })
    }

    /// The search tree of `state` after a search of `budget`
    fn search(&mut self, state: R::State, budget: Budget) -> Node<R> {
        let rules = self.rules().clone();
        let config = SearchConfig::default();
        let mut roots = [Node::new(&rules, state)];
        let mut run = |roots: &mut [Node<R>], simulations| {
            run_mcts(
                &rules,
                &mut self.evaluator,
                roots,
                simulations,
                &config,
                None,
            );
        };
        match budget {
            Budget::Simulations(simulations) => run(&mut roots, simulations),
            Budget::Millis(millis) => {
                let deadline = Instant::now() + Duration::from_millis(millis.into());
                let mut simulations = 0;
                while !roots[0].is_terminal() && (simulations == 0 || Instant::now() < deadline) {
                    simulations += ROUND;
                    run(&mut roots, simulations);
                }
            }
        }
        let [root] = roots;
        root
    }

    fn pie(&mut self, first: usize, budget: Budget) -> Result<Response, String> {
        let state = self.after(&[first])?;
        let value = self
            .openings
            .as_ref()
            .and_then(|o| o.value(first))
            .unwrap_or_else(|| self.search(state, budget).q());
        Ok(Response::Pie {
            swap: decide_swap(value),
            value,
        })
    }

    fn best_move(&mut self, history: &[usize], budget: Budget) -> Result<Response, String> {
        let root = self.search(self.after(history)?, budget);
        if root.is_terminal() {
            return Err("the game is over".to_owned());
        }
        Ok(Response::Move {
            action: root.best_action(),
            value: root.q(),
        })
    }
}

impl<R: Ruleset> Player for Engine<R> {
    fn handle(&mut self, request: Request) -> Response {
        match request {
            Request::Opening => self.opening(),
            Request::Pie { first, budget } => self.pie(first, budget),
            Request::Move { history, budget } => self.best_move(&history, budget),
        }
        .unwrap_or_else(Response::Error)
    }
}

/// Loads the model file `bytes`, with its moves randomized from `seed`.
///
/// # Errors
///
/// When `bytes` are not a model file of a known game.
pub fn load(bytes: &[u8], seed: u64) -> Result<Box<dyn Player>, String> {
    let file = ModelFile::from_bytes(bytes)?;
    match file.header.game.as_str() {
        game if game == Quelhas.name() => Ok(Box::new(Engine::new(Quelhas, file, seed)?)),
        game => Err(format!(
            "the model plays {game}, which this worker does not know"
        )),
    }
}

#[cfg(target_arch = "wasm32")]
// A worker has a single thread, so its futures never move between threads
#[allow(clippy::future_not_send)]
mod worker {
    use crate::{Player, load};
    use cgt_ai_core::protocol::{Envelope, Request, Response};
    use std::cell::RefCell;
    use wasm_bindgen::{JsCast, prelude::*};
    use wasm_bindgen_futures::{JsFuture, spawn_local};
    use web_sys::{DedicatedWorkerGlobalScope, MessageEvent, UrlSearchParams};

    thread_local! {
        static PLAYER: RefCell<Result<Box<dyn Player>, String>> =
            RefCell::new(Err("the AI is still loading".into()));
    }

    async fn fetch(scope: &DedicatedWorkerGlobalScope, url: &str) -> Result<Vec<u8>, String> {
        let fail = |err: JsValue| {
            let reason = err
                .dyn_ref::<js_sys::Error>()
                .map_or_else(|| format!("{err:?}"), |err| String::from(err.message()));
            format!("could not load {url}: {reason}")
        };
        let response: web_sys::Response = JsFuture::from(scope.fetch_with_str(url))
            .await
            .map_err(fail)?
            .unchecked_into();
        if !response.ok() {
            return Err(format!("could not load {url}: HTTP {}", response.status()));
        }
        let buffer = JsFuture::from(response.array_buffer().map_err(fail)?)
            .await
            .map_err(fail)?;
        Ok(js_sys::Uint8Array::new(&buffer).to_vec())
    }

    async fn start_player(scope: &DedicatedWorkerGlobalScope) -> Result<Box<dyn Player>, String> {
        let url = UrlSearchParams::new_with_str(&scope.location().search())
            .ok()
            .and_then(|params| params.get("model"))
            .ok_or("the worker was started without a model")?;
        let seed = (js_sys::Math::random() * f64::from(u32::MAX)) as u64;
        load(&fetch(scope, &url).await?, seed)
    }

    fn post(scope: &DedicatedWorkerGlobalScope, envelope: &Envelope<Response>) {
        let text = serde_json::to_string(envelope).expect("responses serialize to JSON");
        let _ = scope.post_message(&JsValue::from_str(&text));
    }

    #[wasm_bindgen(start)]
    pub fn start() {
        console_error_panic_hook::set_once();
        let scope: DedicatedWorkerGlobalScope = js_sys::global().unchecked_into();
        let reply = scope.clone();
        let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
            let text = event.data().as_string().unwrap_or_default();
            let envelope = match serde_json::from_str::<Envelope<Request>>(&text) {
                Ok(request) => Envelope {
                    id: request.id,
                    body: PLAYER.with_borrow_mut(|player| match player {
                        Ok(player) => player.handle(request.body),
                        Err(err) => Response::Error(err.clone()),
                    }),
                },
                Err(err) => Envelope {
                    id: 0,
                    body: Response::Error(format!("malformed request: {err}")),
                },
            };
            post(&reply, &envelope);
        });
        // A model that fails to load is reported as the reply to each request, which the page
        // shows next to the move it waits for
        spawn_local(async move {
            PLAYER.set(start_player(&scope).await);
            scope.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
            // The handler lives as long as the worker
            on_message.forget();
            post(
                &scope,
                &Envelope {
                    id: 0,
                    body: Response::Ready,
                },
            );
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use burn_store::{BurnpackStore, ModuleSnapshot};
    use cgt_ai_core::quelhas::{Action, State};
    use cgt_ai_model::{GridNetConfig, ModelHeader};

    fn model(openings: Option<OpeningTable>) -> Vec<u8> {
        let network = GridNetConfig::for_ruleset(&Quelhas, 4, 1);
        let mut store = BurnpackStore::from_bytes(None);
        network
            .init::<Flex>(&FlexDevice)
            .save_into(&mut store)
            .unwrap();
        ModelFile {
            header: ModelHeader {
                game: Quelhas.name().to_owned(),
                network,
                openings,
            },
            weights: store.get_bytes().unwrap().to_vec(),
        }
        .to_bytes()
    }

    fn table(values: &[(usize, f64)]) -> OpeningTable {
        OpeningTable {
            game: Quelhas.name().to_owned(),
            checkpoint: "test".to_owned(),
            simulations: 1,
            values: values.iter().copied().collect(),
        }
    }

    #[test]
    fn moves_are_legal() {
        let mut player = load(&model(None), 0).unwrap();
        let first = State::initial().legal_actions()[7].index();
        let state = Quelhas.apply(&Quelhas.initial_state(), first);
        for budget in [Budget::Simulations(16), Budget::Millis(20)] {
            let start = Instant::now();
            let response = player.handle(Request::Move {
                history: vec![first],
                budget,
            });
            let Response::Move { action, .. } = response else {
                panic!("{response:?}");
            };
            assert!(Quelhas.legal_actions(&state).contains(&action));
            if budget == Budget::Millis(20) {
                assert!(start.elapsed() >= Duration::from_millis(20));
            }
        }
    }

    #[test]
    fn illegal_history_is_an_error() {
        let mut player = load(&model(None), 0).unwrap();
        let first = Action::new(10, 0, 0).unwrap().index();
        let response = player.handle(Request::Move {
            history: vec![first, first],
            budget: Budget::Simulations(16),
        });
        assert!(matches!(response, Response::Error(_)), "{response:?}");
    }

    #[test]
    fn openings_come_from_the_table() {
        let mut player = load(&model(None), 0).unwrap();
        assert!(matches!(
            player.handle(Request::Opening),
            Response::Error(_)
        ));

        let (balanced, lopsided) = (
            Action::new(2, 0, 0).unwrap().index(),
            Action::new(3, 0, 0).unwrap().index(),
        );
        let mut player = load(&model(Some(table(&[(balanced, 0.1)]))), 0).unwrap();
        assert_eq!(
            player.handle(Request::Opening),
            Response::Move {
                action: balanced,
                value: -0.1
            }
        );
        let mut player = load(&model(Some(table(&[(lopsided, -0.9)]))), 0).unwrap();
        assert_eq!(
            player.handle(Request::Pie {
                first: lopsided,
                budget: Budget::Simulations(16)
            }),
            Response::Pie {
                swap: true,
                value: -0.9
            }
        );
    }

    #[test]
    fn other_files_are_rejected() {
        assert!(load(b"not a model", 0).is_err());
    }
}
