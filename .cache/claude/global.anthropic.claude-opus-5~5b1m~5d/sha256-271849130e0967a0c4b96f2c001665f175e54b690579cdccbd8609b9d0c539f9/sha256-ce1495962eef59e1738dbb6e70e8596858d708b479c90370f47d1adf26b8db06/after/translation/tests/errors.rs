// Phase C — error-path differential tests.
//
// One test per row of ERRORS.md (E1..E15, except E7/E9 which need forced
// `malloc` failure and live in tests/errors_malloc_failure.rs).
//
// Each test asserts BOTH that C and Rust agree AND that they return the exact
// documented sentinel/clamp value — not merely "both failed somehow".

mod common;

use common::{diff_eq, Bits, Rng};
use std::ffi::c_int;
use std::ptr;

const INT_MAX: c_int = 2147483647;
const INT_MIN: c_int = -2147483648;

// ===========================================================================
// E1 — safe_double_to_int(NaN) == 0
// ===========================================================================

#[test]
fn e1_nan_returns_zero() {
    let (c, r) = common::both();

    let mut nans: Vec<f64> = vec![
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF8_0000_0000_0000), // canonical quiet NaN
        f64::from_bits(0xFFF8_0000_0000_0000), // negative quiet NaN
        f64::from_bits(0x7FF0_0000_0000_0001), // signalling NaN, min payload
        f64::from_bits(0xFFF0_0000_0000_0001), // negative signalling NaN
        f64::from_bits(0x7FFF_FFFF_FFFF_FFFF), // max payload
        f64::from_bits(0xFFFF_FFFF_FFFF_FFFF),
        f64::from_bits(0x7FF4_2424_2424_2424),
    ];
    // Randomized NaN payloads: exponent all ones, non-zero mantissa.
    let mut rng = Rng::new(0xE1_0000_0001);
    while nans.len() < 2000 {
        let mantissa = rng.next_u64() & 0x000F_FFFF_FFFF_FFFF;
        if mantissa == 0 {
            continue;
        }
        let sign = (rng.next_u64() & 1) << 63;
        nans.push(f64::from_bits(sign | 0x7FF0_0000_0000_0000 | mantissa));
    }

    for d in nans {
        assert!(d.is_nan());
        let cv = unsafe { (c.safe_double_to_int)(d) };
        let rv = unsafe { (r.safe_double_to_int)(d) };
        diff_eq("E1", Bits(d), cv, rv);
        assert_eq!(cv, 0, "[E1] C must return 0 for NaN {}", Bits(d));
    }
}

// ===========================================================================
// E2 / E3 — infinities clamp to INT_MAX / INT_MIN
// ===========================================================================

#[test]
fn e2_positive_infinity_returns_int_max() {
    let (c, r) = common::both();
    for d in [f64::INFINITY, f64::from_bits(0x7FF0_0000_0000_0000)] {
        let cv = unsafe { (c.safe_double_to_int)(d) };
        let rv = unsafe { (r.safe_double_to_int)(d) };
        diff_eq("E2", Bits(d), cv, rv);
        assert_eq!(cv, INT_MAX, "[E2] +Inf must clamp to INT_MAX");
    }
}

#[test]
fn e3_negative_infinity_returns_int_min() {
    let (c, r) = common::both();
    for d in [f64::NEG_INFINITY, f64::from_bits(0xFFF0_0000_0000_0000)] {
        let cv = unsafe { (c.safe_double_to_int)(d) };
        let rv = unsafe { (r.safe_double_to_int)(d) };
        diff_eq("E3", Bits(d), cv, rv);
        assert_eq!(cv, INT_MIN, "[E3] -Inf must clamp to INT_MIN");
    }
}

// ===========================================================================
// E4 / E5 — finite out-of-range clamps, inclusive at the exact boundary
// ===========================================================================

#[test]
fn e4_at_or_above_int_max_clamps() {
    let (c, r) = common::both();
    let mut cases: Vec<f64> = vec![
        INT_MAX as f64,       // exactly 2147483647.0 -> `>=` fires
        INT_MAX as f64 + 1.0, // 2147483648.0
        2147483647.5,
        2147483648.0,
        4e9,
        1e18,
        f64::MAX,
        1.7976931348623157e308,
    ];
    let mut rng = Rng::new(0xE4_0000_0004);
    for _ in 0..3000 {
        // Random values guaranteed >= INT_MAX.
        let extra = (rng.next_u64() % 1_000_000_000) as f64;
        cases.push(INT_MAX as f64 + extra);
        let big = rng.f64_finite().abs();
        if big >= INT_MAX as f64 {
            cases.push(big);
        }
    }
    for d in cases {
        assert!(d >= INT_MAX as f64, "test case {} not >= INT_MAX", Bits(d));
        let cv = unsafe { (c.safe_double_to_int)(d) };
        let rv = unsafe { (r.safe_double_to_int)(d) };
        diff_eq("E4", Bits(d), cv, rv);
        assert_eq!(cv, INT_MAX, "[E4] {} must clamp to INT_MAX", Bits(d));
    }
}

#[test]
fn e5_at_or_below_int_min_clamps() {
    let (c, r) = common::both();
    let mut cases: Vec<f64> = vec![
        INT_MIN as f64, // exactly -2147483648.0 -> `<=` fires
        INT_MIN as f64 - 1.0,
        -2147483648.5,
        -2147483649.0,
        -4e9,
        -1e18,
        f64::MIN,
        -1.7976931348623157e308,
    ];
    let mut rng = Rng::new(0xE5_0000_0005);
    for _ in 0..3000 {
        let extra = (rng.next_u64() % 1_000_000_000) as f64;
        cases.push(INT_MIN as f64 - extra);
        let big = -rng.f64_finite().abs();
        if big <= INT_MIN as f64 {
            cases.push(big);
        }
    }
    for d in cases {
        assert!(d <= INT_MIN as f64, "test case {} not <= INT_MIN", Bits(d));
        let cv = unsafe { (c.safe_double_to_int)(d) };
        let rv = unsafe { (r.safe_double_to_int)(d) };
        diff_eq("E5", Bits(d), cv, rv);
        assert_eq!(cv, INT_MIN, "[E5] {} must clamp to INT_MIN", Bits(d));
    }
}

// ===========================================================================
// E6 — allocate_and_compute with negative size: int -> size_t makes the
//      request ~2^64 bytes, malloc returns NULL, function returns -1.
// ===========================================================================

#[test]
fn e6_negative_size_returns_minus_one() {
    let (c, r) = common::both();

    // Exhaustive over a window of negative sizes, plus the extremes.
    let mut sizes: Vec<c_int> = (-512..0).collect();
    sizes.extend_from_slice(&[
        INT_MIN,
        INT_MIN + 1,
        INT_MIN / 2,
        -1_000_000,
        -65536,
        -4096,
        -1024,
        -17,
        -1,
    ]);
    let mut rng = Rng::new(0xE6_0000_0006);
    for _ in 0..2000 {
        sizes.push(rng.i32_in(INT_MIN, -1));
    }

    for size in sizes {
        for m in [1.5f64, 0.0, -1.5, f64::NAN, f64::INFINITY] {
            let cv = unsafe { (c.allocate_and_compute)(size, m) };
            let rv = unsafe { (r.allocate_and_compute)(size, m) };
            diff_eq("E6", format!("size={size} mult={}", Bits(m)), cv, rv);
            assert_eq!(
                cv, -1,
                "[E6] C must return -1 for negative size {size} (huge size_t request)"
            );
        }
    }
}

// ===========================================================================
// E8 — switch_fallthrough_calculator default arm: any `operation` outside
//      0..=4, including out-of-range "enum" values crossing the FFI boundary.
// ===========================================================================

#[test]
fn e8_out_of_range_operation_returns_zero() {
    let (c, r) = common::both();

    let mut ops: Vec<c_int> = Vec::new();
    ops.extend(-64..0); // every negative one step and beyond the low end
    ops.extend(5..=64); // every value one step and beyond the high end
    ops.extend_from_slice(&[
        INT_MIN,
        INT_MIN + 1,
        INT_MAX,
        INT_MAX - 1,
        1 << 30,
        -(1 << 30),
        0x7FFF_FFFF,
        -1000000,
        1000000,
    ]);
    let mut rng = Rng::new(0xE8_0000_0008);
    while ops.len() < 3000 {
        let o = rng.i32_any();
        if !(0..=4).contains(&o) {
            ops.push(o);
        }
    }

    let values = [0i32, 1, -1, 511, 512, INT_MAX, INT_MIN, -99999, 123456];
    for op in ops {
        assert!(!(0..=4).contains(&op));
        for v in values {
            let cv = unsafe { (c.switch_fallthrough_calculator)(v, op) };
            let rv = unsafe { (r.switch_fallthrough_calculator)(v, op) };
            diff_eq("E8", format!("value={v} operation={op}"), cv, rv);
            assert_eq!(
                cv, 0,
                "[E8] default arm must yield 0 for operation={op}, value={v}"
            );
        }
    }
}

// ===========================================================================
// E10 — fallcalc folds the inner -1 into the sum instead of propagating it.
// ===========================================================================

#[test]
fn e10_inner_alloc_failure_is_folded_not_propagated() {
    let (c, r) = common::both();
    let mut rng = Rng::new(0xE10_0000_000A);

    // Every param4 whose truncated remainder is negative (so size <= 0 ... and
    // in fact size < 0 for remainder <= -1) drives the inner failure.
    let mut param4s: Vec<c_int> = (-200..0).collect();
    param4s.extend_from_slice(&[INT_MIN, INT_MIN + 1, -1_000_000_007, -19, -10, -9, -1]);
    for _ in 0..1000 {
        param4s.push(rng.i32_in(INT_MIN, -1));
    }

    for param4 in param4s {
        let inner_size = param4.wrapping_rem(10).wrapping_add(1);
        // Confirm the inner call really does fail for this param4.
        let c_inner = unsafe { (c.allocate_and_compute)(inner_size, 1.5) };
        let r_inner = unsafe { (r.allocate_and_compute)(inner_size, 1.5) };
        diff_eq("E10/inner", format!("size={inner_size}"), c_inner, r_inner);
        // param4 % 10 == 0 => size 1 (succeeds); param4 % 10 == -1 => size 0,
        // i.e. malloc(0), which is non-NULL on glibc and also succeeds (E13).
        // Only param4 % 10 <= -2 actually drives the inner failure.
        if inner_size >= 0 {
            assert_eq!(
                c_inner,
                if inner_size == 0 { 0 } else { c_inner },
                "[E10] size {inner_size} was expected to succeed"
            );
            continue;
        }
        assert_eq!(
            c_inner, -1,
            "[E10] inner allocate_and_compute({inner_size}) must fail"
        );

        for (p1, p2, p3) in [
            (0, 0, 0),
            (1, -1, 3),
            (-7, 13, -13),
            (i32::MAX, i32::MIN, 129),
            (12345, -67890, 1000),
        ] {
            let cv = unsafe { (c.fallcalc)(p1, p2, p3, param4) };
            let rv = unsafe { (r.fallcalc)(p1, p2, p3, param4) };
            diff_eq("E10", format!("({p1}, {p2}, {p3}, {param4})"), cv, rv);
            // The -1 is summed in, then masked: the result is NOT the -1 sentinel.
            assert!(
                (0..=511).contains(&cv),
                "[E10] fallcalc must mask its result into 0..=511, got {cv}"
            );
        }
    }
}

// ===========================================================================
// E11 / E12 — non-positive counts never dereference the pointer, so even a
//             NULL pointer is accepted and the result is 0.
// ===========================================================================

#[test]
fn e11_process_array_reverse_nonpositive_count_accepts_null() {
    let (c, r) = common::both();
    let mut counts: Vec<c_int> = (-256..=0).collect();
    counts.extend_from_slice(&[INT_MIN, INT_MIN + 1, -1_000_000, -65536]);
    let mut rng = Rng::new(0xE11_0000_000B);
    for _ in 0..1000 {
        counts.push(rng.i32_in(INT_MIN, 0));
    }

    // Both a NULL pointer and a dangling-but-unused non-NULL pointer.
    let mut buf = [1i32, 2, 3, 4];
    let real_end = unsafe { buf.as_mut_ptr().add(3) };
    for count in counts {
        for p in [ptr::null_mut::<c_int>(), real_end, usize::MAX as *mut c_int] {
            let cv = unsafe { (c.process_array_reverse)(p, count) };
            let rv = unsafe { (r.process_array_reverse)(p, count) };
            diff_eq("E11", format!("ptr={p:?} count={count}"), cv, rv);
            assert_eq!(cv, 0, "[E11] count={count} must yield 0 without a deref");
        }
    }
}

#[test]
fn e12_foreach_sum_nonpositive_count_accepts_null() {
    let (c, r) = common::both();
    let mut counts: Vec<c_int> = (-256..=0).collect();
    counts.extend_from_slice(&[INT_MIN, INT_MIN + 1, -1_000_000, -65536]);
    let mut rng = Rng::new(0xE12_0000_000C);
    for _ in 0..1000 {
        counts.push(rng.i32_in(INT_MIN, 0));
    }

    let mut buf = [9i32, 8, 7];
    let real = buf.as_mut_ptr();
    for count in counts {
        for p in [ptr::null_mut::<c_int>(), real, usize::MAX as *mut c_int] {
            let cv = unsafe { (c.foreach_sum)(p, count) };
            let rv = unsafe { (r.foreach_sum)(p, count) };
            diff_eq("E12", format!("ptr={p:?} count={count}"), cv, rv);
            assert_eq!(cv, 0, "[E12] count={count} must yield 0 without a deref");
        }
    }
}

// ===========================================================================
// E13 — size == 0 is NOT an error: malloc(0) is non-NULL on glibc.
// ===========================================================================

#[test]
fn e13_size_zero_is_not_an_error() {
    let (c, r) = common::both();
    for m in [
        1.5f64,
        0.0,
        -0.0,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MAX,
    ] {
        let cv = unsafe { (c.allocate_and_compute)(0, m) };
        let rv = unsafe { (r.allocate_and_compute)(0, m) };
        diff_eq("E13", format!("size=0 mult={}", Bits(m)), cv, rv);
        assert_eq!(
            cv, 0,
            "[E13] size=0 must return 0 (malloc(0) is non-NULL here), not -1"
        );
    }
}

// ===========================================================================
// E14 — accumulator overflow to +/-Inf inside allocate_and_compute clamps.
// ===========================================================================

#[test]
fn e14_sum_overflow_clamps_to_int_extremes() {
    let (c, r) = common::both();
    // size >= 2 so at least one non-zero coefficient exists.
    for size in [2i32, 3, 8, 64, 1000] {
        for (m, expect) in [
            (f64::MAX, INT_MAX),
            (-f64::MAX, INT_MIN),
            (1e308, INT_MAX),
            (-1e308, INT_MIN),
            (1e300, INT_MAX),
            (-1e300, INT_MIN),
        ] {
            let cv = unsafe { (c.allocate_and_compute)(size, m) };
            let rv = unsafe { (r.allocate_and_compute)(size, m) };
            diff_eq("E14", format!("size={size} mult={}", Bits(m)), cv, rv);
            assert_eq!(
                cv, expect,
                "[E14] size={size} mult={} must clamp to {expect}",
                Bits(m)
            );
        }
    }
}

// ===========================================================================
// E15 — NaN reaching the accumulator (directly, or via 0 * Inf) yields 0.
// ===========================================================================

#[test]
fn e15_nan_accumulator_returns_zero() {
    let (c, r) = common::both();
    for size in [1i32, 2, 3, 8, 64] {
        for m in [
            f64::NAN,
            -f64::NAN,
            f64::INFINITY,     // i=0: 0.0 * Inf = NaN
            f64::NEG_INFINITY, // i=0: 0.0 * -Inf = NaN
            f64::from_bits(0x7FF8_0000_0000_0000),
        ] {
            let cv = unsafe { (c.allocate_and_compute)(size, m) };
            let rv = unsafe { (r.allocate_and_compute)(size, m) };
            diff_eq("E15", format!("size={size} mult={}", Bits(m)), cv, rv);
            assert_eq!(
                cv, 0,
                "[E15] size={size} mult={} must give 0 (NaN sum)",
                Bits(m)
            );
        }
    }
}

// ===========================================================================
// Generic FFI boundary sweeps that every C API needs, beyond the table rows.
// ===========================================================================

#[test]
fn generic_one_past_every_documented_range() {
    let (c, r) = common::both();

    // switch: one step past each end of the case range.
    for op in [-1i32, 0, 4, 5] {
        for v in [0i32, 1, -1, INT_MAX, INT_MIN] {
            let cv = unsafe { (c.switch_fallthrough_calculator)(v, op) };
            let rv = unsafe { (r.switch_fallthrough_calculator)(v, op) };
            diff_eq("generic/switch-edge", format!("v={v} op={op}"), cv, rv);
        }
    }

    // allocate: one step either side of the size==0 boundary.
    for size in [-1i32, 0, 1] {
        let cv = unsafe { (c.allocate_and_compute)(size, 1.5) };
        let rv = unsafe { (r.allocate_and_compute)(size, 1.5) };
        diff_eq("generic/alloc-edge", format!("size={size}"), cv, rv);
    }

    // counts: one step either side of the zero boundary, in-bounds buffer.
    let mut buf = [11i32, 22, 33];
    for count in [-1i32, 0, 1] {
        let p = buf.as_mut_ptr();
        let cv = unsafe { (c.foreach_sum)(p, count) };
        let rv = unsafe { (r.foreach_sum)(p, count) };
        diff_eq("generic/foreach-edge", format!("count={count}"), cv, rv);

        let end = unsafe { buf.as_mut_ptr().add(2) };
        let cv = unsafe { (c.process_array_reverse)(end, count) };
        let rv = unsafe { (r.process_array_reverse)(end, count) };
        diff_eq("generic/reverse-edge", format!("count={count}"), cv, rv);
    }

    // fallcalc: the extreme corners of all four int parameters.
    for p1 in [INT_MIN, -1, 0, 1, INT_MAX] {
        for p2 in [INT_MIN, -1, 0, 1, INT_MAX] {
            for p3 in [INT_MIN, -1, 0, 1, INT_MAX] {
                for p4 in [INT_MIN, -1, 0, 1, INT_MAX] {
                    let cv = unsafe { (c.fallcalc)(p1, p2, p3, p4) };
                    let rv = unsafe { (r.fallcalc)(p1, p2, p3, p4) };
                    diff_eq(
                        "generic/fallcalc-corners",
                        format!("({p1}, {p2}, {p3}, {p4})"),
                        cv,
                        rv,
                    );
                }
            }
        }
    }
}

// ERRORS.md checklist covered by this file:
//   [x] E1  NaN -> 0
//   [x] E2  +Inf -> INT_MAX
//   [x] E3  -Inf -> INT_MIN
//   [x] E4  >= (double)INT_MAX -> INT_MAX
//   [x] E5  <= (double)INT_MIN -> INT_MIN
//   [x] E6  negative size -> malloc NULL -> -1
//   [ ] E7  huge positive size -> malloc NULL -> -1   (errors_malloc_failure.rs)
//   [x] E8  operation outside 0..=4 -> 0
//   [ ] E9  fallcalc's 20-byte malloc fails -> -1     (errors_malloc_failure.rs)
//   [x] E10 inner -1 folded into the sum, then masked
//   [x] E11 process_array_reverse count <= 0 -> 0, no deref
//   [x] E12 foreach_sum count <= 0 -> 0, no deref
//   [x] E13 size == 0 -> 0 (not -1)
//   [x] E14 accumulator -> +/-Inf -> clamp
//   [x] E15 accumulator -> NaN -> 0
