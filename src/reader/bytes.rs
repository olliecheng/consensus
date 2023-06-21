use std::io::{BufRead, BufReader, ErrorKind, Read};

pub type GenericBufReader = BufReader<Box<dyn Read>>;

pub struct ByteReader {
    pub reader: GenericBufReader,
    buffer: Vec<u8>,
}

#[allow(dead_code)]
impl ByteReader {
    pub fn new(reader: GenericBufReader) -> Self {
        Self {
            reader,
            buffer: Vec::new(),
        }
    }

    pub fn next_byte(&mut self) -> Option<u8> {
        let mut buf = [0];
        match self.reader.read_exact(&mut buf) {
            Err(e) => match e.kind() {
                ErrorKind::UnexpectedEof => None,
                _ => panic!("Reading a byte should not have error {}", e),
            },
            _ => Some(buf[0]),
        }
    }

    pub fn apply_on_slice_until_byte<F>(&mut self, delim: u8, f: F) -> usize
    where
        F: FnMut(&[u8]),
    {
        // clear buffer without changing capacity
        self.buffer.clear();

        let size = self
            .reader
            .read_until(delim, &mut self.buffer)
            .expect("Byte not readable");
        self.buffer.truncate(size - 1);

        self.buffer.chunks(16).for_each(f);

        size
    }

    pub fn apply_until_byte<F>(&mut self, delim: u8, mut f: F) -> usize
    where
        F: FnMut(u8),
    {
        self.apply_on_slice_until_byte(delim, |x| x.iter().for_each(|v| f(*v)))
    }

    pub fn seek_until_byte(&mut self, delim: u8) {
        self.apply_on_slice_until_byte(delim, |_| ());
    }

    pub fn read_line(&mut self, buf: &mut String) -> Result<usize, std::io::Error> {
        self.reader.read_line(buf)
    }

    pub fn read_until(&mut self, byte: u8, buf: &mut Vec<u8>) -> Result<usize, std::io::Error> {
        self.reader.read_until(byte, buf)
    }

    pub fn read_line_trim_newline(&mut self, buf: &mut Vec<u8>) -> (usize, bool) {
        match self.reader.read_until(b'\n', buf) {
            Ok(0) => panic!("Did not expect early EOF"),
            Ok(n) => {
                let last_char = buf[n - 1];
                if last_char == b'\n' {
                    // remove trailing \n
                    buf.truncate(n - 1);
                    (n, false)
                } else {
                    // end of file has been reached
                    (n, true)
                }
            }
            Err(_) => panic!("String reading quality should never fail"),
        }
    }
}
