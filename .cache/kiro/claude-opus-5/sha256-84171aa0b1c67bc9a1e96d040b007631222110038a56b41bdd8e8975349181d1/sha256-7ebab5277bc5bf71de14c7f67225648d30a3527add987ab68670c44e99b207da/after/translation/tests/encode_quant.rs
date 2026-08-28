//! Differential tests for `encode_quant`: the single public symbol declared by
//! `c_src/include/lib.h`. Both implementations are invoked purely through their
//! shared-object exports.
//!
//! Tests are ordered from the most constrained inputs (the sub-behaviours the
//! function is built out of: the `uni` nibble wrap guard, the `lsbit` fixups,
//! the quantiser step arithmetic, the `d ^ (d >> 31)` pseudo-abs and the
//! `>> 5` tie-break) up to fully random 32-bit inputs.

mod common;

use common::{Pair, Rng};

/// The `uni` values that matter structurally: every 4-bit nibble, plus nibbles
/// carried into higher bits and negative representatives.
fn interesting_uni() -> Vec<i32> {
    let mut v = Vec::new();
    for n in -20..=20 {
        v.push(n);
    }
    for base in [0, 16, 32, 48, 64, 240, 256, -16, -32, -256] {
        for lo in 0..16 {
            v.push(base + lo);
        }
    }
    v.extend_from_slice(&[
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 7,
        i32::MIN + 8,
        i32::MAX,
        i32::MAX - 1,
        i32::MAX - 7,
        i32::MAX - 8,
        -1,
        0,
        1,
    ]);
    v.sort_unstable();
    v.dedup();
    v
}

/// Every `lsbit` selector class: 0 (skip), 4 (the special dither case),
/// odd (force set), even-non-zero (force clear), including negatives.
fn interesting_lsbit() -> Vec<i32> {
    vec![
        -8,
        -5,
        -4,
        -3,
        -2,
        -1,
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        9,
        12,
        16,
        1024,
        i32::MIN,
        i32::MAX,
    ]
}

fn interesting_step() -> Vec<i32> {
    vec![
        i32::MIN,
        i32::MIN + 1,
        -1000000,
        -257,
        -8,
        -7,
        -3,
        -1,
        0,
        1,
        2,
        3,
        7,
        8,
        9,
        15,
        16,
        64,
        255,
        256,
        1000,
        65535,
        65536,
        1 << 20,
        1 << 27,
        i32::MAX / 15,
        i32::MAX,
    ]
}

fn interesting_level() -> Vec<i32> {
    vec![
        i32::MIN,
        i32::MIN + 1,
        -2147483000,
        -100000,
        -32768,
        -1000,
        -2,
        -1,
        0,
        1,
        2,
        1000,
        32767,
        100000,
        2147483000,
        i32::MAX - 1,
        i32::MAX,
    ]
}

#[test]
fn symbol_is_exported_by_both() {
    // Loading succeeds only if both .so files export `encode_quant`.
    let p = Pair::load();
    p.check([0, 0, 0, 0, 0, 0]);
}

/// Level 1: the `uni`/`uni1`/`uni2` nibble-guard and the `lsbit` fixups, with
/// the rest of the arithmetic held at benign values.
#[test]
fn uni_and_lsbit_grid_small_step() {
    let p = Pair::load();
    for &uni in &interesting_uni() {
        for &lsbit in &interesting_lsbit() {
            for &step in &[0, 1, 8, 16, 100] {
                p.check([uni, step, 0, 0, 0, lsbit]);
                p.check([uni, step, 5, 37, -11, lsbit]);
            }
        }
    }
}

/// Level 2: the quantiser `diff = ((2*(uni&7)+1)*step)/8` term, including
/// negative and overflow-prone `step` values.
#[test]
fn step_arithmetic_grid() {
    let p = Pair::load();
    for &uni in &[0i32, 1, 3, 7, 8, 9, 15, -1, -8, 16] {
        for &step in &interesting_step() {
            for &lsbit in &[0i32, 1, 2, 4] {
                p.check([uni, step, 0, 0, 0, lsbit]);
                p.check([uni, step, 1234, -4321, 777, lsbit]);
                p.check([uni, step, i32::MIN, i32::MAX, 0, lsbit]);
            }
        }
    }
}

/// Level 3: the pseudo-abs (`d ^ (d >> 31)`) and the `>> 5` `tgt2` tie-break
/// term across extreme `pred`/`tgt`/`tgt2` magnitudes (wrapping territory).
#[test]
fn distance_and_tiebreak_extremes() {
    let p = Pair::load();
    let levels = interesting_level();
    for &pred in &levels {
        for &tgt in &levels {
            for &tgt2 in &[i32::MIN, -1000, 0, 1000, i32::MAX] {
                for &(uni, step, lsbit) in &[
                    (0i32, 8i32, 0i32),
                    (5, 100, 0),
                    (9, 64, 4),
                    (15, 1, 1),
                    (7, i32::MAX, 2),
                ] {
                    p.check([uni, step, pred, tgt, tgt2, lsbit]);
                }
            }
        }
    }
}

/// Level 4: exhaustive over the whole low byte of `uni` crossed with every
/// `lsbit` class, so each `uni`/`uni1`/`uni2` combination is hit.
#[test]
fn exhaustive_low_byte_uni() {
    let p = Pair::load();
    for uni in -128i32..=255 {
        for lsbit in -2i32..=9 {
            p.check([uni, 24, 13, 91, -57, lsbit]);
            p.check([uni, -24, -13, 91, 57, lsbit]);
        }
    }
}

/// Level 5: fully random 32-bit inputs.
#[test]
fn random_full_range() {
    let p = Pair::load();
    let mut rng = Rng::new(0x5EED_1234_ABCD_0001);
    for _ in 0..200_000 {
        p.check([
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
        ]);
    }
}

/// Level 5b: random but in "realistic codec" ranges, where the tie-break
/// branches (`d1 < d0`, `d2 < d0`) actually fire instead of being swamped by
/// overflow.
#[test]
fn random_realistic_ranges() {
    let p = Pair::load();
    let mut rng = Rng::new(0xC0FF_EE00_1234_5677);
    for _ in 0..300_000 {
        let uni = rng.range(-32, 63);
        let step = rng.range(0, 4096);
        let pred = rng.range(-40000, 40000);
        let tgt = rng.range(-40000, 40000);
        let tgt2 = rng.range(-40000, 40000);
        let lsbit = rng.range(-1, 8);
        p.check([uni, step, pred, tgt, tgt2, lsbit]);
    }
}
