// Phase C — error-path differential tests. One test per row of ERRORS.md.
// Each test constructs the exact rejecting condition and asserts the C `.so`
// and the Rust `.so` return the SAME sentinel / saturated value.

mod common;

use common::*;
use std::ffi::c_int;
use std::ptr;

fn both_sdti(d: f64) -> (c_int, c_int) {
    let p = pair();
    (
        unsafe { (p.c.safe_double_to_int)(d) },
        unsafe { (p.rust.safe_double_to_int)(d) },
    )
}

fn both_alloc(size: c_int, m: f64) -> (c_int, c_int) {
    let p = pair();
    (
        unsafe { (p.c.allocate_and_compute)(size, m) },
        unsafe { (p.rust.allocate_and_compute)(size, m) },
    )
}

fn both_switch(v: c_int, op: c_int) -> (c_int, c_int) {
    let p = pair();
    (
        unsafe { (p.c.switch_fallthrough_calculator)(v, op) },
        unsafe { (p.rust.switch_fallthrough_calculator)(v, op) },
    )
}

fn both_fallcalc(a: c_int, b: c_int, c_: c_int, d: c_int) -> (c_int, c_int) {
    let p = pair();
    (
        unsafe { (p.c.fallcalc)(a, b, c_, d) },
        unsafe { (p.rust.fallcalc)(a, b, c_, d) },
    )
}

// --- Rows 1-2: NaN --------------------------------------------------------

#[test]
fn err_01_nan() {
    let mut nans: Vec<f64> = vec![
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF8_0000_0000_0000), // +qNaN
        f64::from_bits(0xFFF8_0000_0000_0000), // -qNaN
        f64::from_bits(0x7FF0_0000_0000_0001), // +sNaN
        f64::from_bits(0xFFF0_0000_0000_0001), // -sNaN
        f64::from_bits(0x7FFF_FFFF_FFFF_FFFF),
        f64::from_bits(0xFFFF_FFFF_FFFF_FFFF),
    ];
    // Randomized NaN payloads (fixed seed).
    let mut rng = Rng::new(SEED ^ 0xA1);
    for _ in 0..2_000 {
        let payload = rng.next_u64() & 0x000F_FFFF_FFFF_FFFF;
        let sign = (rng.next_u64() & 1) << 63;
        if payload != 0 {
            nans.push(f64::from_bits(sign | 0x7FF0_0000_0000_0000 | payload));
        }
    }
    for d in nans {
        let (c, r) = both_sdti(d);
        eq(&format!("row1/2 NaN bits=0x{:016x}", d.to_bits()), c, r);
        assert_eq!(c, 0, "C must return 0 for NaN (bits 0x{:016x})", d.to_bits());
    }
}

// --- Rows 3-4: infinities -------------------------------------------------

#[test]
fn err_02_pos_inf() {
    let (c, r) = both_sdti(f64::INFINITY);
    eq("row3 +Inf", c, r);
    assert_eq!(c, i32::MAX);
}

#[test]
fn err_03_neg_inf() {
    let (c, r) = both_sdti(f64::NEG_INFINITY);
    eq("row4 -Inf", c, r);
    assert_eq!(c, i32::MIN);
}

// --- Rows 5-6: finite saturation -----------------------------------------

#[test]
fn err_04_ge_int_max() {
    let imax = i32::MAX as f64;
    let mut vals = vec![imax, imax + 1.0, imax + 1024.0, 1e300, f64::MAX, 2f64.powi(31)];
    let mut x = imax;
    for _ in 0..32 {
        x = next_after(x, f64::INFINITY);
        vals.push(x);
    }
    let mut rng = Rng::new(SEED ^ 0xA4);
    for _ in 0..5_000 {
        vals.push(rng.range_f64(imax, 1e308));
    }
    for d in vals {
        let (c, r) = both_sdti(d);
        eq(&format!("row5 d={d:?}"), c, r);
        assert_eq!(c, i32::MAX, "C must saturate to INT_MAX for {d:?}");
    }
}

#[test]
fn err_05_le_int_min() {
    let imin = i32::MIN as f64;
    let mut vals = vec![imin, imin - 1.0, imin - 1024.0, -1e300, f64::MIN, -(2f64.powi(31)) - 1.0];
    let mut x = imin;
    for _ in 0..32 {
        x = next_after(x, f64::NEG_INFINITY);
        vals.push(x);
    }
    let mut rng = Rng::new(SEED ^ 0xA5);
    for _ in 0..5_000 {
        vals.push(rng.range_f64(-1e308, imin));
    }
    for d in vals {
        let (c, r) = both_sdti(d);
        eq(&format!("row6 d={d:?}"), c, r);
        assert_eq!(c, i32::MIN, "C must saturate to INT_MIN for {d:?}");
    }
}

// --- Row 7: one step INSIDE the bounds must NOT saturate ------------------

#[test]
fn err_06_just_inside_bounds() {
    let imax = i32::MAX as f64;
    let imin = i32::MIN as f64;

    let a = next_after(imax, 0.0);
    let (c, r) = both_sdti(a);
    eq("row7 nextafter(INT_MAX, 0)", c, r);
    assert_eq!(c, 2147483646, "C truncates {a:?}");

    let b = next_after(imin, 0.0);
    let (c, r) = both_sdti(b);
    eq("row7 nextafter(INT_MIN, 0)", c, r);
    assert_eq!(c, -2147483647, "C truncates {b:?}");

    let mut x = imax;
    let mut y = imin;
    for _ in 0..64 {
        x = next_after(x, 0.0);
        y = next_after(y, 0.0);
        let (c, r) = both_sdti(x);
        eq(&format!("row7 inward+ {x:?}"), c, r);
        assert!(c < i32::MAX, "must not saturate at {x:?}");
        let (c, r) = both_sdti(y);
        eq(&format!("row7 inward- {y:?}"), c, r);
        assert!(c > i32::MIN, "must not saturate at {y:?}");
    }
}

// --- Row 8: zeros and subnormals -----------------------------------------

#[test]
fn err_07_zero_and_subnormal() {
    for d in [
        0.0,
        -0.0,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        5e-324,
        -5e-324,
        1e-300,
        -1e-300,
        f64::from_bits(1),
        f64::from_bits(0x8000_0000_0000_0001),
    ] {
        let (c, r) = both_sdti(d);
        eq(&format!("row8 d={d:?} bits=0x{:016x}", d.to_bits()), c, r);
        assert_eq!(c, 0, "C truncates {d:?} to 0");
    }
}

// --- Rows 9-10: malloc failure => -1 -------------------------------------

#[test]
fn err_08_alloc_negative_size() {
    let mut sizes: Vec<c_int> = (-64..0).collect();
    sizes.extend_from_slice(&[i32::MIN, i32::MIN + 1, -1000, -100_000, -(1 << 20), -(1 << 30)]);
    let mut rng = Rng::new(SEED ^ 0xA9);
    for _ in 0..500 {
        sizes.push(rng.range_i32(i32::MIN, -1));
    }
    for size in sizes {
        let (c, r) = both_alloc(size, 1.5);
        eq(&format!("row9 size={size}"), c, r);
        assert_eq!(c, -1, "C must return -1 for negative size {size}");
    }
}

#[test]
fn err_09_alloc_huge_size() {
    // Probed on this host: any size >= 2^29 makes malloc(size*16) fail.
    for size in [
        i32::MAX,
        i32::MAX - 1,
        i32::MAX / 2,
        1 << 30,
        1 << 29,
        536_870_912,
        1_073_741_823,
    ] {
        let (c, r) = both_alloc(size, 1.5);
        eq(&format!("row10 size={size}"), c, r);
        assert_eq!(c, -1, "C must return -1 when malloc fails (size={size})");
    }
}

// --- Row 11: size == 0 => malloc(0) is non-NULL => 0 (not -1) ------------

#[test]
fn err_10_alloc_zero_size() {
    let mut rng = Rng::new(SEED ^ 0xAB);
    let mut mults = vec![0.0, 1.5, -1.5, f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1e308];
    for _ in 0..1_000 {
        mults.push(rng.next_f64_bits());
    }
    for m in mults {
        let (c, r) = both_alloc(0, m);
        eq(&format!("row11 size=0 mult bits=0x{:016x}", m.to_bits()), c, r);
        assert_eq!(c, 0, "malloc(0) is non-NULL so C must return 0, not -1");
    }
}

// --- Rows 12-14: non-finite multipliers ----------------------------------

#[test]
fn err_11_alloc_nan_multiplier() {
    for size in 1..=16 {
        for nan in [f64::NAN, -f64::NAN, f64::from_bits(0x7FF0_0000_0000_0001)] {
            let (c, r) = both_alloc(size, nan);
            eq(&format!("row12 size={size} nan bits=0x{:016x}", nan.to_bits()), c, r);
            assert_eq!(c, 0, "NaN sum must funnel into the isnan guard => 0");
        }
    }
}

#[test]
fn err_12_alloc_inf_multiplier() {
    // For EVERY size >= 1, points[0].coefficient == 0.0 * inf == NaN and
    // points[0].value == 0, so the first term is 0 * NaN == NaN, which poisons
    // the whole accumulation. The isnan() guard therefore fires and C returns 0
    // -- the isinf() branch is NOT reached via this route.
    for size in 1..=16 {
        for inf in [f64::INFINITY, f64::NEG_INFINITY] {
            let (c, r) = both_alloc(size, inf);
            eq(&format!("row13/14 size={size} mult={inf:?}"), c, r);
            assert_eq!(
                c, 0,
                "0.0 * inf == NaN poisons sum => isnan guard => 0 (size={size}, mult={inf:?})"
            );
        }
    }
}

#[test]
fn err_13_alloc_overflow_to_inf() {
    for size in 2..=16 {
        let (c, r) = both_alloc(size, 1e300);
        eq(&format!("row15 size={size} mult=1e300"), c, r);
        assert_eq!(c, i32::MAX);
        let (c, r) = both_alloc(size, -1e300);
        eq(&format!("row15 size={size} mult=-1e300"), c, r);
        assert_eq!(c, i32::MIN);
        let (c, r) = both_alloc(size, f64::MAX);
        eq(&format!("row15 size={size} mult=DBL_MAX"), c, r);
        assert_eq!(c, i32::MAX);
    }
}

// --- Rows 16-18: process_array_reverse non-positive counts ---------------

#[test]
fn err_14_reverse_zero_count() {
    let p = pair();
    let mut buf: Vec<c_int> = vec![7, 8, 9, 10];
    let end = unsafe { buf.as_mut_ptr().add(3) };
    let c = unsafe { (p.c.process_array_reverse)(end, 0) };
    let r = unsafe { (p.rust.process_array_reverse)(end, 0) };
    eq("row16 count=0", c, r);
    assert_eq!(c, 0);
}

#[test]
fn err_15_reverse_negative_count() {
    let p = pair();
    let mut buf: Vec<c_int> = vec![1, 2, 3, 4];
    let end = unsafe { buf.as_mut_ptr().add(3) };
    let mut counts = vec![i32::MIN, i32::MIN + 1, -1, -2, -1000, -(1 << 20)];
    let mut rng = Rng::new(SEED ^ 0xB1);
    for _ in 0..500 {
        counts.push(rng.range_i32(i32::MIN, -1));
    }
    for n in counts {
        let c = unsafe { (p.c.process_array_reverse)(end, n) };
        let r = unsafe { (p.rust.process_array_reverse)(end, n) };
        eq(&format!("row17 count={n}"), c, r);
        assert_eq!(c, 0, "negative count must yield 0 (count={n})");
    }
}

#[test]
fn err_16_reverse_null_ptr_zero_count() {
    let p = pair();
    for n in [0, -1, -100, i32::MIN] {
        let c = unsafe { (p.c.process_array_reverse)(ptr::null_mut(), n) };
        let r = unsafe { (p.rust.process_array_reverse)(ptr::null_mut(), n) };
        eq(&format!("row18 NULL count={n}"), c, r);
        assert_eq!(c, 0, "NULL + non-positive count must be 0 (count={n})");
    }
}

// --- Rows 19-21: foreach_sum non-positive counts -------------------------

#[test]
fn err_17_foreach_zero_count() {
    let p = pair();
    let mut buf: Vec<c_int> = vec![5, 6, 7];
    let base = buf.as_mut_ptr();
    let c = unsafe { (p.c.foreach_sum)(base, 0) };
    let r = unsafe { (p.rust.foreach_sum)(base, 0) };
    eq("row19 count=0", c, r);
    assert_eq!(c, 0);
}

#[test]
fn err_18_foreach_negative_count() {
    let p = pair();
    let mut buf: Vec<c_int> = vec![5, 6, 7];
    let base = buf.as_mut_ptr();
    let mut counts = vec![i32::MIN, i32::MIN + 1, -1, -2, -1000, -(1 << 20)];
    let mut rng = Rng::new(SEED ^ 0xB4);
    for _ in 0..500 {
        counts.push(rng.range_i32(i32::MIN, -1));
    }
    for n in counts {
        let c = unsafe { (p.c.foreach_sum)(base, n) };
        let r = unsafe { (p.rust.foreach_sum)(base, n) };
        eq(&format!("row20 count={n}"), c, r);
        assert_eq!(c, 0, "negative count must yield 0 (count={n})");
    }
}

#[test]
fn err_19_foreach_null_ptr_zero_count() {
    let p = pair();
    for n in [0, -1, -100, i32::MIN] {
        let c = unsafe { (p.c.foreach_sum)(ptr::null_mut(), n) };
        let r = unsafe { (p.rust.foreach_sum)(ptr::null_mut(), n) };
        eq(&format!("row21 NULL count={n}"), c, r);
        assert_eq!(c, 0, "NULL + non-positive count must be 0 (count={n})");
    }
}

// --- Row 22: out-of-range "enum" operation values => default arm ---------

#[test]
fn err_20_switch_default_arm() {
    let mut ops: Vec<c_int> = vec![
        5,
        6,
        7,
        -1,
        -2,
        -5,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        1 << 30,
        -(1 << 30),
        0x1_0000_0000u64 as u32 as i32, // == 0, sanity: wraps to a *valid* arm
        255,
        256,
        1000,
    ];
    let mut rng = Rng::new(SEED ^ 0xB7);
    for _ in 0..5_000 {
        let o = rng.next_i32();
        if !(0..=4).contains(&o) {
            ops.push(o);
        }
    }
    for op in ops {
        for &v in INTERESTING {
            let (c, r) = both_switch(v, op);
            eq(&format!("row22 value={v} op={op}"), c, r);
            if !(0..=4).contains(&op) {
                assert_eq!(c, 0, "default arm must zero the result (op={op}, value={v})");
            }
        }
    }
}

// --- Rows 23-24: signed overflow inside the switch arms -----------------

#[test]
fn err_21_switch_overflow_values() {
    let extremes = [i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1, 1 << 30, -(1 << 30)];
    for &v in &extremes {
        for op in 0..=4 {
            let (c, r) = both_switch(v, op);
            eq(&format!("row23/24 value={v} op={op}"), c, r);
        }
    }
    // Arms 0/1/2 mask with 0777, so the result must land in 0..=511.
    for &v in &extremes {
        for op in 0..=2 {
            let (c, _) = both_switch(v, op);
            assert!((0..=511).contains(&c), "masked arm out of range: {c}");
        }
    }
    let mut rng = Rng::new(SEED ^ 0xB9);
    for _ in 0..20_000 {
        let v = if rng.next_u64() & 1 == 0 {
            i32::MAX - rng.range_i32(0, 4)
        } else {
            i32::MIN + rng.range_i32(0, 4)
        };
        let op = rng.range_i32(0, 4);
        let (c, r) = both_switch(v, op);
        eq(&format!("row23/24 rnd value={v} op={op}"), c, r);
    }
}

// --- Row 25: negative param3 => truncating % => default arm -------------

#[test]
fn err_22_fallcalc_negative_param3() {
    let mut rng = Rng::new(SEED ^ 0xBA);
    for p3 in -100..0 {
        for _ in 0..64 {
            let (a, b, d) = (rng.range_i32(-1000, 1000), rng.next_i32(), rng.range_i32(0, 100));
            let (c, r) = both_fallcalc(a, b, p3, d);
            eq(&format!("row25 fallcalc({a},{b},{p3},{d})"), c, r);
        }
    }
    // param3 % 5 == 0 for negative multiples of 5 -> a VALID arm; the other
    // negatives give -1..-4 -> default arm. Both must match.
    for p3 in [-5, -10, -15, -1, -2, -3, -4, -6, -7, -8, -9] {
        for a in [0, 1, -1, 7] {
            for b in [0, 1, -1, 100] {
                let (c, r) = both_fallcalc(a, b, p3, 3);
                eq(&format!("row25 fallcalc({a},{b},{p3},3)"), c, r);
            }
        }
    }
}

// --- Rows 26-27: negative param4 => inner alloc failure / zero size -----

#[test]
fn err_23_fallcalc_negative_param4() {
    let mut rng = Rng::new(SEED ^ 0xBC);
    for p4 in -120..0 {
        for _ in 0..32 {
            let (a, b, c3) = (rng.range_i32(-500, 500), rng.range_i32(-500, 500), rng.range_i32(0, 500));
            let (c, r) = both_fallcalc(a, b, c3, p4);
            eq(&format!("row26/27 fallcalc({a},{b},{c3},{p4})"), c, r);
        }
    }
    // The exact boundary: param4 % 10 == -1 (size 0) vs <= -2 (alloc fails).
    for p4 in [-1, -11, -21, -31, -101, -2, -12, -9, -19] {
        for a in [0, 1, -1, 5] {
            let (c, r) = both_fallcalc(a, 0, 0, p4);
            eq(&format!("row27 fallcalc({a},0,0,{p4})"), c, r);
        }
    }
    // i32::MIN % 10 == -8 -> size -7 -> alloc failure
    let (c, r) = both_fallcalc(0, 0, 0, i32::MIN);
    eq("row26 fallcalc(0,0,0,INT_MIN)", c, r);
}

// --- Row 28: base_value overflow ----------------------------------------

#[test]
fn err_24_fallcalc_overflow_params() {
    let extremes = [i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1, 1 << 25, -(1 << 25), 1 << 30];
    for &a in &extremes {
        for &b in &extremes {
            for c3 in [0, 1, 2, 3, 4, 128, 129, -1] {
                for d in [0, 1, 9, -1, -2] {
                    let (c, r) = both_fallcalc(a, b, c3, d);
                    eq(&format!("row28 fallcalc({a},{b},{c3},{d})"), c, r);
                }
            }
        }
    }
}

// --- Rows 29-30: the `param3 > 0200` flag boundary ----------------------

#[test]
fn err_25_fallcalc_flag_boundary() {
    let mut rng = Rng::new(SEED ^ 0xBE);
    for p3 in 120..=136 {
        for _ in 0..256 {
            let (a, b, d) = (rng.next_i32(), rng.next_i32(), rng.range_i32(-20, 20));
            let (c, r) = both_fallcalc(a, b, p3, d);
            eq(&format!("row29/30 fallcalc({a},{b},{p3},{d})"), c, r);
        }
    }
    // param3 == 128 must NOT set the flag (strict >), 129 must.
    for _ in 0..2_000 {
        let (a, b, d) = (rng.next_i32(), rng.next_i32(), rng.range_i32(0, 20));
        let (c128, r128) = both_fallcalc(a, b, 128, d);
        eq("row30 p3=128", c128, r128);
        let (c129, r129) = both_fallcalc(a, b, 129, d);
        eq("row29 p3=129", c129, r129);
        assert_eq!(c129 & 0o200, 0o200, "p3=129 must force bit 7");
    }
}

// --- Rows 31-32: extremes cross product ---------------------------------

#[test]
fn err_26_fallcalc_extremes() {
    // INT_MIN % 5 == -3 in C (truncating), so the default arm is taken.
    let (c, r) = both_fallcalc(0, 0, i32::MIN, 0);
    eq("row31 p3=INT_MIN", c, r);

    let ext = [i32::MIN, -1, 0, i32::MAX];
    for &a in &ext {
        for &b in &ext {
            for &c3 in &ext {
                for &d in &ext {
                    let (c, r) = both_fallcalc(a, b, c3, d);
                    eq(&format!("row32 fallcalc({a},{b},{c3},{d})"), c, r);
                }
            }
        }
    }
    // Wider extremes cross-product (all 6^4 = 1296 combos).
    let ext2 = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX];
    for &a in &ext2 {
        for &b in &ext2 {
            for &c3 in &ext2 {
                for &d in &ext2 {
                    let (c, r) = both_fallcalc(a, b, c3, d);
                    eq(&format!("row32b fallcalc({a},{b},{c3},{d})"), c, r);
                }
            }
        }
    }
}

// --- Rows 33-34: post-mask invariants -----------------------------------

#[test]
fn err_27_fallcalc_never_returns_minus_one() {
    let mut rng = Rng::new(SEED ^ 0xC1);
    for _ in 0..50_000 {
        let (a, b, c3, d) = (rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32());
        let (c, r) = both_fallcalc(a, b, c3, d);
        eq(&format!("row33 fallcalc({a},{b},{c3},{d})"), c, r);
        assert_ne!(c, -1, "the `& 0777` mask makes -1 unreachable");
        assert_ne!(r, -1, "the `& 0777` mask makes -1 unreachable");
    }
}

#[test]
fn err_28_fallcalc_range_invariant() {
    let mut rng = Rng::new(SEED ^ 0xC2);
    for _ in 0..50_000 {
        let (a, b, c3, d) = (rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32());
        let (c, r) = both_fallcalc(a, b, c3, d);
        eq(&format!("row34 fallcalc({a},{b},{c3},{d})"), c, r);
        assert!((0..=511).contains(&c), "C result {c} outside 0..=511");
        assert!((0..=511).contains(&r), "Rust result {r} outside 0..=511");
    }
}
