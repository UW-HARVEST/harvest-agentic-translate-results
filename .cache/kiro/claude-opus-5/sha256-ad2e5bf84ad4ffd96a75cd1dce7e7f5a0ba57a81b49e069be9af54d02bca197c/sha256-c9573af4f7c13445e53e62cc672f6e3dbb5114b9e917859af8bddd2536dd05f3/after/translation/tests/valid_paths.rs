//! Phase B — valid-path differential tests, one `#[test]` per `CONFIGS.md` row.
//!
//! Every test loads BOTH `.so`s via `libloading` and compares `dest` bit-for-bit.

mod common;

use common::*;
use std::ffi::c_int;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Build a `count`-element `src` (3 floats per element) from a per-lane closure.
fn build(count: usize, mut lane: impl FnMut(usize, usize) -> f32) -> Vec<f32> {
    let mut v = Vec::with_capacity(3 * count);
    for e in 0..count {
        for l in 0..3 {
            v.push(lane(e, l));
        }
    }
    v
}

/// Force `src[0] < src[1]` (the *if* arm) for every element.
fn order_if(v: &mut [f32]) {
    for e in v.chunks_mut(3) {
        if !(e[0] < e[1]) {
            e.swap(0, 1);
        }
        if !(e[0] < e[1]) {
            // equal, or a NaN was involved: nudge deterministically
            e[0] = -1.5;
            e[1] = 2.25;
        }
    }
}

/// Force `src[0] > src[1]` (the *else* arm) for every element.
fn order_else(v: &mut [f32]) {
    for e in v.chunks_mut(3) {
        if !(e[0] > e[1]) {
            e.swap(0, 1);
        }
        if !(e[0] > e[1]) {
            e[0] = 2.25;
            e[1] = -1.5;
        }
    }
}

/// `sqd = (dy2-dx2)^2 + 4*dxy^2 >= 0` mathematically, so a *strictly* negative
/// `sqd` is only reachable through rounding: make `(dy2-dx2)^2` underflow to a
/// value whose computed expansion `dy2² - 2·dx2·dy2 + dx2²` rounds negative.
/// Row 5 sweeps near-equal pairs with `dxy = 0` and asserts on whichever side of
/// the clamp the C lands — the point is that both libraries agree.
fn near_equal_pair(rng: &mut Rng) -> (f32, f32) {
    let a = rng.normal_f32();
    let d = rng.below(4);
    let b = match d {
        0 => f32::from_bits(a.to_bits().wrapping_add(1)),
        1 => f32::from_bits(a.to_bits().wrapping_sub(1)),
        2 => f32::from_bits(a.to_bits().wrapping_add(rng.below(8) as u32)),
        _ => a * (1.0 + 1e-7 * (rng.below(8) as f32 - 4.0)),
    };
    (a, b)
}

// ---------------------------------------------------------------------------
// Rows 1-3 — axis A (arm select)
// ---------------------------------------------------------------------------

#[test]
fn row_01_if_arm_single_element() {
    let mut rng = Rng::new(0x1111_0001);
    for i in 0..ITERS {
        let mut s = build(1, |_, _| rng.tame_f32());
        order_if(&mut s);
        diff_call(&format!("row01/#{i}"), &s, 1);
    }
}

#[test]
fn row_02_else_arm_single_element() {
    let mut rng = Rng::new(0x1111_0002);
    for i in 0..ITERS {
        let mut s = build(1, |_, _| rng.tame_f32());
        order_else(&mut s);
        diff_call(&format!("row02/#{i}"), &s, 1);
    }
}

#[test]
fn row_03_exact_tie_takes_else_arm() {
    let mut rng = Rng::new(0x1111_0003);
    // hand-pinned ties, including the +0.0 / -0.0 pair (0.0 < -0.0 is false)
    let pinned: &[(f32, f32)] = &[
        (0.0, 0.0),
        (0.0, -0.0),
        (-0.0, 0.0),
        (-0.0, -0.0),
        (1.0, 1.0),
        (-3.5, -3.5),
        (f32::MIN_POSITIVE, f32::MIN_POSITIVE),
        (f32::INFINITY, f32::INFINITY),
        (f32::NEG_INFINITY, f32::NEG_INFINITY),
    ];
    for (i, &(a, b)) in pinned.iter().enumerate() {
        for dxy in [0.0f32, -0.0, 1.0, -2.5, f32::INFINITY, f32::MIN_POSITIVE] {
            diff_call(&format!("row03/pin{i}"), &[a, b, dxy], 1);
        }
    }
    for i in 0..ITERS {
        let a = rng.tame_f32();
        let s = [a, a, rng.tame_f32()];
        diff_call(&format!("row03/rand{i}"), &s, 1);
    }
}

// ---------------------------------------------------------------------------
// Row 4 — fully random bit patterns
// ---------------------------------------------------------------------------

#[test]
fn row_04_fully_random_bit_patterns() {
    let mut rng = Rng::new(0x1111_0004);
    for i in 0..ITERS {
        let s = build(1, |_, _| rng.any_f32());
        diff_call(&format!("row04/#{i}"), &s, 1);
    }
}

// ---------------------------------------------------------------------------
// Rows 5-9 — axis B (discriminant clamp)
// ---------------------------------------------------------------------------

#[test]
fn row_05_negative_discriminant_clamped() {
    let mut rng = Rng::new(0x1111_0005);
    // dxy = 0 so sqd == dy2² - 2·dx2·dy2 + dx2², which rounds negative for
    // near-equal operands. Also pin pairs that are known to cancel exactly.
    for i in 0..ITERS {
        let (a, b) = near_equal_pair(&mut rng);
        diff_call(&format!("row05/#{i}"), &[a, b, 0.0], 1);
        diff_call(&format!("row05n/#{i}"), &[a, b, -0.0], 1);
    }
    for e in 0..24 {
        let a = 2f32.powi(e * 5 - 60);
        let b = f32::from_bits(a.to_bits() + 1);
        diff_call(&format!("row05/pow{e}"), &[a, b, 0.0], 1);
        diff_call(&format!("row05/pow{e}r"), &[b, a, 0.0], 1);
    }
}

#[test]
fn row_06_discriminant_exactly_plus_zero() {
    // dx2 == dy2 and dxy == 0  =>  sqd == +0.0 exactly.
    let mut rng = Rng::new(0x1111_0006);
    for i in 0..ITERS {
        let a = rng.tame_f32();
        diff_call(&format!("row06/#{i}"), &[a, a, 0.0], 1);
    }
    for a in [0.0f32, -0.0, 1.0, -1.0, 1e-30, 1e30, f32::MIN_POSITIVE] {
        diff_call("row06/pin", &[a, a, 0.0], 1);
    }
}

#[test]
fn row_07_discriminant_negative_zero_is_not_clamped() {
    // dxy = -0.0 gives 4*(-0.0)*(-0.0) = +0.0, so reach -0.0 through the
    // accumulator instead: acc = -0.0 requires dy2² - 2dx2dy2 + dx2² == -0.0.
    // (+0.0) + (-0.0) == +0.0, so also drive `sqrtf` with -0.0 directly by
    // checking every pair whose computed acc is a zero, and let the differential
    // assertion catch any sign disagreement.
    let mut rng = Rng::new(0x1111_0007);
    for i in 0..ITERS {
        let a = rng.tame_f32();
        for dxy in [0.0f32, -0.0] {
            diff_call(&format!("row07/#{i}"), &[a, a, dxy], 1);
            diff_call(&format!("row07n/#{i}"), &[-a, -a, dxy], 1);
        }
    }
    // negative-zero operands everywhere
    for s0 in [0.0f32, -0.0] {
        for s1 in [0.0f32, -0.0] {
            for s2 in [0.0f32, -0.0] {
                diff_call("row07/zeros", &[s0, s1, s2], 1);
            }
        }
    }
}

#[test]
fn row_08_nan_discriminant_skips_clamp() {
    let mut rng = Rng::new(0x1111_0008);
    for i in 0..ITERS {
        // a NaN in exactly one lane, tame values in the others
        let lane = rng.below(3) as usize;
        let s = build(1, |_, l| {
            if l == lane {
                rng.qnan_f32()
            } else {
                rng.tame_f32()
            }
        });
        diff_call(&format!("row08/#{i}"), &s, 1);
    }
    // inf - inf and 0 * inf reaching sqd as NaN
    let pins: &[[f32; 3]] = &[
        [f32::INFINITY, f32::INFINITY, 0.0],
        [f32::INFINITY, f32::NEG_INFINITY, 0.0],
        [f32::NEG_INFINITY, f32::INFINITY, 0.0],
        [0.0, f32::INFINITY, 0.0],
        [f32::INFINITY, 0.0, 0.0],
        [1.0, 1.0, f32::INFINITY],
        [f32::INFINITY, 1.0, f32::INFINITY],
        [1.0, f32::INFINITY, f32::INFINITY],
        [f32::INFINITY, f32::INFINITY, f32::INFINITY],
        [f32::NEG_INFINITY, f32::NEG_INFINITY, f32::INFINITY],
    ];
    for (i, p) in pins.iter().enumerate() {
        diff_call(&format!("row08/pin{i}"), p, 1);
    }
}

#[test]
fn row_09_discriminant_overflows_to_infinity() {
    let mut rng = Rng::new(0x1111_0009);
    for i in 0..ITERS {
        let s = build(1, |_, _| rng.huge_f32());
        diff_call(&format!("row09/#{i}"), &s, 1);
    }
    for i in 0..ITERS / 4 {
        // huge dx2/dy2 with a tame dxy, and vice versa
        let s = [rng.huge_f32(), rng.huge_f32(), rng.tame_f32()];
        diff_call(&format!("row09/mix{i}"), &s, 1);
        let s = [rng.tame_f32(), rng.tame_f32(), rng.huge_f32()];
        diff_call(&format!("row09/mix2_{i}"), &s, 1);
    }
    for p in [f32::MAX, -f32::MAX] {
        diff_call("row09/flt_max", &[p, p, p], 1);
        diff_call("row09/flt_max2", &[p, -p, p], 1);
    }
}

// ---------------------------------------------------------------------------
// Rows 10-12 — axis C (count)
// ---------------------------------------------------------------------------

#[test]
fn row_10_count_zero_writes_nothing() {
    let mut rng = Rng::new(0x1111_0010);
    for i in 0..256 {
        let s = build(4, |_, _| rng.any_f32());
        let mut dc = poison(8);
        let mut dr = poison(8);
        unsafe {
            (c_tfm())(dc.as_mut_ptr(), s.as_ptr(), 0);
            (rust_tfm())(dr.as_mut_ptr(), s.as_ptr(), 0);
        }
        assert_bits_eq(&format!("row10/#{i}"), &s, 0, &dc, &dr);
        // and neither wrote anything at all
        assert!(
            dc.iter().all(|x| x.to_bits() == POISON_BITS),
            "row10: C wrote to dest with count=0"
        );
        assert!(
            dr.iter().all(|x| x.to_bits() == POISON_BITS),
            "row10: Rust wrote to dest with count=0"
        );
    }
}

#[test]
fn row_11_count_two_and_three() {
    let mut rng = Rng::new(0x1111_0011);
    for count in [2usize, 3] {
        for i in 0..ITERS {
            // force element 0 into the if-arm and element 1 into the else-arm
            let mut s = build(count, |_, _| rng.tame_f32());
            {
                let (a, rest) = s.split_at_mut(3);
                order_if(a);
                if count > 1 {
                    order_else(&mut rest[..3]);
                }
            }
            diff_call(&format!("row11/c{count}/#{i}"), &s, count as c_int);
        }
    }
}

#[test]
fn row_12_count_many() {
    let mut rng = Rng::new(0x1111_0012);
    for count in [MANY, 1000] {
        let iters = if count == 1000 { ITERS / 20 } else { ITERS / 4 };
        for i in 0..iters {
            let s = build(count, |_, _| rng.tame_f32());
            diff_call(&format!("row12/c{count}/#{i}"), &s, count as c_int);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 13-18 — axis D (value classes)
// ---------------------------------------------------------------------------

#[test]
fn row_13_mixed_value_classes_per_element() {
    let mut rng = Rng::new(0x1111_0013);
    for i in 0..ITERS / 2 {
        let s = build(MANY, |_, _| match rng.below(9) {
            0 => rng.tame_f32(),
            1 => rng.normal_f32(),
            2 => rng.subnormal_f32(),
            3 => rng.signed_zero(),
            4 => rng.inf(),
            5 => rng.qnan_f32(),
            6 => rng.snan_f32(),
            7 => rng.huge_f32(),
            _ => rng.any_f32(),
        });
        diff_call(&format!("row13/#{i}"), &s, MANY as c_int);
    }
}

#[test]
fn row_14_signed_zeros_only() {
    let mut rng = Rng::new(0x1111_0014);
    for i in 0..ITERS {
        let s = build(MANY, |_, _| rng.signed_zero());
        diff_call(&format!("row14/#{i}"), &s, MANY as c_int);
    }
}

#[test]
fn row_15_subnormals_only() {
    let mut rng = Rng::new(0x1111_0015);
    for i in 0..ITERS {
        let s = build(MANY, |_, _| rng.subnormal_f32());
        diff_call(&format!("row15/#{i}"), &s, MANY as c_int);
    }
    // extreme subnormals: smallest positive, largest subnormal
    let tiny = f32::from_bits(1);
    let big_sub = f32::from_bits(0x007F_FFFF);
    for a in [tiny, -tiny, big_sub, -big_sub] {
        for b in [tiny, -tiny, big_sub, -big_sub] {
            for c in [tiny, -tiny, big_sub, -big_sub, 0.0] {
                diff_call("row15/pin", &[a, b, c], 1);
            }
        }
    }
}

#[test]
fn row_16_infinities_only() {
    let mut rng = Rng::new(0x1111_0016);
    for i in 0..ITERS {
        let s = build(MANY, |_, _| rng.inf());
        diff_call(&format!("row16/#{i}"), &s, MANY as c_int);
    }
    // exhaustive 3-lane sign combinations
    for s0 in [f32::INFINITY, f32::NEG_INFINITY] {
        for s1 in [f32::INFINITY, f32::NEG_INFINITY] {
            for s2 in [f32::INFINITY, f32::NEG_INFINITY, 0.0, -0.0, 1.0] {
                diff_call("row16/pin", &[s0, s1, s2], 1);
            }
        }
    }
}

#[test]
fn row_17_qnan_payloads_and_signs() {
    let mut rng = Rng::new(0x1111_0017);
    for i in 0..ITERS {
        let lane = rng.below(3) as usize;
        let s = build(MANY, |_, l| {
            if l == lane {
                rng.qnan_f32()
            } else {
                rng.tame_f32()
            }
        });
        diff_call(&format!("row17/#{i}"), &s, MANY as c_int);
    }
    // all-NaN, and specific payloads in each single lane
    let nans: &[f32] = &[
        f32::from_bits(0x7FC0_0000),
        f32::from_bits(0xFFC0_0000),
        f32::from_bits(0x7FC0_0001),
        f32::from_bits(0xFFFF_FFFF),
        f32::from_bits(0x7FFF_FFFF),
        f32::from_bits(0x7FD5_5555),
        f32::from_bits(0xFFAA_AAAA),
    ];
    for (i, &n) in nans.iter().enumerate() {
        for lane in 0..3 {
            let mut s = [1.0f32, 2.0, 3.0];
            s[lane] = n;
            diff_call(&format!("row17/pin{i}/l{lane}"), &s, 1);
            let mut s2 = [3.0f32, 2.0, 1.0];
            s2[lane] = n;
            diff_call(&format!("row17/pin{i}/r{lane}"), &s2, 1);
        }
        diff_call(&format!("row17/all{i}"), &[n, n, n], 1);
    }
    // two NaNs at once — exercises which operand role wins
    for &a in nans {
        for &b in nans {
            diff_call("row17/pair", &[a, b, 1.0], 1);
            diff_call("row17/pair2", &[a, 1.0, b], 1);
            diff_call("row17/pair3", &[1.0, a, b], 1);
        }
    }
}

#[test]
fn row_18_signalling_nans() {
    let mut rng = Rng::new(0x1111_0018);
    for i in 0..ITERS {
        let lane = rng.below(3) as usize;
        let s = build(MANY, |_, l| {
            if l == lane {
                rng.snan_f32()
            } else {
                rng.tame_f32()
            }
        });
        diff_call(&format!("row18/#{i}"), &s, MANY as c_int);
    }
    let snans: &[f32] = &[
        f32::from_bits(0x7FA0_0000),
        f32::from_bits(0xFFA0_0000),
        f32::from_bits(0x7F80_0001),
        f32::from_bits(0xFF80_0001),
        f32::from_bits(0x7FBF_FFFF),
        f32::from_bits(0xFFBF_FFFF),
    ];
    for (i, &n) in snans.iter().enumerate() {
        for lane in 0..3 {
            let mut s = [1.0f32, 2.0, 3.0];
            s[lane] = n;
            diff_call(&format!("row18/pin{i}/l{lane}"), &s, 1);
            let mut s2 = [3.0f32, 2.0, 1.0];
            s2[lane] = n;
            diff_call(&format!("row18/pin{i}/r{lane}"), &s2, 1);
        }
        diff_call(&format!("row18/all{i}"), &[n, n, n], 1);
        for &m in snans {
            diff_call("row18/pair", &[n, m, 1.0], 1);
            diff_call("row18/pair2", &[n, 1.0, m], 1);
            diff_call("row18/pair3", &[1.0, n, m], 1);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 19-21 — axis E (dxy)
// ---------------------------------------------------------------------------

#[test]
fn row_19_dxy_signed_zero() {
    let mut rng = Rng::new(0x1111_0019);
    for i in 0..ITERS {
        let s = build(MANY, |_, l| if l == 2 { rng.signed_zero() } else { rng.tame_f32() });
        diff_call(&format!("row19/#{i}"), &s, MANY as c_int);
    }
}

#[test]
fn row_20_dxy_huge_overflows_only_that_term() {
    let mut rng = Rng::new(0x1111_0020);
    for i in 0..ITERS {
        let s = build(MANY, |_, l| if l == 2 { rng.huge_f32() } else { rng.tame_f32() });
        diff_call(&format!("row20/#{i}"), &s, MANY as c_int);
    }
    // 4*dxy*dxy overflow boundary: dxy just below/above sqrt(FLT_MAX)/2
    let edge = (f32::MAX.sqrt()) * 0.5;
    for k in -4i32..=4 {
        let d = f32::from_bits(edge.to_bits().wrapping_add(k as u32));
        diff_call("row20/edge", &[1.0, 2.0, d], 1);
        diff_call("row20/edge-", &[2.0, 1.0, -d], 1);
    }
}

#[test]
fn row_21_dxy_infinite_with_finite_rest() {
    let mut rng = Rng::new(0x1111_0021);
    for i in 0..ITERS {
        let s = build(MANY, |_, l| if l == 2 { rng.inf() } else { rng.tame_f32() });
        diff_call(&format!("row21/#{i}"), &s, MANY as c_int);
    }
    for d in [f32::INFINITY, f32::NEG_INFINITY] {
        for a in [0.0f32, -0.0, 1.0, -1.0, f32::MAX, -f32::MAX, f32::MIN_POSITIVE] {
            for b in [0.0f32, -0.0, 1.0, -1.0, f32::MAX, -f32::MAX] {
                diff_call("row21/pin", &[a, b, d], 1);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 22 — sqrtf domain sweep (glibc sqrtf vs inline sqrtss)
// ---------------------------------------------------------------------------

#[test]
fn row_22_sqrtf_domain_sweep() {
    // dx2 = dy2 = 0 and dxy = t  =>  sqd = 4t², radicand sweeps the domain.
    let mut rng = Rng::new(0x1111_0022);
    for i in 0..ITERS {
        let t = match rng.below(4) {
            0 => rng.subnormal_f32(),
            1 => rng.tame_f32(),
            2 => rng.huge_f32(),
            _ => rng.normal_f32(),
        };
        diff_call(&format!("row22/#{i}"), &[0.0, 0.0, t], 1);
        diff_call(&format!("row22b/#{i}"), &[-0.0, 0.0, t], 1);
    }
    // exact powers of two (radicand exactly representable, sqrt exact/inexact)
    for e in -70i32..=63 {
        let t = 2f32.powi(e);
        diff_call("row22/pow", &[0.0, 0.0, t], 1);
        diff_call("row22/pow2", &[1.0, 1.0, t], 1);
    }
    // radicands straddling a rounding boundary
    for base in [1.0f32, 2.0, 3.0, 1e-20, 1e20] {
        for k in -8i32..=8 {
            let t = f32::from_bits(base.to_bits().wrapping_add(k as u32));
            diff_call("row22/ulp", &[0.0, 0.0, t], 1);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 23-25 — axis G (buffer relationship)
// ---------------------------------------------------------------------------

/// Run `tfm` in-place on a shared buffer, once per library, from the same
/// starting bytes, and compare the whole buffer afterwards.
fn diff_inplace(label: &str, initial: &[f32], count: c_int, dest_off: usize) {
    let mut bc = initial.to_vec();
    let mut br = initial.to_vec();
    unsafe {
        (c_tfm())(bc.as_mut_ptr().add(dest_off), bc.as_ptr(), count);
        (rust_tfm())(br.as_mut_ptr().add(dest_off), br.as_ptr(), count);
    }
    assert_bits_eq(label, initial, count, &bc, &br);
}

#[test]
fn row_23_in_place_dest_equals_src() {
    let mut rng = Rng::new(0x1111_0023);
    for i in 0..ITERS {
        let s = build(MANY, |_, _| rng.tame_f32());
        diff_inplace(&format!("row23/#{i}"), &s, MANY as c_int, 0);
    }
    for i in 0..ITERS / 4 {
        // with exotic values too
        let s = build(MANY, |_, _| match rng.below(5) {
            0 => rng.signed_zero(),
            1 => rng.inf(),
            2 => rng.qnan_f32(),
            3 => rng.subnormal_f32(),
            _ => rng.any_f32(),
        });
        diff_inplace(&format!("row23x/#{i}"), &s, MANY as c_int, 0);
    }
}

#[test]
fn row_24_partial_forward_overlap() {
    let mut rng = Rng::new(0x1111_0024);
    for k in 1usize..=4 {
        for i in 0..ITERS / 4 {
            // buffer big enough for src (3n) and dest (2n) starting at +k
            let n = MANY;
            let s = build(n, |_, _| rng.tame_f32());
            let mut buf = s.clone();
            buf.resize(3 * n + 2 * n + k, 0.0);
            diff_inplace(&format!("row24/k{k}/#{i}"), &buf, n as c_int, k);
        }
    }
}

#[test]
fn row_25_unaligned_buffers() {
    let mut rng = Rng::new(0x1111_0025);
    let n = MANY;
    for i in 0..ITERS / 4 {
        // Allocate as bytes so a 1-byte offset is legitimate, then hand the
        // libraries misaligned float pointers. `movss` and Rust's raw
        // `read_unaligned`-free code both tolerate this on x86-64.
        let vals: Vec<f32> = (0..3 * n).map(|_| rng.tame_f32()).collect();
        let mut src_bytes = vec![0u8; 4 * 3 * n + 1];
        let mut dc_bytes = vec![0xEEu8; 4 * 2 * n + 1];
        let mut dr_bytes = vec![0xEEu8; 4 * 2 * n + 1];
        unsafe {
            std::ptr::copy_nonoverlapping(
                vals.as_ptr() as *const u8,
                src_bytes.as_mut_ptr().add(1),
                4 * 3 * n,
            );
            let sp = src_bytes.as_ptr().add(1) as *const f32;
            (c_tfm())(dc_bytes.as_mut_ptr().add(1) as *mut f32, sp, n as c_int);
            (rust_tfm())(dr_bytes.as_mut_ptr().add(1) as *mut f32, sp, n as c_int);
        }
        assert_eq!(
            dc_bytes, dr_bytes,
            "row25/#{i}: unaligned output bytes differ"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 26 — repeated calls (no hidden state)
// ---------------------------------------------------------------------------

#[test]
fn row_26_repeated_calls_no_hidden_state() {
    let mut rng = Rng::new(0x1111_0026);
    let n = MANY;
    for i in 0..ITERS / 4 {
        let s = build(n, |_, _| rng.tame_f32());
        let mut dc = poison(2 * n);
        let mut dr = poison(2 * n);
        // 5 back-to-back calls on the same buffers must be idempotent and equal
        let mut first_c: Option<Vec<u32>> = None;
        for round in 0..5 {
            unsafe {
                (c_tfm())(dc.as_mut_ptr(), s.as_ptr(), n as c_int);
                (rust_tfm())(dr.as_mut_ptr(), s.as_ptr(), n as c_int);
            }
            assert_bits_eq(&format!("row26/#{i}/r{round}"), &s, n as c_int, &dc, &dr);
            let bits: Vec<u32> = dc.iter().map(|x| x.to_bits()).collect();
            match &first_c {
                None => first_c = Some(bits),
                Some(f) => assert_eq!(*f, bits, "row26/#{i}: C not idempotent"),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 27-28 — value-dependent rounding
// ---------------------------------------------------------------------------

#[test]
fn row_27_dx2_equals_dy2() {
    let mut rng = Rng::new(0x1111_0027);
    for i in 0..ITERS {
        let a = rng.normal_f32();
        let s = build(MANY, |e, l| {
            if l == 2 {
                rng.tame_f32()
            } else if e == 0 {
                a
            } else {
                // every element keeps dx2 == dy2, with its own value
                if l == 0 {
                    let v = rng.normal_f32();
                    // stash so lane 1 can repeat it: recompute deterministically
                    v
                } else {
                    a
                }
            }
        });
        // rebuild strictly: lane0 == lane1 per element
        let mut s = s;
        for e in s.chunks_mut(3) {
            e[1] = e[0];
        }
        diff_call(&format!("row27/#{i}"), &s, MANY as c_int);
    }
}

#[test]
fn row_28_catastrophic_cancellation() {
    let mut rng = Rng::new(0x1111_0028);
    for i in 0..ITERS {
        let (a, b) = near_equal_pair(&mut rng);
        let dxy = match rng.below(3) {
            0 => 0.0,
            1 => rng.subnormal_f32(),
            _ => rng.tame_f32(),
        };
        diff_call(&format!("row28/#{i}"), &[a, b, dxy], 1);
        diff_call(&format!("row28r/#{i}"), &[b, a, dxy], 1);
    }
    // 1-ULP neighbourhoods around exact powers of two, both orders
    for e in -60i32..=60 {
        let a = 2f32.powi(e);
        for k in 1u32..=3 {
            let b = f32::from_bits(a.to_bits().wrapping_add(k));
            let c = f32::from_bits(a.to_bits().wrapping_sub(k));
            for pair in [[a, b], [b, a], [a, c], [c, a]] {
                for dxy in [0.0f32, 2f32.powi(e - 30), 2f32.powi(e)] {
                    diff_call("row28/ulp", &[pair[0], pair[1], dxy], 1);
                }
            }
        }
    }
}
