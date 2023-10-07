pub mod bitenc;
pub mod dna;
pub mod id;
pub mod sequence;

pub use self::id::Identifier;
pub use self::sequence::Seq;

pub type Qual = String;

#[derive(Debug, PartialEq, Clone)]
pub struct Record {
    pub id: Identifier,
    pub data: RecordData,
}

#[derive(Debug, PartialEq, Clone)]
pub struct RecordData {
    pub metadata: Vec<u8>,
    pub seq: Seq,
    pub qual: Vec<u8>,
}
