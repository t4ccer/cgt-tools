use cgt::short::partizan::thermograph::Thermograph;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

#[gen_stub_pyclass]
#[pyclass(name = "Thermograph")]
pub struct PyThermograph(pub Thermograph);

#[gen_stub_pymethods]
#[pymethods]
impl PyThermograph {
    fn __repr__(&self) -> String {
        format!("Thermograph({})", self.0)
    }

    fn _repr_svg_(&self) -> String {
        crate::draw_svg(&self.0)
    }
}
