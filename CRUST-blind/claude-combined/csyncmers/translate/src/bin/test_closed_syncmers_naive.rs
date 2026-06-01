use csyncmers::closed_syncmers::MinimizerResult;
use csyncmers::closed_syncmers_naive::compute_closed_syncmers_naive;

fn run_naive(seq: &str, k: i32, s: i32) -> (Vec<MinimizerResult>, i32) {
    let mut results: Vec<MinimizerResult> = Vec::new();
    let mut num: i32 = 0;
    compute_closed_syncmers_naive(seq, seq.len(), k, s, &mut results, &mut num);
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
fn test_naive_case1() {
    let (res, num) = run_naive("ACGTACGTAC", 5, 2);
    assert_eq!(num, 3);
    assert_entries(
        &res,
        &[(0, 0, 1u128), (2, 2, 1u128), (4, 4, 1u128)],
    );
}

#[test]
fn test_naive_case2_all_a() {
    let (res, num) = run_naive("AAAAAAAAAA", 5, 2);
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
fn test_naive_case3_periodic() {
    let (res, num) = run_naive("ACGTACGTACGTACGTACGT", 6, 3);
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
fn test_naive_case4_long_periodic() {
    let (res, num) = run_naive(
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
fn test_naive_case5_mixed() {
    let (res, num) = run_naive("ACGTGGCCAATTACGTAGCTAGCTACGATCGAT", 8, 3);
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
fn test_naive_case7_k_equals_len() {
    let (res, num) = run_naive("ACGTACGT", 8, 3);
    assert_eq!(num, 1);
    assert_entries(&res, &[(0, 0, 6u128)]);
}

#[test]
fn test_naive_case8_k_equals_s() {
    let (res, num) = run_naive("ACGTACGTAC", 4, 4);
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
fn test_naive_case9_gattaca() {
    let (res, num) = run_naive("GATTACAGATTACAGATTACA", 7, 3);
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

fn main() {}
