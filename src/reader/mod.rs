use crate::pairings::PairingCollection;
use crate::seq::{FastQRead, Identifier, Seq};
use memchr::memchr;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader, Bytes, ErrorKind, Read};
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
    bytes: ByteReader,
    options: Options,
    lines: u64,
    eof: bool,
}

impl FastQReadIterator {
    fn new(reader: BufReader<File>, options: Options) -> Self {
        Self {
            bytes: ByteReader::new(reader),
            options,
            lines: 0,
            eof: false,
        }
    }
}

impl FastQReadIterator {
    fn read_next_byte(&mut self) -> Option<u8> {
        self.bytes.next_byte()
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

    fn apply_until_byte_or_eof<F>(&mut self, stop_byte: u8, f: F) -> Option<()>
    where
        F: FnMut(u8) -> (),
    {
        match self.bytes.apply_until_byte(stop_byte, f) {
            Some(_) => Some(()),
            None => None,
        }
    }

    fn apply_until_byte<F>(&mut self, stop_byte: u8, f: F)
    where
        F: FnMut(u8) -> (),
    {
        let result = self.bytes.apply_until_byte(stop_byte, f);
        assert!(result != None, "Did not expect EOF");
    }

    fn seek_until_byte(&mut self, stop_byte: u8) {
        self.bytes.apply_until_byte(stop_byte, |_| ());
    }

    fn read_until_byte(&mut self, stop_byte: u8) -> Vec<u8> {
        let mut result = Vec::new();
        self.bytes
            .reader
            .read_until(stop_byte, &mut result)
            .expect("Reading should not fail");
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
        match self.bytes.reader.read_line(&mut metadata) {
            Ok(0) => panic!("Did not expect early EOF"),
            Ok(n) => {
                let last_char = &metadata[n - 1..];
                assert!(
                    last_char == "\n",
                    "Last character of metadata should be \n, got {}",
                    last_char
                );
                // remove trailing \n
                metadata.truncate(n - 1);
            }
            Err(_) => panic!("String reading metadata should never fail"),
        };

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
        match self.bytes.reader.read_line(&mut qual) {
            Ok(0) => panic!("Did not expect early EOF"),
            Ok(n) => {
                let last_char = &qual[n - 1..];
                if last_char == "\n" {
                    // remove trailing \n
                    qual.truncate(n - 1);
                } else {
                    // end of file has been reached
                    self.eof = true;
                }
            }
            Err(_) => panic!("String reading quality should never fail"),
        };

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

const BYTE_READER_BUFFER_SIZE: usize = 512;

struct ByteReader {
    pub reader: BufReader<std::fs::File>,
    buffer: [u8; BYTE_READER_BUFFER_SIZE],
    idx: usize,
    buffer_size: usize,
}

impl ByteReader {
    fn new(reader: BufReader<std::fs::File>) -> Self {
        Self {
            reader,
            buffer: [0; BYTE_READER_BUFFER_SIZE],
            idx: BYTE_READER_BUFFER_SIZE,
            buffer_size: BYTE_READER_BUFFER_SIZE,
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        let mut buf = [0];

        match self.reader.read_exact(&mut buf) {
            Err(e) => match e.kind() {
                ErrorKind::UnexpectedEof => None,
                _ => panic!("Reading a byte should not have error {}", e),
            },
            _ => Some(buf[0]),
        }
    }

    fn apply_on_slice_until_byte<F>(&mut self, delim: u8, mut f: F) -> Option<usize>
    where
        F: FnMut(&[u8]) -> (),
    {
        let mut read = 0;
        loop {
            let (done, used) = {
                let available = match self.reader.fill_buf() {
                    Ok(n) => n,
                    Err(_) => panic!("Byte not readable"),
                };

                match memchr::memchr(delim, available) {
                    Some(i) => {
                        f(&available[0..i]);
                        (true, i + 1)
                    }
                    None => {
                        let length = available.len();
                        f(&available[..]);
                        (false, length)
                    }
                }
            };

            self.reader.consume(used);
            read += used;

            if done {
                return Some(read);
            } else if used == 0 {
                return None;
            }
        }
    }

    // modified from https://doc.rust-lang.org/src/std/io/mod.rs.html#1910-1936
    // Returns None if EOF has been reached
    fn apply_until_byte<F>(&mut self, delim: u8, mut f: F) -> Option<usize>
    where
        F: FnMut(u8) -> (),
    {
        self.apply_on_slice_until_byte(delim, |x| x.iter().for_each(|v| f(*v)))
    }
}
