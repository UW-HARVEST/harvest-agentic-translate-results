//! Phase B — the one-shot wrapper `agglom` and the composed low-level
//! pipelines. CONFIGS.md rows C91-C98.

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

/// The 33 `agglom` parameters, in declaration order.
#[derive(Clone, Copy, Debug)]
struct Args {
    f2: [f32; 7],
    f3: [i32; 2],
    f4: [u64; 2],
    f5: u32,
    f7: [u32; 3],
    f9: [f32; 8],
    f10: u16,
    f11: [f32; 3],
    f12: [f32; 3],
    f13: [f32; 3],
}

fn call(f: &FnAgglom, a: &Args) -> f64 {
    unsafe {
        f(
            a.f2[0], a.f2[1], a.f2[2], a.f2[3], a.f2[4], a.f2[5], a.f2[6], a.f3[0], a.f3[1],
            a.f4[0], a.f4[1], a.f5, a.f7[0], a.f7[1], a.f7[2], a.f9[0], a.f9[1], a.f9[2], a.f9[3],
            a.f9[4], a.f9[5], a.f9[6], a.f9[7], a.f10, a.f11[0], a.f11[1], a.f11[2], a.f12[0],
            a.f12[1], a.f12[2], a.f13[0], a.f13[1], a.f13[2],
        )
    }
}

#[track_caller]
fn diff(tag: &str, a: &Args) {
    let l = libs();
    let (c, r) = bind!(l, "agglom", FnAgglom);
    eq_f64(&format!("{tag} agglom {a:?}"), call(&c, a), call(&r, a));
}

fn args_full_random(g: &mut Rng) -> Args {
    Args {
        f2: [
            g.any_f32(), g.any_f32(), g.any_f32(), g.any_f32(), g.any_f32(), g.any_f32(),
            g.any_f32(),
        ],
        f3: [g.next_i32(), g.next_i32()],
        f4: [g.next_u64(), g.next_u64()],
        f5: g.next_u32(),
        f7: [g.next_u32(), g.next_u32(), g.next_u32()],
        f9: [
            g.any_f32(), g.any_f32(), g.any_f32(), g.any_f32(), g.any_f32(), g.any_f32(),
            g.any_f32(), g.any_f32(),
        ],
        f10: g.next_u16(),
        f11: [g.any_f32(), g.any_f32(), g.any_f32()],
        f12: [g.any_f32(), g.any_f32(), g.any_f32()],
        f13: [g.any_f32(), g.any_f32(), g.any_f32()],
    }
}

fn args_mixed(g: &mut Rng) -> Args {
    Args {
        f2: [
            g.mixed_f32(), g.mixed_f32(), g.mixed_f32(), g.mixed_f32(), g.mixed_f32(),
            g.mixed_f32(), g.mixed_f32(),
        ],
        f3: [g.next_i32(), g.next_i32()],
        f4: [g.next_u64(), g.next_u64()],
        f5: g.next_u32(),
        f7: [g.next_u32(), g.next_u32(), g.next_u32()],
        f9: [
            g.mixed_f32(), g.mixed_f32(), g.mixed_f32(), g.mixed_f32(), g.mixed_f32(),
            g.mixed_f32(), g.mixed_f32(), g.mixed_f32(),
        ],
        f10: g.next_u16(),
        f11: [g.mixed_f32(), g.mixed_f32(), g.mixed_f32()],
        f12: [g.mixed_f32(), g.mixed_f32(), g.mixed_f32()],
        f13: [g.mixed_f32(), g.mixed_f32(), g.mixed_f32()],
    }
}

fn args_realistic(g: &mut Rng) -> Args {
    let bd = [8u32, 16, 24, 32][g.below(4) as usize];
    let ch = [1u32, 2, 4, 8][g.below(4) as usize];
    let bs = [192u32, 576, 1152, 4096, 4608][g.below(5) as usize];
    Args {
        f2: [
            g.finite_f32(10.0), g.finite_f32(10.0), g.range_f32(0.0, 5.0),
            g.finite_f32(10.0), g.finite_f32(10.0), g.finite_f32(10.0), g.finite_f32(10.0),
        ],
        f3: [
            (g.next_u32() >> 8) as i32 * if g.below(2) == 0 { 1 } else { -1 },
            ((g.next_u32() >> 16) as i32).max(1) * if g.below(2) == 0 { 1 } else { -1 },
        ],
        f4: [g.next_u64(), g.next_u64()],
        f5: g.next_u32() & 0xFFFF,
        f7: [bs, ch, bd],
        f9: [
            0.0, 0.0, g.range_f32(1.0, 5.0), g.range_f32(-1.0, 1.0),
            g.range_f32(-1.0, 1.0), g.range_f32(1.0, 5.0),
            g.range_f32(0.0, 3.0), g.range_f32(0.0, 3.0),
        ],
        f10: g.next_u16() & 0x7BFF, // finite half
        f11: [g.range_f32(0.0, 360.0), g.range_f32(0.0, 1.0), g.range_f32(0.0, 1.0)],
        f12: [g.range_f32(0.0, 360.0), g.range_f32(0.0, 1.0), g.range_f32(0.0, 1.0)],
        f13: [g.range_f32(0.0, 1.0), g.range_f32(0.0, 1.0), g.range_f32(0.0, 1.0)],
    }
}

// ---------------------------------------------------------------------------
// C91, C92 — broad sweeps
// ---------------------------------------------------------------------------

#[test]
fn c91_agglom_full_bit_range() {
    let mut g = Rng::seeded();
    for i in 0..N * 3 {
        diff(&format!("C91 anybits #{i}"), &args_full_random(&mut g));
    }
    for i in 0..N * 3 {
        diff(&format!("C91 mixed #{i}"), &args_mixed(&mut g));
    }
}

#[test]
fn c92_agglom_realistic() {
    let mut g = Rng::seeded();
    for i in 0..N * 3 {
        diff(&format!("C92 #{i}"), &args_realistic(&mut g));
    }
}

// ---------------------------------------------------------------------------
// C93 — f3 corner cases inside agglom
// ---------------------------------------------------------------------------

#[test]
fn c93_agglom_f3_corners() {
    let mut g = Rng::seeded();
    let specials = [0i32, 1, -1, i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1, 2, -2];
    for &v1 in &specials {
        for &v2 in &specials {
            let mut a = args_realistic(&mut g);
            a.f3 = [v1, v2];
            diff(&format!("C93 f3=({v1},{v2})"), &a);
            let mut b = args_mixed(&mut g);
            b.f3 = [v1, v2];
            diff(&format!("C93 mixed f3=({v1},{v2})"), &b);
        }
    }
}

// ---------------------------------------------------------------------------
// C94 — f4 / f10 / f7 corner cases inside agglom
// ---------------------------------------------------------------------------

#[test]
fn c94_agglom_f4_f10_f7_corners() {
    let mut g = Rng::seeded();
    for st in [
        [0u64, 0],
        [0, 1],
        [1, 0],
        [u64::MAX, u64::MAX],
        [1 << 63, 1],
        [0xFFFF_FFFF_FFFF_F000, 0xFFF],
    ] {
        let mut a = args_realistic(&mut g);
        a.f4 = st;
        diff(&format!("C94 f4={st:016x?}"), &a);
    }
    // every half-float exponent class, incl. inf/NaN encodings
    for n in 0u16..64 {
        for m in [0u16, 1, 512, 1023] {
            let mut a = args_realistic(&mut g);
            a.f10 = (n << 10) | m;
            diff(&format!("C94 f10=0x{:04x}", a.f10), &a);
        }
    }
    // u32-wrapping f7 arguments
    let ext = [0u32, 1, 2, 32, 0xFFFF, 0x8000_0000, 0xFFFF_FFFF];
    for &bs in &ext {
        for &ch in &ext {
            for &bd in &ext {
                let mut a = args_realistic(&mut g);
                a.f7 = [bs, ch, bd];
                diff(&format!("C94 f7=({bs},{ch},{bd})"), &a);
            }
        }
    }
    // f5 across the full width
    for &v in &[0u32, 1, 0xFFFF, 0x1_0000, 0xFFFF_FFFF, 0xDEAD_BEEF, 0x8000_0000] {
        let mut a = args_realistic(&mut g);
        a.f5 = v;
        diff(&format!("C94 f5=0x{v:08x}"), &a);
    }
}

// ---------------------------------------------------------------------------
// C95 — the isnan filters
// ---------------------------------------------------------------------------

#[test]
fn c95_agglom_isnan_filters() {
    let mut g = Rng::seeded();
    let nans = [
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7F80_0001),
        f32::from_bits(0x7FAB_CDEF),
    ];
    let infs = [f32::INFINITY, f32::NEG_INFINITY];
    for &n in &nans {
        // s == 0 early-outs
        let mut a = args_realistic(&mut g);
        a.f11 = [g.range_f32(0.0, 360.0), 0.0, g.range_f32(0.0, 1.0)];
        a.f12 = [g.range_f32(0.0, 360.0), 0.0, g.range_f32(0.0, 1.0)];
        a.f13 = [0.5, 0.5, 0.5]; // delta == 0
        diff("C95 s==0 / delta==0", &a);

        // NaN into each colour triple -> the term must be skipped
        for slot in 0..3 {
            let mut b = args_realistic(&mut g);
            b.f11[slot] = n;
            diff(&format!("C95 f11[{slot}]=nan"), &b);
            let mut c = args_realistic(&mut g);
            c.f12[slot] = n;
            diff(&format!("C95 f12[{slot}]=nan"), &c);
            let mut d = args_realistic(&mut g);
            d.f13[slot] = n;
            diff(&format!("C95 f13[{slot}]=nan"), &d);
            let mut e = args_realistic(&mut g);
            e.f9[slot] = n;
            diff(&format!("C95 f9[{slot}]=nan"), &e);
            let mut f = args_realistic(&mut g);
            f.f2[slot] = n;
            diff(&format!("C95 f2[{slot}]=nan"), &f);
        }
        // NaN everywhere in the float args at once
        let mut z = args_realistic(&mut g);
        z.f2 = [n; 7];
        z.f9 = [n; 8];
        z.f11 = [n; 3];
        z.f12 = [n; 3];
        z.f13 = [n; 3];
        diff("C95 all-nan", &z);
    }
    // ±inf is NOT filtered by isnan and must propagate into `ret`
    for &v in &infs {
        for slot in 0..3 {
            let mut a = args_realistic(&mut g);
            a.f11[slot] = v;
            diff(&format!("C95 f11[{slot}]=inf"), &a);
            let mut b = args_realistic(&mut g);
            b.f12[slot] = v;
            diff(&format!("C95 f12[{slot}]=inf"), &b);
            let mut c = args_realistic(&mut g);
            c.f13[slot] = v;
            diff(&format!("C95 f13[{slot}]=inf"), &c);
        }
        let mut z = args_realistic(&mut g);
        z.f11 = [v; 3];
        z.f12 = [v; 3];
        z.f13 = [v; 3];
        diff("C95 all-inf", &z);
    }
    // f3 == 0 contributes nothing (no error surfaced)
    let mut a = args_realistic(&mut g);
    a.f3 = [12345, 0];
    diff("C95 f3_2==0", &a);
}

// ---------------------------------------------------------------------------
// C96 — degenerate f9 inside agglom -> inf into the f64 accumulator
// ---------------------------------------------------------------------------

#[test]
fn c96_agglom_degenerate_f9() {
    let mut g = Rng::seeded();
    for i in 0..N {
        let mut a = args_realistic(&mut g);
        // p1 == p2 == p3 -> invDenom = 1/0
        let (x, y) = (g.finite_f32(5.0), g.finite_f32(5.0));
        a.f9 = [x, y, x, y, x, y, g.finite_f32(5.0), g.finite_f32(5.0)];
        diff(&format!("C96 coincident #{i}"), &a);

        // collinear
        let mut b = args_realistic(&mut g);
        let (x1, y1) = (g.finite_f32(5.0), g.finite_f32(5.0));
        let (x2, y2) = (g.finite_f32(5.0), g.finite_f32(5.0));
        let t = g.range_f32(-2.0, 2.0);
        b.f9 = [
            x1, y1, x2, y2,
            x1 + t * (x2 - x1), y1 + t * (y2 - y1),
            g.finite_f32(5.0), g.finite_f32(5.0),
        ];
        diff(&format!("C96 collinear #{i}"), &b);
    }
}

// ---------------------------------------------------------------------------
// C97, C98 — composed pipelines across the FFI boundary
// ---------------------------------------------------------------------------

#[test]
fn c97_pipeline_sub_dot_via_f2() {
    // Drive the whole c2Sub -> c2Dot -> compare chain the way the library's
    // own shape tests do, but through the public `f2` dispatcher, and also
    // reproduce it manually from the exported leaves so an error anywhere in
    // the composition shows up.
    let l = libs();
    let (f2c, f2r) = bind!(l, "f2", FnF2);
    let (subc, subr) = bind!(l, "c2Sub", FnC2Bin);
    let (dotc, dotr) = bind!(l, "c2Dot", FnC2Dot);
    let mut g = Rng::seeded();
    for i in 0..N * 2 {
        let a = C2Circle {
            p: C2v { x: g.finite_f32(10.0), y: g.finite_f32(10.0) },
            r: g.range_f32(-1.0, 6.0),
        };
        let b = C2Circle {
            p: C2v { x: g.finite_f32(10.0), y: g.finite_f32(10.0) },
            r: g.range_f32(-1.0, 6.0),
        };
        unsafe {
            let pa = &a as *const C2Circle as *const c_void;
            let pb = &b as *const C2Circle as *const c_void;
            let vc = f2c(pa, C2_TYPE_CIRCLE, pb, C2_TYPE_CIRCLE);
            let vr = f2r(pa, C2_TYPE_CIRCLE, pb, C2_TYPE_CIRCLE);
            eq_i32(&format!("C97 f2 #{i}"), vc, vr);

            // manual re-composition from the exported leaves
            let dc = subc(b.p, a.p);
            let dr = subr(b.p, a.p);
            eq_vec2(&format!("C97 c2Sub #{i}"), dc, dr);
            let d2c = dotc(dc, dc);
            let d2r = dotr(dr, dr);
            eq_f32(&format!("C97 c2Dot #{i}"), d2c, d2r);
            let r2 = (a.r + b.r) * (a.r + b.r);
            eq_i32(
                &format!("C97 recomposed matches f2 (C) #{i}"),
                (d2c < r2) as i32,
                vc,
            );
        }
    }
}

#[test]
fn c98_pipeline_crossed_between_libraries() {
    // Chain the leaf functions, feeding each stage's C output into the next
    // Rust call and vice-versa. If any stage's ABI or value differed, the two
    // chains would drift apart.
    let l = libs();
    let (minc, minr) = bind!(l, "c2Minv", FnC2Bin);
    let (maxc, maxr) = bind!(l, "c2Maxv", FnC2Bin);
    let (clampc, clampr) = bind!(l, "c2Clampv", FnC2Clamp);
    let (subc, subr) = bind!(l, "c2Sub", FnC2Bin);
    let (dotc, dotr) = bind!(l, "c2Dot", FnC2Dot);
    let (vc_, vr_) = bind!(l, "c2V", FnC2V);

    let mut g = Rng::seeded();
    for i in 0..N * 2 {
        let (x, y) = (g.mixed_f32(), g.mixed_f32());
        unsafe {
            // stage 1: c2V
            let mut sc = vc_(x, y);
            let mut sr = vr_(x, y);
            eq_vec2(&format!("C98 stage1 #{i}"), sc, sr);

            for step in 0..6 {
                let lo = C2v { x: g.mixed_f32(), y: g.mixed_f32() };
                let hi = C2v { x: g.mixed_f32(), y: g.mixed_f32() };
                // stage 2: min / max
                let mnc = minc(sc, hi);
                let mnr = minr(sr, hi);
                eq_vec2(&format!("C98 min #{i}/{step}"), mnc, mnr);
                let mxc = maxc(lo, mnc);
                let mxr = maxr(lo, mnr);
                eq_vec2(&format!("C98 max #{i}/{step}"), mxc, mxr);
                // stage 3: clamp (which internally is max(lo, min(a, hi)))
                let clc = clampc(sc, lo, hi);
                let clr = clampr(sr, lo, hi);
                eq_vec2(&format!("C98 clamp #{i}/{step}"), clc, clr);
                assert_eq!(
                    clc.x.to_bits(),
                    mxc.x.to_bits(),
                    "C98 clamp != max(lo,min(a,hi)) in C #{i}/{step}"
                );
                // stage 4: sub, then dot -> scalar, then back into a vector
                let sbc = subc(sc, clc);
                let sbr = subr(sr, clr);
                eq_vec2(&format!("C98 sub #{i}/{step}"), sbc, sbr);
                let dc = dotc(sbc, sbc);
                let dr = dotr(sbr, sbr);
                eq_f32(&format!("C98 dot #{i}/{step}"), dc, dr);
                // feed forward
                sc = vc_(dc, sbc.y);
                sr = vr_(dr, sbr.y);
                eq_vec2(&format!("C98 feedback #{i}/{step}"), sc, sr);
            }
        }
    }
}
