use crate::{grid::PyGrid, py_partizan_game};
use cgt::short::partizan::{
    games::amazons::Amazons, transposition_table::ParallelTranspositionTable,
};
use cgt_py_messages::{GridPreset, Tile};
use pyo3::{PyErr, PyResult, pyclass, pymethods};
use std::{str::FromStr, sync::LazyLock};

static TRANSPOSITION_TABLE: LazyLock<ParallelTranspositionTable<Amazons>> =
    LazyLock::new(ParallelTranspositionTable::new);

#[pyclass(name = "Amazons")]
pub struct PyAmazons(pub Amazons);

#[pymethods]
impl PyAmazons {
    #[new]
    pub fn new(position: &str) -> PyResult<PyAmazons> {
        let inner = Amazons::from_str(position).map_err(|err| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Parse error: invalid Amazons grid: {err}"
            ))
        })?;
        Ok(PyAmazons(inner))
    }

    fn __repr__(&self) -> String {
        format!("Amazons('{}')", self.0)
    }

    fn _repr_svg_(&self) -> String {
        use cgt::drawing::{Draw, svg};
        let bounding_box = self.0.required_canvas::<svg::Canvas>();
        let mut canvas = svg::Canvas::new(bounding_box);
        self.0.draw(&mut canvas);
        canvas.to_svg()
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
