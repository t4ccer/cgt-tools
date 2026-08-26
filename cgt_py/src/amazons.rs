use crate::{grid::PyGrid, py_partizan_game};
use cgt::short::partizan::{
    games::amazons::Amazons, transposition_table::ParallelTranspositionTable,
};
use cgt_py_messages::{GridPreset, Tile};
use pyo3::{PyResult, pyclass, pymethods};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use std::{str::FromStr, sync::LazyLock};

static TRANSPOSITION_TABLE: LazyLock<ParallelTranspositionTable<Amazons>> =
    LazyLock::new(ParallelTranspositionTable::new);

#[gen_stub_pyclass]
#[pyclass(name = "Amazons")]
pub struct PyAmazons(pub Amazons);

#[gen_stub_pymethods]
#[pymethods]
impl PyAmazons {
    #[new]
    pub fn new(position: &str) -> PyResult<PyAmazons> {
        let inner = Amazons::from_str(position)
            .map_err(|err| crate::parsing::parse_error(&err, position))?;
        Ok(PyAmazons(inner))
    }

    fn __repr__(&self) -> String {
        format!("Amazons('{}')", self.0)
    }

    fn _repr_svg_(&self) -> String {
        crate::draw_svg(&self.0)
    }

    #[getter]
    pub fn grid(&self) -> PyGrid {
        PyGrid::from_preset_unchecked(
            GridPreset::Amazons,
            self.0.grid().map(|tile| Tile::from(tile)),
        )
    }
}

py_partizan_game!(PyAmazons);
