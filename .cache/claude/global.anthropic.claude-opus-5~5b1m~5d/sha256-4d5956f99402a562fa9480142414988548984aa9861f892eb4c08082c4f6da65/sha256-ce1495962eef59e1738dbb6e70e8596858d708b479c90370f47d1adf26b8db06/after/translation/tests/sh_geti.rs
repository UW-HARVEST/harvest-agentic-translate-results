//! Phase B — CONFIGS.md rows 80..86
//! `sh_geti`, the top-level driver.  It prints through libc `printf`, so the
//! comparison is done on the raw bytes written to fd 1.
//!
//! The capture happens inside a subprocess (`common::sh_geti_diff`) so that
//! libtest's own progress output can never end up in the captured stream.

mod common;
use common::*;
use std::ffi::c_int;

/// The output `sh_geti(num)` must produce, derived from the C source:
/// two passes (`j = 0` strdup, `j = 1` arena); each prints `test_i i*3` for
/// even `i < num`, in *array* (insertion) order.
fn expected_output(num: c_int) -> String {
    if num <= 0 {
        return String::new();
    }
    let mut s = String::new();
    for _pass in 0..2 {
        let mut i = 0i64;
        while i < num as i64 {
            s.push_str(&format!("test_{} {}\n", i, i * 3));
            i += 2;
        }
    }
    s
}

/// Structural model, independent of `expected_output`.
fn check_shape(num: c_int, out: &[u8]) {
    if num <= 0 {
        assert!(out.is_empty(), "sh_geti({}) must print nothing", num);
        return;
    }
    let n = num as usize;
    let entries = n.div_ceil(2); // i = 0,2,4,... < num
    let text = String::from_utf8(out.to_vec()).expect("output must be UTF-8 here");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines.len(),
        2 * entries,
        "sh_geti({}) must print {} lines (2 passes x {} entries)",
        num,
        2 * entries,
        entries
    );
    for chunk in lines.chunks(entries) {
        let mut seen: Vec<usize> = Vec::new();
        for l in chunk {
            let mut it = l.split(' ');
            let key = it.next().unwrap();
            let val: i64 = it.next().unwrap().parse().unwrap();
            assert!(it.next().is_none(), "unexpected extra field in {:?}", l);
            let i: usize = key.strip_prefix("test_").unwrap().parse().unwrap();
            assert_eq!(i % 2, 0, "only even keys are inserted");
            assert!(i < n);
            assert_eq!(val, (i as i64) * 3, "value must be i*3");
            seen.push(i);
        }
        seen.sort_unstable();
        let want: Vec<usize> = (0..n).step_by(2).collect();
        assert_eq!(seen, want, "every even key must appear exactly once");
    }
}

fn run_and_check(seed: usize, nums: &[c_int]) -> Vec<Vec<u8>> {
    let outs = sh_geti_diff(seed, nums);
    for (i, &num) in nums.iter().enumerate() {
        check_shape(num, &outs[i]);
        assert_eq!(
            show(&outs[i]),
            expected_output(num),
            "exact output for num={} (seed {:#x})",
            num,
            seed
        );
    }
    outs
}

// -------------------------------------------------------------------- row 80
#[test]
fn c80_sh_geti_small() {
    let nums: Vec<c_int> = (0..=9).collect();
    run_and_check(DEFAULT_SEED, &nums);
}

// -------------------------------------------------------------------- row 81
#[test]
fn c81_sh_geti_mid() {
    let nums: Vec<c_int> = (10..=40).collect();
    run_and_check(DEFAULT_SEED, &nums);
}

// -------------------------------------------------------------------- row 82
#[test]
fn c82_sh_geti_power_of_two_boundaries() {
    let nums: Vec<c_int> = vec![
        15, 16, 17, 31, 32, 33, 47, 48, 63, 64, 65, 95, 96, 127, 128, 129,
    ];
    run_and_check(DEFAULT_SEED, &nums);
}

// -------------------------------------------------------------------- row 83
#[test]
fn c83_sh_geti_large() {
    let nums: Vec<c_int> = vec![200, 255, 256, 257, 500, 1000, 2000, 2048, 4096];
    run_and_check(DEFAULT_SEED, &nums);
}

// -------------------------------------------------------------------- row 84
#[test]
fn c84_sh_geti_non_positive() {
    let nums: Vec<c_int> = vec![0, -1, -2, -1000, c_int::MIN];
    let outs = run_and_check(DEFAULT_SEED, &nums);
    for o in outs {
        assert!(o.is_empty());
    }
}

// -------------------------------------------------------------------- row 85
#[test]
fn c85_sh_geti_repeated_calls_share_globals() {
    // `stbds_hash_seed` and the static `buffer` persist across calls, so the
    // whole sequence has to match, not just the individual calls.
    let mut rng = Rng::new(0x8585);
    let nums: Vec<c_int> = (0..30).map(|_| (rng.below(60)) as c_int).collect();
    let out = sh_geti_diff_sequence(DEFAULT_SEED, &nums);
    assert!(!out.is_empty());
    // the concatenation of the per-call canonical outputs
    let want: String = nums.iter().map(|&n| expected_output(n)).collect();
    assert_eq!(show(&out), want, "sequence output for nums = {:?}", nums);
}

// -------------------------------------------------------------------- row 86
#[test]
fn c86_sh_geti_seed_dependence() {
    let mut rng = Rng::new(0x8686);
    let mut seeds = vec![0usize, 1, 2, usize::MAX, DEFAULT_SEED];
    for _ in 0..8 {
        seeds.push(rng.next_u64() as usize);
    }
    let nums: Vec<c_int> = vec![7, 33, 100];
    let mut per_seed = Vec::new();
    for s in &seeds {
        let outs = run_and_check(*s, &nums);
        per_seed.push(outs[1].clone()); // num == 33
    }
    // `sh_geti` prints by walking the *array* (insertion order), not the hash
    // table, so the output is deliberately independent of the hash seed even
    // though the bucket layout is not.
    let distinct: std::collections::HashSet<Vec<u8>> = per_seed.iter().cloned().collect();
    assert_eq!(
        distinct.len(),
        1,
        "sh_geti prints in insertion order, so the seed must not change the output"
    );
}

// exact-output cross-check at additional sizes
#[test]
fn c80b_sh_geti_exact_output() {
    let nums: Vec<c_int> = vec![0, 1, 2, 3, 4, 5, 8, 9, 16, 17, 33, 64, 100, 128, 257, 1000];
    run_and_check(DEFAULT_SEED, &nums);
}
