// Struct Definitions
#[derive(Debug, Clone)]
pub struct MinimizerResult {
    pub minimizer_hash: u128,
    pub kmer_position: usize,
    pub smer_position: usize,
}
// Function Declarations
pub fn compute_closed_syncmers(sequence_input: &str, len: i32, k: i32, s: i32, results: &mut Vec<MinimizerResult>, num_results: &mut i32) {
    results.clear();
    *num_results = 0;

    if len < 0 || k <= 0 || s <= 0 {
        return;
    }

    let len = len as usize;
    let k = k as usize;
    let s = s as usize;
    let sequence = sequence_input.as_bytes();
    let usable_len = len.min(sequence.len());

    if usable_len < k || s > usable_len || s > k {
        return;
    }

    let num_s_mers = usable_len - s + 1;
    let mut s_mer_hashes = vec![0_u128; num_s_mers];

    let mask = mask_for_bases(s);
    let mut hash_fwd = 0_u128;
    let mut hash_rev = 0_u128;
    let rc_shift = 2 * (s - 1);

    for (i, &base_byte) in sequence.iter().take(usable_len).enumerate() {
        let base = base_to_bits(base_byte as char);
        hash_fwd = ((hash_fwd << 2) | u128::from(base)) & mask;
        let comp_base = complement_base(base);
        hash_rev = ((hash_rev >> 2) | (u128::from(comp_base) << rc_shift)) & mask;

        if i >= s - 1 {
            let s_mer_pos = i + 1 - s;
            s_mer_hashes[s_mer_pos] = hash_fwd.min(hash_rev);
        }
    }

    let window_size = k - s + 1;
    let mut deque = vec![0_usize; num_s_mers];
    let mut front = 0_usize;
    let mut back = 0_usize;

    for i in 0..num_s_mers {
        while back > front && s_mer_hashes[deque[back - 1]] > s_mer_hashes[i] {
            back -= 1;
        }
        deque[back] = i;
        back += 1;

        if i >= window_size && deque[front] <= i - window_size {
            front += 1;
        }

        if i >= window_size - 1 {
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
    results.push(MinimizerResult {
        minimizer_hash,
        kmer_position,
        smer_position,
    });
    *size += 1;
}

fn mask_for_bases(s: usize) -> u128 {
    let bit_width = 2 * s;
    if bit_width >= u128::BITS as usize {
        u128::MAX
    } else {
        (1_u128 << bit_width) - 1
    }
}
