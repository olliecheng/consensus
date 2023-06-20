use std::io::{BufRead, BufReader, ErrorKind, Read};

pub type GenericBufReader = BufReader<Box<dyn Read>>;

pub struct ByteReader {
    pub reader: GenericBufReader,
}

#[allow(dead_code)]
impl ByteReader {
    pub fn new(reader: GenericBufReader) -> Self {
        Self { reader }
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

    pub fn apply_on_slice_until_byte<F>(&mut self, delim: u8, mut f: F) -> Option<usize>
    where
        F: FnMut(&[u8], usize),
    {
        let mut read = 0;
        loop {
            let (done, used) = {
                let available = match self.reader.fill_buf() {
                    Ok(n) => n,
                    Err(e) => panic!("Byte not readable with error {:?}", e),
                };

                match memchr::memchr(delim, available) {
                    Some(i) => {
                        let length = i;

                        let chunks: usize = length / 16;
                        let remainder = length - chunks * 16;

                        for idx in 0..chunks {
                            let start_idx = idx * 16;
                            f(&available[start_idx..start_idx + 16], 16);
                        }

                        if remainder != 0 {
                            f(&available[chunks * 16..i], remainder);
                        }

                        (true, i + 1)
                        // f(&available[0..i], i);
                        // (true, i + 1)
                    }
                    None => {
                        let length = available.len();
                        let chunks: usize = length / 16;

                        for i in 0..chunks {
                            let start_idx = i * 16;
                            f(&available[start_idx..start_idx + 16], 16);
                        }

                        (false, chunks * 16)
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

    pub fn apply_until_byte<F>(&mut self, delim: u8, mut f: F) -> Option<usize>
    where
        F: FnMut(u8),
    {
        self.apply_on_slice_until_byte(delim, |x, _| x.iter().for_each(|v| f(*v)))
    }

    pub fn seek_until_byte(&mut self, delim: u8) {
        self.apply_on_slice_until_byte(delim, |_, _| ());
    }

    pub fn read_line(&mut self, buf: &mut String) -> Result<usize, std::io::Error> {
        self.reader.read_line(buf)
    }

    pub fn read_until(&mut self, byte: u8, buf: &mut Vec<u8>) -> Result<usize, std::io::Error> {
        self.reader.read_until(byte, buf)
    }

    pub fn read_line_trim_newline(&mut self, buf: &mut String) -> (usize, bool) {
        match self.read_line(buf) {
            Ok(0) => panic!("Did not expect early EOF"),
            Ok(n) => {
                let last_char = &buf[n - 1..];
                if last_char == "\n" {
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
