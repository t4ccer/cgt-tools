use crate::{dyadic_rational_number::PyDyadicRationalNumber, thermograph::PyThermograph};
use cgt::short::partizan::canonical_form::CanonicalForm;
use pyo3::{prelude::*, pyclass::CompareOp};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use std::{
    ops::{Add, Neg, Sub},
    str::FromStr,
};

#[gen_stub_pyclass]
#[pyclass(from_py_object, name = "CanonicalForm")]
#[derive(Clone)]
pub struct PyCanonicalForm(pub CanonicalForm);

#[gen_stub_pymethods]
#[pymethods]
impl PyCanonicalForm {
    #[new]
    fn new(value: Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(integer) = value.extract::<i64>() {
            return Ok(Self(CanonicalForm::new_integer(integer)));
        } else if let Ok(string) = value.extract::<&str>() {
            match CanonicalForm::from_str(string) {
                Ok(cf) => return Ok(Self(cf)),
                Err(_) => {
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                        "Could not parse CanonicalForm. Invalid input format.",
                    ));
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
        format!("CanonicalForm('{}')", self.0)
    }

    fn __add__(&self, other: &Self) -> Self {
        PyCanonicalForm(Add::add(&self.0, &other.0))
    }

    fn __sub__(&self, other: &Self) -> Self {
        PyCanonicalForm(Sub::sub(&self.0, &other.0))
    }

    fn __neg__(&self) -> Self {
        PyCanonicalForm(Neg::neg(&self.0))
    }

    fn __richcmp__(&self, other: &Self, op: CompareOp) -> bool {
        self.0
            .partial_cmp(&other.0)
            .is_some_and(|ord| op.matches(ord))
    }

    #[getter]
    fn temperature(&self) -> PyDyadicRationalNumber {
        PyDyadicRationalNumber(self.0.temperature())
    }

    #[getter]
    fn thermograph(&self) -> PyThermograph {
        PyThermograph(self.0.thermograph())
    }

    #[getter]
    fn reduced(&self) -> PyCanonicalForm {
        PyCanonicalForm(self.0.reduced())
    }

    #[getter]
    fn birthday(&self) -> u32 {
        self.0.birthday()
    }

    #[getter]
    fn left_options(&self) -> Vec<PyCanonicalForm> {
        self.0
            .left_moves()
            .map(|cf| PyCanonicalForm(cf.into_owned()))
            .collect()
    }

    #[getter]
    fn right_options(&self) -> Vec<PyCanonicalForm> {
        self.0
            .right_moves()
            .map(|cf| PyCanonicalForm(cf.into_owned()))
            .collect()
    }

    #[getter]
    fn left_stop(&self) -> PyDyadicRationalNumber {
        PyDyadicRationalNumber(self.0.left_stop())
    }

    #[getter]
    fn right_stop(&self) -> PyDyadicRationalNumber {
        PyDyadicRationalNumber(self.0.right_stop())
    }
}
