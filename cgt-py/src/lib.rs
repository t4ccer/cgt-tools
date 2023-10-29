use cgt::short::partizan::canonical_form::CanonicalForm;
use pyo3::prelude::*;
use std::{
    ops::{Add, Neg, Sub},
    str::FromStr,
};

#[pyclass(name = "CanonicalForm")]
#[derive(Clone)]
struct PyCanonicalForm(CanonicalForm);

impl From<CanonicalForm> for PyCanonicalForm {
    fn from(cf: CanonicalForm) -> Self {
        Self(cf)
    }
}

#[pymethods]
impl PyCanonicalForm {
    #[new]
    fn py_new(value: &PyAny) -> PyResult<Self> {
        if let Ok(integer) = value.extract::<i64>() {
            return Ok(Self(CanonicalForm::new_integer(integer)));
        } else if let Ok(string) = value.extract::<&str>() {
            match CanonicalForm::from_str(string) {
                Ok(cf) => return Ok(Self(cf)),
                Err(_) => {
                    return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
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
        format!("CanonicalForm({})", self.0)
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
}

#[pymodule]
fn cgt_py(_py: Python, m: &PyModule) -> PyResult<()> {
    macro_rules! add_class {
        ($class:ident) => {
            m.add_class::<$class>()?
        };
    }

    macro_rules! add_function {
        ($func:ident) => {
            m.add_function(wrap_pyfunction!($func, m)?)?;
        };
    }

    add_class!(PyCanonicalForm);

    Ok(())
}
