use crate::file::FastqFile;
use anyhow::{Context, Result};
use csv::ReaderBuilder;
use indexmap::IndexMap;
use serde::Serialize;
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader};

pub type DuplicateMap = IndexMap<RecordIdentifier, Vec<usize>>;

/// A struct representing a record identifier with a head and a tail.
///
/// # Fields
///
/// * `head` - The head part of the record identifier.
/// * `tail` - The tail part of the record identifier.
#[derive(Eq, PartialEq, Hash, Debug)]
pub struct RecordIdentifier {
    pub head: String,
    pub tail: String,
}

/// Implement the `Display` trait for `RecordIdentifier`. This allows a RecordIdentifier to
/// be converted to a string through `.to_string()` or using format macros. See `.from_string()`
/// for the inverse function.
impl std::fmt::Display for RecordIdentifier {
    /// Format the `RecordIdentifier` as a string.
    ///
    /// If the `tail` is empty, only the `head` is returned.
    /// Otherwise, the `head` and `tail` are concatenated with an underscore.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.tail.is_empty() {
            f.write_str(&self.head)
        } else {
            write!(f, "{}_{}", self.head, self.tail)
        }
    }
}

impl RecordIdentifier {
    /// Creates a `RecordIdentifier` from a string slice. See `.to_string()` for the inverse
    /// function.
    ///
    /// # Arguments
    ///
    /// * `s` - A string slice that holds the record identifier.
    ///
    /// # Returns
    ///
    /// A `RecordIdentifier` with the head and tail parts extracted from the input string.
    fn from_string(s: &str) -> Self {
        let split_loc = match s.find('_') {
            Some(v) => v,
            None => s.len() - 1
        };

        RecordIdentifier {
            head: s[..split_loc].to_string(),
            tail: s[(split_loc + 1)..].to_string(),
        }
    }
}

#[derive(Serialize, Debug)]
pub struct DuplicateStatistics {
    pub total_reads: usize,
    pub duplicate_reads: usize,
    pub duplicate_ids: usize,
    pub proportion_duplicate: f64,
    pub distribution: BTreeMap<usize, usize>,
}

/// Reads a FASTQ index file and identifies duplicate records.
///
/// # Arguments
///
/// * `index` - A string slice that holds the path to the index file.
///
/// # Returns
///
/// A tuple containing:
/// - `DuplicateMap`: A map of `RecordIdentifier` to a vector of indices where duplicates are found.
/// - `DuplicateStatistics`: Statistics about the duplicates found.
/// - `FastqFile`: Metadata about the FASTQ file.
///
/// # Errors
///
/// This function will return an error if the file cannot be opened or read, or if the file format is incorrect.
pub fn get_duplicates(index: &str) -> Result<(DuplicateMap, DuplicateStatistics, FastqFile)> {
    let mut map = IndexMap::<RecordIdentifier, Vec<usize>>::new();
    let mut stats = DuplicateStatistics {
        total_reads: 0,
        duplicate_reads: 0,
        duplicate_ids: 0,
        proportion_duplicate: 0.0,
        distribution: BTreeMap::new(),
    };

    let file = std::fs::File::open(index)?;
    let mut file = BufReader::new(file);

    let mut header = String::new();

    // read the first line, which is NOT in CSV format
    file.read_line(&mut header).context("Could not read the first line")?;

    assert!(header.starts_with('#'));
    let info: FastqFile = serde_json::from_str(&header[1..])?;

    // Create CSV builder
    let mut reader = ReaderBuilder::new()
        .delimiter(b'\t')
        .has_headers(true)
        .from_reader(&mut file);

    // Parse each row of the reader
    for read in reader.records() {
        let record = read?;
        stats.total_reads += 1;


        let id = RecordIdentifier::from_string(&record[0]);

        let index = record[1].parse()?;
        if let Some(v) = map.get_mut(&id) {
            v.push(index);
        } else {
            map.insert(id, vec![index]);
        }
    }

    map.shrink_to_fit(); // optimise memory usage

    // Compute information about the duplicates
    stats.duplicate_ids = 0;
    stats.duplicate_reads = map.values().map(|v| {
        let length = v.len();
        if length > 1 {
            stats.duplicate_ids += 1;

            if let Some(x) = stats.distribution.get_mut(&length) {
                *x += 1
            } else {
                stats.distribution.insert(length, 1);
            }
            length
        } else {
            0
        }
    })
        .sum();

    stats
        .distribution
        .insert(1, stats.total_reads - stats.duplicate_reads);

    stats.proportion_duplicate = stats.duplicate_reads as f64 / stats.total_reads as f64;

    Ok((map, stats, info))
}
