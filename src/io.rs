use crate::duplicates::{DuplicateMap, RecordIdentifier, RecordPosition};
use anyhow::Context;
use needletail::parser::SequenceRecord;
use needletail::{parser::FastqReader, FastxReader};
use std::fmt::Write as FmtWrite;
// needed for write! to be implemented on Strings
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom, Write};

#[derive(PartialEq, Eq)]
pub enum ReadType {
    Consensus,
    Single,
    Original,
    Ignored,
}

#[derive(Clone)]
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

impl Record {
    pub fn write_fastq(&self, writer: &mut impl Write) -> Result<(), std::io::Error> {
        write!(writer, "@{}\n{}\n+\n{}", self.id, self.seq, self.qual)
    }

    pub fn write_fasta(&self, writer: &mut impl Write) -> Result<(), std::io::Error> {
        write!(writer, ">{}\n{}", self.id, self.seq)
    }

    /// Adds metadata to the record identifier through an in-place modify.
    ///
    /// # Arguments
    ///
    /// * `umi_group` - The UMI group identifier.
    /// * `read_type` - The type of read (Consensus, Original, Ignored).
    /// * `group_idx` - The index of the read in the group.
    /// * `group_size` - The size of the group.
    /// * `avg_qual` - The average quality score of the group.
    pub fn add_metadata(
        &mut self,
        umi_group: usize,
        read_type: ReadType,
        group_idx: usize,
        group_size: usize,
        avg_qual: f64,
    ) {
        let read_type_label = match read_type {
            ReadType::Consensus => &format!("CON_{group_size}"),
            ReadType::Single => "SIN",
            ReadType::Original => &format!("ORIG_{group_idx}_OF_{group_size}"),
            ReadType::Ignored => "IGN",
        };

        // safe to unwrap because this never returns an error
        //   https://github.com/rust-lang/rust/blob/1.47.0/library/alloc/src/string.rs#L2414-L2427
        // ">{} UG:i:{} BX:Z:{} UT:Z:{}_{}\n{}",
        write!(self.id, " UT:Z:{read_type_label} UG:i:{umi_group}")
            .expect("String writing should not error");

        // don't report the group average quality if the readtype is Original or Ignored
        if !matches!(read_type, ReadType::Original | ReadType::Ignored) {
            write!(self.id, " QL:f:{avg_qual:.2}").expect("String writing should not error");
        }
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
    /// Whether we should NOT consensus call this UMI group, because of quality/other issues
    pub ignore: bool,
    pub consensus: Option<Record>,
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
    reader
        .seek(SeekFrom::Start(pos.pos as u64))
        .with_context(|| format!("Unable to seek file at position {}", pos.pos))?;

    // read the exact number of bytes
    // let mut bytes = Vec::with_capacity(pos.length);
    let mut bytes = vec![0; pos.length];
    reader.read_exact(&mut bytes).with_context(|| {
        format!(
            "Could not read {} lines at position {}",
            pos.length, pos.pos
        )
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
) -> anyhow::Result<impl Iterator<Item = anyhow::Result<UMIGroup>> + '_> {
    // we create two readers, one for sequential access and one for random
    let file = File::open(input).with_context(|| format!("Unable to open file {input}"))?;

    // capacity of 128 KiB
    let mut file_sequential = BufReader::with_capacity(128 * 1024, file);

    // no need for a buffered reader here, as we are only doing random access
    let mut file_random =
        File::open(input).with_context(|| format!("Unable to open file {input}"))?;

    Ok(duplicates
        .into_iter()
        .enumerate()
        // first, read from the file (sequential)
        .filter_map(move |(index, (id, positions))| {
            process_read_from_position(
                index,
                id,
                positions,
                duplicates_only,
                &mut file_sequential,
                &mut file_random,
            )
        }))
}

/// Reads duplicate groups from a list of read positions, returning a UMIGroup
///
/// # Arguments
///
/// * `index` - The index of the UMI group.
/// * `id` - The identifier of the UMI group.
/// * `positions` - A vector of positions where the reads are located.
/// * `duplicates_only` - A boolean indicating whether to process only duplicate reads.
/// * `reader_seq` - A mutable reference to a sequential reader.
/// * `reader_rnd` - A mutable reference to a random access reader.
///
/// # Returns
///
/// This function returns an `Option` containing a `Result<UMIGroup>`.
/// The return value is `None` if we wish to skip over this duplicate group; for instance,
/// if it is requested that single reads are skipped.
fn process_read_from_position<R1, R2>(
    index: usize,
    id: RecordIdentifier,
    positions: Vec<RecordPosition>,
    duplicates_only: bool,
    reader_seq: &mut R1,
    reader_rnd: &mut R2,
) -> Option<anyhow::Result<UMIGroup>>
where
    R1: Read + Seek + Send,
    R2: Read + Seek + Send,
{
    let single = positions.len() == 1;
    // if this UMI group is a single AND we only want to output duplicates,
    // this is skipped over
    if single && duplicates_only {
        return None;
    }

    let mut rec = UMIGroup {
        id,
        index,
        records: Vec::new(),
        avg_qual: 0.0,
        ignore: false,
        consensus: None,
    };

    let mut total_qual = 0u32;
    let mut total_bp = 0usize;

    for pos in positions.iter() {
        if pos.length > 30000 {
            rec.ignore = true;
        }

        let read = if single {
            get_read_at_position(reader_seq, pos)
        } else {
            get_read_at_position(reader_rnd, pos)
        };

        match read {
            Ok(record) => {
                total_qual += record
                    .qual
                    .as_bytes()
                    .iter()
                    .map(|x: &u8| *x as u32 - 33)
                    .sum::<u32>();
                total_bp += record.qual.len();
                rec.records.push(record)
            }
            Err(e) => return Some(Err(e)),
        }
    }

    rec.avg_qual = (total_qual as f64) / (total_bp as f64);

    Some(Ok(rec))
}
