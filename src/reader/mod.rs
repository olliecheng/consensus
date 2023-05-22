use crate::pairings::PairingCollection;
use crate::seq::{Identifier, ReadWithQual, Seq};
use std::fmt;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

// TODO: refactor Reader and FastQReader into separate files, possibly reader/fastq.rs

pub trait Reader {
    fn read(&mut self) -> PairingCollection;
}

pub struct Options {
    bc: usize,
    umi: usize,
}

pub struct FastQReader {
    filename: String,
    options: Options,
}

type BaseIterator = Box<dyn Iterator<Item = Result<String, std::io::Error>>>;

struct FastQReadIterator<F>
where
    F: Fn(&String) -> Identifier,
{
    iter: BaseIterator,
    parser: F,
    lines: u64,
}

impl<F> FastQReadIterator<F>
where
    F: Fn(&String) -> Identifier,
{
    fn new(iter: BaseIterator, parser: F) -> Self {
        Self {
            iter,
            parser,
            lines: 0,
        }
    }

    fn parser(&self, metadata: &String) -> Identifier {
        (self.parser)(metadata)
    }
}

impl<F> Iterator for FastQReadIterator<F>
where
    F: Fn(&String) -> Identifier,
{
    type Item = Result<ReadWithQual, MalformedFileError>;

    fn next(&mut self) -> Option<Self::Item> {
        // TODO: clean this up, it's super ugly as is
        // maybe abstract away read_record()?

        let metadata = self.iter.next();
        let seq = self.iter.next();
        let empty = self.iter.next();
        let qual = self.iter.next();

        if metadata.is_none() {
            // end of read was reached in a well-formatted fastq file
            return None;
        } else if seq.is_none() || empty.is_none() || qual.is_none() {
            // end of read was reached
            // return a result, but make the result an error
            return Some(Err(MalformedFileError {
                description: "File ends with a truncated read. Check your fastq file is well formed. The final read has been ignored.".to_string(),
            }));
        };

        let metadata = metadata.unwrap().unwrap();
        let seq = seq.unwrap().unwrap();
        let qual = qual.unwrap().unwrap();
        let empty = empty.unwrap().unwrap();

        // Perform checks on fastq file
        if metadata.trim().is_empty() {
            return None;
        }
        assert!(
            empty.starts_with("+"),
            "block starting line {} has no starting +",
            self.lines + 1,
        );

        let id = self.parser(&metadata);

        self.lines += 4;

        Some(Ok({
            ReadWithQual {
                metadata,
                seq: Seq::new(&seq),
                id,
                qual,
            }
        }))
    }
}

impl Reader for FastQReader {
    fn read(&mut self) -> PairingCollection {
        let file = File::open(Path::new(&self.filename)).expect("Could not read file");
        let mut collection = PairingCollection::new();
        let lines = io::BufReader::new(file).lines();

        let iter = FastQReadIterator::new(Box::new(lines), |m| self.metadata_parser(m));
        for x in iter {
            let read = x.expect("Error reading.");
            // println!("Hello {:?}", read);
            collection.add_read(read.id, read.seq);
        }
        collection
    }
}

impl FastQReader {
    pub fn new(filename: impl Into<String>) -> Self {
        Self {
            filename: filename.into(),
            options: Options { bc: 16, umi: 12 },
        }
    }

    fn metadata_parser(&self, metadata: &String) -> Identifier {
        assert!(metadata.starts_with('@'));

        // extract barcode
        let bc = &metadata[1..self.options.bc + 1];
        let umi = &metadata[self.options.bc + 2..(self.options.bc + 2 + self.options.umi)];

        Identifier {
            bc: Seq::new(bc),
            umi: Seq::new(umi),
        }
    }
}

#[derive(Debug)]
pub struct MalformedFileError {
    description: String,
}

impl std::error::Error for MalformedFileError {}
impl fmt::Display for MalformedFileError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description)
    }
}
