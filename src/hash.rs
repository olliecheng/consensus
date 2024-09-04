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
    n_projections: usize,
    dim: usize,
}

impl MinHash
{
    pub fn new(n_projections: usize, dim: usize, seed: i64) -> Self {
        // let mut pi = Array::zeros((n_projections, dim));
        // let mut rng = create_rng(seed);

        // generate XOR permutations
        // we will use 32-bit integers as the hash value
        let mut arr = vec![0u32; n_projections];
        rand::thread_rng().fill(&mut arr[..]);

        MinHash {
            pi: Array::from(arr),
            seed,
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
            .broadcast((self.dim, self.n_projections))
            .expect("Should be broadcastable")
            .bitxor(&self.pi);

        // get the minimum along rows (axis 1)
        let min_hashes = all_hashes.map_axis(Axis(1), |view| *view.iter().min().unwrap());
        min_hashes.to_vec()
    }
}

#[derive(Serialize, Deserialize)]
pub struct MinHashLSH {
    pub n_bands: usize,
    pub n_proj: usize,
    pub hash_tables: Vec<HashMap<Vec<u32>, Vec<usize>>>,
    hash_function: MinHash,
}

impl MinHashLSH {
    pub fn new(n_bands: usize, n_proj: usize, dim: usize) -> Self {
        let mut hash_tables = (0..n_bands).map(|_| HashMap::new()).collect();
        let hash_function = MinHash::new(n_bands * n_proj, dim, 0);

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

    pub fn store(&mut self, vec: &[u8], index: usize) {
        let hash = self._hash(vec);

        self.hash_tables.iter_mut()
            .zip(hash.rchunks_exact(self.n_proj))
            .for_each(|(table, hashes)| {
                let entry = table.entry(Vec::from(hashes)).or_default();
                entry.push(index);
            });
    }

    pub fn query(&self, vec: &[u8]) -> Vec<usize> {
        let hash = self._hash(vec);

        self.hash_tables.iter()
            .zip(hash.rchunks_exact(self.n_proj))
            .filter_map(|(table, hashes)| {
                match table.get(hashes) {
                    Some(v) => Some(v.clone()),
                    None => None
                }
            })
            .flatten()
            .collect()
    }
}