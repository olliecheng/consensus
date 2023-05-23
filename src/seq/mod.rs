pub mod dna;
pub mod id;
pub mod seq;

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
