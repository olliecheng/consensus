use super::PairingCollection;
use crate::reader::{self, Reader};

impl PairingCollection {
    pub fn from_reader_fastq(reader: &mut reader::fastq::FastQReader) -> Self {
        let mut collection = Self::new();

        for read in reader.read() {
            collection.add_read(read.id, read.seq);
        }

        collection
    }
}
