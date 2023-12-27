//! Utilities for genetic search

use rand::{rngs::ThreadRng, seq::SliceRandom};
use std::num::NonZeroUsize;

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Object with score attached
pub struct Scored<Object, Score> {
    /// Object under inspection
    pub object: Object,

    /// Score after running evaluation function
    pub score: Score,
}

/// Definition of a genetic algorithm
pub trait Algorithm<Object, Score> {
    /// Mutate object in place
    fn mutate(&self, object: &mut Object, rng: &mut ThreadRng);

    /// Combine two objects into one
    fn cross(&self, lhs: &Object, rhs: &Object, rng: &mut ThreadRng) -> Object;

    /// Get the lowest possible score, used for initial setup
    fn lowest_score(&self) -> Score;

    /// Evaluate fitness of an object. Algorithm will try to maximize this value according to [`Ord`]
    fn score(&self, object: &Object) -> Score;

    /// Create a totally random object, used for initial population
    fn random(&self, rng: &mut ThreadRng) -> Object;
}

/// Genetic algorithm runner
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
    /// Create new instance with given population size and random population
    pub fn new(size: NonZeroUsize, algorithm: Alg) -> Self {
        let mut rng = rand::thread_rng();
        let specimen = (0..size.get())
            .map(|_| algorithm.random(&mut rng))
            .collect::<Vec<_>>();

        Self::with_specimen(specimen, size, algorithm)
    }

    /// Like [`Self::new`] but will use initial populaiton. If initial population is smaller than
    /// generation size rest will be filled with random objects
    pub fn with_specimen(mut specimen: Vec<Object>, size: NonZeroUsize, algorithm: Alg) -> Self {
        let mut rng = rand::thread_rng();
        let to_generate = size.get().saturating_sub(specimen.len());
        specimen.extend((0..to_generate).map(|_| algorithm.random(&mut rng)));
        let specimen = specimen
            .into_iter()
            .map(|object| Scored {
                object,
                score: algorithm.lowest_score(),
            })
            .collect::<Vec<_>>();

        let mut s = Self {
            specimen,
            generation: 0,
            algorithm,
        };
        s.score();
        s
    }

    /// Get object with highest fitness
    pub fn highest_score(&self) -> &Scored<Object, Score> {
        self.specimen.last().expect("unreachable")
    }

    fn score(&mut self) {
        self.specimen
            .iter_mut()
            .for_each(|spec| spec.score = self.algorithm.score(&spec.object));

        self.specimen
            .sort_unstable_by(|lhs, rhs| Ord::cmp(&lhs.score, &rhs.score));
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

    /// Perform one generation step
    pub fn step_generation(&mut self) {
        self.cross();
        self.score();
        self.generation += 1;
    }

    /// Get number of finished (scored) generations
    pub const fn generation(&self) -> usize {
        self.generation
    }

    /// Get underlying algorithm
    pub const fn algorithm(&self) -> &Alg {
        &self.algorithm
    }

    /// Get scored specimen, ordered by their score
    pub fn specimen(&self) -> &[Scored<Object, Score>] {
        &self.specimen
    }
}
