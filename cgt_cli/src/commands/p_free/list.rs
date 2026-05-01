use crate::io::FilePathOr;
use anyhow::Result;
use cgt::{
    misere::game_form::{
        DeadEndingFormContext, GameFormContext, PFreeDeadEndingContext, PFreeDeadEndingFormContext,
        PFreeFormContext, StandardFormContext,
    },
    poset::AntichainIterator,
    result::{UnwrapInfallible, Void},
};
use indicatif::{ProgressBar, ProgressStyle};
use std::{
    borrow::Borrow,
    collections::{BTreeSet, VecDeque},
    io::{self, Stdout, Write},
    process::Stdio,
    sync::{
        RwLock,
        atomic::{self, AtomicU64, AtomicUsize},
    },
    time::Duration,
};

#[derive(Debug, clap::Parser)]
pub struct Args {
    /// Day to print
    #[arg(long)]
    day: u32,

    /// Dot output path
    #[arg(long, default_value = None)]
    dot: Option<FilePathOr<Stdout>>,

    /// Pdf output path
    #[arg(long, default_value = None)]
    pdf: Option<FilePathOr<Stdout>>,

    /// TeX/tikz output path
    #[arg(long, default_value = None)]
    tex: Option<FilePathOr<Stdout>>,
    // TODO: Support variant
}

fn compute_relations<C, Form>(
    context: &C,
    forms: &[Form],
    bar: &ProgressBar,
) -> (Vec<bool>, Vec<bool>)
where
    C: PFreeDeadEndingContext + Send + Sync,
    C::IntegerConstructionError: Void,
    C::Form: Send + Sync,
    Form: Borrow<C::Form> + Send + Sync,
{
    let n = forms.len();

    let lt = vec![false; n * n];
    let mut eq = vec![false; n * n];

    for i in 0..n {
        eq[i * n + i] = true;
    }

    parallel_for(0, n * (n - 1) / 2, |idx| {
        let _ = DropGuard(|| bar.inc(1));

        // Map flat coordinates into a triangle
        let a = 2 * n - 1;
        let mut i = (a - (a * a - 8 * idx).isqrt()) / 2;
        let mut start = i * (2 * n - i - 1) / 2;
        if start > idx {
            i -= 1;
            start = i * (2 * n - i - 1) / 2;
        }
        let j = i + 1 + (idx - start);

        let i_le_j = context.ge_mod_p_free_dead_ending(&forms[i].borrow(), &forms[j].borrow());
        let j_le_i = context.ge_mod_p_free_dead_ending(&forms[j].borrow(), &forms[i].borrow());

        // SAFETY: Each thread gets scheduled with a unique pair of indices that are in bounds
        unsafe {
            if i_le_j && j_le_i {
                eq.as_ptr().add(i * n + j).cast_mut().write(true);
                eq.as_ptr().add(j * n + i).cast_mut().write(true);
            } else if i_le_j {
                lt.as_ptr().add(i * n + j).cast_mut().write(true);
            } else if j_le_i {
                lt.as_ptr().add(j * n + i).cast_mut().write(true);
            }
        }
    });

    (eq, lt)
}

fn precompute_ge_relations<C, Form>(context: &C, forms: &[Form]) -> Vec<bool>
where
    C: PFreeDeadEndingContext + Send + Sync,
    C::IntegerConstructionError: Void,
    C::Form: Send + Sync,
    Form: Borrow<C::Form> + Send + Sync,
{
    let n = forms.len();

    let mut ge = vec![false; n * n];

    for i in 0..n {
        ge[i * n + i] = true;
    }

    parallel_for(0, n * (n - 1) / 2, |idx| {
        // Map flat coordinates into a triangle
        let a = 2 * n - 1;
        let mut i = (a - (a * a - 8 * idx).isqrt()) / 2;
        let mut start = i * (2 * n - i - 1) / 2;
        if start > idx {
            i -= 1;
            start = i * (2 * n - i - 1) / 2;
        }
        let j = i + 1 + (idx - start);

        let i_ge_j = context.ge_mod_p_free_dead_ending(&forms[i].borrow(), &forms[j].borrow());
        let j_ge_i = context.ge_mod_p_free_dead_ending(&forms[j].borrow(), &forms[i].borrow());

        // SAFETY: Each thread gets scheduled with a unique pair of indices that are in bounds
        unsafe {
            ge.as_ptr().add(i * n + j).cast_mut().write(i_ge_j);
            ge.as_ptr().add(j * n + i).cast_mut().write(j_ge_i);
        }
    });

    ge
}

fn compute_partitioned_antichains<C>(
    context: &C,
    forms: &[C::Form],
    bar: &ProgressBar,
) -> Vec<Vec<C::Form>>
where
    C: PFreeDeadEndingContext + Send + Sync,
    C::IntegerConstructionError: Void,
    C::Form: Send + Sync,
{
    let n = forms.len();
    if n == 0 {
        return Vec::new();
    }

    let (eq, lt) = compute_relations(context, forms, bar);

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
                if eq[u * n + v] && class_id[v].is_none() {
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
            if lt[i * n + j] {
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

struct DropGuard<F: FnMut()>(F);

impl<F: FnMut()> Drop for DropGuard<F> {
    fn drop(&mut self) {
        (self.0)()
    }
}

fn parallel_for(from: usize, to: usize, action: impl Fn(usize) + Send + Sync) {
    let i = AtomicUsize::new(from);
    let num_threads = 12; // FIXME: get it from CLI/libc
    std::thread::scope(|scope| {
        for _ in 0..num_threads {
            scope.spawn(|| {
                loop {
                    let i = i.fetch_add(1, atomic::Ordering::Relaxed);
                    if i >= to {
                        break;
                    }
                    action(i);
                }
            });
        }
    });
}

fn parallel<T>(
    mut make_tasks: impl FnMut(crossbeam_channel::Sender<T>) + Send,
    perform_task: impl Fn(T) + Send + Sync,
) where
    T: Send,
{
    let num_threads = 12; // FIXME: get it from CLI/libc
    let (tx, rx) = crossbeam_channel::bounded::<T>(num_threads * 4);

    std::thread::scope(|scope| {
        for _ in 0..num_threads {
            let rx = rx.clone();
            let perform_task = &perform_task;
            scope.spawn(move || {
                loop {
                    match rx.recv() {
                        Ok(g) => perform_task(g),
                        Err(_) => break,
                    }
                }
            });
        }

        make_tasks(tx);
    });
}

#[must_use]
fn next_day<C>(context: &C, previous_day: Vec<C::Form>) -> Vec<C::Form>
where
    C: PFreeDeadEndingContext + Send + Sync,
    C::IntegerConstructionError: Void,
    C::Form: Send + Sync,
{
    let ge = precompute_ge_relations(context, &previous_day);
    let antichains = AntichainIterator::new((0..previous_day.len()).collect(), |lhs, rhs| {
        ge[lhs * previous_day.len() + rhs]
    })
    .collect::<Vec<_>>();

    let n = antichains.len() as u64;
    let bar = ProgressBar::new(n * n).with_style(progress_style());
    bar.enable_steady_tick(Duration::from_secs(1));

    let seen = RwLock::new(Vec::<C::Form>::new());

    parallel(
        |tx| {
            for l in &antichains {
                for r in &antichains {
                    tx.send((
                        l.iter()
                            .map(|idx| previous_day[*idx].clone())
                            .collect::<Vec<_>>(),
                        r.iter()
                            .map(|idx| previous_day[*idx].clone())
                            .collect::<Vec<_>>(),
                    ))
                    .unwrap();
                }
            }
        },
        |(l, r)| {
            let _ = DropGuard(|| bar.inc(1));

            let Ok(non_reduced) = context.new(l, r) else {
                return;
            };

            if !context.is_p_free(&non_reduced) || !context.is_dead_ending(&non_reduced) {
                return;
            }

            let reduced = context.reduced(&non_reduced);

            let already_checked = {
                let seen = seen.read().unwrap();
                if seen.iter().any(|h| context.total_eq(&reduced, h)) {
                    return;
                }
                seen.len()
            };

            // Two therads may have constructed equal games so even though we just checked that
            // the game is new we need to check again after taking the exclusive write lock.
            // We only append games so the first `constructed_so_far` are not equal to our game
            // so we only check the tail, i.e. games inserted by other therads after we did the check
            let mut seen = seen.write().unwrap();
            if seen[already_checked..]
                .iter()
                .any(|h| context.total_eq(&reduced, h))
            {
                return;
            }
            seen.push(reduced);
            bar.set_message(format!("(Found {})", seen.len()));
        },
    );

    bar.finish();
    eprintln!();

    seen.into_inner().unwrap()
}

fn deduplicate_equal<C>(context: &C, games: &mut Vec<C::Form>, bar: &ProgressBar)
where
    C: PFreeDeadEndingContext + Send + Sync,
    C::IntegerConstructionError: Void,
    C::Form: Send + Sync,
{
    let duplicates = AtomicU64::new(0);

    let is_removed = vec![false; games.len()];
    parallel_for(0, games.len(), |idx| {
        let _ = DropGuard(|| bar.inc(1));
        let g = &games[idx];
        if let Some(h) = games[0..idx]
            .iter()
            .find(|h| context.eq_mod_p_free_dead_ending(g, h))
        {
            // SAFETY: Each thread gets scheduled a unique index that is in bounds
            unsafe {
                is_removed.as_ptr().add(idx).cast_mut().write(true);
            }
            let duplicates = duplicates.fetch_add(1, atomic::Ordering::SeqCst);
            bar.println(format!("  {} = {}", context.display(h), context.display(g)));
            bar.set_message(format!("(Found {duplicates})"));
        }
    });

    let mut i = 0;
    games.retain(|_| {
        let retain = !is_removed[i];
        i += 1;
        retain
    });
}
fn generate_hasse<C, W>(
    context: &C,
    mut w: W,
    antichains: &[Vec<C::Form>],
    bar: &ProgressBar,
) -> io::Result<()>
where
    C: PFreeDeadEndingContext + Send + Sync,
    C::IntegerConstructionError: Void,
    C::Form: Send + Sync,
    W: io::Write,
{
    writeln!(w, "graph Hasse {{")?;
    writeln!(w, "  rankdir=BT;")?;

    let day = antichains.iter().flatten().collect::<Vec<_>>();

    let (_, lt) = compute_relations(context, day.as_slice(), bar);

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
            if i == j || !lt[j * day.len() + i] {
                continue;
            }

            for k in 0..day.len() {
                if k == i || k == j {
                    continue;
                }

                if lt[j * day.len() + k] && lt[k * day.len() + i] {
                    continue 'inner;
                }
            }

            writeln!(w, "  {} -- {};", i, j)?;
        }
    }

    writeln!(w, "}}")
}

fn progress_style() -> ProgressStyle {
    ProgressStyle::with_template("[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
        .unwrap()
        .progress_chars("#> ")
}

#[allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]
pub fn run(args: Args) -> Result<()> {
    if args.dot.is_none() && args.pdf.is_none() && args.tex.is_none() {
        eprintln!("Warning: Not generating any output");
    }

    let context = PFreeDeadEndingFormContext::new(PFreeFormContext::new(
        DeadEndingFormContext::new(StandardFormContext),
    ));

    let style = progress_style();

    let mut day = vec![context.new_integer(0).unwrap_infallible()];
    for day_number in 0..args.day {
        eprintln!("Generating day {}/{}", day_number + 1, args.day);
        day = next_day(&context, day);
    }

    {
        eprintln!("Deduplicating");
        let form_count: u64 = day.len() as u64;
        let bar = ProgressBar::new(form_count).with_style(style.clone());
        deduplicate_equal(&context, &mut day, &bar);
        bar.finish();
        eprintln!();
    }

    {
        let game_count: u64 = day.len() as u64;

        let day_antichains = {
            eprintln!("Calculating antichain partitions for day {}", args.day);
            let bar = ProgressBar::new(game_count * (game_count - 1) / 2).with_style(style.clone());
            let res = compute_partitioned_antichains(&context, &day, &bar);
            bar.finish();
            eprintln!();
            res
        };

        eprintln!("Generating Hasse diagram");
        let bar = ProgressBar::new(game_count * (game_count - 1) / 2).with_style(style.clone());
        let mut graphviz = Vec::new();
        // TODO: Reuse `lt` table generated in `compute_partitioned_antichains` if we can remap indices
        generate_hasse(&context, &mut graphviz, &day_antichains, &bar)?;
        bar.finish();
        eprintln!();
        drop(bar);

        if let Some(dot_output) = &args.dot {
            let mut output = dot_output.create()?;
            output.write_all(&graphviz)?;
        }

        if let Some(pdf_output) = &args.pdf {
            let mut output = pdf_output.create()?;

            let mut dot2tex = std::process::Command::new("dot")
                .arg("-Tpdf")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()?;
            dot2tex.stdin.take().unwrap().write_all(&graphviz)?;
            output.write_all(&dot2tex.wait_with_output()?.stdout)?;
        }

        if let Some(tex_output) = &args.tex {
            let mut output = tex_output.create()?;

            let mut dot2tex = std::process::Command::new("dot2tex")
                .arg("--codeonly")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()?;
            dot2tex.stdin.take().unwrap().write_all(&graphviz)?;
            output.write_all(&dot2tex.wait_with_output()?.stdout)?;
        }
    }

    // for g in &day {
    //     for h in &day {
    //         let sum = context.sum(g, h).unwrap();
    //         let sum_reduced = context.reduced(&sum);
    //         println!(
    //             "{} + {} = {}",
    //             context.display(g),
    //             context.display(h),
    //             context.display(&sum_reduced),
    //         );
    //     }
    // }

    Ok(())
}
