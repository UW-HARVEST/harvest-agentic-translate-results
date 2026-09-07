//! Phase B — valid-path differential tests, one test per CONFIGS.md row.
//!
//! Every test drives BOTH shared objects through their exported C symbols
//! (never the Rust crate directly) and compares results byte-for-byte, using
//! many randomized inputs per row with a fixed seed.

mod common;

use common::*;
use std::ffi::c_int;

/// How many randomized inputs each row gets.
const ITERS: usize = 200;

// ===========================================================================
// Rows 1-5: `fma_array`, all pointers distinct, varying `len`
// ===========================================================================

#[test]
fn row01_fma_len0_distinct_leaves_out_untouched() {
    let mut rng = Rng::new(0x0000_0001);
    for _ in 0..ITERS {
        let a = gen_vals(&mut rng, ValueClass::FullRandom, 8);
        let b = gen_vals(&mut rng, ValueClass::FullRandom, 8);
        let c = gen_vals(&mut rng, ValueClass::FullRandom, 8);
        let o = gen_vals(&mut rng, ValueClass::FullRandom, 8);
        diff_fma("row01", Alias::AllDistinct, 0, &a, &b, &c, &o);

        // And confirm the shared semantics: nothing was written.
        let l = libs();
        let res = call_fma(&l.c, Alias::AllDistinct, 0, &a, &b, &c, &o);
        // `call_fma` seeds only the first `len+1` slots (here 1) from
        // `out_init`; the rest are SENTINEL. Nothing may have been written.
        assert_eq!(res.buffers[0][0], o[0], "len=0 must not write to out[0]");
        assert!(
            res.buffers[0][1..].iter().all(|&v| v == SENTINEL),
            "len=0 must not write to out"
        );
    }
}

#[test]
fn row02_fma_len1_distinct_random() {
    let mut rng = Rng::new(0x0000_0002);
    for _ in 0..ITERS {
        let a = gen_vals(&mut rng, ValueClass::FullRandom, 2);
        let b = gen_vals(&mut rng, ValueClass::FullRandom, 2);
        let c = gen_vals(&mut rng, ValueClass::FullRandom, 2);
        let o = gen_vals(&mut rng, ValueClass::FullRandom, 2);
        diff_fma("row02", Alias::AllDistinct, 1, &a, &b, &c, &o);
    }
}

#[test]
fn row03_fma_len2_distinct_random() {
    let mut rng = Rng::new(0x0000_0003);
    for _ in 0..ITERS {
        let a = gen_vals(&mut rng, ValueClass::FullRandom, 3);
        let b = gen_vals(&mut rng, ValueClass::FullRandom, 3);
        let c = gen_vals(&mut rng, ValueClass::FullRandom, 3);
        let o = gen_vals(&mut rng, ValueClass::FullRandom, 3);
        diff_fma("row03", Alias::AllDistinct, 2, &a, &b, &c, &o);
    }
}

#[test]
fn row04_fma_len_small_distinct_random() {
    let mut rng = Rng::new(0x0000_0004);
    for _ in 0..ITERS {
        let n = rng.range_usize(2, 8);
        let a = gen_vals(&mut rng, ValueClass::FullRandom, n + 1);
        let b = gen_vals(&mut rng, ValueClass::FullRandom, n + 1);
        let c = gen_vals(&mut rng, ValueClass::FullRandom, n + 1);
        let o = gen_vals(&mut rng, ValueClass::FullRandom, n + 1);
        diff_fma("row04", Alias::AllDistinct, n as c_int, &a, &b, &c, &o);
    }
}

#[test]
fn row05_fma_len_many_distinct_random() {
    let mut rng = Rng::new(0x0000_0005);
    for _ in 0..ITERS {
        let n = rng.range_usize(64, 1024);
        let a = gen_vals(&mut rng, ValueClass::FullRandom, n + 1);
        let b = gen_vals(&mut rng, ValueClass::FullRandom, n + 1);
        let c = gen_vals(&mut rng, ValueClass::FullRandom, n + 1);
        let o = gen_vals(&mut rng, ValueClass::FullRandom, n + 1);
        diff_fma("row05", Alias::AllDistinct, n as c_int, &a, &b, &c, &o);
    }
}

// ===========================================================================
// Rows 6-12: `fma_array`, len many, each value class (incl. both overflow kinds)
// ===========================================================================

fn fma_value_class_row(row: &str, seed: u64, class: ValueClass) {
    let mut rng = Rng::new(seed);
    for _ in 0..ITERS {
        let n = rng.range_usize(64, 512);
        let a = gen_vals(&mut rng, class, n + 1);
        let b = gen_vals(&mut rng, class, n + 1);
        let c = gen_vals(&mut rng, class, n + 1);
        let o = gen_vals(&mut rng, ValueClass::FullRandom, n + 1);
        diff_fma(row, Alias::AllDistinct, n as c_int, &a, &b, &c, &o);
    }
}

#[test]
fn row06_fma_all_zeros() {
    fma_value_class_row("row06", 0x0000_0006, ValueClass::Zeros);
}

#[test]
fn row07_fma_small_positive() {
    fma_value_class_row("row07", 0x0000_0007, ValueClass::SmallPos);
}

#[test]
fn row08_fma_small_negative() {
    fma_value_class_row("row08", 0x0000_0008, ValueClass::SmallNeg);
}

#[test]
fn row09_fma_mixed_sign_small() {
    fma_value_class_row("row09", 0x0000_0009, ValueClass::MixedSmall);
}

#[test]
fn row10_fma_boundary_values_mul_and_add_wrap() {
    fma_value_class_row("row10", 0x0000_000A, ValueClass::Boundary);

    // Also pin down the exhaustive boundary x boundary cross-product for the
    // multiply, which random sampling would only hit probabilistically.
    let n = BOUNDARY.len();
    let mut a = Vec::new();
    let mut b = Vec::new();
    for &x in BOUNDARY {
        for &y in BOUNDARY {
            a.push(x);
            b.push(y);
        }
    }
    let total = n * n;
    a.push(0);
    b.push(0);
    let mut rng = Rng::new(0xBEEF);
    let c = gen_vals(&mut rng, ValueClass::Boundary, total + 1);
    let o = gen_vals(&mut rng, ValueClass::FullRandom, total + 1);
    diff_fma(
        "row10-exhaustive",
        Alias::AllDistinct,
        total as c_int,
        &a,
        &b,
        &c,
        &o,
    );
}

#[test]
fn row11_fma_every_product_overflows() {
    fma_value_class_row("row11", 0x0000_000B, ValueClass::OverflowMul);
}

#[test]
fn row12_fma_every_add_overflows() {
    // mul1 = 1, mul2 = INT_MAX (or near), add = 1.. so the `+` overflows.
    let mut rng = Rng::new(0x0000_000C);
    for _ in 0..ITERS {
        let n = rng.range_usize(64, 512);
        let a: Vec<i32> = vec![1; n + 1];
        let b = gen_vals(&mut rng, ValueClass::OverflowAdd, n + 1);
        let c: Vec<i32> = (0..n + 1).map(|_| rng.range_i32(1, 8)).collect();
        let o = gen_vals(&mut rng, ValueClass::FullRandom, n + 1);
        diff_fma("row12", Alias::AllDistinct, n as c_int, &a, &b, &c, &o);
    }
}

// ===========================================================================
// Rows 13-18: `fma_array` aliasing schemes
// ===========================================================================

fn fma_alias_row(row: &str, seed: u64, alias: Alias) {
    let mut rng = Rng::new(seed);
    for _ in 0..ITERS {
        let n = rng.range_usize(64, 512);
        // Cycle through value classes so each aliasing scheme also sees
        // overflowing and boundary data, not just benign randoms.
        let class = rng.pick(ALL_VALUE_CLASSES);
        let a = gen_vals(&mut rng, class, n + 2);
        let b = gen_vals(&mut rng, class, n + 2);
        let c = gen_vals(&mut rng, class, n + 2);
        let o = gen_vals(&mut rng, ValueClass::FullRandom, n + 2);
        diff_fma(row, alias, n as c_int, &a, &b, &c, &o);
    }
}

#[test]
fn row13_fma_alias_out_is_mul1() {
    fma_alias_row("row13", 0x0000_000D, Alias::OutIsMul1);
}

#[test]
fn row14_fma_alias_out_is_add() {
    fma_alias_row("row14", 0x0000_000E, Alias::OutIsAdd);
}

#[test]
fn row15_fma_alias_mul1_is_mul2_squaring() {
    fma_alias_row("row15", 0x0000_000F, Alias::Mul1IsMul2);
}

#[test]
fn row16_fma_alias_all_four_same_inner_pattern() {
    fma_alias_row("row16", 0x0000_0010, Alias::AllSame);
}

#[test]
fn row17_fma_alias_mul1_one_ahead_of_out() {
    fma_alias_row("row17", 0x0000_0011, Alias::Mul1AheadOfOut);
}

#[test]
fn row18_fma_alias_out_one_ahead_of_mul1() {
    fma_alias_row("row18", 0x0000_0012, Alias::OutAheadOfMul1);
}

#[test]
fn rows13_18_all_alias_schemes_across_all_lens() {
    // Cross-product sweep: every aliasing scheme x a spread of lengths x every
    // value class. This is the row 13-18 cross-check at small scale.
    let mut rng = Rng::new(0x0000_1318);
    for &alias in ALL_ALIASES {
        for &class in ALL_VALUE_CLASSES {
            for len in [0i32, 1, 2, 3, 5, 8, 17, 64, 129] {
                let n = len.max(0) as usize;
                let a = gen_vals(&mut rng, class, n + 2);
                let b = gen_vals(&mut rng, class, n + 2);
                let c = gen_vals(&mut rng, class, n + 2);
                let o = gen_vals(&mut rng, ValueClass::FullRandom, n + 2);
                diff_fma(
                    &format!("sweep alias={alias:?} class={class:?}"),
                    alias,
                    len,
                    &a,
                    &b,
                    &c,
                    &o,
                );
            }
        }
    }
}

// ===========================================================================
// Rows 19-29: `driver` (stdout compared byte-for-byte)
//
// All stdout capture happens in an isolated single-threaded subprocess (see
// `common::run_driver_batch`), so libtest's own fd-1 progress output cannot
// contaminate the comparison. Each test builds its whole batch of cases first
// and runs them in ONE subprocess.
// ===========================================================================

/// Subprocess entry point. A no-op during a normal test run.
#[test]
fn zz_stdout_worker() {
    common::worker_main_if_requested();
}

#[test]
fn row19_driver_len0_prints_nothing() {
    let mut rng = Rng::new(0x0000_0013);
    let cases: Vec<(Vec<i32>, c_int)> = (0..32)
        .map(|_| (gen_vals(&mut rng, ValueClass::FullRandom, 8), 0))
        .collect();
    for (i, out) in diff_driver_batch("row19", &cases).iter().enumerate() {
        assert!(
            out.is_empty(),
            "driver(len=0) case {i} must print nothing, got {:?}",
            String::from_utf8_lossy(out)
        );
    }
}

/// Build `count` cases of `len` drawn from `len_range` with values from `class`,
/// run them all against both libraries, and check each against the documented
/// wrapping-FMA + `printf("%d\n")` rendering.
fn driver_random_row(row: &str, seed: u64, class: ValueClass, lo: usize, hi: usize, count: usize) {
    let mut rng = Rng::new(seed);
    let cases: Vec<(Vec<i32>, c_int)> = (0..count)
        .map(|_| {
            let n = rng.range_usize(lo, hi);
            (gen_vals(&mut rng, class, n), n as c_int)
        })
        .collect();
    let outs = diff_driver_batch(row, &cases);
    for (i, ((data, len), out)) in cases.iter().zip(outs.iter()).enumerate() {
        assert_eq!(
            out,
            &expected_driver_stdout(data, *len),
            "{row} case #{i} (len={len}) does not match the wrapping-FMA rendering"
        );
    }
}

#[test]
fn row20_driver_len1_random() {
    driver_random_row("row20", 0x0000_0014, ValueClass::FullRandom, 1, 1, ITERS);
}

#[test]
fn row21_driver_len_small_random() {
    driver_random_row("row21", 0x0000_0015, ValueClass::FullRandom, 2, 8, ITERS);
}

#[test]
fn row22_driver_len_many_random() {
    driver_random_row("row22", 0x0000_0016, ValueClass::FullRandom, 64, 1024, 64);
}

#[test]
fn row23_driver_all_zeros() {
    driver_random_row("row23", 0x0000_0017, ValueClass::Zeros, 64, 512, 64);
    // Explicit shape check: len copies of "0\n".
    let out = diff_driver("row23-shape", &vec![0i32; 100], 100);
    assert_eq!(out, b"0\n".repeat(100));
}

#[test]
fn row24_driver_small_positive() {
    driver_random_row("row24", 0x0000_0018, ValueClass::SmallPos, 64, 512, 64);
}

#[test]
fn row25_driver_small_negative_sign_formatting() {
    driver_random_row("row25", 0x0000_0019, ValueClass::SmallNeg, 64, 512, 64);
}

#[test]
fn row26_driver_boundary_values_and_int_min_formatting() {
    driver_random_row("row26", 0x0000_001A, ValueClass::Boundary, 64, 512, 64);

    // Exhaustive: every boundary value as a single-element driver call, so the
    // exact decimal rendering of each wrapped result is pinned; plus all of
    // them in one array to pin the ordering of the print loop.
    let mut cases: Vec<(Vec<i32>, c_int)> = BOUNDARY.iter().map(|&v| (vec![v], 1)).collect();
    cases.push((BOUNDARY.to_vec(), BOUNDARY.len() as c_int));
    let outs = diff_driver_batch("row26-exhaustive", &cases);
    for ((data, len), out) in cases.iter().zip(outs.iter()) {
        assert_eq!(out, &expected_driver_stdout(data, *len));
    }

    // INT_MIN specifically: INT_MIN*INT_MIN wraps to 0, + INT_MIN = INT_MIN,
    // which must print as "-2147483648".
    assert_eq!(outs[0], b"-2147483648\n", "INT_MIN rendering");
}

#[test]
fn row27_driver_every_product_overflows() {
    driver_random_row("row27", 0x0000_001B, ValueClass::OverflowMul, 64, 512, 64);
    driver_random_row("row27b", 0x0000_001B1, ValueClass::OverflowAdd, 64, 512, 64);
}

#[test]
fn row28_driver_repeated_calls_are_stateless() {
    // Many calls of differing lengths, then the SAME calls replayed in reverse
    // order inside the same process. If either implementation carried state
    // between calls (e.g. a leaked VLA buffer), the replayed outputs would
    // differ from the originals.
    let mut rng = Rng::new(0x0000_001C);
    let forward: Vec<(Vec<i32>, c_int)> = (0..120)
        .map(|_| {
            let n = rng.range_usize(0, 40);
            let data = gen_vals(&mut rng, ValueClass::Boundary, n.max(1));
            (data, n as c_int)
        })
        .collect();

    let first = diff_driver_batch("row28-forward", &forward);

    let reversed: Vec<(Vec<i32>, c_int)> = forward.iter().rev().cloned().collect();
    let second = diff_driver_batch("row28-reverse", &reversed);

    for (i, out) in second.iter().enumerate() {
        let orig = &first[first.len() - 1 - i];
        assert_eq!(
            out, orig,
            "driver is not stateless: replayed case {i} (len={}) differs",
            reversed[i].1
        );
    }
}

// ===========================================================================
// Row 29: composed pipeline vs. low-level primitive
// ===========================================================================

#[test]
fn row29_driver_stdout_matches_fma_array_primitive() {
    let mut rng = Rng::new(0x0000_001D);
    let l = libs();

    let cases: Vec<(Vec<i32>, c_int)> = (0..ITERS)
        .map(|_| {
            let n = rng.range_usize(1, 128);
            let class = rng.pick(ALL_VALUE_CLASSES);
            (gen_vals(&mut rng, class, n), n as c_int)
        })
        .collect();

    let outs = diff_driver_batch("row29", &cases);

    for (((data, len), out), _) in cases.iter().zip(outs.iter()).zip(0..) {
        // Low-level primitive, driven exactly the way `inner` drives it, from
        // each library independently. The composed pipeline's stdout must be
        // the decimal rendering of the primitive's result.
        for imp in [&l.c, &l.rs] {
            let mut buf = data.clone();
            unsafe {
                let p = buf.as_mut_ptr();
                (imp.fma_array)(
                    p,
                    p as *const c_int,
                    p as *const c_int,
                    p as *const c_int,
                    *len,
                );
            }
            let mut rendered = String::new();
            for v in &buf {
                rendered.push_str(&v.to_string());
                rendered.push('\n');
            }
            assert_eq!(
                out,
                &rendered.as_bytes().to_vec(),
                "{}: driver pipeline disagrees with its own fma_array primitive (len={len})",
                imp.name
            );
        }
    }
}

// ===========================================================================
// Row 30: exactly `len` elements written, nothing past the end
// ===========================================================================

#[test]
fn row30_fma_writes_exactly_len_elements() {
    let mut rng = Rng::new(0x0000_001E);
    let l = libs();

    for _ in 0..ITERS {
        let n = rng.range_usize(1, 256);
        let a = gen_vals(&mut rng, ValueClass::FullRandom, n + 1);
        let b = gen_vals(&mut rng, ValueClass::FullRandom, n + 1);
        let c = gen_vals(&mut rng, ValueClass::FullRandom, n + 1);
        // out_init is deliberately shorter than the buffer so the tail stays
        // SENTINEL in `call_fma`.
        let o = vec![SENTINEL; 0];

        // Differential first.
        diff_fma("row30", Alias::AllDistinct, n as c_int, &a, &b, &c, &o);

        // Then the absolute check on both: everything at index >= len must
        // still be the sentinel.
        for imp in [&l.c, &l.rs] {
            let res = call_fma(imp, Alias::AllDistinct, n as c_int, &a, &b, &c, &o);
            let outbuf = &res.buffers[0];
            for (i, &v) in outbuf.iter().enumerate().skip(n) {
                assert_eq!(
                    v, SENTINEL,
                    "{}: fma_array wrote past len={n} at index {i}",
                    imp.name
                );
            }
            for i in 0..n {
                let want = a[i].wrapping_mul(b[i]).wrapping_add(c[i]);
                assert_eq!(outbuf[i], want, "{}: wrong value at {i}", imp.name);
            }
        }
    }
}
