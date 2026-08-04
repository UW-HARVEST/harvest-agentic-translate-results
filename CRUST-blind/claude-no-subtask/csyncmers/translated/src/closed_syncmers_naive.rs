// Import necessary modules
use crate::closed_syncmers::{base_to_bits, complement_base, MinimizerResult};
// Function Declarations
pub fn compute_closed_syncmers_naive(sequence: &str, seq_len: usize, k: i32, s: i32, results: &mut Vec<MinimizerResult>, num_results: &mut i32) {
    *num_results = 0;
    results.clear();

    let k_us = k as usize;
    let s_us = s as usize;

    if seq_len < k_us {
        return;
    }
    if k_us < s_us {
        return;
    }

    let bytes = sequence.as_bytes();

    let bits = 2usize * s_us;
    let mask: u128 = if bits >= 128 { !0u128 } else { (1u128 << bits) - 1 };

    // For each k-mer in the sequence: i = 0..=seq_len - k
    let last_kmer_start = seq_len - k_us;
    for i in 0..=last_kmer_start {
        let mut min_hash: u128 = !0u128;
        let mut min_pos_in_kmer: usize = 0;

        // For each s-mer within the k-mer (j = 0..=k - s)
        for j in 0..=(k_us - s_us) {
            let s_mer_pos = i + j;
            let mut hash_fwd: u128 = 0;
            let mut hash_rev: u128 = 0;

            // Forward hash
            for kk in 0..s_us {
                let base = base_to_bits(bytes[s_mer_pos + kk] as char);
                hash_fwd = (hash_fwd << 2) | (base as u128);
            }
            hash_fwd &= mask;

            // Reverse complement hash
            for kk in 0..s_us {
                let pos = s_mer_pos + s_us - 1 - kk;
                let base = base_to_bits(bytes[pos] as char);
                let comp_base = complement_base(base);
                hash_rev = (hash_rev << 2) | (comp_base as u128);
            }
            hash_rev &= mask;

            let canonical_hash = if hash_fwd < hash_rev { hash_fwd } else { hash_rev };
            if canonical_hash < min_hash {
                min_hash = canonical_hash;
                min_pos_in_kmer = j;
            }
        }

        if min_pos_in_kmer == 0 || min_pos_in_kmer == k_us - s_us {
            results.push(MinimizerResult {
                minimizer_hash: min_hash,
                kmer_position: i,
                smer_position: i + min_pos_in_kmer,
            });
            *num_results += 1;
        }
    }
}
