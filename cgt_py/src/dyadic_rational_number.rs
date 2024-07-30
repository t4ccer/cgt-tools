use cgt::numeric::dyadic_rational_number::DyadicRationalNumber;
use pyo3::{prelude::*, pyclass::CompareOp};
use std::{
    ops::{Add, Neg, Sub},
    str::FromStr,
};

crate::wrap_struct!(
    DyadicRationalNumber,
    PyDyadicRationalNumber,
    "DyadicRationalNumber",
    Clone
);

#[pymethods]
impl PyDyadicRationalNumber {
    #[new]
    fn py_new(numerator: &PyAny, denominator: Option<u32>) -> PyResult<Self> {
        if let Ok(numerator) = numerator.extract::<i64>() {
            match denominator {
                None => Ok(Self::from(DyadicRationalNumber::from(numerator))),
                Some(denominator_exponent) => Ok(Self::from(DyadicRationalNumber::new(
                    numerator,
                    denominator_exponent,
                ))),
            }
        } else if let Ok(string) = numerator.extract::<&str>() {
            DyadicRationalNumber::from_str(string)
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
