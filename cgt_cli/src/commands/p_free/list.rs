use crate::io::FilePathOr;
use anyhow::Result;
use cgt::{
    misere::game_form::{
        DeadEndingFormContext, GameFormContext, PFreeDeadEndingContext, PFreeDeadEndingFormContext,
        PFreeFormContext, StandardFormContext,
    },
    result::{UnwrapInfallible, Void},
};
use itertools::Itertools;
use std::{
    collections::{BTreeSet, VecDeque},
    io::{self, BufWriter, Stdout},
};

#[derive(Debug, clap::Parser)]
pub struct Args {
    /// Day to print
    #[arg(long)]
    day: u32,

    #[arg(long, default_value = "-")]
    output: FilePathOr<Stdout>,

    #[arg(long, default_value_t = false)]
    print_equal: bool,
    // TODO: Support variant
}

fn compute_partitioned_antichains<C>(context: &C, forms: &[C::Form]) -> Vec<Vec<C::Form>>
where
    C: PFreeDeadEndingContext,
    C::IntegerConstructionError: Void,
{
    let n = forms.len();
    if n == 0 {
        return Vec::new();
    }

    // Memorize relations
    let mut lt = vec![vec![false; n]; n];
    let mut eq = vec![vec![false; n]; n];

    for i in 0..n {
        eq[i][i] = true;
        for j in (i + 1)..n {
            let i_le_j = context.ge_mod_p_free_dead_ending(&forms[j], &forms[i]);
            let j_le_i = context.ge_mod_p_free_dead_ending(&forms[i], &forms[j]);

            if i_le_j && j_le_i {
                eq[i][j] = true;
                eq[j][i] = true;
            } else if i_le_j {
                lt[i][j] = true;
            } else if j_le_i {
                lt[j][i] = true;
            }
        }
    }

    let mut class_id = vec![None; n];
    let mut classes: Vec<Vec<usize>> = Vec::new();
    let mut queue = VecDeque::new();

    for start in 0..n {
        if class_id[start].is_some() {
            continue;
        }
        let cid = classes.len();
        let mut component = Vec::new();
        queue.push_back(start);
        class_id[start] = Some(cid);
        while let Some(u) = queue.pop_front() {
            component.push(u);
            for v in 0..n {
                if eq[u][v] && class_id[v].is_none() {
                    class_id[v] = Some(cid);
                    queue.push_back(v);
                }
            }
        }
        classes.push(component);
    }

    let num_classes = classes.len();
    let class_id: Vec<usize> = class_id.into_iter().map(Option::unwrap).collect();

    let mut class_edges: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); num_classes];
    let mut class_in_degree = vec![0; num_classes];

    for i in 0..n {
        let a = class_id[i];
        for j in 0..n {
            if lt[i][j] {
                let b = class_id[j];
                if a != b && class_edges[a].insert(b) {
                    class_in_degree[b] += 1;
                }
            }
        }
    }

    // Kahn's algorithm on the class DAG to assign ranks.
    let mut class_rank = vec![0; num_classes];
    let mut queue = VecDeque::new();

    for c in 0..num_classes {
        if class_in_degree[c] == 0 {
            queue.push_back(c);
        }
    }

    while let Some(u) = queue.pop_front() {
        for &v in &class_edges[u] {
            class_rank[v] = class_rank[v].max(class_rank[u] + 1);
            class_in_degree[v] -= 1;
            if class_in_degree[v] == 0 {
                queue.push_back(v);
            }
        }
    }

    // Map ranks back to the original forms and group by rank.
    let ranks: Vec<usize> = (0..n).map(|i| class_rank[class_id[i]]).collect();
    let max_rank = ranks.iter().max().copied().unwrap_or(0);
    let mut antichains: Vec<Vec<C::Form>> = vec![Vec::new(); max_rank + 1];

    for (i, &rank) in ranks.iter().enumerate() {
        antichains[rank].push(forms[i].clone());
    }

    for antichain in &mut antichains {
        // Prefer integer form
        antichain.sort_unstable_by(|lhs, rhs| {
            context
                .to_integer(rhs)
                .cmp(&context.to_integer(lhs))
                .then_with(|| context.total_cmp(lhs, rhs))
        });
    }

    antichains
}

#[must_use]
fn next_day_antichains<C>(
    context: &C,
    previous_day_antichains: &[Vec<C::Form>],
) -> Vec<Vec<C::Form>>
where
    C: PFreeDeadEndingContext,
    C::IntegerConstructionError: Void,
{
    let mut seen: Vec<C::Form> = Vec::new();

    for a1 in previous_day_antichains {
        for a2 in previous_day_antichains {
            for l in a1.iter().powerset() {
                for r in a2.iter().powerset() {
                    let Ok(g) = context.new(
                        l.iter().map(|g| C::Form::clone(g)),
                        r.iter().map(|g| C::Form::clone(g)),
                    ) else {
                        continue;
                    };
                    let g = context.reduced(&g);
                    if !context.is_p_free(&g) || !context.is_dead_ending(&g) {
                        continue;
                    }

                    if seen.iter().any(|h| context.total_eq(&g, h)) {
                        continue;
                    }

                    seen.push(g.clone());
                }
            }
        }
    }

    // FIXME: That is not correct is it?
    // We actually need all maximal antichains if we want to feed it recursively.
    // Otherwise we would never get e.g. {-1, {-3|2}} as a set of moves in day 5 generation
    compute_partitioned_antichains(context, &seen)
}

fn print_equal<C>(context: &C, games: &[C::Form])
where
    C: PFreeDeadEndingContext,
    C::IntegerConstructionError: Void,
{
    let mut seen = vec![false; games.len()];

    for i in 0..games.len() {
        if seen[i] {
            continue;
        }

        eprint!("  {}", context.display(&games[i]),);
        seen[i] = true;

        for j in (i + 1)..games.len() {
            if !seen[j] && context.eq_mod_p_free_dead_ending(&games[i], &games[j]) {
                seen[j] = true;
                eprint!(" = {}", context.display(&games[j]));
            }
        }

        eprintln!();
    }
}

fn deduplicate_equal<C>(context: &C, antichains: &mut [Vec<C::Form>])
where
    C: PFreeDeadEndingContext,
    C::IntegerConstructionError: Void,
{
    let mut seen: Vec<C::Form> = Vec::new();
    for antichain in antichains {
        antichain.retain(|g| {
            if seen.iter().any(|h| context.eq_mod_p_free_dead_ending(g, h)) {
                false
            } else {
                seen.push(g.clone());
                true
            }
        });
    }
}

fn generate_hasse<C, W>(context: &C, mut w: W, antichains: &[Vec<C::Form>]) -> io::Result<()>
where
    C: PFreeDeadEndingContext,
    C::IntegerConstructionError: Void,
    W: io::Write,
{
    writeln!(w, "graph Hasse {{")?;
    writeln!(w, "  rankdir=BT;")?;

    let day = antichains.iter().flatten().collect::<Vec<_>>();
    let mut ge = vec![false; day.len() * day.len()];
    for i in 0..day.len() {
        for j in 0..day.len() {
            ge[i * day.len() + j] = context.ge_mod_p_free_dead_ending(&day[i], &day[j]);
        }
    }

    let mut i = 0;
    for antichain in antichains {
        writeln!(w, "  {{ rank = same;")?;
        for g in antichain {
            writeln!(
                w,
                "    {} [label = \"{}\", texlbl = \"${}$\"]",
                i,
                context.display(g),
                context.display_tex(g),
            )?;
            i += 1;
        }
        writeln!(w, "  }}")?;
    }

    for i in 0..day.len() {
        'inner: for j in 0..day.len() {
            if i == j || !ge[j * day.len() + i] {
                continue;
            }

            for k in 0..day.len() {
                if k == i || k == j {
                    continue;
                }

                if ge[j * day.len() + k] && ge[k * day.len() + i] {
                    continue 'inner;
                }
            }

            writeln!(w, "  {} -- {};", i, j)?;
        }
    }

    writeln!(w, "}}")
}

#[allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]
pub fn run(args: Args) -> Result<()> {
    let mut output = BufWriter::new(args.output.create()?);

    let context = PFreeDeadEndingFormContext::new(PFreeFormContext::new(
        DeadEndingFormContext::new(StandardFormContext),
    ));

    let mut day_antichains = vec![vec![context.new_integer(0).unwrap_infallible()]];
    for _ in 0..args.day {
        day_antichains = next_day_antichains(&context, &mut day_antichains);
    }

    if args.print_equal {
        dbg!(day_antichains.len());
        for antichain in &day_antichains {
            eprintln!("Antichain:");
            print_equal(&context, antichain);
        }
    }

    deduplicate_equal(&context, &mut day_antichains);

    generate_hasse(&context, &mut output, &day_antichains)?;

    Ok(())
}
