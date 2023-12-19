//! Utilities for genetic search

#![allow(missing_docs)]

use rand::{rngs::ThreadRng, seq::SliceRandom};
use std::num::NonZeroUsize;

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Scored<Object, Score> {
    pub object: Object,
    pub score: Score,
}

pub trait Algorithm<Object, Score> {
    fn mutate(&self, object: &mut Object, rng: &mut ThreadRng);

    fn cross(&self, lhs: &Object, rhs: &Object, rng: &mut ThreadRng) -> Object;

    fn lowest_score(&self) -> Score;

    fn score(&self, object: &Object) -> Score;

    fn random(&self, rng: &mut ThreadRng) -> Object;
}

pub struct GeneticAlgorithm<Alg, Object, Score> {
    specimen: Vec<Scored<Object, Score>>,
    generation: usize,
    algorithm: Alg,
}

impl<Alg, Object, Score> GeneticAlgorithm<Alg, Object, Score>
where
    Alg: Algorithm<Object, Score>,
    Score: Clone + Ord,
    Object: Clone,
{
    pub fn new(size: NonZeroUsize, algorithm: Alg) -> Self {
        let mut rng = rand::thread_rng();
        let specimen = (0..size.get())
            .map(|_| algorithm.random(&mut rng))
            .collect::<Vec<_>>();

        Self::with_specimen(specimen, size, algorithm)
    }

    pub fn with_specimen(mut specimen: Vec<Object>, size: NonZeroUsize, algorithm: Alg) -> Self {
        let mut rng = rand::thread_rng();
        let to_generate = specimen.len().checked_sub(size.get()).unwrap_or(0);
        specimen.extend((0..to_generate).map(|_| algorithm.random(&mut rng)));
        let specimen = specimen
            .into_iter()
            .map(|object| Scored {
                object,
                score: algorithm.lowest_score(),
            })
            .collect::<Vec<_>>();

        Self {
            specimen,
            generation: 0,
            algorithm,
        }
    }

    pub fn highest_score(&self) -> &Scored<Object, Score> {
        self.specimen.last().unwrap()
    }

    fn score(&mut self) {
        self.specimen
            .iter_mut()
            .for_each(|spec| spec.score = self.algorithm.score(&spec.object));

        self.specimen
            .sort_unstable_by(|lhs, rhs| Ord::cmp(&lhs.score, &rhs.score))
    }

    fn cross(&mut self) {
        let mut rng = rand::thread_rng();
        let generation_size = self.specimen.len();
        let mid_point = generation_size / 2;
        let mut new_specimen = Vec::with_capacity(generation_size);
        let top_half = &self.specimen[mid_point..];
        new_specimen.extend_from_slice(top_half);
        for _ in new_specimen.len()..generation_size {
            let lhs = self.specimen.choose(&mut rng).unwrap();
            let rhs = self.specimen.choose(&mut rng).unwrap();
            let mut object = self.algorithm.cross(&lhs.object, &rhs.object, &mut rng);
            self.algorithm.mutate(&mut object, &mut rng);
            new_specimen.push(Scored {
                object,
                score: self.algorithm.lowest_score(),
            });
        }
        self.specimen = new_specimen;
    }

    pub fn step_generation(&mut self) {
        self.cross();
        self.score();
        self.generation += 1;
    }

    /// Get number of finished (scored) generations
    pub fn generation(&self) -> usize {
        self.generation
    }

    pub fn algorithm(&self) -> &Alg {
        &self.algorithm
    }

    pub fn specimen(&self) -> &[Scored<Object, Score>] {
        &self.specimen
    }
}
