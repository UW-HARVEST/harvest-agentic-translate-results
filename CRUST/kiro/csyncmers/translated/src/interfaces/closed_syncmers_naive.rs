// Import necessary modules
use crate::closed_syncmers::{MinimizerResult};
// Function Declarations
pub fn compute_closed_syncmers_naive(sequence: &str, seq_len: usize, k: i32, s: i32, results: &mut Vec<MinimizerResult>, num_results: &mut i32) {
    use crate::closed_syncmers::{base_to_bits, complement_base};

    *num_results = 0;
    let k = k as usize;
    let s = s as usize;
    let seq = sequence.as_bytes();
    let mask: u128 = (1u128 << (2 * s)) - 1;

    for i in 0..=seq_len - k {
        let mut min_hash: u128 = u128::MAX;
        let mut min_pos_in_kmer: usize = 0;

        for j in 0..=k - s {
            let smer_pos = i + j;
            let mut hash_fwd: u128 = 0;
            let mut hash_rev: u128 = 0;

            for kk in 0..s {
                hash_fwd = (hash_fwd << 2) | base_to_bits(seq[smer_pos + kk] as char) as u128;
            }
            hash_fwd &= mask;

            for kk in 0..s {
                let pos = smer_pos + s - 1 - kk;
                hash_rev = (hash_rev << 2) | complement_base(base_to_bits(seq[pos] as char)) as u128;
            }
            hash_rev &= mask;

            let canonical = hash_fwd.min(hash_rev);
            if canonical < min_hash {
                min_hash = canonical;
                min_pos_in_kmer = j;
            }
        }

        if min_pos_in_kmer == 0 || min_pos_in_kmer == k - s {
            results.push(MinimizerResult {
                kmer_position: i,
                smer_position: i + min_pos_in_kmer,
                minimizer_hash: min_hash,
            });
            *num_results += 1;
        }
    }
}
