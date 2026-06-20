// Import necessary modules
use crate::closed_syncmers::{MinimizerResult};
// Function Declarations
pub fn compute_closed_syncmers_naive(sequence: &str, seq_len: usize, k: i32, s: i32, results: &mut Vec<MinimizerResult>, num_results: &mut i32) {
    results.clear();
    *num_results = 0;

    if k <= 0 || s <= 0 || s > k {
        return;
    }

    let k = k as usize;
    let s = s as usize;
    let bytes = sequence.as_bytes();
    let seq_len = seq_len.min(bytes.len());

    if seq_len < k {
        return;
    }

    let mask = smer_mask(s);

    for i in 0..=seq_len - k {
        let mut min_hash = u128::MAX;
        let mut min_pos_in_kmer = 0_usize;

        for j in 0..=k - s {
            let s_mer_pos = i + j;
            let mut hash_fwd = 0_u128;
            let mut hash_rev = 0_u128;

            for offset in 0..s {
                let base = crate::closed_syncmers::base_to_bits(bytes[s_mer_pos + offset] as char);
                hash_fwd = (hash_fwd << 2) | u128::from(base);
            }
            hash_fwd &= mask;

            for offset in 0..s {
                let pos = s_mer_pos + s - 1 - offset;
                let base = crate::closed_syncmers::base_to_bits(bytes[pos] as char);
                let comp_base = crate::closed_syncmers::complement_base(base);
                hash_rev = (hash_rev << 2) | u128::from(comp_base);
            }
            hash_rev &= mask;

            let canonical_hash = hash_fwd.min(hash_rev);
            if canonical_hash < min_hash {
                min_hash = canonical_hash;
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

fn smer_mask(s: usize) -> u128 {
    let bits = 2 * s;
    if bits >= u128::BITS as usize {
        u128::MAX
    } else {
        (1_u128 << bits) - 1
    }
}
