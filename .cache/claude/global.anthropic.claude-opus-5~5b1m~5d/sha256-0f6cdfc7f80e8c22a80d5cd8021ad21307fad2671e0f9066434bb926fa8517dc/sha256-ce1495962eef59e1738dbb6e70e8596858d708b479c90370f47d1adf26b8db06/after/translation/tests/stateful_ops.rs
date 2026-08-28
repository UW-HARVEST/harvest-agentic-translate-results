//! Phase B + Phase C for the four `operations[]` table members, called DIRECTLY
//! through their `.so` exports rather than only through `findrep`.
//!
//! These are the lowest-level entry points and they all mutate the hidden statics,
//! so every test carries state across many calls — a single call per function would
//! never reach the wrap-around and sign-change behaviour that `findrep` branches on.
//!
//! Covers CONFIGS.md rows 8–19 and ERRORS.md rows 1, 2, 4, 31.
//!
//! `INT_MIN / -1` is deliberately avoided here: it is fatal in BOTH libraries and is
//! verified separately in `crash.rs` (ERRORS #3). The helper tracks `multiplier` from
//! the returned values so it can steer clear of that operand pair.

mod common;
use common::*;

use std::ffi::c_int;

/// Mirror of the library's hidden state, reconstructed from return values so tests
/// can avoid the fatal divisor without ever reading the statics directly.
#[derive(Debug, Clone, Copy)]
struct Shadow {
    acc: c_int,
    mult: c_int,
}

impl Default for Shadow {
    fn default() -> Self {
        // static int accumulator = 0; static int multiplier = 1;
        Shadow { acc: 0, mult: 1 }
    }
}

/// Call one op on BOTH libraries, assert the returned values agree, and update the
/// shadow state. Returns the agreed value.
#[track_caller]
fn both(p: &Pair, sh: &mut Shadow, idx: usize, a: c_int, b: c_int) -> c_int {
    let (c, r) = unsafe { (p.c.op(idx, a, b), p.r.op(idx, a, b)) };
    const NAMES: [&str; 4] = [
        "add_to_accumulator",
        "multiply_with_multiplier",
        "subtract_from_accumulator",
        "divide_multiplier",
    ];
    assert_eq!(
        c, r,
        "{}({a}, {b}) diverged: C={c} Rust={r} (state before: {sh:?})",
        NAMES[idx]
    );
    match idx {
        0 | 2 => sh.acc = c,
        1 | 3 => sh.mult = c,
        _ => unreachable!(),
    }
    c
}

/// True when `multiplier / b` would raise SIGFPE in the C (ERRORS #3).
fn divide_would_trap(sh: &Shadow, b: c_int) -> bool {
    b != 0 && sh.mult == c_int::MIN && b == -1
}

// ===========================================================================
// add_to_accumulator  (accumulator += a + b)
// ===========================================================================

/// CONFIGS #8 — 500 randomized calls with state carried between them.
#[test]
fn cfg08_add_random_sequence_state_carried() {
    let p = fresh_pair();
    let mut sh = Shadow::default();
    let mut rng = Rng::new(0x0801);
    for _ in 0..500 {
        both(&p, &mut sh, 0, rng.interesting_i32(), rng.interesting_i32());
    }
    // Independent recomputation of the C's semantics.
    let mut rng2 = Rng::new(0x0801);
    let mut expect = 0i32;
    for _ in 0..500 {
        let (a, b) = (rng2.interesting_i32(), rng2.interesting_i32());
        expect = expect.wrapping_add(a.wrapping_add(b));
    }
    assert_eq!(sh.acc, expect, "accumulator drifted from the C semantics");
}

/// CONFIGS #9 / ERRORS #31 — signed overflow in `a + b` and in `accumulator += ..`.
#[test]
fn cfg09_add_overflow_shapes() {
    let p = fresh_pair();
    let mut sh = Shadow::default();
    for (a, b) in [
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN),
        (i32::MAX, 1),
        (i32::MIN, -1),
        (i32::MAX, i32::MIN),
        (1, i32::MAX),
        (-1, i32::MIN),
    ] {
        both(&p, &mut sh, 0, a, b);
    }
    // Repeatedly push the accumulator past INT_MAX.
    for _ in 0..64 {
        both(&p, &mut sh, 0, i32::MAX, i32::MAX);
    }
    for _ in 0..64 {
        both(&p, &mut sh, 0, i32::MIN, i32::MIN);
    }
}

// ===========================================================================
// multiply_with_multiplier  (multiplier *= a * b)
// ===========================================================================

/// CONFIGS #10 — 500 randomized calls; `multiplier` wraps almost immediately.
#[test]
fn cfg10_multiply_random_sequence_state_carried() {
    let p = fresh_pair();
    let mut sh = Shadow::default();
    let mut rng = Rng::new(0x1001);
    for _ in 0..500 {
        both(&p, &mut sh, 1, rng.interesting_i32(), rng.interesting_i32());
    }
}

/// CONFIGS #11 — once `a * b == 0`, `multiplier` is stuck at 0 forever, which kills
/// the `both_active` and `multiplier > 0100` branches in `findrep`.
#[test]
fn cfg11_multiply_by_zero_latches_multiplier_at_zero() {
    let p = fresh_pair();
    let mut sh = Shadow::default();
    assert_eq!(both(&p, &mut sh, 1, 7, 11), 77);
    assert_eq!(both(&p, &mut sh, 1, 0, 12345), 0);
    // Nothing can revive it.
    let mut rng = Rng::new(0x1101);
    for _ in 0..200 {
        let v = both(&p, &mut sh, 1, rng.interesting_i32(), rng.interesting_i32());
        assert_eq!(v, 0, "multiplier must stay latched at 0");
    }
    // 0 * anything == 0 also latches via the second operand.
    let p2 = fresh_pair();
    let mut sh2 = Shadow::default();
    assert_eq!(both(&p2, &mut sh2, 1, 999, 0), 0);
}

/// CONFIGS #12 / ERRORS #31 — multiplication overflow shapes.
#[test]
fn cfg12_multiply_overflow_shapes() {
    let p = fresh_pair();
    for (a, b) in [
        (i32::MIN, 1),
        (1, i32::MIN),
        (-1, i32::MIN),
        (i32::MIN, -1),
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN),
        (65536, 65536),
        (46341, 46341),
        (-46341, 46341),
    ] {
        let mut sh = Shadow::default();
        let fresh = fresh_pair();
        both(&fresh, &mut sh, 1, a, b);
        // And again on a dirty multiplier.
        both(&fresh, &mut sh, 1, a, b);
        both(&fresh, &mut sh, 1, 3, 5);
    }
    // Long chain of squarings on one instance.
    let mut sh = Shadow::default();
    for _ in 0..64 {
        both(&p, &mut sh, 1, 3, 7);
    }
}

// ===========================================================================
// subtract_from_accumulator  (accumulator -= a - b)
// ===========================================================================

/// CONFIGS #13 — 500 randomized calls, state carried.
#[test]
fn cfg13_subtract_random_sequence_state_carried() {
    let p = fresh_pair();
    let mut sh = Shadow::default();
    let mut rng = Rng::new(0x1301);
    for _ in 0..500 {
        both(&p, &mut sh, 2, rng.interesting_i32(), rng.interesting_i32());
    }
    let mut expect = 0i32;
    let mut rng2 = Rng::new(0x1301);
    for _ in 0..500 {
        let (a, b) = (rng2.interesting_i32(), rng2.interesting_i32());
        expect = expect.wrapping_sub(a.wrapping_sub(b));
    }
    assert_eq!(sh.acc, expect, "accumulator drifted from the C semantics");
}

/// CONFIGS #14 / ERRORS #31 — `a - b` overflow shapes.
#[test]
fn cfg14_subtract_overflow_shapes() {
    let p = fresh_pair();
    let mut sh = Shadow::default();
    for (a, b) in [
        (i32::MIN, i32::MAX),
        (i32::MAX, i32::MIN),
        (i32::MIN, 1),
        (i32::MAX, -1),
        (0, i32::MIN),
        (i32::MIN, 0),
    ] {
        both(&p, &mut sh, 2, a, b);
    }
    for _ in 0..64 {
        both(&p, &mut sh, 2, i32::MIN, i32::MAX);
    }
}

// ===========================================================================
// divide_multiplier  (if (b != 0) multiplier /= b)
// ===========================================================================

/// CONFIGS #15 — positive multiplier, `b != 0`: truncation toward zero.
#[test]
fn cfg15_divide_positive_truncates_toward_zero() {
    let p = fresh_pair();
    let mut sh = Shadow::default();
    both(&p, &mut sh, 1, 1_000_000, 1); // multiplier = 1_000_000
    assert_eq!(both(&p, &mut sh, 3, 0, 3), 333_333);
    assert_eq!(both(&p, &mut sh, 3, 0, 7), 47_619);
    let mut rng = Rng::new(0x1501);
    for _ in 0..500 {
        let b = loop {
            let b = rng.range_i32(1, i32::MAX);
            if b != 0 {
                break b;
            }
        };
        both(&p, &mut sh, 3, rng.next_i32(), b);
        // Re-seed the multiplier so it does not collapse to 0 permanently.
        both(&p, &mut sh, 1, rng.range_i32(1, 1 << 20), 1);
    }
}

/// CONFIGS #16 / ERRORS #4 — NEGATIVE dividend must truncate toward zero (C99),
/// not floor. `-7 / 2 == -3`, not `-4`.
#[test]
fn cfg16_divide_negative_truncates_toward_zero_not_floor() {
    let p = fresh_pair();
    let mut sh = Shadow::default();
    both(&p, &mut sh, 1, -7, 1); // multiplier = -7
    assert_eq!(
        both(&p, &mut sh, 3, 0, 2),
        -3,
        "C truncates toward zero: -7 / 2 == -3"
    );
    let p2 = fresh_pair();
    let mut sh2 = Shadow::default();
    both(&p2, &mut sh2, 1, -1_000_001, 1);
    assert_eq!(both(&p2, &mut sh2, 3, 0, 3), -333_333);
    assert_eq!(both(&p2, &mut sh2, 3, 0, 100), -3_333);
}

/// CONFIGS #17 — sign-mixed dividend/divisor, randomized.
#[test]
fn cfg17_divide_sign_mixed() {
    let p = fresh_pair();
    let mut rng = Rng::new(0x1701);
    for _ in 0..2000 {
        let fresh = fresh_pair();
        let mut sh = Shadow::default();
        // Seed an arbitrary multiplier (positive or negative).
        both(&fresh, &mut sh, 1, rng.next_i32(), 1);
        let b = {
            let mut b = rng.interesting_i32();
            if divide_would_trap(&sh, b) {
                b = -2; // steer away from the fatal INT_MIN / -1 (see crash.rs)
            }
            b
        };
        both(&fresh, &mut sh, 3, rng.next_i32(), b);
    }
    let _ = &p;
}

/// CONFIGS #18 / ERRORS #1 + #2 — the `b == 0` guard: no division happens, the
/// multiplier is returned unchanged, but `operation_count` is STILL incremented.
#[test]
fn cfg18_divide_by_zero_guard() {
    let p = fresh_pair();
    let mut sh = Shadow::default();
    both(&p, &mut sh, 1, 12345, 1);
    let before = sh.mult;
    for _ in 0..10 {
        assert_eq!(
            both(&p, &mut sh, 3, 0, 0),
            before,
            "b == 0 must leave the multiplier untouched"
        );
    }
    // ERRORS #2 — same with a negative multiplier.
    let p2 = fresh_pair();
    let mut sh2 = Shadow::default();
    both(&p2, &mut sh2, 1, -98765, 1);
    let before2 = sh2.mult;
    assert_eq!(both(&p2, &mut sh2, 3, 0, 0), before2);
    // Even at INT_MIN, b == 0 is guarded and must NOT trap.
    let p3 = fresh_pair();
    let mut sh3 = Shadow::default();
    both(&p3, &mut sh3, 1, i32::MIN, 1);
    assert_eq!(both(&p3, &mut sh3, 3, 0, 0), i32::MIN);
    // `operation_count` was bumped 1 (multiply) + 1 (guarded divide) = 2 times;
    // observe it through findrep, which adds `operation_count * 010`.
    let a = unsafe { (p3.c.findrep)(0, 0, 0, 0) };
    let b = unsafe { (p3.r.findrep)(0, 0, 0, 0) };
    assert_eq!(a, b, "operation_count diverged after a guarded divide");
}

/// CONFIGS #19 — multiply and divide interleaved so the multiplier repeatedly
/// changes sign and magnitude, 5 000 steps on ONE library instance.
#[test]
fn cfg19_divide_interleaved_with_multiply() {
    let p = fresh_pair();
    let mut sh = Shadow::default();
    let mut rng = Rng::new(0x1901);
    for _ in 0..5000 {
        if rng.below(2) == 0 {
            both(&p, &mut sh, 1, rng.interesting_i32(), rng.interesting_i32());
        } else {
            let mut b = rng.interesting_i32();
            if divide_would_trap(&sh, b) {
                b = 3;
            }
            both(&p, &mut sh, 3, rng.next_i32(), b);
        }
    }
}

/// `divide_multiplier`'s first parameter is UNUSED by the C — only `b` matters.
/// Verify both libraries ignore it identically.
#[test]
fn divide_first_argument_is_ignored() {
    let mut rng = Rng::new(0x1A01);
    for _ in 0..300 {
        let a1 = rng.next_i32();
        let a2 = rng.next_i32();
        let seed = rng.range_i32(1, 1 << 24);
        let b = rng.range_i32(1, 1000);

        let p1 = fresh_pair();
        let mut s1 = Shadow::default();
        both(&p1, &mut s1, 1, seed, 1);
        let r1 = both(&p1, &mut s1, 3, a1, b);

        let p2 = fresh_pair();
        let mut s2 = Shadow::default();
        both(&p2, &mut s2, 1, seed, 1);
        let r2 = both(&p2, &mut s2, 3, a2, b);

        assert_eq!(r1, r2, "first argument of divide_multiplier must be ignored");
    }
}

/// CONFIGS #49 — randomized interleaving of ALL FOUR stateful ops on one instance.
#[test]
fn cfg49_all_ops_interleaved_fuzz() {
    let p = fresh_pair();
    let mut sh = Shadow::default();
    let mut rng = Rng::new(0xA11_0F5);
    for _ in 0..20_000 {
        let idx = rng.below(4) as usize;
        let a = rng.interesting_i32();
        let mut b = rng.interesting_i32();
        if idx == 3 && divide_would_trap(&sh, b) {
            b = 5;
        }
        both(&p, &mut sh, idx, a, b);
    }
    // Finally fold the accumulated state through findrep, which reads all three
    // statics — this catches drift that the per-call return values hid.
    let c = unsafe { (p.c.findrep)(1, 2, 3, 4) };
    let r = unsafe { (p.r.findrep)(1, 2, 3, 4) };
    assert_eq!(c, r, "findrep diverged after 20k interleaved ops");
}
