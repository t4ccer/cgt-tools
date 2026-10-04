//! The web worker that runs the AI, so that its search does not freeze the page

use super::game::{Game, Model, Msg};
use cgt_ai_core::protocol::{Envelope, Request, Response};
use leptos::prelude::*;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, prelude::*};
use web_sys::{ErrorEvent, Event, MessageEvent, Worker, WorkerOptions, WorkerType};

/// The worker script, which `make site` copies next to the islands
const WORKER: &str = "/pkg/worker.js";

struct Client {
    worker: Option<Worker>,
    /// Address of the model file the worker plays with
    ai: String,
    ready: bool,
    /// A request made while the worker was still loading
    queued: Option<String>,
    /// The request the worker is working on
    current: Option<u32>,
    on_message: Option<Closure<dyn FnMut(MessageEvent)>>,
    on_error: Option<Closure<dyn FnMut(Event)>>,
}

impl Client {
    /// Stops the worker, and with it every request it was given
    fn discard(&mut self) {
        if let Some(worker) = self.worker.take() {
            worker.terminate();
        }
        self.ready = false;
        self.queued = None;
        self.current = None;
    }

    fn restart(&mut self, ai: String) -> Result<(), JsValue> {
        self.discard();
        let options = WorkerOptions::new();
        options.set_type(WorkerType::Module);
        let model = js_sys::encode_uri_component(&ai);
        let worker = Worker::new_with_options(&format!("{WORKER}?model={model}"), &options)?;
        worker.set_onmessage(self.on_message.as_ref().map(|c| c.as_ref().unchecked_ref()));
        worker.set_onerror(self.on_error.as_ref().map(|c| c.as_ref().unchecked_ref()));
        self.worker = Some(worker);
        self.ai = ai;
        Ok(())
    }

    fn post(&mut self, text: String) {
        match (&self.worker, self.ready) {
            (Some(worker), true) => {
                let _ = worker.post_message(&JsValue::from_str(&text));
            }
            _ => self.queued = Some(text),
        }
    }
}

/// Starts the worker for the AI of `model`, and returns the function that applies messages to
/// `model` and passes the requests they lead to on to the worker
pub fn connect<G: Game>(model: RwSignal<Model<G>>) -> Callback<Msg> {
    let client = Rc::new(RefCell::new(Client {
        worker: None,
        ai: String::new(),
        ready: false,
        queued: None,
        current: None,
        on_message: None,
        on_error: None,
    }));
    let stored = StoredValue::new_local(client.clone());

    let dispatch = Callback::new(move |msg: Msg| {
        let new_game = matches!(msg, Msg::NewGame | Msg::Start);
        let request = model.try_update(|m| m.update(msg)).flatten();
        stored.with_value(|client| send(client, new_game, request, model));
    });

    let weak = Rc::downgrade(&client);
    let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
        let Some(client) = weak.upgrade() else { return };
        let text = event.data().as_string().unwrap_or_default();
        let envelope: Envelope<Response> = match serde_json::from_str(&text) {
            Ok(envelope) => envelope,
            Err(err) => {
                if let Some(id) = client.borrow_mut().current.take() {
                    dispatch.run(Msg::AiError {
                        id,
                        message: format!("malformed reply of the AI: {err}"),
                    });
                }
                return;
            }
        };
        let id = envelope.id;
        let msg = match envelope.body {
            Response::Ready => {
                let mut c = client.borrow_mut();
                c.ready = true;
                if let Some(text) = c.queued.take() {
                    c.post(text);
                }
                return;
            }
            Response::Move { action, value } => Msg::AiMove { id, action, value },
            Response::Pie { swap, value } => Msg::AiPie { id, swap, value },
            Response::Values(values) => Msg::AiValues { id, values },
            Response::Error(message) => Msg::AiError { id, message },
        };
        client.borrow_mut().current = None;
        dispatch.run(msg);
    });
    let weak = Rc::downgrade(&client);
    let on_error = Closure::<dyn FnMut(Event)>::new(move |event: Event| {
        let Some(client) = weak.upgrade() else { return };
        // A worker that raised an error may not answer anything any more, so the next request
        // starts a new one
        client.borrow_mut().discard();
        // A worker whose script could not be loaded raises a plain event, without a message
        let message = event
            .dyn_ref::<ErrorEvent>()
            .map(ErrorEvent::message)
            .filter(|message| !message.is_empty())
            .unwrap_or_else(|| "the AI could not be loaded".to_owned());
        dispatch.run(Msg::AiLost(message));
    });
    {
        let mut c = client.borrow_mut();
        c.on_message = Some(on_message);
        c.on_error = Some(on_error);
    }
    let ai = model.with_untracked(|m| m.setup.ai.clone());
    // Without a model the page never asks the AI anything
    if !ai.is_empty()
        && let Err(err) = client.borrow_mut().restart(ai)
    {
        leptos::logging::error!("could not start the AI worker: {err:?}");
    }
    dispatch
}

fn send<G: Game>(
    client: &Rc<RefCell<Client>>,
    new_game: bool,
    request: Option<(u32, Request)>,
    model: RwSignal<Model<G>>,
) {
    let mut c = client.borrow_mut();
    let ai = model.with_untracked(|m| m.setup.ai.clone());
    // The worker searches one request at a time, so an abandoned search would otherwise delay
    // the next game's first move of the AI
    let abandoned = c.current.is_some() && (new_game || request.is_some());
    // Only a request starts a worker again after an error, or one that fails to load would be
    // started again and again
    let lost = c.worker.is_none() && request.is_some();
    if (ai != c.ai || abandoned || lost)
        && let Err(err) = c.restart(ai)
    {
        drop(c);
        model.update(|m| {
            m.update(Msg::AiLost(format!(
                "could not start the AI worker: {err:?}"
            )));
        });
        return;
    }
    if let Some((id, request)) = request {
        c.current = Some(id);
        let text = serde_json::to_string(&Envelope { id, body: request })
            .expect("requests serialize to JSON");
        c.post(text);
    }
}
