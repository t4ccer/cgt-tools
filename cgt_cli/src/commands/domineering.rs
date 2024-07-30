mod common;

crate::clap_utils::mk_subcommand! {
    ExhaustiveSearch => exhaustive_search,
    GeneticSearch => genetic_search,
    Evaluate => evaluate,
    LatexTable => latex_table,
}
