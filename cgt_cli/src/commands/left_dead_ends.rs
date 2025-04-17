mod common;

crate::clap_utils::mk_subcommand! {
    Factorizations => factorizations,
    Analyze => analyze,
    Evaluate => evaluate,
}
