//! Poset Utilities
//!
//! Adapted from <https://crates.io/crates/poset>

use std::borrow::Cow;

/// Iterator over the antichains from a set of chains.
#[derive(Debug, Clone)]
pub struct AntichainIterator<'a, T, P>
where
    T: Clone,
{
    vectors: Cow<'a, [Vec<T>]>,
    indices: Vec<Option<usize>>,
    is_finished: bool,
    partial_order: P,
}

impl<'a, T, P> AntichainIterator<'a, T, P>
where
    T: Clone,
    P: Fn(&T, &T) -> bool,
{
    /// Create new iterator from elements according to given partial order
    pub fn new(elements: Vec<T>, partial_order: P) -> AntichainIterator<'a, T, P> {
        let chain_decomposition =
            ChainDecompositionIterator::new(elements, &partial_order).collect::<Vec<_>>();

        AntichainIterator {
            indices: vec![None; chain_decomposition.len()],
            vectors: Cow::Owned(chain_decomposition),
            is_finished: false,
            partial_order,
        }
    }

    /// Create new iterator from chain decomposition according to given partial order
    pub fn with_chain_decomposition(
        vectors: &'a [Vec<T>],
        partial_order: P,
    ) -> AntichainIterator<'a, T, P> {
        AntichainIterator {
            indices: vec![None; vectors.len()],
            vectors: Cow::Borrowed(vectors),
            is_finished: false,
            partial_order,
        }
    }

    fn is_incomparable(&self, combination: &[T]) -> bool {
        for (i, item1) in combination.iter().enumerate() {
            for item2 in combination.iter().skip(i + 1) {
                if (self.partial_order)(item1, item2) || (self.partial_order)(item2, item1) {
                    return false;
                }
            }
        }

        true
    }

    fn advance_indices(&mut self) -> Result<(), ()> {
        for i in (0..self.indices.len()).rev() {
            match self.indices[i] {
                None => {
                    if !self.vectors[i].is_empty() {
                        self.indices[i] = Some(0);
                        return Ok(());
                    }
                }
                Some(idx) if idx + 1 < self.vectors[i].len() => {
                    self.indices[i] = Some(idx + 1);
                    return Ok(());
                }
                _ => {
                    self.indices[i] = None;
                }
            }
        }

        Err(())
    }
}

impl<T, P> Iterator for AntichainIterator<'_, T, P>
where
    T: Clone,
    P: Fn(&T, &T) -> bool,
{
    type Item = Vec<T>;

    fn next(&mut self) -> Option<Self::Item> {
        while !self.is_finished {
            let combination: Vec<T> = self
                .indices
                .iter()
                .enumerate()
                .filter_map(|(i, &idx)| idx.and_then(|idx| self.vectors[i].get(idx)))
                .cloned()
                .collect();

            if self.advance_indices().is_err() {
                self.is_finished = true;
            }

            if self.is_incomparable(&combination) {
                return Some(combination);
            }
        }

        None
    }
}

/// Iterator over chain decomposition of a poset
#[derive(Debug, Clone)]
pub struct ChainDecompositionIterator<T, P, F> {
    vertices: Vec<T>,
    partial_order: P,
    on_inserted: F,
    is_finished: bool,
}

impl<T, P> ChainDecompositionIterator<T, P, fn()>
where
    T: Clone,
    P: Fn(&T, &T) -> bool,
{
    /// Create new iterator over a partial order
    pub fn new(elements: Vec<T>, partial_order: P) -> ChainDecompositionIterator<T, P, fn()> {
        ChainDecompositionIterator::with_callback(elements, partial_order, || {})
    }
}

impl<T, P, F> ChainDecompositionIterator<T, P, F>
where
    T: Clone,
    P: Fn(&T, &T) -> bool,
    F: Fn(),
{
    /// Create new iterator over a partial order with a callback on each element inserted into a chain
    pub fn with_callback(
        elements: Vec<T>,
        partial_order: P,
        on_inserted: F,
    ) -> ChainDecompositionIterator<T, P, F> {
        ChainDecompositionIterator {
            vertices: elements,
            partial_order,
            on_inserted,
            is_finished: false,
        }
    }

    fn minimum_in_pool<'a>(&self, pool: &'a [T]) -> Option<&'a T> {
        pool.iter().find(|&v| {
            !pool
                .iter()
                .any(|w| (self.partial_order)(v, w) && !(self.partial_order)(w, v))
        })
    }

    fn cover_in_pool<'a>(&self, x: &T, y: &T, pool: impl IntoIterator<Item = &'a T>) -> bool
    where
        T: 'a,
    {
        if !((self.partial_order)(y, x) && !(self.partial_order)(x, y)) {
            return false;
        }

        !pool.into_iter().any(|z| {
            ((self.partial_order)(z, x) && !(self.partial_order)(x, z))
                && ((self.partial_order)(y, z) && !(self.partial_order)(z, y))
        })
    }

    fn chain_from_pool(&mut self) -> Vec<T> {
        if self.vertices.is_empty() {
            return Vec::new();
        }

        let other = self.vertices.clone();
        let first = self.minimum_in_pool(&other).unwrap().clone();
        let mut chain = vec![first.clone()];
        let mut latest = &chain[0];

        'outer: loop {
            for x in &other {
                if self.cover_in_pool(latest, x, other.iter()) {
                    chain.push(x.clone());
                    (self.on_inserted)();
                    latest = x;
                    continue 'outer;
                }
            }
            break;
        }

        self.vertices.retain(|x| {
            !chain
                .iter()
                .any(|y| (self.partial_order)(x, y) && (self.partial_order)(y, x))
        });

        chain
    }
}

impl<T, P, F> Iterator for ChainDecompositionIterator<T, P, F>
where
    T: Clone,
    P: Fn(&T, &T) -> bool,
    F: Fn(),
{
    type Item = Vec<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_finished {
            return None;
        }

        let chain = self.chain_from_pool();
        if chain.is_empty() {
            self.is_finished = true;
        }

        Some(chain)
    }
}
