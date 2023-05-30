use super::bytes;
use crate::seq;
use std::fs::File;
use std::io::{BufRead, BufReader, Bytes, ErrorKind, Read};

#[derive(Debug)]
pub struct Record {
    pub id: seq::Identifier,
    pub metadata: String,
    pub seq: seq::Seq,
    pub qual: seq::Qual,
}

pub struct FastQReader {
    filename: String,
    options: crate::options::Options,
}

impl FastQReader {
    pub fn new(filename: String) -> Self {
        Self {
            filename,
            options: crate::options::Options { bc: 16, umi: 12 },
        }
    }
}

impl super::Reader<Record> for FastQReader {
    fn read(&mut self) -> Box<dyn Iterator<Item = Record>> {
        let file = File::open(std::path::Path::new(&self.filename)).expect("Could not read file");
        let reader = BufReader::new(file);

        return Box::new(FastQReadIterator::new(reader, self.options));
    }
}

pub struct FastQReadIterator {
    bytes: bytes::ByteReader,
    options: crate::options::Options,
    lines: u64,
    eof: bool,
}

impl FastQReadIterator {
    pub fn new(reader: BufReader<File>, options: crate::options::Options) -> Self {
        Self {
            bytes: bytes::ByteReader::new(reader),
            options,
            lines: 0,
            eof: false,
        }
    }

    fn read_n_to_seq(&mut self, n: usize, seq: &mut seq::Seq) {
        for _ in 0..n {
            let v = self.bytes.next_byte().expect("Found EOF, not allowed");
            seq.push(v)
        }
    }
}

impl Iterator for FastQReadIterator {
    type Item = Record;

    fn next(&mut self) -> Option<Self::Item> {
        if self.eof {
            return None;
        }

        match self.bytes.next_byte() {
            Some(b'\n') | None => {
                // end of file
                self.eof = true;
                return None;
            }
            Some(b'@') => (),
            _ => panic!("Wrong character: not @ starting line {}", self.lines),
        }

        // read barcode
        let mut bc = seq::Seq::with_capacity(self.options.bc);
        self.read_n_to_seq(self.options.bc, &mut bc);

        // next character must be _
        assert!(
            self.bytes.next_byte().expect("Expected _, not EOF") == b'_',
            "{} Next character after bc must be _",
            self.lines
        );

        // next: UMI
        let mut umi = seq::Seq::with_capacity(self.options.umi);
        self.read_n_to_seq(self.options.umi, &mut umi);

        // read metadata
        let mut metadata = String::new();
        let (_, eof) = self.bytes.read_line_trim_newline(&mut metadata);
        if eof {
            panic!("Metadata should not contain EOF");
        }

        // line 2: fastq sequence
        let mut seq = seq::Seq::new();
        self.bytes.apply_until_byte(b'\n', |x| seq.push(x));

        // line 3: expect a +
        assert!(
            self.bytes.next_byte().expect("Expected + not EOF") == b'+',
            "3rd line of each block should start with a + ({})",
            self.lines
        );
        self.bytes.seek_until_byte(b'\n');

        // line 4: read quality scores - for now, add to a string
        let mut qual = String::new();

        let (_, eof) = self.bytes.read_line_trim_newline(&mut qual);
        if eof {
            self.eof = true;
        }

        self.lines += 4;

        Some(Record {
            metadata,
            seq,
            id: seq::Identifier { bc, umi },
            qual,
        })
    }
}
