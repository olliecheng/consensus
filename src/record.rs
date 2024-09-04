use bincode;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Serialize, Deserialize, Eq, PartialEq, Hash, Debug, Default)]
pub struct RecordIdentifier {
    pub bc: String,
    pub umi: String,
}

// let mut record = ["", "", "?", "?", "", ""];
#[derive(Serialize, Deserialize, PartialEq, Debug, Default)]
pub struct Record {
    pub id: RecordIdentifier,
    pub read_id: Vec<u8>,
    pub loc: usize,
    pub avg_qual: f64,
    pub qual: Vec<u8>,
}
