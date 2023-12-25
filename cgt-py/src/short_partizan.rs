use crate::canonical_form::PyCanonicalForm;
use cgt::{
    drawing::svg::Svg,
    short::partizan::{
        games::{domineering::Domineering, ski_jumps::SkiJumps, toads_and_frogs::ToadsAndFrogs},
        partizan_game::PartizanGame,
        transposition_table::ParallelTranspositionTable,
    },
};
use pyo3::prelude::*;
use std::str::FromStr;

crate::impl_py_partizan_game!(
    "Domineering",
    Domineering,
    PyDomineering,
    "DomineeringTranspositionTable",
    ParallelTranspositionTable<Domineering>,
    PyDomineeringTranspositionTable
);

crate::impl_py_partizan_game!(
    "SkiJumps",
    SkiJumps,
    PySkiJumps,
    "SkiJumpsTranspositionTable",
    ParallelTranspositionTable<SkiJumps>,
    PySkiJumpsTranspositionTable
);

crate::impl_py_partizan_game!(
    "ToadsAndFrogs",
    ToadsAndFrogs,
    PyToadsAndFrogs,
    "ToadsAndFrogsTranspositionTable",
    ParallelTranspositionTable<ToadsAndFrogs>,
    PyToadsAndFrogsTranspositionTable
);
