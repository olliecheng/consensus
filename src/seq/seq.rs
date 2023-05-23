pub use crate::seq::dna;
use std::fmt;

use bio::data_structures::bitenc::{BitEnc, BitEncIter};
use derive_more::{From, Into};

#[derive(Eq, PartialEq, From, Into, Debug, Hash)]
pub struct Seq(BitEnc);

impl Seq {
    pub fn new() -> Self {
        Self(BitEnc::new(2))
    }

    pub fn with_capacity(n: usize) -> Self {
        Self(BitEnc::with_capacity(2, n))
    }

    pub fn push(&mut self, b: u8) {
        self.0.push(dna::a_to_b(b))
    }

    pub fn add_iter(&mut self, i: impl Iterator<Item = u8>) {
        i.map(dna::a_to_b).for_each(|x| self.0.push(x));
    }
}

impl fmt::Display for Seq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self
            .0
            .iter()
            .map(dna::b_to_a)
            .collect::<Vec<&str>>()
            .join("");
        write!(f, "{}", s)
    }
}
