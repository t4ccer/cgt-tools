//! Browser-side plumbing shared by the widgets

use futures_signals::signal::{Mutable, SignalExt as _};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use wasm_bindgen::{
    JsValue,
    prelude::{Closure, wasm_bindgen},
};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "Object")]
    pub type AnyWidgetModel;

    #[wasm_bindgen(method, js_name = "on")]
    fn on(this: &AnyWidgetModel, event: &str, callback: &JsValue);

    #[wasm_bindgen(method, js_name = "get")]
    fn get(this: &AnyWidgetModel, key: &str) -> JsValue;

    #[wasm_bindgen(method, js_name = "set")]
    fn set(this: &AnyWidgetModel, key: &str, value: &JsValue);

    #[wasm_bindgen(method, js_name = "save_changes")]
    fn save_changes(this: &AnyWidgetModel);
}

fn resend_state(model: &Arc<AnyWidgetModel>, name: &str) {
    let Some(current) = model.get(name).as_string() else {
        return;
    };

    if current.is_empty() {
        return;
    }

    model.set(name, &JsValue::from_str(""));
    model.set(name, &JsValue::from_str(&current));
    model.save_changes();
}

pub fn bind_traitlet<T>(model: &Arc<AnyWidgetModel>, name: &str, state: &Mutable<T>)
where
    T: Serialize + for<'de> Deserialize<'de> + Clone + PartialEq + 'static,
{
    let read_traitlet = {
        let model = Arc::clone(model);
        let name = name.to_owned();
        let state = state.clone();
        move || {
            let Some(raw) = model.get(&name).as_string() else {
                return;
            };

            // A widget restored without the traitlet ever having been written holds the
            // empty string the traitlet defaults to
            if raw.is_empty() {
                return;
            }

            state.set_neq(serde_json::from_str::<T>(&raw).unwrap());
        }
    };

    // A widget in a saved notebook is never told anything by python, so the traitlet it was
    // restored with is the only place its state can come from. Reading it before subscribing
    // below also stops the first signal emission from pushing an empty state over it
    read_traitlet();

    let closure = Closure::<dyn FnMut()>::new(read_traitlet);
    model.on(&format!("change:{name}"), closure.as_ref());
    closure.forget();

    wasm_bindgen_futures::spawn_local(state.signal_cloned().for_each({
        let model = Arc::clone(model);
        let name = name.to_owned();
        move |value| {
            let json = serde_json::to_string(&value).unwrap();
            if model.get(&name).as_string().as_deref() != Some(json.as_str()) {
                model.set(&name, &JsValue::from_str(&json));
                model.save_changes();
            }

            async {}
        }
    }));

    resend_state(model, name);
}
