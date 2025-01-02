use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub struct FastqFile {
    pub nailpolish_version: String,
    pub file_path: String,
    pub index_date: String,
    pub elapsed: f64,
    pub gb: f64,
    pub matched_read_count: usize,
    pub unmatched_read_count: usize,
    pub read_count: usize,
    pub avg_qual: f64,
    pub avg_len: f64,
    pub filtered_reads: usize,
}
