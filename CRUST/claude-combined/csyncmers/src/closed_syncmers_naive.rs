// Import necessary modules
use crate::closed_syncmers::{base_to_bits, complement_base, MinimizerResult};
// Function Declarations
pub fn compute_closed_syncmers_naive(sequence: &str, seq_len: usize, k: i32, s: i32, results: &mut Vec<MinimizerResult>, num_results: &mut i32) {
    *num_results = 0;
    results.clear();

    let k = k as usize;
    let s = s as usize;

    if seq_len < k {
        return;
    }

    let mask: u128 = (1u128 << (2 * s)) - 1;
    let seq_bytes = sequence.as_bytes();

    // For each k-mer in the sequence
    for i in 0..=(seq_len - k) {
        let mut min_hash: u128 = !0u128;
        let mut min_pos_in_kmer: usize = 0;

        // For each s-mer within the k-mer, compute its hash
        for j in 0..=(k - s) {
            let s_mer_pos = i + j;
            let mut hash_fwd: u128 = 0;
            let mut hash_rev: u128 = 0;

            // Compute forward hash
            for kk in 0..s {
                let base = base_to_bits(seq_bytes[s_mer_pos + kk] as char);
                hash_fwd = (hash_fwd << 2) | base as u128;
            }
            hash_fwd &= mask;

            // Compute reverse complement hash
            for kk in 0..s {
                let pos = s_mer_pos + s - 1 - kk;
                let base = base_to_bits(seq_bytes[pos] as char);
                let comp_base = complement_base(base);
                hash_rev = (hash_rev << 2) | comp_base as u128;
            }
            hash_rev &= mask;

            // Compute canonical hash of that s-mer
            let canonical_hash = if hash_fwd < hash_rev { hash_fwd } else { hash_rev };
            if canonical_hash < min_hash {
                min_hash = canonical_hash;
                min_pos_in_kmer = j;
            }
        }

        // Check if the minimal s-mer is at the first or last position within the k-mer
        if min_pos_in_kmer == 0 || min_pos_in_kmer == k - s {
            results.push(MinimizerResult {
                minimizer_hash: min_hash,
                kmer_position: i,
                smer_position: i + min_pos_in_kmer,
            });
            *num_results += 1;
        }
    }
}
