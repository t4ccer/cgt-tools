mod common;

crate::clap_utils::mk_subcommand! {
    Bipartite => bipartite,
    Genetic => genetic,
    Latex => latex,
    Graph => graph,
    ThreeCaterpillar => three_caterpillar,
}
