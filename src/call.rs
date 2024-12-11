use crate::duplicates::DuplicateMap;
use crate::io::{iter_duplicates, ReadType, Record, UMIGroup};

use spoa::{AlignmentEngine, AlignmentType};

use rayon::prelude::*;

use std::io::prelude::*;
use std::io::Cursor;

use anyhow::Result;

use crate::io;

enum GroupType {
    Simplex(UMIGroup),
    Duplex(usize),
}


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
    rayon::ThreadPoolBuilder::new().num_threads(threads).build_global()?;

    let mut duplicate_iterator = iter_duplicates(input, duplicates, duplicates_only)?
        .peekable();

    let chunk_size = 100usize * threads;

    let mut chunk_buffer = Vec::with_capacity(chunk_size);
    let mut duplicate_buffer = Vec::new();

    let mut idx = 0;
    while let Some(elem) = duplicate_iterator.next() {
        idx += 1;

        if (idx > 0) && (idx % 100000 == 0) {
            eprintln!("Called {} reads...", idx);
        }

        // ensure that there was no issue in reading
        let group = elem?;

        let single = group.records.len() == 1;
        if (single && !duplicates_only) || group.ignore {
            chunk_buffer.push(GroupType::Simplex(group));
        } else {
            chunk_buffer.push(GroupType::Duplex(duplicate_buffer.len()));
            duplicate_buffer.push(group);
        }

        let end_of_buffer = duplicate_iterator.peek().is_none();

        // if we have filled the buffer OR are at the end, process this
        if (chunk_buffer.len() == chunk_size) || end_of_buffer {
            let mut duplicate_output = Vec::with_capacity(duplicate_buffer.len());

            // generate new records into a separate buffer
            duplicate_buffer
                .par_iter()
                .map(|grp| call_record(grp, output_originals))
                .collect_into_vec(&mut duplicate_output);

            for e in chunk_buffer.iter() {
                let output = match e {
                    GroupType::Simplex(group) => {
                        &call_record(group, output_originals)
                    }
                    // if this is a duplex read, then use the buffer
                    GroupType::Duplex(idx) => {
                        &duplicate_output[*idx]
                    }
                };

                writer.write_all(output)?;
            }

            // empty the buffer
            duplicate_buffer.clear();
            chunk_buffer.clear();
        }
    }

    Ok(())
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
fn call_record(group: &UMIGroup, output_originals: bool) -> Vec<u8> {
    let length = group.records.len();
    let mut output = Cursor::new(Vec::new());

    // process ignored reads first
    if group.ignore {
        for record in group.records.iter() {
            io::write_read(&mut output, record, &group, ReadType::Ignored, false).unwrap();
        }
        return output.into_inner();
    }


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
        "umi_group_id={} avg_input_quality={:.2}",
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

