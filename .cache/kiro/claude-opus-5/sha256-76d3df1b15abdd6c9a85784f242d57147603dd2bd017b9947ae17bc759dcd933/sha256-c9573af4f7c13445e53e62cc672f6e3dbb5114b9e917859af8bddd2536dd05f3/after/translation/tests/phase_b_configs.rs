//! Phase B, part 1 — `CONFIGS.md` rows C1..C15: the two lowest-level entry
//! points, `fma_array` and `call_fma`.
//!
//! Lowest level first on purpose: a bug in the innermost multiply-add loop is
//! far easier to read off a direct `fma_array` buffer diff than off a `driver`
//! stdout diff.
//!
//! These rows compare return values and output buffers, never process stdout,
//! so they are safe to run on libtest's default thread pool. The `driver` rows
//! (C16..C33) live in `phase_b_driver.rs` instead, because capturing fd 1
//! requires that nothing else in the process writes to it concurrently.

mod common;

use common::*;

/// Per-row iteration count. Large enough that value-dependent bugs (overflow,
/// sign handling, off-by-one indices) are hit, small enough to stay fast.
const ITERS: usize = 200;

// ===========================================================================
// C1..C9 — `fma_array`, the lowest-level entry point
// ===========================================================================

/// C1 — `len == 1`, randomised full-range operands.
#[test]
fn c1_fma_array_len1_random() {
    let mut rng = Rng::new(SEED ^ 1);
    for it in 0..ITERS {
        let mul1 = random_vec(&mut rng, 1);
        let mul2 = random_vec(&mut rng, 1);
        let add = random_vec(&mut rng, 1);
        let prefill = random_vec(&mut rng, 1);
        assert_fma_array_matches(&prefill, &mul1, &mul2, &add, 1, &format!("C1 it={it}"));
    }
}

/// C2 — `len == 2`, randomised full-range operands.
#[test]
fn c2_fma_array_len2_random() {
    let mut rng = Rng::new(SEED ^ 2);
    for it in 0..ITERS {
        let mul1 = random_vec(&mut rng, 2);
        let mul2 = random_vec(&mut rng, 2);
        let add = random_vec(&mut rng, 2);
        let prefill = random_vec(&mut rng, 2);
        assert_fma_array_matches(&prefill, &mul1, &mul2, &add, 2, &format!("C2 it={it}"));
    }
}

/// C3 — small `len` in `3..=8`, below any vector width.
#[test]
fn c3_fma_array_small_len_random() {
    let mut rng = Rng::new(SEED ^ 3);
    for it in 0..ITERS {
        let n = rng.range(3, 8);
        let mul1 = random_vec(&mut rng, n);
        let mul2 = random_vec(&mut rng, n);
        let add = random_vec(&mut rng, n);
        let prefill = random_vec(&mut rng, n);
        assert_fma_array_matches(
            &prefill,
            &mul1,
            &mul2,
            &add,
            n as i32,
            &format!("C3 it={it} n={n}"),
        );
    }
}

/// C4 — `len == 1000`, spanning many vector iterations plus a remainder.
#[test]
fn c4_fma_array_large_len_random() {
    let mut rng = Rng::new(SEED ^ 4);
    for it in 0..20 {
        let n = 1000;
        let mul1 = random_vec(&mut rng, n);
        let mul2 = random_vec(&mut rng, n);
        let add = random_vec(&mut rng, n);
        let prefill = random_vec(&mut rng, n);
        assert_fma_array_matches(
            &prefill,
            &mul1,
            &mul2,
            &add,
            n as i32,
            &format!("C4 it={it}"),
        );
    }
}

/// C5 — `len == 100_000`.
#[test]
fn c5_fma_array_very_large_len_random() {
    let mut rng = Rng::new(SEED ^ 5);
    for it in 0..3 {
        let n = 100_000;
        let mul1 = random_vec(&mut rng, n);
        let mul2 = random_vec(&mut rng, n);
        let add = random_vec(&mut rng, n);
        let prefill = random_vec(&mut rng, n);
        assert_fma_array_matches(
            &prefill,
            &mul1,
            &mul2,
            &add,
            n as i32,
            &format!("C5 it={it}"),
        );
    }
}

/// C6 — operands drawn only from `EXTREMES`, so `mul1*mul2` and `+add` overflow
/// constantly. This is the row that pins down the wrapping semantics.
#[test]
fn c6_fma_array_extreme_values() {
    let mut rng = Rng::new(SEED ^ 6);
    for it in 0..ITERS {
        let n = rng.range(1, 64);
        let mul1 = extreme_vec(&mut rng, n);
        let mul2 = extreme_vec(&mut rng, n);
        let add = extreme_vec(&mut rng, n);
        let prefill = extreme_vec(&mut rng, n);
        assert_fma_array_matches(
            &prefill,
            &mul1,
            &mul2,
            &add,
            n as i32,
            &format!("C6 it={it} n={n}"),
        );
    }
}

/// C7 — the exact operand shape `call_fma` builds (`mul1` all ones, `add` all
/// zeros) driven directly, so the composed path is checked against the general
/// one rather than assumed equivalent.
#[test]
fn c7_fma_array_ones_zeros_shape() {
    let mut rng = Rng::new(SEED ^ 7);
    for it in 0..ITERS {
        let n = rng.range(1, 64);
        let ones = vec![1i32; n];
        let zeros = vec![0i32; n];
        let data = if rng.bool() {
            random_vec(&mut rng, n)
        } else {
            extreme_vec(&mut rng, n)
        };
        let prefill = random_vec(&mut rng, n);
        let out = assert_fma_array_matches(
            &prefill,
            &ones,
            &data,
            &zeros,
            n as i32,
            &format!("C7 it={it} n={n}"),
        );
        // Both agreed; the value should also equal `data` element-wise, which is
        // what makes `call_fma` return `data[len-1]`.
        assert_eq!(out, data, "C7 it={it}: ones*data+zeros should equal data");
    }
}

/// C8 — degenerate operands: `mul2` all zeros (result is `add` alone), then
/// `add` all zeros (result is the raw product).
#[test]
fn c8_fma_array_degenerate_operands() {
    let mut rng = Rng::new(SEED ^ 8);
    for it in 0..ITERS {
        let n = rng.range(1, 64);
        let prefill = random_vec(&mut rng, n);

        // mul2 == 0  =>  out == add
        let mul1 = extreme_vec(&mut rng, n);
        let zeros = vec![0i32; n];
        let add = random_vec(&mut rng, n);
        let out = assert_fma_array_matches(
            &prefill,
            &mul1,
            &zeros,
            &add,
            n as i32,
            &format!("C8a it={it} n={n}"),
        );
        assert_eq!(out, add, "C8a it={it}");

        // add == 0  =>  out == mul1*mul2 (wrapping)
        let mul2 = extreme_vec(&mut rng, n);
        let out = assert_fma_array_matches(
            &prefill,
            &mul1,
            &mul2,
            &zeros,
            n as i32,
            &format!("C8b it={it} n={n}"),
        );
        let expect: Vec<i32> = (0..n).map(|i| mul1[i].wrapping_mul(mul2[i])).collect();
        assert_eq!(out, expect, "C8b it={it}");
    }
}

/// C9 — `out` buffer longer than `len`: the C must write exactly `len` elements
/// and leave the tail at its pre-filled sentinel values. Comparing the whole
/// buffer catches an over- or under-write that a `len`-only comparison hides.
#[test]
fn c9_fma_array_writes_exactly_len() {
    let mut rng = Rng::new(SEED ^ 9);
    for it in 0..ITERS {
        let n = rng.range(1, 48);
        let slack = rng.range(1, 16);
        let cap = n + slack;

        let prefill = random_vec(&mut rng, cap);
        let mul1 = random_vec(&mut rng, cap);
        let mul2 = random_vec(&mut rng, cap);
        let add = random_vec(&mut rng, cap);

        let out = assert_fma_array_matches(
            &prefill,
            &mul1,
            &mul2,
            &add,
            n as i32,
            &format!("C9 it={it} n={n} cap={cap}"),
        );
        assert_eq!(
            &out[n..],
            &prefill[n..],
            "C9 it={it}: tail past len was modified"
        );
    }
}

// ===========================================================================
// C10..C15 — `call_fma`, the mid-level entry point
// ===========================================================================

/// C10 — `len == 1`: the element the C explicitly pre-sets with `out[0] = 0`
/// before `fma_array` overwrites it is also the one it returns.
#[test]
fn c10_call_fma_len1_random() {
    let mut rng = Rng::new(SEED ^ 10);
    for it in 0..ITERS {
        let data = random_vec(&mut rng, 1);
        let v = assert_call_fma_matches(&data, 1, &format!("C10 it={it}"));
        assert_eq!(v, data[0], "C10 it={it}: expected data[0]");
    }
}

/// C11 — `len` in `2..=8`, randomised full-range data.
#[test]
fn c11_call_fma_small_len_random() {
    let mut rng = Rng::new(SEED ^ 11);
    for it in 0..ITERS {
        let n = rng.range(2, 8);
        let data = random_vec(&mut rng, n);
        let v = assert_call_fma_matches(&data, n as i32, &format!("C11 it={it} n={n}"));
        assert_eq!(v, data[n - 1], "C11 it={it}");
    }
}

/// C12 — `len == 100`, the largest value `driver` can ever pass down.
#[test]
fn c12_call_fma_len100_random() {
    let mut rng = Rng::new(SEED ^ 12);
    for it in 0..ITERS {
        let data = random_vec(&mut rng, 100);
        let v = assert_call_fma_matches(&data, 100, &format!("C12 it={it}"));
        assert_eq!(v, data[99], "C12 it={it}");
    }
}

/// C13 — `len == 4096`: three VLAs of that size, well past a page.
#[test]
fn c13_call_fma_large_len_random() {
    let mut rng = Rng::new(SEED ^ 13);
    for it in 0..50 {
        let n = 4096;
        let data = random_vec(&mut rng, n);
        let v = assert_call_fma_matches(&data, n as i32, &format!("C13 it={it}"));
        assert_eq!(v, data[n - 1], "C13 it={it}");
    }
}

/// C14 — `data` from `EXTREMES` incl. `INT_MAX`/`INT_MIN`.
#[test]
fn c14_call_fma_extreme_values() {
    let mut rng = Rng::new(SEED ^ 14);
    for it in 0..ITERS {
        let n = rng.range(1, 64);
        let data = extreme_vec(&mut rng, n);
        let v = assert_call_fma_matches(&data, n as i32, &format!("C14 it={it} n={n}"));
        assert_eq!(v, data[n - 1], "C14 it={it}");
    }
}

/// C15 — `data` buffer longer than `len`: only `data[len-1]` may influence the
/// result, so the ignored tail must not leak into it.
#[test]
fn c15_call_fma_ignores_tail() {
    let mut rng = Rng::new(SEED ^ 15);
    for it in 0..ITERS {
        let n = rng.range(1, 48);
        let cap = n + rng.range(1, 32);
        let data = random_vec(&mut rng, cap);
        let v = assert_call_fma_matches(&data, n as i32, &format!("C15 it={it} n={n} cap={cap}"));
        assert_eq!(v, data[n - 1], "C15 it={it}");
    }
}
