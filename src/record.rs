use bincode;
use std::collections::BTreeSet;

#[derive(
    serde::Serialize,
    serde::Deserialize,
    rkyv::Archive,
    rkyv::Serialize,
    Eq,
    PartialEq,
    Hash,
    Debug,
    Default
)]
pub struct RecordIdentifier {
    pub bc: String,
    pub umi: String,
}


#[derive(
    serde::Serialize,
    serde::Deserialize,
    rkyv::Archive,
    rkyv::Serialize,
    PartialEq,
    Debug,
    Default
)]
pub struct Record {
    pub id: RecordIdentifier,
    pub loc: usize,
    pub avg_qual: f64,
    pub hash: Option<Vec<u64>>,
}
