pub mod dna;
pub mod id;
pub mod sequence;

pub use self::id::Identifier;
pub use self::sequence::Seq;

pub type Qual = String;

#[derive(Debug, PartialEq)]
pub struct Record {
    pub id: Identifier,
    pub metadata: String,
    pub seq: Seq,
    pub qual: Qual,
}
