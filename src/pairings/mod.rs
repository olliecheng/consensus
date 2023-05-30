pub mod from_reader;
use crate::reader;
use crate::seq::{Identifier, Seq};

use std::collections::{HashMap, HashSet};
use std::vec::Vec;

use xxhash_rust::xxh3::Xxh3Builder as Hasher;

#[derive(Debug)]
pub struct BarcodeCollection {}

#[derive(Debug)]
pub struct PairingCollection {
    pub pairings: HashMap<Identifier, Vec<Seq>, Hasher>,
    duplicates_set: HashSet<Identifier, Hasher>,
}

impl PairingCollection {
    pub fn new() -> Self {
        // xxhash's Xxh3 builder provides good performance
        let s = Hasher::new();

        let pairings = HashMap::with_hasher(s);
        let duplicates_set = HashSet::with_hasher(s);

        Self {
            pairings,
            duplicates_set,
        }
    }

    pub fn add_read(&mut self, id: Identifier, seq: Seq) -> bool {
        if let Some(v) = self.pairings.get_mut(&id) {
            // duplicate exists
            v.push(seq);
            self.duplicates_set.insert(id);
            true
        } else {
            self.pairings.insert(id, vec![seq]);
            false
        }
    }

    // https://github.com/rust-lang/rfcs/blob/master/text/1522-conservative-impl-trait.md
    // impl trait - rust 1.26
    pub fn duplicates(&mut self) -> impl Iterator<Item = (&Identifier, &Vec<Seq>)> {
        self.duplicates_set
            .iter()
            .map(|x| self.pairings.get_key_value(x).unwrap())
    }
}
