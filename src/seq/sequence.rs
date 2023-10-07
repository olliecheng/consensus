pub use crate::seq::dna;
use std::fmt;

use super::bitenc::BitEnc;

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct Seq(pub BitEnc);

impl Seq {
    pub fn new() -> Self {
        // Use a default size of 80 to represent a typical seq length
        Self(BitEnc::with_capacity(2, 80))
    }

    pub fn with_capacity(n: usize) -> Self {
        Self(BitEnc::with_capacity(2, n))
    }

    pub fn push(&mut self, b: u8) {
        self.0.push(dna::dna_to_u8(b))
    }

    pub fn push_iter(&mut self, i: impl Iterator<Item = u8>) {
        i.map(dna::dna_to_u8).for_each(|x| self.0.push(x));
    }

    pub fn push_u32_chunk_of_n<'a>(&mut self, chunk: &'a [u8; 16], n: usize) {
        self.0.push_block_with_n_elems(dna::dna_to_u32(chunk), n);
    }

    pub fn len(&self) -> usize {
        self.0.nr_symbols()
    }

    pub fn from_string(s: &str) -> Self {
        let length = s.len();
        let mut seq = Self::with_capacity(length);
        seq.push_iter(s.bytes());
        seq
    }

    pub fn to_ascii(&self) -> Vec<u8> {
        self.0.iter().map(|x| dna::u8_to_dna(x) as u8).collect()
    }
}

impl fmt::Display for Seq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = String::from_utf8(self.to_ascii()).unwrap();
        write!(f, "{}", s)
    }
}

#[cfg(test)]
mod tests {
    use crate::seq::Seq;

    #[test]
    fn empty_equality() {
        let s1 = Seq::with_capacity(100);
        let s2 = Seq::with_capacity(2);
        assert_eq!(s1, s2)
    }

    #[test]
    fn check_equal() {
        let s1 = Seq::from_string("ATCGABC");
        let s2 = Seq::from_string("ATCGABC");
        assert_eq!(s1, s2)
    }

    #[test]
    fn check_notequal() {
        let s1 = Seq::from_string("ATCAAGC");
        let s2 = Seq::from_string("ATCGAGC");
        assert_ne!(s1, s2);
    }

    #[test]
    fn push() {
        let s1 = Seq::from_string("ATCG");
        let mut s2 = Seq::new();
        s2.push(b'A');
        s2.push(b'T');
        s2.push(b'C');
        s2.push(b'G');

        assert_eq!(s1, s2)
    }

    #[test]
    fn push_iter() {
        let s1 = Seq::from_string("ATCGCTA");
        let mut s2 = Seq::new();
        s2.push_iter("ATCGCTA".bytes());
        assert_eq!(s1, s2)
    }

    #[test]
    fn format() {
        let orig_seq = "ATCGACTC".to_string();
        let s = Seq::from_string(&orig_seq);
        assert_eq!(s.to_string(), orig_seq);
        assert_eq!(format!("{}", s), orig_seq)
    }

    #[test]
    fn test_bit_order() {
        let s1 = Seq::from_string("ATCG");
        let s2 = Seq::from_string("GCTA");
        assert_ne!(s1, s2);
    }

    #[test]
    fn to_ascii() {
        let s1 = Seq::from_string("ATcgACG");
        assert_eq!(s1.to_ascii(), vec![65u8, 84, 67, 71, 65, 67, 71]);
    }
}
