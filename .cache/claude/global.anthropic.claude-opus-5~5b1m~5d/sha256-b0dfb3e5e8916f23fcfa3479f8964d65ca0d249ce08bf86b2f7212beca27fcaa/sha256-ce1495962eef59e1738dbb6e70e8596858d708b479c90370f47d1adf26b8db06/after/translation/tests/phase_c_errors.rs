//! Phase C — error / rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`. The C library has no explicit error
//! surface (no sentinels, no `errno`, no asserts, no pointer or length
//! parameters), so its rejection surface is entirely implicit and arithmetic:
//! every `int` is a valid successful return, and the only way to know what the
//! compiled C does at the overflow boundaries is to ask the C `.so` and demand
//! the Rust `.so` return the *same bits*.
//!
//! Each test therefore asserts equality of the concrete returned value or the
//! concrete emitted bytes — never merely "both did something".

mod common;

use common::{Pair, Rng};

const I32_MAX: i32 = i32::MAX;
const I32_MIN: i32 = i32::MIN;

// ---------------------------------------------------------------------------
// Rows 1-2: extreme single arguments on a fresh accumulator
// ---------------------------------------------------------------------------

/// ERRORS row 1 — `update == INT_MAX` on a fresh library. No trap; returns
/// `INT_MAX`.
#[test]
fn err01_sum_int_max_fresh() {
    let p = Pair::fresh("ERRORS row 1: static_sum(INT_MAX) fresh");
    assert_eq!(p.assert_sum(I32_MAX), I32_MAX);
}

/// ERRORS row 2 — `update == INT_MIN` on a fresh library.
#[test]
fn err02_sum_int_min_fresh() {
    let p = Pair::fresh("ERRORS row 2: static_sum(INT_MIN) fresh");
    assert_eq!(p.assert_sum(I32_MIN), I32_MIN);
}

// ---------------------------------------------------------------------------
// Rows 3-6: the four signed-overflow corners of `sum += update`
// ---------------------------------------------------------------------------

/// ERRORS row 3 — positive overflow: `sum == INT_MAX`, `update == 1`, expected
/// to wrap to `INT_MIN`.
#[test]
fn err03_positive_overflow_wraps_to_int_min() {
    let p = Pair::fresh("ERRORS row 3: INT_MAX + 1");
    p.seed_to(I32_MAX);
    let got = p.assert_sum(1);
    assert_eq!(
        got, I32_MIN,
        "the built C .so wraps INT_MAX + 1 to INT_MIN; Rust must do the same"
    );
}

/// ERRORS row 4 — negative overflow: `sum == INT_MIN`, `update == -1`, expected
/// to wrap to `INT_MAX`.
#[test]
fn err04_negative_overflow_wraps_to_int_max() {
    let p = Pair::fresh("ERRORS row 4: INT_MIN + (-1)");
    p.seed_to(I32_MIN);
    let got = p.assert_sum(-1);
    assert_eq!(got, I32_MAX);
}

/// ERRORS row 5 — maximal positive overflow: `INT_MAX + INT_MAX == -2`.
#[test]
fn err05_int_max_plus_int_max() {
    let p = Pair::fresh("ERRORS row 5: INT_MAX + INT_MAX");
    p.seed_to(I32_MAX);
    let got = p.assert_sum(I32_MAX);
    assert_eq!(got, -2);
}

/// ERRORS row 6 — maximal negative overflow: `INT_MIN + INT_MIN == 0`.
#[test]
fn err06_int_min_plus_int_min() {
    let p = Pair::fresh("ERRORS row 6: INT_MIN + INT_MIN");
    p.seed_to(I32_MIN);
    let got = p.assert_sum(I32_MIN);
    assert_eq!(got, 0);
}

// ---------------------------------------------------------------------------
// Rows 7-8: identity update and zero-crossing
// ---------------------------------------------------------------------------

/// ERRORS row 7 — `update == 0` must leave the accumulator untouched, at every
/// interesting accumulator value.
#[test]
fn err07_zero_update_is_identity() {
    let mut rng = Rng::for_row(107);
    let mut seeds = vec![0, 1, -1, I32_MAX, I32_MIN, I32_MAX / 2, I32_MIN / 2];
    for _ in 0..32 {
        seeds.push(rng.i32_any());
    }
    for seed in seeds {
        let p = Pair::fresh(format!("ERRORS row 7: static_sum(0) with sum = {seed}"));
        p.seed_to(seed);
        // Repeat: an identity operation must stay an identity.
        for _ in 0..3 {
            assert_eq!(p.assert_sum(0), seed);
        }
    }
}

/// ERRORS row 8 — positive accumulator driven across zero by a large negative
/// update, exhaustively at the boundary values that straddle the wrap.
#[test]
fn err08_cross_zero_boundary_values() {
    // (seed, update) pairs sitting exactly on/next to the wrap boundary.
    let cases: [(i32, i32); 12] = [
        (I32_MAX, -1),
        (I32_MAX, I32_MIN),
        (I32_MAX - 1, 2),
        (1, I32_MIN),
        (0, I32_MIN),
        (-1, I32_MIN),
        (I32_MIN, 1),
        (I32_MIN, I32_MAX),
        (I32_MIN + 1, -2),
        (I32_MIN + 1, -1),
        (2, -3),
        (-2, 3),
    ];
    for (seed, update) in cases {
        let p = Pair::fresh(format!("ERRORS row 8: sum = {seed}, update = {update}"));
        p.seed_to(seed);
        let got = p.assert_sum(update);
        assert_eq!(
            got,
            seed.wrapping_add(update),
            "sum = {seed}, update = {update}"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 9: argument truncation across the FFI boundary
// ---------------------------------------------------------------------------

/// ERRORS row 9 — an out-of-`int`-range value supplied in a 64-bit register.
/// Only the low 32 bits are part of the `int` parameter; both sides must agree.
/// This is the closest analogue this API has to "an out-of-range enum value
/// passed across FFI": a bit pattern with no meaning in the declared parameter
/// type.
#[test]
fn err09_out_of_range_argument_truncation() {
    let cases: [i64; 12] = [
        0x0000_0001_0000_0000,        // low half zero, high bit set
        0x0000_0001_0000_0005,        // == 5 after truncation
        0xFFFF_FFFF_0000_0000u64 as i64,
        0xFFFF_FFFF_FFFF_FFFFu64 as i64, // == -1
        0x0000_0000_8000_0000,        // == INT_MIN
        0x0000_0000_7FFF_FFFF,        // == INT_MAX
        0xDEAD_BEEF_8000_0000u64 as i64,
        0xDEAD_BEEF_7FFF_FFFFu64 as i64,
        i64::MIN,
        i64::MAX,
        0x1234_5678_9ABC_DEF0,
        0x0000_0002_FFFF_FFFF,
    ];
    for w in cases {
        let p = Pair::fresh(format!("ERRORS row 9: static_sum(<i64>{w:#018x})"));
        let got = p.assert_sum_wide(w);
        assert_eq!(
            got, w as i32,
            "the int parameter must be the low 32 bits of {w:#018x}"
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 10-13: `driver` overflow regimes
// ---------------------------------------------------------------------------

/// ERRORS row 10 — `stride == INT_MAX`: `i * stride` overflows for every
/// `i >= 2`, yet ten lines must still be produced with no trap.
#[test]
fn err10_driver_stride_int_max_product_overflow() {
    let p = Pair::fresh("ERRORS row 10: driver(INT_MAX)");
    let out = p.assert_driver(I32_MAX);
    assert_eq!(out.iter().filter(|&&b| b == b'\n').count(), 10);
    assert_eq!(out, model_driver(0, I32_MAX).into_bytes());
    // The row's premise: products really do overflow.
    for i in 2..10i32 {
        assert!(i.checked_mul(I32_MAX).is_none(), "i = {i} should overflow");
    }
}

/// ERRORS row 11 — `stride == INT_MIN`.
#[test]
fn err11_driver_stride_int_min_product_overflow() {
    let p = Pair::fresh("ERRORS row 11: driver(INT_MIN)");
    let out = p.assert_driver(I32_MIN);
    assert_eq!(out.iter().filter(|&&b| b == b'\n').count(), 10);
    assert_eq!(out, model_driver(0, I32_MIN).into_bytes());
    for i in 2..10i32 {
        assert!(i.checked_mul(I32_MIN).is_none(), "i = {i} should overflow");
    }
}

/// ERRORS row 12 — `stride == 0`: every update is the identity, so the current
/// accumulator is printed ten times unchanged. Checked from a fresh instance
/// and from a randomized carried-in accumulator.
#[test]
fn err12_driver_stride_zero_is_identity() {
    let p = Pair::fresh("ERRORS row 12: driver(0) fresh");
    assert_eq!(p.assert_driver(0), b"0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n");

    let mut rng = Rng::for_row(112);
    let mut seeds = vec![I32_MAX, I32_MIN, 1, -1];
    for _ in 0..16 {
        seeds.push(rng.i32_any());
    }
    for seed in seeds {
        let p = Pair::fresh(format!("ERRORS row 12: driver(0) with sum = {seed}"));
        p.seed_to(seed);
        let out = p.assert_driver(0);
        let want: String = (0..10).map(|_| format!("{seed}\n")).collect();
        assert_eq!(out, want.into_bytes());
    }
}

/// ERRORS row 13 — products fit but the accumulated total overflows. The
/// divisor must be at least 9, since the loop's largest multiplier is `i == 9`
/// and `9 * (INT_MAX / 8)` would itself overflow.
#[test]
fn err13_driver_accumulator_only_overflow() {
    for stride in [I32_MAX / 9, I32_MAX / 10, I32_MIN / 9, I32_MIN / 10] {
        let p = Pair::fresh(format!("ERRORS row 13: driver({stride})"));
        let out = p.assert_driver(stride);
        assert_eq!(out, model_driver(0, stride).into_bytes());

        let mut sum: i32 = 0;
        let mut saw = false;
        for i in 0..10i32 {
            assert!(
                i.checked_mul(stride).is_some(),
                "product i={i} stride={stride} should NOT overflow"
            );
            let (n, o) = sum.overflowing_add(i.wrapping_mul(stride));
            saw |= o;
            sum = n;
        }
        assert!(saw, "accumulator should overflow for stride = {stride}");
    }
}

// ---------------------------------------------------------------------------
// Rows 14-15: documented sequence, and wide argument to `driver`
// ---------------------------------------------------------------------------

/// ERRORS row 14 — `stride == -1` on a fresh library, exact expected bytes.
#[test]
fn err14_driver_stride_minus_one_exact_bytes() {
    let p = Pair::fresh("ERRORS row 14: driver(-1) fresh");
    let out = p.assert_driver(-1);
    assert_eq!(out, b"0\n-1\n-3\n-6\n-10\n-15\n-21\n-28\n-36\n-45\n");
}

/// ERRORS row 15 — `driver` invoked with an out-of-`int`-range 64-bit value;
/// the low 32 bits must be used as `stride` on both sides.
#[test]
fn err15_driver_wide_argument_truncation() {
    type DriverWide = unsafe extern "C" fn(i64);

    let cases: [i64; 8] = [
        0x0000_0001_0000_0001,
        0xFFFF_FFFF_0000_0000u64 as i64,
        0xFFFF_FFFF_FFFF_FFFFu64 as i64,
        0x0000_0000_8000_0000,
        i64::MIN,
        i64::MAX,
        0xDEAD_BEEF_0000_0002u64 as i64,
        0x1234_5678_9ABC_DEF0,
    ];

    for w in cases {
        let p = Pair::fresh(format!("ERRORS row 15: driver(<i64>{w:#018x})"));

        // Re-declare `driver` with a widened parameter to force the extra bits
        // into the argument register.
        let c_wide: DriverWide = unsafe { std::mem::transmute(p.c.driver) };
        let rs_wide: DriverWide = unsafe { std::mem::transmute(p.rs.driver) };

        let c_out = common::capture_stdout(|| unsafe { c_wide(w) });
        let rs_out = common::capture_stdout(|| unsafe { rs_wide(w) });

        assert_eq!(
            c_out,
            rs_out,
            "driver(<i64>{w:#018x}) stdout diverged:\n  C   : {:?}\n  Rust: {:?}",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&rs_out),
        );
        assert_eq!(
            c_out,
            model_driver(0, w as i32).into_bytes(),
            "driver must use only the low 32 bits of {w:#018x}"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 16: shared static state across both entry points
// ---------------------------------------------------------------------------

/// ERRORS row 16 — the two entry points share one accumulator. `static_sum`
/// must observe `driver`'s writes and vice versa, in both orders.
#[test]
fn err16_shared_static_state_both_directions() {
    let mut rng = Rng::for_row(116);

    // static_sum -> driver: driver must start from static_sum's value.
    for _ in 0..64 {
        let p = Pair::fresh("ERRORS row 16: static_sum then driver");
        let seed = rng.i32_any();
        p.seed_to(seed);
        let stride = rng.i32_any();
        assert_eq!(p.assert_driver(stride), model_driver(seed, stride).into_bytes());
    }

    // driver -> static_sum: static_sum must continue from driver's value.
    for _ in 0..64 {
        let p = Pair::fresh("ERRORS row 16: driver then static_sum");
        let stride = rng.i32_any();
        p.assert_driver(stride);
        let after = model_driver_final(0, stride);
        let u = rng.i32_any();
        assert_eq!(p.assert_sum(u), after.wrapping_add(u));
    }
}

// ---------------------------------------------------------------------------
// Row 17: totality — no input may abort, panic, or produce a sentinel
// ---------------------------------------------------------------------------

/// ERRORS row 17 — the contract is total. Sweep the boundary neighbourhoods
/// plus a broad random sample and confirm every single call returns normally
/// with identical bits on both sides. A Rust-side overflow panic (which under
/// `panic = "abort"` would kill the process) or any divergence fails here.
#[test]
fn err17_total_contract_no_rejection() {
    // Values one step past every documented boundary, plus power-of-two edges.
    let mut probes: Vec<i32> = vec![
        I32_MIN,
        I32_MIN + 1,
        I32_MIN + 2,
        -2,
        -1,
        0,
        1,
        2,
        I32_MAX - 2,
        I32_MAX - 1,
        I32_MAX,
    ];
    for shift in 0..31u32 {
        probes.push(1i32 << shift);
        probes.push(-(1i32 << shift));
        probes.push((1i32 << shift) - 1);
    }

    // Every ordered pair of probes, applied to a fresh accumulator.
    for &a in &probes {
        let p = Pair::fresh(format!("ERRORS row 17: boundary pair sweep, seed {a}"));
        p.seed_to(a);
        let mut expect = a;
        for &b in &probes {
            expect = expect.wrapping_add(b);
            assert_eq!(p.assert_sum(b), expect, "seed {a}, update {b}");
        }
    }

    // Broad random sweep, same instance.
    let mut rng = Rng::for_row(117);
    let p = Pair::fresh("ERRORS row 17: broad random totality sweep");
    let mut expect: i32 = 0;
    for _ in 0..5_000 {
        let u = rng.i32_any();
        expect = expect.wrapping_add(u);
        assert_eq!(p.assert_sum(u), expect);
    }

    // And `driver` over every probe value, each on a fresh instance.
    for &s in &probes {
        let p = Pair::fresh(format!("ERRORS row 17: driver totality, stride {s}"));
        let out = p.assert_driver(s);
        assert_eq!(out, model_driver(0, s).into_bytes());
    }
}

// ---------------------------------------------------------------------------
// Generic FFI boundary probes (required even though absent from ERRORS.md)
// ---------------------------------------------------------------------------

/// The public API takes no pointers, so there is no null-pointer parameter to
/// probe. Assert that mechanically — if the header ever grows a pointer
/// parameter this test documents that the coverage claim must be revisited —
/// and probe the nearest reachable analogue: a pointer-shaped bit pattern
/// (including 0) arriving in the argument register.
#[test]
fn generic_no_pointer_parameters_pointerlike_bit_patterns() {
    type PtrArg = unsafe extern "C" fn(*const std::ffi::c_void) -> std::ffi::c_int;

    let header = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/include/staticloop.h"),
    )
    .expect("read staticloop.h");
    let decls: Vec<&str> = header
        .lines()
        .filter(|l| l.contains("static_sum") || l.contains("driver"))
        .collect();
    assert_eq!(decls.len(), 2, "unexpected public declarations: {decls:?}");
    for d in &decls {
        assert!(
            !d.contains('*'),
            "a pointer parameter appeared in the public API ({d:?}); \
             null-pointer differential coverage is now required"
        );
    }

    let ptrs: [usize; 6] = [0, 1, 0xFFFF_FFFF, 0x8000_0000, usize::MAX, 0xDEAD_BEEF];
    for raw in ptrs {
        let p = Pair::fresh(format!("generic: static_sum(<ptr>{raw:#x})"));
        let c_f: PtrArg = unsafe { std::mem::transmute(p.c.static_sum) };
        let rs_f: PtrArg = unsafe { std::mem::transmute(p.rs.static_sum) };
        let cv = unsafe { c_f(raw as *const std::ffi::c_void) };
        let rv = unsafe { rs_f(raw as *const std::ffi::c_void) };
        assert_eq!(cv, rv, "pointer-shaped argument {raw:#x} diverged");
        assert_eq!(cv, raw as u32 as i32);
    }
}

/// The API has no length/size/count parameters, so "zero and oversized
/// lengths" reduce to `0` and to values past `INT_MAX`. Cover both as
/// arguments to each entry point.
#[test]
fn generic_zero_and_oversized_length_analogues() {
    // Zero.
    let p = Pair::fresh("generic: zero-length analogue");
    assert_eq!(p.assert_sum(0), 0);
    assert_eq!(p.assert_driver(0), b"0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n");

    // "Oversized": beyond the representable range of the declared int.
    for w in [
        i32::MAX as i64 + 1,
        u32::MAX as i64,
        u32::MAX as i64 + 1,
        i64::from(i32::MIN) - 1,
        1i64 << 40,
    ] {
        let p = Pair::fresh(format!("generic: oversized-length analogue {w}"));
        let got = p.assert_sum_wide(w);
        assert_eq!(got, w as i32);
    }
}

/// There are no enums in this API, so the "out-of-range enum value" class is
/// covered by feeding each entry point `int` values that carry no distinguished
/// meaning — including every value one step past the type's documented range
/// once truncated. Verified mechanically: the header declares no `enum`.
#[test]
fn generic_no_enum_parameters_meaningless_values() {
    let header = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/include/staticloop.h"),
    )
    .expect("read staticloop.h");
    assert!(
        !header.contains("enum"),
        "the public API grew an enum; dedicated out-of-range-variant \
         differential coverage is now required"
    );

    // Values that would be "invalid variants" for any enum-like parameter.
    let mut rng = Rng::for_row(999);
    for _ in 0..200 {
        let p = Pair::fresh("generic: meaningless int argument");
        let v = rng.i32_any();
        assert_eq!(p.assert_sum(v), v);
    }
    for v in [-1, 12345, I32_MIN, I32_MAX, 0x7F7F_7F7F, -0x7F7F_7F7F] {
        let p = Pair::fresh(format!("generic: meaningless driver argument {v}"));
        assert_eq!(p.assert_driver(v), model_driver(0, v).into_bytes());
    }
}

// ---------------------------------------------------------------------------
// Independent model of the C
// ---------------------------------------------------------------------------

fn model_driver(start: i32, stride: i32) -> String {
    let mut sum = start;
    let mut s = String::new();
    for i in 0..10i32 {
        sum = sum.wrapping_add(i.wrapping_mul(stride));
        s.push_str(&sum.to_string());
        s.push('\n');
    }
    s
}

fn model_driver_final(start: i32, stride: i32) -> i32 {
    let mut sum = start;
    for i in 0..10i32 {
        sum = sum.wrapping_add(i.wrapping_mul(stride));
    }
    sum
}
