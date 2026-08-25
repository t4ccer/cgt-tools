use cgt::{
    grid::{CharTile, FiniteGrid, vec_grid::VecGrid},
    result::UnwrapInfallible,
    short::partizan::games::{
        amazons::{self, Amazons},
        domineering,
        fission::{self, Fission},
        konane::{self, Konane},
    },
};
use cgt_py_messages::{GridBackendMessage, GridFrontendMessage, GridPreset, Sequence, Tile};
use jupyter_rust_widget_backend::{Response, RustWidget};
use pyo3::{
    Bound, IntoPyObjectExt, Py, PyAny, PyResult, Python, exceptions::PyValueError, pyclass,
    pyfunction, pymethods,
};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use crate::{amazons::PyAmazons, domineering::PyDomineering, fission::PyFission, konane::PyKonane};

#[gen_stub_pyclass]
#[pyclass(name = "Grid")]
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
            .map_err(|err| PyValueError::new_err(err.to_string()))
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
                .map_err(|err| PyValueError::new_err(err.to_string()))?,
        )))
    }

    #[getter]
    pub fn amazons(&self) -> PyResult<PyAmazons> {
        Ok(PyAmazons(Amazons::new(
            self.grid
                .try_map(|t| amazons::Tile::try_from(*t))
                .map_err(|err| PyValueError::new_err(err.to_string()))?,
        )))
    }

    #[getter]
    pub fn konane(&self) -> PyResult<PyKonane> {
        Ok(PyKonane(Konane::new(
            self.grid
                .try_map(|t| konane::Tile::try_from(*t))
                .map_err(|err| PyValueError::new_err(err.to_string()))?,
        )))
    }
}

struct GridWidget {
    preset: GridPreset,
    grid: VecGrid<Tile>,

    /// Which version of the grid is held. A frontend that has fallen behind can send one
    /// from before an edit made in another, and that must not be taken
    sequence: Sequence,
}

impl GridWidget {
    fn set_grid_message(&self) -> GridFrontendMessage {
        GridFrontendMessage::SetGrid {
            sequence: self.sequence,
            grid: self.grid.clone(),
        }
    }
}

impl RustWidget for GridWidget {
    type BackendMessage = GridBackendMessage;
    type FrontendMessage = GridFrontendMessage;

    fn esm(&self) -> String {
        let bundle = include_str!("../widget/bundle.js");
        let preset = format!("const preset = {};", self.preset.into_flag_bits());
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

    fn handle_message(&mut self, event: Self::BackendMessage) -> Response<Self::FrontendMessage> {
        match event {
            GridBackendMessage::Initialized => Response {
                message: Some(self.set_grid_message()),
                run_on_update: false,
            },
            GridBackendMessage::SetGrid { sequence, grid } => {
                // Anything numbered at or below what is already held describes a grid this
                // side has moved on from, and running the update callbacks over it would
                // report a move that has since been played over
                let taken = sequence > self.sequence;
                if taken {
                    self.sequence = sequence;
                    self.grid = grid;
                }

                Response {
                    message: Some(self.set_grid_message()),
                    run_on_update: taken,
                }
            }
        }
    }

    fn value<'py>(&mut self) -> impl pyo3::IntoPyObject<'py> {
        PyGrid {
            known_preset: Some(self.preset),
            grid: self.grid.clone(),
        }
    }
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
    GridWidget {
        preset,
        grid,
        sequence: Sequence::INITIAL,
    }
    .into_widget(py, "cgt_py")
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
