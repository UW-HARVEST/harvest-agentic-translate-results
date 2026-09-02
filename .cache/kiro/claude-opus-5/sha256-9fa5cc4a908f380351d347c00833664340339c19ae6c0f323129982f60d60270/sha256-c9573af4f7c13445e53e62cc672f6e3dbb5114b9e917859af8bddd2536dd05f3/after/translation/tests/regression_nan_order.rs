//! Regression tests for the x86 SSE NaN operand-order behaviour of `c2Dot` and
//! `c2Mulvs`.
//!
//! Background: the C is built at `-O0`, so each float expression becomes one
//! scalar SSE instruction `OPss dst, src`. When an operand is NaN, x86 forwards
//! `src1` (= `dst`) quieted rather than computing anything, so the *operand
//! order* is observable in the returned NaN payload and sign bit. gcc's chosen
//! order inside `c2Dot` is asymmetric:
//!
//!   * x-term multiply : `src1 = a.x`
//!   * y-term multiply : `src1 = b.y`   (not `a.y`!)
//!   * final add       : `src1 = y_term` (not `x_term`!)
//!
//! A naive `a.x * b.x + a.y * b.y` translation gets the final add backwards, and
//! LLVM's register allocator commutes `fadd`/`fmul` freely so source order alone
//! cannot pin it down. `src/lib.rs` therefore models the selection rule via
//! `mul_ss` / `add_ss`. These tests fail if that modelling is ever removed.

#![allow(non_snake_case)]

mod common;
use common::*;

const NAN_A: u32 = 0x7F80_0001; // sNaN, payload 1        -> quiets to 0x7FC00001
const NAN_B: u32 = 0x7F92_3456; // sNaN, payload 0x123456 -> quiets to 0x7FD23456
const NAN_C: u32 = 0xFF9A_BCDE; // negative sNaN          -> quiets to 0xFFDABCDE

fn q(bits: u32) -> u32 {
    bits | 0x0040_0000
}

#[test]
fn regression_c2dot_y_term_multiply_takes_b_as_src1() {
    let (c, r) = libs();
    let f = f32::from_bits;
    // x-term is a plain 1.0 so it cannot mask the y-term's NaN.
    let a = c2v { x: 1.0, y: f(NAN_A) };
    let b = c2v { x: 1.0, y: f(NAN_B) };
    unsafe {
        let cd = (c.c2Dot)(a, b).to_bits();
        let rd = (r.c2Dot)(a, b).to_bits();
        assert_eq!(
            cd,
            q(NAN_B),
            "precondition: the C's y-term multiply should take b.y as src1"
        );
        assert_eq!(
            rd, cd,
            "c2Dot y-term operand order diverged: C=0x{cd:08x} RS=0x{rd:08x} \
             (a.y quiets to 0x{:08x}, b.y quiets to 0x{:08x})",
            q(NAN_A),
            q(NAN_B)
        );
    }
}

#[test]
fn regression_c2dot_x_term_multiply_takes_a_as_src1() {
    let (c, r) = libs();
    let f = f32::from_bits;
    let a = c2v { x: f(NAN_A), y: 1.0 };
    let b = c2v { x: f(NAN_B), y: 1.0 };
    unsafe {
        let cd = (c.c2Dot)(a, b).to_bits();
        let rd = (r.c2Dot)(a, b).to_bits();
        assert_eq!(
            cd,
            q(NAN_A),
            "precondition: the C's x-term multiply should take a.x as src1"
        );
        assert_eq!(rd, cd, "c2Dot x-term operand order diverged");
    }
}

#[test]
fn regression_c2dot_final_add_takes_y_term_as_src1() {
    let (c, r) = libs();
    let f = f32::from_bits;
    // x_term = quiet(NAN_A), y_term = quiet(NAN_C): both NaN, distinguishable.
    let a = c2v {
        x: f(NAN_A),
        y: f(NAN_C),
    };
    let b = c2v { x: 1.0, y: 1.0 };
    unsafe {
        let cd = (c.c2Dot)(a, b).to_bits();
        let rd = (r.c2Dot)(a, b).to_bits();
        assert_eq!(
            cd,
            q(NAN_C),
            "precondition: the C's final add should return the y term"
        );
        assert_eq!(
            rd, cd,
            "c2Dot final-add operand order diverged: C=0x{cd:08x} RS=0x{rd:08x} \
             (x_term=0x{:08x}, y_term=0x{:08x})",
            q(NAN_A),
            q(NAN_C)
        );
    }
}

#[test]
fn regression_c2mulvs_takes_the_vector_component_as_src1() {
    let (c, r) = libs();
    let f = f32::from_bits;
    let a = c2v {
        x: f(NAN_A),
        y: f(NAN_C),
    };
    let s = f(NAN_B);
    unsafe {
        let cm = (c.c2Mulvs)(a, s);
        let rm = (r.c2Mulvs)(a, s);
        assert_eq!(
            cm.x.to_bits(),
            q(NAN_A),
            "precondition: c2Mulvs should take a.x as src1"
        );
        assert_eq!(
            cm.y.to_bits(),
            q(NAN_C),
            "precondition: c2Mulvs should take a.y as src1"
        );
        assert_eq!(
            (rm.x.to_bits(), rm.y.to_bits()),
            (cm.x.to_bits(), cm.y.to_bits()),
            "c2Mulvs operand order diverged"
        );
    }
}

/// The sweep that originally caught the bug: fully random 32-bit patterns, so
/// NaN x NaN operand pairs occur naturally. MUST reach zero mismatches.
#[test]
fn regression_zero_strict_bit_mismatches_over_random_patterns() {
    let (c, r) = libs();
    let n: u64 = std::env::var("REGRESSION_ITERS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2_000_000);

    let mut rng = Rng::seeded(9999);
    let mut bad = Vec::new();
    for i in 0..n {
        let (a, b) = (rng.vec_raw(), rng.vec_raw());
        unsafe {
            let (cd, rd) = ((c.c2Dot)(a, b), (r.c2Dot)(a, b));
            if cd.to_bits() != rd.to_bits() && bad.len() < 10 {
                bad.push(format!(
                    "#{i} c2Dot(({}, {}), ({}, {})) C=0x{:08x} RS=0x{:08x}",
                    show(a.x),
                    show(a.y),
                    show(b.x),
                    show(b.y),
                    cd.to_bits(),
                    rd.to_bits()
                ));
            }
        }
    }
    assert!(bad.is_empty(), "c2Dot strict-bit mismatches:\n  {}", bad.join("\n  "));

    let mut rng = Rng::seeded(4242);
    let mut bad = Vec::new();
    for i in 0..n {
        let a = rng.vec_raw();
        let s = rng.raw_f32();
        unsafe {
            let (cm, rm) = ((c.c2Mulvs)(a, s), (r.c2Mulvs)(a, s));
            if (cm.x.to_bits(), cm.y.to_bits()) != (rm.x.to_bits(), rm.y.to_bits())
                && bad.len() < 10
            {
                bad.push(format!("#{i} c2Mulvs({}, {})", show_v(a), show(s)));
            }
        }
    }
    assert!(bad.is_empty(), "c2Mulvs strict-bit mismatches:\n  {}", bad.join("\n  "));

    // And the non-arithmetic helpers, which return an operand verbatim.
    let mut rng = Rng::seeded(777);
    for i in 0..(n / 4) {
        let (a, b, d) = (rng.vec_raw(), rng.vec_raw(), rng.vec_raw());
        unsafe {
            diff_assert!(v_eq((c.c2Sub)(a, b), (r.c2Sub)(a, b)), "c2Sub #{i}");
            diff_assert!(v_eq((c.c2Maxv)(a, b), (r.c2Maxv)(a, b)), "c2Maxv #{i}");
            diff_assert!(v_eq((c.c2Minv)(a, b), (r.c2Minv)(a, b)), "c2Minv #{i}");
            diff_assert!(
                v_eq((c.c2Clampv)(a, b, d), (r.c2Clampv)(a, b, d)),
                "c2Clampv #{i}"
            );
            diff_assert!(v_eq((c.c2V)(a.x, a.y), (r.c2V)(a.x, a.y)), "c2V #{i}");
        }
    }
}

/// Exhaustive NaN/subnormal cross product over all four `c2Dot` lanes.
#[test]
fn regression_c2dot_special_bit_pattern_cross_product() {
    let (c, r) = libs();
    for &ab in SPECIAL_BITS.iter() {
        for &bb in SPECIAL_BITS.iter() {
            for &cb in SPECIAL_BITS.iter() {
                for &db in SPECIAL_BITS.iter() {
                    let a = c2v {
                        x: f32::from_bits(ab),
                        y: f32::from_bits(bb),
                    };
                    let b = c2v {
                        x: f32::from_bits(cb),
                        y: f32::from_bits(db),
                    };
                    unsafe {
                        let (cd, rd) = ((c.c2Dot)(a, b), (r.c2Dot)(a, b));
                        assert_eq!(
                            cd.to_bits(),
                            rd.to_bits(),
                            "c2Dot(0x{ab:08x},0x{bb:08x} . 0x{cb:08x},0x{db:08x})"
                        );
                    }
                }
            }
        }
    }
}
