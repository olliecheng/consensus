use crate::duplicates::{DuplicateMap, RecordIdentifier, RecordPosition};
use anyhow::Context;
use needletail::parser::SequenceRecord;
use needletail::{parser::FastqReader, FastxReader};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom, Write};

pub enum ReadType {
    Consensus,
    Original,
}

pub struct Record {
    pub id: String,
    pub seq: String,
    pub qual: String,
}

impl TryFrom<SequenceRecord<'_>> for Record {
    type Error = std::string::FromUtf8Error;

    fn try_from(rec: SequenceRecord) -> Result<Self, Self::Error> {
        Ok(Record {
            id: String::from_utf8(rec.id().to_vec())?,
            seq: String::from_utf8(rec.seq().to_vec())?,
            qual: String::from_utf8(rec.qual().unwrap_or(&[]).to_vec())?,
        })
    }
}

pub struct UMIGroup {
    /// The "Identifier" of this group, typically a "BC_UMI" string
    pub id: RecordIdentifier,
    /// A 0-indexed integer unique to each UMI group
    pub index: usize,
    /// Each individual record within the UMI group
    pub records: Vec<Record>,
    /// The average PHRED quality of the UMI group
    pub avg_qual: f64,
}

/// Retrieves a FASTQ record from a file at a specified position.
///
/// # Errors
///
/// This function will return an error if:
/// * The file cannot be seeked to the specified position, where position is the start byte
///   of the required read.
/// * The FASTQ reader encounters an unexpected EOF.
/// * The record ID, sequence, or quality scores cannot be converted from UTF-8.
///
/// # Example
///
/// ```
/// use std::fs::File;
/// use anyhow::Result;
///
/// let mut file = File::open("example.fastq")?;
/// let record = get_read_at_position(&mut file, 12345)?;
/// println!("Record ID: {}", record.id);
/// ```
pub fn get_read_at_position<R: Read + Seek + Send>(
    // file: &mut File,
    reader: &mut R,
    // file_sequential: &mut dyn Read,
    // file_random: &mut dyn Read,
    pos: &RecordPosition,
) -> anyhow::Result<Record> {
    // go to the position of the record
    reader.seek(SeekFrom::Start(pos.pos as u64)).with_context(|| {
        format!("Unable to seek file at position {}", pos.pos)
    })?;

    // read the exact number of bytes
    // let mut bytes = Vec::with_capacity(pos.length);
    let mut bytes = vec![0; pos.length];
    reader.read_exact(&mut bytes).with_context(|| {
        format!("Could not read {} lines at position {}", pos.length, pos.pos)
    })?;

    // eprintln!("fastq: {}", std::str::from_utf8(&bytes)?);

    // create a needletail 'reader' with the file at this location
    let mut fq_reader = FastqReader::new(&bytes[..]);

    let rec = fq_reader.next().context("Unexpected EOF")??;

    Record::try_from(rec).context("Could not perform utf8 conversions")
}


/// Iterates over records in a FASTQ file by UMI group.
///
/// # Returns
/// This function returns an iterator of Results. When an Error is encountered,
/// the caller should immediately stop. See the documentation for `until_err` to see an example.
///
/// # Errors
///
/// This function will return an error if:
/// * The file cannot be opened.
/// * The record cannot be read at the specified position.
///
/// The iterator yields Some(Err) if:
/// * There are issues reading the read at at the specified position. See the documentation for
///   `get_read_at_position` for more.
pub fn iter_duplicates(
    input: &str,
    duplicates: DuplicateMap,
    duplicates_only: bool,
) -> anyhow::Result<impl Iterator<Item=anyhow::Result<UMIGroup>> + '_> {
    // we create two readers, one for sequential access and one for random
    let file = File::open(input).with_context(|| format!("Unable to open file {input}"))?;

    // capacity of 128 KiB
    let mut file_sequential = BufReader::with_capacity(128 * 1024, file);

    // no need for a buffered reader here, as we are only doing random access
    let mut file_random = File::open(input).with_context(|| format!("Unable to open file {input}"))?;

    Ok(duplicates
        .into_iter()
        .enumerate()
        // first, read from the file (sequential)
        .filter_map(move |(index, (id, positions))| -> Option<anyhow::Result<UMIGroup>> {
            let single = positions.len() == 1;
            // if this UMI group is a single AND we only want to output duplicates,
            // this is skipped over
            if single && duplicates_only {
                return None;
            }

            let mut rec = UMIGroup { id, index, records: Vec::new(), avg_qual: 0.0 };
            let mut total_qual = 0u32;

            for pos in positions.iter() {
                if pos.length > 30000 {
                    eprintln!(
                        "Skipping at position {} due to length {}",
                        pos.pos,
                        pos.length
                    );
                    continue;
                }

                let read = if single {
                    get_read_at_position(&mut file_sequential, pos)
                } else {
                    get_read_at_position(&mut file_random, pos)
                };

                match read {
                    Ok(record) => {
                        total_qual += record.qual
                            .as_bytes()
                            .iter()
                            .map(|x: &u8| *x as u32)
                            .sum::<u32>();
                        rec.records.push(record)
                    }
                    Err(e) => { return Some(Err(e)) }
                }
            }

            rec.avg_qual = (total_qual as f64) / (positions.len() as f64);

            Some(Ok(rec))
        }))
}

/// Utility function to extract the error from an iterator and stop iteration immediately. Useful
/// for iterators which yield a Result<T>.
///
/// # Returns
///
/// This function returns an `Option<T>`. If the item is `Ok`, it returns `Some(T)`.
/// If the item is `Err`, it updates `err` with the error and returns `None`.
///
/// # Example
/// ```
/// let mut err = Ok(());
/// let items = vec![Ok(1), Ok(2), Err(anyhow!("error")), Ok(3)];
/// let results: Vec<_> = items
///   .into_iter()
///   .scan(&mut err, until_err)
///   .collect();
/// assert_eq!(results, vec![1, 2]);
/// assert!(err.is_err());
/// ```
pub fn until_err<T>(err: &mut &mut anyhow::Result<()>, item: anyhow::Result<T>) -> Option<T> {
    match item {
        Ok(item) => Some(item),
        Err(e) => {
            **err = Err(e);
            None
        }
    }
}

/// Formats a record and group into a valid FASTQ read and writes it to the provided writer.
///
/// # Arguments
///
/// * `writer` - A mutable reference to an object that implements the `Write` trait. Since this is
///   `std::io::Write` and *not* `std::fmt::Write`, this does not accept Strings. It is acceptable
///   to use a `std::io::Cursor` instead.
/// * `record` - A reference to the `Record` struct containing the read information.
/// * `group` - A reference to the `UMIGroup` struct containing the group information.
/// * `read_type` - The type of read, either `CONSENSUS` or `ORIGINAL`.
/// * `fastq` - A boolean indicating whether to format the output as FASTQ.
///
/// # Returns
///
/// This function returns a `std::io::Result<()>` indicating the success or failure of the write operation.
pub fn write_read(
    writer: &mut impl Write,
    record: &Record,
    group: &UMIGroup,
    read_type: ReadType,
    fastq: bool,
) -> std::io::Result<()> {
    let read_type_label = match read_type {
        ReadType::Consensus => { "CONSENSUS" }
        ReadType::Original => { "ORIGINAL" }
    };

    if fastq {
        writeln!(
            writer,
            ">{} UG:i:{} BX:Z:{} UT:Z:{}_{}\n{}\n+\n{}",
            record.id,
            group.index,
            group.id,
            read_type_label,
            group.records.len(),
            record.seq,
            record.qual
        )
    } else {
        writeln!(
            writer,
            ">{} UG:i:{} BX:Z:{} UT:Z:{}_{}\n{}",
            record.id,
            group.index,
            group.id,
            read_type_label,
            group.records.len(),
            record.seq
        )
    }
}