use crate::duplicates::DuplicateMap;
use bio::io::fastq;
use bio::io::fastq::{FastqRead, Reader};
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::io::{Seek, SeekFrom};

pub fn consensus(
    input: &str,
    output: &str,
    duplicates: DuplicateMap,
) -> Result<(), Box<dyn Error>> {
    let mut file = File::open(input)?;

    for (id, positions) in duplicates.iter() {
        if positions.len() == 1 {
            // TODO: do something here later
            continue;
        }
        let sequences = Vec::<fastq::Record>::new();

        eprintln!("{positions:?}");
        for pos in positions.iter() {
            let mut buffer = [0; 100];
            let mut record = fastq::Record::new();
            file.seek(SeekFrom::Start(*pos as u64)).unwrap();
            // file.read_exact(&mut buffer)?;

            // eprintln!("{:?}", String::from_utf8(buffer.to_vec()).unwrap());

            let mut reader = fastq::Reader::new(&mut file);
            reader.read(&mut record).unwrap();
            eprintln!("{}", record.id());
        }
        // println!("key: {key:?}, val: {val:?}");
    }

    return Ok(());
}
