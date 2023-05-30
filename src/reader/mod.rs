mod bytes;
pub mod fastq;
use std::io::Read;

pub trait Reader<T> {
    fn read(&mut self) -> Box<dyn Iterator<Item = T>>;

    fn read_file(filename: &str) -> Box<dyn Iterator<Item = T>>;
    fn read_from_reader(reader: Box<dyn Read>) -> Box<dyn Iterator<Item = T>>;
}
