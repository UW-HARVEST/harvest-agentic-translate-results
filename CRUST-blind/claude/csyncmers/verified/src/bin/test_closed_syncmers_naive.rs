use csyncmers::closed_syncmers::MinimizerResult;
use csyncmers::closed_syncmers_naive::compute_closed_syncmers_naive;

fn run(seq: &str, k: i32, s: i32) -> (Vec<MinimizerResult>, i32) {
    let mut results: Vec<MinimizerResult> = Vec::new();
    let mut n: i32 = 0;
    compute_closed_syncmers_naive(seq, seq.len(), k, s, &mut results, &mut n);
    (results, n)
}

fn assert_match(
    got: &[MinimizerResult],
    expected: &[(usize, usize, u128)],
) {
    assert_eq!(got.len(), expected.len(), "result length mismatch");
    for (i, (g, e)) in got.iter().zip(expected.iter()).enumerate() {
        assert_eq!(g.kmer_position, e.0, "kmer_position mismatch at index {}", i);
        assert_eq!(g.smer_position, e.1, "smer_position mismatch at index {}", i);
        assert_eq!(g.minimizer_hash, e.2, "minimizer_hash mismatch at index {}", i);
    }
}

#[test]
fn test_naive_acgt5_3() {
    let (got, n) = run("ACGTA", 5, 3);
    assert_eq!(n, 1);
    assert_match(&got, &[(0, 0, 6)]);
}

#[test]
fn test_naive_acgt6_3() {
    let (got, n) = run("ACGTAC", 5, 3);
    assert_eq!(n, 2);
    assert_match(&got, &[(0, 0, 6), (1, 1, 6)]);
}

#[test]
fn test_naive_acgt8_3() {
    let (got, n) = run("ACGTACGT", 5, 3);
    assert_eq!(n, 3);
    assert_match(&got, &[(0, 0, 6), (1, 1, 6), (2, 4, 6)]);
}

#[test]
fn test_naive_periodic_5_2() {
    let (got, n) = run("ACGTACGTAC", 5, 2);
    assert_eq!(n, 3);
    assert_match(&got, &[(0, 0, 1), (2, 2, 1), (4, 4, 1)]);
}

#[test]
fn test_naive_repeat_a() {
    let (got, n) = run("AAAAAAAAAA", 5, 2);
    assert_eq!(n, 6);
    assert_match(
        &got,
        &[
            (0, 0, 0),
            (1, 1, 0),
            (2, 2, 0),
            (3, 3, 0),
            (4, 4, 0),
            (5, 5, 0),
        ],
    );
}

#[test]
fn test_naive_mixed_6_3() {
    let (got, n) = run("AAACCCGGGTTT", 6, 3);
    assert_eq!(n, 7);
    assert_match(
        &got,
        &[
            (0, 0, 0),
            (1, 1, 1),
            (2, 2, 5),
            (3, 3, 21),
            (4, 7, 5),
            (5, 8, 1),
            (6, 9, 0),
        ],
    );
}

#[test]
fn test_naive_mixed_6_2() {
    let (got, n) = run("AAACCCGGGTTT", 6, 2);
    assert_eq!(n, 6);
    assert_match(
        &got,
        &[
            (0, 0, 0),
            (1, 1, 0),
            (2, 2, 1),
            (3, 3, 5),
            (4, 8, 1),
            (5, 9, 0),
        ],
    );
}

#[test]
fn test_naive_lowercase_input() {
    let (got, n) = run("acgtacgtac", 5, 2);
    assert_eq!(n, 3);
    assert_match(&got, &[(0, 0, 1), (2, 2, 1), (4, 4, 1)]);
}

#[test]
fn test_naive_k_equals_len() {
    let (got, n) = run("ACGTACGT", 8, 3);
    assert_eq!(n, 1);
    assert_match(&got, &[(0, 0, 6)]);
}

#[test]
fn test_naive_k_equals_s() {
    let (got, n) = run("ACGTACGT", 4, 4);
    assert_eq!(n, 5);
    assert_match(
        &got,
        &[
            (0, 0, 27),
            (1, 1, 108),
            (2, 2, 177),
            (3, 3, 108),
            (4, 4, 27),
        ],
    );
}

#[test]
fn test_naive_handcrafted_15_5() {
    let (got, n) = run(
        "TGCAGTCAGCATCGATCGATCGTAGCTAGCTAGCTGCATCGTAGCTAGCATCGATCGTACGT",
        15,
        5,
    );
    assert_eq!(n, 10);
    assert_match(
        &got,
        &[
            (1, 1, 121),
            (7, 7, 147),
            (8, 18, 99),
            (18, 18, 99),
            (22, 22, 156),
            (23, 23, 156),
            (26, 26, 156),
            (27, 37, 99),
            (37, 37, 99),
            (43, 53, 99),
        ],
    );
}

#[test]
fn test_naive_handcrafted_7_3() {
    let (got, n) = run("GATTACAGATTACAGATTACAGATTACA", 7, 3);
    assert_eq!(n, 6);
    assert_match(
        &got,
        &[
            (1, 1, 3),
            (4, 8, 3),
            (8, 8, 3),
            (11, 15, 3),
            (15, 15, 3),
            (18, 22, 3),
        ],
    );
}

#[test]
fn test_naive_longer_random_12_4() {
    let (got, n) = run(
        "ACGTGCATCGATCGTACGATCGATCGTAGCATCGATGCTAGCATCGATGCATGCAGCTAGC",
        12,
        4,
    );
    assert_eq!(n, 11);
    assert_match(
        &got,
        &[
            (0, 0, 27),
            (3, 11, 24),
            (11, 11, 24),
            (15, 15, 24),
            (23, 23, 24),
            (27, 27, 36),
            (35, 35, 36),
            (39, 39, 36),
            (42, 42, 54),
            (44, 44, 54),
            (46, 54, 39),
        ],
    );
}

#[test]
fn test_naive_len_less_than_k_returns_no_results() {
    // Naive Rust guards seq_len < k_usize before doing arithmetic.
    let mut results: Vec<MinimizerResult> = vec![MinimizerResult {
        minimizer_hash: 555,
        kmer_position: 7,
        smer_position: 7,
    }];
    let mut n: i32 = 9;
    compute_closed_syncmers_naive("ACGT", 4, 5, 3, &mut results, &mut n);
    assert_eq!(n, 0);
    assert!(results.is_empty(), "results should be cleared when seq_len < K");
}

#[test]
fn test_naive_clears_input_results_vec() {
    let mut results: Vec<MinimizerResult> = vec![MinimizerResult {
        minimizer_hash: 1234,
        kmer_position: 999,
        smer_position: 999,
    }];
    let mut n: i32 = 100;
    compute_closed_syncmers_naive("ACGTA", 5, 5, 3, &mut results, &mut n);
    assert_eq!(n, 1);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].minimizer_hash, 6);
    assert_eq!(results[0].kmer_position, 0);
    assert_eq!(results[0].smer_position, 0);
}

#[test]
fn test_naive_matches_fast_on_random_like_inputs() {
    // The C test suite uses agreement between the two implementations as the
    // ground truth. We reproduce a deterministic set of inputs here.
    use csyncmers::closed_syncmers::compute_closed_syncmers;

    let inputs: &[(&str, i32, i32)] = &[
        ("ACGTACGTACGTACGTACGTACGT", 11, 5),
        ("AAAAAACCCCCCGGGGGGTTTTTT", 9, 3),
        ("CGTAGCTAGCTAGCTAGCTAGCTAGCTAGCTA", 13, 4),
        (
            "TGCAGTCAGCATCGATCGATCGTAGCTAGCTAGCTGCATCGTAGCTAGCATCGATCGTACGT",
            15,
            5,
        ),
        ("GATTACAGATTACAGATTACAGATTACA", 7, 3),
    ];

    for (seq, k, s) in inputs {
        let mut fast_results: Vec<MinimizerResult> = Vec::new();
        let mut fast_n: i32 = 0;
        compute_closed_syncmers(seq, seq.len() as i32, *k, *s, &mut fast_results, &mut fast_n);

        let mut naive_results: Vec<MinimizerResult> = Vec::new();
        let mut naive_n: i32 = 0;
        compute_closed_syncmers_naive(seq, seq.len(), *k, *s, &mut naive_results, &mut naive_n);

        assert_eq!(
            fast_n, naive_n,
            "count mismatch for seq={:?} K={} S={}",
            seq, k, s
        );
        assert_eq!(fast_results.len(), naive_results.len());
        for (i, (a, b)) in fast_results.iter().zip(naive_results.iter()).enumerate() {
            assert_eq!(
                a.kmer_position, b.kmer_position,
                "kmer_position mismatch at index {} for seq={:?}",
                i, seq
            );
            assert_eq!(
                a.smer_position, b.smer_position,
                "smer_position mismatch at index {} for seq={:?}",
                i, seq
            );
            assert_eq!(
                a.minimizer_hash, b.minimizer_hash,
                "minimizer_hash mismatch at index {} for seq={:?}",
                i, seq
            );
        }
    }
}

fn main() {}
