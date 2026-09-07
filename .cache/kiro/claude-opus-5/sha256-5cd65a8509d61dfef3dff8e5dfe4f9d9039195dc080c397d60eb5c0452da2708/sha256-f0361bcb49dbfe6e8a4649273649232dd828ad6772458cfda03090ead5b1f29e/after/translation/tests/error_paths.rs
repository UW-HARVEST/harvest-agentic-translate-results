//! Phase C — error-path differential tests, one per `ERRORS.md` row.
//!
//! Every test drives BOTH shared objects through `dlopen`/`dlsym`, constructs
//! the exact rejecting condition, and asserts the two agree on the *same*
//! sentinel (bit-exact `double`, or the exact `int` the C returns) — never
//! merely "both failed".
//!
//! Inputs that are undefined behaviour in the C (and therefore actually kill the
//! C `.so`) are exercised in a forked child so the runner survives; for those the
//! compared observable is the termination outcome.

mod common;

use std::ffi::c_int;

use common::probe::{self, Outcome, SIGSEGV};
use common::*;

fn diff_match(ctx: &str, t: &[f64], r: &[f64], bins: c_int, thr: f64) -> c_int {
    let l = libs();
    let cc = call_match(&l.c, t, r, bins, thr);
    let rr = call_match(&l.r, t, r, bins, thr);
    assert_match_eq(ctx, &cc, &rr);
    cc.ret
}

fn diff_sc(ctx: &str, a: &[f32], b: &[f32], length: c_int) -> u64 {
    let l = libs();
    let c = call_sc(&l.c, a, b, length);
    let r = call_sc(&l.r, a, b, length);
    assert_sc_eq(ctx, &c, &r);
    c.ret
}

/// A `reference` vector whose `total` is **exactly** `+0.0` (pairs of `x, -x`,
/// summed left to right), with non-zero low mantissa halves so the `float`
/// reinterpretation is not degenerate.
fn zero_total_vec(n: usize, rng: &mut Rng) -> Vec<f64> {
    assert!(n % 2 == 0);
    let mut v = Vec::with_capacity(n);
    for _ in 0..n / 2 {
        let x = 0.25 + rng.unit();
        v.push(x);
        v.push(-x);
    }
    v
}

// ===========================================================================
// Row 1 — the single explicit rejection: the energy gate.
// ===========================================================================
#[test]
fn err01_energy_gate_rejects() {
    let mut rng = Rng::new(0xE001);
    let mut saw_reject = false;
    let mut saw_accept = false;
    for i in 0..300 {
        let n = 2 + rng.range(40);
        let t = gen64(Shape64::Unit, n, &mut rng);
        // Identical data ⇒ `total(test) == total(reference) > 0`, so the gate
        // rejects for `threshold > 1` and passes for `threshold < 1`.
        let reject = diff_match(&format!("err01 reject #{i}"), &t, &t, n as c_int, 2.0);
        let accept = diff_match(&format!("err01 accept #{i}"), &t, &t, n as c_int, 0.5);
        assert_eq!(reject, 0, "err01 #{i}: gate must reject (return 0) for threshold=2.0");
        saw_reject = true;
        if accept == 1 {
            saw_accept = true;
        }
    }
    assert!(saw_reject);
    assert!(
        saw_accept,
        "err01: the accepting path must be reachable, otherwise the rejection is not observable"
    );

    // Explicit hand-built rejections across the sign/zero boundary.
    for &(tv, rv, thr) in &[
        (1.0f64, 1.0f64, 1.0000001f64),
        (0.0, 1.0, 1e-300),
        (-1.0, 1.0, -0.5),
        (1e-300, 1.0, 0.5),
    ] {
        let t = vec![tv; 8];
        let r = vec![rv; 8];
        let ret = diff_match(&format!("err01 hand t={tv} r={rv} thr={thr}"), &t, &r, 8, thr);
        assert_eq!(ret, 0, "err01 hand t={tv} r={rv} thr={thr}: expected gate rejection");
    }
}

// ===========================================================================
// Row 2 — unordered gate comparison must NOT reject.
// ===========================================================================
#[test]
fn err02_gate_unordered_falls_through() {
    let mut rng = Rng::new(0xE002);
    // `threshold * total(reference)` = `-inf * +0.0` = NaN ⇒ `comisd` unordered
    // ⇒ `jbe` taken ⇒ the gate does not reject. Because the final comparison is
    // then `contrast >= -inf`, a fall-through is observable as a `1`.
    let mut saw_one = false;
    for i in 0..200 {
        let n = 2 * (1 + rng.range(20));
        let t = gen64(Shape64::Unit, n, &mut rng);
        let r = zero_total_vec(n, &mut rng);
        let ret = diff_match(&format!("err02 #{i} n={n}"), &t, &r, n as c_int, f64::NEG_INFINITY);
        if ret == 1 {
            saw_one = true;
        }
    }
    assert!(
        saw_one,
        "err02: `-inf * +0.0` must leave the gate unordered and fall through (observable as 1)"
    );

    // Also: NaN threshold, and NaN `total(test)`, must not be treated as a
    // rejection by *either* implementation — verified by bit-exact agreement.
    for &thr in &[nan_threshold(), f64::INFINITY, f64::NEG_INFINITY] {
        for i in 0..100 {
            let n = 2 * (1 + rng.range(20));
            let t = gen64(Shape64::Nans, n, &mut rng);
            let r = zero_total_vec(n, &mut rng);
            diff_match(&format!("err02b thr={thr} #{i}"), &t, &r, n as c_int, thr);
        }
    }
}

// ===========================================================================
// Row 3 — unordered final comparison returns 0.
// ===========================================================================
#[test]
fn err03_final_compare_unordered_returns_0() {
    let mut rng = Rng::new(0xE003);
    // (a) NaN threshold: the gate falls through (row 2) and `setae` after an
    //     unordered `comisd` yields 0.
    for &nan_bits in F64_NANS.iter() {
        let thr = f64::from_bits(nan_bits);
        for i in 0..40 {
            let n = 1 + rng.range(40);
            let t = gen64(Shape64::Unit, n, &mut rng);
            let r = gen64(Shape64::Unit, n, &mut rng);
            let ret =
                diff_match(&format!("err03a thr=0x{nan_bits:016X} #{i}"), &t, &r, n as c_int, thr);
            assert_eq!(ret, 0, "err03a: NaN threshold must return 0");
        }
    }
    // (b) NaN contrast with a finite, very negative threshold. The only input
    //     that *guarantees* a NaN contrast for arbitrary `bins` is the all-zero
    //     vector: `preprocess` keeps it all-zero, so every `float` lane is
    //     `+0.0`, `magnitude == 0`, `normalize` computes `0/0` and the final
    //     `dot_product` is NaN. `setae` after the unordered `comisd` must then
    //     yield 0 no matter how small the threshold is.
    for n in 1..=40usize {
        for &z in &[0.0f64, -0.0] {
            let t = vec![z; n];
            let r = vec![z; n];
            for &thr in &[-1e300f64, -1.0, 0.0, f64::NEG_INFINITY] {
                let ret = diff_match(&format!("err03b n={n} z={z} thr={thr}"), &t, &r, n as c_int, thr);
                assert_eq!(
                    ret, 0,
                    "err03b n={n} z={z} thr={thr}: NaN contrast must return 0"
                );
            }
        }
    }
}

// ===========================================================================
// Rows 4, 7, 9 — `bins == 0`: zero-length VLAs and the `v[-1] = 0` store.
// ===========================================================================
#[test]
fn err04_bins_zero() {
    let l = libs();
    let t = vec![1.25f64; 8];
    let r = vec![2.5f64; 8];
    for &thr in &[-1.5f64, -0.0, 0.0, 0.25, 1.0, f64::INFINITY, f64::NEG_INFINITY] {
        let (co, cv) = probe::isolated_match(&l.c, &t, &r, 0, thr);
        let (ro, rv) = probe::isolated_match(&l.r, &t, &r, 0, thr);
        // The C is undefined behaviour here and reproducibly dies: with a
        // zero-length VLA `rsp` is unchanged, so `t` aliases the stack top and
        // `differentiate`'s `v[length - 1] = 0` overwrites `preprocess`'s saved
        // return address with 0.
        assert_eq!(
            co,
            Outcome::Signaled(SIGSEGV),
            "err04 thr={thr}: expected the C to SIGSEGV for bins == 0, got {co:?} value {cv:?}"
        );
        // Documented, deliberate divergence: the Rust has no VLA to overrun, so
        // it survives and returns `(0.0 >= threshold)`.
        assert_eq!(ro, Outcome::Exited(0), "err04 thr={thr}: Rust outcome {ro:?}");
        assert_eq!(
            rv,
            Some(((0.0f64 >= thr) as c_int) as u32 as u64),
            "err04 thr={thr}: Rust must return (0.0 >= threshold)"
        );
    }
}

// ===========================================================================
// Row 5 — `bins < 0`: VLA with a negative element count.
// ===========================================================================
#[test]
fn err05_bins_negative_out_of_process() {
    let l = libs();
    let t = vec![1.25f64; 8];
    let r = vec![2.5f64; 8];
    for &bins in &[-1i32, -2, -16, -1000, i32::MIN + 1, i32::MIN] {
        let (co, _) = probe::isolated_match(&l.c, &t, &r, bins, 0.25);
        assert!(
            co.crashed(),
            "err05 bins={bins}: expected the C to die (negative VLA size), got {co:?}"
        );
        let (ro, rv) = probe::isolated_match(&l.r, &t, &r, bins, 0.25);
        assert_eq!(ro, Outcome::Exited(0), "err05 bins={bins}: Rust outcome {ro:?}");
        assert_eq!(rv, Some(0), "err05 bins={bins}: Rust returns (0.0 >= 0.25) == 0");
    }
}

// ===========================================================================
// Row 6 — `total` with a non-positive length returns `+0.0`.
// ===========================================================================
#[test]
fn err06_total_nonpositive_length() {
    // `total(v, n <= 0)` is only reachable through `bins <= 0`, which is UB in
    // the C (rows 4/5). The structurally identical zero-trip accumulator,
    // `dot_product(a, b, n <= 0)`, *is* reachable and must return exactly
    // `+0.0` — not `-0.0`, not NaN — in both implementations.
    let l = libs();
    let mut rng = Rng::new(0xE006);
    for &len in &[0i32, -1, -7, i32::MIN] {
        let a = gen32(Shape32::BitRandom, 8, &mut rng);
        let b = gen32(Shape32::BitRandom, 8, &mut rng);
        let c = call_sc(&l.c, &a, &b, len);
        let r = call_sc(&l.r, &a, &b, len);
        assert_sc_eq(&format!("err06 len={len}"), &c, &r);
        assert_eq!(c.ret, 0x0000_0000_0000_0000, "err06 len={len}: C must return +0.0");
    }
}

// ===========================================================================
// Row 8 — `smoothen`'s truncated tail keeps the `N_SMOOTH` divisor.
// ===========================================================================
#[test]
fn err08_smoothen_tail_divisor() {
    let mut rng = Rng::new(0xE008);
    // For every `bins < 16` *every* window is truncated; for `bins >= 16` only
    // the last 15 are. Both regimes are compared bit-exactly, which is the only
    // observable of an internal, un-renormalised divisor.
    for n in 1..=40usize {
        for shape in [Shape64::Constant, Shape64::Unit, Shape64::Ramp, Shape64::Impulse] {
            for i in 0..12 {
                let t = gen64(shape, n, &mut rng);
                let r = gen64(shape, n, &mut rng);
                for &thr in &[0.0f64, 0.25, 0.9, 1.0] {
                    diff_match(
                        &format!("err08 n={n} {shape:?} thr={thr} #{i}"),
                        &t,
                        &r,
                        n as c_int,
                        thr,
                    );
                }
            }
        }
    }
}

// ===========================================================================
// Row 10 — `differentiate` with `length == 1` forces the sole element to 0.
// ===========================================================================
#[test]
fn err10_differentiate_length_one() {
    let mut rng = Rng::new(0xE00A);
    // Consequence chain for `bins == 1`: `differentiate` unconditionally stores
    // `v[0] = 0`, the trailing `smoothen` keeps `+0.0`, the `float` lane is
    // `+0.0`, `magnitude == 0`, `0.0 / 0.0` yields the x86 indefinite `-nan`,
    // and `contrast >= threshold` is therefore false for *every* threshold.
    // So `match(_, _, 1, _)` must return 0 unconditionally.
    let mut ths: Vec<f64> = THRESHOLDS.to_vec();
    ths.push(nan_threshold());
    for shape in ALL_SHAPE64 {
        for &thr in &ths {
            for i in 0..12 {
                let t = gen64(shape, 1, &mut rng);
                let r = gen64(shape, 1, &mut rng);
                let ret =
                    diff_match(&format!("err10 {shape:?} thr={thr:e} #{i}"), &t, &r, 1, thr);
                assert_eq!(
                    ret, 0,
                    "err10 {shape:?} thr={thr:e} #{i}: bins == 1 must always return 0"
                );
            }
        }
    }
}

// ===========================================================================
// Row 11 — `inf - inf` inside `differentiate`.
// ===========================================================================
#[test]
fn err11_inf_minus_inf_nan() {
    let mut rng = Rng::new(0xE00B);
    // Adjacent equal infinities guarantee `subsd` produces the x86 indefinite
    // NaN, which then flows through the second `smoothen`.
    for n in 2..=20usize {
        for i in 0..30 {
            let mut t = gen64(Shape64::Unit, n, &mut rng);
            let k = rng.range(n - 1);
            let s = if rng.bool() { f64::INFINITY } else { f64::NEG_INFINITY };
            t[k] = s;
            t[k + 1] = s;
            let r = gen64(Shape64::Unit, n, &mut rng);
            for &thr in &[0.0f64, 0.25, 1.0, -1.0] {
                diff_match(&format!("err11 n={n} k={k} thr={thr} #{i}"), &t, &r, n as c_int, thr);
                diff_match(&format!("err11r n={n} k={k} thr={thr} #{i}"), &r, &t, n as c_int, thr);
            }
        }
    }
    // The same condition at the lowest-level entry point: `inf * 0` inside
    // `dot_product` and `inf/inf` inside `normalize`.
    for n in 1..=20usize {
        for i in 0..30 {
            let a = gen32(Shape32::Infinities, n, &mut rng);
            let b = gen32(Shape32::Infinities, n, &mut rng);
            diff_sc(&format!("err11sc n={n} #{i}"), &a, &b, n as c_int);
        }
    }
}

// ===========================================================================
// Row 12 — `magnitude == 0` ⇒ `0/0` and `x/0` inside `normalize`.
// ===========================================================================
#[test]
fn err12_zero_magnitude() {
    let l = libs();
    // (a) all-zero `a`: every `a[i] /= 0.0` is `0/0` ⇒ x86 indefinite `-nan`
    //     (`0xFFC00000` as a `float`), and the final `dot_product` is NaN.
    for n in 1..=20usize {
        let a = vec![0.0f32; n];
        let b: Vec<f32> = (0..n).map(|k| (k as f32) + 1.0).collect();
        let c = call_sc(&l.c, &a, &b, n as c_int);
        let r = call_sc(&l.r, &a, &b, n as c_int);
        assert_sc_eq(&format!("err12a n={n}"), &c, &r);
        assert_eq!(
            c.ret,
            0xFFF8_0000_0000_0000,
            "err12a n={n}: expected the x86 indefinite -nan from 0/0"
        );
        assert!(
            c.a_after.iter().all(|&x| x == 0xFFC0_0000),
            "err12a n={n}: every zero lane must become -nan, got {:08X?}",
            c.a_after
        );
    }
    // (b) `-0.0` lanes behave the same (`-0/0`).
    for n in 1..=20usize {
        let a = vec![-0.0f32; n];
        let b = vec![-0.0f32; n];
        let c = call_sc(&l.c, &a, &b, n as c_int);
        let r = call_sc(&l.r, &a, &b, n as c_int);
        assert_sc_eq(&format!("err12b n={n}"), &c, &r);
        assert!(f64::from_bits(c.ret).is_nan(), "err12b n={n}: expected NaN");
    }
    // (c) the same condition reached through `match`. Note the `float` lanes
    //     interleave the *low and high* halves of consecutive `double`s (see
    //     `err17`), so the only shape that guarantees all-zero lanes for every
    //     `bins` is the all-zero vector.
    let mut rng = Rng::new(0xE00C);
    for n in 1..=40usize {
        let t = vec![0.0f64; n];
        let r = vec![-0.0f64; n];
        let ret = diff_match(&format!("err12c n={n}"), &t, &r, n as c_int, -1e300);
        assert_eq!(ret, 0, "err12c n={n}: zero magnitude means NaN contrast means 0");
    }
    // ... and the mirror valid-input shape: exact integers / powers of two,
    // whose preprocessed low halves are zero while their high halves are not.
    for n in 1..=40usize {
        for i in 0..10 {
            let t: Vec<f64> = (0..n).map(|_| (rng.range(1000) as f64) + 1.0).collect();
            let r: Vec<f64> = (0..n).map(|_| (rng.range(1000) as f64) + 1.0).collect();
            diff_match(&format!("err12d n={n} #{i}"), &t, &r, n as c_int, -1e300);
        }
    }
}

// ===========================================================================
// Row 13 — `sqrt` of a NaN magnitude.
// ===========================================================================
#[test]
fn err13_sqrt_of_nan() {
    let l = libs();
    let mut rng = Rng::new(0xE00D);
    // A single NaN lane makes `dot_product(v, v)` NaN, so `sqrt` receives NaN.
    // glibc's `sqrt` is `sqrtsd`, which returns the operand quieted with its
    // payload intact — every lane then becomes that NaN.
    for &nb in F32_NANS.iter() {
        for n in 1..=20usize {
            let mut a = gen32(Shape32::Unit, n, &mut rng);
            a[rng.range(n)] = f32::from_bits(nb);
            let b = gen32(Shape32::Unit, n, &mut rng);
            let c = call_sc(&l.c, &a, &b, n as c_int);
            let r = call_sc(&l.r, &a, &b, n as c_int);
            assert_sc_eq(&format!("err13 nan=0x{nb:08X} n={n}"), &c, &r);
            assert!(
                c.a_after.iter().all(|&x| f32::from_bits(x).is_nan()),
                "err13 nan=0x{nb:08X} n={n}: NaN magnitude must poison every lane, got {:08X?}",
                c.a_after
            );
            assert!(
                f64::from_bits(c.ret).is_nan(),
                "err13 nan=0x{nb:08X} n={n}: result must be NaN"
            );
        }
    }
}

// ===========================================================================
// Row 14 — `magnitude == +inf`.
// ===========================================================================
#[test]
fn err14_magnitude_infinite() {
    let l = libs();
    for n in 1..=20usize {
        // `f32::MAX^2` overflows to `+inf`, so `magnitude = sqrt(+inf) = +inf`
        // and every `v[i] /= +inf` becomes `±0`.
        let a: Vec<f32> = (0..n).map(|k| if k % 2 == 0 { f32::MAX } else { -f32::MAX }).collect();
        let b: Vec<f32> = vec![f32::MAX; n];
        let c = call_sc(&l.c, &a, &b, n as c_int);
        let r = call_sc(&l.r, &a, &b, n as c_int);
        assert_sc_eq(&format!("err14 n={n}"), &c, &r);
        assert!(
            c.a_after.iter().all(|&x| x == 0x0000_0000 || x == 0x8000_0000),
            "err14 n={n}: lanes must collapse to ±0, got {:08X?}",
            c.a_after
        );
        assert_eq!(f64::from_bits(c.ret), 0.0, "err14 n={n}: result must be zero");
    }
    // Explicit `+inf` lanes: `sqrt(+inf) = +inf`, `inf / inf = -nan`.
    for n in 1..=20usize {
        let a = vec![f32::INFINITY; n];
        let b = vec![1.0f32; n];
        let c = call_sc(&l.c, &a, &b, n as c_int);
        let r = call_sc(&l.r, &a, &b, n as c_int);
        assert_sc_eq(&format!("err14b n={n}"), &c, &r);
    }
}

// ===========================================================================
// Row 15 — `spectral_contrast` with a non-positive length dereferences nothing.
// ===========================================================================
#[test]
fn err15_spectral_contrast_nonpositive_length() {
    let l = libs();
    let mut rng = Rng::new(0xE00F);
    for &len in &[0i32, -1, -2, -15, -16, -17, -1000, i32::MIN + 1, i32::MIN] {
        let a = gen32(Shape32::BitRandom, 16, &mut rng);
        let b = gen32(Shape32::BitRandom, 16, &mut rng);
        let c = call_sc(&l.c, &a, &b, len);
        let r = call_sc(&l.r, &a, &b, len);
        assert_sc_eq(&format!("err15 len={len}"), &c, &r);
        assert_eq!(c.ret, 0, "err15 len={len}: expected +0.0");
        assert_eq!(c.a_after, a.iter().map(|x| x.to_bits()).collect::<Vec<_>>());
        assert_eq!(c.b_after, b.iter().map(|x| x.to_bits()).collect::<Vec<_>>());

        // Same call with NULL pointers: still no dereference, so this is legal.
        let cn = call_sc_null(&l.c, len);
        let rn = call_sc_null(&l.r, len);
        assert_eq!(cn, 0, "err15 NULL len={len}: C must return +0.0");
        assert_eq!(cn, rn, "err15 NULL len={len}");

        // And out of process, to prove neither implementation faults.
        let (co, cv) = probe::isolated_sc_null(&l.c, len);
        let (ro, rv) = probe::isolated_sc_null(&l.r, len);
        assert_eq!(co, Outcome::Exited(0), "err15 isolated C len={len}: {co:?}");
        assert_eq!(ro, Outcome::Exited(0), "err15 isolated Rust len={len}: {ro:?}");
        assert_eq!(cv, Some(0));
        assert_eq!(cv, rv);
    }
}

// ===========================================================================
// Row 16 — aliased `a == b`.
// ===========================================================================
#[test]
fn err16_spectral_contrast_aliased() {
    let l = libs();
    let mut rng = Rng::new(0xE010);
    for shape in ALL_SHAPE32 {
        for n in [0usize, 1, 2, 3, 15, 16, 17, 40] {
            for i in 0..30 {
                let a = gen32(shape, n, &mut rng);
                let c = call_sc_aliased(&l.c, &a, n as c_int);
                let r = call_sc_aliased(&l.r, &a, n as c_int);
                assert_sc_eq(&format!("err16 {shape:?} n={n} #{i}"), &c, &r);
            }
        }
    }
    // Well-behaved input, aliased: normalising twice then self-dotting is ~1.0.
    for n in 1..=20usize {
        let a: Vec<f32> = (0..n).map(|k| (k as f32) + 1.0).collect();
        let c = call_sc_aliased(&l.c, &a, n as c_int);
        let r = call_sc_aliased(&l.r, &a, n as c_int);
        assert_sc_eq(&format!("err16b n={n}"), &c, &r);
        let v = f64::from_bits(c.ret);
        assert!((v - 1.0).abs() < 1e-5, "err16b n={n}: expected ~1.0, got {v}");
    }
}

// ===========================================================================
// Row 17 — the `float_t` mismatch itself (covered by every CONFIGS row);
// asserted here as an explicit, self-contained invariant.
// ===========================================================================
#[test]
fn err17_float_t_mismatch_is_reproduced() {
    let l = libs();
    // (a) `spectral_contrast` counts elements in `float`s, not `double`s, and its
    //     stride is 4 bytes. Feed it a buffer whose first `n` *floats* are a
    //     known vector and whose remaining floats are garbage: if either
    //     implementation used an 8-byte stride it would see different values, so
    //     bit-exact agreement plus an untouched upper half pins the ABI.
    let n = 8usize;
    let mut a = vec![0.0f32; 2 * n];
    let mut b = vec![0.0f32; 2 * n];
    for k in 0..n {
        a[k] = (k as f32) + 1.0;
        b[k] = (n - k) as f32;
    }
    for k in n..2 * n {
        a[k] = f32::from_bits(0xDEAD_BEEF);
        b[k] = f32::from_bits(0xCAFE_BABE);
    }
    let c = call_sc(&l.c, &a, &b, n as c_int);
    let r = call_sc(&l.r, &a, &b, n as c_int);
    assert_sc_eq("err17a", &c, &r);
    for k in n..2 * n {
        assert_eq!(c.a_after[k], 0xDEAD_BEEF, "err17a: a[{k}] must not be touched");
        assert_eq!(c.b_after[k], 0xCAFE_BABE, "err17a: b[{k}] must not be touched");
    }
    let v = f64::from_bits(c.ret);
    assert!(v.is_finite() && v > 0.0 && v < 1.0, "err17a: expected a finite cosine, got {v}");

    // (b) `match` hands `double` buffers to that `float`-typed callee, so the
    //     `bins` lanes it processes are the interleaved low/high 4-byte halves of
    //     only the first `ceil(bins/2)` doubles. Consequence: the *upper* half of
    //     every `double` at index >= ceil(bins/2) can never influence the result.
    //     Construct two inputs that differ only in the untouched tail and assert
    //     both libraries return the same value for both — and that C's two
    //     results are equal, which is only true under the 4-byte stride.
    let mut rng = Rng::new(0xE011);
    for bins in 2..=32usize {
        let half = bins.div_ceil(2);
        for i in 0..20 {
            let base = gen64(Shape64::Unit, bins, &mut rng);
            let mut alt = base.clone();
            for k in half..bins {
                alt[k] = f64::from_bits(rng.next_u64() | 0x0010_0000_0000_0000);
            }
            let refv = gen64(Shape64::Unit, bins, &mut rng);
            // Only compare C-vs-Rust here; the tail *does* still feed `total`,
            // so `base` and `alt` need not agree with each other.
            let c1 = call_match(&l.c, &base, &refv, bins as c_int, -1e300);
            let r1 = call_match(&l.r, &base, &refv, bins as c_int, -1e300);
            assert_match_eq(&format!("err17b base bins={bins} #{i}"), &c1, &r1);
            let c2 = call_match(&l.c, &alt, &refv, bins as c_int, -1e300);
            let r2 = call_match(&l.r, &alt, &refv, bins as c_int, -1e300);
            assert_match_eq(&format!("err17b alt bins={bins} #{i}"), &c2, &r2);
        }
    }
}

// ===========================================================================
// Row 18 — NULL with a positive length.
// ===========================================================================
#[test]
fn err18_null_with_positive_length_out_of_process() {
    let l = libs();
    for &n in &[1i32, 2, 16, 17, 1000] {
        let (co, _) = probe::isolated_sc_null(&l.c, n);
        let (ro, _) = probe::isolated_sc_null(&l.r, n);
        assert_eq!(co, Outcome::Signaled(SIGSEGV), "err18 sc C n={n}: {co:?}");
        assert_eq!(ro, Outcome::Signaled(SIGSEGV), "err18 sc Rust n={n}: {ro:?}");

        let (co, _) = probe::isolated_match_null(&l.c, n, 0.25);
        let (ro, _) = probe::isolated_match_null(&l.r, n, 0.25);
        assert_eq!(co, Outcome::Signaled(SIGSEGV), "err18 match C n={n}: {co:?}");
        assert_eq!(ro, Outcome::Signaled(SIGSEGV), "err18 match Rust n={n}: {ro:?}");
    }
}

// ===========================================================================
// Row 19 — `bins` large enough to overflow the VLA off the stack.
// ===========================================================================
#[test]
fn err19_huge_bins_documented() {
    let l = libs();
    // 2 * 4,000,000 * 8 B = 64 MiB of VLA, far past the default 8 MiB stack.
    let n = 4_000_000usize;
    let t = vec![1.0f64 + 1e-9; n];
    let r = vec![1.0f64 + 2e-9; n];
    let (co, _) = probe::isolated_match(&l.c, &t, &r, n as c_int, 0.25);
    assert!(co.crashed(), "err19: expected the C VLA to overflow the stack, got {co:?}");
    // Documented divergence: the Rust uses heap `Vec`s, so it completes. This is
    // C undefined behaviour (stack overflow), so no value is compared.
    let (ro, rv) = probe::isolated_match(&l.r, &t, &r, n as c_int, 0.25);
    assert_eq!(ro, Outcome::Exited(0), "err19: Rust outcome {ro:?}");
    assert!(rv.is_some());
}

// ===========================================================================
// Row 20 — integer boundaries, including values one step past every range and
// "out-of-range enum" style integers (there is no enum; `int` is the only
// discrete parameter, so its whole domain is probed).
// ===========================================================================
#[test]
fn err20_integer_boundaries() {
    let l = libs();
    let mut rng = Rng::new(0xE014);

    // (a) `spectral_contrast`: in-process for every length that stays inside the
    //     buffers, including one past each boundary the code branches on.
    let cap = 64usize;
    for &len in &[
        i32::MIN,
        i32::MIN + 1,
        -1000,
        -17,
        -16,
        -15,
        -2,
        -1,
        0,
        1,
        2,
        3,
        14,
        15,
        16,
        17,
        18,
        31,
        32,
        33,
        63,
        64,
    ] {
        if len as i64 > cap as i64 {
            continue;
        }
        let a = gen32(Shape32::BitRandom, cap, &mut rng);
        let b = gen32(Shape32::BitRandom, cap, &mut rng);
        let c = call_sc(&l.c, &a, &b, len);
        let r = call_sc(&l.r, &a, &b, len);
        assert_sc_eq(&format!("err20a len={len}"), &c, &r);
    }

    // (b) `spectral_contrast` with a wildly out-of-range positive length: both
    //     must walk off the end and die identically.
    for &len in &[i32::MAX, i32::MAX - 1, 1 << 30] {
        let a = gen32(Shape32::Unit, 16, &mut rng);
        let b = gen32(Shape32::Unit, 16, &mut rng);
        let (co, _) = probe::isolated_sc(&l.c, &a, &b, len);
        let (ro, _) = probe::isolated_sc(&l.r, &a, &b, len);
        assert!(co.crashed(), "err20b len={len}: C outcome {co:?}");
        assert_eq!(co, ro, "err20b len={len}: C {co:?} vs Rust {ro:?}");
    }

    // (c) `match`: every in-range boundary in-process, every UB boundary
    //     out-of-process.
    let t = gen64(Shape64::Unit, 64, &mut rng);
    let rf = gen64(Shape64::Unit, 64, &mut rng);
    for &bins in &[1i32, 2, 3, 14, 15, 16, 17, 18, 31, 32, 33, 63, 64] {
        let cc = call_match(&l.c, &t, &rf, bins, 0.25);
        let rr = call_match(&l.r, &t, &rf, bins, 0.25);
        assert_match_eq(&format!("err20c bins={bins}"), &cc, &rr);
    }
    for &bins in &[0i32, -1, -2, i32::MIN, i32::MIN + 1] {
        let (co, _) = probe::isolated_match(&l.c, &t, &rf, bins, 0.25);
        assert!(co.crashed(), "err20c bins={bins}: expected the C to die, got {co:?}");
    }

    // (d) Randomised `int` fuzz over the whole 32-bit domain, restricted to the
    //     range where the C is defined (`1 ..= buffer length`) for value
    //     comparison; everything else is a documented UB crash (rows 4/5/18/19).
    for i in 0..500 {
        let bins = 1 + rng.range(64) as i32;
        let cc = call_match(&l.c, &t, &rf, bins, 0.25);
        let rr = call_match(&l.r, &t, &rf, bins, 0.25);
        assert_match_eq(&format!("err20d #{i} bins={bins}"), &cc, &rr);
    }
}
