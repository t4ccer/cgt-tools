use crate::{grid::PyGrid, py_partizan_game};
use cgt::short::partizan::{
    games::fission::Fission, transposition_table::ParallelTranspositionTable,
};
use cgt_py_messages::{GridPreset, Tile};
use pyo3::{PyErr, PyResult, pyclass, pymethods};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use std::{str::FromStr, sync::LazyLock};

static TRANSPOSITION_TABLE: LazyLock<ParallelTranspositionTable<Fission>> =
    LazyLock::new(ParallelTranspositionTable::new);

#[gen_stub_pyclass]
#[pyclass(name = "Fission")]
pub struct PyFission(pub Fission);

#[gen_stub_pymethods]
#[pymethods]
impl PyFission {
    #[new]
    pub fn new(position: &str) -> PyResult<PyFission> {
        let inner = Fission::from_str(position).map_err(|err| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Parse error: invalid Fission grid: {err}"
            ))
        })?;
        Ok(PyFission(inner))
    }

    fn __repr__(&self) -> String {
        format!("Fission('{}')", self.0)
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
            GridPreset::Fission,
            self.0.grid().map(|tile| Tile::from(tile)),
        )
    }
}

py_partizan_game!(PyFission);
