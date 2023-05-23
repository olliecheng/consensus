use crate::pairings::PairingCollection;
use crate::seq::{FastQRead, Identifier, Seq};
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader, Bytes, Read};
use std::path::Path;

// TODO: refactor Reader and FastQReader into separate files, possibly reader/fastq.rs

pub trait Reader {
    fn read(&mut self) -> PairingCollection;
}

#[derive(Clone, Copy)]
pub struct Options {
    bc: usize,
    umi: usize,
}

pub struct FastQReader {
    filename: String,
    options: Options,
}

struct FastQReadIterator {
    bytes: Bytes<BufReader<File>>,
    options: Options,
    lines: u64,
    eof: bool,
}

impl FastQReadIterator {
    fn new(reader: BufReader<File>, options: Options) -> Self {
        Self {
            bytes: reader.bytes(),
            options,
            lines: 0,
            eof: false,
        }
    }
}

impl FastQReadIterator {
    fn read_next_byte(&mut self) -> Option<u8> {
        match self.bytes.next() {
            Some(x) => Some(x.expect("Reading a byte should never fail")),
            None => None,
        }
    }

    fn read_next_byte_without_eof(&mut self) -> u8 {
        self.read_next_byte()
            .expect("Found end of file, not allowed")
    }

    fn apply_n_bytes<F>(&mut self, n: usize, mut f: F)
    where
        F: FnMut(u8) -> (),
    {
        for _ in 0..n {
            let v = self.read_next_byte_without_eof();
            f(v)
        }
    }

    fn apply_until_byte_or_eof<F>(&mut self, stop_byte: u8, mut f: F) -> Option<()>
    where
        F: FnMut(u8) -> (),
    {
        loop {
            let v = self.read_next_byte()?;
            if v == stop_byte {
                break;
            }
            f(v)
        }
        Some(())
    }

    fn apply_until_byte<F>(&mut self, stop_byte: u8, mut f: F)
    where
        F: FnMut(u8) -> (),
    {
        if let None = self.apply_until_byte_or_eof(stop_byte, f) {
            panic!("Did not expect EOF");
        };
    }

    fn seek_until_byte(&mut self, stop_byte: u8) {
        self.apply_until_byte(stop_byte, |x| ())
    }

    fn read_until_byte(&mut self, stop_byte: u8) -> Vec<u8> {
        let mut result = Vec::new();
        loop {
            let v = self.read_next_byte_without_eof();
            if v == stop_byte {
                break;
            }
            result.push(v);
        }
        result
    }
}

impl Iterator for FastQReadIterator {
    type Item = Result<FastQRead, MalformedFileError>;

    fn next(&mut self) -> Option<Self::Item> {
        // TODO: clean this up, it's super ugly as is

        if self.eof {
            return None;
        }

        // first line: metadata
        // first character: @
        match self.read_next_byte() {
            Some(b'\n') | None => {
                // end of file
                self.eof = true;
                return None;
            }
            Some(b'@') => (),
            _ => panic!("Wrong character: not @ starting line {}", self.lines),
        }

        // next: barcode
        let mut bc = Seq::with_capacity(self.options.bc);
        self.apply_n_bytes(self.options.bc, |x| bc.push(x));

        // next character must be _
        assert!(
            self.read_next_byte_without_eof() == b'_',
            "{} Next character after bc must be _",
            self.lines
        );

        // next: UMI
        let mut umi = Seq::with_capacity(self.options.umi);
        self.apply_n_bytes(self.options.umi, |x| umi.push(x));

        // read the rest into metadata
        let mut metadata = String::new();
        self.apply_until_byte(b'\n', |x| metadata.push(x as char));

        // line 2: fastq sequence
        let mut seq = Seq::new();
        self.apply_until_byte(b'\n', |x| seq.push(x));

        // line 3: expect a +
        assert!(
            self.read_next_byte_without_eof() == b'+',
            "3rd line of each block should start with a + ({})",
            self.lines
        );
        self.seek_until_byte(b'\n');

        // line 4: read quality scores - for now, add to a string
        let mut qual = String::new();
        if let None = self.apply_until_byte_or_eof(b'\n', |x| qual.push(x as char)) {
            self.eof = true;
        }

        self.lines += 4;

        Some(Ok({
            FastQRead {
                metadata,
                seq,
                id: Identifier { bc, umi },
                qual,
            }
        }))
    }
}

impl Reader for FastQReader {
    fn read(&mut self) -> PairingCollection {
        let file = File::open(Path::new(&self.filename)).expect("Could not read file");
        let reader = BufReader::new(file);

        let mut collection = PairingCollection::new();

        let iter = FastQReadIterator::new(reader, self.options);
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
