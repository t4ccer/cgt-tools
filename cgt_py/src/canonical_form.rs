use crate::{dyadic_rational_number::PyDyadicRationalNumber, thermograph::PyThermograph};
use cgt::short::partizan::canonical_form::CanonicalForm;
use pyo3::{prelude::*, pyclass::CompareOp};
use std::{
    ops::{Add, Neg, Sub},
    str::FromStr,
};

crate::wrap_struct!(CanonicalForm, PyCanonicalForm, "CanonicalForm", Clone);

#[pymethods]
impl PyCanonicalForm {
    #[new]
    fn py_new(value: &PyAny) -> PyResult<Self> {
        if let Ok(integer) = value.extract::<i64>() {
            return Ok(Self::from(CanonicalForm::new_integer(integer)));
        } else if let Ok(string) = value.extract::<&str>() {
            match CanonicalForm::from_str(string) {
                Ok(cf) => return Ok(Self::from(cf)),
                Err(_) => {
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                        "Could not parse CanonicalForm. Invalid input format.",
                    ))
                }
            }
        } else if let Ok(canonical_form) = value.extract::<PyCanonicalForm>() {
            return Ok(canonical_form);
        }

        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Could not convert to CanonicalForm. Expected integer or string.",
        ))
    }

    fn __repr__(&self) -> String {
        format!("CanonicalForm('{}')", self.inner)
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
        self.inner
            .partial_cmp(&other.inner)
            .map_or(false, |ord| op.matches(ord))
    }

    fn temperature(&self) -> PyDyadicRationalNumber {
        self.inner.temperature().into()
    }

    fn thermograph(&self) -> PyThermograph {
        PyThermograph::from(self.inner.thermograph())
    }
}
