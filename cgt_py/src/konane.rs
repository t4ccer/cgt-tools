use crate::{grid::PyGrid, py_partizan_game};
use cgt::short::partizan::{
    games::konane::Konane, transposition_table::ParallelTranspositionTable,
};
use cgt_py_messages::{GridPreset, Tile};
use pyo3::{PyErr, PyResult, pyclass, pymethods};
use std::{str::FromStr, sync::LazyLock};

static TRANSPOSITION_TABLE: LazyLock<ParallelTranspositionTable<Konane>> =
    LazyLock::new(ParallelTranspositionTable::new);

#[pyclass(name = "Konane")]
pub struct PyKonane(pub Konane);

#[pymethods]
impl PyKonane {
    #[new]
    pub fn new(position: &str) -> PyResult<PyKonane> {
        let inner = Konane::from_str(position).map_err(|err| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Parse error: invalid Konane grid: {err}"
            ))
        })?;
        Ok(PyKonane(inner))
    }

    fn __repr__(&self) -> String {
        format!("Konane('{}')", self.0)
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
            GridPreset::Konane,
            self.0.grid().map(|tile| Tile::from(tile)),
        )
    }
}

py_partizan_game!(PyKonane);
