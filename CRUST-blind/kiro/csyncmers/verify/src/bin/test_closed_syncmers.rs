use csyncmers::closed_syncmers::{base_to_bits, complement_base, add_minimizer, compute_closed_syncmers, MinimizerResult};

#[test]
fn test_base_to_bits_upper() {
    assert_eq!(base_to_bits('A'), 0);
    assert_eq!(base_to_bits('C'), 1);
    assert_eq!(base_to_bits('G'), 2);
    assert_eq!(base_to_bits('T'), 3);
}

#[test]
fn test_base_to_bits_lower() {
    assert_eq!(base_to_bits('a'), 0);
    assert_eq!(base_to_bits('c'), 1);
    assert_eq!(base_to_bits('g'), 2);
    assert_eq!(base_to_bits('t'), 3);
}

#[test]
fn test_base_to_bits_unknown() {
    assert_eq!(base_to_bits('N'), 0);
    assert_eq!(base_to_bits('X'), 0);
}

#[test]
fn test_complement_base() {
    assert_eq!(complement_base(0), 3); // A -> T
    assert_eq!(complement_base(1), 2); // C -> G
    assert_eq!(complement_base(2), 1); // G -> C
    assert_eq!(complement_base(3), 0); // T -> A
}

#[test]
fn test_add_minimizer() {
    let mut results: Vec<MinimizerResult> = Vec::new();
    let mut size: i32 = 0;
    add_minimizer(&mut results, &mut size, 42, 5, 7);
    assert_eq!(size, 1);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].minimizer_hash, 42);
    assert_eq!(results[0].kmer_position, 5);
    assert_eq!(results[0].smer_position, 7);

    add_minimizer(&mut results, &mut size, 99, 10, 12);
    assert_eq!(size, 2);
    assert_eq!(results.len(), 2);
    assert_eq!(results[1].minimizer_hash, 99);
    assert_eq!(results[1].kmer_position, 10);
    assert_eq!(results[1].smer_position, 12);
}

fn run_syncmers(seq: &str, k: i32, s: i32) -> Vec<(usize, u128, usize)> {
    let mut results: Vec<MinimizerResult> = Vec::new();
    let mut num_results: i32 = 0;
    compute_closed_syncmers(seq, seq.len() as i32, k, s, &mut results, &mut num_results);
    assert_eq!(num_results as usize, results.len());
    results.iter().map(|r| (r.kmer_position, r.minimizer_hash, r.smer_position)).collect()
}

// C ground truth: ACGTACGTACGT K=5 S=3 -> 6 results
// OPT 0 6 0, 1 6 1, 2 6 4, 4 6 4, 5 6 5, 6 6 8
#[test]
fn test_syncmers_acgtacgtacgt_k5_s3() {
    let r = run_syncmers("ACGTACGTACGT", 5, 3);
    assert_eq!(r.len(), 6);
    assert_eq!(r[0], (0, 6, 0));
    assert_eq!(r[1], (1, 6, 1));
    assert_eq!(r[2], (2, 6, 4));
    assert_eq!(r[3], (4, 6, 4));
    assert_eq!(r[4], (5, 6, 5));
    assert_eq!(r[5], (6, 6, 8));
}

// C ground truth: AAAAAAAAAA K=4 S=2 -> 7 results, all hash=0
#[test]
fn test_syncmers_all_a_k4_s2() {
    let r = run_syncmers("AAAAAAAAAA", 4, 2);
    assert_eq!(r.len(), 7);
    for i in 0..7 {
        assert_eq!(r[i], (i, 0, i));
    }
}

// C ground truth: ACGTACGT K=4 S=2 -> 3 results
// OPT 0 1 0, 2 1 2, 4 1 4
#[test]
fn test_syncmers_acgtacgt_k4_s2() {
    let r = run_syncmers("ACGTACGT", 4, 2);
    assert_eq!(r.len(), 3);
    assert_eq!(r[0], (0, 1, 0));
    assert_eq!(r[1], (2, 1, 2));
    assert_eq!(r[2], (4, 1, 4));
}

// C ground truth: TTTTTCCCCC K=5 S=3 -> 5 results
// OPT 0 0 0, 1 0 1, 2 0 2, 3 21 5, 5 21 5
#[test]
fn test_syncmers_tttttccccc_k5_s3() {
    let r = run_syncmers("TTTTTCCCCC", 5, 3);
    assert_eq!(r.len(), 5);
    assert_eq!(r[0], (0, 0, 0));
    assert_eq!(r[1], (1, 0, 1));
    assert_eq!(r[2], (2, 0, 2));
    assert_eq!(r[3], (3, 21, 5));
    assert_eq!(r[4], (5, 21, 5));
}

// C ground truth: GATTACA K=4 S=2 -> 2 results
// OPT 0 0 2, 2 0 2
#[test]
fn test_syncmers_gattaca_k4_s2() {
    let r = run_syncmers("GATTACA", 4, 2);
    assert_eq!(r.len(), 2);
    assert_eq!(r[0], (0, 0, 2));
    assert_eq!(r[1], (2, 0, 2));
}

// C ground truth: ACGTACGTACGTACGTACGT K=7 S=3 -> 8 results
#[test]
fn test_syncmers_long_k7_s3() {
    let r = run_syncmers("ACGTACGTACGTACGTACGT", 7, 3);
    assert_eq!(r.len(), 8);
    assert_eq!(r[0], (0, 6, 0));
    assert_eq!(r[1], (1, 6, 1));
    assert_eq!(r[2], (4, 6, 4));
    assert_eq!(r[3], (5, 6, 5));
    assert_eq!(r[4], (8, 6, 8));
    assert_eq!(r[5], (9, 6, 9));
    assert_eq!(r[6], (12, 6, 12));
    assert_eq!(r[7], (13, 6, 13));
}

// C ground truth: CCCCCCCCCC K=5 S=3 -> 6 results, all hash=21
#[test]
fn test_syncmers_all_c_k5_s3() {
    let r = run_syncmers("CCCCCCCCCC", 5, 3);
    assert_eq!(r.len(), 6);
    for i in 0..6 {
        assert_eq!(r[i], (i, 21, i));
    }
}

// C ground truth: GATTACAGATTACA K=6 S=3 -> 4 results
// OPT 1 3 1, 4 4 4, 5 3 8, 8 3 8
#[test]
fn test_syncmers_gattacagattaca_k6_s3() {
    let r = run_syncmers("GATTACAGATTACA", 6, 3);
    assert_eq!(r.len(), 4);
    assert_eq!(r[0], (1, 3, 1));
    assert_eq!(r[1], (4, 4, 4));
    assert_eq!(r[2], (5, 3, 8));
    assert_eq!(r[3], (8, 3, 8));
}

// C ground truth: AAAAA K=5 S=2 -> 1 result: 0 0 0
#[test]
fn test_syncmers_exact_length_k5_s2() {
    let r = run_syncmers("AAAAA", 5, 2);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0], (0, 0, 0));
}

// C ground truth: seq < K -> returns 0 results (prints error to stderr)
#[test]
fn test_syncmers_seq_shorter_than_k() {
    let r = run_syncmers("ACGT", 5, 3);
    assert_eq!(r.len(), 0);
}

fn main() {}
