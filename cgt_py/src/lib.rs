use pyo3::prelude::*;

mod canonical_form;
mod dyadic_rational_number;
mod nimber;
mod rational;
mod short_partizan;
mod thermograph;

use crate::{
    canonical_form::*, dyadic_rational_number::*, nimber::*, rational::*, short_partizan::*,
    thermograph::*,
};

#[macro_export]
macro_rules! wrap_struct {
    ($struct:path, $py_struct:ident, $py_class:expr $(, $trait:tt)*) => {
        #[derive($($trait),*)]
        #[pyclass(name = $py_class)]
        #[repr(transparent)]
        pub struct $py_struct {
            inner: $struct,
        }

        impl From<$struct> for $py_struct {
            fn from(inner: $struct) -> Self {
                $py_struct { inner }
            }
        }
    };
}

#[macro_export]
macro_rules! impl_py_partizan_game {
    ($game_str:expr, $game:ident, $py_game:ident, $tt_str:expr, $tt:path, $py_tt:ident) => {
        crate::wrap_struct!($tt, $py_tt, $tt_str, Default);
        crate::wrap_struct!($game, $py_game, $game_str, Clone);

        #[pymethods]
        impl $py_game {
            #[new]
            fn py_new(position: &str) -> PyResult<Self> {
                let inner = $game::from_str(position)
                    .or(Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                        "Parse error",
                    )))?;
                Ok(Self::from(inner))
            }

            fn __repr__(&self) -> String {
                format!("{}('{}')", stringify!($game), self.inner)
            }

            fn _repr_svg_(&self) -> String {
                let mut buf = String::new();
                self.inner
                    .to_svg(&mut buf)
                    .expect("Write to String should not fail");
                buf
            }

            #[staticmethod]
            fn transposition_table() -> $py_tt {
                $py_tt::default()
            }

            fn canonical_form(&self, transposition_table: Option<&$py_tt>) -> PyCanonicalForm {
                match transposition_table {
                    Some(transposition_table) => {
                        PyCanonicalForm::from(self.inner.canonical_form(&transposition_table.inner))
                    }
                    None => PyCanonicalForm::from(
                        self.inner
                            .canonical_form(&Self::transposition_table().inner),
                    ),
                }
            }

            fn left_moves(&self) -> Vec<Self> {
                self.inner
                    .left_moves()
                    .into_iter()
                    .map(Self::from)
                    .collect()
            }

            fn right_moves(&self) -> Vec<Self> {
                self.inner
                    .right_moves()
                    .into_iter()
                    .map(Self::from)
                    .collect()
            }
        }
    };
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
    add_function!(mex);
    add_class!(PyDomineering);
    add_class!(PyDomineeringTranspositionTable);
    add_class!(PyRational);
    add_class!(PyDyadicRationalNumber);
    add_class!(PyThermograph);
    add_class!(PySkiJumps);
    add_class!(PyToadsAndFrogs);

    Ok(())
}
