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
    let len = len as usize;
    let k = k as usize;
    let s = s as usize;

    if len < k {
        eprintln!("Sequence length is less than K");
        return;
    }

    let seq = sequence_input.as_bytes();
    let num_s_mers = len - s + 1;
    let mask: u128 = (1u128 << (2 * s)) - 1;
    let rc_shift = 2 * (s - 1);

    // Precompute all s-mer canonical hashes
    let mut s_mer_hashes = vec![0u128; num_s_mers];
    let mut hash_fwd: u128 = 0;
    let mut hash_rev: u128 = 0;

    for i in 0..len {
        let base = base_to_bits(seq[i] as char);
        hash_fwd = ((hash_fwd << 2) | base as u128) & mask;
        let comp = complement_base(base);
        hash_rev = ((hash_rev >> 2) | ((comp as u128) << rc_shift)) & mask;
        if i + 1 >= s {
            let pos = i + 1 - s;
            s_mer_hashes[pos] = hash_fwd.min(hash_rev);
        }
    }

    // Sliding window minimum with deque
    let window_size = k - s + 1;
    let mut deque: Vec<usize> = Vec::with_capacity(num_s_mers);
    let mut front = 0usize;

    for i in 0..num_s_mers {
        while deque.len() > front && s_mer_hashes[*deque.last().unwrap()] > s_mer_hashes[i] {
            deque.pop();
        }
        deque.push(i);
        if i >= window_size && deque[front] + window_size <= i {
            front += 1;
        }
        if i + 1 >= window_size {
            let min_pos = deque[front];
            let kmer_pos = i + 1 - window_size;
            if min_pos == kmer_pos || min_pos == kmer_pos + k - s {
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
    results.push(MinimizerResult { minimizer_hash, kmer_position, smer_position });
    *size += 1;
}
