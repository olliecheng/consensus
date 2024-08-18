use crate::duplicates::DuplicateMap;
use crate::record::RecordIdentifier;

use bio::io::fastq;
use bio::io::fastq::FastqRead;
use spoa::{self};

use std::fs::File;
use std::io::prelude::*;
use std::io::{Seek, SeekFrom};
use std::process::{Command, Stdio};

use rayon::prelude::*;
use rayon::{self};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};

// required for writeln! on a string
use std::fmt::Write as FmtWrite;

struct DuplicateRecord {
    id: RecordIdentifier,
    records: Vec<fastq::Record>,
}

pub fn custom_command(
    input: &str,
    writer: &Arc<Mutex<impl Write + Send>>,
    duplicates: DuplicateMap,
    threads: u8,
    shell: &str,
    command: &str,
) -> Result<()> {
    set_threads(threads)?;
    let mut err = Ok(());

    iter_duplicates(input, duplicates, true)?
        .scan(&mut err, until_err)
        .par_bridge()
        .for_each(|rec| {
            assert!(rec.records.len() != 1);
            let mut fastq_str = String::new();

            for record in rec.records.iter() {
                write!(fastq_str, "{}", record).unwrap() //.context("Could not format string")?;
            }

            let mut child = Command::new(shell)
                .args(["-c", command])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .expect("Could not execute process");

            let mut stdin = child.stdin.take().expect("Failed to open stdin");
            let thread = std::thread::spawn(move || {
                stdin
                    .write_all(fastq_str.as_bytes())
                    .expect("Failed to write to stdin");
            });

            let output = child.wait_with_output().expect("Failed to read stdout");

            let err = thread.join();
            if let Err(e) = err {
                std::panic::resume_unwind(e);
            }

            let writer = Arc::clone(writer);
            let mut writer = writer.lock().expect("Could not lock mutex");

            writer
                .write_all(&output.stdout)
                .expect("Could not write to output");
        });

    err
}

pub fn consensus(
    input: &str,
    writer: &Arc<Mutex<impl Write + Send>>,
    duplicates: DuplicateMap,
    threads: u8,
    duplicates_only: bool,
    output_originals: bool,
) -> Result<()> {
    set_threads(threads)?;
    let mut err = Ok(());

    iter_duplicates(input, duplicates, duplicates_only)?
        // convert this sequential iterator into a parallel one for consensus calling
        .scan(&mut err, until_err)
        .par_bridge()
        .for_each(|rec| {
            let single = rec.records.len() == 1;

            let mut poa_graph;
            if single {
                let consensus = std::str::from_utf8(rec.records[0].seq()).unwrap();

                let writer = Arc::clone(writer);
                let mut writer = writer.lock().unwrap();

                writeln!(
                    writer,
                    ">{0}_{1}_SIN\n{2}",
                    rec.id.bc, rec.id.umi, consensus
                )
                .unwrap();
            } else {
                let mut alignment_engine =
                    spoa::AlignmentEngine::new(spoa::AlignmentType::kOV, 5, -4, -8, -6, -10, -4);
                poa_graph = spoa::Graph::new();

                let record_count = rec.records.len();

                let mut existing_reads: Option<String> = if output_originals {
                    Some(String::new())
                } else {
                    None
                };

                for (index, record) in rec.records.iter().enumerate() {
                    let seq = record.seq();
                    let qual = record.qual();

                    if output_originals {
                        let s = format!(
                            ">{0}_{1}_DUP_{2}_of_{3}\n{4}\n",
                            rec.id.bc,
                            rec.id.umi,
                            index + 1,
                            record_count,
                            std::str::from_utf8(seq).unwrap()
                        );

                        if let Some(x) = existing_reads.as_mut() {
                            x.push_str(&s)
                        }
                    }

                    let align = alignment_engine.align_from_bytes(seq, &poa_graph);
                    poa_graph.add_alignment_from_bytes(&align, seq, qual);
                }

                let consensus = poa_graph.consensus();
                let consensus = consensus
                    .to_str()
                    .expect("spoa module should produce valid utf-8");

                let writer = Arc::clone(writer);
                let mut writer = writer.lock().unwrap();

                if output_originals {
                    // unwrap is fine here, as this is only Some() if output_originals is set
                    if let Some(s) = existing_reads {
                        writer.write_all(s.as_bytes()).unwrap();
                    }
                }

                // this is repeated, but I'm not sure how to pass out
                // a string slice from the if scope without creating
                // a borrow checker error.
                writeln!(
                    writer,
                    ">{0}_{1}_CON_{2}\n{3}",
                    rec.id.bc, rec.id.umi, record_count, consensus
                )
                .unwrap();
            };
        });

    err
}

fn set_threads(threads: u8) -> Result<()> {
    // set number of threads that Rayon uses
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads.into())
        .build_global()
        .with_context(|| format!("Unable to set the number of threads to {threads}"))
}

fn iter_duplicates(
    input: &str,
    duplicates: DuplicateMap,
    duplicates_only: bool,
) -> Result<impl Iterator<Item = Result<DuplicateRecord>> + '_> {
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
