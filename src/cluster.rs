use crate::record::Record;

use anyhow::Result;

use itertools::Itertools;
use std::io::Cursor;
use crate::index::{ArchivedIndexPosition, Index, IndexPosition};
use crate::metrics::{Metric, Distance};
use rkyv::{util::archived_root, option::ArchivedOption, Deserialize};

struct RecordDist;

pub fn cluster_from(index: &str) -> Result<()> {
    let file = std::fs::File::open(index)?;

    // this is unsafe because of the risk of undefined behaviour
    // if the underlying file is modified.
    let mmap = unsafe { memmap2::Mmap::map(&file)? };
    let index = unsafe { archived_root::<Index>(&mmap[..]) };

    for (k, v) in index.lsh.hash_tables[0].iter() {
        let length = v.len();
        if length > 9 {
            println!("{}", v.len());
        }
    }

    // we create a mutable copy of the sorted indices, as this will be modified during
    // execution. The memory mapped `index` is immutable.
    let mut sorted_indices = index.sorted_indices.to_vec();

    let mut counts = std::collections::BTreeMap::new();

    // duplicates are considered as within a threshold of 2
    let threshold = 2;
    let mut collisions = 0u64;

    // in order to avoid an immutable borrow, we will index the array by position
    for (count, vec_index) in (0..sorted_indices.len()).enumerate() {
        if count % 50000 == 0 {
            info!("Processed: {count}");
            info!("Collision count: {} out of {}", collisions, count);
        }

        // skip read if it has been seen already
        let i = match sorted_indices[vec_index] {
            ArchivedIndexPosition::Removed => { continue }
            ArchivedIndexPosition::Present(i) => i as usize
        };

        // WARNING: THIS IS THE INDEXING OPERATION
        // Do *not* perform any mutable operation to `index.records` which would
        // push or remove elements or change the length in any way!
        // We use an unsafe block to avoid the bounds check here.
        let record = unsafe {
            index.records.get_unchecked(i)
        };

        let ArchivedOption::Some(hash) = &record.hash else {
            println!("Skipping, as there is no hash");
            continue;
        };

        // we query the LSH against this record
        let mut query = index.lsh.query_hash(hash);
        query.retain(|x| *x != i);

        let query_indices = query
            .into_iter()
            .filter(|j| *j > i); // only select elements we haven't seen yet

        let mut matches = 0;
        for j in query_indices {
            collisions += 1;
            matches += 1;
            let new_record = &index.records[j];

            let distance = record.id.distance_to(&new_record.id);
            if let Distance::Dist(d) = distance {
                if d <= threshold {
                    counts.entry(d).and_modify(|curr| *curr += 1).or_insert(1);
                    // we update this value to be type Removed, so it will be skipped over
                    // in the future
                    sorted_indices[j] = ArchivedIndexPosition::Removed;
                }
            }
        }
        println!("Collision {matches}");
    }

    println!("Counts: {:?}", counts);

    info!("Done retrieving records");

    Ok(())
}
