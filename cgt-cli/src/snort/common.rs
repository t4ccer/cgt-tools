use cgt::{numeric::rational::Rational, short::partizan::games::snort};

#[derive(serde::Serialize, serde::Deserialize)]
pub enum Log {
    Generation {
        generation: usize,
        top_score: Rational,
        temperature: Rational,
    },
    HighFitness {
        position: Scored,
        canonical_form: String,
        temperature: Rational,
        degree: usize,
    },
}

#[derive(Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Scored {
    pub position: snort::Position,
    pub score: Rational,
}

impl Scored {
    pub fn without_score(position: snort::Position) -> Self {
        Scored {
            position,
            score: Rational::NegativeInfinity,
        }
    }
}
