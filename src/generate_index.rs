use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

// use csv::{Writer, WriterBuilder};

use bk_tree::{metrics, BKTree, Metric};
use triple_accel::hamming;

use itertools::Itertools;
use murmur3::murmur3_32;
use std::io::Cursor;

use crate::record::Record;

pub struct Hamming;

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

fn iter_lines<W: std::io::Write>(mut reader: BufReader<File>, mut wtr: W) {
    let mut position: usize = 0;
    let mut count: usize = 0;

    // write headers
    // wtr.write_record([
    //     "Read",
    //     "CellBarcode",
    //     "FlankEditDist",
    //     "BarcodeEditDist",
    //     "UMI",
    //     "Position",
    //     "MinHash",
    // ])
    // .unwrap();

    let mut result = String::new();

    // let mut tree: BKTree<String> = BKTree::new(metrics::Levenshtein);
    let mut tree: BKTree<String, Hamming> = BKTree::new(Hamming);

    let mut records = vec![Record::default()];

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

            record.id.bc = String::from(bc);
            record.id.umi = String::from(umi);
            record.read_id = String::from(id);
            record.loc = position;

            // let record = [id, bc, "?", "?", umi, String::from(position)];
            // wtr.write_field(id).unwrap();
            // wtr.write_field(bc).unwrap();
            // wtr.write_field("?").unwrap();
            // wtr.write_field("?").unwrap();
            // wtr.write_field(umi).unwrap();
            // wtr.write_field(&position.to_string()).unwrap();

            let mut id_str = String::from(bc);
            id_str.push_str("_");
            id_str.push_str(umi);
            // tree.add(id_str);
        } else if count % 4 == 1 {
            let bytes = result.as_bytes();
            let heap_size = 20;
            let max_size = 100;

            if bytes.len() > 20 {
                let heap = &mut records.last_mut().unwrap().minhash;

                // this is the sequence itself!
                // first, we take the front 100 characters, and take a rolling window
                let size = 5;
                let length = std::cmp::min(bytes.len(), 100);
                // let windows = bytes[0..length].windows(size);

                let windows = (5..10).map(|size| bytes[0..length].windows(size)).flatten();

                windows
                    .map(|x| murmur3_32(&mut Cursor::new(x), 123).unwrap())
                    .for_each(|x| {
                        if !heap.contains(&x) {
                            if heap.len() < heap_size {
                                heap.insert(x);
                            } else if let Some(&top) = heap.last() {
                                if x < top {
                                    heap.pop_last();
                                    heap.insert(x);
                                }
                            }
                        }
                    });

                // println!("Heap: {:?}", heap);
                // let min_n = &hashes[0..10];

                // wtr.write_field(heap.into_iter().map(|x| x.to_string()).join(","))
                //     .unwrap();
            } else {
                // wtr.write_field("too_small");
            }
            // wtr.write_record(None::<&[u8]>).unwrap();
            records.push(Record::default());
        }
        count += 1;
        position += bsize;

        // reset string
        result.clear();
    }
    // wtr.flush().unwrap();

    println!("Completed tree addition");
    println!(
        "{:?}",
        tree.find("TCTGGCTCATTCTCCG_GCAGCGAAGCCC", 10)
            .collect::<Vec<_>>()
    );

    bincode::serialize_into(wtr, &records).unwrap();
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
