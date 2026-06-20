// Import necessary modules
use crate::closed_syncmers::{MinimizerResult};
// Function Declarations
pub fn compute_closed_syncmers_naive(sequence: &str, seq_len: usize, k: i32, s: i32, results: &mut Vec<MinimizerResult>, num_results: &mut i32) {
    results.clear();
    *num_results = 0;

    if k <= 0 || s <= 0 {
        return;
    }

    let k = k as usize;
    let s = s as usize;
    let sequence = sequence.as_bytes();
    let usable_len = seq_len.min(sequence.len());

    if usable_len < k || s > usable_len || s > k {
        return;
    }

    let mask = mask_for_bases(s);

    for i in 0..=usable_len - k {
        let mut min_hash = u128::MAX;
        let mut min_pos_in_kmer = 0_usize;

        for j in 0..=k - s {
            let s_mer_pos = i + j;
            let mut hash_fwd = 0_u128;
            let mut hash_rev = 0_u128;

            for k_offset in 0..s {
                let base = super::closed_syncmers::base_to_bits(sequence[s_mer_pos + k_offset] as char);
                hash_fwd = (hash_fwd << 2) | u128::from(base);
            }
            hash_fwd &= mask;

            for k_offset in 0..s {
                let pos = s_mer_pos + s - 1 - k_offset;
                let base = super::closed_syncmers::base_to_bits(sequence[pos] as char);
                let comp_base = super::closed_syncmers::complement_base(base);
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

fn mask_for_bases(s: usize) -> u128 {
    let bit_width = 2 * s;
    if bit_width >= u128::BITS as usize {
        u128::MAX
    } else {
        (1_u128 << bit_width) - 1
    }
}
