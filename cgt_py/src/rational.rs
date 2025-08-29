use cgt::numeric::rational::Rational;
use pyo3::{prelude::*, pyclass::CompareOp};
use std::{
    ops::{Add, Neg, Sub},
    str::FromStr,
};

crate::wrap_struct!(Rational, PyRational, "Rational", Clone);

#[pymethods]
impl PyRational {
    #[new]
    fn py_new(numerator: Py<PyAny>, denominator: Option<u32>) -> PyResult<Self> {
        Python::with_gil(|gil| {
            if let Ok(numerator) = numerator.extract::<i64>(gil) {
                match denominator {
                    None => Ok(Self::from(Rational::from(numerator))),
                    Some(denominator) => match Rational::new_fraction(numerator, denominator) {
                        Some(rational) => Ok(Self::from(rational)),
                        None => Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                            "Invalid Rational",
                        )),
                    },
                }
            } else if let Ok(string) = numerator.extract::<&str>(gil) {
                Rational::from_str(string)
                    .map_err(|err| {
                        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                            "Could not parse Rational: {}",
                            err
                        ))
                    })
                    .map(Self::from)
            } else {
                Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                    "Could not convert to Rational.",
                ))
            }
        })
    }

    fn __repr__(&self) -> String {
        format!("Rational('{}')", self.inner)
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
