use cgt::{
    display_error::DisplayError,
    grid::{CharTile, FiniteGrid, vec_grid::VecGrid},
    result::UnwrapInfallible,
    short::partizan::games::{
        amazons::{self, Amazons},
        domineering,
        fission::{self, Fission},
        konane::{self, Konane},
    },
};
use cgt_py_messages::{GridPreset, Tile};
use pyo3::{
    Bound, IntoPyObjectExt, Py, PyAny, PyResult, Python, exceptions::PyValueError, prelude::*,
    pyclass, pyfunction, pymethods, types::PyDict,
};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use crate::{amazons::PyAmazons, domineering::PyDomineering, fission::PyFission, konane::PyKonane};

#[gen_stub_pyclass]
#[pyclass(name = "Grid", eq)]
#[derive(PartialEq)]
pub struct PyGrid {
    // If `Some` then grid grid has tiles that can be represented in this game
    pub known_preset: Option<GridPreset>,
    pub grid: VecGrid<Tile>,
}

impl PyGrid {
    pub fn from_preset_unchecked(preset: GridPreset, grid: VecGrid<Tile>) -> PyGrid {
        PyGrid {
            known_preset: Some(preset),
            grid,
        }
    }

    pub fn from_preset(preset: GridPreset, grid: VecGrid<Tile>) -> PyResult<PyGrid> {
        let grid = Self::from_preset_unchecked(preset, grid);
        grid.is_valid_for(preset)?;
        Ok(grid)
    }

    fn try_into_grid<T>(&self) -> PyResult<String>
    where
        T: Copy + CharTile + TryFrom<Tile>,
        T::Error: std::error::Error,
    {
        self.grid
            .try_map(|t| T::try_from(*t))
            .map(|grid| std::fmt::from_fn(|f| grid.display(f, '|')).to_string())
            .map_err(|err| PyValueError::new_err(err.display_error().to_string()))
    }

    fn is_valid_for(&self, preset: GridPreset) -> PyResult<()> {
        match preset {
            GridPreset::Domineering => self.domineering().map(drop),
            GridPreset::Fission => self.fission().map(drop),
            GridPreset::Amazons => self.amazons().map(drop),
            GridPreset::Konane => self.konane().map(drop),
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyGrid {
    #[getter]
    pub fn game(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        match self.known_preset {
            Some(preset) => match preset {
                GridPreset::Domineering => self.domineering()?.into_py_any(py),
                GridPreset::Fission => self.fission()?.into_py_any(py),
                GridPreset::Amazons => self.amazons()?.into_py_any(py),
                GridPreset::Konane => self.konane()?.into_py_any(py),
            },
            None => Err(PyValueError::new_err(
                "This grid is not associated with any game",
            )),
        }
    }

    #[getter]
    pub fn domineering(&self) -> PyResult<PyDomineering> {
        // This is hacky to to handle fail when converting large grid to SmallBitGrid
        // until we will have small grid optimization for VecGrid
        PyDomineering::new(&self.try_into_grid::<domineering::Tile>()?)
    }

    #[getter]
    pub fn fission(&self) -> PyResult<PyFission> {
        Ok(PyFission(Fission::new(
            self.grid
                .try_map(|t| fission::Tile::try_from(*t))
                .map_err(|err| PyValueError::new_err(err.display_error().to_string()))?,
        )))
    }

    #[getter]
    pub fn amazons(&self) -> PyResult<PyAmazons> {
        Ok(PyAmazons(Amazons::new(
            self.grid
                .try_map(|t| amazons::Tile::try_from(*t))
                .map_err(|err| PyValueError::new_err(err.display_error().to_string()))?,
        )))
    }

    #[getter]
    pub fn konane(&self) -> PyResult<PyKonane> {
        Ok(PyKonane(Konane::new(
            self.grid
                .try_map(|t| konane::Tile::try_from(*t))
                .map_err(|err| PyValueError::new_err(err.display_error().to_string()))?,
        )))
    }

    #[staticmethod]
    pub fn decode_from_traitlet(preset_bits: u32, raw_grid: &str) -> PyResult<PyGrid> {
        let preset = GridPreset::from_flag_bits(preset_bits)
            .ok_or_else(|| PyValueError::new_err(format!("unknown grid preset: {preset_bits}")))?;
        let grid = serde_json::from_str::<VecGrid<Tile>>(raw_grid)
            .map_err(|err| PyValueError::new_err(err.to_string()))?;

        Ok(PyGrid::from_preset_unchecked(preset, grid))
    }

    pub fn encode_to_traitlet(&self) -> String {
        serde_json::to_string(&self.grid).unwrap()
    }
}

fn grid_esm(preset: GridPreset) -> String {
    let bundle = include_str!("../widget/bundle.js");
    let preset = format!("const preset = {};", preset.into_flag_bits());
    let epilogue = r#" async function render({model, el}) {
                           await JupyterCGT.render_grid(model, el, preset);
                       }
                       export default { render }"#;
    let mut esm = String::with_capacity(bundle.len() + preset.len() + epilogue.len());
    esm.push_str(bundle);
    esm.push_str(&preset);
    esm.push_str(epilogue);
    esm
}

pub fn inject_grid_widget(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    let ctx = PyDict::new(py);
    ctx.set_item("TraitletWidget", m.getattr("TraitletWidget")?)?;
    ctx.set_item("traitlets", py.import("traitlets")?)?;
    ctx.set_item("Grid", py.get_type::<PyGrid>())?;
    ctx.set_item("StateTraitlet", m.getattr("StateTraitlet")?)?;

    let py_code = cr#"
class GridWidget(TraitletWidget):
    grid = StateTraitlet().tag(
        sync=True,
        to_json=lambda grid, widget: grid.encode_to_traitlet(),
        from_json=lambda raw_grid, widget: Grid.decode_from_traitlet(widget._preset, raw_grid),
    )

    def __init__(self, esm, preset, grid):
        self._preset = preset
        TraitletWidget.__init__(self, esm, grid=grid)
"#;

    py.run(py_code, Some(&ctx), None)?;
    m.add("GridWidget", ctx.get_item("GridWidget")?.unwrap())?;

    Ok(())
}

fn default_grid(preset: GridPreset) -> VecGrid<Tile> {
    let (width, height, tile) = match preset {
        GridPreset::Domineering => (8, 8, Tile::Taken),
        GridPreset::Fission => (4, 4, Tile::Empty),
        GridPreset::Amazons => (4, 4, Tile::Empty),
        GridPreset::Konane => (5, 5, Tile::Empty),
    };
    FiniteGrid::filled(width, height, tile).unwrap_infallible()
}

fn grid_from_position(preset: GridPreset, position: &Bound<'_, PyAny>) -> PyResult<PyGrid> {
    if let Ok(grid) = position.cast::<PyGrid>() {
        return PyGrid::from_preset(preset, grid.borrow().grid.clone());
    }

    let grid = match preset {
        GridPreset::Domineering => position.cast::<PyDomineering>()?.borrow().grid(),
        GridPreset::Fission => position.cast::<PyFission>()?.borrow().grid(),
        GridPreset::Amazons => position.cast::<PyAmazons>()?.borrow().grid(),
        GridPreset::Konane => position.cast::<PyKonane>()?.borrow().grid(),
    };

    Ok(grid)
}

fn make_grid_widget<'py>(
    py: Python<'py>,
    preset: GridPreset,
    position: Option<&Bound<'_, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    let grid = match position {
        None => default_grid(preset),
        Some(position) => grid_from_position(preset, position)?.grid,
    };

    py.import("cgt_py")?.getattr("GridWidget")?.call1((
        grid_esm(preset),
        preset.into_flag_bits(),
        PyGrid::from_preset_unchecked(preset, grid),
    ))
}

#[gen_stub_pyfunction]
#[pyfunction(name = "DomineeringWidget")]
#[pyo3(signature = (position = None))]
pub fn make_domineering_widget<'py>(
    py: Python<'py>,
    position: Option<&Bound<'_, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    make_grid_widget(py, GridPreset::Domineering, position)
}

#[gen_stub_pyfunction]
#[pyfunction(name = "FissionWidget")]
#[pyo3(signature = (position = None))]
pub fn make_fission_widget<'py>(
    py: Python<'py>,
    position: Option<&Bound<'_, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    make_grid_widget(py, GridPreset::Fission, position)
}

#[gen_stub_pyfunction]
#[pyfunction(name = "AmazonsWidget")]
#[pyo3(signature = (position = None))]
pub fn make_amazons_widget<'py>(
    py: Python<'py>,
    position: Option<&Bound<'_, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    make_grid_widget(py, GridPreset::Amazons, position)
}

#[gen_stub_pyfunction]
#[pyfunction(name = "KonaneWidget")]
#[pyo3(signature = (position = None))]
pub fn make_konane_widget<'py>(
    py: Python<'py>,
    position: Option<&Bound<'_, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    make_grid_widget(py, GridPreset::Konane, position)
}
