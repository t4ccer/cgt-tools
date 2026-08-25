use crate::{grid::PyGrid, py_partizan_game};
use cgt::{
    grid::FiniteGrid as _,
    short::partizan::{
        games::domineering::Domineering, transposition_table::ParallelTranspositionTable,
    },
};
use cgt_py_messages::{GridPreset, Tile};
use pyo3::{PyErr, PyResult, pyclass, pymethods};
use std::{str::FromStr, sync::LazyLock};

static TRANSPOSITION_TABLE: LazyLock<ParallelTranspositionTable<Domineering>> =
    LazyLock::new(ParallelTranspositionTable::new);

#[pyclass(name = "Domineering")]
pub struct PyDomineering(pub Domineering);

#[pymethods]
impl PyDomineering {
    #[new]
    pub fn new(position: &str) -> PyResult<PyDomineering> {
        let inner = Domineering::from_str(position).or(Err(PyErr::new::<
            pyo3::exceptions::PyValueError,
            _,
        >(
            "Parse error: invalid Domineering grid",
        )))?;
        Ok(PyDomineering(inner))
    }

    fn __repr__(&self) -> String {
        format!("Domineering('{}')", self.0)
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
            GridPreset::Domineering,
            self.0.grid().map(|tile| Tile::from(tile)).unwrap(),
        )
    }
}

py_partizan_game!(PyDomineering);
