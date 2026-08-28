//! Phase B — valid-path differential tests for the integer / bit-twiddling
//! layer: `f3` (floored division), `f4` (xorshift128+ PRNG), `f5` (bit
//! reversal) and `f7` (tflac frame-size estimate).
//!
//! Covers `CONFIGS.md` rows C15 … C29.

mod common;

use common::*;

const N: usize = 40_000;

// ---------------------------------------------------------------------------
// C15 … C19 — f3 across every sign / overflow quadrant
// ---------------------------------------------------------------------------

fn chk_f3(p: &Pair, v1: i32, v2: i32, tag: &str) {
    same(
        tag,
        (v1, v2),
        unsafe { (p.c.f3)(v1, v2) },
        unsafe { (p.rs.f3)(v1, v2) },
    );
}

#[test]
fn c15_f3_pos_pos() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x15);
    // the `v1 >= 0 && v2 >= 0` early-return path
    for _ in 0..N {
        let v1 = r.next_u32() as i32 & i32::MAX;
        let v2 = (r.next_u32() as i32 & i32::MAX).max(1);
        chk_f3(p, v1, v2, "f3(+,+)");
    }
    for &a in SPECIAL_I32 {
        for &b in SPECIAL_I32 {
            if a >= 0 && b >= 0 {
                chk_f3(p, a, b, "f3(+,+)/special");
            }
        }
    }
}

#[test]
fn c16_f3_pos_neg() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x16);
    for _ in 0..N {
        let v1 = r.next_u32() as i32 & i32::MAX;
        let v2 = -((r.next_u32() as i32 & i32::MAX).max(1));
        chk_f3(p, v1, v2, "f3(+,-)");
    }
    for &a in SPECIAL_I32 {
        for &b in SPECIAL_I32 {
            if a >= 0 && b < 0 {
                chk_f3(p, a, b, "f3(+,-)/special");
            }
        }
    }
}

#[test]
fn c17_f3_neg_pos() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x17);
    for _ in 0..N {
        let v1 = -((r.next_u32() as i32 & i32::MAX).max(1));
        let v2 = (r.next_u32() as i32 & i32::MAX).max(1);
        chk_f3(p, v1, v2, "f3(-,+)");
    }
    for &a in SPECIAL_I32 {
        for &b in SPECIAL_I32 {
            if a < 0 && b > 0 {
                chk_f3(p, a, b, "f3(-,+)/special");
            }
        }
    }
}

#[test]
fn c18_f3_neg_neg() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x18);
    for _ in 0..N {
        let v1 = -((r.next_u32() as i32 & i32::MAX).max(1));
        let v2 = -((r.next_u32() as i32 & i32::MAX).max(1));
        chk_f3(p, v1, v2, "f3(-,-)");
    }
    for &a in SPECIAL_I32 {
        for &b in SPECIAL_I32 {
            if a < 0 && b < 0 {
                chk_f3(p, a, b, "f3(-,-)/special");
            }
        }
    }
}

#[test]
fn c19_f3_fully_random() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x19);
    for _ in 0..N {
        chk_f3(p, r.edgy_i32(), r.edgy_i32(), "f3/edgy");
    }
    for _ in 0..N {
        chk_f3(p, r.next_i32(), r.next_i32(), "f3/uniform");
    }
    // small divisors / dividends are where the fix-up path is most active
    for _ in 0..N {
        let v1 = r.next_i32() % 1000;
        let v2 = r.next_i32() % 17;
        chk_f3(p, v1, v2, "f3/small");
    }
    // full cross product of the boundary corpus
    for &a in SPECIAL_I32 {
        for &b in SPECIAL_I32 {
            chk_f3(p, a, b, "f3/cross");
        }
    }
}

#[test]
fn c20_f3_exhaustive_small_grid() {
    let p = pair();
    // every pair in [-40, 40]^2 — 6561 combinations, covers all sign
    // combinations and every remainder fix-up
    for v1 in -40..=40i32 {
        for v2 in -40..=40i32 {
            chk_f3(p, v1, v2, "f3/grid");
        }
    }
    // and a band right next to the overflow boundaries
    for d1 in 0..8i32 {
        for d2 in 0..8i32 {
            for (v1, v2) in [
                (i32::MIN + d1, i32::MIN + d2),
                (i32::MIN + d1, i32::MAX - d2),
                (i32::MAX - d1, i32::MIN + d2),
                (i32::MAX - d1, i32::MAX - d2),
                (i32::MIN + d1, d2 - 4),
                (d1 - 4, i32::MIN + d2),
                (i32::MAX - d1, d2 - 4),
                (d1 - 4, i32::MAX - d2),
            ] {
                chk_f3(p, v1, v2, "f3/boundary-band");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C21 … C23 — f4
// ---------------------------------------------------------------------------

fn chk_f4_once(p: &Pair, state: [u64; 2]) {
    let mut sc = CnRnd { state };
    let mut sr = CnRnd { state };
    let cv = unsafe { (p.c.f4)(&mut sc) };
    let rv = unsafe { (p.rs.f4)(&mut sr) };
    same("f4/value", state, cv, rv);
    same("f4/state", state, sc.state, sr.state);
}

#[test]
fn c21_f4_single_call() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x21);
    for _ in 0..N {
        chk_f4_once(p, [r.next_u64(), r.next_u64()]);
    }
    for _ in 0..N {
        chk_f4_once(p, [r.edgy_u64(), r.edgy_u64()]);
    }
}

#[test]
fn c22_f4_iterated_sequence() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x22);
    // The real consumer pattern: seed once, then draw a long stream. A single
    // call cannot detect a state-update bug that only shows up after mixing.
    for _ in 0..400 {
        let seed = [r.next_u64(), r.next_u64()];
        let mut sc = CnRnd { state: seed };
        let mut sr = CnRnd { state: seed };
        for i in 0..256 {
            let cv = unsafe { (p.c.f4)(&mut sc) };
            let rv = unsafe { (p.rs.f4)(&mut sr) };
            same("f4/stream", (seed, i), cv, rv);
            same("f4/stream-state", (seed, i), sc.state, sr.state);
            // the C is documented to return [0,1); confirm the invariant holds
            // for both, which also guards against a mis-shifted mantissa
            assert!(
                (0.0..1.0).contains(&cv),
                "C f4 out of [0,1): {cv} at step {i} seed {seed:?}"
            );
        }
    }
}

#[test]
fn c23_f4_degenerate_and_single_bit_states() {
    let p = pair();
    // all-zero state: cn_rnd_next returns 0 forever
    let mut sc = CnRnd { state: [0, 0] };
    let mut sr = CnRnd { state: [0, 0] };
    for i in 0..16 {
        let cv = unsafe { (p.c.f4)(&mut sc) };
        let rv = unsafe { (p.rs.f4)(&mut sr) };
        same("f4/zero-state", i, cv, rv);
        same("f4/zero-state-arr", i, sc.state, sr.state);
    }
    // every single-bit state, in both slots
    for bit in 0..64 {
        chk_f4_once(p, [1u64 << bit, 0]);
        chk_f4_once(p, [0, 1u64 << bit]);
        chk_f4_once(p, [1u64 << bit, 1u64 << bit]);
        chk_f4_once(p, [!(1u64 << bit), u64::MAX]);
    }
    // wrap-around of `x + y`
    for &a in SPECIAL_U64 {
        for &b in SPECIAL_U64 {
            chk_f4_once(p, [a, b]);
        }
    }
    // states chosen so that value >> 12 == 0 (mantissa 0 -> exactly 0.0)
    chk_f4_once(p, [0, 0]);
}

// ---------------------------------------------------------------------------
// C24 / C25 — f5
// ---------------------------------------------------------------------------

#[test]
fn c24_f5_exhaustive_low_16_bits() {
    let p = pair();
    for a in 0u32..=0xFFFF {
        same("f5/exhaustive", a, unsafe { (p.c.f5)(a) }, unsafe {
            (p.rs.f5)(a)
        });
    }
}

#[test]
fn c25_f5_high_bits_set() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x25);
    for bit in 16..32 {
        let a = 1u32 << bit;
        same("f5/highbit", a, unsafe { (p.c.f5)(a) }, unsafe {
            (p.rs.f5)(a)
        });
        let a = (1u32 << bit) | 0xFFFF;
        same("f5/highbit|low", a, unsafe { (p.c.f5)(a) }, unsafe {
            (p.rs.f5)(a)
        });
    }
    for &a in SPECIAL_U32 {
        same("f5/special", a, unsafe { (p.c.f5)(a) }, unsafe {
            (p.rs.f5)(a)
        });
    }
    for _ in 0..N {
        let a = r.next_u32();
        same("f5/random", a, unsafe { (p.c.f5)(a) }, unsafe {
            (p.rs.f5)(a)
        });
    }
    // high bits must be discarded: f5(a) == f5(a & 0xFFFF) in both impls
    for _ in 0..10_000 {
        let a = r.next_u32();
        let lo = a & 0xFFFF;
        assert_eq!(unsafe { (p.c.f5)(a) }, unsafe { (p.c.f5)(lo) });
        assert_eq!(unsafe { (p.rs.f5)(a) }, unsafe { (p.rs.f5)(lo) });
    }
}

// ---------------------------------------------------------------------------
// C26 … C29 — f7
// ---------------------------------------------------------------------------

fn chk_f7(p: &Pair, bs: u32, ch: u32, bd: u32, tag: &str) {
    same(
        tag,
        (bs, ch, bd),
        unsafe { (p.c.f7)(bs, ch, bd) },
        unsafe { (p.rs.f7)(bs, ch, bd) },
    );
}

#[test]
fn c26_f7_channels_not_2() {
    let p = pair();
    for &ch in &[0u32, 1, 3, 4, 5, 6, 7, 8, 255, 65535] {
        for &bd in &[8u32, 12, 16, 20, 24, 31, 32, 33, 64] {
            for &bs in &[0u32, 1, 2, 16, 192, 4096, 32768, 65535, 65536] {
                chk_f7(p, bs, ch, bd, "f7(ch!=2)");
            }
        }
    }
}

#[test]
fn c27_f7_channels_2_depth_not_32() {
    let p = pair();
    for &bd in &[0u32, 4, 8, 12, 16, 20, 24, 31, 33, 64] {
        for &bs in &[0u32, 1, 2, 16, 192, 4096, 32768, 65535, 65536] {
            chk_f7(p, bs, 2, bd, "f7(ch==2,bd!=32)");
        }
    }
}

#[test]
fn c28_f7_channels_2_depth_32() {
    let p = pair();
    for &bs in &[0u32, 1, 2, 16, 192, 4096, 32768, 65535, 65536, u32::MAX] {
        chk_f7(p, bs, 2, 32, "f7(ch==2,bd==32)");
    }
    // and the neighbours of 32 so the `!= 32` predicate is pinned down
    for &bd in &[30u32, 31, 32, 33, 34] {
        for &bs in &[1u32, 4096, 65535] {
            chk_f7(p, bs, 2, bd, "f7(bd near 32)");
        }
    }
}

#[test]
fn c29_f7_overflow_and_random() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x29);
    // full cross product of the boundary corpus (25^3 = 15625)
    for &bs in SPECIAL_U32 {
        for &ch in SPECIAL_U32 {
            for &bd in SPECIAL_U32 {
                chk_f7(p, bs, ch, bd, "f7/cross");
            }
        }
    }
    for _ in 0..N {
        chk_f7(p, r.next_u32(), r.next_u32(), r.next_u32(), "f7/random");
    }
    for _ in 0..N {
        chk_f7(p, r.edgy_u32(), r.edgy_u32(), r.edgy_u32(), "f7/edgy");
    }
    // channels near 2 with wrapping-prone magnitudes
    for _ in 0..N {
        let ch = r.below(6);
        chk_f7(p, r.next_u32(), ch, r.next_u32(), "f7/ch-small");
    }
}
