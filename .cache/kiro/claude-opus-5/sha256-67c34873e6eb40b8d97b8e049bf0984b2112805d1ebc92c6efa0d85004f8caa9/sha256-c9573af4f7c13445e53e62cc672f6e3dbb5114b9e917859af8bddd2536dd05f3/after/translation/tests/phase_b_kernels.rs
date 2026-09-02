//! Phase B — valid-path differential tests for the dispatcher and the integer
//! kernels: `f2`, `f3`, `f4`, `f5`, `f7`. CONFIGS.md rows C23-C47.

mod common;

use common::*;
use std::ffi::c_void;

macro_rules! bind {
    ($l:expr, $name:expr, $ty:ty) => {{
        let c: libloading::Symbol<$ty> = $l.c.get($name);
        let r: libloading::Symbol<$ty> = $l.r.get($name);
        (c, r)
    }};
}

const N: usize = 4000;

fn rand_circle(g: &mut Rng, tame: bool) -> C2Circle {
    if tame {
        C2Circle {
            p: C2v { x: g.finite_f32(10.0), y: g.finite_f32(10.0) },
            r: g.range_f32(-2.0, 8.0),
        }
    } else {
        C2Circle {
            p: C2v { x: g.mixed_f32(), y: g.mixed_f32() },
            r: g.mixed_f32(),
        }
    }
}

fn rand_aabb(g: &mut Rng, tame: bool) -> C2Aabb {
    if tame {
        let x0 = g.finite_f32(10.0);
        let y0 = g.finite_f32(10.0);
        C2Aabb {
            min: C2v { x: x0, y: y0 },
            max: C2v { x: x0 + g.range_f32(-2.0, 20.0), y: y0 + g.range_f32(-2.0, 20.0) },
        }
    } else {
        C2Aabb {
            min: C2v { x: g.mixed_f32(), y: g.mixed_f32() },
            max: C2v { x: g.mixed_f32(), y: g.mixed_f32() },
        }
    }
}

// ---------------------------------------------------------------------------
// C23-C27 — f2 dispatcher, all four valid enum combinations
// ---------------------------------------------------------------------------

#[test]
fn c23_f2_circle_circle() {
    let l = libs();
    let (c, r) = bind!(l, "f2", FnF2);
    let mut g = Rng::seeded();
    for i in 0..N {
        for tame in [true, false] {
            let a = rand_circle(&mut g, tame);
            let b = rand_circle(&mut g, tame);
            unsafe {
                let pa = &a as *const C2Circle as *const c_void;
                let pb = &b as *const C2Circle as *const c_void;
                eq_i32(
                    &format!("C23 f2(CIRCLE,CIRCLE) #{i} tame={tame} A={a:?} B={b:?}"),
                    c(pa, C2_TYPE_CIRCLE, pb, C2_TYPE_CIRCLE),
                    r(pa, C2_TYPE_CIRCLE, pb, C2_TYPE_CIRCLE),
                );
            }
        }
    }
}

#[test]
fn c24_f2_circle_aabb() {
    let l = libs();
    let (c, r) = bind!(l, "f2", FnF2);
    let mut g = Rng::seeded();
    for i in 0..N {
        for tame in [true, false] {
            let a = rand_circle(&mut g, tame);
            let b = rand_aabb(&mut g, tame);
            unsafe {
                let pa = &a as *const C2Circle as *const c_void;
                let pb = &b as *const C2Aabb as *const c_void;
                eq_i32(
                    &format!("C24 f2(CIRCLE,AABB) #{i} tame={tame}"),
                    c(pa, C2_TYPE_CIRCLE, pb, C2_TYPE_AABB),
                    r(pa, C2_TYPE_CIRCLE, pb, C2_TYPE_AABB),
                );
            }
        }
    }
}

#[test]
fn c25_f2_aabb_circle_argument_swap() {
    // The C does `c2CircletoAABB(*(c2Circle*)B, *(c2AABB*)A)` here — B is the
    // circle and A is the box, i.e. the arguments are swapped relative to the
    // other arms. This row exists specifically to catch getting that backwards.
    let l = libs();
    let (c, r) = bind!(l, "f2", FnF2);
    let (cc, rc) = bind!(l, "c2CircletoAABB", FnCircleAabb);
    let mut g = Rng::seeded();
    for i in 0..N {
        for tame in [true, false] {
            let boxa = rand_aabb(&mut g, tame);
            let circ = rand_circle(&mut g, tame);
            unsafe {
                let pa = &boxa as *const C2Aabb as *const c_void;
                let pb = &circ as *const C2Circle as *const c_void;
                let got_c = c(pa, C2_TYPE_AABB, pb, C2_TYPE_CIRCLE);
                let got_r = r(pa, C2_TYPE_AABB, pb, C2_TYPE_CIRCLE);
                eq_i32(&format!("C25 f2(AABB,CIRCLE) #{i} tame={tame}"), got_c, got_r);
                // and it must equal c2CircletoAABB(circle, box) in both impls
                eq_i32(
                    &format!("C25 swap-consistency C #{i}"),
                    cc(circ, boxa),
                    got_c,
                );
                eq_i32(
                    &format!("C25 swap-consistency Rust #{i}"),
                    rc(circ, boxa),
                    got_r,
                );
            }
        }
    }
}

#[test]
fn c26_f2_aabb_aabb() {
    let l = libs();
    let (c, r) = bind!(l, "f2", FnF2);
    let mut g = Rng::seeded();
    for i in 0..N {
        for tame in [true, false] {
            let a = rand_aabb(&mut g, tame);
            let b = rand_aabb(&mut g, tame);
            unsafe {
                let pa = &a as *const C2Aabb as *const c_void;
                let pb = &b as *const C2Aabb as *const c_void;
                eq_i32(
                    &format!("C26 f2(AABB,AABB) #{i} tame={tame}"),
                    c(pa, C2_TYPE_AABB, pb, C2_TYPE_AABB),
                    r(pa, C2_TYPE_AABB, pb, C2_TYPE_AABB),
                );
            }
        }
    }
}

#[test]
fn c27_f2_aliased_pointers() {
    let l = libs();
    let (c, r) = bind!(l, "f2", FnF2);
    let mut g = Rng::seeded();
    for i in 0..N {
        // A 16-byte buffer is valid to reinterpret as either shape.
        let buf = rand_aabb(&mut g, i % 2 == 0);
        let p = &buf as *const C2Aabb as *const c_void;
        for (ta, tb) in [
            (C2_TYPE_CIRCLE, C2_TYPE_CIRCLE),
            (C2_TYPE_CIRCLE, C2_TYPE_AABB),
            (C2_TYPE_AABB, C2_TYPE_CIRCLE),
            (C2_TYPE_AABB, C2_TYPE_AABB),
        ] {
            unsafe {
                eq_i32(
                    &format!("C27 f2 aliased({ta},{tb}) #{i}"),
                    c(p, ta, p, tb),
                    r(p, ta, p, tb),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C28-C34 — f3 floored division, every sign quadrant
// ---------------------------------------------------------------------------

fn f3_sweep(tag: &str, gen: impl Fn(&mut Rng) -> (i32, i32), n: usize) {
    let l = libs();
    let (c, r) = bind!(l, "f3", FnF3);
    let mut g = Rng::seeded();
    for i in 0..n {
        let (v1, v2) = gen(&mut g);
        unsafe { eq_i32(&format!("{tag} f3({v1},{v2}) #{i}"), c(v1, v2), r(v1, v2)) }
    }
}

#[test]
fn c28_f3_pos_pos() {
    f3_sweep(
        "C28",
        |g| {
            let v1 = (g.next_u32() >> 1) as i32;
            let v2 = ((g.next_u32() >> 1) as i32).max(1);
            (v1, v2)
        },
        N * 4,
    );
    // explicit magnitudes
    let l = libs();
    let (c, r) = bind!(l, "f3", FnF3);
    for &v1 in &[0i32, 1, 2, 7, 1000, i32::MAX - 1, i32::MAX] {
        for &v2 in &[1i32, 2, 3, 7, 1000, i32::MAX] {
            unsafe { eq_i32(&format!("C28 f3({v1},{v2})"), c(v1, v2), r(v1, v2)) }
        }
    }
}

#[test]
fn c29_f3_pos_neg() {
    f3_sweep(
        "C29",
        |g| {
            let v1 = (g.next_u32() >> 1) as i32;
            let v2 = -(((g.next_u32() >> 1) as i32).max(1));
            (v1, v2)
        },
        N * 4,
    );
}

#[test]
fn c30_f3_neg_pos() {
    f3_sweep(
        "C30",
        |g| {
            let v1 = -(((g.next_u32() >> 1) as i32).max(1));
            let v2 = ((g.next_u32() >> 1) as i32).max(1);
            (v1, v2)
        },
        N * 4,
    );
}

#[test]
fn c31_f3_neg_neg() {
    f3_sweep(
        "C31",
        |g| {
            let v1 = -(((g.next_u32() >> 1) as i32).max(1));
            let v2 = -(((g.next_u32() >> 1) as i32).max(1));
            (v1, v2)
        },
        N * 4,
    );
}

#[test]
fn c32_f3_exact_multiples_vs_remainder() {
    let l = libs();
    let (c, r) = bind!(l, "f3", FnF3);
    let mut g = Rng::seeded();
    for i in 0..N * 2 {
        let q = ((g.next_u32() >> 12) as i32).max(1);
        let d = ((g.next_u32() >> 12) as i32).max(1);
        for (sv, sd) in [(1i32, 1i32), (1, -1), (-1, 1), (-1, -1)] {
            let v2 = d.wrapping_mul(sd);
            // exact multiple: r == 0 (wrapping is fine — f3 must handle any i32)
            let v1e = q.wrapping_mul(d).wrapping_mul(sv);
            // inexact: r != 0
            let v1i = v1e.wrapping_add(sv.wrapping_mul((d / 2).max(1) % d.max(2)));
            unsafe {
                eq_i32(&format!("C32 exact f3({v1e},{v2}) #{i}"), c(v1e, v2), r(v1e, v2));
                eq_i32(&format!("C32 inexact f3({v1i},{v2}) #{i}"), c(v1i, v2), r(v1i, v2));
            }
        }
    }
}

#[test]
fn c33_f3_quotient_zero() {
    let l = libs();
    let (c, r) = bind!(l, "f3", FnF3);
    let mut g = Rng::seeded();
    for i in 0..N * 2 {
        let small = ((g.next_u32() >> 20) as i32).max(1);
        let big = small + 1 + (g.next_u32() >> 16) as i32;
        for (sv, sd) in [(1i32, 1i32), (1, -1), (-1, 1), (-1, -1)] {
            let (v1, v2) = (small * sv, big * sd);
            unsafe { eq_i32(&format!("C33 f3({v1},{v2}) #{i}"), c(v1, v2), r(v1, v2)) }
        }
    }
}

#[test]
fn c34_f3_full_random_sweep() {
    // Completely unconstrained i32 pairs: hits every branch including the
    // INT_MIN guards and v2 == 0.
    f3_sweep("C34", |g| (g.next_i32(), g.next_i32()), N * 12);
    // plus a dense sweep of small values around zero
    let l = libs();
    let (c, r) = bind!(l, "f3", FnF3);
    for v1 in -40i32..=40 {
        for v2 in -40i32..=40 {
            unsafe { eq_i32(&format!("C34 dense f3({v1},{v2})"), c(v1, v2), r(v1, v2)) }
        }
    }
}

// ---------------------------------------------------------------------------
// C35-C38 — f4 xorshift128+ (stateful, mutates through the pointer)
// ---------------------------------------------------------------------------

#[test]
fn c35_f4_single_step_random_states() {
    let l = libs();
    let (c, r) = bind!(l, "f4", FnF4);
    let mut g = Rng::seeded();
    for i in 0..N * 4 {
        let st = [g.next_u64(), g.next_u64()];
        let mut sc = CnRnd { state: st };
        let mut sr = CnRnd { state: st };
        unsafe {
            let vc = c(&mut sc);
            let vr = r(&mut sr);
            eq_f64(&format!("C35 f4 #{i} state={st:016x?}"), vc, vr);
            eq_rnd(&format!("C35 f4 state #{i}"), sc, sr);
        }
        // documented invariant of the C: result is always in [0.0, 1.0)
        assert!(
            (0.0..1.0).contains(&unsafe { c(&mut CnRnd { state: st }) }),
            "C35 f4 out of [0,1) for state {st:016x?}"
        );
    }
}

#[test]
fn c36_f4_long_chains() {
    let l = libs();
    let (c, r) = bind!(l, "f4", FnF4);
    let mut g = Rng::seeded();
    for chain in 0..40 {
        let st = [g.next_u64(), g.next_u64()];
        let mut sc = CnRnd { state: st };
        let mut sr = CnRnd { state: st };
        for step in 0..1000 {
            unsafe {
                let vc = c(&mut sc);
                let vr = r(&mut sr);
                eq_f64(&format!("C36 f4 chain{chain} step{step}"), vc, vr);
                eq_rnd(&format!("C36 f4 chain{chain} step{step} state"), sc, sr);
            }
        }
    }
}

#[test]
fn c37_f4_boundary_states() {
    let l = libs();
    let (c, r) = bind!(l, "f4", FnF4);
    let states: &[[u64; 2]] = &[
        [0, 0],
        [0, 1],
        [1, 0],
        [1, 1],
        [u64::MAX, u64::MAX],
        [u64::MAX, 0],
        [0, u64::MAX],
        [1 << 63, 1],
        [1, 1 << 63],
        [0xFFFF_FFFF_FFFF_F000, 0xFFF],
        [0x0123_4567_89AB_CDEF, 0xFEDC_BA98_7654_3210],
    ];
    for st in states {
        let mut sc = CnRnd { state: *st };
        let mut sr = CnRnd { state: *st };
        for step in 0..64 {
            unsafe {
                eq_f64(
                    &format!("C37 f4 state={st:016x?} step{step}"),
                    c(&mut sc),
                    r(&mut sr),
                );
                eq_rnd(&format!("C37 f4 state={st:016x?} step{step} state"), sc, sr);
            }
        }
    }
    // {0,0} is a fixed point of xorshift128+: verify it stays 0.0 forever
    let mut z = CnRnd { state: [0, 0] };
    for _ in 0..16 {
        let v = unsafe { c(&mut z) };
        assert_eq!(v.to_bits(), 0.0f64.to_bits(), "C37 zero-state must yield 0.0");
    }
    assert_eq!(z.state, [0, 0], "C37 zero-state must be a fixed point");
}

#[test]
fn c38_f4_state_readback_byte_exact() {
    // Compares the raw 16 bytes of cn_rnd_t after each call, not just the
    // returned double — the state mutation is part of the observable contract.
    let l = libs();
    let (c, r) = bind!(l, "f4", FnF4);
    let mut g = Rng::seeded();
    for i in 0..N {
        let st = [g.next_u64(), g.next_u64()];
        let mut sc = CnRnd { state: st };
        let mut sr = CnRnd { state: st };
        unsafe {
            c(&mut sc);
            r(&mut sr);
        }
        let bc: [u8; 16] = unsafe { std::mem::transmute(sc) };
        let br: [u8; 16] = unsafe { std::mem::transmute(sr) };
        assert_eq!(bc, br, "C38 f4 raw state bytes differ #{i} (init {st:016x?})");
    }
}

// ---------------------------------------------------------------------------
// C39-C41 — f5 bit reversal
// ---------------------------------------------------------------------------

#[test]
fn c39_f5_low16_random() {
    let l = libs();
    let (c, r) = bind!(l, "f5", FnF5);
    let mut g = Rng::seeded();
    for i in 0..N * 4 {
        let a = g.next_u32() & 0xFFFF;
        unsafe { eq_u32(&format!("C39 f5(0x{a:08x}) #{i}"), c(a), r(a)) }
    }
}

#[test]
fn c40_f5_full_width_random() {
    let l = libs();
    let (c, r) = bind!(l, "f5", FnF5);
    let mut g = Rng::seeded();
    for i in 0..N * 4 {
        let a = g.next_u32();
        unsafe { eq_u32(&format!("C40 f5(0x{a:08x}) #{i}"), c(a), r(a)) }
    }
    for &a in &[
        0u32,
        0xFFFF_FFFF,
        0xFFFF_0000,
        0x0000_FFFF,
        0xDEAD_BEEF,
        0x8000_0000,
        0x0000_8000,
        0x0000_0001,
        0xAAAA_5555,
        0x5555_AAAA,
    ] {
        unsafe { eq_u32(&format!("C40 f5(0x{a:08x})"), c(a), r(a)) }
    }
}

#[test]
fn c41_f5_exhaustive_low16() {
    let l = libs();
    let (c, r) = bind!(l, "f5", FnF5);
    for a in 0u32..=0xFFFF {
        unsafe { eq_u32(&format!("C41 f5(0x{a:04x})"), c(a), r(a)) }
    }
}

// ---------------------------------------------------------------------------
// C42-C47 — f7 frame-size bound
// ---------------------------------------------------------------------------

fn f7_check(tag: &str, cases: impl Iterator<Item = (u32, u32, u32)>) {
    let l = libs();
    let (c, r) = bind!(l, "f7", FnF7);
    for (i, (bs, ch, bd)) in cases.enumerate() {
        unsafe {
            eq_u32(
                &format!("{tag} f7(bs={bs},ch={ch},bd={bd}) #{i}"),
                c(bs, ch, bd),
                r(bs, ch, bd),
            )
        }
    }
}

#[test]
fn c42_f7_channels2_bitdepth32() {
    let mut g = Rng::seeded();
    let cases: Vec<_> = (0..N * 2).map(|_| (g.next_u32(), 2u32, 32u32)).collect();
    f7_check("C42", cases.into_iter());
    f7_check("C42 small", (0u32..2000).map(|bs| (bs, 2, 32)));
}

#[test]
fn c43_f7_channels2_bitdepth_not32() {
    let mut g = Rng::seeded();
    let cases: Vec<_> = (0..N * 2)
        .map(|_| {
            let bd = loop {
                let v = g.next_u32();
                if v != 32 {
                    break v;
                }
            };
            (g.next_u32(), 2u32, bd)
        })
        .collect();
    f7_check("C43", cases.into_iter());
    f7_check(
        "C43 small",
        (0u32..200).flat_map(|bs| [8u32, 16, 24, 31, 33, 64].map(move |bd| (bs, 2u32, bd))),
    );
}

#[test]
fn c44_f7_channels_not2() {
    let mut g = Rng::seeded();
    let cases: Vec<_> = (0..N * 2)
        .map(|_| {
            let ch = loop {
                let v = g.next_u32();
                if v != 2 {
                    break v;
                }
            };
            (g.next_u32(), ch, g.next_u32())
        })
        .collect();
    f7_check("C44", cases.into_iter());
    f7_check(
        "C44 small",
        (1u32..40)
            .filter(|c| *c != 2)
            .flat_map(|ch| [8u32, 16, 24, 32, 33].map(move |bd| (1024u32, ch, bd))),
    );
}

#[test]
fn c45_f7_channels_zero() {
    let mut g = Rng::seeded();
    let cases: Vec<_> = (0..N).map(|_| (g.next_u32(), 0u32, g.next_u32())).collect();
    f7_check("C45", cases.into_iter());
}

#[test]
fn c46_f7_realistic_flac_shapes() {
    let bss = [1u32, 2, 16, 192, 576, 1152, 2304, 4096, 4608, 16384, 65535];
    let bds = [4u32, 8, 12, 16, 20, 24, 32];
    let chs = [1u32, 2, 3, 4, 5, 6, 7, 8];
    let mut cases = Vec::new();
    for &bs in &bss {
        for &bd in &bds {
            for &ch in &chs {
                cases.push((bs, ch, bd));
            }
        }
    }
    f7_check("C46", cases.into_iter());
}

#[test]
fn c47_f7_unsigned_wrap() {
    let extremes = [
        0u32,
        1,
        2,
        3,
        7,
        8,
        0xFFFF,
        0x1_0000,
        0x7FFF_FFFF,
        0x8000_0000,
        0xFFFF_FFFE,
        0xFFFF_FFFF,
    ];
    let mut cases = Vec::new();
    for &bs in &extremes {
        for &ch in &extremes {
            for &bd in &extremes {
                cases.push((bs, ch, bd));
            }
        }
    }
    // plus 2 and 32 in every slot, since those are the branch constants
    for &bs in &extremes {
        for &bd in &extremes {
            cases.push((bs, 2, bd));
            cases.push((bs, bd, 32));
        }
    }
    f7_check("C47", cases.into_iter());

    let mut g = Rng::seeded();
    let rnd: Vec<_> = (0..N * 4)
        .map(|_| (g.next_u32(), g.next_u32(), g.next_u32()))
        .collect();
    f7_check("C47 rnd", rnd.into_iter());
}
