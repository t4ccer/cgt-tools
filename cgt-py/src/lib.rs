use pyo3::prelude::*;

mod canonical_form;
mod domineering;
mod nimber;
mod rational;

use crate::canonical_form::*;
use crate::domineering::*;
use crate::nimber::*;
use crate::rational::*;

// TODO: Pretty printers
// TODO: SVG rendering & html()

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

    Ok(())
}
