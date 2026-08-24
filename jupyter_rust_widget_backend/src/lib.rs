use pyo3::types::PyDict;
use pyo3::{IntoPyObjectExt, prelude::*};
use serde::{Deserialize, Serialize};

pub struct Response<FrontendMessage> {
    pub message: Option<FrontendMessage>,
    pub run_on_update: bool,
}

impl<FrontendMessage> Response<FrontendMessage> {
    fn map<R>(self, f: impl FnOnce(FrontendMessage) -> R) -> Response<R> {
        Response {
            run_on_update: self.run_on_update,
            message: self.message.map(f),
        }
    }
}

pub trait RustWidget: Send + Sync + 'static {
    type BackendMessage: for<'de> Deserialize<'de> + Send + Sync + 'static;
    type FrontendMessage: Serialize + Send + Sync + 'static;

    fn esm(&self) -> String;

    fn handle_message(&mut self, event: Self::BackendMessage) -> Response<Self::FrontendMessage>;

    fn value<'py>(&mut self) -> impl IntoPyObject<'py>;

    fn into_widget<'py>(self, py: Python<'py>, module: &str) -> PyResult<Bound<'py, PyAny>>
    where
        Self: Sized,
    {
        let some_widget_instance = SomeWidget::new(self);
        let base_bound = Bound::new(py, some_widget_instance)?;
        let current_module = py.import(module)?;
        let rust_widget_class = current_module.getattr("RustWidget")?;
        let widget_instance = rust_widget_class.call1((base_bound,))?;
        Ok(widget_instance)
    }
}

enum WidgetArg {
    Message(String),
    Value,
}

enum WidgetRet {
    Message(Response<String>),
    Value(PyResult<Py<PyAny>>),
}

#[pyclass(subclass)]
struct SomeWidget {
    #[pyo3(get, set)]
    pub _esm: String,
    event_handler:
        Box<dyn for<'py> FnMut(Python<'py>, WidgetArg) -> WidgetRet + Send + Sync + 'static>,
}

impl SomeWidget {
    fn new<W>(mut widget: W) -> SomeWidget
    where
        W: RustWidget,
    {
        SomeWidget {
            _esm: widget.esm(),
            event_handler: Box::new(move |py, arg| match arg {
                WidgetArg::Message(raw_message) => {
                    let message =
                        serde_json::de::from_str::<W::BackendMessage>(&raw_message).unwrap();
                    WidgetRet::Message(
                        widget
                            .handle_message(message)
                            .map(|msg| serde_json::to_string(&msg).unwrap()),
                    )
                }
                WidgetArg::Value => WidgetRet::Value(widget.value().into_py_any(py)),
            }),
        }
    }
}

#[pymethods]
impl SomeWidget {
    fn _wasm_handle_custom_msg(
        &mut self,
        py: Python<'_>,
        widget: Bound<'_, PyAny>,
        // Assume we always get an object from wasm
        // https://anywidget.dev/en/jupyter-widgets-the-good-parts#data-types
        data_from_wasm: Bound<'_, PyDict>,
        _buffers: Bound<'_, PyAny>,
    ) -> PyResult<()> {
        let json_module = py.import("json")?;
        let raw_json: String = json_module
            .call_method1("dumps", (data_from_wasm,))?
            .extract()?;

        if let WidgetRet::Message(response) = (self.event_handler)(py, WidgetArg::Message(raw_json))
        {
            if response.run_on_update {
                let on_change = widget.getattr("_on_change_handler")?;
                if !on_change.is_none() {
                    let value = self._wasm_value(py)?;
                    on_change.call1((value,))?;
                }
            }

            if let Some(msg) = response.message {
                widget.call_method1("send", (msg,))?;
            }
        }

        Ok(())
    }

    fn _wasm_value(&mut self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        if let WidgetRet::Value(val) = (self.event_handler)(py, WidgetArg::Value) {
            val
        } else {
            unreachable!()
        }
    }
}

pub fn inject_rust_widget(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    let ctx = PyDict::new(py);
    ctx.set_item("anywidget", py.import("anywidget")?)?;

    let py_code = cr#"
class RustWidget(anywidget.AnyWidget):
    def on_change(self, f):
        self._on_change_handler = f
        if f is not None:
            f(self.value())

    def __init__(self, rust_handler, *args, **kwargs):
        self._esm = rust_handler._esm
        anywidget.AnyWidget.__init__(self, *args, **kwargs)
        self.on_msg(rust_handler._wasm_handle_custom_msg)
        self.value = rust_handler._wasm_value
        self._on_change_handler = None
"#;

    py.run(py_code, Some(&ctx), None)?;
    let widget_class = ctx.get_item("RustWidget")?.unwrap();
    m.add("RustWidget", widget_class)?;

    Ok(())
}
