use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

use csv::{Reader, Writer, WriterBuilder};
use regex::Regex;

use crate::index::IndexGenerationErr::{InvalidClusterRow, RowNotInClusters};
use anyhow::{bail, Context, Result};
use needletail::parser::SequenceRecord;
use needletail::FastxReader;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::duplicates::RecordIdentifier;
use crate::file::FastqFile;
use crate::io::Record;
use tempfile::tempfile_in;

#[derive(Debug, Serialize, Deserialize)]
pub struct IndexRecord {
    pub id: String,
    pub pos: usize,
    pub avg_qual: f64,
    pub n_bases: usize,
    pub rec_len: usize,
}

/// Writes a read record to the given CSV writer, and also returns the average PHRED quality
/// score of the read.
///
/// # Arguments
///
/// * `wtr` - A mutable reference to a CSV writer.
/// * `rec` - A reference to a `SequenceRecord` containing the read data.
/// * `identifier` - A string slice representing the identifier of the read.
/// * `position` - The position of the read.
///
/// # Returns
///
/// Returns a `Result` containing the average PHRED quality score of the read as an `f64`.
///
/// # Errors
///
/// This function will return an error if writing to the CSV writer fails.
fn write_read<W: Write>(
    wtr: &mut Writer<W>,
    rec: &SequenceRecord,
    identifier: String,
    position: usize,
) -> Result<f64> {
    let len = rec.num_bases();
    let qual: u32 = rec
        .qual()
        .expect(".fastq should not fail here")
        .iter()
        .map(|x| *x as u32)
        .sum();

    let avg_qual = (qual as f64) / (len as f64);

    // we transform the quality to a PHRED score (ASCII ! to I)
    // https://en.wikipedia.org/wiki/Phred_quality_score
    let phred_qual = avg_qual - 33f64;

    // round to 2dp
    let phred_qual = (phred_qual * 100.0).round() / 100.0;

    wtr.serialize(IndexRecord {
        id: identifier,
        pos: position,
        avg_qual: phred_qual,
        n_bases: len,
        rec_len: rec.all().len() + 1,
    })?;

    Ok(phred_qual)
}

/// Iterates over lines in a FASTQ file, extracting barcodes using a regex
/// and writing the results to a CSV writer.
///
/// # Arguments
///
/// * `reader` - A `BufReader` for the input FASTQ file.
/// * `wtr` - A mutable reference to a CSV writer.
/// * `re` - A reference to a `Regex` for extracting barcodes from read headers.
/// * `skip_invalid_ids` - A boolean indicating whether to skip invalid IDs.
/// * `info` - A mutable `FastqFile` struct containing information about the FASTQ file.
///
/// # Returns
///
/// Returns a `Result` containing an updated `FastqFile` struct which contains information about
/// the file that was just read.
///
/// # Errors
///
/// This function will return an error if reading from the FASTQ file or writing to the CSV writer fails.
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
    let mut total_quality = 0u32;
    let mut total_len = 0;

    while let Some(rec) = fastq_reader.next() {
        info.read_count += 1;

        if info.read_count % 50000 == 0 {
            info!("Processed: {}", info.read_count)
        }

        let sequence_rec = rec.expect("Invalid record");
        let position = sequence_rec.position().byte() as usize;
        let file_len = sequence_rec.all().len() + 1;
        let mut rec = Record::try_from(sequence_rec)?;

        match extract_bc_from_header(&rec.id, re, position) {
            Ok((len, identifier)) => {
                let expected_len = *expected_len.get_or_insert(len);

                if expected_len != len {
                    bail!(IndexGenerationErr::DifferentMatchCounts {
                        header: rec.id,
                        re: re.clone(),
                        pos: position,
                        count: len,
                        expected: expected_len
                    })
                }

                rec.id = identifier.to_string();

                rec.write_index(wtr, position, file_len)?;
                total_quality += rec.phred_quality_total();
                total_len += rec.len();
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

    info.avg_qual = (total_quality as f64) / (info.matched_read_count as f64);
    info.avg_len = (total_len as f64) / (info.matched_read_count as f64);
    info.gb = (fastq_reader.position().byte() as f64) / (1024u32.pow(3) as f64);

    Ok(info)
}

/// Iterates over lines in a FASTQ file, matching read identifiers with a cluster file instead of
/// a header format, and writing the results to a CSV writer.
///
/// # Arguments
///
/// * `reader` - A `BufReader` for the input FASTQ file.
/// * `wtr` - A mutable reference to a CSV writer.
/// * `clusters` - A mutable reference to a CSV reader for the cluster file.
/// * `skip_invalid_ids` - A boolean indicating whether to skip invalid IDs.
/// * `info` - A mutable `FastqFile` struct containing information about the FASTQ file.
///
/// # Returns
///
/// Returns a `Result` containing an updated `FastqFile` struct which contains information about
/// the file that was just read.
///
/// # Errors
///
/// This function will return an error if reading from the FASTQ file, reading from the cluster file,
/// or writing to the CSV writer fails.
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

        let read_id = record[0].to_string();
        let identifier = match record.len() {
            // in this case, there is just one identifier (no BC and UMI) so we read the first
            // column directly as the 'identifier'
            2 => record[1].to_string(),

            // in this case, there are two identifiers (i.e. BC and UMI) so we combine them to
            // produce an 'identifier'
            3 => format!("{}_{}", &record[1], &record[2]),

            // doesn't make sense
            _ => bail!(InvalidClusterRow {
                row: record.as_slice().to_string()
            }),
        };

        cluster_map.insert(read_id, identifier);
    }

    info!("Finished reading clusters. ");

    let mut fastq_reader = needletail::parser::FastqReader::new(reader);

    // we store the total quality and length so that we can take an average at the end
    let mut total_quality = 0u32;
    let mut total_len = 0;

    while let Some(rec) = fastq_reader.next() {
        info.read_count += 1;

        // print progress notification
        if info.read_count % 50000 == 0 {
            info!("Processed: {}", info.read_count);
        }

        let sequence_rec = rec.expect("Invalid record");
        let position = sequence_rec.position().byte() as usize;
        let file_len = sequence_rec.all().len() + 1;
        let mut rec = Record::try_from(sequence_rec)?;

        let Some(identifier) = cluster_map.get(&rec.id) else {
            if !skip_invalid_ids {
                bail!(RowNotInClusters { header: rec.id })
            }
            info.unmatched_read_count += 1;
            continue;
        };
        info.matched_read_count += 1;

        rec.id = identifier.clone();
        rec.write_index(wtr, position, file_len)?;

        total_quality += rec.phred_quality_total();
        total_len += rec.len();
    }

    wtr.flush()?;

    // compute summary statistics
    info.avg_qual = (total_quality as f64) / (info.matched_read_count as f64);
    info.avg_len = (total_len as f64) / (info.matched_read_count as f64);
    info.gb = (fastq_reader.position().byte() as f64) / (1024u32.pow(3) as f64);

    Ok(info)
}

/// Extracts barcodes from a read header using a regex pattern.
///
/// # Arguments
///
/// * `header` - A string slice representing the read header.
/// * `re` - A reference to a `Regex` for extracting barcodes from the header.
/// * `pos` - The position of the read.
///
/// # Returns
///
/// Returns a `Result` containing a tuple with the number of captures and the
/// concatenated barcode string (identifier).
///
/// # Errors
///
/// This function will return an error if the regex does not match the header.
fn extract_bc_from_header(
    header: &str,
    re: &Regex,
    pos: usize,
) -> Result<(usize, RecordIdentifier)> {
    let Some(captures) = re.captures(header) else {
        bail!(IndexGenerationErr::NoMatch {
            header: String::from(header.trim()),
            re: re.clone(),
            pos
        });
    };

    let captures = captures
        .iter()
        .skip(1)
        .flatten()
        .map(|m| m.as_str())
        .collect::<Vec<_>>();

    Ok((
        captures.len(),
        RecordIdentifier {
            head: captures[0].to_string(),
            tail: captures[1..].join("_"),
        },
    ))
}

/// Constructs an index from a FASTQ file and writes the results to an output file.
///
/// # Notes
/// This method will create a temporary file in the directory of the output file, and the OS
/// will automatically clean up this file after execution.
///
/// # Arguments
///
/// * `infile` - A string slice representing the path to the input FASTQ file.
/// * `outfile` - A string slice representing the path to the output file.
/// * `barcode_regex` - A string slice representing the regex pattern for extracting barcodes.
/// * `skip_unmatched` - A boolean indicating whether to skip unmatched reads.
/// * `clusters` - An optional string representing the path to the cluster file.
///
/// # Returns
///
/// Returns a `Result` indicating success or failure.
///
/// # Errors
///
/// This function will return an error if reading from the input file, writing to the output file,
/// or processing the data fails.
pub fn construct_index(
    infile: &str,
    outfile: &str,
    barcode_regex: &str,
    skip_unmatched: bool,
    clusters: &Option<String>,
) -> Result<()> {
    // time everything!
    let now = std::time::Instant::now();

    // open file
    let f = File::open(infile).expect("File could not be opened");
    let reader = BufReader::new(f);

    // populate with some default information
    let file_info = FastqFile {
        nailpolish_version: crate::cli::VERSION.to_string(),
        file_path: std::fs::canonicalize(infile)?.display().to_string(),
        index_date: format!("{:?}", chrono::offset::Local::now()),
        ..FastqFile::default()
    };

    // get the directory of the output file
    let mut tempfile_dir = std::path::absolute(outfile)?;
    tempfile_dir.pop();

    // create a temporary file at this directory
    let mut temp_file = tempfile_in(tempfile_dir)?;
    let mut wtr = WriterBuilder::new()
        .delimiter(b'\t')
        .from_writer(&mut temp_file);

    // write headers and file information
    // wtr.write_record([
    //     "id",
    //     "pos",
    //     "avg_qual",
    //     "n_bases",
    //     "rec_len"
    // ])?;

    // parse the file
    let re = Regex::new(barcode_regex)?;
    let mut result = match clusters {
        // no cluster file has been used
        None => iter_lines_with_regex(reader, &mut wtr, &re, skip_unmatched, file_info),

        // cluster file is being used
        Some(filepath) => {
            let mut cluster_rdr = csv::ReaderBuilder::new()
                .delimiter(b';')
                .has_headers(false)
                .from_path(filepath)?;

            iter_lines_with_cluster_file(
                reader,
                &mut wtr,
                &mut cluster_rdr,
                skip_unmatched,
                file_info,
            )
        }
    }?;

    // amount of time passed
    result.elapsed = now.elapsed().as_secs_f64();

    // report results
    if skip_unmatched {
        info!(
            "Stats: {} matched reads, {} unmatched reads, {:.1}s runtime",
            result.matched_read_count, result.unmatched_read_count, result.elapsed,
        )
    } else {
        info!(
            "Stats: {} reads, {:.1}s runtime",
            result.matched_read_count, result.elapsed
        )
    }

    info!("Writing to {outfile}...");

    // write to actual output file
    let mut wtr_out = File::create(outfile)?;
    writeln!(wtr_out, "#{}", serde_json::to_string(&result)?)?;

    // drop the mutable write, and seek to the start so we can read
    drop(wtr);
    temp_file.seek(std::io::SeekFrom::Start(0))?;

    // copy from the temporary file into the final output file
    std::io::copy(&mut temp_file, &mut wtr_out)?;

    Ok(())
}

#[derive(Error, Debug)]
enum IndexGenerationErr {
    #[error(
        "no matches produced:
position {pos}
    `{header}`
with capture group
    {re:?}
suggestion: if some of the reads should not produce a barcode, pass the --skip-unmatched flag"
    )]
    NoMatch {
        header: String,
        re: Regex,
        pos: usize,
    },

    #[error(
        "inconsistent identifier count:
position {pos}
    `{header}`
has {count} matches, whereas {expected} matches were expected
using capture group
    {re:?}"
    )]
    DifferentMatchCounts {
        header: String,
        re: Regex,
        pos: usize,
        count: usize,
        expected: usize,
    },

    #[error(
        "invalid cluster row: should be of the format
  `READ_ID;BC;UMI`
or
  `READ_ID;BC`, but instead got
{row}"
    )]
    InvalidClusterRow { row: String },

    #[error("Row {header} of input file not present in cluster file")]
    RowNotInClusters { header: String },
}
