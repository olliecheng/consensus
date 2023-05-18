use bio::alphabets;
use bio::data_structures::bitenc::{BitEnc, BitEncIter};
use delegate::delegate;
use std::fmt;

static alph: alphabets::Alphabet = alphabets::dna::alphabet();
static dna_ranks: alphabets::RankTransform = alphabets::RankTransform::new(&alph);
static dna_width: usize = dna_ranks.get_width(); // should be 2...

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct Seq {
    pub v: BitEnc,
}

impl Seq {
    pub fn new() -> Self {
        Self {
            v: BitEnc::new(dna_width),
        }
    }

    pub fn with_contents(sequence: &str) -> Self {
        let n = sequence.len();
        let v = BitEnc::with_capacity(dna_width, n);

        // write contents in
        // NOTE: THIS WILL FAIL FOR NON-ASCII INPUT
        // ENSURE THAT ALL SEQ INPUT IS ASCII
        for (i, value) in dna_ranks
            .transform(sequence.bytes())
            .into_iter()
            .enumerate()
        {
            v.set(i, value);
        }

        Self { v }
    }

    pub fn with_capacity(n: usize) -> Self {
        Self {
            v: BitEnc::with_capacity(dna_width, n),
        }
    }

    // 'Pass on' some boilerplate code into the inner BitEnc instance
    // through a clever (external) macro.
    // See https://github.com/Kobzol/rust-delegate
    delegate! {
        to self.v {
            pub fn push(&mut self, value: u8);
            pub fn push_values(&mut self, n: usize, value: u8);
            pub fn set(&mut self, i: usize, value: u8);
            pub fn get(&self, i: usize) -> Option<u8>;
            pub fn iter(&self) -> BitEncIter<'_>;
            pub fn is_empty(&self) -> bool;

            #[call(nr_symbols)]
            pub fn len(&self) -> usize;

        }
    }
}

impl fmt::Display for Seq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str_rep_of_vec = self.v.iter()
        write!(f, "{}", self.v)
    }
}
