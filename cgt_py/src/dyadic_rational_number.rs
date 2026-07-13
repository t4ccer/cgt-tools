use cgt::numeric::dyadic_rational_number::DyadicRationalNumber;
use pyo3::{prelude::*, pyclass::CompareOp};
use std::{
    ops::{Add, Neg, Sub},
    str::FromStr,
};

#[pyclass(from_py_object, name = "DyadicRationalNumber")]
#[derive(Clone)]
pub struct PyDyadicRationalNumber(pub DyadicRationalNumber);

#[pymethods]
impl PyDyadicRationalNumber {
    #[new]
    #[pyo3(signature = (numerator, denominator_exponent = None))]
    fn new(
        numerator: Bound<'_, PyAny>,
        denominator_exponent: Option<u32>,
    ) -> PyResult<PyDyadicRationalNumber> {
        if let Ok(numerator) = numerator.extract::<i64>() {
            match denominator_exponent {
                None => Ok(PyDyadicRationalNumber(DyadicRationalNumber::from(
                    numerator,
                ))),
                Some(denominator_exponent) => Ok(PyDyadicRationalNumber(
                    DyadicRationalNumber::new(numerator, denominator_exponent),
                )),
            }
        } else if let Ok(string) = numerator.extract::<&str>() {
            DyadicRationalNumber::from_str(string)
                .map_err(|err| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Could not parse DyadicRationalNumber: {}",
                        err
                    ))
                })
                .map(PyDyadicRationalNumber)
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Could not convert to DyadicRationalNumber.",
            ))
        }
    }

    fn __repr__(&self) -> String {
        format!("DyadicRationalNumber('{}')", self.0)
    }

    fn __add__(&self, other: &PyDyadicRationalNumber) -> PyDyadicRationalNumber {
        PyDyadicRationalNumber(Add::add(&self.0, &other.0))
    }

    fn __sub__(&self, other: &PyDyadicRationalNumber) -> PyDyadicRationalNumber {
        PyDyadicRationalNumber(Sub::sub(&self.0, &other.0))
    }

    fn __neg__(&self) -> PyDyadicRationalNumber {
        PyDyadicRationalNumber(Neg::neg(&self.0))
    }

    fn __richcmp__(&self, other: &PyDyadicRationalNumber, op: CompareOp) -> bool {
        op.matches(self.0.cmp(&other.0))
    }

    fn __float__(&self) -> f32 {
        self.0
            .to_rational()
            .as_f32()
            .expect("DyadicRationalNumber must be finite")
    }
}
