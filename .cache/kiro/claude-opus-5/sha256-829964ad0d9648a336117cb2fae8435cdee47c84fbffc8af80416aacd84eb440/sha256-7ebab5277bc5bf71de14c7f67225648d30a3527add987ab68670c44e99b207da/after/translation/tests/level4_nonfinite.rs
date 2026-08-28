//! Level 4: exhaustive non-finite input grid (0, ±0, ±inf, several NaN
//! payloads, ±FLT_MAX) over every exported function.
//!
//! What is asserted here is the strongest property that is actually well
//! defined for this program:
//!
//!   * whenever the C result is not a NaN, the Rust result is bit-identical;
//!   * NaN-ness agrees exactly -- C never returns a number where Rust returns
//!     NaN, or vice versa.
//!
//! The *payload and sign bit of a NaN* are deliberately not compared. On x86-64
//! SSE, `ADDSS`/`SUBSS`/`MULSS` return the destination operand's NaN when both
//! operands are NaN, and the hardware "indefinite" NaN (0xFFC00000) for invalid
//! operations such as `0 * inf`. Which of the two products ends up in the
//! destination register is a register-allocation decision, so the NaN bits are a
//! codegen artifact rather than program behaviour. This is demonstrable from the
//! C side alone: compiling c_src/src/lib.c at -O1 instead of the CMake default
//! changes `c2Dot`'s NaN payload in 11284 of the 28561 cases below, without any
//! source change. See `tests/nan_report.rs` (run with `--ignored`) for the full
//! per-symbol breakdown.

#![allow(non_snake_case)]

mod common;
use common::*;

/// Values chosen to drive `0*inf`, `inf-inf`, `0/0` and `inf/inf` through the
/// helpers, plus NaNs with distinguishable payloads and both signs.
const NONFINITE: &[f32] = &[
    0.0,
    -0.0,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
    -f32::NAN,
    f32::from_bits(0x7fc0_1234), // quiet NaN, non-default payload
    f32::from_bits(0xffc0_abcd), // negative quiet NaN
    f32::from_bits(0x7f80_0001), // signalling NaN
    1.0,
    -1.0,
    f32::MAX,
    f32::MIN,
];

/// Bit-identical unless both sides are NaN.
fn chk(ctx: &str, c: f32, r: f32) {
    if c.to_bits() == r.to_bits() {
        return;
    }
    assert!(
        c.is_nan() && r.is_nan(),
        "{ctx}: C=0x{:08x} ({c:?}) vs Rust=0x{:08x} ({r:?}) -- \
         differing values and not both NaN",
        c.to_bits(),
        r.to_bits()
    );
}

fn chk_v(ctx: &str, c: c2v, r: c2v) {
    chk(&format!("{ctx}.x"), c.x, r.x);
    chk(&format!("{ctx}.y"), c.y, r.y);
}

/// Compares a `#[repr(C)]` value word by word with the same NaN allowance.
fn chk_bytes<T>(ctx: &str, c: &T, r: &T) {
    let n = std::mem::size_of::<T>();
    let cb = unsafe { std::slice::from_raw_parts(c as *const T as *const u8, n) };
    let rb = unsafe { std::slice::from_raw_parts(r as *const T as *const u8, n) };
    for i in (0..n).step_by(4) {
        let cw = u32::from_ne_bytes([cb[i], cb[i + 1], cb[i + 2], cb[i + 3]]);
        let rw = u32::from_ne_bytes([rb[i], rb[i + 1], rb[i + 2], rb[i + 3]]);
        if cw == rw {
            continue;
        }
        let (cf, rf) = (f32::from_bits(cw), f32::from_bits(rw));
        assert!(
            cf.is_nan() && rf.is_nan(),
            "{ctx}: word @+{i}: C=0x{cw:08x} ({cf:?}) vs Rust=0x{rw:08x} ({rf:?})"
        );
    }
}

#[test]
fn nonfinite_scalar_returns() {
    for sym in ["c2Dot", "c2Det2"] {
        let (c, r) = both::<FnFvv>(sym);
        for &ax in NONFINITE {
            for &ay in NONFINITE {
                for &bx in NONFINITE {
                    for &by in NONFINITE {
                        let a = c2v { x: ax, y: ay };
                        let b = c2v { x: bx, y: by };
                        unsafe { chk(&format!("{sym}({a:?},{b:?})"), c(a, b), r(a, b)) };
                    }
                }
            }
        }
    }
    let (c, r) = both::<FnFv>("c2Len");
    for &x in NONFINITE {
        for &y in NONFINITE {
            let a = c2v { x, y };
            unsafe { chk(&format!("c2Len({a:?})"), c(a), r(a)) };
        }
    }
}

#[test]
fn nonfinite_vector_returns() {
    for sym in ["c2Neg", "c2Skew", "c2CCW90", "c2Norm"] {
        let (c, r) = both::<FnVv>(sym);
        for &x in NONFINITE {
            for &y in NONFINITE {
                let a = c2v { x, y };
                unsafe { chk_v(&format!("{sym}({a:?})"), c(a), r(a)) };
            }
        }
    }
    for sym in ["c2Mulvs", "c2Div"] {
        let (c, r) = both::<FnVvf>(sym);
        for &x in NONFINITE {
            for &y in NONFINITE {
                for &k in NONFINITE {
                    let a = c2v { x, y };
                    unsafe { chk_v(&format!("{sym}({a:?},{k:?})"), c(a, k), r(a, k)) };
                }
            }
        }
    }
    for sym in ["c2Add", "c2Sub", "c2Maxv", "c2Minv"] {
        let (c, r) = both::<FnVvv>(sym);
        for &ax in NONFINITE {
            for &ay in NONFINITE {
                for &bx in NONFINITE {
                    for &by in NONFINITE {
                        let a = c2v { x: ax, y: ay };
                        let b = c2v { x: bx, y: by };
                        unsafe { chk_v(&format!("{sym}({a:?},{b:?})"), c(a, b), r(a, b)) };
                    }
                }
            }
        }
    }
    let (c, r) = both::<FnVvvv>("c2Clampv");
    for &ax in NONFINITE {
        for &ay in NONFINITE {
            for &lo in NONFINITE {
                for &hi in NONFINITE {
                    let a = c2v { x: ax, y: ay };
                    let l = c2v { x: lo, y: hi };
                    let h = c2v { x: hi, y: lo };
                    unsafe {
                        chk_v(
                            &format!("c2Clampv({a:?},{l:?},{h:?})"),
                            c(a, l, h),
                            r(a, l, h),
                        )
                    };
                }
            }
        }
    }
}

/// `c2Maxv` / `c2Minv` are pure comparisons -- no arithmetic, so these must be
/// bit-identical even for NaN inputs (the ternary simply selects an operand).
#[test]
fn nonfinite_minmax_is_strictly_identical() {
    for sym in ["c2Maxv", "c2Minv", "c2Clampv"] {
        for &ax in NONFINITE {
            for &ay in NONFINITE {
                for &bx in NONFINITE {
                    for &by in NONFINITE {
                        let a = c2v { x: ax, y: ay };
                        let b = c2v { x: bx, y: by };
                        let (cv, rv) = if sym == "c2Clampv" {
                            let (c, r) = both::<FnVvvv>(sym);
                            unsafe { (c(a, b, a), r(a, b, a)) }
                        } else {
                            let (c, r) = both::<FnVvv>(sym);
                            unsafe { (c(a, b), r(a, b)) }
                        };
                        assert_eq!(
                            (cv.x.to_bits(), cv.y.to_bits()),
                            (rv.x.to_bits(), rv.y.to_bits()),
                            "{sym}({a:?},{b:?}) must be bit-identical (selection only)"
                        );
                    }
                }
            }
        }
    }
}

/// `c2Neg` / `c2Skew` / `c2CCW90` only move and flip sign bits, so they too must
/// be bit-identical for every input including NaNs.
#[test]
fn nonfinite_signflips_are_strictly_identical() {
    for sym in ["c2Neg", "c2Skew", "c2CCW90"] {
        let (c, r) = both::<FnVv>(sym);
        for &x in NONFINITE {
            for &y in NONFINITE {
                let a = c2v { x, y };
                unsafe {
                    let (cv, rv) = (c(a), r(a));
                    assert_eq!(
                        (cv.x.to_bits(), cv.y.to_bits()),
                        (rv.x.to_bits(), rv.y.to_bits()),
                        "{sym}({a:?}) must be bit-identical"
                    );
                }
            }
        }
    }
}

#[test]
fn nonfinite_rotations() {
    for sym in ["c2Mulrv", "c2MulrvT"] {
        let (c, r) = both::<FnVrv>(sym);
        for &rc in NONFINITE {
            for &rs in NONFINITE {
                for &bx in NONFINITE {
                    for &by in NONFINITE {
                        let rot = c2r { c: rc, s: rs };
                        let b = c2v { x: bx, y: by };
                        unsafe { chk_v(&format!("{sym}({rot:?},{b:?})"), c(rot, b), r(rot, b)) };
                    }
                }
            }
        }
    }
    let (c, r) = both::<FnVxv>("c2Mulxv");
    for &f in NONFINITE {
        for &g in NONFINITE {
            for &h in NONFINITE {
                for &k in NONFINITE {
                    let x = c2x {
                        p: c2v { x: f, y: g },
                        r: c2r { c: h, s: k },
                    };
                    let b = c2v { x: k, y: h };
                    unsafe { chk_v(&format!("c2Mulxv({x:?},{b:?})"), c(x, b), r(x, b)) };
                }
            }
        }
    }
}

#[test]
fn nonfinite_simplex_ops() {
    let mut g = Rng::new(4242);
    let pick = |g: &mut Rng| NONFINITE[g.below(NONFINITE.len() as u32) as usize];

    let metric = both::<FnSimplexF>("c2GJKSimplexMetric");
    let dfun = both::<FnSimplexV>("c2D");
    let lfun = both::<FnSimplexV>("c2L");
    let wit = both::<FnWitness>("c2Witness");
    let s2 = both::<FnSimplexVoid>("c22");
    let s3 = both::<FnSimplexVoid>("c23");

    for it in 0..60_000u32 {
        let mut s = c2Simplex::default();
        for v in s.verts.iter_mut() {
            v.sA = c2v {
                x: pick(&mut g),
                y: pick(&mut g),
            };
            v.sB = c2v {
                x: pick(&mut g),
                y: pick(&mut g),
            };
            v.p = c2v {
                x: pick(&mut g),
                y: pick(&mut g),
            };
            v.u = pick(&mut g);
            v.iA = g.below(4) as i32;
            v.iB = g.below(4) as i32;
        }
        s.div = pick(&mut g);
        s.count = (it % 5) as i32;

        {
            let (mut a, mut b) = (s, s);
            unsafe {
                chk(
                    &format!("c2GJKSimplexMetric(count={})", s.count),
                    metric.0(&mut a),
                    metric.1(&mut b),
                )
            };
        }
        {
            let (mut a, mut b) = (s, s);
            unsafe {
                chk_v(
                    &format!("c2D(count={})", s.count),
                    dfun.0(&mut a),
                    dfun.1(&mut b),
                )
            };
        }
        {
            let (mut a, mut b) = (s, s);
            unsafe {
                chk_v(
                    &format!("c2L(count={})", s.count),
                    lfun.0(&mut a),
                    lfun.1(&mut b),
                )
            };
        }
        {
            let (mut cs, mut rs) = (s, s);
            let (mut ca, mut cb) = (c2v { x: 1.0, y: 2.0 }, c2v { x: 3.0, y: 4.0 });
            let (mut ra, mut rb) = (ca, cb);
            unsafe {
                wit.0(&mut cs, &mut ca, &mut cb);
                wit.1(&mut rs, &mut ra, &mut rb);
            }
            chk_v(&format!("c2Witness.a(count={})", s.count), ca, ra);
            chk_v(&format!("c2Witness.b(count={})", s.count), cb, rb);
            chk_bytes("c2Witness/simplex", &cs, &rs);
        }
        for (name, f, need) in [("c22", s2, 2i32), ("c23", s3, 3)] {
            let mut base = s;
            base.count = need;
            let (mut cs, mut rs) = (base, base);
            unsafe {
                f.0(&mut cs);
                f.1(&mut rs);
            }
            // `count` and the integer indices must match exactly: the branch the
            // C code takes is fully determined by the (NaN-aware) comparisons.
            assert_eq!(cs.count, rs.count, "{name}: count differs on {base:?}");
            for k in 0..4 {
                assert_eq!(
                    (cs.verts[k].iA, cs.verts[k].iB),
                    (rs.verts[k].iA, rs.verts[k].iB),
                    "{name}: vertex {k} indices differ on {base:?}"
                );
            }
            chk_bytes(&format!("{name} on {base:?}"), &cs, &rs);
        }
    }
}

/// `c2Support` returns an index chosen purely by comparison, so it must agree
/// exactly even when the dot products are NaN.
#[test]
fn nonfinite_support_indices() {
    let (c, r) = both::<FnSupport>("c2Support");
    let mut g = Rng::new(777);
    let pick = |g: &mut Rng| NONFINITE[g.below(NONFINITE.len() as u32) as usize];
    for _ in 0..60_000 {
        let n = 1 + g.below(8) as usize;
        let mut verts = [c2v::default(); 8];
        for v in verts.iter_mut().take(n) {
            *v = c2v {
                x: pick(&mut g),
                y: pick(&mut g),
            };
        }
        let d = c2v {
            x: pick(&mut g),
            y: pick(&mut g),
        };
        unsafe {
            assert_eq!(
                c(verts.as_ptr(), n as i32, d),
                r(verts.as_ptr(), n as i32, d),
                "c2Support(n={n}, verts={verts:?}, d={d:?})"
            );
        }
    }
}

/// The whole solver driven with non-finite shape data. `dist`, the witness
/// points and the returned cache must agree (NaN payloads excepted), and the
/// integer parts of the cache -- which decide control flow on the next call --
/// must be exactly equal.
#[test]
fn nonfinite_c2GJK() {
    use std::ffi::{c_int, c_void};
    let (cf, rf) = both::<FnGJK>("c2GJK");
    let mut g = Rng::new(31337);
    let pick = |g: &mut Rng| NONFINITE[g.below(NONFINITE.len() as u32) as usize];

    for i in 0..30_000u32 {
        let circle = c2Circle {
            p: c2v {
                x: pick(&mut g),
                y: pick(&mut g),
            },
            r: pick(&mut g),
        };
        let aabb = c2AABB {
            min: c2v {
                x: pick(&mut g),
                y: pick(&mut g),
            },
            max: c2v {
                x: pick(&mut g),
                y: pick(&mut g),
            },
        };
        let cap = c2Capsule {
            a: c2v {
                x: pick(&mut g),
                y: pick(&mut g),
            },
            b: c2v {
                x: pick(&mut g),
                y: pick(&mut g),
            },
            r: pick(&mut g),
        };
        let shapes: [(*const c_void, c_int); 3] = [
            (&circle as *const c2Circle as *const c_void, C2_TYPE_CIRCLE),
            (&aabb as *const c2AABB as *const c_void, C2_TYPE_AABB),
            (&cap as *const c2Capsule as *const c_void, C2_TYPE_CAPSULE),
        ];
        let (pa, ta) = shapes[(i % 3) as usize];
        let (pb, tb) = shapes[((i / 3) % 3) as usize];
        let use_radius = (i % 2) as c_int;

        let mut co = (c2v { x: 5.0, y: 6.0 }, c2v { x: 7.0, y: 8.0 }, -1i32, c2GJKCache::default());
        let mut ro = co;
        let cd = unsafe {
            cf(
                pa, ta, std::ptr::null(), pb, tb, std::ptr::null(),
                &mut co.0, &mut co.1, use_radius, &mut co.2, &mut co.3,
            )
        };
        let rd = unsafe {
            rf(
                pa, ta, std::ptr::null(), pb, tb, std::ptr::null(),
                &mut ro.0, &mut ro.1, use_radius, &mut ro.2, &mut ro.3,
            )
        };
        let ctx = format!("c2GJK nonfinite[{i}] ta={ta} tb={tb} radius={use_radius}");
        chk(&format!("{ctx}/dist"), cd, rd);
        chk_v(&format!("{ctx}/outA"), co.0, ro.0);
        chk_v(&format!("{ctx}/outB"), co.1, ro.1);
        assert_eq!(co.2, ro.2, "{ctx}/iterations");
        assert_eq!(co.3.count, ro.3.count, "{ctx}/cache.count");
        assert_eq!(co.3.iA, ro.3.iA, "{ctx}/cache.iA");
        assert_eq!(co.3.iB, ro.3.iB, "{ctx}/cache.iB");
        chk_bytes(&format!("{ctx}/cache"), &co.3, &ro.3);
    }
}

/// `gjk_cache` with non-finite parameters: must not crash, must not touch the
/// caller's `a9`/`b9` buffers.
#[test]
fn nonfinite_gjk_cache() {
    let (cf, rf) = both::<FnGjkCache>("gjk_cache");
    let mut g = Rng::new(2024);
    let pick = |g: &mut Rng| NONFINITE[g.below(NONFINITE.len() as u32) as usize];
    for i in 0..20_000u32 {
        let p: [f32; 9] = std::array::from_fn(|_| pick(&mut g));
        let seed_a = c2v { x: 1.25, y: -2.5 };
        let seed_b = c2v { x: 3.75, y: -4.0 };
        let (mut ca, mut cb) = (seed_a, seed_b);
        let (mut ra, mut rb) = (seed_a, seed_b);
        let rev = (i % 3) as i8 - 1;
        unsafe {
            cf(rev, &mut ca, &mut cb, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8]);
            rf(rev, &mut ra, &mut rb, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8]);
        }
        assert_eq!(
            (ca, cb, ra, rb),
            (seed_a, seed_b, seed_a, seed_b),
            "gjk_cache[{i}] must leave a9/b9 untouched (params={p:?})"
        );
    }
}
