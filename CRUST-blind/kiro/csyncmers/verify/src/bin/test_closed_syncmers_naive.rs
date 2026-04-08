use csyncmers::closed_syncmers::MinimizerResult;
use csyncmers::closed_syncmers_naive::compute_closed_syncmers_naive;

fn run_naive(seq: &str, k: i32, s: i32) -> Vec<(usize, u128, usize)> {
    let mut results: Vec<MinimizerResult> = Vec::new();
    let mut num_results: i32 = 0;
    compute_closed_syncmers_naive(seq, seq.len(), k, s, &mut results, &mut num_results);
    assert_eq!(num_results as usize, results.len());
    results.iter().map(|r| (r.kmer_position, r.minimizer_hash, r.smer_position)).collect()
}

// C ground truth: ACGTACGTACGT K=5 S=3 -> 6 results
// NAIVE 0 6 0, 1 6 1, 2 6 4, 4 6 4, 5 6 5, 6 6 8
#[test]
fn test_naive_acgtacgtacgt_k5_s3() {
    let r = run_naive("ACGTACGTACGT", 5, 3);
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
fn test_naive_all_a_k4_s2() {
    let r = run_naive("AAAAAAAAAA", 4, 2);
    assert_eq!(r.len(), 7);
    for i in 0..7 {
        assert_eq!(r[i], (i, 0, i));
    }
}

// C ground truth: ACGTACGT K=4 S=2 -> 3 results
#[test]
fn test_naive_acgtacgt_k4_s2() {
    let r = run_naive("ACGTACGT", 4, 2);
    assert_eq!(r.len(), 3);
    assert_eq!(r[0], (0, 1, 0));
    assert_eq!(r[1], (2, 1, 2));
    assert_eq!(r[2], (4, 1, 4));
}

// C ground truth: TTTTTCCCCC K=5 S=3 -> 5 results
#[test]
fn test_naive_tttttccccc_k5_s3() {
    let r = run_naive("TTTTTCCCCC", 5, 3);
    assert_eq!(r.len(), 5);
    assert_eq!(r[0], (0, 0, 0));
    assert_eq!(r[1], (1, 0, 1));
    assert_eq!(r[2], (2, 0, 2));
    assert_eq!(r[3], (3, 21, 5));
    assert_eq!(r[4], (5, 21, 5));
}

// C ground truth: GATTACA K=4 S=2 -> 2 results
#[test]
fn test_naive_gattaca_k4_s2() {
    let r = run_naive("GATTACA", 4, 2);
    assert_eq!(r.len(), 2);
    assert_eq!(r[0], (0, 0, 2));
    assert_eq!(r[1], (2, 0, 2));
}

// C ground truth: ACGTACGTACGTACGTACGT K=7 S=3 -> 8 results
#[test]
fn test_naive_long_k7_s3() {
    let r = run_naive("ACGTACGTACGTACGTACGT", 7, 3);
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
fn test_naive_all_c_k5_s3() {
    let r = run_naive("CCCCCCCCCC", 5, 3);
    assert_eq!(r.len(), 6);
    for i in 0..6 {
        assert_eq!(r[i], (i, 21, i));
    }
}

// C ground truth: GATTACAGATTACA K=6 S=3 -> 4 results
#[test]
fn test_naive_gattacagattaca_k6_s3() {
    let r = run_naive("GATTACAGATTACA", 6, 3);
    assert_eq!(r.len(), 4);
    assert_eq!(r[0], (1, 3, 1));
    assert_eq!(r[1], (4, 4, 4));
    assert_eq!(r[2], (5, 3, 8));
    assert_eq!(r[3], (8, 3, 8));
}

// C ground truth: AAAAA K=5 S=2 -> 1 result
#[test]
fn test_naive_exact_length_k5_s2() {
    let r = run_naive("AAAAA", 5, 2);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0], (0, 0, 0));
}

// Both methods should agree on all inputs
#[test]
fn test_naive_matches_optimized() {
    use csyncmers::closed_syncmers::compute_closed_syncmers;
    let cases = [
        ("ACGTACGTACGT", 5, 3),
        ("GATTACAGATTACA", 6, 3),
        ("TTTTTCCCCC", 5, 3),
        ("CCCCCCCCCC", 5, 3),
    ];
    for (seq, k, s) in cases {
        let mut r1: Vec<MinimizerResult> = Vec::new();
        let mut n1: i32 = 0;
        compute_closed_syncmers(seq, seq.len() as i32, k, s, &mut r1, &mut n1);

        let mut r2: Vec<MinimizerResult> = Vec::new();
        let mut n2: i32 = 0;
        compute_closed_syncmers_naive(seq, seq.len(), k, s, &mut r2, &mut n2);

        assert_eq!(n1, n2, "count mismatch for {seq} k={k} s={s}");
        for i in 0..n1 as usize {
            assert_eq!(r1[i].kmer_position, r2[i].kmer_position, "kmer_position mismatch at {i} for {seq}");
            assert_eq!(r1[i].minimizer_hash, r2[i].minimizer_hash, "minimizer_hash mismatch at {i} for {seq}");
            assert_eq!(r1[i].smer_position, r2[i].smer_position, "smer_position mismatch at {i} for {seq}");
        }
    }
}

fn main() {}
