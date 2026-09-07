//! Phase C — error/rejection-path differential tests.
//! One test (or clearly-labelled block) per row of ERRORS.md.
//!
//! Every call goes through a `.so` export loaded with `libloading`.
#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::{c_char, c_int, c_void};

const POISON: c2Proxy = c2Proxy {
    radius: -123.5,
    count: -9,
    verts: [c2v { x: 1.25, y: -1.25 }; 8],
};

fn mk_simplex(count: c_int, div: f32) -> c2Simplex {
    let mut s = c2Simplex::default();
    for (k, v) in s.verts.iter_mut().enumerate() {
        let f = (k as f32) + 1.0;
        *v = c2sv {
            sA: c2v { x: f, y: -f },
            sB: c2v { x: 2.0 * f, y: 3.0 * f },
            p: c2v { x: f * 0.5, y: -f * 1.5 },
            u: f * 0.25,
            iA: k as c_int,
            iB: (3 - k) as c_int,
        };
    }
    s.div = div;
    s.count = count;
    s
}

/// Counts that no `switch` in the C source has a real `case` for.
const BAD_COUNTS: [c_int; 6] = [0, 4, 5, -1, -7, c_int::MIN];

// ===========================================================================
// Row 1 (+ generic out-of-range enum across the FFI boundary)
// ===========================================================================

#[test]
fn row01_make_proxy_out_of_range_enum() {
    let (c, r) = load();
    let f_c: libloading::Symbol<FnMakeProxy> = c.sym("c2MakeProxy");
    let f_r: libloading::Symbol<FnMakeProxy> = r.sym("c2MakeProxy");

    // A C enum accepts any int; these have no valid variant.
    let bad: [u32; 8] = [3, 4, 99, 255, 0x8000_0000, 0xFFFF_FFFF, 0x7FFF_FFFF, 1 << 16];
    let circ = c2Circle { p: c2v { x: 3.0, y: -4.0 }, r: 7.0 };

    for ty in bad {
        let mut pc = POISON;
        let mut pr = POISON;
        unsafe {
            f_c(&circ as *const _ as *const c_void, ty, &mut pc);
            f_r(&circ as *const _ as *const c_void, ty, &mut pr);
        }
        assert!(
            proxy_eq(&pc, &pr),
            "c2MakeProxy bad enum {ty:#x}: C={pc:?} R={pr:?}"
        );
        // The C `switch` has no `default:`, so nothing at all is written.
        assert!(
            proxy_eq(&pc, &POISON),
            "c2MakeProxy bad enum {ty:#x}: C wrote to the proxy: {pc:?}"
        );
        assert!(
            proxy_eq(&pr, &POISON),
            "c2MakeProxy bad enum {ty:#x}: Rust wrote to the proxy: {pr:?}"
        );
    }

    // NULL shape pointer with an out-of-range type: the C never dereferences
    // `shape` on that path, so neither may the Rust.
    for ty in bad {
        let mut pc = POISON;
        let mut pr = POISON;
        unsafe {
            f_c(std::ptr::null(), ty, &mut pc);
            f_r(std::ptr::null(), ty, &mut pr);
        }
        assert!(proxy_eq(&pc, &pr), "c2MakeProxy NULL shape, bad enum {ty:#x}");
        assert!(proxy_eq(&pc, &POISON), "c2MakeProxy NULL shape wrote proxy (C)");
    }
}

// ===========================================================================
// Rows 2 & 3: c2GJKSimplexMetric rejection -> 0.0f
// ===========================================================================

#[test]
fn row02_row03_simplex_metric_bad_count() {
    let (c, r) = load();
    let f_c: libloading::Symbol<FnSimplexF> = c.sym("c2GJKSimplexMetric");
    let f_r: libloading::Symbol<FnSimplexF> = r.sym("c2GJKSimplexMetric");

    let mut counts: Vec<c_int> = vec![1]; // row 2
    counts.extend_from_slice(&BAD_COUNTS); // row 3
    for count in counts {
        for div in [1.0f32, 0.0, -0.0, f32::NAN, 1e30] {
            let mut sc = mk_simplex(count, div);
            let mut sr = sc;
            let (vc, vr) = unsafe { (f_c(&mut sc), f_r(&mut sr)) };
            assert!(
                feq(vc, vr),
                "c2GJKSimplexMetric count{count} div{}: C={} R={}",
                fdesc(div), fdesc(vc), fdesc(vr)
            );
            // Documented sentinel: the rejection arm returns exactly 0.0f.
            assert!(
                feq_strict(vc, 0.0),
                "c2GJKSimplexMetric count{count}: C sentinel changed: {}",
                fdesc(vc)
            );
            assert!(
                feq_strict(vr, 0.0),
                "c2GJKSimplexMetric count{count}: Rust sentinel is {}",
                fdesc(vr)
            );
        }
    }
}

// ===========================================================================
// Rows 4, 5, 6: c2D
// ===========================================================================

#[test]
fn row04_row05_c2D_bad_count() {
    let (c, r) = load();
    let f_c: libloading::Symbol<FnSimplexV> = c.sym("c2D");
    let f_r: libloading::Symbol<FnSimplexV> = r.sym("c2D");

    let mut counts: Vec<c_int> = vec![3]; // row 4
    counts.extend_from_slice(&BAD_COUNTS); // row 5
    for count in counts {
        let mut sc = mk_simplex(count, 1.0);
        let mut sr = sc;
        let (vc, vr) = unsafe { (f_c(&mut sc), f_r(&mut sr)) };
        assert!(veq(vc, vr), "c2D count{count}: C={} R={}", vdesc(vc), vdesc(vr));
        assert!(
            veq_strict(vc, c2v { x: 0.0, y: 0.0 }),
            "c2D count{count}: C sentinel changed: {}",
            vdesc(vc)
        );
        assert!(
            veq_strict(vr, c2v { x: 0.0, y: 0.0 }),
            "c2D count{count}: Rust sentinel is {}",
            vdesc(vr)
        );
    }
}

#[test]
fn row06_c2D_det_not_positive() {
    let (c, r) = load();
    let f_c: libloading::Symbol<FnSimplexV> = c.sym("c2D");
    let f_r: libloading::Symbol<FnSimplexV> = r.sym("c2D");
    let ccw_c: libloading::Symbol<FnVv> = c.sym("c2CCW90");
    let skew_c: libloading::Symbol<FnVv> = c.sym("c2Skew");

    // (a.p, b.p) pairs where Det2(ab, -a.p) is <= 0 or NaN, plus one > 0 control.
    let cases: [(c2v, c2v); 6] = [
        // collinear with the origin -> det == 0 -> CCW90 arm
        (c2v { x: 1.0, y: 1.0 }, c2v { x: 2.0, y: 2.0 }),
        // det < 0 -> CCW90 arm
        (c2v { x: 1.0, y: 0.0 }, c2v { x: 0.0, y: -1.0 }),
        // NaN -> `> 0` is false -> CCW90 arm
        (c2v { x: f32::NAN, y: 1.0 }, c2v { x: 2.0, y: 3.0 }),
        (c2v { x: 1.0, y: 2.0 }, c2v { x: f32::NAN, y: f32::NAN }),
        // a == b -> ab == 0 -> det == 0 -> CCW90 arm
        (c2v { x: 5.0, y: -5.0 }, c2v { x: 5.0, y: -5.0 }),
        // control: det > 0 -> Skew arm
        (c2v { x: 1.0, y: 0.0 }, c2v { x: 0.0, y: 1.0 }),
    ];
    for (i, (ap, bp)) in cases.iter().enumerate() {
        let mut sc = mk_simplex(2, 1.0);
        sc.verts[0].p = *ap;
        sc.verts[1].p = *bp;
        let mut sr = sc;
        let (vc, vr) = unsafe { (f_c(&mut sc), f_r(&mut sr)) };
        assert!(
            veq(vc, vr),
            "c2D case{i} a={} b={}: C={} R={}",
            vdesc(*ap), vdesc(*bp), vdesc(vc), vdesc(vr)
        );
        // Identify which arm the C took, to prove the rejection arm was hit.
        let ab = c2v { x: bp.x - ap.x, y: bp.y - ap.y };
        let expect = unsafe { if i == 5 { skew_c(ab) } else { ccw_c(ab) } };
        assert!(veq(vc, expect), "c2D case{i}: C took the unexpected arm");
    }
}

// ===========================================================================
// Rows 7 & 8: c2L
// ===========================================================================

#[test]
fn row07_row08_c2L() {
    let (c, r) = load();
    let f_c: libloading::Symbol<FnSimplexV> = c.sym("c2L");
    let f_r: libloading::Symbol<FnSimplexV> = r.sym("c2L");

    // row 7: count not in {1,2} -> (0,0)
    let mut counts: Vec<c_int> = vec![3];
    counts.extend_from_slice(&BAD_COUNTS);
    for count in counts {
        let mut sc = mk_simplex(count, 1.0);
        let mut sr = sc;
        let (vc, vr) = unsafe { (f_c(&mut sc), f_r(&mut sr)) };
        assert!(veq(vc, vr), "c2L count{count}: C={} R={}", vdesc(vc), vdesc(vr));
        assert!(veq_strict(vc, c2v { x: 0.0, y: 0.0 }), "c2L count{count}: C sentinel");
        assert!(veq_strict(vr, c2v { x: 0.0, y: 0.0 }), "c2L count{count}: Rust sentinel");
    }

    // row 8: div == 0 (and -0, NaN, inf) with count 2 -> inf/NaN propagation
    for div in [0.0f32, -0.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::MIN_POSITIVE] {
        for count in [1, 2] {
            let mut sc = mk_simplex(count, div);
            let mut sr = sc;
            let (vc, vr) = unsafe { (f_c(&mut sc), f_r(&mut sr)) };
            assert!(
                veq(vc, vr),
                "c2L count{count} div={}: C={} R={}",
                fdesc(div), vdesc(vc), vdesc(vr)
            );
        }
    }
}

// ===========================================================================
// Rows 9 & 10: c2Witness
// ===========================================================================

#[test]
fn row09_row10_witness() {
    let (c, r) = load();
    let f_c: libloading::Symbol<FnWitness> = c.sym("c2Witness");
    let f_r: libloading::Symbol<FnWitness> = r.sym("c2Witness");

    // row 9
    for count in BAD_COUNTS {
        let mut sc = mk_simplex(count, 1.0);
        let mut sr = sc;
        let (mut ac, mut bc) = (c2v { x: 9.0, y: 9.0 }, c2v { x: -9.0, y: -9.0 });
        let (mut ar, mut br) = (ac, bc);
        unsafe {
            f_c(&mut sc, &mut ac, &mut bc);
            f_r(&mut sr, &mut ar, &mut br);
        }
        assert!(
            veq(ac, ar) && veq(bc, br),
            "c2Witness count{count}: C=({},{}) R=({},{})",
            vdesc(ac), vdesc(bc), vdesc(ar), vdesc(br)
        );
        assert!(
            veq_strict(ac, c2v { x: 0.0, y: 0.0 }) && veq_strict(bc, c2v { x: 0.0, y: 0.0 }),
            "c2Witness count{count}: C sentinel changed"
        );
        assert!(
            veq_strict(ar, c2v { x: 0.0, y: 0.0 }) && veq_strict(br, c2v { x: 0.0, y: 0.0 }),
            "c2Witness count{count}: Rust sentinel differs"
        );
    }

    // row 10
    for div in [0.0f32, -0.0, f32::NAN, f32::INFINITY, f32::MIN_POSITIVE] {
        for count in [1, 2, 3] {
            let mut sc = mk_simplex(count, div);
            let mut sr = sc;
            let (mut ac, mut bc) = (c2v { x: 9.0, y: 9.0 }, c2v { x: -9.0, y: -9.0 });
            let (mut ar, mut br) = (ac, bc);
            unsafe {
                f_c(&mut sc, &mut ac, &mut bc);
                f_r(&mut sr, &mut ar, &mut br);
            }
            assert!(
                veq(ac, ar) && veq(bc, br),
                "c2Witness count{count} div={}: C=({},{}) R=({},{})",
                fdesc(div), vdesc(ac), vdesc(bc), vdesc(ar), vdesc(br)
            );
        }
    }
}

// ===========================================================================
// Rows 11 & 12: c2Support
// ===========================================================================

#[test]
fn row11_row12_support() {
    let (c, r) = load();
    let f_c: libloading::Symbol<FnSupport> = c.sym("c2Support");
    let f_r: libloading::Symbol<FnSupport> = r.sym("c2Support");

    let verts = [
        c2v { x: 1.0, y: 2.0 },
        c2v { x: -3.0, y: 4.0 },
        c2v { x: 5.0, y: -6.0 },
        c2v { x: 7.0, y: 8.0 },
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -1.0, y: -1.0 },
        c2v { x: 100.0, y: 0.0 },
        c2v { x: 0.0, y: 100.0 },
    ];

    // row 11: count <= 0. `verts[0]` is still dereferenced, so the pointer must
    // stay valid; the return value is the sentinel 0.
    for count in [0i32, -1, -5, i32::MIN] {
        for d in [c2v { x: 1.0, y: 1.0 }, c2v { x: 0.0, y: 0.0 }, c2v { x: f32::NAN, y: 1.0 }] {
            let (ic, ir) =
                unsafe { (f_c(verts.as_ptr(), count, d), f_r(verts.as_ptr(), count, d)) };
            assert_eq!(ic, ir, "c2Support count{count} d={}", vdesc(d));
            assert_eq!(ic, 0, "c2Support count{count}: C sentinel changed");
            assert_eq!(ir, 0, "c2Support count{count}: Rust sentinel differs");
        }
    }

    // row 12: `dot > dmax` never true -> first index wins.
    // (a) all verts identical, (b) NaN direction, (c) NaN verts.
    let same = [c2v { x: 2.5, y: -2.5 }; 8];
    let nanv = [c2v { x: f32::NAN, y: f32::NAN }; 8];
    for count in [1i32, 2, 4, 8] {
        for d in [
            c2v { x: 1.0, y: 0.0 },
            c2v { x: 0.0, y: 0.0 },
            c2v { x: f32::NAN, y: f32::NAN },
            c2v { x: f32::INFINITY, y: f32::NEG_INFINITY },
        ] {
            for (label, vs) in [("same", &same), ("nan", &nanv), ("mixed", &verts)] {
                let (ic, ir) = unsafe { (f_c(vs.as_ptr(), count, d), f_r(vs.as_ptr(), count, d)) };
                assert_eq!(ic, ir, "c2Support {label} count{count} d={}", vdesc(d));
            }
        }
    }
}

// ===========================================================================
// Rows 13, 14, 15: c2Div / c2Norm division by zero and NaN
// ===========================================================================

#[test]
fn row13_row14_row15_div_norm() {
    let (c, r) = load();
    let div_c: libloading::Symbol<FnVvf> = c.sym("c2Div");
    let div_r: libloading::Symbol<FnVvf> = r.sym("c2Div");
    let norm_c: libloading::Symbol<FnVv> = c.sym("c2Norm");
    let norm_r: libloading::Symbol<FnVv> = r.sym("c2Norm");

    let vs = [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: -0.0 },
        c2v { x: 0.0, y: -0.0 },
        c2v { x: 1.0, y: 0.0 },
        c2v { x: -1.0, y: 2.0 },
        c2v { x: f32::MAX, y: f32::MAX },
        c2v { x: f32::MIN_POSITIVE, y: 0.0 },
        c2v { x: f32::from_bits(1), y: f32::from_bits(1) },
        c2v { x: f32::NAN, y: 0.0 },
        c2v { x: f32::INFINITY, y: 1.0 },
        c2v { x: f32::NEG_INFINITY, y: f32::INFINITY },
    ];
    // row 13: b == 0 (and -0, NaN, inf)
    for v in vs {
        for b in [0.0f32, -0.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::MIN_POSITIVE] {
            let (a, bb) = unsafe { (div_c(v, b), div_r(v, b)) };
            assert!(
                veq(a, bb),
                "c2Div v={} b={}: C={} R={}",
                vdesc(v), fdesc(b), vdesc(a), vdesc(bb)
            );
        }
        // rows 14 & 15: zero-length and non-finite normalisation
        let (a, bb) = unsafe { (norm_c(v), norm_r(v)) };
        assert!(veq(a, bb), "c2Norm v={}: C={} R={}", vdesc(v), vdesc(a), vdesc(bb));
    }
    // Row 14 documents the exact result: NaN on both components.
    let z = c2v { x: 0.0, y: 0.0 };
    let nc = unsafe { norm_c(z) };
    let nr = unsafe { norm_r(z) };
    assert!(nc.x.is_nan() && nc.y.is_nan(), "c2Norm(0,0) C is not NaN: {}", vdesc(nc));
    assert!(nr.x.is_nan() && nr.y.is_nan(), "c2Norm(0,0) Rust is not NaN: {}", vdesc(nr));
}

// ===========================================================================
// Rows 16 & 17: NaN ternary semantics and inverted clamp range
// ===========================================================================

#[test]
fn row16_row17_minmax_nan_and_inverted_clamp() {
    let (c, r) = load();
    let max_c: libloading::Symbol<FnVvv> = c.sym("c2Maxv");
    let max_r: libloading::Symbol<FnVvv> = r.sym("c2Maxv");
    let min_c: libloading::Symbol<FnVvv> = c.sym("c2Minv");
    let min_r: libloading::Symbol<FnVvv> = r.sym("c2Minv");
    let cl_c: libloading::Symbol<FnVvvv> = c.sym("c2Clampv");
    let cl_r: libloading::Symbol<FnVvvv> = r.sym("c2Clampv");

    let nan = c2v { x: f32::NAN, y: f32::NAN };
    let one = c2v { x: 1.0, y: 1.0 };

    // Row 16: the C ternary `a>b?a:b` is false for NaN, so `b` is selected.
    unsafe {
        let (mc, mr) = (max_c(nan, one), max_r(nan, one));
        assert!(veq(mc, mr), "c2Maxv(NaN, 1): C={} R={}", vdesc(mc), vdesc(mr));
        assert!(veq_strict(mc, one), "c2Maxv(NaN, 1) must select b: C={}", vdesc(mc));
        assert!(veq_strict(mr, one), "c2Maxv(NaN, 1) must select b: R={}", vdesc(mr));

        let (mc, mr) = (min_c(nan, one), min_r(nan, one));
        assert!(veq(mc, mr), "c2Minv(NaN, 1)");
        assert!(veq_strict(mc, one), "c2Minv(NaN, 1) must select b: C={}", vdesc(mc));
        assert!(veq_strict(mr, one), "c2Minv(NaN, 1) must select b: R={}", vdesc(mr));

        // NaN on the right selects b, i.e. the NaN itself.
        let (mc, mr) = (max_c(one, nan), max_r(one, nan));
        assert!(veq(mc, mr), "c2Maxv(1, NaN)");
        assert!(mc.x.is_nan() && mc.y.is_nan(), "c2Maxv(1, NaN) must select b (NaN)");
        assert!(mr.x.is_nan() && mr.y.is_nan(), "c2Maxv(1, NaN) must select b (NaN) [Rust]");
    }

    // Row 17: inverted range -> the result is `lo`.
    let lo = c2v { x: 10.0, y: 20.0 };
    let hi = c2v { x: -10.0, y: -20.0 };
    let a = c2v { x: 0.0, y: 0.0 };
    unsafe {
        let (cc, cr) = (cl_c(a, lo, hi), cl_r(a, lo, hi));
        assert!(veq(cc, cr), "c2Clampv inverted: C={} R={}", vdesc(cc), vdesc(cr));
        assert!(veq_strict(cc, lo), "c2Clampv inverted must be lo: C={}", vdesc(cc));
        assert!(veq_strict(cr, lo), "c2Clampv inverted must be lo: R={}", vdesc(cr));
        // ±0 and NaN bounds too
        for (l, h) in [
            (c2v { x: 0.0, y: 0.0 }, c2v { x: -0.0, y: -0.0 }),
            (nan, one),
            (one, nan),
            (nan, nan),
        ] {
            assert!(veq(cl_c(a, l, h), cl_r(a, l, h)), "c2Clampv lo={} hi={}", vdesc(l), vdesc(h));
        }
    }
}

// ===========================================================================
// Rows 18–27, 35: c2GJK null-pointer / flag rejections
// ===========================================================================

#[repr(C, align(16))]
struct Blob([u8; 32]);

fn blob_of<T: Copy>(v: &T) -> Blob {
    let mut b = Blob([0u8; 32]);
    unsafe {
        std::ptr::copy_nonoverlapping(
            v as *const T as *const u8,
            b.0.as_mut_ptr(),
            std::mem::size_of::<T>(),
        );
    }
    b
}

#[allow(clippy::too_many_arguments)]
unsafe fn gjk(
    f: &libloading::Symbol<FnGJK>,
    A: &Blob,
    tA: u32,
    ax: *const c2x,
    B: &Blob,
    tB: u32,
    bx: *const c2x,
    outA: *mut c2v,
    outB: *mut c2v,
    use_radius: c_int,
    it: *mut c_int,
    cache: *mut c2GJKCache,
) -> f32 {
    unsafe {
        f(
            A.0.as_ptr() as *const c_void,
            tA,
            ax,
            B.0.as_ptr() as *const c_void,
            tB,
            bx,
            outA,
            outB,
            use_radius,
            it,
            cache,
        )
    }
}

#[test]
fn row18_to_row27_and_row35_gjk_null_and_flags() {
    let (c, r) = load();
    let g_c: libloading::Symbol<FnGJK> = c.sym("c2GJK");
    let g_r: libloading::Symbol<FnGJK> = r.sym("c2GJK");

    let circ = c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 15.0 };
    let cap = c2Capsule { a: c2v { x: 100.0, y: -25.0 }, b: c2v { x: 75.0, y: 100.0 }, r: 10.0 };
    let bA = blob_of(&circ);
    let bB = blob_of(&cap);
    let ident = c2x { p: c2v { x: 0.0, y: 0.0 }, r: c2r { c: 1.0, s: 0.0 } };
    let moved = c2x { p: c2v { x: 12.0, y: -34.0 }, r: c2r { c: 0.6, s: 0.8 } };

    // Rows 18 & 19: NULL transform must be substituted with the identity, i.e.
    // NULL and an explicit identity must give identical results.
    for use_radius in [0i32, 1] {
        unsafe {
            let mut sink = [c2v::default(); 4];
            let mut its = [0 as c_int; 4];
            let d_null_c = gjk(&g_c, &bA, C2_TYPE_CIRCLE, std::ptr::null(), &bB, C2_TYPE_CAPSULE,
                std::ptr::null(), &mut sink[0], &mut sink[1], use_radius, &mut its[0], std::ptr::null_mut());
            let d_id_c = gjk(&g_c, &bA, C2_TYPE_CIRCLE, &ident, &bB, C2_TYPE_CAPSULE,
                &ident, &mut sink[2], &mut sink[3], use_radius, &mut its[1], std::ptr::null_mut());
            assert!(
                feq(d_null_c, d_id_c) && veq(sink[0], sink[2]) && veq(sink[1], sink[3]),
                "C: NULL transform != explicit identity (use_radius={use_radius})"
            );

            let mut sinkr = [c2v::default(); 4];
            let d_null_r = gjk(&g_r, &bA, C2_TYPE_CIRCLE, std::ptr::null(), &bB, C2_TYPE_CAPSULE,
                std::ptr::null(), &mut sinkr[0], &mut sinkr[1], use_radius, &mut its[2], std::ptr::null_mut());
            let d_id_r = gjk(&g_r, &bA, C2_TYPE_CIRCLE, &ident, &bB, C2_TYPE_CAPSULE,
                &ident, &mut sinkr[2], &mut sinkr[3], use_radius, &mut its[3], std::ptr::null_mut());
            assert!(
                feq(d_null_c, d_null_r) && veq(sink[0], sinkr[0]) && veq(sink[1], sinkr[1]) && its[0] == its[2],
                "row18/19: NULL-transform result differs (use_radius={use_radius}): C={} R={}",
                fdesc(d_null_c), fdesc(d_null_r)
            );
            assert!(feq(d_id_c, d_id_r) && its[1] == its[3], "row18/19: identity result differs");

            // one NULL, one non-NULL, both directions
            for (ax, bx) in [
                (std::ptr::null(), &moved as *const c2x),
                (&moved as *const c2x, std::ptr::null()),
            ] {
                let (mut a1, mut b1, mut i1) = (c2v::default(), c2v::default(), 0 as c_int);
                let (mut a2, mut b2, mut i2) = (c2v::default(), c2v::default(), 0 as c_int);
                let dc = gjk(&g_c, &bA, C2_TYPE_CIRCLE, ax, &bB, C2_TYPE_CAPSULE, bx,
                    &mut a1, &mut b1, use_radius, &mut i1, std::ptr::null_mut());
                let dr = gjk(&g_r, &bA, C2_TYPE_CIRCLE, ax, &bB, C2_TYPE_CAPSULE, bx,
                    &mut a2, &mut b2, use_radius, &mut i2, std::ptr::null_mut());
                assert!(
                    feq(dc, dr) && veq(a1, a2) && veq(b1, b2) && i1 == i2,
                    "row18/19 mixed NULL: C={} R={}", fdesc(dc), fdesc(dr)
                );
            }
        }
    }

    // Rows 20, 25, 26, 27: NULL cache / outA / outB / iterations must all be
    // silently skipped, and the sentinels left untouched.
    const SA: c2v = c2v { x: 1111.5, y: -2222.5 };
    const SB: c2v = c2v { x: -3333.5, y: 4444.5 };
    const SI: c_int = -0x5EED;
    for mask in 0..16u32 {
        let want_a = mask & 1 != 0;
        let want_b = mask & 2 != 0;
        let want_i = mask & 4 != 0;
        let want_c = mask & 8 != 0;
        for use_radius in [0i32, 1] {
            let mut ac = SA;
            let mut bc = SB;
            let mut ic = SI;
            let mut cc = c2GJKCache { metric: -7.5, count: 0, iA: [9; 3], iB: [-9; 3], div: -7.5 };
            let mut ar = SA;
            let mut br = SB;
            let mut ir = SI;
            let mut cr = cc;
            unsafe {
                let dc = gjk(
                    &g_c, &bA, C2_TYPE_CIRCLE, std::ptr::null(), &bB, C2_TYPE_CAPSULE, std::ptr::null(),
                    if want_a { &mut ac } else { std::ptr::null_mut() },
                    if want_b { &mut bc } else { std::ptr::null_mut() },
                    use_radius,
                    if want_i { &mut ic } else { std::ptr::null_mut() },
                    if want_c { &mut cc } else { std::ptr::null_mut() },
                );
                let dr = gjk(
                    &g_r, &bA, C2_TYPE_CIRCLE, std::ptr::null(), &bB, C2_TYPE_CAPSULE, std::ptr::null(),
                    if want_a { &mut ar } else { std::ptr::null_mut() },
                    if want_b { &mut br } else { std::ptr::null_mut() },
                    use_radius,
                    if want_i { &mut ir } else { std::ptr::null_mut() },
                    if want_c { &mut cr } else { std::ptr::null_mut() },
                );
                assert!(feq(dc, dr), "rows20/25-27 mask{mask}: dist C={} R={}", fdesc(dc), fdesc(dr));
            }
            assert!(veq(ac, ar), "mask{mask}: outA C={} R={}", vdesc(ac), vdesc(ar));
            assert!(veq(bc, br), "mask{mask}: outB");
            assert_eq!(ic, ir, "mask{mask}: iterations");
            assert!(cache_eq(&cc, &cr), "mask{mask}: cache C={cc:?} R={cr:?}");
            if !want_a {
                assert!(veq_strict(ac, SA) && veq_strict(ar, SA), "mask{mask}: NULL outA written");
            }
            if !want_b {
                assert!(veq_strict(bc, SB) && veq_strict(br, SB), "mask{mask}: NULL outB written");
            }
            if !want_i {
                assert_eq!(ic, SI, "mask{mask}: NULL iterations written (C)");
                assert_eq!(ir, SI, "mask{mask}: NULL iterations written (Rust)");
            }
            if !want_c {
                assert_eq!(cc.metric.to_bits(), (-7.5f32).to_bits(), "mask{mask}: NULL cache written (C)");
                assert_eq!(cr.metric.to_bits(), (-7.5f32).to_bits(), "mask{mask}: NULL cache written (Rust)");
            }
        }
    }
}

// ===========================================================================
// Rows 21, 22, 23, 24: cache acceptance / rejection
// ===========================================================================

#[test]
fn row21_to_row24_cache_accept_reject() {
    let (c, r) = load();
    let g_c: libloading::Symbol<FnGJK> = c.sym("c2GJK");
    let g_r: libloading::Symbol<FnGJK> = r.sym("c2GJK");

    let bbx = c2AABB { min: c2v { x: -20.0, y: -20.0 }, max: c2v { x: 20.0, y: 20.0 } };
    let cap = c2Capsule { a: c2v { x: 60.0, y: -10.0 }, b: c2v { x: 90.0, y: 40.0 }, r: 6.0 };
    let bA = blob_of(&bbx);
    let bB = blob_of(&cap);

    // Row 21: count == 0 -> cache rejected. The result must equal a NULL-cache run.
    unsafe {
        let mut fresh = c2GJKCache { metric: 12345.0, count: 0, iA: [2, 2, 2], iB: [1, 1, 1], div: 999.0 };
        let (mut a1, mut b1, mut i1) = (c2v::default(), c2v::default(), 0 as c_int);
        let d_zero = gjk(&g_c, &bA, C2_TYPE_AABB, std::ptr::null(), &bB, C2_TYPE_CAPSULE,
            std::ptr::null(), &mut a1, &mut b1, 1, &mut i1, &mut fresh);
        let (mut a2, mut b2, mut i2) = (c2v::default(), c2v::default(), 0 as c_int);
        let d_null = gjk(&g_c, &bA, C2_TYPE_AABB, std::ptr::null(), &bB, C2_TYPE_CAPSULE,
            std::ptr::null(), &mut a2, &mut b2, 1, &mut i2, std::ptr::null_mut());
        assert!(
            feq(d_zero, d_null) && veq(a1, a2) && i1 == i2,
            "row21: count==0 must behave like a fresh simplex"
        );
    }

    // Rows 21–23: sweep metric/div/count to drive both the accept and the
    // reject arm of `!(min_metric < max_metric*2.0f && metric < -1.0e8f)`.
    let metrics: [f32; 12] = [
        0.0, -0.0, 1.0, -1.0, 1e7, -1e7, -1e8, -1.0000001e8, -1e9, -1e30,
        f32::NAN, f32::NEG_INFINITY,
    ];
    let divs: [f32; 5] = [1.0, 0.0, -1.0, 1e6, f32::NAN];
    let mut accept = 0usize;
    let mut reject = 0usize;
    for count in [1i32, 2, 3] {
        for &metric in &metrics {
            for &div in &divs {
                let base = c2GJKCache { metric, count, iA: [0, 1, 2], iB: [0, 1, 0], div };
                for use_radius in [0i32, 1] {
                    let mut cc = base;
                    let mut cr = base;
                    let (mut ac, mut bc, mut ic) = (c2v::default(), c2v::default(), 0 as c_int);
                    let (mut ar, mut br, mut ir) = (c2v::default(), c2v::default(), 0 as c_int);
                    unsafe {
                        let dc = gjk(&g_c, &bA, C2_TYPE_AABB, std::ptr::null(), &bB,
                            C2_TYPE_CAPSULE, std::ptr::null(), &mut ac, &mut bc, use_radius,
                            &mut ic, &mut cc);
                        let dr = gjk(&g_r, &bA, C2_TYPE_AABB, std::ptr::null(), &bB,
                            C2_TYPE_CAPSULE, std::ptr::null(), &mut ar, &mut br, use_radius,
                            &mut ir, &mut cr);
                        assert!(
                            feq(dc, dr),
                            "rows21-23 count{count} metric={} div={} ur{use_radius}: dist C={} R={}",
                            fdesc(metric), fdesc(div), fdesc(dc), fdesc(dr)
                        );
                    }
                    assert!(veq(ac, ar) && veq(bc, br), "rows21-23: witness points differ");
                    assert_eq!(ic, ir, "rows21-23: iterations differ");
                    assert!(cache_eq(&cc, &cr), "rows21-23: cache out C={cc:?} R={cr:?}");
                    // Classify which arm the C took: metric < -1e8 is the
                    // necessary condition for the reject arm.
                    if metric < -1.0e8 {
                        reject += 1;
                    } else {
                        accept += 1;
                    }
                }
            }
        }
    }
    assert!(accept > 0 && reject > 0, "rows22/23: both cache arms must be exercised ({accept}/{reject})");

    // Row 24: cache->count < 0. `!!count` is true so the cache is "good", but
    // the read loop never runs, leaving s.count negative -> every switch takes
    // its default arm -> dist 0, a == b == (0,0), iter 0.
    for count in [-1i32, -3, i32::MIN] {
        for use_radius in [0i32, 1] {
            let base = c2GJKCache { metric: 1.0, count, iA: [0; 3], iB: [0; 3], div: 1.0 };
            let mut cc = base;
            let mut cr = base;
            let (mut ac, mut bc, mut ic) = (c2v { x: 8.0, y: 8.0 }, c2v { x: 8.0, y: 8.0 }, -5 as c_int);
            let (mut ar, mut br, mut ir) = (ac, bc, ic);
            unsafe {
                let dc = gjk(&g_c, &bA, C2_TYPE_AABB, std::ptr::null(), &bB, C2_TYPE_CAPSULE,
                    std::ptr::null(), &mut ac, &mut bc, use_radius, &mut ic, &mut cc);
                let dr = gjk(&g_r, &bA, C2_TYPE_AABB, std::ptr::null(), &bB, C2_TYPE_CAPSULE,
                    std::ptr::null(), &mut ar, &mut br, use_radius, &mut ir, &mut cr);
                assert!(
                    feq(dc, dr),
                    "row24 count{count} ur{use_radius}: dist C={} R={}",
                    fdesc(dc), fdesc(dr)
                );
            }
            assert!(veq(ac, ar) && veq(bc, br), "row24 count{count}: witness C=({},{}) R=({},{})",
                vdesc(ac), vdesc(bc), vdesc(ar), vdesc(br));
            assert_eq!(ic, ir, "row24 count{count}: iterations");
            assert!(cache_eq(&cc, &cr), "row24 count{count}: cache C={cc:?} R={cr:?}");
            assert_eq!(ic, 0, "row24: C iterations sentinel changed");
            assert_eq!(cc.count, count, "row24: C writes the negative count straight back");
        }
    }
}

// ===========================================================================
// Rows 28–34: terminal branches of c2GJK, with explicit branch classification.
// ===========================================================================

#[test]
fn row28_to_row34_gjk_terminal_branches() {
    let (c, r) = load();
    let g_c: libloading::Symbol<FnGJK> = c.sym("c2GJK");
    let g_r: libloading::Symbol<FnGJK> = r.sym("c2GJK");

    let mut hit_branch = 0usize; // row 28: dist 0 and a == b via hit
    let mut collapse = 0usize; // row 33: midpoint collapse
    let mut shrink = 0usize; // rows 33/34: full radius shrink applied
    let mut no_radius_pos = 0usize; // row 35: use_radius == 0 keeps the core distance
    let mut cap20 = 0usize; // row 32: iteration cap
    let mut iters_seen = std::collections::BTreeSet::new();

    let mut rng = Rng::new(SEED ^ 0x28);
    for i in 0..4000usize {
        // Concentric-ish sweep so every terminal branch is reached.
        let sep = (i % 200) as f32 * 0.75;
        let ang = rng.unit() * std::f32::consts::TAU;
        let ctr = c2v { x: sep * ang.cos(), y: sep * ang.sin() };
        let (tA, tB) = ([C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE][i % 3],
                        [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE][(i / 3) % 3]);
        let bA = match tA {
            C2_TYPE_CIRCLE => blob_of(&c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 15.0 }),
            C2_TYPE_AABB => blob_of(&c2AABB {
                min: c2v { x: -10.0, y: -10.0 },
                max: c2v { x: 10.0, y: 10.0 },
            }),
            _ => blob_of(&c2Capsule {
                a: c2v { x: -12.0, y: 0.0 },
                b: c2v { x: 12.0, y: 0.0 },
                r: 5.0,
            }),
        };
        let bB = match tB {
            C2_TYPE_CIRCLE => blob_of(&c2Circle { p: ctr, r: 8.0 }),
            C2_TYPE_AABB => blob_of(&c2AABB {
                min: c2v { x: ctr.x - 7.0, y: ctr.y - 7.0 },
                max: c2v { x: ctr.x + 7.0, y: ctr.y + 7.0 },
            }),
            _ => blob_of(&c2Capsule {
                a: c2v { x: ctr.x - 9.0, y: ctr.y - 3.0 },
                b: c2v { x: ctr.x + 9.0, y: ctr.y + 3.0 },
                r: 4.0,
            }),
        };
        for use_radius in [0i32, 1] {
            let (mut ac, mut bc, mut ic) = (c2v::default(), c2v::default(), 0 as c_int);
            let (mut ar, mut br, mut ir) = (c2v::default(), c2v::default(), 0 as c_int);
            unsafe {
                let dc = gjk(&g_c, &bA, tA, std::ptr::null(), &bB, tB, std::ptr::null(),
                    &mut ac, &mut bc, use_radius, &mut ic, std::ptr::null_mut());
                let dr = gjk(&g_r, &bA, tA, std::ptr::null(), &bB, tB, std::ptr::null(),
                    &mut ar, &mut br, use_radius, &mut ir, std::ptr::null_mut());
                assert!(
                    feq(dc, dr),
                    "rows28-34 i{i} tA{tA} tB{tB} sep{sep} ur{use_radius}: dist C={} R={}",
                    fdesc(dc), fdesc(dr)
                );
                assert!(veq(ac, ar) && veq(bc, br),
                    "rows28-34 i{i} tA{tA} tB{tB} sep{sep} ur{use_radius}: witness C=({},{}) R=({},{})",
                    vdesc(ac), vdesc(bc), vdesc(ar), vdesc(br));
                assert_eq!(ic, ir, "rows28-34 i{i}: iterations");
                iters_seen.insert(ic);
                if ic == 20 {
                    cap20 += 1;
                }
                if dc == 0.0 {
                    if veq_strict(ac, bc) {
                        collapse += 1;
                    } else {
                        hit_branch += 1;
                    }
                } else if use_radius == 1 {
                    shrink += 1;
                } else {
                    no_radius_pos += 1;
                }
            }
        }
    }
    assert!(collapse > 10, "row33: midpoint collapse under-covered ({collapse})");
    assert!(shrink > 10, "rows33/34: radius shrink under-covered ({shrink})");
    assert!(no_radius_pos > 10, "row35: use_radius==0 path under-covered ({no_radius_pos})");
    assert!(iters_seen.len() >= 3, "rows29-32: loop exits under-covered ({iters_seen:?})");
    let _ = (hit_branch, cap20);
}

// ===========================================================================
// Row 38: non-finite shape coordinates (no validation anywhere)
// ===========================================================================

#[test]
fn row38_nonfinite_shape_data() {
    let (c, r) = load();
    let g_c: libloading::Symbol<FnGJK> = c.sym("c2GJK");
    let g_r: libloading::Symbol<FnGJK> = r.sym("c2GJK");

    let bad: [f32; 8] = [
        f32::NAN, -f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::MAX, -f32::MAX,
        f32::MIN_POSITIVE, f32::from_bits(1),
    ];
    let mut rng = Rng::new(SEED ^ 0x38);
    for i in 0..2000usize {
        let pick = |rng: &mut Rng| -> f32 {
            if rng.below(2) == 0 { bad[rng.below(8) as usize] } else { rng.sym(50.0) }
        };
        let tA = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE][i % 3];
        let tB = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE][(i / 3) % 3];
        let mk = |rng: &mut Rng, ty: u32| -> Blob {
            match ty {
                C2_TYPE_CIRCLE => {
                    let (x, y, r_) = (pick(rng), pick(rng), pick(rng));
                    blob_of(&c2Circle { p: c2v { x, y }, r: r_ })
                }
                C2_TYPE_AABB => {
                    let (a, b, cc, d) = (pick(rng), pick(rng), pick(rng), pick(rng));
                    blob_of(&c2AABB { min: c2v { x: a, y: b }, max: c2v { x: cc, y: d } })
                }
                _ => {
                    let (a, b, cc, d, e) = (pick(rng), pick(rng), pick(rng), pick(rng), pick(rng));
                    blob_of(&c2Capsule {
                        a: c2v { x: a, y: b },
                        b: c2v { x: cc, y: d },
                        r: e,
                    })
                }
            }
        };
        let bA = mk(&mut rng, tA);
        let bB = mk(&mut rng, tB);
        for use_radius in [0i32, 1] {
            let (mut ac, mut bc, mut ic) = (c2v::default(), c2v::default(), 0 as c_int);
            let (mut ar, mut br, mut ir) = (c2v::default(), c2v::default(), 0 as c_int);
            let mut cc = c2GJKCache { metric: 0.0, count: 0, iA: [0; 3], iB: [0; 3], div: 0.0 };
            let mut cr = cc;
            unsafe {
                let dc = gjk(&g_c, &bA, tA, std::ptr::null(), &bB, tB, std::ptr::null(),
                    &mut ac, &mut bc, use_radius, &mut ic, &mut cc);
                let dr = gjk(&g_r, &bA, tA, std::ptr::null(), &bB, tB, std::ptr::null(),
                    &mut ar, &mut br, use_radius, &mut ir, &mut cr);
                assert!(feq(dc, dr), "row38 i{i}: dist C={} R={}", fdesc(dc), fdesc(dr));
            }
            assert!(veq(ac, ar) && veq(bc, br),
                "row38 i{i} tA{tA} tB{tB}: witness C=({},{}) R=({},{})",
                vdesc(ac), vdesc(bc), vdesc(ar), vdesc(br));
            assert_eq!(ic, ir, "row38 i{i}: iterations");
            assert!(cache_eq(&cc, &cr), "row38 i{i}: cache C={cc:?} R={cr:?}");
        }
    }
}

// ===========================================================================
// Rows 39 & 40: c22 rejection arms (with the exact arm asserted)
// ===========================================================================

#[test]
fn row39_row40_c22_arms() {
    let (c, r) = load();
    let f_c: libloading::Symbol<FnSimplex> = c.sym("c22");
    let f_r: libloading::Symbol<FnSimplex> = r.sym("c22");

    // (a.p, b.p, expected resulting count, label)
    let cases: [(c2v, c2v, c_int, &str); 8] = [
        // v = Dot(a, a-b) <= 0  ->  keep a, count 1
        (c2v { x: 1.0, y: 0.0 }, c2v { x: 3.0, y: 0.0 }, 1, "row39 v<=0"),
        (c2v { x: 0.0, y: 0.0 }, c2v { x: 1.0, y: 1.0 }, 1, "row39 v==0 (a at origin)"),
        // u = Dot(b, b-a) <= 0  ->  a = b, count 1
        (c2v { x: 3.0, y: 0.0 }, c2v { x: 1.0, y: 0.0 }, 1, "row40 u<=0"),
        (c2v { x: 1.0, y: 1.0 }, c2v { x: 0.0, y: 0.0 }, 1, "row40 u==0 (b at origin)"),
        // interior -> count 2
        (c2v { x: -1.0, y: 1.0 }, c2v { x: 1.0, y: 1.0 }, 2, "interior"),
        // degenerate a == b -> u = v = 0 -> first arm (v <= 0)
        (c2v { x: 2.0, y: 2.0 }, c2v { x: 2.0, y: 2.0 }, 1, "row39 a==b"),
        // NaN -> every `<= 0` is false -> the else arm (count 2)
        (c2v { x: f32::NAN, y: 0.0 }, c2v { x: 1.0, y: 0.0 }, 2, "row39 NaN -> else"),
        (c2v { x: 1.0, y: 0.0 }, c2v { x: f32::NAN, y: 0.0 }, 2, "row40 NaN -> else"),
    ];
    for (i, (ap, bp, want, label)) in cases.iter().enumerate() {
        let mut sc = mk_simplex(2, 0.0);
        sc.verts[0].p = *ap;
        sc.verts[1].p = *bp;
        let mut sr = sc;
        unsafe {
            f_c(&mut sc);
            f_r(&mut sr);
        }
        assert!(simplex_eq(&sc, &sr), "c22 {label} (case {i}): C={sc:?} R={sr:?}");
        assert_eq!(
            sc.count, *want,
            "c22 {label} (case {i}): C took an unexpected arm (count {})",
            sc.count
        );
    }
}

// ===========================================================================
// Rows 41 & 42: c23 rejection arms — all seven, plus degenerate area == 0
// ===========================================================================

#[test]
fn row41_row42_c23_arms() {
    let (c, r) = load();
    let f_c: libloading::Symbol<FnSimplex> = c.sym("c23");
    let f_r: libloading::Symbol<FnSimplex> = r.sym("c23");

    // Hand-built triangles that select each of the seven arms.
    let cases: [([c2v; 3], &str); 10] = [
        // arm 1: vAB <= 0 && uCA <= 0  -> vertex A
        ([c2v { x: 1.0, y: 0.0 }, c2v { x: 4.0, y: 1.0 }, c2v { x: 4.0, y: -1.0 }], "arm1 vertex A"),
        // arm 2: uAB <= 0 && vBC <= 0  -> vertex B
        ([c2v { x: 4.0, y: 1.0 }, c2v { x: 1.0, y: 0.0 }, c2v { x: 4.0, y: -1.0 }], "arm2 vertex B"),
        // arm 3: uBC <= 0 && vCA <= 0  -> vertex C
        ([c2v { x: 4.0, y: 1.0 }, c2v { x: 4.0, y: -1.0 }, c2v { x: 1.0, y: 0.0 }], "arm3 vertex C"),
        // edge/interior variants
        ([c2v { x: -1.0, y: 1.0 }, c2v { x: 1.0, y: 1.0 }, c2v { x: 0.0, y: 4.0 }], "edge AB"),
        ([c2v { x: 0.0, y: 4.0 }, c2v { x: -1.0, y: 1.0 }, c2v { x: 1.0, y: 1.0 }], "edge BC"),
        ([c2v { x: 1.0, y: 1.0 }, c2v { x: 0.0, y: 4.0 }, c2v { x: -1.0, y: 1.0 }], "edge CA"),
        // origin inside the triangle -> interior arm, count 3
        ([c2v { x: -1.0, y: -1.0 }, c2v { x: 2.0, y: -1.0 }, c2v { x: 0.0, y: 2.0 }], "interior"),
        // row 42: collinear -> area == 0 -> all three *ABC are 0
        ([c2v { x: 1.0, y: 1.0 }, c2v { x: 2.0, y: 2.0 }, c2v { x: 3.0, y: 3.0 }], "row42 collinear"),
        // row 42: all three identical
        ([c2v { x: 2.0, y: -3.0 }; 3], "row42 all identical"),
        // row 42: two identical
        ([c2v { x: 1.0, y: 0.0 }, c2v { x: 1.0, y: 0.0 }, c2v { x: 0.0, y: 1.0 }], "row42 two identical"),
    ];
    let mut counts = [0usize; 4];
    for (i, (ps, label)) in cases.iter().enumerate() {
        let mut sc = mk_simplex(3, 0.0);
        for k in 0..3 {
            sc.verts[k].p = ps[k];
        }
        let mut sr = sc;
        unsafe {
            f_c(&mut sc);
            f_r(&mut sr);
        }
        assert!(simplex_eq(&sc, &sr), "c23 {label} (case {i}): C={sc:?} R={sr:?}");
        if (1..=3).contains(&sc.count) {
            counts[sc.count as usize] += 1;
        }
    }
    assert!(
        counts[1] > 0 && counts[2] > 0 && counts[3] > 0,
        "c23: all three outcome arities must be reached: {counts:?}"
    );

    // Also NaN vertices: every `<= 0` test is false -> the interior arm.
    for k in 0..3 {
        let mut sc = mk_simplex(3, 0.0);
        sc.verts[0].p = c2v { x: 1.0, y: 0.0 };
        sc.verts[1].p = c2v { x: 0.0, y: 1.0 };
        sc.verts[2].p = c2v { x: -1.0, y: -1.0 };
        sc.verts[k].p = c2v { x: f32::NAN, y: f32::NAN };
        let mut sr = sc;
        unsafe {
            f_c(&mut sc);
            f_r(&mut sr);
        }
        assert!(simplex_eq(&sc, &sr), "c23 NaN vert{k}: C={sc:?} R={sr:?}");
    }
}

// ===========================================================================
// Row 43: c2BBVerts with an inverted / degenerate / non-finite AABB
// ===========================================================================

#[test]
fn row43_bbverts_inverted() {
    let (c, r) = load();
    let f_c: libloading::Symbol<FnBBVerts> = c.sym("c2BBVerts");
    let f_r: libloading::Symbol<FnBBVerts> = r.sym("c2BBVerts");

    let boxes = [
        c2AABB { min: c2v { x: 10.0, y: 20.0 }, max: c2v { x: -10.0, y: -20.0 } },
        c2AABB { min: c2v { x: 5.0, y: 5.0 }, max: c2v { x: 5.0, y: 5.0 } },
        c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: -0.0, y: -0.0 } },
        c2AABB { min: c2v { x: f32::NAN, y: 1.0 }, max: c2v { x: 2.0, y: f32::NAN } },
        c2AABB {
            min: c2v { x: f32::INFINITY, y: f32::NEG_INFINITY },
            max: c2v { x: f32::NEG_INFINITY, y: f32::INFINITY },
        },
        c2AABB { min: c2v { x: f32::MAX, y: -f32::MAX }, max: c2v { x: -f32::MAX, y: f32::MAX } },
    ];
    for (i, bb) in boxes.iter().enumerate() {
        let mut b1 = *bb;
        let mut b2 = *bb;
        let mut oc = [c2v { x: -77.0, y: 77.0 }; 4];
        let mut or_ = oc;
        unsafe {
            f_c(oc.as_mut_ptr(), &mut b1);
            f_r(or_.as_mut_ptr(), &mut b2);
        }
        for k in 0..4 {
            assert!(
                veq(oc[k], or_[k]),
                "c2BBVerts box{i} vert{k}: C={} R={}",
                vdesc(oc[k]), vdesc(or_[k])
            );
        }
        // The C never reorders an inverted box; the corners come out verbatim.
        assert!(veq(oc[0], bb.min) && veq(oc[2], bb.max), "c2BBVerts box{i}: C reordered");
        assert!(veq(or_[0], bb.min) && veq(or_[2], bb.max), "c2BBVerts box{i}: Rust reordered");
        // The input struct must not be mutated by either side.
        assert!(
            veq_strict(b1.min, b2.min) && veq_strict(b1.max, b2.max),
            "c2BBVerts box{i}: input mutated differently"
        );
    }
}

// ===========================================================================
// Rows 44 & 45: gjk_cache — unused pointer params and every `reverse` value
// ===========================================================================

#[test]
fn row44_row45_gjk_cache_params() {
    let (c, r) = load();
    let f_c: libloading::Symbol<FnGjkCache> = c.sym("gjk_cache");
    let f_r: libloading::Symbol<FnGjkCache> = r.sym("gjk_cache");

    // Row 45: exercise the whole `char` range for `reverse`.
    let mut revs: Vec<c_char> = (i8::MIN..=i8::MAX).step_by(7).map(|v| v as c_char).collect();
    revs.extend_from_slice(&[0, 1, -1, 2, 127, -128]);
    assert!(revs.contains(&0), "reverse == 0 must be covered");
    assert!(revs.iter().any(|&v| v != 0), "reverse != 0 must be covered");

    const SA: c2v = c2v { x: 101.25, y: -202.5 };
    const SB: c2v = c2v { x: -303.75, y: 404.0 };

    let mut rng = Rng::new(SEED ^ 0x44);
    for (i, &rev) in revs.iter().enumerate() {
        for kind in 0..4 {
            let (a1, a2, a3, a4, b1, b2, b3, b4, b5) = match kind {
                0 => (-20.0, -20.0, 20.0, 20.0, 60.0, -10.0, 90.0, 40.0, 6.0),
                1 => (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
                2 => (
                    f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -f32::NAN,
                    f32::MAX, -f32::MAX, f32::MIN_POSITIVE, f32::from_bits(1), f32::NAN,
                ),
                _ => (
                    rng.sym(500.0), rng.sym(500.0), rng.sym(500.0), rng.sym(500.0),
                    rng.sym(500.0), rng.sym(500.0), rng.sym(500.0), rng.sym(500.0),
                    rng.sym(100.0),
                ),
            };
            // Row 44: with real buffers, with NULL, and with a deliberately
            // misaligned/dangling-looking value — the C never dereferences them.
            let (mut ac, mut bc) = (SA, SB);
            let (mut ar, mut br) = (SA, SB);
            unsafe {
                f_c(rev, &mut ac, &mut bc, a1, a2, a3, a4, b1, b2, b3, b4, b5);
                f_r(rev, &mut ar, &mut br, a1, a2, a3, a4, b1, b2, b3, b4, b5);
                f_c(rev, std::ptr::null_mut(), std::ptr::null_mut(), a1, a2, a3, a4, b1, b2, b3, b4, b5);
                f_r(rev, std::ptr::null_mut(), std::ptr::null_mut(), a1, a2, a3, a4, b1, b2, b3, b4, b5);
                f_c(rev, 1usize as *mut c2v, 3usize as *mut c2v, a1, a2, a3, a4, b1, b2, b3, b4, b5);
                f_r(rev, 1usize as *mut c2v, 3usize as *mut c2v, a1, a2, a3, a4, b1, b2, b3, b4, b5);
            }
            assert!(
                veq_strict(ac, ar) && veq_strict(bc, br),
                "gjk_cache rev{rev} kind{kind} (i{i}): C=({},{}) R=({},{})",
                vdesc(ac), vdesc(bc), vdesc(ar), vdesc(br)
            );
            assert!(
                veq_strict(ac, SA) && veq_strict(bc, SB),
                "gjk_cache rev{rev} kind{kind}: C dereferenced a9/b9"
            );
            assert!(
                veq_strict(ar, SA) && veq_strict(br, SB),
                "gjk_cache rev{rev} kind{kind}: Rust dereferenced a9/b9"
            );
        }
    }
}

// ===========================================================================
// Row 22 (targeted): drive the *computed* simplex metric across the -1.0e8f
// floor and the *2.0f ratio, so the cache-reject arm actually fires.
//
// The C tests `metric`, the metric recomputed from the cache-seeded simplex —
// NOT `cache->metric`. With `count == 3` that metric is
// `Det2(p1-p0, p2-p0)`, so it only reaches -1e8 for large coordinates.
// Seeding an AABB with `iA = [0,2,1]` (a reversed winding) and `iB = [0,0,0]`
// makes the metric exactly `-w*h`, which lets the threshold be swept directly.
// ===========================================================================

#[test]
fn row22_cache_reject_metric_floor_and_ratio() {
    let (c, r) = load();
    let g_c: libloading::Symbol<FnGJK> = c.sym("c2GJK");
    let g_r: libloading::Symbol<FnGJK> = r.sym("c2GJK");
    let metric_c: libloading::Symbol<FnSimplexF> = c.sym("c2GJKSimplexMetric");

    // Straddle -1.0e8f from far above to far below, densely near the boundary.
    let areas: [f32; 22] = [
        1.0e5, 1.0e6, 5.0e6, 9.9e6, 1.0e7, 1.01e7, 5.0e7, 9.0e7, 9.9e7, 9.99e7,
        9.9999e7, 1.0e8, 1.0000001e8, 1.00001e8, 1.001e8, 1.01e8, 1.1e8, 2.0e8,
        1.0e9, 1.0e10, 1.0e12, 1.0e15,
    ];
    // metric_old values relative to the computed metric, to move the
    // `min_metric < max_metric * 2.0f` test across its boundary.
    let old_factors: [f32; 12] = [
        0.0, -0.0, 1.0, -1.0, 0.4, 0.5, 0.6, 0.75, 1.5, 2.0, 3.0, -2.0,
    ];

    let mut rejected = 0usize;
    let mut accepted = 0usize;

    for &area in &areas {
        let side = area.sqrt();
        let bbx = c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: side, y: side } };
        // B is deliberately small and offset so the solver still has work to do.
        let cap = c2Capsule {
            a: c2v { x: side * 2.0, y: -side },
            b: c2v { x: side * 2.5, y: side },
            r: side * 0.05,
        };
        let bA = blob_of(&bbx);
        let bB = blob_of(&cap);

        // Reproduce the metric the C will compute for this seeded simplex.
        let mut probe = c2Simplex::default();
        let av = [
            bbx.min,
            c2v { x: bbx.max.x, y: bbx.min.y },
            bbx.max,
            c2v { x: bbx.min.x, y: bbx.max.y },
        ];
        let bv = [cap.a, cap.b];
        for (k, &ia) in [0usize, 2, 1].iter().enumerate() {
            probe.verts[k].p = c2v { x: bv[0].x - av[ia].x, y: bv[0].y - av[ia].y };
        }
        probe.count = 3;
        let computed = unsafe { metric_c(&mut probe) };

        for &f in &old_factors {
            let metric_old = computed * f;
            let base = c2GJKCache {
                metric: metric_old,
                count: 3,
                iA: [0, 2, 1],
                iB: [0, 0, 0],
                div: 1.0,
            };
            for use_radius in [0i32, 1] {
                let mut cc = base;
                let mut cr = base;
                let (mut ac, mut bc, mut ic) = (c2v::default(), c2v::default(), 0 as c_int);
                let (mut ar, mut br, mut ir) = (c2v::default(), c2v::default(), 0 as c_int);
                unsafe {
                    let dc = gjk(&g_c, &bA, C2_TYPE_AABB, std::ptr::null(), &bB,
                        C2_TYPE_CAPSULE, std::ptr::null(), &mut ac, &mut bc, use_radius,
                        &mut ic, &mut cc);
                    let dr = gjk(&g_r, &bA, C2_TYPE_AABB, std::ptr::null(), &bB,
                        C2_TYPE_CAPSULE, std::ptr::null(), &mut ar, &mut br, use_radius,
                        &mut ir, &mut cr);
                    assert!(
                        feq(dc, dr),
                        "row22 area={} computed_metric={} f={} ur{use_radius}: dist C={} R={}",
                        fdesc(area), fdesc(computed), fdesc(f), fdesc(dc), fdesc(dr)
                    );
                }
                assert!(
                    veq(ac, ar) && veq(bc, br),
                    "row22 area={} computed_metric={} f={}: witness C=({},{}) R=({},{})",
                    fdesc(area), fdesc(computed), fdesc(f), vdesc(ac), vdesc(bc), vdesc(ar), vdesc(br)
                );
                assert_eq!(ic, ir, "row22 area={} f={}: iterations", fdesc(area), fdesc(f));
                assert!(
                    cache_eq(&cc, &cr),
                    "row22 area={} computed_metric={} f={}: cache C={cc:?} R={cr:?}",
                    fdesc(area), fdesc(computed), fdesc(f)
                );

                // Replicate the C predicate to classify the arm actually taken.
                let min_metric = if computed < metric_old { computed } else { metric_old };
                let max_metric = if computed > metric_old { computed } else { metric_old };
                if min_metric < max_metric * 2.0 && computed < -1.0e8 {
                    rejected += 1;
                } else {
                    accepted += 1;
                }
            }
        }
    }
    assert!(
        rejected > 20,
        "row22: the cache-REJECT arm (metric < -1.0e8f) was not exercised enough ({rejected})"
    );
    assert!(
        accepted > 20,
        "row22: the cache-ACCEPT arm was not exercised enough ({accepted})"
    );
}

// ===========================================================================
// Row 37 / Row 36 (documented UB, asserted only where it is well-defined)
//
// `cache->count > 3` makes the C read `cache->iA[i]` past the `[3]` array, write
// `saveA[i]` past its `[3]`, and write `verts[i]` past the four `c2sv` slots of
// `c2Simplex` — i.e. out-of-bounds stack access whose result depends on the
// compiler's frame layout. `typeA`/`typeB` out of enum range leaves `c2Proxy`
// entirely uninitialised inside `c2GJK` (ERRORS.md row 1), so the C then reads
// uninitialised stack. Neither can be pinned to a defined value, so they are
// documented rather than asserted; the *defined* half of row 36 (a direct
// `c2MakeProxy` call with a bad enum) is asserted in
// `row01_make_proxy_out_of_range_enum`, and the defined half of row 37
// (`count` in 1..=3, and `count < 0`) in `row21_to_row24_cache_accept_reject`.
// ===========================================================================

#[test]
fn row36_row37_documented_ub_boundaries_are_covered_elsewhere() {
    // The in-range boundary values are the ones that must agree; assert that the
    // largest defined cache count (3) and the smallest (1) both round-trip.
    let (c, r) = load();
    let g_c: libloading::Symbol<FnGJK> = c.sym("c2GJK");
    let g_r: libloading::Symbol<FnGJK> = r.sym("c2GJK");

    let bbx = c2AABB { min: c2v { x: -8.0, y: -8.0 }, max: c2v { x: 8.0, y: 8.0 } };
    let cap = c2Capsule { a: c2v { x: 30.0, y: 0.0 }, b: c2v { x: 45.0, y: 10.0 }, r: 3.0 };
    let bA = blob_of(&bbx);
    let bB = blob_of(&cap);

    for count in [1i32, 2, 3] {
        // every in-range index combination for an AABB (4 verts) x capsule (2)
        for i0 in 0..4i32 {
            for j0 in 0..2i32 {
                let base = c2GJKCache {
                    metric: 1.0,
                    count,
                    iA: [i0, (i0 + 1) % 4, (i0 + 2) % 4],
                    iB: [j0, 1 - j0, j0],
                    div: 1.0,
                };
                let mut cc = base;
                let mut cr = base;
                let (mut ac, mut bc, mut ic) = (c2v::default(), c2v::default(), 0 as c_int);
                let (mut ar, mut br, mut ir) = (c2v::default(), c2v::default(), 0 as c_int);
                unsafe {
                    let dc = gjk(&g_c, &bA, C2_TYPE_AABB, std::ptr::null(), &bB,
                        C2_TYPE_CAPSULE, std::ptr::null(), &mut ac, &mut bc, 1, &mut ic, &mut cc);
                    let dr = gjk(&g_r, &bA, C2_TYPE_AABB, std::ptr::null(), &bB,
                        C2_TYPE_CAPSULE, std::ptr::null(), &mut ar, &mut br, 1, &mut ir, &mut cr);
                    assert!(
                        feq(dc, dr),
                        "row37 count{count} iA0={i0} iB0={j0}: dist C={} R={}",
                        fdesc(dc), fdesc(dr)
                    );
                }
                assert!(veq(ac, ar) && veq(bc, br), "row37 count{count} iA0={i0} iB0={j0}: witness");
                assert_eq!(ic, ir, "row37 count{count} iA0={i0} iB0={j0}: iterations");
                assert!(cache_eq(&cc, &cr), "row37: cache C={cc:?} R={cr:?}");
            }
        }
    }
}
