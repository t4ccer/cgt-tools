use cgt::numeric::nimber::Nimber;
use pyo3::{prelude::*, pyclass::CompareOp};
use std::ops::{Add, Neg, Sub};

crate::wrap_struct!(Nimber, PyNimber, "Nimber", Clone);

#[pymethods]
impl PyNimber {
    #[new]
    fn py_new(value: u32) -> Self {
        PyNimber::from(Nimber::new(value))
    }

    fn __repr__(&self) -> String {
        format!("Nimber({})", self.inner.value())
    }

    fn __add__(&self, other: &Self) -> Self {
        Self::from(Add::add(&self.inner, &other.inner))
    }

    fn __sub__(&self, other: &Self) -> Self {
        Self::from(Sub::sub(&self.inner, &other.inner))
    }

    fn __neg__(&self) -> Self {
        Self::from(Neg::neg(&self.inner))
    }

    fn __richcmp__(&self, other: &Self, op: CompareOp) -> bool {
        op.matches(self.inner.cmp(&other.inner))
    }
}

#[pyfunction]
pub fn mex(nimbers: Vec<PyNimber>) -> PyNimber {
    PyNimber::from(Nimber::mex(
        nimbers
            .into_iter()
            .map(|py_nimber| py_nimber.inner)
            .collect(),
    ))
}
