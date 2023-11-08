use pyo3::prelude::*;

mod canonical_form;
mod domineering;
mod nimber;
mod rational;
mod ski_jumps;
mod thermograph;
mod toads_and_frogs;

use crate::{
    canonical_form::*, domineering::*, nimber::*, rational::*, ski_jumps::*, thermograph::*,
    toads_and_frogs::*,
};

// TODO: Pretty printers
// TODO: SVG rendering & html()

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
    add_class!(PyThermograph);
    add_class!(PySkiJumps);
    add_class!(PyToadsAndFrogs);

    Ok(())
}
