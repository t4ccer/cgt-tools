use cgt::{drawing::svg::Svg, short::partizan::thermograph::Thermograph};
use pyo3::prelude::*;

crate::wrap_struct!(Thermograph, PyThermograph, "Thermograph", Clone);

#[pymethods]
impl PyThermograph {
    fn __repr__(&self) -> String {
        format!("Thermograph({})", self.inner)
    }

    fn _repr_svg_(&self) -> String {
        let mut buf = String::new();
        self.inner
            .to_svg(&mut buf)
            .expect("Write to String should not fail");
        buf
    }
}
