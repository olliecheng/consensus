use crate::duplicates::DuplicateMap;
use crate::duplicates::RecordIdentifier;
use bio::io::fastq;
use bio::io::fastq::FastqRead;
use rayon::ThreadPoolBuildError;

use std;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::io::{Seek, SeekFrom};
use std::process::{Command, Stdio};

// required for writeln! on a string
use std::fmt::Write as FmtWrite;

use std::sync::{Arc, Mutex};

use rayon::prelude::*;
use rayon::{self};

use spoa::{self};

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
) -> Result<(), Box<dyn Error>> {
    set_threads(threads)?;

    iter_duplicates(input, duplicates, true)?
        .par_bridge()
        .for_each(|rec| {
            assert!(rec.records.len() != 1);
            let mut fastq_str = String::new();

            for record in rec.records.iter() {
                write!(fastq_str, "{}", record).unwrap();
            }

            let mut child = Command::new(shell)
                .args(["-c", command])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .expect("Failed to execute process");

            let mut stdin = child.stdin.take().expect("Failed to open stdin");
            std::thread::spawn(move || {
                stdin
                    .write_all(fastq_str.as_bytes())
                    .expect("Failed to write to stdin");
            });

            let output = child.wait_with_output().expect("Failed to read stdout");

            let writer = Arc::clone(writer);
            let mut writer = writer.lock().unwrap();

            writer
                .write_all(&output.stdout)
                .expect("Failed to write to output");
        });
    Ok(())
}

pub fn consensus(
    input: &str,
    writer: &Arc<Mutex<impl Write + Send>>,
    duplicates: DuplicateMap,
    threads: u8,
    duplicates_only: bool,
    output_originals: bool,
) -> Result<(), Box<dyn Error>> {
    set_threads(threads)?;

    iter_duplicates(input, duplicates, duplicates_only)?
        // convert this sequential iterator into a parallel one for consensus calling
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
                        match existing_reads.as_mut() {
                            Some(x) => x.push_str(&s),
                            None => {}
                        }
                    }

                    let align = alignment_engine.align_from_bytes(seq, &poa_graph);
                    poa_graph.add_alignment_from_bytes(&align, seq, &qual);
                }

                let consensus = poa_graph.consensus();
                let consensus = consensus
                    .to_str()
                    .expect("spoa module should produce valid utf-8");

                let writer = Arc::clone(writer);
                let mut writer = writer.lock().unwrap();

                if output_originals {
                    // unwrap is fine here, as this is only Some() if output_originals is set
                    match existing_reads {
                        Some(s) => {
                            writer.write_all(s.as_bytes()).unwrap();
                        }
                        None => {}
                    };
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

    return Ok(());
}

fn set_threads(threads: u8) -> Result<(), ThreadPoolBuildError> {
    // set number of threads that Rayon uses
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads.into())
        .build_global()
}

fn iter_duplicates(
    input: &str,
    duplicates: DuplicateMap,
    duplicates_only: bool,
) -> Result<impl Iterator<Item = DuplicateRecord>, Box<dyn Error>> {
    let mut file = File::open(input)?;

    Ok(duplicates
        .into_iter()
        // first, read from the file (sequential)
        .filter_map(move |(id, positions)| {
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
                file.seek(SeekFrom::Start(*pos as u64))
                    .expect("Reading from file should not fail!");

                let mut reader = fastq::Reader::new(&mut file);
                reader.read(&mut record).unwrap();
                rec.records.push(record);
            }
            Some(rec)
        }))
}
