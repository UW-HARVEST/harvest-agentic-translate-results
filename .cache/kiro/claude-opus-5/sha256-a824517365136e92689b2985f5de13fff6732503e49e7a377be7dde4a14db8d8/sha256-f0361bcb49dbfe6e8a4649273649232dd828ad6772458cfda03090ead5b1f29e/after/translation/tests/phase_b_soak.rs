//! Phase B soak: a large uniform random differential run over the whole
//! `i32^4` domain, plus a stratified sweep that guarantees coverage of every
//! `(sign pattern) x (float-window state) x (low-byte class)` cell.

mod common;

use common::{Pair, Rng, BITS_1000_0, BITS_1_0};

#[test]
fn soak_uniform_random() {
    let p = Pair::load();
    let mut r = Rng::new(common::FIXED_SEED ^ 0x50AC);
    for _ in 0..1_000_000 {
        let (a, b, c, d) = (r.next_i32(), r.next_i32(), r.next_i32(), r.next_i32());
        let gc = p.c(a, b, c, d);
        let gr = p.rust(a, b, c, d);
        if gc != gr {
            panic!("soak divergence at memchra2({a}, {b}, {c}, {d}): C = {gc}, Rust = {gr}");
        }
    }
}

/// Stratified: every combination of
///   - 16 sign patterns of (a, b, c, d)
///   - 4 float-window states of `a`
///   - 4 low-byte classes of (b, c, d)
/// is visited with 40 random tuples each (16 * 4 * 4 * 40 = 10 240 calls).
#[test]
fn soak_stratified_cells() {
    let p = Pair::load();
    let mut r = Rng::new(common::FIXED_SEED ^ 0x57A7);

    for sign_mask in 0u32..16 {
        for fstate in 0u32..4 {
            for byte_class in 0u32..4 {
                for _ in 0..40 {
                    // `a` from its float-window state, then sign-adjusted.
                    let a_bits = match fstate {
                        0 => r.range_u32(0x8000_0000, u32::MAX), // negative float bits
                        1 => r.range_u32(1, BITS_1_0 - 1),       // 0 < f < 1
                        2 => r.range_u32(BITS_1_0, BITS_1000_0 - 1), // 1 <= f < 1000
                        _ => r.range_u32(BITS_1000_0, 0x7FFF_FFFF), // f >= 1000
                    };
                    let mut a = a_bits as i32;
                    if sign_mask & 1 != 0 {
                        a = a.wrapping_neg();
                    }

                    let low = match byte_class {
                        0 => 0x00u32,
                        1 => 0xFFu32,
                        2 => 0x80u32,
                        _ => r.next_u32() & 0xFF,
                    };
                    let mk = |r: &mut Rng, neg: bool| {
                        let v = ((r.next_u32() & 0x7FFF_FF00) | low) as i32;
                        if neg {
                            v.wrapping_neg()
                        } else {
                            v
                        }
                    };
                    let b = mk(&mut r, sign_mask & 2 != 0);
                    let c = mk(&mut r, sign_mask & 4 != 0);
                    let d = mk(&mut r, sign_mask & 8 != 0);

                    let gc = p.c(a, b, c, d);
                    let gr = p.rust(a, b, c, d);
                    assert_eq!(
                        gc, gr,
                        "stratified divergence [sign={sign_mask} float={fstate} \
                         byte={byte_class}] memchra2({a}, {b}, {c}, {d}): \
                         C = {gc}, Rust = {gr}"
                    );
                }
            }
        }
    }
}

/// Exhaustive over a dense contiguous block: catches value-dependent bugs that
/// uniform sampling of a 2^128 domain would miss.
#[test]
fn soak_exhaustive_dense_block() {
    let p = Pair::load();
    // (a, b) exhaustive over -60..=60 with c, d pinned to two shapes.
    for a in -60i32..=60 {
        for b in -60i32..=60 {
            for &(c, d) in &[(0i32, 0i32), (-1, 1)] {
                let gc = p.c(a, b, c, d);
                let gr = p.rust(a, b, c, d);
                assert_eq!(
                    gc, gr,
                    "dense-block divergence at memchra2({a}, {b}, {c}, {d}): \
                     C = {gc}, Rust = {gr}"
                );
            }
        }
    }
    // And a dense sweep of `a` through the float window's integral steps.
    for k in 0..1000u32 {
        let a = (k as f32).to_bits() as i32;
        for &(b, c, d) in &[(0, 0, 0), (7, -8, 9)] {
            let gc = p.c(a, b, c, d);
            let gr = p.rust(a, b, c, d);
            assert_eq!(gc, gr, "float-step divergence at a = {a} (f = {k})");
        }
    }
}
