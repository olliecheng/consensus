use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

use csv::{Reader, Writer, WriterBuilder};
use regex::Regex;

use crate::generate_index::IndexGenerationErr::{InvalidClusterRow, RowNotInClusters};
use anyhow::{bail, Context, Result};
use needletail::parser::SequenceRecord;
use needletail::{FastxReader, Sequence};
use thiserror::Error;

use crate::file::FastqFile;
use tempfile::tempfile_in;

// returns the average PHRED quality of the read
fn write_read<W: Write>(
    wtr: &mut Writer<W>,
    rec: &SequenceRecord,
    identifier: &str,
    position: usize,
) -> Result<f64> {
    let len = rec.num_bases();
    let qual: u32 = rec.qual().expect(".fastq should not fail here")
        .iter()
        .map(|x| *x as u32)
        .sum();

    let avg_qual = (qual as f64) / (len as f64);

    // we transform the quality to a PHRED score (ASCII ! to I)
    // https://en.wikipedia.org/wiki/Phred_quality_score
    let phred_qual = avg_qual - 33f64;

    wtr.write_record(
        [
            identifier,
            &position.to_string(),
            &format!("{:.2}", phred_qual),
            &len.to_string()
        ]
    )?;
    Ok(phred_qual)
}

fn iter_lines_with_regex<W: Write>(
    reader: BufReader<File>,
    wtr: &mut Writer<W>,
    re: &Regex,
    skip_invalid_ids: bool,
    mut info: FastqFile,
) -> Result<FastqFile> {
    // expected_len is used to ensure that every read has the same format
    let mut expected_len: Option<usize> = None;

    let mut fastq_reader = needletail::parser::FastqReader::new(reader);
    let mut total_quality = 0f64;
    let mut total_len = 0;

    while let Some(rec) = fastq_reader.next() {
        info.read_count += 1;

        if info.read_count % 50000 == 0 {
            info!("Processed: {}", info.read_count)
        }

        let rec = rec.expect("Invalid record");
        let id = std::str::from_utf8(rec.id()).context("Could not convert id to string")?;
        let position = rec.position().byte() as usize;

        match extract_bc_from_header(id, re, position) {
            Ok((len, identifier)) => {
                match expected_len {
                    None => {
                        expected_len = Some(len)
                    }
                    Some(expected) => {
                        if expected != len {
                            bail!(
                                IndexGenerationErr::DifferentMatchCounts {
                                    header: id.to_string(),
                                    re: re.clone(),
                                    pos: position,
                                    count: len,
                                    expected
                                }
                            )
                        }
                    }
                }

                total_quality += write_read(wtr, &rec, &identifier, position)?;
                total_len += rec.num_bases();
                info.matched_read_count += 1;
            }
            Err(e) => {
                if !skip_invalid_ids {
                    bail!(e)
                }
                info.unmatched_read_count += 1;
            }
        };
    }

    wtr.flush()?;

    info.avg_qual = total_quality / (info.matched_read_count as f64);
    info.avg_len = (total_len as f64) / (info.matched_read_count as f64);
    info.gb = (fastq_reader.position().byte() as f64) / (1024u32.pow(3) as f64);

    Ok(info)
}

fn iter_lines_with_cluster_file<W: Write>(
    reader: BufReader<File>,
    wtr: &mut Writer<W>,
    clusters: &mut Reader<File>,
    skip_invalid_ids: bool,
    mut info: FastqFile,
) -> Result<FastqFile> {
    // first, we will read the clusters file
    info!("Reading identifiers from clusters file...");

    let mut cluster_map = std::collections::HashMap::new();

    for result in clusters.records() {
        let record = result?;

        let len = record.len();

        let read_id = record[0].to_string();
        let identifier = match record.len() {
            2 => record[1].to_string(),
            3 => format!("{}_{}", record[1].to_string(), record[2].to_string()),
            _ => bail!(InvalidClusterRow {row: record.as_slice().to_string()})
        };

        cluster_map.insert(read_id, identifier);
    }

    info!("Finished reading clusters. ");


    let mut fastq_reader = needletail::parser::FastqReader::new(reader);
    let mut total_quality = 0f64;
    let mut total_len = 0;

    while let Some(rec) = fastq_reader.next() {
        info.read_count += 1;
        if info.read_count % 50000 == 0 {
            info!("Processed: {}", info.read_count);
        }

        let rec = rec.expect("Invalid record");
        let id = std::str::from_utf8(rec.id()).context("Could not convert id to string")?;
        let position = rec.position().byte() as usize;

        let Some(identifier) = cluster_map.get(id) else {
            if !skip_invalid_ids {
                bail!(RowNotInClusters {header: id.to_string()})
            }
            info.unmatched_read_count += 1;
            continue;
        };
        info.matched_read_count += 1;

        total_quality += write_read(wtr, &rec, &identifier, position)?;
        total_len += rec.num_bases();
    }

    wtr.flush()?;

    info.avg_qual = total_quality / (info.matched_read_count as f64);
    info.avg_len = (total_len as f64) / (info.matched_read_count as f64);
    info.gb = (fastq_reader.position().byte() as f64) / (1024u32.pow(3) as f64);

    Ok(info)
}

fn extract_bc_from_header(header: &str, re: &Regex, pos: usize) -> Result<(usize, String)> {
    let Some(captures) = re.captures(header) else {
        bail!(IndexGenerationErr::NoMatch {
            header: String::from(header.trim()),
            re: re.clone(),
            pos
        });
    };
    let captures = captures.iter()
        .skip(1)
        .flatten()
        .map(|m| m.as_str())
        .collect::<Vec<_>>();

    Ok(
        (
            captures.len(),
            captures.join("_"),
        )
    )
}

pub fn construct_index(
    infile: &str,
    outfile: &str,
    barcode_regex: &str,
    skip_unmatched: bool,
    clusters: &Option<String>,
) -> Result<()> {
    // time everything!
    let now = std::time::Instant::now();

    let f = File::open(infile).expect("File could not be opened");
    let reader = BufReader::new(f);

    let file_info = FastqFile {
        nailpolish_version: crate::cli::VERSION.to_string(),
        file_path: std::fs::canonicalize(infile)?.display().to_string(),
        index_date: format!("{:?}", chrono::offset::Local::now()),
        ..FastqFile::default()
    };

    let mut temp_file = tempfile_in("./")?;
    let mut wtr = WriterBuilder::new()
        .delimiter(b'\t')
        .from_writer(&mut temp_file);

    // write headers and file information
    wtr.write_record([
        "id",
        "pos",
        "avg_qual",
        "len"
    ])?;


    let re = Regex::new(barcode_regex)?;
    let mut result = match clusters {
        None => { iter_lines_with_regex(reader, &mut wtr, &re, skip_unmatched, file_info) }
        Some(filepath) => {
            let mut cluster_rdr = csv::ReaderBuilder::new()
                .delimiter(';' as u8)
                .has_headers(false)
                .from_path(filepath)?;

            iter_lines_with_cluster_file(reader, &mut wtr, &mut cluster_rdr, skip_unmatched, file_info)
        }
    }?;


    // amount of time passed
    result.elapsed = now.elapsed().as_secs_f64();

    // report results
    if skip_unmatched {
        info!(
            "Stats: {} matched reads, {} unmatched reads, {:.1}s runtime",
            result.matched_read_count,
            result.unmatched_read_count,
            result.elapsed,
        )
    } else {
        info!("Stats: {} reads, {:.1}s runtime", result.matched_read_count, result.elapsed)
    }

    info!("Writing to {outfile}...");

    // write to actual output file
    let mut wtr_out = File::create(outfile)?;
    writeln!(wtr_out, "#{}", serde_json::to_string(&result)?)?;

    // drop the mutable write, and seek to the start so we can read
    drop(wtr);
    temp_file.seek(std::io::SeekFrom::Start(0))?;

    // copy from the temporary file into the final output file
    std::io::copy(
        &mut temp_file,
        &mut wtr_out,
    )?;

    Ok(())
}

#[derive(Error, Debug)]
enum IndexGenerationErr {
    #[error("no matches produced:
position {pos}
    `{header}`
with capture group
    {re:?}
suggestion: if some of the reads should not produce a barcode, pass the --skip-unmatched flag")]
    NoMatch { header: String, re: Regex, pos: usize },

    #[error("inconsistent identifier count:
position {pos}
    `{header}`
has {count} matches, whereas {expected} matches were expected
using capture group
    {re:?}")]
    DifferentMatchCounts {
        header: String,
        re: Regex,
        pos: usize,
        count: usize,
        expected: usize,
    },

    #[error("invalid cluster row: should be of the format
  `READ_ID;BC;UMI`
or
  `READ_ID;BC`, but instead got
{row}")]
    InvalidClusterRow {
        row: String
    },

    #[error("Row {header} of input file not present in cluster file")]
    RowNotInClusters {
        header: String
    },
}
