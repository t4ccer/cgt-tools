use cgt::{
    drawing::{Draw, svg},
    short::partizan::thermograph::Thermograph,
};
use pyo3::prelude::*;

crate::wrap_struct!(Thermograph, PyThermograph, "Thermograph", Clone);

#[pymethods]
impl PyThermograph {
    fn __repr__(&self) -> String {
        format!("Thermograph({})", self.inner)
    }

    fn _repr_svg_(&self) -> String {
        let bb = self.inner.required_canvas::<svg::Canvas>();
        let mut canvas = svg::Canvas::new(bb);
        self.inner.draw(&mut canvas);
        canvas.to_svg()
    }
}
