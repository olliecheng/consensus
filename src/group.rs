use crate::duplicates::DuplicateMap;
use crate::io::{iter_duplicates, until_err, ReadType};

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
pub fn group(
    input: &str,
    writer: &mut impl Write,
    duplicates: DuplicateMap,
) -> Result<()> {
    // Start with a placeholder error object. This will be mutated if there are errors during
    // iteration through the reads.
    let mut err = Ok(());

    let duplicate_iterator = iter_duplicates(
        input,
        duplicates,
        false,
    )?;

    duplicate_iterator
        // iterate until an error is found, writing into &err
        .scan(&mut err, until_err)
        .try_for_each(|group| -> Result<()> {
            for rec in group.records.iter() {
                // write to the output file
                crate::io::write_read(
                    writer,
                    &rec,
                    &group,
                    ReadType::ORIGINAL,
                    true,
                )?
            }

            Ok(())
        })?;

    err
}
