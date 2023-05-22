pub use bio_seq::prelude::dna;
pub use bio_seq::prelude::{Dna, Seq as SeqImp};

use derive_more::{Display, From, Into};

use std::hash::{Hash, Hasher};

#[derive(Eq, PartialEq, From, Into, Display, Debug)]
pub struct Seq(SeqImp<Dna>);

impl Hash for Seq {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let seq_slice = &self.0[..];
        seq_slice.hash(state);
    }
}

impl Seq {
    pub fn new(s: &str) -> Self {
        let v = SeqImp::<Dna>::try_from(s).unwrap();
        Seq(v)
    }
}
