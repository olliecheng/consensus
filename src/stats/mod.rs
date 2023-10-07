use crate::pairings;
use crate::seq::dna;
use crate::seq::Seq;

use std::fmt;

#[derive(Clone, Debug)]
pub struct Stats {
    pub label: String,     // label for this stats group
    pub duplicates: usize, // number of duplicates
    pub pc_t: f32,         // percentage of T
    pub pc_a: f32,         // percentage of A
    pub len: f32,          // average length
    pub qc: f32,           // average quality score
    pairs: usize,          // number of pairings added
}

impl Stats {
    pub fn from(p: &pairings::Pairing) -> Self {
        let read_cnt = p.reads.len();
        let id_len = (p.id.bc.len() + p.id.umi.len()) as f32;

        let pc_t = [&p.id.bc, &p.id.umi]
            .into_iter()
            .map(|x: &Seq| x.0.iter().filter(|b| *b == dna::T).count() as f32)
            .sum::<f32>()
            / id_len
            * 100.0;

        let pc_a = [&p.id.bc, &p.id.umi]
            .into_iter()
            .map(|x: &Seq| x.0.iter().filter(|b| *b == dna::A).count() as f32)
            .sum::<f32>()
            / id_len
            * 100.0;

        let read_length: usize = p.reads.iter().map(|x| x.seq.len()).sum();

        Self {
            label: String::new(),
            duplicates: read_cnt,
            pc_t,
            pc_a,
            len: (read_length as f32) / (read_cnt as f32),
            qc: 0.0,
            pairs: 1,
        }
    }

    pub fn add_pairing(&mut self, p: &pairings::Pairing) -> Self {
        let n = Self::from(p);

        let old_dups = self.duplicates as f32;
        let new_dups = old_dups + (n.duplicates as f32);

        let old_pairs = self.pairs as f32;
        let new_pairs = old_pairs + 1.0;

        self.pc_t = (self.pc_t * old_pairs / new_pairs) + (n.pc_t / new_pairs);
        self.pc_a = (self.pc_a * old_pairs / new_pairs) + (n.pc_a / new_pairs);
        self.len = (self.len * old_dups / new_dups) + (n.len / new_dups);
        self.qc = (self.pc_t * old_dups / new_dups) + (n.qc / new_dups);

        self.duplicates += n.duplicates;
        self.pairs += 1;

        return n;
    }

    pub fn default(label: String) -> Self {
        Self {
            label,
            duplicates: 0,
            pc_t: 0.0,
            pc_a: 0.0,
            len: 0.0,
            qc: 0.0,
            pairs: 0,
        }
    }

    pub fn display_header() -> &'static str {
        "label\tquant\t%T\t%A\tlength" // \tquality"
    }
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}\t{}\t{:.2}\t{:.2}\t{:.2}", //"\t{:.2}",
            self.label,
            self.duplicates,
            self.pc_t,
            self.pc_a,
            self.len //, self.qc
        )
    }
}
