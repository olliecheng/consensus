use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;

#[derive(Eq, PartialEq, Hash, Debug)]
pub struct RecordIdentifier {
    pub bc: String,
    pub umi: String,
}

#[derive(Serialize, Debug)]
pub struct DuplicateStatistics {
    pub total_reads: usize,
    pub duplicate_reads: usize,
    pub duplicate_ids: usize,
    pub proportion_duplicate: f64,
    pub distribution: HashMap<usize, usize>,
}

pub fn get_duplicates(
    index: &str,
) -> Result<(HashMap<RecordIdentifier, Vec<usize>>, DuplicateStatistics), Box<dyn Error>> {
    let mut map = HashMap::<RecordIdentifier, Vec<usize>>::new();
    let mut stats = DuplicateStatistics {
        total_reads: 0,
        duplicate_reads: 0,
        duplicate_ids: 0,
        proportion_duplicate: 0.0,
        distribution: HashMap::new(),
    };

    let mut reader = ReaderBuilder::new()
        .delimiter(b'\t')
        .has_headers(true)
        .from_path(index)?;

    for read in reader.records() {
        let record = read?;
        stats.total_reads += 1;

        let id = RecordIdentifier {
            bc: record[1].to_string(),
            umi: record[4].to_string(),
        };

        let index = record[5].parse()?;
        if let Some(v) = map.get_mut(&id) {
            v.push(index);
        } else {
            map.insert(id, vec![index]);
        }
    }

    map.retain(|_, v| v.len() > 1);
    map.shrink_to_fit(); // optimise memory usage

    stats.duplicate_ids = map.len();
    stats.duplicate_reads = map
        .iter()
        .map(|(_, v)| {
            let length = v.len();
            if let Some(x) = stats.distribution.get_mut(&length) {
                *x += 1
            } else {
                stats.distribution.insert(length, 1);
            }
            length
        })
        .sum();
    stats
        .distribution
        .insert(1, stats.total_reads - stats.duplicate_reads);

    stats.proportion_duplicate = stats.duplicate_reads as f64 / stats.total_reads as f64;

    return Ok((map, stats));
}
