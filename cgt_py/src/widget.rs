//! Kernel-side plumbing shared by the widgets: the anywidget subclass they are built on

use pyo3::{prelude::*, types::PyDict};

pub fn inject_traitlet_widget(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    let ctx = PyDict::new(py);
    ctx.set_item("anywidget", py.import("anywidget")?)?;
    ctx.set_item("traitlets", py.import("traitlets")?)?;

    let py_code = cr#"
class StateTraitlet(traitlets.Any):
    """Traitlet that reports every assignment, including one that compares equal to what it
    already holds. A frontend re-sends the position it is showing as soon as it has mounted,
    and only by skipping that comparison does a callback set up before the widget was
    displayed hear about the position rather than waiting for the first edit
    """

    def set(self, obj, value):
        # traitlets.TraitType.set without its `silent` check
        new_value = self._validate(obj, value)
        try:
            old_value = obj._trait_values[self.name]
        except KeyError:
            old_value = self.default_value

        obj._trait_values[self.name] = new_value
        obj._notify_trait(self.name, old_value, new_value)


class TraitletWidget(anywidget.AnyWidget):
    _esm = traitlets.Unicode().tag(sync=True)

    def __init__(self, esm, *args, **kwargs):
        anywidget.AnyWidget.__init__(self, *args, _esm=esm, **kwargs)
"#;

    py.run(py_code, Some(&ctx), None)?;
    m.add("StateTraitlet", ctx.get_item("StateTraitlet")?.unwrap())?;
    m.add("TraitletWidget", ctx.get_item("TraitletWidget")?.unwrap())?;

    Ok(())
}
