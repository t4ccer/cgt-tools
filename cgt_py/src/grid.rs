use cgt::{
    grid::{CharTile, FiniteGrid, vec_grid::VecGrid},
    short::partizan::games::{
        amazons::{self, Amazons},
        domineering,
        fission::{self, Fission},
    },
};
use cgt_py_messages::{GridBackendMessage, GridFrontendMessage, GridPreset, Tile};
use jupyter_rust_widget_backend::{Response, RustWidget};
use pyo3::{
    Bound, IntoPyObjectExt, Py, PyAny, PyResult, Python, exceptions::PyValueError, pyclass,
    pyfunction, pymethods,
};

use crate::{amazons::PyAmazons, domineering::PyDomineering, fission::PyFission};

#[pyclass]
struct PyGrid {
    // If `Some` then grid grid has tiles that can be represented in this game
    known_preset: Option<GridPreset>,
    grid: VecGrid<Tile>,
}

impl PyGrid {
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
}

#[pymethods]
impl PyGrid {
    #[getter]
    pub fn game(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        match self.known_preset {
            Some(preset) => match preset {
                GridPreset::Domineering => self.domineering()?.into_py_any(py),
                GridPreset::Fission => self.fission()?.into_py_any(py),
                GridPreset::Amazons => self.amazons()?.into_py_any(py),
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
}

struct GridWidget {
    preset: GridPreset,
    grid: VecGrid<Tile>,
}

impl RustWidget for GridWidget {
    type BackendMessage = GridBackendMessage;
    type FrontendMessage = GridFrontendMessage;

    fn esm(&self) -> String {
        let bundle = include_str!("../../cgt_py_widgets/dist/bundle.js");
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
                message: Some(GridFrontendMessage::SetGrid(self.grid.clone())),
                run_on_update: false,
            },
            GridBackendMessage::SetGrid { grid } => {
                self.grid = grid;
                Response {
                    message: Some(GridFrontendMessage::SetGrid(self.grid.clone())),
                    run_on_update: true,
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

#[pyfunction(name = "DomineeringWidget")]
pub fn make_domineering_widget(py: Python<'_>) -> PyResult<Bound<'_, PyAny>> {
    GridWidget {
        preset: GridPreset::Domineering,
        grid: FiniteGrid::filled(8, 8, Tile::Taken).unwrap(),
    }
    .into_widget(py, "cgt_py")
}

#[pyfunction(name = "FissionWidget")]
pub fn make_fission_widget(py: Python<'_>) -> PyResult<Bound<'_, PyAny>> {
    GridWidget {
        preset: GridPreset::Fission,
        grid: FiniteGrid::filled(4, 4, Tile::Empty).unwrap(),
    }
    .into_widget(py, "cgt_py")
}

#[pyfunction(name = "AmazonsWidget")]
pub fn make_amazons_widget(py: Python<'_>) -> PyResult<Bound<'_, PyAny>> {
    GridWidget {
        preset: GridPreset::Amazons,
        grid: FiniteGrid::filled(4, 4, Tile::Empty).unwrap(),
    }
    .into_widget(py, "cgt_py")
}
