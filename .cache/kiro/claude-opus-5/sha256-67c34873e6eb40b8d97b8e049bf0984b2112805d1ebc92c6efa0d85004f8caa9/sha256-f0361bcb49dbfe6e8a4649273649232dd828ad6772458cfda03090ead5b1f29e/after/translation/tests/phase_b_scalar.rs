//! Phase B — rows C13..C24: `f3` (floored division), `f4` (xorshift128+),
//! `f5` (bit reversal), `f7` (tflac frame-size bound).

mod harness;

use harness::*;

const INT_MIN: i32 = i32::MIN;
const INT_MAX: i32 = i32::MAX;

fn chk_f3(p: &Pair, v1: i32, v2: i32) {
    let (c, r) = unsafe { ((p.c.f3)(v1, v2), (p.r.f3)(v1, v2)) };
    assert_eq!(c, r, "f3({v1}, {v2}) diverged: C={c} Rust={r}");
}

// ------------------------------------------------------------- C13..C16, C17
#[test]
fn c13_c16_f3_all_sign_quadrants() {
    let p = load();
    let mut rng = Rng::new(SEED ^ 13);

    // C13: v1 >= 0, v2 > 0
    for _ in 0..50_000 {
        let v1 = (rng.next_u32() >> 1) as i32;
        let v2 = ((rng.next_u32() >> 1) as i32).max(1);
        chk_f3(&p, v1, v2);
    }
    // C14: v1 >= 0, v2 < 0, v2 != INT_MIN
    for _ in 0..50_000 {
        let v1 = (rng.next_u32() >> 1) as i32;
        let v2 = -(((rng.next_u32() >> 1) as i32).max(1));
        chk_f3(&p, v1, v2);
    }
    // C15: v1 < 0 (not INT_MIN), v2 > 0
    for _ in 0..50_000 {
        let v1 = -(((rng.next_u32() >> 1) as i32).max(1));
        let v2 = ((rng.next_u32() >> 1) as i32).max(1);
        chk_f3(&p, v1, v2);
    }
    // C16: v1 < 0 (not INT_MIN), v2 < 0 (not INT_MIN)
    for _ in 0..50_000 {
        let v1 = -(((rng.next_u32() >> 1) as i32).max(1));
        let v2 = -(((rng.next_u32() >> 1) as i32).max(1));
        chk_f3(&p, v1, v2);
    }
    // Fully unconstrained bit patterns (hits every arm incl. the INT_MIN ones).
    for _ in 0..200_000 {
        chk_f3(&p, rng.next_i32(), rng.next_i32());
    }
}

#[test]
fn c17_f3_exhaustive_small_grid() {
    let p = load();
    for v1 in -40i32..=40 {
        for v2 in -40i32..=40 {
            chk_f3(&p, v1, v2);
        }
    }
    // Extremes crossed with small divisors and vice versa.
    let extremes = [
        INT_MIN,
        INT_MIN + 1,
        -3,
        -2,
        -1,
        0,
        1,
        2,
        3,
        INT_MAX - 1,
        INT_MAX,
        i32::MIN / 2,
        i32::MAX / 2,
    ];
    for &a in &extremes {
        for &b in &extremes {
            chk_f3(&p, a, b);
        }
        for b in -100i32..=100 {
            chk_f3(&p, a, b);
            chk_f3(&p, b, a);
        }
    }
}

// -------------------------------------------------------------------- C18/C19
#[test]
fn c18_f4_single_call_value_and_mutated_state() {
    let p = load();
    let mut rng = Rng::new(SEED ^ 18);
    let mut states: Vec<[u64; 2]> = vec![
        [0, 0],
        [1, 0],
        [0, 1],
        [1, 1],
        [u64::MAX, u64::MAX],
        [u64::MAX, 0],
        [0, u64::MAX],
        [1 << 63, 1],
        [0xdead_beef_dead_beef, 0xcafe_babe_cafe_babe],
    ];
    for _ in 0..50_000 {
        states.push([rng.next_u64(), rng.next_u64()]);
    }
    for st in states {
        let mut sc = CnRnd { state: st };
        let mut sr = CnRnd { state: st };
        let (dc, dr) = unsafe { ((p.c.f4)(&mut sc), (p.r.f4)(&mut sr)) };
        assert_eq!(
            dc.to_bits(),
            dr.to_bits(),
            "f4 value diverged for state {st:?}: C={:#018x} Rust={:#018x}",
            dc.to_bits(),
            dr.to_bits()
        );
        assert_eq!(sc, sr, "f4 mutated state diverged for {st:?}");
    }
}

#[test]
fn c19_f4_thousand_call_chain() {
    let p = load();
    let mut rng = Rng::new(SEED ^ 19);
    for trial in 0..40 {
        let st = if trial == 0 {
            [0u64, 0u64] // the degenerate fixed point
        } else {
            [rng.next_u64(), rng.next_u64()]
        };
        let mut sc = CnRnd { state: st };
        let mut sr = CnRnd { state: st };
        for i in 0..1000 {
            let (dc, dr) = unsafe { ((p.c.f4)(&mut sc), (p.r.f4)(&mut sr)) };
            assert_eq!(
                dc.to_bits(),
                dr.to_bits(),
                "f4 chain diverged at step {i} (seed {st:?})"
            );
            assert_eq!(sc, sr, "f4 chain state diverged at step {i} (seed {st:?})");
        }
    }
}

// ------------------------------------------------------------------------ C20
#[test]
fn c20_f5_bit_reversal() {
    let p = load();
    let mut rng = Rng::new(SEED ^ 20);
    let mut inputs: Vec<u32> = vec![0, 1, 0xFFFF, 0xFFFF_0000, 0xFFFF_FFFF, 0x8000, 0x0001];
    for b in 0..32 {
        inputs.push(1u32 << b);
        inputs.push(!(1u32 << b));
    }
    // The whole low-16 domain, exhaustively, plus a random high half.
    for lo in 0u32..=0xFFFF {
        inputs.push(lo);
    }
    for _ in 0..100_000 {
        inputs.push(rng.next_u32());
    }
    for a in inputs {
        let (c, r) = unsafe { ((p.c.f5)(a), (p.r.f5)(a)) };
        assert_eq!(c, r, "f5({a:#010x}) diverged: C={c:#010x} Rust={r:#010x}");
    }
}

// ------------------------------------------------------------------ C21..C24
fn chk_f7(p: &Pair, bs: u32, ch: u32, bd: u32) {
    let (c, r) = unsafe { ((p.c.f7)(bs, ch, bd), (p.r.f7)(bs, ch, bd)) };
    assert_eq!(
        c, r,
        "f7({bs}, {ch}, {bd}) diverged: C={c:#010x} Rust={r:#010x}"
    );
}

#[test]
fn c21_c24_f7_all_four_flag_combinations() {
    let p = load();
    let mut rng = Rng::new(SEED ^ 21);

    let blocks: &[u32] = &[
        0,
        1,
        2,
        3,
        16,
        4096,
        65535,
        65536,
        0x00FF_FFFF,
        0x7FFF_FFFF,
        0x8000_0000,
        0xFFFF_FFFF,
    ];
    let bitdepths: &[u32] = &[
        0,
        1,
        4,
        8,
        12,
        16,
        20,
        24,
        31,
        32,
        33,
        64,
        0x7FFF_FFFF,
        0xFFFF_FFFF,
    ];
    let channels: &[u32] = &[0, 1, 2, 3, 4, 8, 255, 0x7FFF_FFFF, 0xFFFF_FFFF];

    // C21 (ch==2, bd==32) / C22 (ch==2, bd!=32) / C23 (ch!=2, bd==32) /
    // C24 (ch!=2, bd!=32) — the full cross product covers all four.
    for &bs in blocks {
        for &bd in bitdepths {
            for &ch in channels {
                chk_f7(&p, bs, ch, bd);
            }
        }
    }
    // C21/C22 focused: channels pinned to 2, bitdepth on/off 32.
    for _ in 0..50_000 {
        chk_f7(&p, rng.next_u32(), 2, 32);
        chk_f7(&p, rng.next_u32(), 2, rng.next_u32());
        chk_f7(&p, rng.next_u32(), rng.next_u32() | 1, 32); // odd => never 2
    }
    // C24 wide random sweep (wrapping overflow territory).
    for _ in 0..200_000 {
        chk_f7(&p, rng.next_u32(), rng.next_u32(), rng.next_u32());
    }
    // Small dense grid around every branch boundary.
    for bs in 0u32..40 {
        for ch in 0u32..6 {
            for bd in 28u32..36 {
                chk_f7(&p, bs, ch, bd);
            }
        }
    }
}
