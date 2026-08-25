#![allow(non_local_definitions)] // These come from pyo3 marcos

use jupyter_rust_widget_backend::inject_rust_widget;
use pyo3::prelude::*;

pub mod amazons;
pub mod bipartite_snort;
pub mod canonical_form;
pub mod col;
pub mod digraph_placement;
pub mod domineering;
pub mod dyadic_rational_number;
pub mod fission;
pub mod graph;
pub mod grid;
pub mod konane;
pub mod snort;
pub mod thermograph;

macro_rules! py_partizan_game {
    ($pystruct:ident) => {
        #[gen_stub_pymethods]
        #[pymethods]
        impl $pystruct {
            #[getter]
            fn canonical_form(
                this: pyo3::Bound<'_, $pystruct>,
            ) -> crate::canonical_form::PyCanonicalForm {
                use cgt::short::partizan::partizan_game::PartizanGame;

                crate::canonical_form::PyCanonicalForm(
                    this.borrow().0.canonical_form(&*TRANSPOSITION_TABLE),
                )
            }

            #[getter]
            fn left_options(&self) -> Vec<$pystruct> {
                use cgt::short::partizan::partizan_game::PartizanGame;

                self.0.left_moves().into_iter().map($pystruct).collect()
            }

            #[getter]
            fn right_options(&self) -> Vec<$pystruct> {
                use cgt::short::partizan::partizan_game::PartizanGame;

                self.0.right_moves().into_iter().map($pystruct).collect()
            }

            #[getter]
            fn sensible_right_options(&self) -> Vec<$pystruct> {
                use cgt::short::partizan::partizan_game::PartizanGame;

                self.0
                    .sensible_right_moves(&*TRANSPOSITION_TABLE)
                    .into_iter()
                    .map($pystruct)
                    .collect()
            }

            #[getter]
            fn sensible_left_options(&self) -> Vec<$pystruct> {
                use cgt::short::partizan::partizan_game::PartizanGame;

                self.0
                    .sensible_left_moves(&*TRANSPOSITION_TABLE)
                    .into_iter()
                    .map($pystruct)
                    .collect()
            }
        }
    };
}
pub(crate) use py_partizan_game;

#[pymodule]
fn cgt_py(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    inject_rust_widget(py, m)?;
    m.add_function(wrap_pyfunction!(grid::make_domineering_widget, m)?)?;
    m.add_function(wrap_pyfunction!(grid::make_fission_widget, m)?)?;
    m.add_function(wrap_pyfunction!(grid::make_amazons_widget, m)?)?;
    m.add_function(wrap_pyfunction!(grid::make_konane_widget, m)?)?;

    m.add_function(wrap_pyfunction!(graph::make_snort_widget, m)?)?;
    m.add_function(wrap_pyfunction!(graph::make_col_widget, m)?)?;
    m.add_function(wrap_pyfunction!(graph::make_digraph_placement_widget, m)?)?;
    m.add_function(wrap_pyfunction!(graph::make_bipartite_snort_widget, m)?)?;

    m.add_class::<crate::amazons::PyAmazons>()?;
    m.add_class::<crate::bipartite_snort::PyBipartiteSnort>()?;
    m.add_class::<crate::canonical_form::PyCanonicalForm>()?;
    m.add_class::<crate::col::PyCol>()?;
    m.add_class::<crate::digraph_placement::PyDigraphPlacement>()?;
    m.add_class::<crate::domineering::PyDomineering>()?;
    m.add_class::<crate::dyadic_rational_number::PyDyadicRationalNumber>()?;
    m.add_class::<crate::fission::PyFission>()?;
    m.add_class::<crate::graph::PyGraph>()?;
    m.add_class::<crate::grid::PyGrid>()?;
    m.add_class::<crate::graph::PyVertexColors>()?;
    m.add_class::<crate::konane::PyKonane>()?;
    m.add_class::<crate::snort::PySnort>()?;
    m.add_class::<crate::thermograph::PyThermograph>()?;

    Ok(())
}

pyo3_stub_gen::define_stub_info_gatherer!(stub_info);
