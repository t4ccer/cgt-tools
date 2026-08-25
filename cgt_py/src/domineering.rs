use crate::{grid::PyGrid, py_partizan_game};
use cgt::{
    grid::FiniteGrid as _,
    result::UnwrapInfallible,
    short::partizan::{
        games::domineering::Domineering, transposition_table::ParallelTranspositionTable,
    },
};
use cgt_py_messages::{GridPreset, Tile};
use pyo3::{PyErr, PyResult, pyclass, pymethods};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use std::{str::FromStr, sync::LazyLock};

static TRANSPOSITION_TABLE: LazyLock<ParallelTranspositionTable<Domineering>> =
    LazyLock::new(ParallelTranspositionTable::new);

#[gen_stub_pyclass]
#[pyclass(name = "Domineering")]
pub struct PyDomineering(pub Domineering);

#[gen_stub_pymethods]
#[pymethods]
impl PyDomineering {
    #[new]
    pub fn new(position: &str) -> PyResult<PyDomineering> {
        let inner = Domineering::from_str(position).map_err(|err| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Parse error: invalid Domineering grid: {err}"
            ))
        })?;
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
            self.0.grid().map(Tile::from).unwrap_infallible(),
        )
    }
}

py_partizan_game!(PyDomineering);
