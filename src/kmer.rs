pub trait BinaryKmer {
    fn kmer_to_binary(kmer: &[u8]) -> Self;
}

impl BinaryKmer for u32 {
    fn kmer_to_binary(kmer: &[u8]) -> Self {
        let (bytes, rest) = kmer.split_at(size_of::<Self>());
        assert_eq!(rest.len(), 0, "kmer was the wrong size");
        Self::from_ne_bytes(bytes.try_into().unwrap())
    }
}

impl BinaryKmer for u64 {
    fn kmer_to_binary(kmer: &[u8]) -> Self {
        let (bytes, rest) = kmer.split_at(size_of::<Self>());
        assert_eq!(rest.len(), 0, "kmer was the wrong size");
        Self::from_ne_bytes(bytes.try_into().unwrap())
    }
}

pub fn shingles<T: BinaryKmer>(seq: &[u8]) -> Vec<T>
{
    let size = size_of::<T>();
    seq.windows(size).map(|x| T::kmer_to_binary(x)).collect()
}
