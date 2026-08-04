use csyncmers::closed_syncmers::{
    add_minimizer, base_to_bits, complement_base, compute_closed_syncmers, MinimizerResult,
};

fn run(seq: &str, k: i32, s: i32) -> (Vec<MinimizerResult>, i32) {
    let mut results: Vec<MinimizerResult> = Vec::new();
    let mut num: i32 = 0;
    compute_closed_syncmers(seq, seq.len() as i32, k, s, &mut results, &mut num);
    results.truncate(num as usize);
    (results, num)
}

fn assert_entries(actual: &[MinimizerResult], expected: &[(usize, usize, u128)]) {
    assert_eq!(actual.len(), expected.len(), "result count mismatch");
    for (i, exp) in expected.iter().enumerate() {
        assert_eq!(actual[i].kmer_position, exp.0, "kmer_position at {}", i);
        assert_eq!(actual[i].smer_position, exp.1, "smer_position at {}", i);
        assert_eq!(actual[i].minimizer_hash, exp.2, "minimizer_hash at {}", i);
    }
}

#[test]
fn test_base_to_bits_all() {
    assert_eq!(base_to_bits('A'), 0);
    assert_eq!(base_to_bits('a'), 0);
    assert_eq!(base_to_bits('C'), 1);
    assert_eq!(base_to_bits('c'), 1);
    assert_eq!(base_to_bits('G'), 2);
    assert_eq!(base_to_bits('g'), 2);
    assert_eq!(base_to_bits('T'), 3);
    assert_eq!(base_to_bits('t'), 3);
    // unknown should map to 0
    assert_eq!(base_to_bits('N'), 0);
    assert_eq!(base_to_bits('Z'), 0);
}

#[test]
fn test_complement_base() {
    assert_eq!(complement_base(0), 3); // A -> T
    assert_eq!(complement_base(1), 2); // C -> G
    assert_eq!(complement_base(2), 1); // G -> C
    assert_eq!(complement_base(3), 0); // T -> A
}

#[test]
fn test_add_minimizer_appends() {
    let mut results: Vec<MinimizerResult> = Vec::new();
    let mut size: i32 = 0;
    add_minimizer(&mut results, &mut size, 42u128, 5usize, 7usize);
    assert_eq!(size, 1);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].minimizer_hash, 42u128);
    assert_eq!(results[0].kmer_position, 5);
    assert_eq!(results[0].smer_position, 7);

    add_minimizer(&mut results, &mut size, 100u128, 9usize, 11usize);
    assert_eq!(size, 2);
    assert_eq!(results.len(), 2);
    assert_eq!(results[1].minimizer_hash, 100u128);
    assert_eq!(results[1].kmer_position, 9);
    assert_eq!(results[1].smer_position, 11);
}

#[test]
fn test_case1_acgtacgtac_k5_s2() {
    let (res, num) = run("ACGTACGTAC", 5, 2);
    assert_eq!(num, 3);
    assert_entries(
        &res,
        &[(0, 0, 1u128), (2, 2, 1u128), (4, 4, 1u128)],
    );
}

#[test]
fn test_case2_all_a() {
    let (res, num) = run("AAAAAAAAAA", 5, 2);
    assert_eq!(num, 6);
    assert_entries(
        &res,
        &[
            (0, 0, 0u128),
            (1, 1, 0u128),
            (2, 2, 0u128),
            (3, 3, 0u128),
            (4, 4, 0u128),
            (5, 5, 0u128),
        ],
    );
}

#[test]
fn test_case3_periodic_acgt_k6_s3() {
    let (res, num) = run("ACGTACGTACGTACGTACGT", 6, 3);
    assert_eq!(num, 8);
    assert_entries(
        &res,
        &[
            (0, 0, 6u128),
            (1, 1, 6u128),
            (4, 4, 6u128),
            (5, 5, 6u128),
            (8, 8, 6u128),
            (9, 9, 6u128),
            (12, 12, 6u128),
            (13, 13, 6u128),
        ],
    );
}

#[test]
fn test_case4_long_periodic_k10_s4() {
    let (res, num) = run(
        "ACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGT",
        10,
        4,
    );
    assert_eq!(num, 9);
    assert_entries(
        &res,
        &[
            (0, 0, 27u128),
            (4, 4, 27u128),
            (8, 8, 27u128),
            (12, 12, 27u128),
            (16, 16, 27u128),
            (20, 20, 27u128),
            (24, 24, 27u128),
            (28, 28, 27u128),
            (32, 32, 27u128),
        ],
    );
}

#[test]
fn test_case5_mixed_k8_s3() {
    let (res, num) = run("ACGTGGCCAATTACGTAGCTAGCTACGATCGAT", 8, 3);
    assert_eq!(num, 12);
    assert_entries(
        &res,
        &[
            (0, 0, 6u128),
            (1, 1, 6u128),
            (2, 7, 16u128),
            (3, 8, 3u128),
            (8, 8, 3u128),
            (9, 9, 3u128),
            (12, 12, 6u128),
            (13, 13, 6u128),
            (16, 16, 9u128),
            (17, 17, 9u128),
            (19, 24, 6u128),
            (24, 24, 6u128),
        ],
    );
}

#[test]
fn test_case7_k_equals_len() {
    let (res, num) = run("ACGTACGT", 8, 3);
    assert_eq!(num, 1);
    assert_entries(&res, &[(0, 0, 6u128)]);
}

#[test]
fn test_case8_k_equals_s() {
    let (res, num) = run("ACGTACGTAC", 4, 4);
    assert_eq!(num, 7);
    assert_entries(
        &res,
        &[
            (0, 0, 27u128),
            (1, 1, 108u128),
            (2, 2, 177u128),
            (3, 3, 108u128),
            (4, 4, 27u128),
            (5, 5, 108u128),
            (6, 6, 177u128),
        ],
    );
}

#[test]
fn test_case9_gattaca() {
    let (res, num) = run("GATTACAGATTACAGATTACA", 7, 3);
    assert_eq!(num, 4);
    assert_entries(
        &res,
        &[
            (1, 1, 3u128),
            (4, 8, 3u128),
            (8, 8, 3u128),
            (11, 15, 3u128),
        ],
    );
}

#[test]
fn test_len_less_than_k_returns_zero() {
    // Sequence shorter than K — should return num_results == 0
    let mut results: Vec<MinimizerResult> = Vec::new();
    let mut num: i32 = 7; // intentional non-zero starting value should be reset
    compute_closed_syncmers("ACGT", 4, 5, 2, &mut results, &mut num);
    assert_eq!(num, 0);
    assert_eq!(results.len(), 0);
}

fn main() {}
