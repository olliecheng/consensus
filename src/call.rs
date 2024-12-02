use crate::duplicates::DuplicateMap;
use crate::io::{iter_duplicates, until_err, ReadType, Record, UMIGroup};

use spoa::{AlignmentEngine, AlignmentType};

use std::io::prelude::*;
use std::io::Cursor;

use anyhow::Result;

use crate::io;
use pariter::IteratorExt as _;


/// Generates consensus sequences from the input in a thread-stable manner.
///
/// # Arguments
///
/// * `input` - A string slice that holds the path to the input file.
/// * `writer` - A mutable reference to an object that implements the `Write` trait,
///   used for writing the output.
/// * `duplicates` - A `DuplicateMap` containing the duplicate reads.
/// * `threads` - The number of threads to use for parallel processing.
/// * `duplicates_only` - A boolean indicating whether to process only duplicate reads.
/// * `output_originals` - A boolean indicating whether to include the original reads in the output.
///
/// # Returns
///
/// * `Result<()>` - Returns `Ok(())` if successful, or an error if an error occurs
///   during processing.
pub fn consensus(
    input: &str,
    writer: &mut impl Write,
    duplicates: DuplicateMap,
    threads: usize,
    duplicates_only: bool,
    output_originals: bool,
) -> Result<()> {
    // Start with a placeholder error object. This will be mutated if there are errors during
    // iteration through the reads.
    let mut err = Ok(());

    let cache_size = threads * 3;

    let result = crossbeam::thread::scope(|scope| -> Result<()> {
        let duplicate_iterator = iter_duplicates(
            input,
            duplicates,
            duplicates_only,
        )?;

        // convert this sequential iterator into a parallel one for consensus calling
        duplicate_iterator
            .scan(&mut err, until_err) // iterate until an error is found, writing into &err
            .parallel_map_scoped_custom(
                scope,
                |o| o.threads(threads).buffer_size(cache_size),
                |r| call_record(r, output_originals),
            )
            // write every read in a global thread in order
            .for_each(|output| {
                writer.write_all(&output).unwrap();
            });

        Ok(())
    });

    // Threads can't send regular errors well between them, so
    // if there is an issue here we panic
    result.unwrap_or_else(|e| {
        error!("Caught a panic which is unrecoverable");
        std::panic::resume_unwind(e)
    })?;

    err
}

/// Generates a consensus sequence from a group of reads.
///
/// # Arguments
///
/// * `group` - A `UMIGroup` containing the reads to be processed.
/// * `output_originals` - A boolean indicating whether to include the original reads in the
///   output alongside the consensus read.
///
/// # Returns
///
/// A `String` containing the consensus sequence in FASTQ format.
fn call_record(group: UMIGroup, output_originals: bool) -> Vec<u8> {
    let length = group.records.len();
    let mut output = Cursor::new(Vec::new());

    // for singletons, the read is its own consensus
    if length == 1 {
        let record = &group.records[0];
        io::write_read(&mut output, record, &group, ReadType::Consensus, false).unwrap();
        return output.into_inner();
    }

    // initialise `spoa` machinery
    let mut alignment_engine =
        AlignmentEngine::new(AlignmentType::kOV, 5, -4, -8, -6, -10, -4);
    let mut poa_graph = spoa::Graph::new();

    // add each read in the duplicate group to the graph
    for record in group.records.iter() {
        if output_originals {
            // Write the original reads as well
            io::write_read(&mut output, record, &group, ReadType::Original, false).unwrap();
        }

        // Align to the graph
        let align = alignment_engine.align_from_bytes(record.seq.as_ref(), &poa_graph);
        poa_graph.add_alignment_from_bytes(&align, record.seq.as_ref(), record.qual.as_ref());
    }

    // Create a consensus read
    let consensus_str = poa_graph.consensus();
    let consensus_str = consensus_str
        .to_str()
        .expect("spoa module did not produce valid utf-8");

    let id_string = format!(
        "consensus_{} avg_input_quality={:.2}",
        group.index,
        group.avg_qual
    );

    let consensus = Record {
        id: id_string,
        seq: consensus_str.to_string(),
        qual: "".to_string(),
    };

    io::write_read(&mut output, &consensus, &group, ReadType::Consensus, false).unwrap();

    output.into_inner()
}

