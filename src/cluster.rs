use crate::generate_index::Hamming;
use crate::record::Record;

use anyhow::Result;

use bk_tree::{metrics, BKTree, Metric};
use triple_accel::hamming;

use itertools::Itertools;
use std::io::Cursor;

struct RecordDist;

pub fn cluster_from(index: &str) -> Result<()> {
    let records: Vec<Record> = bincode::deserialize_from(std::fs::File::open(index)?)?;
    info!("Done retreiving records");

    Ok(())
}
