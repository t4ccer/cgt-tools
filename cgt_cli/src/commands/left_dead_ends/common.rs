use cgt::{
    misere::left_dead_end::{
        LeftDeadEndContext,
        interned::{Interner, LeftDeadEnd},
    },
    total::{TotalWrappable, TotalWrapper},
};
use itertools::Itertools;

pub fn to_all_factorizations(interner: &Interner, game: &LeftDeadEnd) -> Vec<Vec<LeftDeadEnd>> {
    let fs = interner.factors(game);
    let mut acc: Vec<Vec<LeftDeadEnd>> = Vec::new();

    for (f, c) in fs.into_iter().filter(|&(f, c)| {
        TotalWrapper::new(f) != TotalWrapper::new(LeftDeadEnd::new_integer(0))
            && TotalWrapper::new(c) != TotalWrapper::new(LeftDeadEnd::new_integer(0))
    }) {
        let fs = if interner.is_atom(&f) {
            vec![vec![f]]
        } else {
            to_all_factorizations(interner, &f)
        };

        let cs = if interner.is_atom(&c) {
            vec![vec![c]]
        } else {
            to_all_factorizations(interner, &c)
        };

        let res = fs
            .into_iter()
            .cartesian_product(cs)
            .map(|(fs, cs)| {
                let mut v = Vec::new();
                v.extend(fs);
                v.extend(cs);
                v.sort_by(TotalWrappable::total_cmp);
                v
            })
            .collect::<Vec<_>>();
        acc.extend(res);
    }

    acc.sort_by(TotalWrappable::total_cmp);
    acc.dedup_by(|lhs, rhs| TotalWrappable::total_eq(lhs, rhs));
    acc
}
