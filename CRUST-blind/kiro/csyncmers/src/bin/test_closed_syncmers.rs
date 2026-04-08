use csyncmers::closed_syncmers::{base_to_bits, complement_base, compute_closed_syncmers, add_minimizer};
use csyncmers::closed_syncmers_naive::compute_closed_syncmers_naive;

// Helper: run compute_closed_syncmers and return (num_results, vec of (kmer_pos, hash, smer_pos))
fn run_optimized(seq: &str, k: i32, s: i32) -> (i32, Vec<(usize, u128, usize)>) {
    let mut results = Vec::new();
    let mut num = 0i32;
    compute_closed_syncmers(seq, seq.len() as i32, k, s, &mut results, &mut num);
    let tuples: Vec<_> = results.iter().map(|r| (r.kmer_position, r.minimizer_hash, r.smer_position)).collect();
    (num, tuples)
}

fn run_naive(seq: &str, k: i32, s: i32) -> (i32, Vec<(usize, u128, usize)>) {
    let mut results = Vec::new();
    let mut num = 0i32;
    compute_closed_syncmers_naive(seq, seq.len(), k, s, &mut results, &mut num);
    let tuples: Vec<_> = results.iter().map(|r| (r.kmer_position, r.minimizer_hash, r.smer_position)).collect();
    (num, tuples)
}

// ===== base_to_bits tests =====

#[test]
fn test_base_to_bits_uppercase() {
    assert_eq!(base_to_bits('A'), 0);
    assert_eq!(base_to_bits('C'), 1);
    assert_eq!(base_to_bits('G'), 2);
    assert_eq!(base_to_bits('T'), 3);
}

#[test]
fn test_base_to_bits_lowercase() {
    assert_eq!(base_to_bits('a'), 0);
    assert_eq!(base_to_bits('c'), 1);
    assert_eq!(base_to_bits('g'), 2);
    assert_eq!(base_to_bits('t'), 3);
}

#[test]
fn test_base_to_bits_unknown() {
    assert_eq!(base_to_bits('N'), 0);
    assert_eq!(base_to_bits('X'), 0);
    assert_eq!(base_to_bits('Z'), 0);
}

// ===== complement_base tests =====

#[test]
fn test_complement_base() {
    // A(0) <-> T(3), C(1) <-> G(2)
    assert_eq!(complement_base(0), 3);
    assert_eq!(complement_base(3), 0);
    assert_eq!(complement_base(1), 2);
    assert_eq!(complement_base(2), 1);
}

// ===== add_minimizer tests =====

#[test]
fn test_add_minimizer() {
    let mut results = Vec::new();
    let mut size = 0i32;
    add_minimizer(&mut results, &mut size, 42, 5, 7);
    assert_eq!(size, 1);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].minimizer_hash, 42);
    assert_eq!(results[0].kmer_position, 5);
    assert_eq!(results[0].smer_position, 7);

    add_minimizer(&mut results, &mut size, 99, 10, 12);
    assert_eq!(size, 2);
    assert_eq!(results.len(), 2);
}

// ===== compute_closed_syncmers tests =====

#[test]
fn test_acgtacgtacgt_k5_s3() {
    let (num, res) = run_optimized("ACGTACGTACGT", 5, 3);
    assert_eq!(num, 6);
    assert_eq!(res, vec![
        (0, 6, 0), (1, 6, 1), (2, 6, 4), (4, 6, 4), (5, 6, 5), (6, 6, 8),
    ]);
}

#[test]
fn test_all_a_k5_s3() {
    let (num, res) = run_optimized("AAAAAAAAAA", 5, 3);
    assert_eq!(num, 6);
    for (i, &(kpos, hash, spos)) in res.iter().enumerate() {
        assert_eq!(kpos, i);
        assert_eq!(hash, 0);
        assert_eq!(spos, i);
    }
}

#[test]
fn test_seq_shorter_than_k() {
    let (num, res) = run_optimized("ACGT", 5, 3);
    assert_eq!(num, 0);
    assert!(res.is_empty());
}

#[test]
fn test_acgtacgt_k4_s2() {
    let (num, res) = run_optimized("ACGTACGT", 4, 2);
    assert_eq!(num, 3);
    assert_eq!(res, vec![(0, 1, 0), (2, 1, 2), (4, 1, 4)]);
}

#[test]
fn test_long_seq_k7_s3() {
    let (num, res) = run_optimized("ACGTACGTACGTACGTACGT", 7, 3);
    assert_eq!(num, 8);
    assert_eq!(res, vec![
        (0, 6, 0), (1, 6, 1), (4, 6, 4), (5, 6, 5),
        (8, 6, 8), (9, 6, 9), (12, 6, 12), (13, 6, 13),
    ]);
}

#[test]
fn test_single_kmer_k6_s3() {
    let (num, res) = run_optimized("ACGTAC", 6, 3);
    assert_eq!(num, 1);
    assert_eq!(res, vec![(0, 6, 0)]);
}

#[test]
fn test_lowercase_same_as_uppercase() {
    let (num_upper, res_upper) = run_optimized("ACGTACGTACGT", 5, 3);
    let (num_lower, res_lower) = run_optimized("acgtacgtacgt", 5, 3);
    assert_eq!(num_upper, num_lower);
    assert_eq!(res_upper, res_lower);
}

#[test]
fn test_all_c_k5_s3() {
    let (num, res) = run_optimized("CCCCCCCCCC", 5, 3);
    assert_eq!(num, 6);
    for &(_, hash, _) in &res {
        assert_eq!(hash, 21);
    }
}

#[test]
fn test_all_t_k5_s3() {
    let (num, res) = run_optimized("TTTTTTTTTTT", 5, 3);
    assert_eq!(num, 7);
    for &(_, hash, _) in &res {
        assert_eq!(hash, 0);
    }
}

#[test]
fn test_gattaca_k5_s2() {
    let (num, res) = run_optimized("GATTACA", 5, 2);
    assert_eq!(num, 1);
    assert_eq!(res, vec![(2, 0, 2)]);
}

#[test]
fn test_seq_equals_k() {
    let (num, res) = run_optimized("ACGTACGT", 8, 3);
    assert_eq!(num, 1);
    assert_eq!(res, vec![(0, 6, 0)]);
}

#[test]
fn test_n_in_sequence() {
    let (num, res) = run_optimized("ACGTNNACGT", 5, 3);
    assert_eq!(num, 5);
    assert_eq!(res, vec![
        (0, 6, 0), (1, 6, 1), (2, 0, 4), (4, 0, 4), (5, 1, 5),
    ]);
}

#[test]
fn test_atcgatcgatcg_k6_s2() {
    let (num, res) = run_optimized("ATCGATCGATCG", 6, 2);
    assert_eq!(num, 2);
    assert_eq!(res, vec![(0, 3, 0), (4, 3, 4)]);
}

#[test]
fn test_nacgt_k5_s3() {
    let (num, res) = run_optimized("NACGT", 5, 3);
    assert_eq!(num, 1);
    assert_eq!(res, vec![(0, 1, 0)]);
}

#[test]
fn test_all_g_k5_s3() {
    let (num, res) = run_optimized("GGGGGGGGG", 5, 3);
    assert_eq!(num, 5);
    for &(_, hash, _) in &res {
        assert_eq!(hash, 21);
    }
}

// ===== compute_closed_syncmers_naive tests =====

#[test]
fn test_naive_acgtacgtacgt_k5_s3() {
    let (num, res) = run_naive("ACGTACGTACGT", 5, 3);
    assert_eq!(num, 6);
    assert_eq!(res, vec![
        (0, 6, 0), (1, 6, 1), (2, 6, 4), (4, 6, 4), (5, 6, 5), (6, 6, 8),
    ]);
}

#[test]
fn test_naive_all_a_k5_s3() {
    let (num, res) = run_naive("AAAAAAAAAA", 5, 3);
    assert_eq!(num, 6);
    for (i, &(kpos, hash, spos)) in res.iter().enumerate() {
        assert_eq!(kpos, i);
        assert_eq!(hash, 0);
        assert_eq!(spos, i);
    }
}

#[test]
fn test_naive_gattaca_k5_s2() {
    let (num, res) = run_naive("GATTACA", 5, 2);
    assert_eq!(num, 1);
    assert_eq!(res, vec![(2, 0, 2)]);
}

#[test]
fn test_naive_single_kmer() {
    let (num, res) = run_naive("ACGTAC", 6, 3);
    assert_eq!(num, 1);
    assert_eq!(res, vec![(0, 6, 0)]);
}

#[test]
fn test_naive_n_in_sequence() {
    let (num, res) = run_naive("ACGTNNACGT", 5, 3);
    assert_eq!(num, 5);
    assert_eq!(res, vec![
        (0, 6, 0), (1, 6, 1), (2, 0, 4), (4, 0, 4), (5, 1, 5),
    ]);
}

// ===== optimized vs naive agreement =====

#[test]
fn test_optimized_matches_naive() {
    let seqs = [
        ("ACGTACGTACGT", 5, 3),
        ("AAAAAAAAAA", 5, 3),
        ("ACGTACGT", 4, 2),
        ("ACGTACGTACGTACGTACGT", 7, 3),
        ("GATTACA", 5, 2),
        ("CCCCCCCCCC", 5, 3),
        ("TTTTTTTTTTT", 5, 3),
        ("ATCGATCGATCG", 6, 2),
        ("GGGGGGGGG", 5, 3),
        ("ACGTNNACGT", 5, 3),
    ];
    for &(seq, k, s) in &seqs {
        let (n_opt, r_opt) = run_optimized(seq, k, s);
        let (n_naive, r_naive) = run_naive(seq, k, s);
        assert_eq!(n_opt, n_naive, "count mismatch for seq={} k={} s={}", seq, k, s);
        assert_eq!(r_opt, r_naive, "results mismatch for seq={} k={} s={}", seq, k, s);
    }
}

fn main() {}
