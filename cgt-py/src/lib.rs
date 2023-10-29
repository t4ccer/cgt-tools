use cgt::{numeric::nimber::Nimber, short::partizan::canonical_form::CanonicalForm};
use pyo3::{prelude::*, pyclass::CompareOp};
use std::{
    ops::{Add, Neg, Sub},
    str::FromStr,
};

#[pyclass(name = "Nimber")]
#[derive(Clone)]
struct PyNimber {
    inner: Nimber,
}

impl From<Nimber> for PyNimber {
    fn from(nimber: Nimber) -> Self {
        Self { inner: nimber }
    }
}

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

#[pyclass(name = "CanonicalForm")]
#[derive(Clone)]
struct PyCanonicalForm {
    inner: CanonicalForm,
}

impl From<CanonicalForm> for PyCanonicalForm {
    fn from(canonical_form: CanonicalForm) -> Self {
        Self {
            inner: canonical_form,
        }
    }
}

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
    add_class!(PyNimber);

    Ok(())
}
