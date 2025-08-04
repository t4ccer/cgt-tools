use cgt::short::partizan::thermograph::Thermograph;
use pyo3::prelude::*;

crate::wrap_struct!(Thermograph, PyThermograph, "Thermograph", Clone);

#[pymethods]
impl PyThermograph {
    fn __repr__(&self) -> String {
        format!("Thermograph({})", self.inner)
    }
}
