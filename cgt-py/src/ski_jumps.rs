use crate::canonical_form::PyCanonicalForm;
use cgt::short::partizan::{
    games::ski_jumps::SkiJumps, partizan_game::PartizanGame,
    transposition_table::TranspositionTable,
};
use pyo3::prelude::*;
use std::str::FromStr;

crate::wrap_struct!(SkiJumps, PySkiJumps, "SkiJumps", Clone);
crate::wrap_struct!(
    TranspositionTable<SkiJumps>,
    PySkiJumpsTranspositionTable,
    "SkiJumpsTranspositionTable"
);

#[pymethods]
impl PySkiJumps {
    #[new]
    fn py_new(position: &str) -> PyResult<Self> {
        let grid = SkiJumps::from_str(position)
            .or(Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Parse error",
            )))?;
        Ok(Self::from(grid))
    }

    fn __repr__(&self) -> String {
        format!("SkiJumps('{}')", self.inner)
    }

    fn to_svg(&self) -> String {
        self.inner.to_svg()
    }

    #[staticmethod]
    fn transposition_table() -> PySkiJumpsTranspositionTable {
        PySkiJumpsTranspositionTable::from(TranspositionTable::new())
    }

    fn canonical_form(
        &self,
        transposition_table: Option<&PySkiJumpsTranspositionTable>,
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
