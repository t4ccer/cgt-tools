use serde::{Deserialize, Serialize};
use std::{
    marker::PhantomData,
    sync::{Arc, Mutex},
};
use wasm_bindgen::{
    JsValue,
    prelude::{Closure, wasm_bindgen},
};
use web_sys::Element;

pub struct Context<M> {
    model: Arc<AnyWidgetModel>,
    _ty: PhantomData<M>,
}

impl<M> Clone for Context<M> {
    fn clone(&self) -> Self {
        Self {
            model: self.model.clone(),
            _ty: self._ty,
        }
    }
}

impl<M> Context<M>
where
    M: Serialize,
{
    fn new(model: Arc<AnyWidgetModel>) -> Self {
        Self {
            model,
            _ty: PhantomData,
        }
    }

    pub fn send_message(&self, message: &M) {
        let json = serde_json::to_string(message).unwrap();
        let message = web_sys::js_sys::JSON::parse(&json).unwrap();
        self.model.send(&message);
    }
}

pub trait WasmWidget: Send + Sync + 'static {
    type BackendMessage: Serialize + Send + Sync + 'static;
    type FrontendMessage: for<'de> Deserialize<'de> + Send + Sync + 'static;

    fn mount(
        &mut self,
        context: Context<Self::BackendMessage>,
        element: Element,
    ) -> Result<(), JsValue>;
    fn handle_message(&mut self, message: Self::FrontendMessage) -> Result<(), JsValue>;

    fn render(self, model: AnyWidgetModel, el: Element) -> Result<(), JsValue>
    where
        Self: Sized,
    {
        let model = Arc::new(model);
        let widget = Arc::new(Mutex::new(SomeWidget::new(self)));

        {
            let mut widget = widget.lock().unwrap();
            (widget.mount_fn)(Arc::clone(&model), el);
        }

        let closure = Closure::<dyn FnMut(String)>::new({
            let widget = Arc::clone(&widget);
            move |message_json: String| {
                let mut widget = widget.lock().unwrap();
                (widget.message_fn)(message_json);
            }
        });

        model.on("msg:custom", closure.as_ref());
        closure.forget();

        Ok(())
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "Object")]
    pub type AnyWidgetModel;

    #[wasm_bindgen(method, js_name = "on")]
    fn on(this: &AnyWidgetModel, event: &str, callback: &JsValue);

    #[wasm_bindgen(method, js_name = "send")]
    fn send(this: &AnyWidgetModel, payload: &JsValue);
}

struct SomeWidget {
    mount_fn: Box<dyn FnMut(Arc<AnyWidgetModel>, Element) + Send + Sync + 'static>,
    message_fn: Box<dyn FnMut(String) + Send + Sync + 'static>,
}

impl SomeWidget {
    fn new<W>(widget: W) -> Self
    where
        W: WasmWidget,
    {
        let widget = Arc::new(Mutex::new(widget));

        let mount_fn = Box::new({
            let widget = Arc::clone(&widget);
            move |model: Arc<AnyWidgetModel>, el: Element| {
                let context = Context::<W::BackendMessage>::new(model);
                let mut widget = widget.lock().unwrap();
                let _ = widget.mount(context, el);
            }
        });

        let message_fn = Box::new({
            let widget = Arc::clone(&widget);
            move |raw_json_in: String| {
                let message = serde_json::from_str::<W::FrontendMessage>(&raw_json_in).unwrap();
                let mut lock = widget.lock().unwrap();
                let _ = lock.handle_message(message);
            }
        });

        SomeWidget {
            mount_fn,
            message_fn,
        }
    }
}
