use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::ops::BitXor;
use lsh_rs::VecHash;
use tab_hash::Tab32Twisted;

use std::hash::{BuildHasher, Hasher};

use ndarray::prelude::*;
use rand::{rngs::StdRng, SeedableRng, Rng};

use rkyv::{with::Skip, Archive, Serialize};
use rkyv::vec::ArchivedVec;

/// A hash family for the [Jaccard Index](https://en.wikipedia.org/wiki/Jaccard_index)
/// The generic integer N, needs to be able to hold the number of dimensions.
/// so a `u8` with a vector of > 255 dimensions will cause a `panic`.
pub struct MinHash {
    pub pi: Vec<u32>,
    hashers: Vec<Tab32Twisted>,
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
        StdRng::seed_from_u64(42).fill(&mut pi[..]);

        let mut hashers = Vec::new();
        for _ in 0..n_hashes {
            let hasher = Tab32Twisted::new();
            hashers.push(hasher);
        }

        MinHash {
            pi,
            n_bands,
            n_projections,
            n_hashes,
            hashers,
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

        // "hashes to check"
        let key_rngs = [3995498159u32, 1477833553, 1199840539, 2559637847, 3009001293, 154490722, 204597617, 1987378923, 1075782831, 4165816728, 788225858, 3478346083, 3616788989, 144221021, 490502320, 3500218662, 3864353613, 1309612394, 1070264186, 753392426, 928440792, 1556836655, 3953363780, 991355508, 859981582];

        for kmer in kmer_windows {
            // hash the window
            let mut hash = FNV_OFFSET;
            fnv_hash(&mut hash, kmer);

            if kmer == b"TTTTTTTTTTTTTT" {
                dbg!(hash);
                panic!("Err");
            }

            // XOR hashes with the permutations
            let hashes = std::iter::repeat(hash)
                .take(self.n_hashes)
                .zip(self.pi.iter())
                .map(|(a, b)| a ^ b);

            // convert kmer into u32s
            // let kmer_secs: Vec<_> = kmer.chunks_exact(4)
            //     .map(|c| u32::from_le_bytes(c.try_into().expect("Should not fail")))
            //     .collect();
            //
            // let hashes: Vec<_> = self.hashers
            //     .iter()
            //     .map(|hasher| {
            //         kmer_secs.iter()
            //             .fold(0, |c, val| {
            //                 c ^ hasher.hash(*val)
            //             })
            //     })
            //     .collect();


            // let hashes_ = hashes.collect::<Vec<_>>();
            //
            // println!("New read");
            // for hash in hashes_.iter() {
            //     if key_rngs.contains(hash) {
            //         eprintln!("{}, {}", hash, std::str::from_utf8(kmer).unwrap());
            //     }
            // }

            // println!("hashes\n{}", hashes_.iter().map(|x| x.to_string()).collect::<Vec<_>>().join("\n"));

            // update the minhashes to include the new minimums
            minhashes.iter_mut()
                .zip(hashes.into_iter())
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
            // c.iter()
            //     .fold(0u32, |acc, v| {
            //         acc ^ self.hashers[0].hash(*v)
            //     })
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

    pub fn print_stats(&self) {
        for (i, ht) in self.hash_tables.iter().enumerate() {
            eprintln!("Hash table {}", i);
            let lengths: Vec<_> = ht.values().map(|x| x.len() as f64).collect();
            eprintln!("Mean: {}", lengths.iter().sum::<f64>() / (lengths.len() as f64));
            let max = ht.iter()
                .max_by_key(|e| e.1.len())
                .unwrap();
            eprintln!("Max: {}, with length {}", max.0, max.1.len());
        }
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