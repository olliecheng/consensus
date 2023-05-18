// TODO: use bit-packed seq obj
// use bio_seq::prelude::*;
//#[derive(PartialEq, Eq, Debug)]
// struct DNASeq(Seq<Dna>);

// impl Hash for DNASeq {
//     fn hash<H: Hasher>(&self, state: &mut H) {
//         self.bv.hash(state);
//     }
// }
// use std::hash::{Hash, Hasher};

// #[derive(Hash, Eq, PartialEq, Debug)]
// pub struct Seq(String);

// #[derive(Hash, Eq, PartialEq, Debug)]
// pub struct Qual(String);

pub mod id;
pub mod seq;
pub mod transformer;

pub use self::id::Identifier;
pub use self::seq::Seq;

pub type Qual = String;

#[derive(Debug)]
pub struct ReadWithQual {
    pub id: id::Identifier,
    pub metadata: String,
    pub seq: Seq,
    pub qual: Qual,
}
