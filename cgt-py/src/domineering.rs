use crate::canonical_form::PyCanonicalForm;
use cgt::{
    grid::small_bit_grid::SmallBitGrid,
    short::partizan::{
        games::domineering::Domineering, partizan_game::PartizanGame,
        transposition_table::TranspositionTable,
    },
};
use pyo3::prelude::*;
use std::str::FromStr;

crate::wrap_struct!(Domineering, PyDomineering, "Domineering", Clone);
crate::wrap_struct!(
    TranspositionTable<Domineering>,
    PyDomineeringTranspositionTable,
    "DomineeringTranspositionTable"
);

#[pymethods]
impl PyDomineering {
    #[new]
    fn py_new(position: &str) -> PyResult<Self> {
        let grid = SmallBitGrid::from_str(position)
            .or(Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Parse error",
            )))?;
        Ok(Self::from(Domineering::new(grid)))
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
