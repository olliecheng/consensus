use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

use csv::{Reader, Writer, WriterBuilder};
use regex::Regex;

use crate::generate_index::IndexGenerationErr::{InvalidClusterRow, RowNotInClusters};
use anyhow::{bail, Context, Result};
use needletail::{FastxReader, Sequence};
use thiserror::Error;

fn iter_lines_with_regex<W: Write>(
    mut reader: BufReader<File>,
    mut wtr: Writer<W>,
    re: &Regex,
    skip_invalid_ids: bool,
) -> Result<(usize, usize)> {
    let mut position = 0usize;
    let mut count = 0usize;
    let mut stats = (0, 0);
    let mut expected_len = 0usize;

    // write headers
    wtr.write_record([
        "Identifier",
        "Position",
    ])?;

    let mut result = String::new();
    // TODO: update this to use needletail
    while let Ok(bsize) = reader.read_line(&mut result) {
        if bsize == 0 {
            // EOF has been reached
            break;
        }

        if count % 4 == 0 {
            match extract_bc_from_header(&result, re, position) {
                Ok((len, identifier)) => {
                    if expected_len == 0 {
                        expected_len = len;
                    } else if expected_len != len {
                        // this should never be happening unless optional capture groups
                        // are used in the regex
                        bail!(IndexGenerationErr::DifferentMatchCounts {
                            header: result,
                            re: re.clone(),
                            pos: position,
                            count: len,
                            expected: expected_len
                        })
                    };

                    wtr.write_record([&identifier, &position.to_string()])?;
                    stats.0 += 1;
                }
                Err(e) => {
                    if !skip_invalid_ids {
                        bail!(e)
                    }
                    stats.1 += 1;
                }
            };
        }

        count += 1;
        position += bsize;

        // reset string
        result.clear();
    }
    wtr.flush()?;

    Ok(stats)
}

fn iter_lines_with_cluster_file<W: Write>(
    reader: BufReader<File>,
    mut wtr: Writer<W>,
    clusters: &mut Reader<File>,
    skip_invalid_ids: bool,
) -> Result<(usize, usize)> {
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


    let mut stats = (0, 0);
    let mut fastq_reader = needletail::parser::FastqReader::new(reader);

    let mut count = 0usize;
    loop {
        count += 1;
        if count % 50000 == 0 {
            info!("Processed: {count}");
        }

        let position = fastq_reader.position().byte() as usize;

        let Some(r) = fastq_reader.next() else {
            break
        };

        let rec = r.expect("Invalid record");

        let id = std::str::from_utf8(rec.id()).context("Could not convert id to string")?;
        let Some(identifier) = cluster_map.get(id) else {
            if !skip_invalid_ids {
                bail!(RowNotInClusters {header: id.to_string()})
            }
            stats.1 += 1;
            continue;
        };
        stats.0 += 1;

        wtr.write_record([&identifier, &position.to_string()])?;
        stats.0 += 1;
    }
    wtr.flush()?;

    Ok(stats)
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

    let wtr = WriterBuilder::new()
        .delimiter(b'\t')
        .from_path(outfile)?;

    let re = Regex::new(barcode_regex)?;
    let result = match clusters {
        None => { iter_lines_with_regex(reader, wtr, &re, skip_unmatched) }
        Some(filepath) => {
            let mut cluster_rdr = csv::ReaderBuilder::new()
                .delimiter(';' as u8)
                .has_headers(false)
                .from_path(filepath)?;

            iter_lines_with_cluster_file(reader, wtr, &mut cluster_rdr, skip_unmatched)
        }
    }?;

    let elapsed = now.elapsed().as_secs_f32();

    if skip_unmatched {
        info!("Stats: {} matched reads, {} unmatched reads, {elapsed:.1}s runtime", result.0, result.1)
    } else {
        info!("Stats: {} reads, {elapsed:.1}s runtime", result.0)
    }

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
