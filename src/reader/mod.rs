use crate::pairings::PairingCollection;
use crate::seq::{FastQRead, Identifier, Seq};
use std::fmt;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
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

type BaseIterator = Box<dyn Iterator<Item = Result<String, std::io::Error>>>;

struct FastQReadIterator {
    reader: BufReader<File>,
    options: Options,
    lines: u64,
    eof: bool,
}

impl FastQReadIterator {
    fn new(reader: BufReader<File>, options: Options) -> Self {
        Self {
            reader,
            options,
            lines: 0,
            eof: false,
        }
    }
}

impl FastQReadIterator {
    fn read_next_byte(&mut self, v: &mut [u8]) -> Result<(), std::io::Error> {
        self.reader.read_exact(v)
    }

    fn read_next_byte_and_assert(&mut self, v: &mut [u8], b: u8) {
        self.read_next_byte(v).expect(&format!(
            "Reading a single byte should never fail. {}",
            self.lines,
        ));

        assert!(
            v == [b],
            "{} Single character error: {} != {}",
            self.lines,
            v[0],
            b
        );
    }

    fn read_bytes(&mut self, n: usize) -> Vec<u8> {
        let mut v = vec![0u8; n];
        self.reader.read_exact(&mut v).expect(&format!(
            "Reading a fixed size chunk of size {} should never fail. {}",
            n, self.lines
        ));
        v
    }
}

impl Iterator for FastQReadIterator {
    type Item = Result<FastQRead, MalformedFileError>;

    fn next(&mut self) -> Option<Self::Item> {
        // TODO: clean this up, it's super ugly as is
        let mut temp_c = [0];

        if self.eof {
            return None;
        }

        // first line: metadata
        // first character: @
        match self.read_next_byte(&mut temp_c) {
            Err(_) => {
                // end of file was reached, we should return None
                self.eof = true;
                return None;
            }
            _ => (),
        }
        match temp_c {
            [b'\n'] => {
                // end of file was reached
                self.eof = true;
                return None;
            }
            [b'@'] => (),
            _ => panic!("{}: Expected @ or \\n, got {}", self.lines, temp_c[0]),
        }

        // next: barcode
        let mut bc = Seq::with_capacity(self.options.bc);
        bc.add_iter(self.read_bytes(self.options.bc).into_iter());

        // next character: _
        self.read_next_byte_and_assert(&mut temp_c, b'_');

        // next: UMI
        let mut umi = Seq::with_capacity(self.options.umi);
        umi.add_iter(self.read_bytes(self.options.umi).into_iter());

        // read the rest into metadata
        let mut metadata = String::new();
        match self.reader.read_line(&mut metadata) {
            Ok(0) => panic!(
                "Metadata: File should not end abruptly, line {}",
                self.lines
            ),
            Err(_) => panic!("Metadata: Reading should not fail, line {}", self.lines),
            _ => {}
        };
        let metadata = metadata.trim_end().to_string();

        // line 2: fastq sequence
        let mut seq = Seq::new();
        for b in self.reader.by_ref().bytes() {
            let b = b.expect("Seq: Reading a byte should never fail");
            if b == b'\n' {
                break;
            }

            seq.push(b);
        }
        // line 3: expect a +
        self.read_next_byte_and_assert(&mut temp_c, b'+');
        // read bytes until newline
        for b in self.reader.by_ref().bytes() {
            if b.expect("+: Reading a byte should never fail") == b'\n' {
                break;
            }
        }

        // line 4: read quality scores - for now, add to a string
        let mut qual = String::new();
        match self.reader.read_line(&mut qual) {
            Ok(0) => self.eof = true,
            Err(_) => panic!("Qual: Reading should not fail, line {}", self.lines),
            _ => {}
        };
        let qual = qual.trim_end().to_string();

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
