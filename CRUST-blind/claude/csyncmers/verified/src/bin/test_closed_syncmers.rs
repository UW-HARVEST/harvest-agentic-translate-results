use csyncmers::closed_syncmers::{
    add_minimizer, base_to_bits, complement_base, compute_closed_syncmers, MinimizerResult,
};

fn run(seq: &str, k: i32, s: i32) -> (Vec<MinimizerResult>, i32) {
    let mut results: Vec<MinimizerResult> = Vec::new();
    let mut n: i32 = 0;
    compute_closed_syncmers(seq, seq.len() as i32, k, s, &mut results, &mut n);
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
fn test_base_to_bits_unknown_returns_zero() {
    assert_eq!(base_to_bits('N'), 0);
    assert_eq!(base_to_bits('n'), 0);
    assert_eq!(base_to_bits('X'), 0);
    assert_eq!(base_to_bits(' '), 0);
    assert_eq!(base_to_bits('U'), 0);
    assert_eq!(base_to_bits('1'), 0);
}

#[test]
fn test_complement_base() {
    assert_eq!(complement_base(0), 3); // A -> T
    assert_eq!(complement_base(1), 2); // C -> G
    assert_eq!(complement_base(2), 1); // G -> C
    assert_eq!(complement_base(3), 0); // T -> A
}

#[test]
fn test_add_minimizer_pushes_and_increments() {
    let mut results: Vec<MinimizerResult> = Vec::new();
    let mut size: i32 = 0;
    add_minimizer(&mut results, &mut size, 42u128, 7usize, 9usize);
    assert_eq!(size, 1);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].minimizer_hash, 42u128);
    assert_eq!(results[0].kmer_position, 7);
    assert_eq!(results[0].smer_position, 9);

    add_minimizer(&mut results, &mut size, 100u128, 11usize, 13usize);
    assert_eq!(size, 2);
    assert_eq!(results.len(), 2);
    assert_eq!(results[1].minimizer_hash, 100u128);
    assert_eq!(results[1].kmer_position, 11);
    assert_eq!(results[1].smer_position, 13);
}

#[test]
fn test_compute_acgt5_3() {
    // expected from C: kmer=0 smer=0 hash=6
    let (got, n) = run("ACGTA", 5, 3);
    assert_eq!(n, 1);
    assert_match(&got, &[(0, 0, 6)]);
}

#[test]
fn test_compute_acgt6_3() {
    let (got, n) = run("ACGTAC", 5, 3);
    assert_eq!(n, 2);
    assert_match(&got, &[(0, 0, 6), (1, 1, 6)]);
}

#[test]
fn test_compute_acgt8_3() {
    let (got, n) = run("ACGTACGT", 5, 3);
    assert_eq!(n, 3);
    assert_match(&got, &[(0, 0, 6), (1, 1, 6), (2, 4, 6)]);
}

#[test]
fn test_compute_periodic_5_2() {
    let (got, n) = run("ACGTACGTAC", 5, 2);
    assert_eq!(n, 3);
    assert_match(&got, &[(0, 0, 1), (2, 2, 1), (4, 4, 1)]);
}

#[test]
fn test_compute_repeat_a() {
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
fn test_compute_mixed_6_3() {
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
fn test_compute_mixed_6_2() {
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
fn test_compute_lowercase_input() {
    // Lowercase should be treated identically to uppercase.
    let (got, n) = run("acgtacgtac", 5, 2);
    assert_eq!(n, 3);
    assert_match(&got, &[(0, 0, 1), (2, 2, 1), (4, 4, 1)]);
}

#[test]
fn test_compute_k_equals_len() {
    let (got, n) = run("ACGTACGT", 8, 3);
    assert_eq!(n, 1);
    assert_match(&got, &[(0, 0, 6)]);
}

#[test]
fn test_compute_len_less_than_k_returns_no_results() {
    let mut results: Vec<MinimizerResult> = vec![MinimizerResult {
        minimizer_hash: 999,
        kmer_position: 99,
        smer_position: 99,
    }];
    let mut n: i32 = 5;
    compute_closed_syncmers("ACGT", 4, 5, 3, &mut results, &mut n);
    assert_eq!(n, 0);
    assert!(
        results.is_empty(),
        "results should be cleared when len < K"
    );
}

#[test]
fn test_compute_k_equals_s() {
    // window_size = K - S + 1 = 1; every k-mer is a closed syncmer.
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
fn test_compute_handcrafted_15_5() {
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
fn test_compute_handcrafted_7_3() {
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
fn test_compute_longer_random_12_4() {
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
fn test_compute_clears_input_results_vec() {
    // Ensure pre-existing results get cleared.
    let mut results: Vec<MinimizerResult> = vec![MinimizerResult {
        minimizer_hash: 1234,
        kmer_position: 999,
        smer_position: 999,
    }];
    let mut n: i32 = 100;
    compute_closed_syncmers("ACGTA", 5, 5, 3, &mut results, &mut n);
    assert_eq!(n, 1);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].minimizer_hash, 6);
    assert_eq!(results[0].kmer_position, 0);
    assert_eq!(results[0].smer_position, 0);
}

fn main() {}
