use crate::canonical_form::PyCanonicalForm;
use cgt::{
    drawing::svg::Svg,
    short::partizan::{
        games::toads_and_frogs::ToadsAndFrogs, partizan_game::PartizanGame,
        transposition_table::TranspositionTable,
    },
};
use pyo3::prelude::*;
use std::str::FromStr;

crate::wrap_struct!(ToadsAndFrogs, PyToadsAndFrogs, "ToadsAndFrogs", Clone);
crate::wrap_struct!(
    TranspositionTable<ToadsAndFrogs>,
    PyToadsAndFrogsTranspositionTable,
    "ToadsAndFrogsTranspositionTable"
);

#[pymethods]
impl PyToadsAndFrogs {
    #[new]
    fn py_new(position: &str) -> PyResult<Self> {
        let grid = ToadsAndFrogs::from_str(position)
            .or(Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Parse error",
            )))?;
        Ok(Self::from(grid))
    }

    fn __repr__(&self) -> String {
        format!("ToadsAndFrogs('{}')", self.inner)
    }

    fn to_svg(&self) -> String {
        let mut buf = String::new();
        self.inner
            .to_svg(&mut buf)
            .expect("Write to String should not fail");
        buf
    }

    #[staticmethod]
    fn transposition_table() -> PyToadsAndFrogsTranspositionTable {
        PyToadsAndFrogsTranspositionTable::from(TranspositionTable::new())
    }

    fn canonical_form(
        &self,
        transposition_table: Option<&PyToadsAndFrogsTranspositionTable>,
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
