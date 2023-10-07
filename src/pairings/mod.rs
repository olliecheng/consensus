pub mod from_reader;
use crate::seq::{Identifier, Record, RecordData};

use std::collections::HashMap;
use std::vec::Vec;

use std::fmt;
use xxhash_rust::xxh3::Xxh3Builder as Hasher;

#[derive(Debug)]
pub struct BarcodeCollection {}

#[derive(Debug)]
pub struct Pairing<'a> {
    pub id: &'a Identifier,
    pub reads: &'a Vec<RecordData>,
}

impl<'a> fmt::Display for Pairing<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut reads_str = String::new();
        for i in self.reads.iter() {
            reads_str = format!("{}Seq_{}\n", reads_str, i.seq);
        }
        write!(f, "BC_{}\t UMI_{}\n{}", self.id.bc, self.id.umi, reads_str)
    }
}

#[derive(Debug)]
pub struct PairingCollection {
    pub pairings: HashMap<Identifier, Vec<RecordData>, Hasher>,
    // duplicates_set: HashSet<Identifier, Hasher>,
    pub total_reads: i64,
}

impl<'a> From<(&'a Identifier, &'a Vec<RecordData>)> for Pairing<'a> {
    fn from(value: (&'a Identifier, &'a Vec<RecordData>)) -> Self {
        Self {
            id: value.0,
            reads: value.1,
        }
    }
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
        let id = record.id;
        self.total_reads += 1;

        if let Some(v) = self.pairings.get_mut(&id) {
            // duplicate exists
            v.push(record.data);
            true
        } else {
            self.pairings.insert(id, vec![record.data]);
            false
        }
    }

    // https://github.com/rust-lang/rfcs/blob/master/text/1522-conservative-impl-trait.md
    // impl trait - rust 1.26
    pub fn duplicates(&mut self) -> impl Iterator<Item = Pairing> {
        self.all().filter(|x| x.reads.len() > 1)
    }

    pub fn all(&mut self) -> impl Iterator<Item = Pairing> {
        self.pairings.iter().map(|x| Pairing::from(x))
    }
}
