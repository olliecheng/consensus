use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use needletail::{Sequence, FastxReader};

use rand::Rng;

use lsh_rs::prelude::*;

use itertools::Itertools;
use murmur3::murmur3_32;
use std::io::Cursor;
use serde::{Deserialize, Serialize};
use crate::record::Record;

#[derive(Serialize, Deserialize)]
pub struct Index {
    pub records: Vec<Record>,
    pub lsh: crate::hash::MinHashLSH,
}

fn iter_lines<W: Write>(reader: BufReader<File>, wtr: W) {
    let mut fastq_reader = needletail::parser::FastqReader::new(reader);
    let mut records = Vec::new();

    let subseq_size = 100;
    let shingle_size = 8;
    let dim = subseq_size - shingle_size + 1;
    let mut lsh = crate::hash::MinHashLSH::new(8, 10, dim);

    loop {
        let position = fastq_reader.position().byte() as usize;

        if let Some(r) = fastq_reader.next() {
            let rec = r.expect("Invalid record");

            let id = rec.id();
            let id_obj = crate::record::RecordIdentifier {
                bc: String::from_utf8((&id[1..18]).to_vec()).unwrap(),
                umi: String::from_utf8((&id[18..31]).to_vec()).unwrap(),
            };

            let qual = rec.qual().expect(".fastq must have quality");
            let avg_qual = (
                qual.iter().map(|x| *x as u64).sum::<u64>() as f64
            ) / (
                qual.len() as f64
            );
            let seq = rec.raw_seq();

            let rec_elem = Record {
                id: id_obj,
                read_id: Vec::from(seq),
                loc: position,
                avg_qual,
                qual: Vec::from(rec.qual().expect(".fastq must have quality")),
            };

            records.push(rec_elem);

            if seq.len() > subseq_size {
                let subset = &seq[..subseq_size];
                lsh.store(subset, records.len() - 1);
            }
        } else {
            break;
        }
    }

    {
        // print summary statistics
        println!("Hash table statistics: ");
        for table in lsh.hash_tables {
            println!("Table: avg {} / {}", table.values().map(|x| x.len()).sum::<usize>(), table.len());
        }
    }

    let index = Index {
        records,
        lsh,
    };

    // dump LSH
    bincode::serialize_into(wtr, &index).unwrap();
}


pub fn construct_index(infile: &str, outfile: &str) {
    let f = File::open(infile).expect("File could not be opened");
    let reader = BufReader::new(f);

    let wtr = std::io::BufWriter::new(File::create(outfile).unwrap());
    iter_lines(reader, wtr);
}
