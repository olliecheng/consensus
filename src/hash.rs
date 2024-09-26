use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::ops::BitXor;
use lsh_rs::VecHash;

use std::hash::{BuildHasher, Hasher};

use ndarray::prelude::*;
use rand::{Rng};

use rkyv::{with::Skip, Archive, Serialize};
use rkyv::vec::ArchivedVec;

/// A hash family for the [Jaccard Index](https://en.wikipedia.org/wiki/Jaccard_index)
/// The generic integer N, needs to be able to hold the number of dimensions.
/// so a `u8` with a vector of > 255 dimensions will cause a `panic`.
pub struct MinHash {
    pub pi: Vec<u32>,
    n_bands: usize,
    n_projections: usize,
    n_hashes: usize,
    window_size: usize,
}

const FNV_PRIME: u32 = 16777619;
const FNV_OFFSET: u32 = 2166136261;

fn fnv_hash<'a>(hash: &'a mut u32, bytes: &[u8]) -> &'a u32 {
    for b in bytes.iter() {
        *hash ^= *b as u32;
        *hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

impl MinHash
{
    /// n_bands is the number of bands to be computed in total
    /// n_projections is the number of hashes within each band
    pub fn new(n_bands: usize, n_projections: usize, window_size: usize) -> Self {
        // let mut pi = Array::zeros((n_projections, dim));
        // let mut rng = create_rng(seed);

        // generate XOR permutations
        // we will use 32-bit integers as the hash value
        let n_hashes = n_projections * n_bands;
        let mut pi = vec![0u32; n_hashes];
        rand::thread_rng().fill(&mut pi[..]);

        MinHash {
            pi,
            n_bands,
            n_projections,
            n_hashes,
            window_size,
        }
    }
}

impl lsh_rs::VecHash<u8, u32> for MinHash
{
    fn hash_vec_query(&self, v: &[u8]) -> Vec<u32> {
        // we implement a rolling hash
        let kmer_windows = v.windows(self.window_size);
        let mut minhashes = vec![u32::MAX; self.n_hashes];

        for kmer in kmer_windows {
            // hash the window
            let mut hash = FNV_OFFSET;
            fnv_hash(&mut hash, kmer);

            // XOR hashes with the permutations
            let hashes = std::iter::repeat(hash)
                .take(self.n_hashes)
                .zip(self.pi.iter())
                .map(|(a, b)| a ^ b);

            // update the minhashes to include the new minimums
            minhashes.iter_mut()
                .zip(hashes)
                .for_each(|(orig, new)| {
                    *orig = std::cmp::min(*orig, new);
                });
        }

        // for each band (of size n_projections), we compute the overall hash
        minhashes.chunks_exact(self.n_projections).map(|c| {
            let mut hash = FNV_OFFSET;
            let chunk_sum = c.iter()
                .fold(0u32, |acc, &v| acc.wrapping_add(v))
                .to_be_bytes();
            fnv_hash(&mut hash, &chunk_sum);
            hash
        }).collect()
    }
}

#[derive(Archive, Serialize)]
pub struct MinHashLSH {
    pub n_bands: usize,
    pub n_proj: usize,
    pub hash_tables: Vec<HashMap<u32, Vec<usize>>>,

    #[with(Skip)]
    hash_function: Option<MinHash>,
}

impl MinHashLSH {
    pub fn new(n_bands: usize, n_proj: usize, window_size: usize) -> Self {
        let hash_tables = (0..n_bands).map(|_| HashMap::new()).collect();
        let hash_function = Some(MinHash::new(n_bands, n_proj, window_size));

        Self {
            n_bands,
            n_proj,
            hash_tables,
            hash_function,
        }
    }

    fn _hash(&self, vec: &[u8]) -> Vec<u32> {
        self.hash_function
            .as_ref()
            .expect("Fatal error: hash function is not loaded")
            .hash_vec_query(vec)
    }

    pub fn store(&mut self, vec: &[u8], index: usize) -> Vec<u32> {
        let hash = self._hash(vec);

        self.hash_tables.iter_mut()
            .zip(&hash)
            .for_each(|(table, hash)| {
                let entry = table.entry(*hash).or_default();
                entry.push(index);
            });

        hash
    }

    pub fn query(&self, vec: &[u8]) -> Vec<usize> {
        let hash = self._hash(vec);
        self.query_hash(&hash)
    }
}

impl MinHashLSH {
    pub fn query_hash(&self, hash: &[u32]) -> Vec<usize> {
        self.hash_tables.iter()
            .zip(hash)
            .filter_map(|(table, hash)| {
                table.get(&hash).cloned()
            })
            .flatten()
            .collect()
    }
}

impl ArchivedMinHashLSH {
    pub fn query_hash(&self, hash: &[u32]) -> Vec<usize> {
        self.hash_tables.iter()
            .zip(hash)
            .filter_map(|(table, hash)| {
                table
                    .get(&hash)
                    .and_then(|v| Some(v.to_vec()))
            })
            .flatten()
            .map(|u| u as usize)
            .collect()
    }
}