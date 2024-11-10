#![allow(dead_code)]

use std::collections::BTreeSet;

use anyhow::Result;
use cgt::misere::left_dead_end::LeftDeadEnd;
use clap::{self, Parser};
use itertools::Itertools;
use poset::{PartialOrder, Poset, PosetBehaviour};
use rand::{distributions::uniform::SampleRange, thread_rng, Rng};

fn next_integer(r: impl SampleRange<u64>) -> LeftDeadEnd {
    let mut rng = thread_rng();
    let g = rng.gen_range(r);
    LeftDeadEnd::new_integer(g)
}

fn next_molecule(r: impl SampleRange<u64> + Clone) -> LeftDeadEnd {
    let mut rng = thread_rng();

    let res = loop {
        let size = rng.gen_range(2..3);
        let mut moves = Vec::with_capacity(size);
        for _ in 0..size {
            moves.push(next_integer(r.clone()));
        }

        let res = LeftDeadEnd::new_moves(moves);

        if res.factors().len() > 2 {
            eprintln!("next_molecule: Got: {res}");
            dbg!(res.factors());
            break res;
        } else {
            eprintln!("next_molecule: Rejected: {res}");
        }
    };

    res
}

fn next_atom_of_molecules(r: impl SampleRange<u64> + Clone) -> LeftDeadEnd {
    loop {
        let mut m = Vec::new();
        for _ in 0..2 {
            m.push(next_molecule(r.clone()));
        }
        let g = LeftDeadEnd::new_moves(m);
        if g.is_atom() {
            return g;
        }
    }
}

fn next_candidate(r: impl SampleRange<u64> + Clone) -> LeftDeadEnd {
    let mut rng = thread_rng();
    let mut m = LeftDeadEnd::new_integer(0);
    for _ in 0..rng.gen_range(3..4) {
        let g = next_atom_of_molecules(r.clone());
        eprintln!("  += {g}");
        m += &g;
    }
    m
}

fn round() {
    loop {
        let c = next_candidate(0..4);

        eprintln!("Trying");

        let fs = c.factors();
        eprintln!("Factors: {fs:?}");
        let s = fs.into_iter().sum();

        if c != s {
            eprintln!("Got: {c}");
            break;
        } else {
            eprintln!("Rejected");
        }
    }
}

fn next_day_via_powerset<'a>(
    games: impl Iterator<Item = &'a LeftDeadEnd>,
) -> BTreeSet<LeftDeadEnd> {
    games
        .powerset()
        .fold(BTreeSet::new(), |mut acc: BTreeSet<LeftDeadEnd>, x| {
            let g = LeftDeadEnd::new_moves(x.into_iter().cloned().collect());
            // if !acc.contains(&g) {
            //     println!("{g}");
            // }
            acc.insert(g);
            acc
        })
}

#[derive(Debug, Clone, Parser)]
pub struct Args {}
pub fn run(_args: Args) -> Result<()> {
    let day0 = BTreeSet::from_iter(vec![LeftDeadEnd::new_integer(0)]);
    println!("Day 1:");
    let day1 = next_day_via_powerset(day0.iter());
    println!("Day 2:");
    let day2 = next_day_via_powerset(day1.iter());
    println!("Day 3:");
    let day3 = next_day_via_powerset(day2.iter());
    println!("Day 4:");
    let day4 = next_day_via_powerset(day3.iter());

    println!("Day 5:");
    let p_ord = PartialOrder::new(LeftDeadEnd::ge_games);
    let p = Poset::with_elements(day4, p_ord);
    // for (idx, chain) in p.chain_decomposition().enumerate() {
    //     eprintln!("Chain {idx}:");
    //     for chain_elem in chain {
    //         eprintln!("  {chain_elem}")
    //     }
    // }

    let day5 = p
        .antichains(p.chain_decomposition().unwrap())
        .map(LeftDeadEnd::new_moves)
        .map(|p| {
            println!("{p}");
            p
        })
        .collect::<Vec<_>>();

    println!("Day 6:");
    let p_ord = PartialOrder::new(LeftDeadEnd::ge_games);
    let p = Poset::with_elements(day5, p_ord);
    let _day6 = p
        .antichains(p.chain_decomposition().unwrap())
        .map(LeftDeadEnd::new_moves)
        .map(|p| {
            println!("{p}");
            p
        })
        .collect::<Vec<_>>();

    // for _ in 0..4 {
    //     day_no += 1;
    //     eprintln!("Day {day_no}:");

    //     let this_day = next_day(prev_day.iter());
    //     eprintln!("  #: {}", this_day.len());
    //     prev_day = this_day;
    // }

    // round();

    Ok(())
}
