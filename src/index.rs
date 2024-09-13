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
    pub sorted_indices: Vec<IndexPosition>,
    pub lsh: crate::hash::MinHashLSH,
}

#[derive(Serialize, Deserialize, Eq, PartialEq)]
pub enum IndexPosition {
    Removed,
    Present(usize),
}

fn iter_lines<W: Write>(reader: BufReader<File>, wtr: W) {
    let mut fastq_reader = needletail::parser::FastqReader::new(reader);
    let mut records = Vec::new();

    let subseq_size = 100;
    let shingle_size = 8;
    let dim = subseq_size - shingle_size + 1;
    let mut lsh = crate::hash::MinHashLSH::new(8, 30, dim);

    loop {
        let position = fastq_reader.position().byte() as usize;

        let Some(r) = fastq_reader.next() else {
            break
        };

        let rec = r.expect("Invalid record");

        let id = rec.id();
        let id_obj = crate::record::RecordIdentifier {
            bc: String::from_utf8((&id[0..16]).to_vec()).unwrap(),
            umi: String::from_utf8((&id[17..29]).to_vec()).unwrap(),
        };

        let qual = rec.qual().expect(".fastq must have quality");
        let avg_qual = (
            qual.iter().map(|x| *x as u64).sum::<u64>() as f64
        ) / (
            qual.len() as f64
        );

        let seq = rec.raw_seq();

        let mut rec_elem = Record {
            id: id_obj,
            loc: position,
            avg_qual,
            hash: None,
        };

        if seq.len() > subseq_size {
            let subset = &seq[..subseq_size];
            rec_elem.hash = Some(lsh.store(subset, records.len()));
        }

        records.push(rec_elem);
    }

    {
        // print summary statistics
        println!("Hash table statistics: ");
        for table in &lsh.hash_tables {
            println!("Table: avg {} / {}", table.values().map(|x| x.len()).sum::<usize>(), table.len());
        }
    }

    info!("Sorting");

    let mut sorted_indices = (0..records.len())
        .map(|i| IndexPosition::Present(i))
        .collect_vec();

    sorted_indices
        .sort_unstable_by(|a, b| {
            let extract_qual = |pos: &IndexPosition| {
                let IndexPosition::Present(i) = pos else {
                    panic!("This should never happen");
                };
                records[*i].avg_qual;
            };

            let qual_a = extract_qual(a);
            let qual_b = extract_qual(b);
            qual_a.partial_cmp(&qual_b)
                .unwrap()
                .reverse()
        });

    records.shrink_to_fit();
    sorted_indices.shrink_to_fit();

    let index = Index {
        records,
        sorted_indices,
        lsh,
    };

    info!("Saving index...");
    // dump LSH
    bincode::serialize_into(wtr, &index).unwrap();
}


pub fn construct_index(infile: &str, outfile: &str) {
    let f = File::open(infile).expect("File could not be opened");
    let reader = BufReader::new(f);

    let wtr = std::io::BufWriter::new(File::create(outfile).unwrap());
    iter_lines(reader, wtr);
}
