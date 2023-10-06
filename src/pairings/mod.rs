pub mod from_reader;
use crate::seq::{Identifier, Record};

use std::collections::HashMap;
use std::vec::Vec;

use xxhash_rust::xxh3::Xxh3Builder as Hasher;

#[derive(Debug)]
pub struct BarcodeCollection {}

#[derive(Debug)]
pub struct PairingCollection {
    pub pairings: HashMap<Identifier, Vec<Record>, Hasher>,
    // duplicates_set: HashSet<Identifier, Hasher>,
    pub total_reads: i64,
}

impl PairingCollection {
    pub fn new() -> Self {
        // xxhash's Xxh3 builder provides good performance
        let s = Hasher::new();

        let pairings = HashMap::with_hasher(s);
        // let duplicates_set = HashSet::with_hasher(s);

        Self {
            pairings,
            // duplicates_set,
            total_reads: 0,
        }
    }

    pub fn add_read(&mut self, record: Record) -> bool {
        let id = record.id.clone();
        self.total_reads += 1;

        if let Some(v) = self.pairings.get_mut(&id) {
            // duplicate exists
            v.push(record);
            true
        } else {
            self.pairings.insert(id, vec![record]);
            false
        }
    }

    // https://github.com/rust-lang/rfcs/blob/master/text/1522-conservative-impl-trait.md
    // impl trait - rust 1.26
    pub fn duplicates(&mut self) -> impl Iterator<Item = (&Identifier, &Vec<Record>)> {
        self.pairings
            .keys()
            .map(|x| self.pairings.get_key_value(x).unwrap())
            .filter(|x| x.1.len() > 1)
    }
}
