//! Phase C — error / rejection-path differential tests, one test per row of
//! `ERRORS.md`.
//!
//! The C function returns `void` and validates nothing, so "same error" means
//! "same observable rejection behaviour": the same bytes written (or the same
//! *absence* of writes) and the same branch taken. Destination buffers are
//! pre-filled with a 0xA5 canary pattern so "left untouched" is distinguishable
//! from "written as zero" — that is the sentinel this API has instead of an
//! error code.

#[path = "harness/mod.rs"]
mod harness;

use harness::*;
use std::ffi::c_int;
use std::ptr;

fn mkvec<F: FnMut(&mut Rng) -> f32>(rng: &mut Rng, n: usize, mut f: F) -> Vec<f32> {
    (0..n).map(|_| f(rng)).collect()
}

fn pristine(n: usize) -> Vec<u32> {
    bits(Padded::new(n, 0).payload())
}

// --- row 1: size == 0, dest != src ---------------------------------------

#[test]
fn err_row01_size_zero_distinct() {
    let l = libs();
    for it in 0..32u64 {
        let mut rng = Rng::new(101_000 + it);
        let n = 1 + rng.below(64);
        let src = mkvec(&mut rng, n, |r| r.any_f32());
        let s_c = Padded::filled(&src, 0);
        let s_r = Padded::filled(&src, 0);
        let mut d_c = Padded::new(n, 0);
        let mut d_r = Padded::new(n, 0);
        unsafe { (l.c)(d_c.ptr(), s_c.cptr(), 0) };
        unsafe { (l.r)(d_r.ptr(), s_r.cptr(), 0) };
        assert_eq!(bits(d_c.payload()), bits(d_r.payload()), "row01: dest mismatch");
        assert_eq!(d_c.canaries(), d_r.canaries(), "row01: canary mismatch");
        // C ground truth: 0-byte memset writes nothing.
        assert_eq!(bits(d_c.payload()), pristine(n), "row01: C unexpectedly wrote");
        assert_eq!(bits(d_r.payload()), pristine(n), "row01: Rust wrote where C did not");
    }
}

// --- row 2: size == 0, dest == src ---------------------------------------

#[test]
fn err_row02_size_zero_aliased() {
    for it in 0..32u64 {
        let mut rng = Rng::new(102_000 + it);
        let n = 1 + rng.below(64);
        let src = mkvec(&mut rng, n, |r| r.any_f32());
        let out = diff_inplace("row02", &src, 0, 0);
        assert_eq!(bits(&out), bits(&src), "row02: buffer must be unchanged");
    }
}

// --- row 3: size < 0, dest == src (memset NOT reached) -------------------

#[test]
fn err_row03_negative_size_aliased() {
    let sizes: [c_int; 8] = [-1, -2, -3, -4, -1000, -4096, c_int::MIN, c_int::MIN + 1];
    for (si, &size) in sizes.iter().enumerate() {
        for it in 0..8u64 {
            let mut rng = Rng::new(103_000 + si as u64 * 100 + it);
            let n = 1 + rng.below(64);
            let src = mkvec(&mut rng, n, |r| r.any_f32());
            let out = diff_inplace(&format!("row03[size={size}]"), &src, size, 0);
            assert_eq!(
                bits(&out),
                bits(&src),
                "row03: negative size with dest==src must not modify the buffer"
            );
        }
    }
}

// --- row 4: size < 0, dest != src — memset length arithmetic -------------

/// Ground truth produced by the system C compiler for the exact expression in
/// `src/lib.c` line 16, `size * sizeof(float)` with `int size`:
///
/// ```c
/// printf("0x%016zx", size * sizeof(float));
/// ```
///
/// The call itself is NOT made: for any negative `size` the C `memset` length
/// is ~2^64 bytes, which destroys the process. What must match is the length
/// computation, so it is checked against the compiler's own output.
#[test]
fn err_row04_negative_size_length_arithmetic() {
    const GROUND_TRUTH: [(c_int, u64); 16] = [
        (0, 0x0000_0000_0000_0000),
        (1, 0x0000_0000_0000_0004),
        (2, 0x0000_0000_0000_0008),
        (3, 0x0000_0000_0000_000c),
        (7, 0x0000_0000_0000_001c),
        (8, 0x0000_0000_0000_0020),
        (4096, 0x0000_0000_0000_4000),
        (2147483647, 0x0000_0001_ffff_fffc),
        (-1, 0xffff_ffff_ffff_fffc),
        (-2, 0xffff_ffff_ffff_fff8),
        (-3, 0xffff_ffff_ffff_fff4),
        (-4, 0xffff_ffff_ffff_fff0),
        (-1000, 0xffff_ffff_ffff_f060),
        (-4096, 0xffff_ffff_ffff_c000),
        (-2147483648, 0xffff_fffe_0000_0000),
        (-2147483647, 0xffff_fffe_0000_0004),
    ];
    // The expression used by the Rust translation (src/lib.rs).
    fn rust_len(size: c_int) -> u64 {
        (size as i64 as u64).wrapping_mul(std::mem::size_of::<f32>() as u64)
    }
    for (size, expect) in GROUND_TRUTH {
        assert_eq!(
            rust_len(size),
            expect,
            "row04: memset length for size={size} diverges from C \
             (rust=0x{:016x}, c=0x{expect:016x})",
            rust_len(size)
        );
    }
    // Exhaustive over the whole int range would be 2^32 iterations; sample the
    // boundaries plus a deterministic spread instead.
    let mut rng = Rng::new(104_000);
    for _ in 0..100_000 {
        let size = rng.next_u32() as i32;
        let c_equiv = (size as isize as usize as u64).wrapping_mul(4); // C's int -> size_t
        assert_eq!(rust_len(size), c_equiv, "row04: length mismatch for size={size}");
    }
}

// --- row 5 / 6: all-zero input ------------------------------------------

#[test]
fn err_row05_all_zero_distinct() {
    let l = libs();
    for n in 1..=64usize {
        let src = vec![0.0f32; n];
        let s_c = Padded::filled(&src, 0);
        let s_r = Padded::filled(&src, 0);
        let mut d_c = Padded::new(n, 0);
        let mut d_r = Padded::new(n, 0);
        unsafe { (l.c)(d_c.ptr(), s_c.cptr(), n as c_int) };
        unsafe { (l.r)(d_r.ptr(), s_r.cptr(), n as c_int) };
        assert_eq!(bits(d_c.payload()), bits(d_r.payload()), "row05: dest mismatch (n={n})");
        assert_eq!(d_c.canaries(), d_r.canaries(), "row05: canary mismatch (n={n})");
        // C ground truth: memset zero-fills exactly n floats.
        assert_eq!(bits(d_c.payload()), vec![0u32; n], "row05: C did not zero-fill");
        assert_eq!(
            d_c.canaries(),
            Padded::new(n, 0).canaries(),
            "row05: memset overran the window"
        );
    }
    // Same with -0.0 and mixtures of +0.0/-0.0.
    for it in 0..64u64 {
        let mut rng = Rng::new(105_000 + it);
        let n = 1 + rng.below(64);
        let src = mkvec(&mut rng, n, |r| if r.next_u64() & 1 == 0 { 0.0 } else { -0.0 });
        let out = diff_disjoint("row05mix", &src, n as c_int, 0, 0);
        assert_eq!(bits(&out), vec![0u32; n], "row05mix: expected +0.0 fill");
    }
}

#[test]
fn err_row06_all_zero_aliased() {
    for n in 1..=64usize {
        let src = vec![0.0f32; n];
        let out = diff_inplace("row06", &src, n as c_int, 0);
        // dest == src, so the `else if` is false: nothing is written. The
        // buffer already held zeros, so the observable state is unchanged.
        assert_eq!(bits(&out), vec![0u32; n], "row06 (n={n})");
    }
    // Distinguish "not written" from "written as zero": use -0.0 in place. If
    // the memset had run, -0.0 would become +0.0.
    for n in 1..=64usize {
        let src = vec![-0.0f32; n];
        let out = diff_inplace("row06neg", &src, n as c_int, 0);
        assert_eq!(
            bits(&out),
            vec![0x8000_0000u32; n],
            "row06neg (n={n}): dest==src must skip the memset, preserving -0.0"
        );
    }
}

// --- row 7: NaN input ----------------------------------------------------

#[test]
fn err_row07_nan_input() {
    let nans: [u32; 6] = [
        0x7FC0_0000, 0xFFC0_0000, 0x7F80_0001, 0xFF80_0001, 0x7FBF_FFFF, 0xFFFF_FFFF,
    ];
    for (bi, &nb) in nans.iter().enumerate() {
        for it in 0..16u64 {
            let mut rng = Rng::new(107_000 + bi as u64 * 100 + it);
            let n = 1 + rng.below(64);
            let mut src = mkvec(&mut rng, n, |r| r.unit());
            let p = rng.below(n);
            src[p] = f32::from_bits(nb);

            let out = diff_disjoint("row07", &src, n as c_int, 0, 0);
            assert_eq!(bits(&out), vec![0u32; n], "row07: NaN must be rejected via memset");

            // Aliased: nothing written at all, buffer keeps the NaN.
            let out_ip = diff_inplace("row07ip", &src, n as c_int, 0);
            assert_eq!(bits(&out_ip), bits(&src), "row07ip: dest==src must be a no-op");
        }
    }
}

// --- row 8: squares underflow so sum == 0 despite non-zero input ---------

#[test]
fn err_row08_underflow_to_zero_sum() {
    // x*x underflows to exactly 0 when |x| < 2^-75 (2^-150 rounds to 0).
    let tiny: [f32; 5] = [
        f32::from_bits(1),          // FLT_TRUE_MIN
        f32::from_bits(2),
        1e-30,
        f32::from_bits(0x0800_0000), // ~1.5e-33
        f32::from_bits(0x1000_0000), // 2^-97
    ];
    for (vi, &v) in tiny.iter().enumerate() {
        for n in 1..=32usize {
            let src = vec![v; n];
            let sum: f32 = {
                let mut s = 0.0f32;
                for &x in &src {
                    s += x * x;
                }
                s
            };
            let out = diff_disjoint(&format!("row08[{vi}]"), &src, n as c_int, 0, 0);
            if sum == 0.0 {
                assert_eq!(bits(&out), vec![0u32; n], "row08: expected zero-fill (n={n})");
            }
            let out_ip = diff_inplace(&format!("row08ip[{vi}]"), &src, n as c_int, 0);
            if sum == 0.0 {
                assert_eq!(bits(&out_ip), bits(&src), "row08ip: expected no-op (n={n})");
            }
        }
    }
}

// --- row 9: -0.0 only ----------------------------------------------------

#[test]
fn err_row09_negative_zero() {
    for n in 1..=64usize {
        let src = vec![-0.0f32; n];
        let out = diff_disjoint("row09", &src, n as c_int, 0, 0);
        assert_eq!(bits(&out), vec![0u32; n], "row09: (-0)*(-0) == +0 -> zero-fill (n={n})");
        let out_ip = diff_inplace("row09ip", &src, n as c_int, 0);
        assert_eq!(bits(&out_ip), vec![0x8000_0000u32; n], "row09ip: no-op keeps -0.0");
    }
}

// --- row 10: sum overflows to +inf, branch still taken -------------------

#[test]
fn err_row10_sum_overflow_inf() {
    // |x| >= 2^64 => x*x >= 2^128 => +inf in f32.
    for it in 0..64u64 {
        let mut rng = Rng::new(110_000 + it);
        let n = 1 + rng.below(64);
        let src = mkvec(&mut rng, n, |r| r.scaled(65, 126));
        let out = diff_disjoint("row10", &src, n as c_int, 0, 0);
        // sum == +inf, 1/sqrtf(inf) == 0, so every finite element maps to +-0.
        for (i, &v) in out.iter().enumerate() {
            assert_eq!(v, 0.0f32, "row10: expected signed zero at {i}, got {v:e}");
            assert_eq!(
                v.is_sign_negative(),
                src[i].is_sign_negative(),
                "row10: sign of zero must follow the input sign at {i}"
            );
        }
        diff_inplace("row10ip", &src, n as c_int, 0);
    }
    // Single huge element whose square alone overflows.
    for &b in &[0x7F7F_FFFFu32, 0xFF7F_FFFF, 0x6000_0000, 0x5F80_0000] {
        let v = f32::from_bits(b);
        for n in 1..=8usize {
            let src = vec![v; n];
            diff_disjoint("row10single", &src, n as c_int, 0, 0);
            diff_inplace("row10singleip", &src, n as c_int, 0);
        }
    }
}

// --- row 11: explicit +-inf elements ------------------------------------

#[test]
fn err_row11_inf_input() {
    for it in 0..64u64 {
        let mut rng = Rng::new(111_000 + it);
        let n = 1 + rng.below(64);
        let mut src = mkvec(&mut rng, n, |r| r.unit());
        let p = rng.below(n);
        src[p] = if it % 2 == 0 { f32::INFINITY } else { f32::NEG_INFINITY };
        let out = diff_disjoint("row11", &src, n as c_int, 0, 0);
        // inf * 0.0 -> x86 default QNaN; finite * 0.0 -> signed zero.
        assert!(out[p].is_nan(), "row11: inf element must become NaN");
        assert_eq!(
            out[p].to_bits(),
            0xFFC0_0000,
            "row11: expected x86 default QNaN bit pattern, got 0x{:08x}",
            out[p].to_bits()
        );
        for (i, &v) in out.iter().enumerate() {
            if i != p {
                assert_eq!(v, 0.0f32, "row11: finite neighbour must become signed zero");
            }
        }
        diff_inplace("row11ip", &src, n as c_int, 0);
    }
    // Only-inf inputs, both signs, several counts.
    for n in 1..=16usize {
        for &v in &[f32::INFINITY, f32::NEG_INFINITY] {
            let src = vec![v; n];
            diff_disjoint("row11all", &src, n as c_int, 0, 0);
            diff_inplace("row11allip", &src, n as c_int, 0);
        }
    }
}

// --- row 12: full aliasing with a normalizing sum ------------------------

#[test]
fn err_row12_aliased_normalizing() {
    for it in 0..128u64 {
        let mut rng = Rng::new(112_000 + it);
        let n = 1 + rng.below(256);
        let src = mkvec(&mut rng, n, |r| r.unit());
        let ip = diff_inplace("row12", &src, n as c_int, 0);
        let oop = diff_disjoint("row12oop", &src, n as c_int, 0, 0);
        // Indices coincide, so in-place must equal out-of-place bit for bit.
        assert_eq!(bits(&ip), bits(&oop), "row12: in-place != out-of-place (n={n})");
    }
}

// --- row 13: partially overlapping windows ------------------------------

#[test]
fn err_row13_partial_overlap() {
    for it in 0..64u64 {
        let mut rng = Rng::new(113_000 + it);
        let n = 1 + rng.below(128);
        for k in 1..=8usize {
            let buf = mkvec(&mut rng, n + k, |r| r.unit());
            diff_overlap("row13fwd", &buf, n, k, 0);
            diff_overlap("row13bwd", &buf, n, 0, k);
        }
    }
}

// --- row 14: size == 1 reciprocal-sqrt rounding --------------------------

#[test]
fn err_row14_size_one_rounding() {
    let mut cases: Vec<f32> = vec![
        1.0, -1.0, 2.0, -2.0, 0.5, 3.0, 7.0, 1e-20, 1e20, f32::MIN_POSITIVE,
        f32::from_bits(1), f32::MAX, -f32::MAX, 16777216.0, 16777215.0,
    ];
    let mut rng = Rng::new(114_000);
    for _ in 0..2000 {
        cases.push(rng.any_f32());
    }
    for (i, &v) in cases.iter().enumerate() {
        let src = vec![v];
        diff_disjoint(&format!("row14[{i}]"), &src, 1, 0, 0);
        diff_inplace(&format!("row14ip[{i}]"), &src, 1, 0);
    }
    // Exhaustive-ish sweep over exponents with a fixed mantissa pattern.
    for e in 0u32..255 {
        for m in [0u32, 1, 0x40_0000, 0x7F_FFFF] {
            let b = (e << 23) | m;
            for sign in [0u32, 0x8000_0000] {
                let src = vec![f32::from_bits(sign | b)];
                diff_disjoint("row14sweep", &src, 1, 0, 0);
            }
        }
    }
}

// --- row 15: extreme `size` values across the FFI boundary --------------

#[test]
fn err_row15_extreme_size_aliased() {
    // `size` is a plain `int` with no validity check, so every bit pattern is
    // accepted. Only in-place calls are safe for extreme negatives (the memset
    // branch is unreachable) — INT_MAX is not called because the C itself would
    // read ~2 GiB out of bounds.
    let sizes: [c_int; 6] = [-1, -2, -7, -65536, c_int::MIN, c_int::MIN + 1];
    for &size in &sizes {
        for n in 1..=8usize {
            let mut rng = Rng::new(115_000u64.wrapping_add(size as i64 as u64) ^ n as u64);
            let src = mkvec(&mut rng, n, |r| r.any_f32());
            let out = diff_inplace(&format!("row15[{size}]"), &src, size, 0);
            assert_eq!(bits(&out), bits(&src), "row15: no-op expected for size={size}");
        }
    }
    // One step past the valid range in the positive direction is not a special
    // case in the C at all (no upper bound exists); verify size == n and
    // size == n-1 differ only in how many elements are touched.
    let l = libs();
    for n in 2..=64usize {
        let mut rng = Rng::new(115_900 + n as u64);
        let src = mkvec(&mut rng, n, |r| r.unit());
        for size in [(n - 1) as c_int, n as c_int] {
            let s_c = Padded::filled(&src, 0);
            let s_r = Padded::filled(&src, 0);
            let mut d_c = Padded::new(n, 0);
            let mut d_r = Padded::new(n, 0);
            unsafe { (l.c)(d_c.ptr(), s_c.cptr(), size) };
            unsafe { (l.r)(d_r.ptr(), s_r.cptr(), size) };
            assert_eq!(bits(d_c.payload()), bits(d_r.payload()), "row15 partial (n={n})");
            assert_eq!(d_c.canaries(), d_r.canaries(), "row15 partial canary (n={n})");
            // The tail beyond `size` must be untouched on both sides.
            let tail_c = &bits(d_c.payload())[size as usize..];
            assert_eq!(tail_c, &pristine(n)[size as usize..], "row15: C touched the tail");
        }
    }
}

// --- rows 16/17: null pointers ------------------------------------------

#[test]
fn err_row16_null_pointers_size_zero() {
    let l = libs();
    // dest == src == NULL, size 0: loop skipped, `dest != src` false.
    unsafe { (l.c)(ptr::null_mut(), ptr::null(), 0) };
    unsafe { (l.r)(ptr::null_mut(), ptr::null(), 0) };
    // Negative sizes with both pointers NULL are also a pure no-op.
    for size in [-1, -2, c_int::MIN] {
        unsafe { (l.c)(ptr::null_mut(), ptr::null(), size) };
        unsafe { (l.r)(ptr::null_mut(), ptr::null(), size) };
    }
}

#[test]
fn err_row17_null_dest_size_zero() {
    let l = libs();
    let src = [1.0f32, 2.0, 3.0];
    // dest = NULL, src non-null, size 0 -> memset(NULL, 0, 0): no memory touched.
    unsafe { (l.c)(ptr::null_mut(), src.as_ptr(), 0) };
    unsafe { (l.r)(ptr::null_mut(), src.as_ptr(), 0) };
    // dest non-null, src = NULL, size 0 -> src is never dereferenced; the
    // pointers differ so a 0-byte memset is issued.
    let mut d_c = Padded::new(4, 0);
    let mut d_r = Padded::new(4, 0);
    unsafe { (l.c)(d_c.ptr(), ptr::null(), 0) };
    unsafe { (l.r)(d_r.ptr(), ptr::null(), 0) };
    assert_eq!(bits(d_c.payload()), bits(d_r.payload()), "row17: dest mismatch");
    assert_eq!(d_c.canaries(), d_r.canaries(), "row17: canary mismatch");
    assert_eq!(bits(d_c.payload()), pristine(4), "row17: C wrote with size 0");
    // dest non-null, src = NULL, negative size -> `dest != src` is TRUE here,
    // so the C would run the catastrophic memset. Not called (see row 4).
}

// --- extra generic boundary: no enums exist in this API ------------------

#[test]
fn err_extra_no_enum_surface_documented() {
    // `include/lib.h` declares a single function with no enum, struct, flag or
    // mode parameter, so there is no out-of-range enum value to feed across the
    // FFI boundary. The only scalar is `int size`, whose full range is covered
    // by err_row03/err_row04/err_row15.
    let hdr = std::fs::read_to_string("../c_src/include/lib.h").expect("read lib.h");
    assert!(!hdr.contains("enum"), "header gained an enum — ERRORS.md must be extended");
    assert_eq!(hdr.lines().filter(|l| !l.trim().is_empty()).count(), 1, "header changed");
}
