use crate::canonical_form::PyCanonicalForm;
use cgt::{
    drawing::svg::Svg,
    grid::small_bit_grid::SmallBitGrid,
    short::partizan::{
        games::domineering::Domineering, partizan_game::PartizanGame,
        transposition_table::ParallelTranspositionTable,
    },
};
use pyo3::prelude::*;
use std::str::FromStr;

crate::wrap_struct!(Domineering, PyDomineering, "Domineering", Clone);
crate::wrap_struct!(
    ParallelTranspositionTable<Domineering>,
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
        let mut buf = String::new();
        self.inner
            .to_svg(&mut buf)
            .expect("Write to String should not fail");
        buf
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
        PyDomineeringTranspositionTable::from(ParallelTranspositionTable::new())
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
