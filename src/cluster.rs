use crate::record::Record;

use anyhow::Result;

use itertools::Itertools;
use std::io::Cursor;
use crate::index::IndexPosition;
use crate::metrics::{Metric, Distance};

struct RecordDist;

pub fn cluster_from(index: &str) -> Result<()> {
    let reader = std::fs::File::open(index)?;
    let mut index: crate::index::Index = bincode::deserialize_from(reader)?;

    let mut counts = std::collections::BTreeMap::new();

    // duplicates are considered as within a threshold of 2
    let threshold = 2;

    // in order to avoid an immutable borrow, we will index the array by position
    for vec_index in 0..index.sorted_indices.len() {
        // skip read if it has been seen already
        let i = match index.sorted_indices[vec_index] {
            IndexPosition::Removed => { continue }
            IndexPosition::Present(i) => i
        };

        // WARNING: THIS IS THE INDEXING OPERATION
        // Do *not* perform any mutable operation to `index.records` which would
        // push or remove elements or change the length in any way!
        // We use an unsafe block to avoid the bounds check here.
        let record = unsafe {
            index.records.get_unchecked(i)
        };

        let Some(hash) = &record.hash else {
            println!("Skipping, as there is no hash");
            continue;
        };

        // we query the LSH against this record
        let mut query = index.lsh.query_hash(hash);
        query.retain(|x| *x != i);

        let query_indices = query
            .into_iter()
            .filter(|j| *j > i); // only select elements we haven't seen yet

        for j in query_indices {
            let new_record = &index.records[j];

            let distance = record.id.distance_to(&new_record.id);
            if let Distance::Dist(d) = distance {
                if d <= threshold {
                    counts.entry(d).and_modify(|curr| *curr += 1).or_insert(1);
                    // we update this value to be type Removed, so it will be skipped over
                    // in the future
                    index.sorted_indices[j] = IndexPosition::Removed;
                }
            }
        }
    }

    println!("Counts: {:?}", counts);

    info!("Done retrieving records");

    Ok(())
}
