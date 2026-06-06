// Import necessary modules
use std::collections::HashMap;
// Struct Definitions
#[derive(Debug, Clone)]
pub struct MinimizerResult {
    pub minimizer_hash: u128,
    pub kmer_position: usize,
    pub smer_position: usize,
}
// Function Declarations
pub fn compute_closed_syncmers(sequence_input: &str, len: i32, k: i32, s: i32, results: &mut Vec<MinimizerResult>, num_results: &mut i32) {
    *num_results = 0;
    results.clear();

    if len < k {
        eprintln!("Sequence length is less than K");
        return;
    }

    let len_us = len as usize;
    let k_us = k as usize;
    let s_us = s as usize;

    let bytes = sequence_input.as_bytes();

    let num_s_mers = len_us - s_us + 1;
    let mut s_mer_hashes: Vec<u128> = vec![0u128; num_s_mers];

    let mask: u128 = if (2 * s_us) >= 128 {
        u128::MAX
    } else {
        (1u128 << (2 * s_us)) - 1
    };
    let mut hash_fwd: u128 = 0;
    let mut hash_rev: u128 = 0;
    let rc_shift: u32 = (2 * (s_us - 1)) as u32;

    for i in 0..len_us {
        let base = base_to_bits(bytes[i] as char);
        hash_fwd = ((hash_fwd << 2) | base as u128) & mask;
        let comp_base = complement_base(base);
        hash_rev = ((hash_rev >> 2) | ((comp_base as u128) << rc_shift)) & mask;
        if i + 1 >= s_us {
            let s_mer_pos = i + 1 - s_us;
            let canonical_hash = if hash_fwd < hash_rev { hash_fwd } else { hash_rev };
            s_mer_hashes[s_mer_pos] = canonical_hash;
        }
    }

    let window_size = k_us - s_us + 1;
    let mut deque: Vec<usize> = vec![0usize; num_s_mers];
    let mut front: usize = 0;
    let mut back: usize = 0;

    for i in 0..num_s_mers {
        while back > front && s_mer_hashes[deque[back - 1]] > s_mer_hashes[i] {
            back -= 1;
        }
        deque[back] = i;
        back += 1;
        if i >= window_size && deque[front] <= i - window_size {
            front += 1;
        }

        if i + 1 >= window_size {
            let min_pos = deque[front];
            let kmer_pos = i + 1 - window_size;
            if min_pos == kmer_pos || min_pos == kmer_pos + k_us - s_us {
                add_minimizer(results, num_results, s_mer_hashes[min_pos], kmer_pos, min_pos);
            }
        }
    }
}
pub fn base_to_bits(base: char) -> u8 {
    match base {
        'A' | 'a' => 0,
        'C' | 'c' => 1,
        'G' | 'g' => 2,
        'T' | 't' => 3,
        _ => 0,
    }
}
pub fn complement_base(base: u8) -> u8 {
    3 - base
}
pub fn add_minimizer(results: &mut Vec<MinimizerResult>, size: &mut i32, minimizer_hash: u128, kmer_position: usize, smer_position: usize) {
    results.push(MinimizerResult {
        minimizer_hash,
        kmer_position,
        smer_position,
    });
    *size += 1;
}
