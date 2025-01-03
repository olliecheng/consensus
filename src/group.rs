use crate::duplicates::DuplicateMap;
use crate::io::{ReadType, UMIGroupCollection};

use std::io::prelude::*;

use anyhow::Result;

/// Adds tags to duplicate reads from the input to show what group they are in.
///
/// # Arguments
///
/// * `input` - A string slice that holds the name of the input file.
/// * `writer` - A mutable reference to an object that implements the `Write` trait, used to write the output.
/// * `duplicates` - A `DuplicateMap` containing the duplicate reads.
///
/// # Returns
///
/// * `Result<()>` - Returns `Ok(())` if successful, or an error if an error occurs during processing.
pub fn group(collection: &mut UMIGroupCollection, writer: &mut impl Write) -> Result<()> {
    let mut duplicate_iterator = collection.stream_iter(false);

    let mut count = 0usize;

    let mut first = true;
    while let Some(mut group) = duplicate_iterator.next()? {
        count += 1;
        if count % 500000 == 0 {
            info!("Processed: {} reads", count);
        }

        let group_size = group.records.len();
        for (idx, rec) in group.records.iter_mut().enumerate() {
            if first {
                first = false
            } else {
                writer.write_all(b"\n")?
            }

            rec.add_metadata(group.index, ReadType::Original, idx + 1, group_size, 0.0);
            rec.write_fastq(writer)?;
        }
    }

    Ok(())
}
