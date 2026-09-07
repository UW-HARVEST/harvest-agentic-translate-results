//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! The public ABI is four plain `int`s: no pointers, no lengths, no enums. So
//! rows E1–E14 (the C code's twelve `||`-guard rejection conditions plus the
//! two `continue` skips) are *statically unreachable* from the export — every
//! internal call site passes a fixed valid argument. Each E-row is therefore
//! verified by its **observable consequence**: the exact arithmetic term that
//! would disappear (or change) from `memchra2`'s return value if the Rust
//! translation had inverted, dropped, or mis-typed that guard.
//!
//! Concretely, each E-row test
//!   1. asserts C and Rust agree bit-for-bit over randomized inputs, and
//!   2. asserts the return value is *inconsistent* with the guard having
//!      fired — i.e. reconstructs the expected contribution of that helper
//!      from a reference model and checks the C `.so` really includes it, so
//!      that the equality in (1) is meaningful rather than vacuous.
//!
//! Rows B1–B7 are the reachable generic FFI boundaries (int extremes, one step
//! past each internal value branch, non-finite float bit patterns, and the
//! unsigned-signature reinterpretation).

mod common;

use common::{Pair, Rng, BITS_1000_0, BITS_1_0};

const N: usize = 1500;

// ---------------------------------------------------------------------------
// Reference model of the C, used only to prove the E-row guards are NOT taken.
// ---------------------------------------------------------------------------

fn formatted(a: i32, b: i32, c: i32, d: i32) -> Vec<u8> {
    // snprintf(buffer, 64, "test%d-%d-%d-%d", a, b, c, d); max 51 bytes, so no
    // truncation is possible.
    let s = format!("test{a}-{b}-{c}-{d}");
    assert!(s.len() < 64, "unexpected snprintf truncation: {} bytes", s.len());
    s.into_bytes()
}

fn dash_count(a: i32, b: i32, c: i32, d: i32) -> i32 {
    formatted(a, b, c, d).iter().filter(|&&x| x == b'-').count() as i32
}

fn buf_sum(a: i32, b: i32, c: i32, d: i32) -> i32 {
    formatted(a, b, c, d)
        .iter()
        .fold(0i32, |acc, &x| acc.wrapping_add((x as i8) as i32))
}

fn interpreted(b: i32, c: i32, d: i32) -> i32 {
    i32::from_le_bytes([(b & 0xFF) as u8, (c & 0xFF) as u8, (d & 0xFF) as u8, 0])
}

fn xor_low_bytes(a: i32, b: i32, c: i32, d: i32) -> i32 {
    let mut r = 0i32;
    for v in [a, b, c, d] {
        r ^= ((v as u32) & 0xFF) as i32;
    }
    r
}

/// Full model of `memchra2` with every guard in its NOT-taken state, i.e. the
/// behaviour that rows E1..E14 require. Used to prove each helper's
/// contribution is actually present in the C `.so`'s result.
fn model(a: i32, b: i32, c: i32, d: i32) -> i32 {
    let mut result: i32 = 0;
    result = result.wrapping_add(dash_count(a, b, c, d).wrapping_mul(10)); // E11/E12 not taken
    result = result.wrapping_add(
        // E7/E8 not taken
        a.wrapping_add(b).wrapping_add(c).wrapping_add(d),
    );
    result = result.wrapping_add(3i32.wrapping_mul(5)); // E3..E6 not taken -> matches == 3
    let f = f32::from_bits(a as u32);
    if f > 0.0f32 && f < 1000.0f32 {
        result = result.wrapping_add(f as i32);
    }
    let bs = buf_sum(a, b, c, d); // E1/E2 not taken -> not -1
    if bs > 0 {
        result = result.wrapping_add(bs % 256);
    }
    result ^= interpreted(b, c, d); // E9/E10 not taken
    result = result.wrapping_add(xor_low_bytes(a, b, c, d)); // E13/E14 not taken
    result
}

/// Shared body for the E-rows: C == Rust, and C == the all-guards-not-taken
/// model (so the guard really did not fire in the C either).
fn e_row<F>(p: &Pair, label: &str, n: usize, mut gen: F)
where
    F: FnMut(&mut Rng) -> (i32, i32, i32, i32),
{
    let mut r = Rng::new(common::FIXED_SEED ^ label.len() as u64 ^ 0xE0);
    for _ in 0..n {
        let (a, b, c, d) = gen(&mut r);
        let gc = p.c(a, b, c, d);
        let gr = p.rust(a, b, c, d);
        assert_eq!(
            gc.to_le_bytes(),
            gr.to_le_bytes(),
            "[{label}] memchra2({a}, {b}, {c}, {d}): C = {gc}, Rust = {gr}"
        );
        assert_eq!(
            gc,
            model(a, b, c, d),
            "[{label}] guard unexpectedly fired in the C at ({a}, {b}, {c}, {d}): \
             C = {gc}, all-guards-not-taken model = {}",
            model(a, b, c, d)
        );
    }
}

fn any(r: &mut Rng) -> (i32, i32, i32, i32) {
    (r.next_i32(), r.next_i32(), r.next_i32(), r.next_i32())
}

// ------------------------------------------------------- E1 / E2
// process_buffer: buffer == NULL, *buffer == '\0'  ->  return -1
// Never taken: the caller passes `char buffer[64]` that snprintf filled with
// "test...". Observable: `buf_sum > 0` holds, so `result += buf_sum % 256`.
#[test]
fn e1_e2_process_buffer_guards() {
    let p = Pair::load();
    e_row(&p, "E1/E2", N, any);
    // Explicitly confirm the -1 path is not being taken: buf_sum's own
    // contribution must be the positive `% 256` term, never derived from -1.
    for &(a, b, c, d) in &[(0, 0, 0, 0), (1, 2, 3, 4), (i32::MIN, i32::MAX, -1, 1)] {
        let bs = buf_sum(a, b, c, d);
        assert!(bs > 0, "buf_sum must be positive, got {bs}");
        assert_eq!(formatted(a, b, c, d)[0], b't', "buffer[0] must be 't'");
        p.assert_eq_at("E1/E2/exact", a, b, c, d);
    }
}

// ------------------------------------------------------- E3..E6
// process_strings: strings == NULL, count <= 0 -> return 0;
//                  *i == NULL, **i == '\0'     -> continue
// Never taken: fixed 4-element array of non-empty literals, count = 4.
// Observable: matches == 3, contributing +15.
#[test]
fn e3_e6_process_strings_guards() {
    let p = Pair::load();
    e_row(&p, "E3/E6", N, any);
    // The +15 term is a constant; prove it is present by differencing the C
    // result against the model with `matches` forced to the rejected value 0.
    let (a, b, c, d) = (7, 11, 13, 17);
    let with_15 = p.c(a, b, c, d);
    let without_15 = model(a, b, c, d).wrapping_sub(15);
    assert_ne!(
        with_15, without_15,
        "the +15 from matches==3 must be present in the C result"
    );
    assert_eq!(with_15, model(a, b, c, d));
    p.assert_eq_at("E3/E6/exact", a, b, c, d);
}

// ------------------------------------------------------- E7 / E8
// safe_sum_array: arr == NULL, size == 0 -> return 0
// Never taken: `int values[4]`, size = 4. Observable: result += a+b+c+d.
#[test]
fn e7_e8_safe_sum_array_guards() {
    let p = Pair::load();
    e_row(&p, "E7/E8", N, any);
    // Vary only `a` by a known delta: the sum term must move the result by the
    // same delta (mod the other a-dependent terms, so pick `a` values that
    // keep dash_count, the float branch and the low byte identical).
    // a = 0x40000000 (f = 2.0) and a = 0x40000000 + 256 keep the low byte and
    // the float window, but differ in the sum by 256.
    let (b, c, d) = (5, 6, 7);
    let a1 = 0x4000_0000i32;
    let a2 = a1 + 256;
    let r1 = p.c(a1, b, c, d);
    let r2 = p.c(a2, b, c, d);
    assert_eq!(r1, model(a1, b, c, d));
    assert_eq!(r2, model(a2, b, c, d));
    assert_ne!(r1, r2, "the a+b+c+d sum term must influence the result");
    p.assert_eq_at("E7/E8/exact", a1, b, c, d);
    p.assert_eq_at("E7/E8/exact", a2, b, c, d);
}

// ------------------------------------------------------- E9 / E10
// interpret_as_int: bytes == NULL, len < sizeof(int) -> return 0
// Never taken: len = 4, exactly sizeof(int) — the boundary that PASSES.
// Observable: result ^= LE32(b&0xFF, c&0xFF, d&0xFF, 0).
#[test]
fn e9_e10_interpret_as_int_guards() {
    let p = Pair::load();
    e_row(&p, "E9/E10", N, any);
    // len == 4 is one step inside the boundary; len == 3 would reject. Prove
    // the xor term is live by finding two inputs differing only in b's low
    // byte and confirming the result changes accordingly.
    assert_eq!(std::mem::size_of::<i32>(), 4, "sizeof(int) assumption");
    for &(b_lo, expect_nonzero) in &[(0x00u32, false), (0x01, true), (0xFF, true)] {
        let b = (0x1000_0000u32 | b_lo) as i32;
        let (c, d) = (0, 0);
        let a = 0;
        assert_eq!(
            interpreted(b, c, d) != 0,
            expect_nonzero,
            "interpret_as_int model mismatch for low byte {b_lo:#x}"
        );
        assert_eq!(p.c(a, b, c, d), model(a, b, c, d));
        p.assert_eq_at("E9/E10/exact", a, b, c, d);
    }
}

// ------------------------------------------------------- E11 / E12
// count_occurrences: text == NULL, *text == '\0' -> return 0
// Never taken: buffer[0] == 't'. Observable: result += dash_count * 10.
#[test]
fn e11_e12_count_occurrences_guards() {
    let p = Pair::load();
    e_row(&p, "E11/E12", N, any);
    // dash_count ranges over 3..=7 with the sign pattern; each distinct value
    // must be reflected as a distinct +10*k term in the C result.
    for mask in 0u32..16 {
        let mk = |neg: bool| if neg { -1i32 } else { 1i32 };
        let (a, b, c, d) = (
            mk(mask & 1 != 0),
            mk(mask & 2 != 0),
            mk(mask & 4 != 0),
            mk(mask & 8 != 0),
        );
        let dc = dash_count(a, b, c, d);
        assert_eq!(dc, 3 + mask.count_ones() as i32, "dash_count model");
        assert_eq!(p.c(a, b, c, d), model(a, b, c, d));
        p.assert_eq_at("E11/E12/exact", a, b, c, d);
    }
}

// ------------------------------------------------------- E13 / E14
// complex_iteration: data == NULL, count == 0 -> return -1
// Never taken: `values`, count = 4. Observable: result += xor of low bytes
// (so a spurious -1 would shift the result by xor+1).
#[test]
fn e13_e14_complex_iteration_guards() {
    let p = Pair::load();
    e_row(&p, "E13/E14", N, any);
    // Pick inputs where the xor is 0: the result must equal the model, and a
    // -1 return would be visible as an off-by-one.
    let (a, b, c, d) = (0x0100_0055u32 as i32, 0x0200_0055u32 as i32, 0x0300_0055u32 as i32, 0x0400_0055u32 as i32);
    assert_eq!(xor_low_bytes(a, b, c, d), 0, "xor must cancel here");
    let got = p.c(a, b, c, d);
    assert_eq!(got, model(a, b, c, d));
    assert_ne!(
        got,
        model(a, b, c, d).wrapping_sub(1),
        "a spurious -1 from complex_iteration must not be present"
    );
    p.assert_eq_at("E13/E14/exact", a, b, c, d);
}

// ------------------------------------------------------- B1
#[test]
fn b1_int_min_extremes() {
    let p = Pair::load();
    p.assert_eq_at("B1", i32::MIN, i32::MIN, i32::MIN, i32::MIN);
    // INT_MIN in each single position.
    for i in 0..4 {
        let mut v = [1i32, 1, 1, 1];
        v[i] = i32::MIN;
        p.assert_eq_at("B1/single", v[0], v[1], v[2], v[3]);
    }
    // Confirm the 51-byte worst case really does not truncate.
    assert_eq!(
        format!("test{0}-{0}-{0}-{0}", i32::MIN).len(),
        51,
        "worst-case snprintf length"
    );
}

// ------------------------------------------------------- B2
#[test]
fn b2_int_max_extremes() {
    let p = Pair::load();
    p.assert_eq_at("B2", i32::MAX, i32::MAX, i32::MAX, i32::MAX);
    for i in 0..4 {
        let mut v = [1i32, 1, 1, 1];
        v[i] = i32::MAX;
        p.assert_eq_at("B2/single", v[0], v[1], v[2], v[3]);
    }
    // Mixed extremes.
    p.assert_eq_at("B2/mixed", i32::MIN, i32::MAX, i32::MIN, i32::MAX);
    p.assert_eq_at("B2/mixed", i32::MAX, i32::MIN, i32::MAX, i32::MIN);
}

// ------------------------------------------------------- B3
#[test]
fn b3_all_zero() {
    let p = Pair::load();
    p.assert_eq_at("B3", 0, 0, 0, 0);
    assert!(
        !(f32::from_bits(0) > 0.0f32),
        "+0.0f must fail the f > 0.0f guard"
    );
    for i in 0..4 {
        let mut v = [0i32; 4];
        v[i] = -1;
        p.assert_eq_at("B3/one-neg", v[0], v[1], v[2], v[3]);
        v[i] = 1;
        p.assert_eq_at("B3/one-pos", v[0], v[1], v[2], v[3]);
    }
}

// ------------------------------------------------------- B4
#[test]
fn b4_float_window_boundaries() {
    let p = Pair::load();
    let mut cases: Vec<u32> = vec![
        0,
        1,
        2,
        BITS_1_0 - 1,
        BITS_1_0,
        BITS_1_0 + 1,
        BITS_1000_0 - 1,
        BITS_1000_0,
        BITS_1000_0 + 1,
    ];
    // one step either side of every integral (int)f step from 1..999 is a lot;
    // sample the exponent boundaries instead: bits(2^k) for k = 0..9.
    for k in 0..10u32 {
        let v = (2.0f32).powi(k as i32);
        cases.push(v.to_bits());
        cases.push(v.to_bits() - 1);
        cases.push(v.to_bits() + 1);
    }
    for &bits in &cases {
        let a = bits as i32;
        for &(b, c, d) in &[(0, 0, 0), (1, 2, 3), (-1, -2, -3), (i32::MAX, i32::MIN, 0)] {
            p.assert_eq_at("B4", a, b, c, d);
        }
    }
}

// ------------------------------------------------------- B5
#[test]
fn b5_non_finite_float_bit_patterns() {
    let p = Pair::load();
    let patterns: [u32; 8] = [
        0x7F80_0000, // +inf
        0xFF80_0000, // -inf
        0x7FC0_0000, // quiet NaN
        0xFFC0_0000, // negative quiet NaN
        0x7F80_0001, // signalling NaN
        0xFF80_0001, // negative signalling NaN
        0x8000_0000, // -0.0
        0x8000_0001, // smallest negative subnormal
    ];
    for &bits in &patterns {
        let a = bits as i32;
        for &(b, c, d) in &[(0, 0, 0), (5, -6, 7), (i32::MIN, i32::MAX, -1)] {
            p.assert_eq_at("B5", a, b, c, d);
        }
    }
}

// ------------------------------------------------------- B6
#[test]
fn b6_smallest_subnormal() {
    let p = Pair::load();
    for a in [1i32, 2, 3, 0x007F_FFFF, 0x0080_0000] {
        let f = f32::from_bits(a as u32);
        assert!(f > 0.0f32 && f < 1000.0f32, "must be inside the window");
        assert_eq!(f as i32, 0, "(int)f must truncate to 0 here");
        for &(b, c, d) in &[(0, 0, 0), (-1, -1, -1), (i32::MAX, i32::MIN, 12345)] {
            p.assert_eq_at("B6", a, b, c, d);
        }
    }
}

// ------------------------------------------------------- B7
#[test]
fn b7_unsigned_ffi_signature() {
    let p = Pair::load();
    let cases: [u32; 10] = [
        0x0000_0000,
        0xFFFF_FFFF,
        0x8000_0000,
        0x7FFF_FFFF,
        0x7F80_0000,
        0x7FC0_0000,
        BITS_1_0,
        BITS_1000_0,
        0x0000_0001,
        0xDEAD_BEEF,
    ];
    for &a in &cases {
        for &b in &cases {
            let gc = p.c_u(a, b, b, a);
            let gr = p.rust_u(a, b, b, a);
            assert_eq!(
                gc, gr,
                "[B7] memchra2({a:#x}, {b:#x}, {b:#x}, {a:#x}): C = {gc:#x}, Rust = {gr:#x}"
            );
            // And the signed view of the identical words.
            let sc = p.c(a as i32, b as i32, b as i32, a as i32);
            assert_eq!(sc as u32, gc, "[B7] signed/unsigned view mismatch in C");
        }
    }
}
