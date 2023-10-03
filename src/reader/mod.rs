mod bytes;
pub mod fastq;

use crate::options::Cli;
use std::io::Read;

pub trait Reader<T> {
    fn read(&self) -> Box<dyn Iterator<Item = T>>;

    fn read_file(filename: &str, options: Cli) -> Box<dyn Iterator<Item = T>>;
    fn read_from_reader(reader: Box<dyn Read>, options: Cli) -> Box<dyn Iterator<Item = T>>;
}
