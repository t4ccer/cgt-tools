use cgt::short::partizan::{games::domineering::Domineering, partizan_game::PartizanGame};
use pyo3::prelude::*;

#[pyclass(name = "Domineering")]
#[derive(Clone)]
pub struct PyDomineering {
    inner: Domineering,
}

impl From<Domineering> for PyDomineering {
    fn from(domineering: Domineering) -> Self {
        Self { inner: domineering }
    }
}

#[pymethods]
impl PyDomineering {
    #[new]
    fn py_new(position: &str) -> PyResult<Self> {
        Ok(Self::from(Domineering::parse(position).map_err(|err| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", err))
        })?))
    }

    fn __repr__(&self) -> String {
        format!("Domineering('{}')", self.inner)
    }

    fn to_svg(&self) -> String {
        self.inner.to_svg()
    }

    fn decompositions(&self) -> Vec<Self> {
        self.inner
            .decompositions()
            .into_iter()
            .map(Self::from)
            .collect()
    }
}
