use crate::record::Record;

use anyhow::Result;

use itertools::Itertools;
use std::io::Cursor;

struct RecordDist;

pub fn cluster_from(index: &str) -> Result<()> {
    let index: crate::index::Index = bincode::deserialize_from(std::fs::File::open(index)?)?;

    let seen = Vec::new();
    let mut count = 0u32;
    for i in index.sorted_indices {
        count += 1;
        if seen.contains(&i) {
            continue;
        }

        let record = &index.records[i];
        if let Some(hash) = &(record.hash) {
            let mut query = index.lsh.query_hash(hash);
            query.retain(|x| *x != i);

            if query.len() > 0 {
                println!("{:?}: {}", record.id, query.len());
                for j in query {
                    let record = &index.records[j];
                    println!("  {:?}", record.id);
                }
            }
            println!("");
        } else {
            println!("Skip because no hash");
        }
    }

    info!("Done retrieving records (count: {})", count);

    Ok(())
}
