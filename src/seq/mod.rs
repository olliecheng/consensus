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

pub type Seq = String;
pub type Qual = String;

pub type Barcode = String;
pub type Molecular = String;

#[derive(Hash, Eq, PartialEq, Debug)]
pub struct Identifier {
    pub bc: Barcode,
    pub umi: Molecular,
}

#[derive(Debug)]
pub struct ReadWithQual {
    pub id: Identifier,
    pub metadata: String,
    pub seq: Seq,
    pub qual: Qual,
}
