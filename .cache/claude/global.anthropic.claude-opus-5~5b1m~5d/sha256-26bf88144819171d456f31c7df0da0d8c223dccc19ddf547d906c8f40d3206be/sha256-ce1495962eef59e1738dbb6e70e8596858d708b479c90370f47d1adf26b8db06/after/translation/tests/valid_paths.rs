// Phase B -- valid-path differential tests.
//
// One test per row of CONFIGS.md. Every test drives BOTH `c_src/build/
// libSieve.so` and `translation/target/<profile>/libSieve.so` through
// `dlopen`/`dlsym` (libloading) and compares the exact stdout byte stream.

mod common;

use common::*;

// --- row 1 -----------------------------------------------------------------
// A=positive, B=9, C=1: the loop-exit test fires on the very first iteration.
#[test]
fn cfg_01_single_digit_nine() {
    let out = assert_same(&[9]);
    assert_eq!(out, b"9\n", "sieve(9) must emit exactly one line");
    assert_eq!(out, expected(&[9]));
}

// --- row 2 -----------------------------------------------------------------
// A=zero, B=0, C=10.
#[test]
fn cfg_02_zero() {
    let out = assert_same(&[0]);
    assert_eq!(out, b"0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n");
    assert_eq!(out, expected(&[0]));
}

// --- row 3 -----------------------------------------------------------------
// Every single-digit positive start: remainder classes 1..8 -> 2..9 lines.
#[test]
fn cfg_03_all_single_digit_starts() {
    for v in 1..=8i64 {
        let out = assert_same(&[v]);
        // prints v..=9 inclusive, each line "<digit>\n" == 2 bytes
        assert_eq!(out.len() as i64, (10 - v) * 2, "line count for sieve({v})");
        assert_eq!(out, expected(&[v]));
    }
}

// --- row 4 -----------------------------------------------------------------
// Exhaustive two-digit starts: every remainder class 0..9 at a 2-digit width.
#[test]
fn cfg_04_two_digit_exhaustive() {
    let vals: Vec<i64> = (10..=99).collect();
    let out = assert_same(&vals);
    assert_eq!(out, expected(&vals));
}

// --- row 5 -----------------------------------------------------------------
// Randomized wide positives: every remainder class at 4..10 digit widths.
#[test]
fn cfg_05_random_positive_wide() {
    let mut rng = Pcg32::new(0x5EED_0005);
    let vals: Vec<i64> = (0..400).map(|_| rng.range(1_000, 1_000_000_000)).collect();
    let out = assert_same(&vals);
    assert_eq!(out, expected(&vals));
}

// --- row 6 -----------------------------------------------------------------
// Runs that cross a power-of-ten boundary, so the %d field width grows
// mid-loop (8 -> 9 stays, 98 -> 99, 999999998 -> 999999999, ...).
#[test]
fn cfg_06_positive_carry_across_power_of_ten() {
    let mut vals: Vec<i64> = vec![8, 98, 998, 9998, 99_998, 999_998, 9_999_998, 99_999_998, 999_999_998, 2_147_483_638];
    // ... plus the 10^k - 2 / 10^k - 1 / 10^k neighbourhoods.
    for k in 1..=9u32 {
        let p = 10i64.pow(k);
        vals.push(p - 2);
        vals.push(p - 1);
        vals.push(p);
        vals.push(p + 1);
    }
    let out = assert_same(&vals);
    assert_eq!(out, expected(&vals));
}

// --- row 7 -----------------------------------------------------------------
// F: the largest input that terminates without signed overflow.
#[test]
fn cfg_07_max_terminating_value() {
    let out = assert_same(&[2_147_483_639]);
    assert_eq!(out, b"2147483639\n");
}

// --- row 8 -----------------------------------------------------------------
// Exhaustive top-of-range window that still converges on 2147483639.
#[test]
fn cfg_08_top_of_range_exhaustive() {
    let vals: Vec<i64> = (2_147_483_630..=2_147_483_639).collect();
    let out = assert_same(&vals);
    assert_eq!(out, expected(&vals));
}

// --- row 9 -----------------------------------------------------------------
// A=negative, B=-9: "ends in 9" but C's truncated % yields -9, so no early
// exit -- the loop climbs all the way to +9.
#[test]
fn cfg_09_negative_nine() {
    let out = assert_same(&[-9]);
    assert_eq!(
        out,
        b"-9\n-8\n-7\n-6\n-5\n-4\n-3\n-2\n-1\n0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n"
    );
}

// --- row 10 ----------------------------------------------------------------
// Every negative remainder class -9..-1.
#[test]
fn cfg_10_negative_single_digit_exhaustive() {
    let vals: Vec<i64> = (-9..=-1).collect();
    let out = assert_same(&vals);
    assert_eq!(out, expected(&vals));
}

// --- row 11 ----------------------------------------------------------------
// Negative multiples of ten (remainder exactly 0 on the negative side).
#[test]
fn cfg_11_negative_multiples_of_ten() {
    let vals: Vec<i64> = vec![-10, -20, -100, -1000, -10_000];
    let out = assert_same(&vals);
    assert_eq!(out, expected(&vals));
}

// --- row 12 ----------------------------------------------------------------
// Exhaustive small negatives: widths 1..3 plus the -1 -> 0 sign transition.
#[test]
fn cfg_12_negative_exhaustive_small() {
    let vals: Vec<i64> = (-300..=-1).collect();
    let out = assert_same(&vals);
    assert_eq!(out, expected(&vals));
}

// --- row 13 ----------------------------------------------------------------
// Randomized negatives: long runs of varying length.
#[test]
fn cfg_13_random_negative() {
    let mut rng = Pcg32::new(0x5EED_0013);
    let vals: Vec<i64> = (0..200).map(|_| rng.range(-5_000, -1)).collect();
    let out = assert_same(&vals);
    assert_eq!(out, expected(&vals));
}

// --- row 14 ----------------------------------------------------------------
// ~10^5 iterations each: sustained stdio buffer refills.
#[test]
fn cfg_14_large_negative_long_run() {
    for v in [-99_999i64, -100_000, -123_457] {
        let out = assert_same(&[v]);
        assert_eq!(out, expected(&[v]));
    }
}

// --- row 15 ----------------------------------------------------------------
// ~10^6 iterations / ~9 MiB of output through the FFI boundary.
#[test]
fn cfg_15_million_line_run() {
    let out = assert_same(&[-1_000_000]);
    assert_eq!(out, expected(&[-1_000_000]));
    assert_eq!(out.iter().filter(|&&b| b == b'\n').count(), 1_000_010);
}

// --- row 16 ----------------------------------------------------------------
// G=0 calls: loading the library and resolving `sieve` must emit nothing.
#[test]
fn cfg_16_zero_calls() {
    let out = assert_same(&[]);
    assert!(out.is_empty(), "library load produced output: {out:?}");
}

// --- row 17 ----------------------------------------------------------------
// Many interleaved calls, mixed signs/widths, in one process: proves the
// concatenated stream matches and that no state leaks across calls.
#[test]
fn cfg_17_many_interleaved_calls() {
    let mut rng = Pcg32::new(0x5EED_0017);
    let vals: Vec<i64> = (0..300).map(|_| rng.range(-200, 200)).collect();
    let out = assert_same(&vals);
    assert_eq!(out, expected(&vals));
}

// --- row 18 ----------------------------------------------------------------
// H: stdout is a FIFO (non-seekable), a different stdio buffering path.
#[test]
fn cfg_18_stdout_is_a_pipe() {
    let mut rng = Pcg32::new(0x5EED_0018);
    let mut vals: Vec<i64> = (0..120).map(|_| rng.range(-400, 400)).collect();
    vals.extend_from_slice(&[9, 0, -9, 2_147_483_639]);
    let out = assert_same_fifo(&vals);
    assert_eq!(out, expected(&vals));
}

// --- row 19 ----------------------------------------------------------------
// Contiguous sweep across zero: every sign/width/remainder transition.
#[test]
fn cfg_19_contiguous_sweep() {
    let vals: Vec<i64> = (-64..=64).collect();
    let out = assert_same(&vals);
    assert_eq!(out, expected(&vals));
}

// --- row 20 ----------------------------------------------------------------
// Extreme-but-bounded bit patterns reinterpreted as `int`.
#[test]
fn cfg_20_extreme_bit_patterns() {
    let pats: [u32; 8] = [
        0x0000_0000, 0x0000_0001, 0x0000_0009, 0x7FFF_FFF7, 0x7FFF_FFF0, 0xFFFF_FFFF, 0xFFFF_FFF7,
        0xFFFF_FC18,
    ];
    let vals: Vec<i64> = pats.iter().map(|&p| p as i32 as i64).collect();
    let out = assert_same(&vals);
    assert_eq!(out, expected(&vals));
}

// --- row 21 ----------------------------------------------------------------
// Broad property-style fuzz over the whole terminating sub-domain.
#[test]
fn cfg_21_broad_random_fuzz() {
    let mut rng = Pcg32::new(0x5EED_0021);
    let mut vals: Vec<i64> = Vec::with_capacity(600);
    for i in 0..600 {
        let v = match i % 5 {
            0 | 1 | 2 => rng.range(0, 2_147_483_639),
            3 => rng.range(-3_000, -1),
            _ => {
                // cluster around interesting boundaries
                let base = [0i64, 9, 10, 99, 100, 2_147_483_639, -1, -10, -9][(rng.next_u32() % 9) as usize];
                base + rng.range(-5, 5)
            }
        };
        // stay inside the terminating, bounded-runtime domain
        vals.push(v.clamp(-3_000, 2_147_483_639));
    }
    let out = assert_same(&vals);
    assert_eq!(out, expected(&vals));
}

// --- row 22 ----------------------------------------------------------------
// Concurrent callers. `sieve` holds no static/global state, so N threads
// calling it simultaneously must produce the same multiset of lines as a
// sequential run (glibc holds the stdout lock across a whole printf, so
// individual lines are never torn). Interleaving order is not deterministic,
// hence the multiset comparison.
#[test]
fn cfg_22_concurrent_callers() {
    let mut rng = Pcg32::new(0x5EED_0022);
    let vals: Vec<i64> = (0..96).map(|_| rng.range(-150, 150)).collect();
    for threads in [2usize, 4, 8] {
        assert_same_multiset_threaded(&vals, threads);
    }
    // pathological: many threads, all on the immediate-exit input
    assert_same_multiset_threaded(&vec![9i64; 64], 16);
}
