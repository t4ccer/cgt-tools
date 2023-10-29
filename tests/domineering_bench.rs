use cgt::short::partizan::{
    games::domineering::Domineering, partizan_game::PartizanGame,
    transposition_table::TranspositionTable,
};
use std::hint::black_box;

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

#[test]
#[cfg(not(miri))]
fn bench_domineering() {
    let profiler = dhat::Profiler::builder().build();

    let width = black_box(4);
    let height = black_box(4);

    let transposition_table = TranspositionTable::new();
    for i in 0..(width * height) {
        let domineering = Domineering::from_number(width as u8, height as u8, i).unwrap();
        let _ = domineering.canonical_form(&transposition_table);
    }

    let stats = dhat::HeapStats::get();
    eprintln!("{:#?}", stats);
    drop(profiler);
}
