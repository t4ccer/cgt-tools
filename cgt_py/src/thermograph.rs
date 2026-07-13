use cgt::{
    drawing::{Draw, svg},
    short::partizan::thermograph::Thermograph,
};
use pyo3::prelude::*;

#[pyclass(name = "Thermograph")]
pub struct PyThermograph(pub Thermograph);

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
