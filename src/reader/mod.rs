mod bytes;
pub mod fastq;

use crate::options::Options;

use crate::pairings::PairingCollection;
use crate::seq::{FastQRead, Identifier, Seq};
use memchr::memchr;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader, Bytes, ErrorKind, Read};
use std::path::Path;

pub trait Reader<T> {
    fn read(&mut self) -> Box<dyn Iterator<Item = T>>;
}
