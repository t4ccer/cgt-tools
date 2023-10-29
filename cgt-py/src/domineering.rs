use crate::canonical_form::PyCanonicalForm;
use cgt::short::partizan::{
    games::domineering::Domineering, partizan_game::PartizanGame,
    transposition_table::TranspositionTable,
};
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

#[pyclass(name = "DomineeringTranspositionTable")]
pub struct PyDomineeringTranspositionTable {
    inner: TranspositionTable<Domineering>,
}

impl From<TranspositionTable<Domineering>> for PyDomineeringTranspositionTable {
    fn from(transposition_table: TranspositionTable<Domineering>) -> Self {
        Self {
            inner: transposition_table,
        }
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

    #[staticmethod]
    fn transposition_table() -> PyDomineeringTranspositionTable {
        PyDomineeringTranspositionTable::from(TranspositionTable::new())
    }

    fn canonical_form(
        &self,
        transposition_table: Option<&PyDomineeringTranspositionTable>,
    ) -> PyCanonicalForm {
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
}
