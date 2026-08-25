use cgt::{
    drawing::{Draw, svg},
    short::partizan::thermograph::Thermograph,
};
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
        let bb = self.0.required_canvas::<svg::Canvas>();
        let mut canvas = svg::Canvas::new(bb);
        self.0.draw(&mut canvas);
        canvas.to_svg()
    }
}
