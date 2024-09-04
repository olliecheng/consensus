use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

use rand::Rng;

use lsh_rs::prelude::*;


// use csv::{Writer, WriterBuilder};

use bk_tree::{metrics, BKTree, Metric};
use triple_accel::hamming;

use itertools::Itertools;
use murmur3::murmur3_32;
use std::io::Cursor;
use serde::{Deserialize, Serialize};
use crate::record::Record;

pub struct Hamming;

#[derive(Serialize, Deserialize)]
pub struct Index {
    pub records: Vec<Record>,
    pub lsh: crate::hash::MinHashLSH,
}

impl<K: AsRef<str> + ?Sized> Metric<K> for Hamming {
    fn distance(&self, a: &K, b: &K) -> u32 {
        let a_bytes = a.as_ref().as_bytes();
        let b_bytes = b.as_ref().as_bytes();
        hamming(a_bytes, b_bytes)
    }

    fn threshold_distance(&self, a: &K, b: &K, threshold: u32) -> Option<u32> {
        let a_bytes = a.as_ref().as_bytes();
        let b_bytes = b.as_ref().as_bytes();

        let v = hamming(a_bytes, b_bytes);

        if v <= threshold {
            Some(v)
        } else {
            None
        }
    }
}

fn iter_lines<W: Write>(mut reader: BufReader<File>, mut wtr: W) {
    let mut position: usize = 0;
    let mut count: usize = 0;

    let mut result = String::new();
    let mut records = vec![Record::default()];

    let subseq_size = 50;
    let shingle_size = 8;
    let dim = subseq_size - shingle_size + 1;

    let mut lsh = crate::hash::MinHashLSH::new(10, 20, dim);

    while let Ok(bsize) = reader.read_line(&mut result) {
        if bsize == 0 {
            // EOF has been reached
            break;
        }

        if count % 4 == 0 {
            let record = &mut records.last_mut().unwrap();

            // extract barcode, UMI, and the read ID
            // format: @TCTGGCTCATTCTCCG_GCAGCGAAGCCC#32b5d571-ad88-4ac7-bc46-f2ff03de65aa_+1of1
            let i = result.find('_').unwrap();
            let j = result.find('#').unwrap();
            let k = result.rfind('_').unwrap();

            let bc = &result[1..i];
            let umi = &result[(i + 1)..j];
            let id = &result[(j + 1)..k];

            let id_obj = crate::record::RecordIdentifier {
                bc: String::from(bc),
                umi: String::from(umi),
            };

            let rec = crate::record::Record {
                id: id_obj,
                read_id: String::from(id),
                loc: position,
            };

            records.push(rec);
        } else if count % 4 == 1 {
            let bytes = result.as_bytes().to_vec();
            if bytes.len() > subseq_size {
                let subset = &bytes[..subseq_size];
                lsh.store(subset, records.len() - 1);
            }
        }

        // report progress
        if count % 1000 == 0 {
            println!("Progress: {count}");
        }

        count += 1;
        position += bsize;

        // reset string
        result.clear();
    }

    {
        // print summary statistics
        println!("Statistics: ");
        for table in lsh.hash_tables {
            println!("Table: avg");
            print_with_3_digits(
                table.values().map(|x| x.len()).sum(),
                table.len(),
            )
        }
    }

    let index = Index {
        records,
        lsh,
    };

    // dump LSH
    bincode::serialize_into(wtr, &index).unwrap();
}


fn print_with_3_digits(a: usize, b: usize) {
    let a_mul = (a as u128) * 1000;
    let b = b as u128;
    let div = a_mul / b;

    let frac = div % 1000;
    let rest = div / 1000;

    println!("{}.{:#03}", rest, frac);
}

pub fn construct_index(infile: &str, outfile: &str) {
    let f = File::open(infile).expect("File could not be opened");
    let reader = BufReader::new(f);

    //     let wtr = WriterBuilder::new()
    //         .delimiter(b'\t')
    //         .from_path(outfile)
    //         .unwrap();

    let wtr = std::io::BufWriter::new(File::create(outfile).unwrap());

    iter_lines(reader, wtr);
}
