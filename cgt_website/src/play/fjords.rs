//! Fjords: claiming vertices of a board dealt from a seed

use super::{
    game::{EVEN, Game, MAX_BOARDS, Model, Msg},
    view::game_view,
};
use cgt_ai_core::{
    fjords::{BOARD_SIZE, Fjords, NUM_VERTICES, State, coordinates, edge, num_edges},
    ruleset::Player,
};
use leptos::prelude::*;

impl Game for Fjords {
    const SIDES: [&'static str; 2] = ["Blue", "Red"];
    const PIE_RULE: bool = false;
    const LAST_MOVE_LOSES: bool = false;
    const SETTLES: bool = true;
    const DEALT: bool = true;

    fn deal(&self, seed: u64) -> State {
        State::deal(seed, EDGE_PROBABILITY)
    }

    fn moves_available(&self, state: &State, side: Player) -> usize {
        state.reachable(side).count_ones() as usize
    }

    fn settled(&self, state: &State) -> Option<(Player, usize)> {
        state
            .settled()
            .map(|settled| (settled.winner, settled.margin))
    }
}

const DEFAULT_SEED: u64 = 1337;

/// Probability of an edge on the boards of the page, which the first page for Fjords used too.
/// The network trains on denser boards.
const EDGE_PROBABILITY: f64 = 0.75;

#[cfg(feature = "hydrate")]
fn seed_from_address() -> Option<u64> {
    let search = window().location().search().ok()?;
    web_sys::UrlSearchParams::new_with_str(&search)
        .ok()?
        .get("seed")?
        .parse()
        .ok()
}

/// Keeps the seed in the address of the page, so that a board can be shared or come back to
#[cfg(feature = "hydrate")]
fn put_seed_in_address(seed: u64) {
    let window = window();
    let Ok(url) = window
        .location()
        .href()
        .and_then(|href| web_sys::Url::new(&href))
    else {
        return;
    };
    url.search_params().set("seed", &seed.to_string());
    if let Ok(history) = window.history() {
        let _ = history.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&url.href()));
    }
}

/// A seed to deal boards from, different on each click
#[cfg(feature = "hydrate")]
fn random_seed() -> u64 {
    (js_sys::Math::random() * 2_147_483_648.0) as u64
}

#[cfg(not(feature = "hydrate"))]
const fn random_seed() -> u64 {
    DEFAULT_SEED
}

fn seed_step(
    model: RwSignal<Model<Fjords>>,
    dispatch: Callback<Msg>,
    from_address: StoredValue<bool>,
) -> AnyView {
    let seed = Memo::new(move |_| model.with(|m| m.setup.seed));
    let tried = Memo::new(move |_| model.with(|m| m.balancing.as_ref().map(|b| b.tried)));
    #[cfg(feature = "hydrate")]
    {
        // Only once the page is hydrated, which needs the board it was rendered with
        Effect::new(move |_| {
            if !from_address.get_value() {
                from_address.set_value(true);
                if let Some(value) = seed_from_address() {
                    dispatch.run(Msg::ChooseSeed(value));
                }
            }
        });
        Effect::new(move |previous: Option<u64>| {
            let value = seed.get();
            if previous.is_some_and(|previous| previous != value) {
                put_seed_in_address(value);
            }
            value
        });
    }
    #[cfg(not(feature = "hydrate"))]
    let _ = from_address;
    let finding = move || {
        tried.get().map(|tried| {
            view! {
                <p class="play-detail">
                    {format!("Looking for an even board, {tried} dealt so far…")}
                </p>
            }
        })
    };
    let error = move || {
        model.with(|m| {
            m.balancing_error.clone().map(|err| {
                view! { <p class="play-detail play-error">{format!("No even board: {err}")}</p> }
            })
        })
    };
    view! {
        <fieldset>
            <legend>"Board"</legend>
            <div class="play-field">
                <span>"Seed"</span>
                <input
                    type="number"
                    min="0"
                    prop:value=move || seed.get().to_string()
                    on:change=move |ev| {
                        if let Ok(value) = event_target_value(&ev).trim().parse() {
                            dispatch.run(Msg::ChooseSeed(value));
                        }
                    }
                />
            </div>
            <div class="play-buttons">
                <button
                    type="button"
                    class="btn btn-outline btn-sm"
                    on:click=move |_| dispatch.run(Msg::ChooseSeed(random_seed()))
                >
                    "Random"
                </button>
                {move || {
                    if tried.get().is_some() {
                        view! {
                            <button
                                type="button"
                                class="btn btn-outline btn-sm"
                                on:click=move |_| dispatch.run(Msg::StopFinding)
                            >
                                "Stop"
                            </button>
                        }
                            .into_any()
                    } else {
                        view! {
                            <button
                                type="button"
                                class="btn btn-outline btn-sm"
                                title=format!(
                                    "Deals random boards until the AI gives each side between {}% and {}% to win, for at most {MAX_BOARDS} boards",
                                    (1.0 - EVEN) * 50.0,
                                    (1.0 + EVEN) * 50.0,
                                )
                                on:click=move |_| dispatch.run(Msg::FindEven(random_seed()))
                            >
                                "Even board"
                            </button>
                        }
                            .into_any()
                    }
                }}
            </div>
            {finding}
            {error}
        </fieldset>
    }
    .into_any()
}

const SPACING: f64 = 50.0;
const RADIUS: f64 = 14.0;
const MARGIN: f64 = 36.0;

/// Centre of `vertex` on the board, odd rows shifted half a vertex to the right
fn position(vertex: usize) -> (f64, f64) {
    let (q, r) = coordinates(vertex);
    let shift = if r % 2 == 1 { SPACING / 2.0 } else { 0.0 };
    (
        (q as f64).mul_add(SPACING, MARGIN + shift),
        (r as f64).mul_add(SPACING * 3f64.sqrt() / 2.0, MARGIN),
    )
}

fn board(model: RwSignal<Model<Fjords>>, dispatch: Callback<Msg>) -> AnyView {
    let edges = move || {
        model.with(|m| {
            (0..num_edges())
                .filter(|&e| m.state.has_edge(e))
                .map(|e| {
                    let (a, b) = edge(e);
                    let ((x1, y1), (x2, y2)) = (position(a), position(b));
                    view! { <line class="fjords-edge" x1=x1 y1=y1 x2=x2 y2=y2></line> }
                })
                .collect_view()
        })
    };
    let vertices = (0..NUM_VERTICES)
        .map(|v| {
            let (x, y) = position(v);
            let has = move |bits: u64| bits >> v & 1 == 1;
            let classes = move || {
                model.with(|m| {
                    let s = &m.state;
                    let (left, right) = (s.reachable(Player::Left), s.reachable(Player::Right));
                    let mut classes = vec!["vertex"];
                    if has(s.left) {
                        classes.push("stone-left");
                    } else if has(s.right) {
                        classes.push("stone-right");
                    } else if has(left) && has(right) {
                        classes.push("reach-both");
                    } else if has(left) {
                        classes.push("reach-left");
                    } else if has(right) {
                        classes.push("reach-right");
                    }
                    if m.moves.last().is_some_and(|last| last.action == v) {
                        classes.push("last");
                    }
                    if m.human_to_move() && m.is_legal(v) {
                        classes.push("active");
                    }
                    classes.join(" ")
                })
            };
            view! {
                <circle
                    class=classes
                    cx=x
                    cy=y
                    r=RADIUS
                    on:click=move |_| dispatch.run(Msg::Play(v))
                ></circle>
            }
        })
        .collect_view();
    let labels = (0..BOARD_SIZE)
        .map(|i| {
            let label = |x: f64, y: f64, text: String| {
                view! {
                    <text class="board-label" x=x y=y>
                        {text}
                    </text>
                }
            };
            let (column, _) = position(i);
            let (_, row) = position(i * BOARD_SIZE);
            view! {
                {label(column, MARGIN / 2.0 - 4.0, char::from(b'a' + i as u8).to_string())}
                {label(MARGIN / 2.0 - 8.0, row, (i + 1).to_string())}
            }
        })
        .collect_view();

    let (right, bottom) = position(NUM_VERTICES - 1);
    let (width, height) = (right + RADIUS + 6.0, bottom + RADIUS + 6.0);
    view! {
        <svg class="board" viewBox=format!("0 0 {width} {height}") role="img" aria-label="Fjords board">
            {labels}
            {edges}
            {vertices}
        </svg>
    }
    .into_any()
}

/// A game of Fjords against one of the AIs in `models`, given by their names and the addresses
/// of their model files, the first being the default
#[island]
pub fn FjordsGame(models: Vec<(String, String)>) -> impl IntoView {
    let from_address = StoredValue::new(false);
    game_view::<Fjords>(models, DEFAULT_SEED, board, move |model, dispatch| {
        seed_step(model, dispatch, from_address)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::play::game::{Opponent, Phase, Setup};
    use cgt_ai_core::{protocol::Request, ruleset::Ruleset};

    #[test]
    fn settled_games_end_early() {
        let mut model = Model::<Fjords>::new(Setup::new("ai".into(), DEFAULT_SEED));
        model.update(Msg::ChooseOpponent(Opponent::Human));
        model.update(Msg::Start);
        while model.phase == Phase::Playing {
            let action = model.state.legal_actions()[0];
            model.update(Msg::Play(action));
        }
        let Phase::Over { winner, margin } = model.phase.clone() else {
            panic!("{:?}", model.phase);
        };
        let settled = model.state.settled();
        if let Some(margin) = margin {
            assert_eq!(
                settled.map(|s| (s.winner, s.margin)),
                Some((winner, margin))
            );
            assert_eq!(model.state.winner(), None);
        } else {
            assert_eq!(model.state.winner(), Some(winner));
        }
        assert!(model.move_chance(model.moves.len() - 1).is_some());
    }

    #[test]
    fn games_can_be_played_out() {
        let mut model = Model::<Fjords>::new(Setup::new("ai".into(), DEFAULT_SEED));
        model.update(Msg::ChooseOpponent(Opponent::Human));
        model.update(Msg::ChooseEndSettled(false));
        model.update(Msg::Start);
        while model.phase == Phase::Playing {
            let action = model.state.legal_actions()[0];
            model.update(Msg::Play(action));
        }
        assert!(matches!(model.phase, Phase::Over { margin: None, .. }));
        assert!(model.state.winner().is_some());
    }

    fn evaluated(request: Option<(u32, Request)>) -> (u32, usize) {
        match request {
            Some((id, Request::Evaluate { positions })) => (id, positions.len()),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn even_boards_are_screened_then_confirmed() {
        let mut model = Model::<Fjords>::new(Setup::new("ai".into(), DEFAULT_SEED));
        let (id, boards) = evaluated(model.update(Msg::FindEven(100)));
        // Only the board of seed 102 looks even to the network
        let mut values = vec![0.9; boards];
        values[2] = -0.05;
        let Some((confirm, Request::Move { position, .. })) =
            model.update(Msg::AiValues { id, values })
        else {
            panic!("no search to confirm the even board");
        };
        let mut expected = Vec::new();
        Fjords.write_state(&Fjords.deal(102), &mut expected);
        assert_eq!(position, expected);

        // The search finds it uneven after all, so more boards are dealt, from 108 on
        let (id, _) = evaluated(model.update(Msg::AiMove {
            id: confirm,
            action: 0,
            value: 0.5,
        }));
        let mut values = vec![0.9; boards];
        values[0] = 0.02;
        let (confirm, _) = model.update(Msg::AiValues { id, values }).unwrap();
        assert_eq!(model.setup.seed, DEFAULT_SEED);
        assert_eq!(
            model.update(Msg::AiMove {
                id: confirm,
                action: 0,
                value: -0.08,
            }),
            None
        );
        assert_eq!(model.setup.seed, 108);
        assert_eq!(model.state, Fjords.deal(108));
        assert!(model.balancing.is_none());
    }

    #[test]
    fn searches_for_even_boards_stop() {
        let mut model = Model::<Fjords>::new(Setup::new("ai".into(), DEFAULT_SEED));
        let (id, _) = evaluated(model.update(Msg::FindEven(0)));
        model.update(Msg::StopFinding);
        assert_eq!(
            model.update(Msg::AiValues {
                id,
                values: vec![0.0; 8]
            }),
            None
        );
        assert_eq!(model.setup.seed, DEFAULT_SEED);

        // Without an even board among the most boards it deals, the search gives up
        let mut request = model.update(Msg::FindEven(0));
        while let Some((id, Request::Evaluate { positions })) = request {
            request = model.update(Msg::AiValues {
                id,
                values: vec![1.0; positions.len()],
            });
        }
        assert!(model.balancing.is_none());
        assert!(model.balancing_error.is_some());
        assert_eq!(model.setup.seed, DEFAULT_SEED);
    }

    #[test]
    fn notation_names_vertices() {
        let model = Model::<Fjords>::new(Setup::new("ai".into(), DEFAULT_SEED));
        assert_eq!(model.notation(cgt_ai_core::fjords::vertex(2, 4)), "B c5");
    }
}
