// Exhaustive NaN / infinity propagation parity for `calculate_subtree_sum`
// (deepens row C27 of CONFIGS.md).
//
// IEEE-754 does not specify WHICH NaN a `+` returns when both operands are NaN,
// and `fadd` is commutative so LLVM reorders the operands freely. The C's
// `addsd %xmm1,%xmm0` puts the recursive result in the destination register, and
// x86 returns the destination's NaN (quieted). This file pins that down over the
// full cross-product of interesting bit patterns and several tree shapes.

mod common;
use common::*;

/// Every "interesting" double: both infinities, quiet and signalling NaNs of
/// both signs, NaNs with payloads, zeroes of both signs, and finite values.
const POOL: [u64; 16] = [
    0x7FF0_0000_0000_0000, // +inf
    0xFFF0_0000_0000_0000, // -inf
    0x7FF8_0000_0000_0000, // +qNaN (default)
    0xFFF8_0000_0000_0000, // -qNaN (x86 "indefinite")
    0x7FF0_0000_0000_0001, // +sNaN, min payload
    0xFFF0_0000_0000_0001, // -sNaN, min payload
    0x7FF8_0000_DEAD_BEEF, // +qNaN, custom payload
    0xFFF8_0000_DEAD_BEEF, // -qNaN, custom payload
    0x7FF4_2424_2424_2424, // +sNaN, custom payload
    0xFFFF_FFFF_FFFF_FFFF, // -qNaN, all-ones payload
    0x0000_0000_0000_0000, // +0.0
    0x8000_0000_0000_0000, // -0.0
    0x3FF0_0000_0000_0000, // 1.0
    0xBFF0_0000_0000_0000, // -1.0
    0x7FEF_FFFF_FFFF_FFFF, // f64::MAX (so MAX+MAX overflows to +inf)
    0xFFEF_FFFF_FFFF_FFFF, // f64::MIN
];

fn pick(combo: usize, slot: usize) -> f64 {
    f64::from_bits(POOL[(combo / POOL.len().pow(slot as u32)) % POOL.len()])
}

/// Shape: root with `k` direct children. Exhaustive over POOL^(k+1).
#[test]
fn nan_parity_flat_fanout_exhaustive() {
    for k in 0..=2usize {
        let n = k + 1;
        let total = POOL.len().pow(n as u32);
        for combo in 0..total {
            let vals: Vec<f64> = (0..n).map(|s| pick(combo, s)).collect();
            let p = fresh_pair();
            let build = |l: &Lib| {
                l.add_node(1, -1, b"r", vals[0]);
                for j in 0..k {
                    l.add_node(2 + j as i32, 1, b"c", vals[j + 1]);
                }
            };
            build(&p.c);
            build(&p.r);
            for id in [1i32, 2, 3, 4] {
                eq_bits(
                    "C27/flat",
                    (k, combo, vals.iter().map(|v| v.to_bits()).collect::<Vec<_>>()),
                    p.c.subtree_bits(id),
                    p.r.subtree_bits(id),
                );
            }
        }
    }
}

/// Shape: a 3-level tree root(1) -> {2,3}, 2 -> {4}. Exhaustive over POOL^4
/// would be 65 536 loads; sample deterministically instead but cover every value
/// in every slot at least once.
#[test]
fn nan_parity_multilevel() {
    for combo in 0..POOL.len().pow(4) {
        // stride so every slot sees every pool value, without 65k library loads
        if combo % 7 != 0 {
            continue;
        }
        let vals: Vec<f64> = (0..4).map(|s| pick(combo, s)).collect();
        let p = fresh_pair();
        let build = |l: &Lib| {
            l.add_node(1, -1, b"root", vals[0]);
            l.add_node(2, 1, b"a", vals[1]);
            l.add_node(3, 1, b"b", vals[2]);
            l.add_node(4, 2, b"aa", vals[3]);
        };
        build(&p.c);
        build(&p.r);
        for id in [1i32, 2, 3, 4, 5] {
            eq_bits(
                "C27/multilevel",
                (combo, id, vals.iter().map(|v| v.to_bits()).collect::<Vec<_>>()),
                p.c.subtree_bits(id),
                p.r.subtree_bits(id),
            );
        }
    }
}

/// Shape: deep chain, so NaNs must survive many nested accumulations.
#[test]
fn nan_parity_deep_chain() {
    let mut rng = Rng::new(0xC27_C4A1);
    for _ in 0..400 {
        let depth = 1 + rng.below(30) as usize;
        let vals: Vec<f64> = (0..depth)
            .map(|_| f64::from_bits(POOL[rng.below(POOL.len() as u64) as usize]))
            .collect();
        let p = fresh_pair();
        let build = |l: &Lib| {
            for k in 0..depth {
                let parent = if k == 0 { -1 } else { k as i32 };
                l.add_node(k as i32 + 1, parent, b"n", vals[k]);
            }
        };
        build(&p.c);
        build(&p.r);
        for id in 0..=(depth as i32 + 1) {
            eq_bits(
                "C27/chain",
                (depth, id),
                p.c.subtree_bits(id),
                p.r.subtree_bits(id),
            );
        }
    }
}

/// Wide fan-out with many NaNs, so the accumulator meets NaN repeatedly and the
/// "last child wins" consequence of the destination-operand rule is exercised.
#[test]
fn nan_parity_wide_fanout() {
    let mut rng = Rng::new(0xC27_D1DE);
    for _ in 0..400 {
        let k = 1 + rng.below(90) as usize;
        let vals: Vec<f64> = (0..=k)
            .map(|_| f64::from_bits(POOL[rng.below(POOL.len() as u64) as usize]))
            .collect();
        let p = fresh_pair();
        let build = |l: &Lib| {
            l.add_node(1, -1, b"r", vals[0]);
            for j in 0..k {
                l.add_node(2 + j as i32, 1, b"c", vals[j + 1]);
            }
        };
        build(&p.c);
        build(&p.r);
        eq_bits("C27/wide", k, p.c.subtree_bits(1), p.r.subtree_bits(1));
        // and the clamped int view must agree too
        let cs = unsafe { (p.c.calculate_subtree_sum)(1) };
        let rs = unsafe { (p.r.calculate_subtree_sum)(1) };
        eq_i32("C27/wide-d2i", k, p.c.safe_d2i(cs), p.r.safe_d2i(rs));
    }
}
