use crate::duplicates::DuplicateMap;
use crate::duplicates::RecordIdentifier;

use bio::io::fastq;
use bio::io::fastq::FastqRead;
use spoa::{AlignmentEngine, AlignmentType};

use std::fs::File;
use std::io::prelude::*;
use std::io::{Seek, SeekFrom};
use std::process::{Command, Stdio};

use anyhow::{anyhow, Context, Result};

// required for writeln! on a string
use std::fmt::Write as FmtWrite;

use pariter::IteratorExt as _;

struct DuplicateRecord {
    id: RecordIdentifier,
    records: Vec<fastq::Record>,
}

fn run_command(stdin: &str, shell: &str, command: &str) -> Result<Vec<u8>> {
    let mut child = Command::new(shell)
        .args(["-c", command])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Could not execute process");

    let mut stdin_pipe = child
        .stdin
        .take()
        .with_context(|| format!("Failed to take stdin pipe on input instance:\n{}", stdin))?;
    stdin_pipe
        .write_all(stdin.as_bytes())
        .with_context(|| format!("Failed to write to stdin pipe on instance:\n{}", stdin))?;

    // drop the stdin pipe to close the stream
    drop(stdin_pipe);

    let output = child
        .wait_with_output()
        .with_context(|| format!("Failed to wait on output for input instance:\n{}", stdin))?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        let stderr = String::from_utf8(output.stderr)?;
        match output.status.code() {
            Some(code) => Err(anyhow!(
                "Exited with status code: {code}. Stderr:\n{stderr}"
            )),
            None => Err(anyhow!("Process terminated by signal. Stderr:\n{stderr}")),
        }
    }
}

pub fn custom_command(
    input: &str,
    writer: &mut impl Write,
    duplicates: DuplicateMap,
    threads: usize,
    shell: &str,
    command: &str,
) -> Result<()> {
    let cache_size = threads * 3;

    let scope_obj = crossbeam::thread::scope(|scope| -> Result<()> {
        // this will store any errors
        let mut err = Ok(());

        iter_duplicates(input, duplicates, true)?
            .parallel_map_scoped_custom(
                scope,
                |o| o.threads(threads).buffer_size(cache_size),
                |rec| {
                    // propagate errors
                    let rec = rec?;

                    assert_ne!(rec.records.len(), 1);
                    let mut fastq_str = String::new();

                    for record in rec.records.iter() {
                        write!(fastq_str, "{}", record).context("Could not format string")?;
                    }

                    run_command(input, shell, command)
                },
            )
            .scan(&mut err, until_err)
            .for_each(|output| {
                writer
                    .write_all(&output)
                    .expect("Could not write to output");
            });
        err
    });

    scope_obj.unwrap_or_else(|e| {
        error!("Caught a panic which is unrecoverable");
        std::panic::resume_unwind(e)
    })
}

pub fn consensus(
    input: &str,
    writer: &mut impl Write,
    duplicates: DuplicateMap,
    threads: usize,
    duplicates_only: bool,
    output_originals: bool,
) -> Result<()> {
    let mut err = Ok(());
    let cache_size = threads * 3;

    let result = crossbeam::thread::scope(|s| -> Result<()> {
        let duplicate_iterator = iter_duplicates(input, duplicates, duplicates_only)?;

        duplicate_iterator
            // convert this sequential iterator into a parallel one for consensus calling
            .scan(&mut err, until_err)
            .parallel_map_scoped_custom(
                s,
                |o| o.threads(threads).buffer_size(cache_size),
                |rec| {
                    let single = rec.records.len() == 1;

                    let mut poa_graph;
                    if single {
                        let consensus = std::str::from_utf8(rec.records[0].seq()).unwrap();

                        format!(">{0}_{1}_SIN\n{2}\n", rec.id.bc, rec.id.umi, consensus)
                    } else {
                        let mut output = String::new();

                        // TODO: find a way to move this outside of the parallel map
                        let mut alignment_engine =
                            AlignmentEngine::new(AlignmentType::kOV, 5, -4, -8, -6, -10, -4);
                        poa_graph = spoa::Graph::new();

                        let record_count = rec.records.len();

                        for (index, record) in rec.records.iter().enumerate() {
                            let seq = record.seq();
                            let qual = record.qual();

                            if output_originals {
                                writeln!(
                                    output,
                                    ">{0}_{1}_DUP_{2}_of_{3}\n{4}",
                                    rec.id.bc,
                                    rec.id.umi,
                                    index + 1,
                                    record_count,
                                    std::str::from_utf8(seq).unwrap()
                                )
                                    .expect("string writing should not fail");
                            }

                            let align = alignment_engine.align_from_bytes(seq, &poa_graph);
                            poa_graph.add_alignment_from_bytes(&align, seq, qual);
                        }

                        let consensus = poa_graph.consensus();
                        let consensus = consensus
                            .to_str()
                            .expect("spoa module should produce valid utf-8");

                        writeln!(
                            output,
                            ">{0}_{1}_CON_{2}\n{3}",
                            rec.id.bc, rec.id.umi, record_count, consensus
                        )
                            .expect("string writing should not fail");

                        output
                    }
                },
            )
            .for_each(|output| {
                writer.write_all(output.as_bytes()).unwrap();
            });
        Ok(())
    });

    result.unwrap_or_else(|e| {
        error!("Caught a panic which is unrecoverable");
        std::panic::resume_unwind(e)
    })?;
    err
}

fn iter_duplicates(
    input: &str,
    duplicates: DuplicateMap,
    duplicates_only: bool,
) -> Result<impl Iterator<Item=Result<DuplicateRecord>> + '_> {
    let mut file = File::open(input).with_context(|| format!("Unable to open file {input}"))?;

    Ok(duplicates
        .into_iter()
        // first, read from the file (sequential)
        .filter_map(move |(id, positions)| -> Option<Result<DuplicateRecord>> {
            // if we choose not to only output duplicates, we can skip over this
            if (positions.len() == 1) && (duplicates_only) {
                return None;
            }

            let mut rec = DuplicateRecord {
                id,
                records: Vec::new(),
            };

            for pos in positions.iter() {
                let mut record = fastq::Record::new();
                let err = file.seek(SeekFrom::Start(*pos as u64));
                if let Err(e) = err {
                    let context = format!("Unable to seek to file {} at position {}", input, pos);
                    return Some(Err(anyhow::Error::new(e).context(context)));
                }

                let mut reader = fastq::Reader::new(&mut file);

                let err = reader.read(&mut record);
                if let Err(e) = err {
                    let context = format!("Unable to read from file {} at position {}", input, pos);
                    return Some(Err(anyhow::Error::new(e).context(context)));
                }

                rec.records.push(record);
            }
            Some(Ok(rec))
        }))
}

/// Utility function to extract the error from an iterator
fn until_err<T>(err: &mut &mut Result<()>, item: Result<T>) -> Option<T> {
    match item {
        Ok(item) => Some(item),
        Err(e) => {
            **err = Err(e);
            None
        }
    }
}
