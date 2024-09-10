use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::ops::BitXor;
use lsh_rs::VecHash;
use serde::{Serialize, Deserialize};
use ndarray::prelude::*;
use rand::{Rng};

/// A hash family for the [Jaccard Index](https://en.wikipedia.org/wiki/Jaccard_index)
/// The generic integer N, needs to be able to hold the number of dimensions.
/// so a `u8` with a vector of > 255 dimensions will cause a `panic`.
#[derive(Serialize, Deserialize, Clone)]
pub struct MinHash {
    pub pi: Array1<u32>,
    seed: i64,
    n_bands: usize,
    n_projections: usize,
    dim: usize,
}

impl MinHash
{
    pub fn new(n_bands: usize, n_projections: usize, dim: usize, seed: i64) -> Self {
        // let mut pi = Array::zeros((n_projections, dim));
        // let mut rng = create_rng(seed);

        // generate XOR permutations
        // we will use 32-bit integers as the hash value
        let mut arr = vec![0u32; n_projections * n_bands];
        rand::thread_rng().fill(&mut arr[..]);

        MinHash {
            pi: Array::from(arr),
            seed,
            n_bands,
            n_projections,
            dim,
        }
    }
}

impl lsh_rs::VecHash<u8, u32> for MinHash
{
    fn hash_vec_query(&self, v: &[u8]) -> Vec<u32> {
        let windows = v.windows(8);
        let quality_hashes = Array2::from_shape_vec(
            (self.dim, 1),
            windows
                .map(|x| gxhash::gxhash32(x, self.seed))
                .collect(),
        ).expect("Should not fail");

        // let pi_broadcast = self.pi.broadcast((self.dim, self.n_projections)).unwrap();
        let all_hashes = quality_hashes
            .broadcast((self.dim, self.n_projections * self.n_bands))
            .expect("Should be broadcastable")
            .bitxor(&self.pi);

        // get the minimum along rows (axis 1)
        let min_hashes = all_hashes.map_axis(Axis(0), |view| *view.iter().min().unwrap());
        min_hashes
            .to_vec()
            .rchunks_exact(self.n_projections)
            .map(|v| {
                let mut bytes = Vec::with_capacity(v.len() * 4);
                for x in v {
                    bytes.extend(x.to_ne_bytes())
                }
                gxhash::gxhash32(&bytes, self.seed)
            })
            .collect()
    }
}

#[derive(Serialize, Deserialize)]
pub struct MinHashLSH {
    pub n_bands: usize,
    pub n_proj: usize,
    pub hash_tables: Vec<HashMap<u32, Vec<usize>>>,
    hash_function: MinHash,
}

impl MinHashLSH {
    pub fn new(n_bands: usize, n_proj: usize, dim: usize) -> Self {
        let hash_tables = (0..n_bands).map(|_| HashMap::new()).collect();
        let hash_function = MinHash::new(n_bands, n_proj, dim, 0);

        Self {
            n_bands,
            n_proj,
            hash_tables,
            hash_function,
        }
    }

    fn _hash(&self, vec: &[u8]) -> Vec<u32> {
        self.hash_function.hash_vec_query(vec)
    }

    pub fn store(&mut self, vec: &[u8], index: usize) -> Vec<u32> {
        let hash = self._hash(vec);
        // println!("{hash:?}");

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