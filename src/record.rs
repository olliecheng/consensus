use bincode;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Serialize, Deserialize, Eq, PartialEq, Hash, Debug, Default)]
pub struct RecordIdentifier {
    pub bc: String,
    pub umi: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Default)]
pub struct Record {
    pub id: RecordIdentifier,
    pub loc: usize,
    pub avg_qual: f64,
    pub hash: Option<Vec<u32>>,
}
