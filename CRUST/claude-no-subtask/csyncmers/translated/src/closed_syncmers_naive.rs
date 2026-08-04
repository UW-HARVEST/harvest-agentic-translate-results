// Import necessary modules
use crate::closed_syncmers::{base_to_bits, complement_base, MinimizerResult};

// Function Declarations
pub fn compute_closed_syncmers_naive(
    sequence: &str,
    seq_len: usize,
    k: i32,
    s: i32,
    results: &mut Vec<MinimizerResult>,
    num_results: &mut i32,
) {
    *num_results = 0;
    let k_us = k as usize;
    let s_us = s as usize;

    if seq_len < k_us {
        return;
    }

    let mask: u128 = (1u128 << (2 * s_us)) - 1;
    let bytes = sequence.as_bytes();

    // For each k-mer in the sequence
    for i in 0..=seq_len - k_us {
        let mut min_hash: u128 = !0u128;
        let mut min_pos_in_kmer: usize = 0;

        // For each s-mer within the k-mer, compute its hash
        for j in 0..=k_us - s_us {
            let s_mer_pos = i + j;
            let mut hash_fwd: u128 = 0;
            let mut hash_rev: u128 = 0;

            // Compute forward hash
            for kk in 0..s_us {
                let base = base_to_bits(bytes[s_mer_pos + kk] as char);
                hash_fwd = (hash_fwd << 2) | (base as u128);
            }
            hash_fwd &= mask;

            // Compute reverse complement hash
            for kk in 0..s_us {
                let pos = s_mer_pos + s_us - 1 - kk;
                let base = base_to_bits(bytes[pos] as char);
                let comp_base = complement_base(base);
                hash_rev = (hash_rev << 2) | (comp_base as u128);
            }
            hash_rev &= mask;

            // Compute canonical hash of that s-mer
            let canonical_hash = if hash_fwd < hash_rev {
                hash_fwd
            } else {
                hash_rev
            };
            if canonical_hash < min_hash {
                min_hash = canonical_hash;
                min_pos_in_kmer = j;
            }
        }

        // Check if the minimal s-mer is at the first or last position within the k-mer
        if min_pos_in_kmer == 0 || min_pos_in_kmer == k_us - s_us {
            results.push(MinimizerResult {
                kmer_position: i,
                smer_position: i + min_pos_in_kmer,
                minimizer_hash: min_hash,
            });
            *num_results += 1;
        }
    }
}
