pub use crate::seq::dna;
use std::fmt;

use bio::data_structures::bitenc::BitEnc;

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct Seq(BitEnc);

impl Seq {
    pub fn new() -> Self {
        // Self(BitEnc::new(2))

        // Use a default size of 100 to represent a typical seq length
        Self(BitEnc::with_capacity(2, 80))
    }

    pub fn with_capacity(n: usize) -> Self {
        Self(BitEnc::with_capacity(2, n))
    }

    pub fn push(&mut self, b: u8) {
        self.0.push(dna::a_to_b(b))
    }

    pub fn push_iter(&mut self, i: impl Iterator<Item = u8>) {
        i.map(dna::a_to_b).for_each(|x| self.0.push(x));
    }

    pub fn from_string(s: &str) -> Self {
        let length = s.len();
        let mut seq = Self::with_capacity(length);
        seq.push_iter(s.bytes());
        seq
    }
}

impl fmt::Display for Seq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self
            .0
            .iter()
            .map(dna::b_to_a)
            .collect::<Vec<&str>>()
            .join("");
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
        let s1 = Seq::from_string("ATCDABC");
        let s2 = Seq::from_string("ATCGABC");
        assert_ne!(s1, s2)
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
}
