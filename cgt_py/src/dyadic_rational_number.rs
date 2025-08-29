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
    #[pyo3(signature = (numerator, denominator_exponent = None))]
    fn py_new(numerator: Py<PyAny>, denominator_exponent: Option<u32>) -> PyResult<Self> {
        Python::with_gil(|gil| {
            if let Ok(numerator) = numerator.extract::<i64>(gil) {
                match denominator_exponent {
                    None => Ok(Self::from(DyadicRationalNumber::from(numerator))),
                    Some(denominator_exponent) => Ok(Self::from(DyadicRationalNumber::new(
                        numerator,
                        denominator_exponent,
                    ))),
                }
            } else if let Ok(string) = numerator.extract::<&str>(gil) {
                DyadicRationalNumber::from_str(string)
                    .map_err(|err| {
                        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                            "Could not parse DyadicRationalNumber: {}",
                            err
                        ))
                    })
                    .map(Self::from)
            } else {
                Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                    "Could not convert to DyadicRationalNumber.",
                ))
            }
        })
    }

    fn __repr__(&self) -> String {
        format!("DyadicRationalNumber('{}')", self.inner)
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
