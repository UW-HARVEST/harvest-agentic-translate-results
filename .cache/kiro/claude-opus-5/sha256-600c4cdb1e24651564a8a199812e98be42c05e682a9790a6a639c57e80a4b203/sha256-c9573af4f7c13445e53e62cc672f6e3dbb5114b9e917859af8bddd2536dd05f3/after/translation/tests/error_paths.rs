//! Phase C — error / rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`. Each constructs the exact invalid input the
//! C source checks for and asserts that BOTH `.so`s return the SAME sentinel
//! (same `int`, same `float` bit pattern), not merely "both failed".

#![allow(non_snake_case)]
mod common;

use common::*;

/// Out-of-range `C2_TYPE` values. C enums accept any `int`, so all of these are
/// real inputs that reach the `switch` statements.
const BAD_TYPES: [i32; 12] = [3, 4, 5, 255, 256, 1000, -1, -2, -256, i32::MIN, i32::MAX, 0x1_0000];

/// Simplex `count` values outside the 1..=3 the C `switch`es name.
const BAD_COUNTS: [i32; 8] = [0, 4, 5, 8, -1, -2, i32::MIN, i32::MAX];

fn some_shapes(rng: &mut Rng) -> [Shape; 3] {
    [
        gen_shape(rng, C2_TYPE_CIRCLE, Class::Near, false),
        gen_shape(rng, C2_TYPE_AABB, Class::Near, false),
        gen_shape(rng, C2_TYPE_CAPSULE, Class::Near, false),
    ]
}

fn collided(api: &Api, a: &Shape, tya: i32, b: &Shape, tyb: i32) -> i32 {
    let mut ab = a.bytes.clone();
    let mut bb = b.bytes.clone();
    // Pad so an over-large read of the wrong struct type stays inside our
    // allocation (the C code casts the void* to whatever `type` says).
    ab.resize(64, 0);
    bb.resize(64, 0);
    unsafe {
        (api.c2Collided)(
            ab.as_mut_ptr() as *const std::ffi::c_void,
            tya,
            bb.as_mut_ptr() as *const std::ffi::c_void,
            tyb,
        )
    }
}

// ---------------------------------------------------------------------------
// Rows 1-4 — c2Collided `default:` labels
// ---------------------------------------------------------------------------

/// Row 1 — `typeA` out of range → `return 0`.
#[test]
fn err_collided_bad_typeA() {
    let p = pair();
    let mut rng = Rng::new(1001);
    let shapes = some_shapes(&mut rng);
    for &bad in &BAD_TYPES {
        for a in &shapes {
            for b in &shapes {
                for tyb in [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE, 3, -1] {
                    let rc = collided(p.c, a, bad, b, tyb);
                    let rr = collided(p.rs, a, bad, b, tyb);
                    same(&format!("c2Collided typeA={bad} typeB={tyb}"), rc, rr);
                    assert_eq!(rc, 0, "C should reject typeA={bad} with 0, got {rc}");
                }
            }
        }
    }
}

/// Rows 2-4 — `typeB` out of range for each valid `typeA` → `return 0`.
#[test]
fn err_collided_bad_typeB_circle() {
    err_collided_bad_typeB(C2_TYPE_CIRCLE);
}
#[test]
fn err_collided_bad_typeB_aabb() {
    err_collided_bad_typeB(C2_TYPE_AABB);
}
#[test]
fn err_collided_bad_typeB_capsule() {
    err_collided_bad_typeB(C2_TYPE_CAPSULE);
}

fn err_collided_bad_typeB(tya: i32) {
    let p = pair();
    let mut rng = Rng::new(1002 + tya as u64);
    let shapes = some_shapes(&mut rng);
    for &bad in &BAD_TYPES {
        for a in &shapes {
            for b in &shapes {
                let rc = collided(p.c, a, tya, b, bad);
                let rr = collided(p.rs, a, tya, b, bad);
                same(&format!("c2Collided typeA={tya} typeB={bad}"), rc, rr);
                assert_eq!(rc, 0, "C should reject typeB={bad} with 0, got {rc}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 5 — c2MakeProxy has no `default:` label
// ---------------------------------------------------------------------------

/// Row 5 — an unrecognised `type` must leave `*p` COMPLETELY untouched.
#[test]
fn err_makeproxy_bad_type_leaves_output_untouched() {
    let p = pair();
    let mut rng = Rng::new(1005);
    let seed = c2Proxy {
        radius: f32::from_bits(0xABCD_EF01),
        count: -31337,
        verts: [
            c2v { x: f32::from_bits(0x0101_0101), y: f32::from_bits(0x0202_0202) },
            c2v { x: f32::from_bits(0x0303_0303), y: f32::from_bits(0x0404_0404) },
            c2v { x: f32::from_bits(0x0505_0505), y: f32::from_bits(0x0606_0606) },
            c2v { x: f32::from_bits(0x0707_0707), y: f32::from_bits(0x0808_0808) },
            c2v { x: f32::from_bits(0x0909_0909), y: f32::from_bits(0x0A0A_0A0A) },
            c2v { x: f32::from_bits(0x0B0B_0B0B), y: f32::from_bits(0x0C0C_0C0C) },
            c2v { x: f32::from_bits(0x0D0D_0D0D), y: f32::from_bits(0x0E0E_0E0E) },
            c2v { x: f32::from_bits(0x0F0F_0F0F), y: f32::from_bits(0x1010_1010) },
        ],
    };
    for &bad in &BAD_TYPES {
        for shape in some_shapes(&mut rng) {
            let mut pc = seed;
            let mut pr = seed;
            let mut bc = shape.bytes.clone();
            let mut br = shape.bytes.clone();
            unsafe {
                (p.c.c2MakeProxy)(bc.as_mut_ptr() as *const std::ffi::c_void, bad, &mut pc);
                (p.rs.c2MakeProxy)(br.as_mut_ptr() as *const std::ffi::c_void, bad, &mut pr);
            }
            same(&format!("c2MakeProxy type={bad}"), pc, pr);
            same(&format!("c2MakeProxy type={bad}: proxy must be untouched"), pc, seed);
        }
    }
    // NULL shape pointer with a bad type: the C never dereferences `shape`
    // before the switch, so this must be a no-op too.
    for &bad in &BAD_TYPES {
        let mut pc = seed;
        let mut pr = seed;
        unsafe {
            (p.c.c2MakeProxy)(std::ptr::null(), bad, &mut pc);
            (p.rs.c2MakeProxy)(std::ptr::null(), bad, &mut pr);
        }
        same(&format!("c2MakeProxy(NULL, {bad})"), pc, pr);
        same(&format!("c2MakeProxy(NULL, {bad}) untouched"), pc, seed);
    }
}

// ---------------------------------------------------------------------------
// Rows 6-11 — simplex accessors with out-of-range `count` / zero `div`
// ---------------------------------------------------------------------------

fn poisoned_simplex(rng: &mut Rng, count: i32, div: f32) -> c2Simplex {
    let mut s = c2Simplex::default();
    for v in s.v.iter_mut() {
        *v = c2sv {
            sA: rng.vec_coord(),
            sB: rng.vec_coord(),
            p: rng.vec_coord(),
            u: rng.coord(),
            iA: rng.below(8) as i32,
            iB: rng.below(8) as i32,
        };
    }
    s.div = div;
    s.count = count;
    s
}

/// Row 6 — `c2GJKSimplexMetric` with `count` outside 1..=3 → `0.0f`.
#[test]
fn err_simplexmetric_out_of_range_count() {
    let p = pair();
    let mut rng = Rng::new(1006);
    for &count in &BAD_COUNTS {
        for _ in 0..64 {
            let s = poisoned_simplex(&mut rng, count, 1.0);
            let mut sc = s;
            let mut sr = s;
            let rc = unsafe { (p.c.c2GJKSimplexMetric)(&mut sc) };
            let rr = unsafe { (p.rs.c2GJKSimplexMetric)(&mut sr) };
            same(&format!("c2GJKSimplexMetric count={count}"), rc, rr);
            same(&format!("c2GJKSimplexMetric count={count} untouched"), sc, sr);
            assert_eq!(rc.to_bits(), 0.0f32.to_bits(), "C should return +0.0 for count={count}");
        }
    }
}

/// Row 7 — `c2D` with `count` 0/3/out-of-range → `c2V(0,0)`.
#[test]
fn err_c2d_out_of_range_count() {
    let p = pair();
    let mut rng = Rng::new(1007);
    for count in BAD_COUNTS.iter().copied().chain([3]) {
        for _ in 0..64 {
            let s = poisoned_simplex(&mut rng, count, 1.0);
            let mut sc = s;
            let mut sr = s;
            let rc = unsafe { (p.c.c2D)(&mut sc) };
            let rr = unsafe { (p.rs.c2D)(&mut sr) };
            same(&format!("c2D count={count}"), rc, rr);
            same(&format!("c2D count={count} untouched"), sc, sr);
            assert_eq!((rc.x.to_bits(), rc.y.to_bits()), (0, 0), "C should return (0,0) for count={count}");
        }
    }
}

/// Row 8 — `c2L` with `count` 0/3/out-of-range → `c2V(0,0)`.
#[test]
fn err_c2l_out_of_range_count() {
    let p = pair();
    let mut rng = Rng::new(1008);
    for count in BAD_COUNTS.iter().copied().chain([3]) {
        for _ in 0..64 {
            let s = poisoned_simplex(&mut rng, count, 1.0);
            let mut sc = s;
            let mut sr = s;
            let rc = unsafe { (p.c.c2L)(&mut sc) };
            let rr = unsafe { (p.rs.c2L)(&mut sr) };
            same(&format!("c2L count={count}"), rc, rr);
            same(&format!("c2L count={count} untouched"), sc, sr);
            assert_eq!((rc.x.to_bits(), rc.y.to_bits()), (0, 0), "C should return (0,0) for count={count}");
        }
    }
}

/// Row 9 — `c2Witness` with `count` outside 1..=3 → both outputs `c2V(0,0)`.
#[test]
fn err_witness_out_of_range_count() {
    let p = pair();
    let mut rng = Rng::new(1009);
    for &count in &BAD_COUNTS {
        for _ in 0..64 {
            let s = poisoned_simplex(&mut rng, count, 1.0);
            let poison = c2v { x: f32::from_bits(0x1357_9BDF), y: f32::from_bits(0x2468_ACE0) };
            let (mut ac, mut bc, mut ar, mut br) = (poison, poison, poison, poison);
            let mut sc = s;
            let mut sr = s;
            unsafe {
                (p.c.c2Witness)(&mut sc, &mut ac, &mut bc);
                (p.rs.c2Witness)(&mut sr, &mut ar, &mut br);
            }
            same(&format!("c2Witness count={count}"), (ac, bc), (ar, br));
            same(&format!("c2Witness count={count} untouched"), sc, sr);
            assert_eq!(
                (ac.x.to_bits(), ac.y.to_bits(), bc.x.to_bits(), bc.y.to_bits()),
                (0, 0, 0, 0),
                "C should zero both outputs for count={count}"
            );
        }
    }
}

/// Row 10 — `c2Witness` with `div == 0` (`1.0f/0` is never guarded).
#[test]
fn err_witness_zero_div() {
    let p = pair();
    let mut rng = Rng::new(1010);
    for div in [0.0f32, -0.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::MIN_POSITIVE, f32::from_bits(1)] {
        for count in [1i32, 2, 3] {
            for _ in 0..64 {
                let s = poisoned_simplex(&mut rng, count, div);
                let poison = c2v { x: 1.0, y: 2.0 };
                let (mut ac, mut bc, mut ar, mut br) = (poison, poison, poison, poison);
                let mut sc = s;
                let mut sr = s;
                unsafe {
                    (p.c.c2Witness)(&mut sc, &mut ac, &mut bc);
                    (p.rs.c2Witness)(&mut sr, &mut ar, &mut br);
                }
                same(&format!("c2Witness div={div} count={count}"), (ac, bc), (ar, br));
            }
        }
    }
}

/// Row 11 — `c2L` with `div == 0`.
#[test]
fn err_c2l_zero_div() {
    let p = pair();
    let mut rng = Rng::new(1011);
    for div in [0.0f32, -0.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::from_bits(1)] {
        for count in [1i32, 2] {
            for _ in 0..64 {
                let s = poisoned_simplex(&mut rng, count, div);
                let mut sc = s;
                let mut sr = s;
                let rc = unsafe { (p.c.c2L)(&mut sc) };
                let rr = unsafe { (p.rs.c2L)(&mut sr) };
                same(&format!("c2L div={div} count={count}"), rc, rr);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 12-14 — unguarded division and non-finite propagation
// ---------------------------------------------------------------------------

/// Row 12 — `c2Div` by zero: `±inf` components, `NaN` for a zero numerator.
#[test]
fn err_div_by_zero() {
    let p = pair();
    let mut rng = Rng::new(1012);
    for b in [0.0f32, -0.0] {
        for a in [
            c2v { x: 1.0, y: -1.0 },
            c2v { x: 0.0, y: 0.0 },
            c2v { x: -0.0, y: 0.0 },
            c2v { x: f32::MAX, y: f32::MIN },
            c2v { x: f32::NAN, y: 1.0 },
            c2v { x: f32::INFINITY, y: f32::NEG_INFINITY },
        ] {
            unsafe {
                same(&format!("c2Div({a:?}, {b})"), (p.c.c2Div)(a, b), (p.rs.c2Div)(a, b));
                same_strict_v(&format!("c2Div strict({a:?}, {b})"), (p.c.c2Div)(a, b), (p.rs.c2Div)(a, b));
            }
        }
        for _ in 0..256 {
            let a = rng.vec_coord();
            unsafe { same_strict_v("c2Div random/0", (p.c.c2Div)(a, b), (p.rs.c2Div)(a, b)) };
        }
    }
}

/// Row 13 — `c2Norm` of the zero vector: `0 * inf` → NaN.
#[test]
fn err_norm_zero_vector() {
    let p = pair();
    for a in [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: 0.0 },
        c2v { x: 0.0, y: -0.0 },
        c2v { x: -0.0, y: -0.0 },
    ] {
        unsafe {
            let (rc, rr) = ((p.c.c2Norm)(a), (p.rs.c2Norm)(a));
            same(&format!("c2Norm({a:?})"), rc, rr);
            same_strict_v(&format!("c2Norm strict({a:?})"), rc, rr);
            assert!(rc.x.is_nan() && rc.y.is_nan(), "C c2Norm(0) should be NaN, got {rc:?}");
        }
    }
}

/// Row 14 — non-finite scalars through every helper, strict bit equality.
#[test]
fn err_nonfinite_scalar_helpers() {
    let p = pair();
    let vals = [
        f32::NAN,
        f32::from_bits(0xFFC0_0000),
        f32::from_bits(0x7F80_0001), // sNaN
        f32::INFINITY,
        f32::NEG_INFINITY,
        0.0,
        -0.0,
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        FLT_EPSILON,
        1.0,
        -1.0,
    ];
    for &x in &vals {
        let a = c2v { x, y: 1.0 };
        // `fin` keeps the OTHER operand finite, so at most one NaN reaches any
        // single arithmetic op and NaN propagation is destination-order
        // independent -> strict bit equality is required.
        let fin = c2v { x: 2.0, y: 3.0 };
        unsafe {
            same_strict(&format!("c2Len({a:?})"), (p.c.c2Len)(a), (p.rs.c2Len)(a));
            same_strict_v(&format!("c2Norm({a:?})"), (p.c.c2Norm)(a), (p.rs.c2Norm)(a));
            same_strict("c2Dot one-exotic", (p.c.c2Dot)(a, fin), (p.rs.c2Dot)(a, fin));
            same_strict("c2Det2 one-exotic", (p.c.c2Det2)(a, fin), (p.rs.c2Det2)(a, fin));
            same_strict_v("c2Add one-exotic", (p.c.c2Add)(a, fin), (p.rs.c2Add)(a, fin));
            same_strict_v("c2Sub one-exotic", (p.c.c2Sub)(a, fin), (p.rs.c2Sub)(a, fin));
            same_strict_v("c2Maxv one-exotic", (p.c.c2Maxv)(a, fin), (p.rs.c2Maxv)(a, fin));
            same_strict_v("c2Minv one-exotic", (p.c.c2Minv)(a, fin), (p.rs.c2Minv)(a, fin));
            same_strict_v("c2Clampv one-exotic", (p.c.c2Clampv)(a, fin, fin), (p.rs.c2Clampv)(a, fin, fin));
            same_strict_v("c2Mulvs one-exotic", (p.c.c2Mulvs)(fin, x), (p.rs.c2Mulvs)(fin, x));
            same_strict_v("c2Div one-exotic", (p.c.c2Div)(fin, x), (p.rs.c2Div)(fin, x));
            same_strict_v("c2Neg one-exotic", (p.c.c2Neg)(a), (p.rs.c2Neg)(a));
            same_strict_v("c2Skew one-exotic", (p.c.c2Skew)(a), (p.rs.c2Skew)(a));
            same_strict_v("c2CCW90 one-exotic", (p.c.c2CCW90)(a), (p.rs.c2CCW90)(a));

            // Exotic x exotic: NaN PAYLOAD is unspecified (see NOTES-nan.md),
            // so compare canonically -- still catches NaN vs non-NaN, wrong
            // sign of inf, wrong magnitude, etc.
            for &y in &vals {
                let b = c2v { x: y, y: 1.0 };
                same("c2Add nonfinite", (p.c.c2Add)(a, b), (p.rs.c2Add)(a, b));
                same("c2Sub nonfinite", (p.c.c2Sub)(a, b), (p.rs.c2Sub)(a, b));
                same("c2Mulvs nonfinite", (p.c.c2Mulvs)(a, y), (p.rs.c2Mulvs)(a, y));
                same("c2Div nonfinite", (p.c.c2Div)(a, y), (p.rs.c2Div)(a, y));
                same("c2Maxv nonfinite", (p.c.c2Maxv)(a, b), (p.rs.c2Maxv)(a, b));
                same("c2Minv nonfinite", (p.c.c2Minv)(a, b), (p.rs.c2Minv)(a, b));
                same("c2Clampv nonfinite", (p.c.c2Clampv)(a, b, a), (p.rs.c2Clampv)(a, b, a));
                same(&format!("c2Dot({a:?},{b:?})"), (p.c.c2Dot)(a, b), (p.rs.c2Dot)(a, b));
                same(&format!("c2Det2({a:?},{b:?})"), (p.c.c2Det2)(a, b), (p.rs.c2Det2)(a, b));
                let r = c2r { c: x, s: y };
                same("c2Mulrv nonfinite", (p.c.c2Mulrv)(r, b), (p.rs.c2Mulrv)(r, b));
                same("c2MulrvT nonfinite", (p.c.c2MulrvT)(r, b), (p.rs.c2MulrvT)(r, b));
                let xf = c2x { p: b, r };
                same("c2Mulxv nonfinite", (p.c.c2Mulxv)(xf, a), (p.rs.c2Mulxv)(xf, a));
                // Rotations with a FINITE vector: only one exotic operand per
                // product, so strict equality still applies.
                let r1 = c2r { c: x, s: 0.0 };
                same_strict_v("c2Mulrv strict", (p.c.c2Mulrv)(r1, fin), (p.rs.c2Mulrv)(r1, fin));
                same_strict_v("c2MulrvT strict", (p.c.c2MulrvT)(r1, fin), (p.rs.c2MulrvT)(r1, fin));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 15-17 — c2Support boundary counts
// ---------------------------------------------------------------------------

/// Row 15 — `count <= 0`: the loop never runs but `verts[0]` IS dereferenced.
#[test]
fn err_support_nonpositive_count() {
    let p = pair();
    let mut rng = Rng::new(1015);
    for &count in &[0i32, -1, -2, -100, i32::MIN] {
        for _ in 0..256 {
            let verts: Vec<c2v> = (0..8).map(|_| rng.vec_any()).collect();
            let d = rng.vec_any();
            let rc = unsafe { (p.c.c2Support)(verts.as_ptr(), count, d) };
            let rr = unsafe { (p.rs.c2Support)(verts.as_ptr(), count, d) };
            same(&format!("c2Support count={count}"), rc, rr);
            assert_eq!(rc, 0, "C should return index 0 for count={count}");
        }
    }
}

/// Row 16 — `count` larger than the logical vertex count (reads past the end,
/// but stays inside a deliberately over-allocated buffer).
#[test]
fn err_support_count_past_end() {
    let p = pair();
    let mut rng = Rng::new(1016);
    for &count in &[9i32, 12, 16, 32, 64] {
        for _ in 0..256 {
            // Allocate 64 slots so the over-read is inside our own memory.
            let verts: Vec<c2v> = (0..64).map(|_| rng.vec_any()).collect();
            let d = rng.vec_any();
            let rc = unsafe { (p.c.c2Support)(verts.as_ptr(), count, d) };
            let rr = unsafe { (p.rs.c2Support)(verts.as_ptr(), count, d) };
            same(&format!("c2Support count={count}"), rc, rr);
            assert!((0..count).contains(&rc));
        }
    }
}

/// Row 17 — all dots equal, and `NaN` dots (`dot > dmax` is false for NaN) →
/// the first index always wins.
#[test]
fn err_support_ties_and_nan() {
    let p = pair();
    // All identical vertices: every dot is equal, so index 0 must win.
    for count in 1..=8i32 {
        let verts = [c2v { x: 1.0, y: 2.0 }; 8];
        for d in [
            c2v { x: 1.0, y: 1.0 },
            c2v { x: -1.0, y: 0.0 },
            c2v { x: 0.0, y: 0.0 },
            c2v { x: f32::NAN, y: 0.0 },
            c2v { x: f32::INFINITY, y: f32::NEG_INFINITY },
        ] {
            let rc = unsafe { (p.c.c2Support)(verts.as_ptr(), count, d) };
            let rr = unsafe { (p.rs.c2Support)(verts.as_ptr(), count, d) };
            same(&format!("c2Support tie count={count} d={d:?}"), rc, rr);
            assert_eq!(rc, 0, "ties must keep index 0");
        }
    }
    // NaN vertices mixed with finite ones, every position.
    for nan_at in 0..8usize {
        let mut verts = [c2v { x: 1.0, y: 0.0 }; 8];
        verts[nan_at] = c2v { x: f32::NAN, y: f32::NAN };
        for k in 0..8usize {
            verts[k] = if k == nan_at { verts[k] } else { c2v { x: k as f32, y: -(k as f32) } };
        }
        for count in 1..=8i32 {
            for d in [c2v { x: 1.0, y: 1.0 }, c2v { x: -1.0, y: -1.0 }, c2v { x: 0.0, y: 1.0 }] {
                let rc = unsafe { (p.c.c2Support)(verts.as_ptr(), count, d) };
                let rr = unsafe { (p.rs.c2Support)(verts.as_ptr(), count, d) };
                same(&format!("c2Support NaN@{nan_at} count={count} d={d:?}"), rc, rr);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 18-23 — c2GJK NULL-pointer guards
// ---------------------------------------------------------------------------

/// Rows 18-19 — `ax_ptr`/`bx_ptr` NULL substitute `c2xIdentity()`.
#[test]
fn err_gjk_null_transforms() {
    let p = pair();
    let ident = unsafe { (p.c.c2xIdentity)() };
    let mut rng = Rng::new(1018);
    for tya in TYPES {
        for tyb in TYPES {
            for class in ALL_CLASSES {
                for i in 0..16 {
                    let sa = gen_shape(&mut rng, tya, class, false);
                    let sb = gen_shape(&mut rng, tyb, class, true);
                    for ur in [0, 1] {
                        let combos = [
                            (None, None),
                            (Some(ident), None),
                            (None, Some(ident)),
                            (Some(ident), Some(ident)),
                        ];
                        for (ax, bx) in combos {
                            diff_gjk(p, &format!("null-xform {}x{} {class:?} ur={ur} #{i} {ax:?}/{bx:?}", type_name(tya), type_name(tyb)), &sa, ax, &sb, bx, ur, OutSel::ALL, None);
                        }
                        // NULL must be indistinguishable from explicit identity.
                        for api in [p.c, p.rs] {
                            let n = run_gjk(api, &sa, None, &sb, None, ur, OutSel::ALL, None);
                            let e = run_gjk(api, &sa, Some(ident), &sb, Some(ident), ur, OutSel::ALL, None);
                            same(&format!("{}: NULL == identity", api.which), n, e);
                        }
                    }
                }
            }
        }
    }
}

/// Rows 20-23 — NULL `outA`/`outB`/`iterations`/`cache`: writes skipped, no crash.
#[test]
fn err_gjk_null_outputs() {
    let p = pair();
    let mut rng = Rng::new(1020);
    for tya in TYPES {
        for tyb in TYPES {
            for class in ALL_CLASSES {
                for i in 0..16 {
                    let sa = gen_shape(&mut rng, tya, class, false);
                    let sb = gen_shape(&mut rng, tyb, class, true);
                    for ur in [0, 1] {
                        for a in [false, true] {
                            for b in [false, true] {
                                for it in [false, true] {
                                    let sel = OutSel { a, b, iters: it };
                                    // cache NULL (row 23)
                                    diff_gjk(p, &format!("null-out {sel:?} {}x{} {class:?} ur={ur} #{i}", type_name(tya), type_name(tyb)), &sa, None, &sb, None, ur, sel, None);
                                    // cache non-NULL, same selection
                                    diff_gjk(p, &format!("null-out+cache {sel:?} {}x{} {class:?} ur={ur} #{i}", type_name(tya), type_name(tyb)), &sa, None, &sb, None, ur, sel, Some(c2GJKCache::default()));
                                }
                            }
                        }
                        // The returned distance must not depend on which
                        // out-params were supplied.
                        for api in [p.c, p.rs] {
                            let full = run_gjk(api, &sa, None, &sb, None, ur, OutSel::ALL, None);
                            let none = run_gjk(api, &sa, None, &sb, None, ur, OutSel { a: false, b: false, iters: false }, None);
                            same(&format!("{}: dist independent of out-params", api.which), full.dist, none.dist);
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 24-26 — cache gating
// ---------------------------------------------------------------------------

/// Row 24 — `cache->count == 0` → cache is NOT read; simplex re-seeded.
#[test]
fn err_gjk_cache_count_zero() {
    let p = pair();
    let mut rng = Rng::new(1024);
    for tya in TYPES {
        for tyb in TYPES {
            for class in ALL_CLASSES {
                for i in 0..16 {
                    let sa = gen_shape(&mut rng, tya, class, false);
                    let sb = gen_shape(&mut rng, tyb, class, true);
                    // count == 0 but everything else garbage: must be ignored.
                    let cache = c2GJKCache {
                        metric: rng.wild(),
                        count: 0,
                        iA: [99, -5, i32::MAX],
                        iB: [-1, 77, i32::MIN],
                        div: rng.wild(),
                    };
                    for ur in [0, 1] {
                        diff_gjk(p, &format!("cache count=0 {}x{} {class:?} ur={ur} #{i}", type_name(tya), type_name(tyb)), &sa, None, &sb, None, ur, OutSel::ALL, Some(cache));
                        // Must equal the NULL-cache result except for the
                        // write-back, so compare the distance/witness points.
                        for api in [p.c, p.rs] {
                            let with = run_gjk(api, &sa, None, &sb, None, ur, OutSel::ALL, Some(cache));
                            let without = run_gjk(api, &sa, None, &sb, None, ur, OutSel::ALL, None);
                            same(&format!("{}: count=0 cache is cold", api.which), (with.dist, with.a, with.b, with.iters), (without.dist, without.a, without.b, without.iters));
                        }
                    }
                }
            }
        }
    }
}

/// Row 25 — the metric gate `!(min < max*2 && metric < -1.0e8f)`.
///
/// The second conjunct is essentially unsatisfiable for real simplices, so
/// `cache_was_read` is set for every cache with `count != 0`. Both libraries
/// must agree for the whole spectrum of `metric`, including the values that
/// straddle `-1.0e8f`.
#[test]
fn err_gjk_cache_metric_reject() {
    let p = pair();
    let mut rng = Rng::new(1025);
    let metrics = [
        0.0f32, -0.0, 1.0, -1.0, -9.9e7, -1.0e8, -1.000_000_1e8, -1.1e8, -1.0e9, -1.0e30,
        f32::MIN, f32::MAX, f32::NAN, f32::INFINITY, f32::NEG_INFINITY, FLT_EPSILON, -FLT_EPSILON,
    ];
    for &metric in &metrics {
        for tya in TYPES {
            for tyb in TYPES {
                for count in [1i32, 2, 3] {
                    for i in 0..6 {
                        let sa = gen_shape(&mut rng, tya, Class::Near, false);
                        let sb = gen_shape(&mut rng, tyb, Class::Near, true);
                        let na = match tya { C2_TYPE_CIRCLE => 1u32, C2_TYPE_AABB => 4, _ => 2 };
                        let nb = match tyb { C2_TYPE_CIRCLE => 1u32, C2_TYPE_AABB => 4, _ => 2 };
                        let mut cache = c2GJKCache { metric, count, div: 1.0, ..Default::default() };
                        for k in 0..count as usize {
                            cache.iA[k] = rng.below(na) as i32;
                            cache.iB[k] = rng.below(nb) as i32;
                        }
                        for ur in [0, 1] {
                            diff_gjk(p, &format!("metric gate {metric} count={count} {}x{} ur={ur} #{i}", type_name(tya), type_name(tyb)), &sa, None, &sb, None, ur, OutSel::ALL, Some(cache));
                        }
                    }
                }
            }
        }
    }
}

/// Row 26 — cache indices past the shape's real vertex count.
///
/// The C code indexes `cache->iA[i]` for `i < cache->count` (an `int[3]`) and
/// `pA.verts[iA]` (a `c2v[8]`) with no validation at all. Three regimes have to
/// be separated:
///
/// 1. `count` in 1..=3 and `iA`/`iB` in 4..=7 — every access is *in bounds* of
///    the declared arrays, but the vertex slot was never written by
///    `c2MakeProxy`. In C that slot is uninitialised stack, whose contents come
///    from whatever the caller left behind, so the *value* is not reproducible
///    by any implementation (measured: 39/272 samples differ, see
///    `tests/probe_ub.rs`). What IS required is that Rust returns
///    deterministically instead of panicking — which is what this test pins.
/// 2. `iA >= 8` — reads past `c2Proxy::verts` into adjacent stack. Isolated
///    into a child process below.
/// 3. `count > 3` — the write loop `verts + i` runs past `c2Simplex`'s four
///    `c2sv` slots and corrupts the caller's stack frame, in BOTH libraries
///    identically. That reliably crashes the process, so it is also isolated.
#[test]
fn err_gjk_cache_index_past_count() {
    let p = pair();
    let mut rng = Rng::new(1026);
    let mut returned = 0u32;
    for tya in TYPES {
        for tyb in TYPES {
            for count in [1i32, 2, 3] {
                for idx in 4..=7i32 {
                    let sa = gen_shape(&mut rng, tya, Class::Near, false);
                    let sb = gen_shape(&mut rng, tyb, Class::Near, true);
                    let cache = c2GJKCache {
                        metric: 0.0,
                        count,
                        iA: [idx, idx, idx],
                        iB: [idx, idx, idx],
                        div: 1.0,
                    };
                    for ur in [0, 1] {
                        // Rust must RETURN (not panic / abort), and do so
                        // deterministically.
                        let a = run_gjk(p.rs, &sa, None, &sb, None, ur, OutSel::ALL, Some(cache));
                        let b = run_gjk(p.rs, &sa, None, &sb, None, ur, OutSel::ALL, Some(cache));
                        same(
                            &format!("Rust cache idx={idx} count={count} {}x{} ur={ur} is deterministic", type_name(tya), type_name(tyb)),
                            a,
                            b,
                        );
                        returned += 1;
                    }
                }
            }
        }
    }
    assert_eq!(returned, 3 * 3 * 3 * 4 * 2);
    eprintln!(
        "err_gjk_cache_index_past_count: {returned} in-bounds-but-uninitialised cache \
         configurations returned deterministically from the Rust .so (no panic/abort). \
         Value comparison against C is intentionally omitted: C reads uninitialised stack \
         there (see NOTES-ub.md)."
    );

    // Regimes 2 and 3, isolated so a crash cannot take the suite down.
    for probe in ["probe_cache_index_oob_child", "probe_cache_count_overflow_child"] {
        let exe = std::env::current_exe().expect("current_exe");
        let out = std::process::Command::new(&exe)
            .args(["--exact", "--ignored", "--nocapture", probe])
            .env("RUN_UB_PROBE", "1")
            .output()
            .expect("failed to re-exec the test binary");
        eprintln!(
            "  {probe}: exit={:?} signal={:?}",
            out.status.code(),
            std::os::unix::process::ExitStatusExt::signal(&out.status)
        );
    }
}

/// Regime 2: `iA >= 8` reads past `c2Proxy::verts`. Isolated.
#[test]
#[ignore = "child process: reads past c2Proxy::verts (UB in both libraries)"]
fn probe_cache_index_oob_child() {
    if std::env::var("RUN_UB_PROBE").is_err() {
        return;
    }
    let p = pair();
    let mut rng = Rng::new(2026);
    for &idx in &[8i32, 9, 16, 32] {
        let sa = gen_shape(&mut rng, C2_TYPE_CIRCLE, Class::Near, false);
        let sb = gen_shape(&mut rng, C2_TYPE_CIRCLE, Class::Near, true);
        let cache = c2GJKCache { metric: 0.0, count: 1, iA: [idx; 3], iB: [idx; 3], div: 1.0 };
        let o = run_gjk(p.rs, &sa, None, &sb, None, 1, OutSel::ALL, Some(cache));
        eprintln!("PROBE Rust idx={idx} -> dist={}", o.dist);
    }
}

/// Regime 3: `cache->count > 3` overruns `c2Simplex`. Isolated.
#[test]
#[ignore = "child process: cache->count > 3 corrupts the stack in both libraries"]
fn probe_cache_count_overflow_child() {
    if std::env::var("RUN_UB_PROBE").is_err() {
        return;
    }
    let p = pair();
    let mut rng = Rng::new(3026);
    for &count in &[4i32, 5, 8] {
        let sa = gen_shape(&mut rng, C2_TYPE_CIRCLE, Class::Near, false);
        let sb = gen_shape(&mut rng, C2_TYPE_CIRCLE, Class::Near, true);
        let cache = c2GJKCache { metric: 0.0, count, iA: [0; 3], iB: [0; 3], div: 1.0 };
        let o = run_gjk(p.rs, &sa, None, &sb, None, 1, OutSel::ALL, Some(cache));
        eprintln!("PROBE Rust count={count} -> dist={}", o.dist);
    }
}

// ---------------------------------------------------------------------------
// Rows 27-34 — c2GJK loop guards and the use_radius block
// ---------------------------------------------------------------------------

/// Row 27 — the `while (iter < 20)` cap: `*iterations` must always be in range
/// and identical between the two libraries.
#[test]
fn err_gjk_iteration_cap() {
    let p = pair();
    let mut rng = Rng::new(1027);
    for _ in 0..8000 {
        let tya = TYPES[rng.below(3) as usize];
        let tyb = TYPES[rng.below(3) as usize];
        let class = ALL_CLASSES[rng.below(ALL_CLASSES.len() as u32) as usize];
        let sa = gen_shape(&mut rng, tya, class, false);
        let sb = gen_shape(&mut rng, tyb, class, true);
        let ur = rng.below(2) as i32;
        let oc = run_gjk(p.c, &sa, None, &sb, None, ur, OutSel::ALL, None);
        let or = run_gjk(p.rs, &sa, None, &sb, None, ur, OutSel::ALL, None);
        same("iteration cap", oc.clone(), or);
        let it = oc.iters.unwrap();
        assert!((0..=20).contains(&it), "iteration count {it} escaped the cap");
    }
}

/// Row 28 — the `hit` path: `s.count == 3` → `a = b`, `dist = 0`.
#[test]
fn err_gjk_hit_zero_distance() {
    let p = pair();
    let mut rng = Rng::new(1028);
    let mut hits = 0u32;
    for tya in TYPES {
        for tyb in TYPES {
            for i in 0..256 {
                // Heavily overlapping shapes reliably enclose the origin.
                let sa = gen_shape(&mut rng, tya, Class::Overlap, false);
                let sb = gen_shape(&mut rng, tyb, Class::Overlap, true);
                for ur in [0, 1] {
                    diff_gjk(p, &format!("hit path {}x{} ur={ur} #{i}", type_name(tya), type_name(tyb)), &sa, None, &sb, None, ur, OutSel::ALL, Some(c2GJKCache::default()));
                    let oc = run_gjk(p.c, &sa, None, &sb, None, ur, OutSel::ALL, None);
                    if oc.dist == 0.0 {
                        hits += 1;
                        // On the `hit` path C sets `a = b`.
                        let (a, b) = (oc.a.unwrap(), oc.b.unwrap());
                        assert_eq!((a.x.to_bits(), a.y.to_bits()), (b.x.to_bits(), b.y.to_bits()), "dist==0 should imply a == b");
                    }
                }
            }
        }
    }
    assert!(hits > 0, "the hit path was never reached");
    eprintln!("err_gjk_hit_zero_distance: {hits} zero-distance results");
}

/// Rows 29-31 — the three early `break`s (`d1 > d0`, degenerate direction,
/// duplicate support point). Exercised by degenerate/collinear configurations.
#[test]
fn err_gjk_degenerate_inputs() {
    let p = pair();
    let mut rng = Rng::new(1029);
    // Collinear / coincident / zero-extent shapes drive all three guards.
    for tya in TYPES {
        for tyb in TYPES {
            for i in 0..256 {
                let cases: [(Shape, Shape); 4] = [
                    (gen_shape(&mut rng, tya, Class::Degenerate, false), gen_shape(&mut rng, tyb, Class::Degenerate, true)),
                    (gen_shape(&mut rng, tya, Class::Coincident, false), gen_shape(&mut rng, tyb, Class::Coincident, true)),
                    (gen_shape(&mut rng, tya, Class::Tiny, false), gen_shape(&mut rng, tyb, Class::Tiny, true)),
                    (gen_shape(&mut rng, tya, Class::Grid, false), gen_shape(&mut rng, tyb, Class::Grid, true)),
                ];
                for (k, (sa, sb)) in cases.iter().enumerate() {
                    for ur in [0, 1] {
                        diff_gjk(p, &format!("degenerate break {}x{} case{k} ur={ur} #{i}", type_name(tya), type_name(tyb)), sa, None, sb, None, ur, OutSel::ALL, Some(c2GJKCache::default()));
                    }
                }
            }
        }
    }
    // Explicitly: two identical zero-radius points (direction is exactly zero,
    // so `c2Dot(d,d) < FLT_EPSILON^2` fires on the first iteration).
    for v in [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: 1.0, y: -1.0 },
        c2v { x: 1e18, y: -1e18 },
        c2v { x: f32::from_bits(1), y: 0.0 },
    ] {
        let s = Shape::circle(c2Circle { p: v, r: 0.0 });
        for ur in [0, 1] {
            diff_gjk(p, &format!("zero direction {v:?} ur={ur}"), &s, None, &s, None, ur, OutSel::ALL, Some(c2GJKCache::default()));
        }
    }
}

/// Rows 32-34 — the `use_radius` block: midpoint collapse, coincident witness
/// points after the shrink, and `c2Norm` of a zero difference.
#[test]
fn err_gjk_radius_collapse() {
    let p = pair();
    // dist <= rA + rB  (overlap) and dist <= FLT_EPSILON both collapse to the
    // midpoint with dist = 0.
    for k in 0..64u32 {
        let ra = k as f32 * 0.5;
        let rb = (k % 9) as f32 * 0.25;
        for gap_mul in [0.0f32, 0.25, 0.5, 0.999, 1.0, 1.000_001, 1.5, 2.0] {
            let gap = (ra + rb) * gap_mul;
            let a = Shape::circle(c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: ra });
            let b = Shape::circle(c2Circle { p: c2v { x: gap, y: 0.0 }, r: rb });
            for ur in [0, 1] {
                diff_gjk(p, &format!("radius collapse ra={ra} rb={rb} mul={gap_mul} ur={ur}"), &a, None, &b, None, ur, OutSel::ALL, None);
            }
            // Also the dist <= FLT_EPSILON side.
            let b2 = Shape::circle(c2Circle { p: c2v { x: FLT_EPSILON * 0.5, y: 0.0 }, r: rb });
            for ur in [0, 1] {
                diff_gjk(p, &format!("eps collapse ra={ra} rb={rb} ur={ur}"), &a, None, &b2, None, ur, OutSel::ALL, None);
            }
        }
    }
    // Radii whose sum is negative / non-finite: `dist > rA + rB` takes the
    // shrink path with a NaN or inf offset.
    let mut rng = Rng::new(1032);
    for i in 0..2000 {
        let ra = match rng.below(5) { 0 => f32::NAN, 1 => f32::INFINITY, 2 => f32::NEG_INFINITY, 3 => -rng.range(0.0, 10.0), _ => rng.range(0.0, 10.0) };
        let rb = match rng.below(5) { 0 => f32::NAN, 1 => f32::INFINITY, 2 => f32::NEG_INFINITY, 3 => -rng.range(0.0, 10.0), _ => rng.range(0.0, 10.0) };
        let a = Shape::circle(c2Circle { p: rng.vec_grid(), r: ra });
        let b = Shape::circle(c2Circle { p: rng.vec_grid(), r: rb });
        for ur in [0, 1] {
            diff_gjk(p, &format!("radius exotic ra={ra} rb={rb} ur={ur} #{i}"), &a, None, &b, None, ur, OutSel::ALL, None);
        }
    }
}

/// Row 35 — negative shape radii are never rejected.
#[test]
fn err_gjk_negative_radius() {
    let p = pair();
    let mut rng = Rng::new(1035);
    for i in 0..4000 {
        let ra = -rng.range(0.0, 50.0);
        let rb = -rng.range(0.0, 50.0);
        let a = if rng.below(2) == 0 {
            Shape::circle(c2Circle { p: rng.vec_grid(), r: ra })
        } else {
            Shape::capsule(c2Capsule { a: rng.vec_grid(), b: rng.vec_grid(), r: ra })
        };
        let b = if rng.below(2) == 0 {
            Shape::circle(c2Circle { p: rng.vec_grid(), r: rb })
        } else {
            Shape::capsule(c2Capsule { a: rng.vec_grid(), b: rng.vec_grid(), r: rb })
        };
        for ur in [0, 1] {
            diff_gjk(p, &format!("negative radius ra={ra} rb={rb} ur={ur} #{i}"), &a, None, &b, None, ur, OutSel::ALL, Some(c2GJKCache::default()));
        }
    }
}

/// Row 36 — `c2GJK` with an out-of-range type.
///
/// `c2MakeProxy` has no `default:` label, so the C leaves `c2Proxy pA` (a plain
/// stack local) UNINITIALISED and then reads `pA.count` / `pA.verts[...]`. That
/// is undefined behaviour: `pA.count` can be any `int`, which makes
/// `c2Support`'s `for (i = 1; i < count; ++i)` walk arbitrarily far and can
/// segfault. So the C side is invoked in a CHILD PROCESS, and only the Rust
/// side is required to behave (return, deterministically, without aborting).
#[test]
fn err_gjk_bad_type_no_crash() {
    let p = pair();
    let mut rng = Rng::new(1036);

    // Rust side: must always return, and must be deterministic.
    for &bad in &BAD_TYPES {
        for ty in TYPES {
            let good = gen_shape(&mut rng, ty, Class::Near, false);
            let mut junk = good.clone();
            junk.ty = bad;
            for ur in [0, 1] {
                let a = run_gjk(p.rs, &junk, None, &good, None, ur, OutSel::ALL, None);
                let b = run_gjk(p.rs, &junk, None, &good, None, ur, OutSel::ALL, None);
                same(&format!("Rust c2GJK typeA={bad} is deterministic"), a, b);
                let a = run_gjk(p.rs, &good, None, &junk, None, ur, OutSel::ALL, None);
                let b = run_gjk(p.rs, &good, None, &junk, None, ur, OutSel::ALL, None);
                same(&format!("Rust c2GJK typeB={bad} is deterministic"), a, b);
                let a = run_gjk(p.rs, &junk, None, &junk, None, ur, OutSel::ALL, None);
                let b = run_gjk(p.rs, &junk, None, &junk, None, ur, OutSel::ALL, None);
                same(&format!("Rust c2GJK both types={bad} is deterministic"), a, b);
            }
        }
    }

    // C side, isolated so a segfault cannot take the suite down.
    let exe = std::env::current_exe().expect("current_exe");
    let out = std::process::Command::new(&exe)
        .args(["--exact", "--ignored", "--nocapture", "probe_c_gjk_bad_type_child"])
        .env("RUN_C_BAD_TYPE_PROBE", "1")
        .output()
        .expect("failed to re-exec the test binary");
    eprintln!(
        "C-side bad-type probe (isolated): exit={:?}\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
            .lines()
            .filter(|l| l.starts_with("PROBE"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    // No assertion on the child's status: reading uninitialised stack is UB and
    // may legitimately crash. The point is that it is isolated and reported.
}

/// Child half of `err_gjk_bad_type_no_crash`. Never run as part of the suite.
#[test]
#[ignore = "child process of err_gjk_bad_type_no_crash; reads uninitialised C stack"]
fn probe_c_gjk_bad_type_child() {
    if std::env::var("RUN_C_BAD_TYPE_PROBE").is_err() {
        eprintln!("PROBE skipped (env not set)");
        return;
    }
    let p = pair();
    let mut rng = Rng::new(1036);
    let mut ok = 0u32;
    let mut agree = 0u32;
    for &bad in &BAD_TYPES {
        let good = gen_shape(&mut rng, C2_TYPE_CIRCLE, Class::Near, false);
        let mut junk = good.clone();
        junk.ty = bad;
        for ur in [0, 1] {
            let o = run_gjk(p.c, &junk, None, &good, None, ur, OutSel::ALL, None);
            let r = run_gjk(p.rs, &junk, None, &good, None, ur, OutSel::ALL, None);
            let same_bits = o.bits() == r.bits();
            if same_bits {
                agree += 1;
            }
            eprintln!(
                "PROBE C typeA={bad} ur={ur} -> dist={} iters={:?} | Rust dist={} iters={:?} | agree={same_bits}",
                o.dist, o.iters, r.dist, r.iters
            );
            ok += 1;
        }
    }
    eprintln!("PROBE completed {ok} C calls with invalid types without crashing; {agree}/{ok} agreed with Rust");
}

// ---------------------------------------------------------------------------
// Rows 37-44 — boolean wrappers and the public entry point
// ---------------------------------------------------------------------------

/// Row 37 — `c2AABBtoAABB` with inverted boxes (never validated).
#[test]
fn err_aabbtoaabb_inverted() {
    let p = pair();
    let mut rng = Rng::new(1037);
    for i in 0..4000 {
        let mk_inv = |rng: &mut Rng| {
            let c = rng.vec_grid();
            let e = c2v { x: 1.0 + rng.below(5) as f32, y: 1.0 + rng.below(5) as f32 };
            c2AABB { min: c2v { x: c.x + e.x, y: c.y + e.y }, max: c2v { x: c.x - e.x, y: c.y - e.y } }
        };
        let a = mk_inv(&mut rng);
        let b = if rng.below(2) == 0 { mk_inv(&mut rng) } else {
            let c = rng.vec_grid();
            c2AABB { min: c2v { x: c.x - 1.0, y: c.y - 1.0 }, max: c2v { x: c.x + 1.0, y: c.y + 1.0 } }
        };
        unsafe {
            same(&format!("c2AABBtoAABB inverted #{i} {a:?} {b:?}"), (p.c.c2AABBtoAABB)(a, b), (p.rs.c2AABBtoAABB)(a, b));
        }
    }
}

/// Row 38 — `NaN` in any AABB component: all four `<` are false → `return 1`.
#[test]
fn err_aabbtoaabb_nan() {
    let p = pair();
    let n = f32::NAN;
    let base = c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 1.0, y: 1.0 } };
    // Every single-component NaN substitution, on either box.
    for which in 0..8 {
        let mut a = base;
        let mut b = c2AABB { min: c2v { x: 10.0, y: 10.0 }, max: c2v { x: 11.0, y: 11.0 } };
        match which {
            0 => a.min.x = n,
            1 => a.min.y = n,
            2 => a.max.x = n,
            3 => a.max.y = n,
            4 => b.min.x = n,
            5 => b.min.y = n,
            6 => b.max.x = n,
            _ => b.max.y = n,
        }
        let rc = unsafe { (p.c.c2AABBtoAABB)(a, b) };
        let rr = unsafe { (p.rs.c2AABBtoAABB)(a, b) };
        same(&format!("c2AABBtoAABB NaN@{which}"), rc, rr);
    }
    // All-NaN.
    let all = c2AABB { min: c2v { x: n, y: n }, max: c2v { x: n, y: n } };
    let rc = unsafe { (p.c.c2AABBtoAABB)(all, all) };
    let rr = unsafe { (p.rs.c2AABBtoAABB)(all, all) };
    same("c2AABBtoAABB all-NaN", rc, rr);
    assert_eq!(rc, 1, "C reports a hit when every comparison is false");
    // Infinities.
    for (ax, bx) in [(f32::INFINITY, f32::NEG_INFINITY), (f32::NEG_INFINITY, f32::INFINITY)] {
        let a = c2AABB { min: c2v { x: ax, y: ax }, max: c2v { x: ax, y: ax } };
        let b = c2AABB { min: c2v { x: bx, y: bx }, max: c2v { x: bx, y: bx } };
        let rc = unsafe { (p.c.c2AABBtoAABB)(a, b) };
        let rr = unsafe { (p.rs.c2AABBtoAABB)(a, b) };
        same(&format!("c2AABBtoAABB inf {ax}/{bx}"), rc, rr);
    }
}

/// Row 39 — `c2CircletoCircle` with negative radii: `(A.r+B.r)^2` hides the sign.
#[test]
fn err_circle_negative_radius() {
    let p = pair();
    let mut rng = Rng::new(1039);
    for i in 0..4000 {
        let a = c2Circle { p: rng.vec_grid(), r: -rng.range(0.0, 20.0) };
        let b = c2Circle { p: rng.vec_grid(), r: -rng.range(0.0, 20.0) };
        unsafe {
            same(&format!("c2CircletoCircle neg #{i} {a:?} {b:?}"), (p.c.c2CircletoCircle)(a, b), (p.rs.c2CircletoCircle)(a, b));
        }
    }
    // Sign-symmetry probe: (-r) must behave exactly like (+r).
    for k in 1..40u32 {
        let r = k as f32;
        for d in [0.0f32, r, 2.0 * r - 0.5, 2.0 * r, 2.0 * r + 0.5] {
            let pos_a = c2Circle { p: c2v { x: 0.0, y: 0.0 }, r };
            let pos_b = c2Circle { p: c2v { x: d, y: 0.0 }, r };
            let neg_a = c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: -r };
            let neg_b = c2Circle { p: c2v { x: d, y: 0.0 }, r: -r };
            unsafe {
                same(&format!("neg==pos r={r} d={d} (C)"), (p.c.c2CircletoCircle)(pos_a, pos_b), (p.c.c2CircletoCircle)(neg_a, neg_b));
                same(&format!("neg r={r} d={d}"), (p.c.c2CircletoCircle)(neg_a, neg_b), (p.rs.c2CircletoCircle)(neg_a, neg_b));
            }
        }
    }
    // NaN / inf radii.
    for r in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::MAX, f32::MIN] {
        let a = c2Circle { p: c2v { x: 0.0, y: 0.0 }, r };
        let b = c2Circle { p: c2v { x: 5.0, y: 0.0 }, r };
        unsafe { same(&format!("c2CircletoCircle r={r}"), (p.c.c2CircletoCircle)(a, b), (p.rs.c2CircletoCircle)(a, b)) };
    }
}

/// Row 40 — `c2CircletoAABB` with an inverted box: `c2Clampv` collapses to `lo`.
#[test]
fn err_circletoaabb_inverted() {
    let p = pair();
    let mut rng = Rng::new(1040);
    for i in 0..4000 {
        let c = rng.vec_grid();
        let e = c2v { x: 1.0 + rng.below(5) as f32, y: 1.0 + rng.below(5) as f32 };
        let bb = c2AABB { min: c2v { x: c.x + e.x, y: c.y + e.y }, max: c2v { x: c.x - e.x, y: c.y - e.y } };
        let circ = c2Circle { p: rng.vec_grid(), r: rng.range(-10.0, 10.0) };
        unsafe {
            same(&format!("c2CircletoAABB inverted #{i} {circ:?} {bb:?}"), (p.c.c2CircletoAABB)(circ, bb), (p.rs.c2CircletoAABB)(circ, bb));
        }
    }
    // Deterministic: inverted box always clamps to `min`.
    let bb = c2AABB { min: c2v { x: 5.0, y: 5.0 }, max: c2v { x: -5.0, y: -5.0 } };
    for gx in -8..=8 {
        for gy in -8..=8 {
            for r in [0.0f32, 1.0, 5.0, 15.0, -3.0] {
                let circ = c2Circle { p: c2v { x: gx as f32, y: gy as f32 }, r };
                unsafe { same(&format!("c2CircletoAABB inv sweep {gx},{gy},{r}"), (p.c.c2CircletoAABB)(circ, bb), (p.rs.c2CircletoAABB)(circ, bb)) };
            }
        }
    }
}

/// Row 41 — degenerate capsule (`a == b`) makes `c2Dot(n,n) == 0`, so the
/// `da >= 0 && db < 0` branch divides by zero.
#[test]
fn err_circletocapsule_degenerate() {
    let p = pair();
    let mut rng = Rng::new(1041);
    for i in 0..4000 {
        let v = if rng.below(2) == 0 { rng.vec_grid() } else { rng.vec_any() };
        let cap = c2Capsule { a: v, b: v, r: match rng.below(4) { 0 => 0.0, 1 => -rng.range(0.0, 5.0), 2 => rng.wild(), _ => rng.range(0.0, 10.0) } };
        let circ = c2Circle { p: if rng.below(2) == 0 { v } else { rng.vec_any() }, r: rng.wild() };
        unsafe {
            same(&format!("c2CircletoCapsule degenerate #{i} {circ:?} {cap:?}"), (p.c.c2CircletoCapsule)(circ, cap), (p.rs.c2CircletoCapsule)(circ, cap));
        }
    }
    // Circle exactly at the degenerate capsule's point: da == db == 0, so the
    // `db < 0` test is false and the `bp` branch is taken.
    for v in [c2v { x: 0.0, y: 0.0 }, c2v { x: 3.0, y: -4.0 }, c2v { x: 1e18, y: -1e18 }] {
        for r in [0.0f32, 1.0, -1.0, f32::NAN, f32::INFINITY] {
            let cap = c2Capsule { a: v, b: v, r };
            let circ = c2Circle { p: v, r };
            unsafe { same(&format!("c2CircletoCapsule coincident {v:?} r={r}"), (p.c.c2CircletoCapsule)(circ, cap), (p.rs.c2CircletoCapsule)(circ, cap)) };
        }
    }
}

/// Row 42 — the boolean wrappers use `if (c2GJK(...))`, so `-0.0f` is falsy
/// (→ `return 1`) while `NaN` is truthy (→ `return 0`).
#[test]
fn err_bool_wrappers_zero_semantics() {
    let p = pair();
    let mut rng = Rng::new(1042);
    // Configurations engineered to make c2GJK return exactly 0.0, -0.0 or NaN.
    for i in 0..6000 {
        let mk_cap = |rng: &mut Rng| {
            let a = rng.vec_grid();
            c2Capsule {
                a,
                b: if rng.below(3) == 0 { a } else { rng.vec_grid() },
                r: match rng.below(6) {
                    0 => 0.0,
                    1 => -0.0,
                    2 => f32::NAN,
                    3 => f32::INFINITY,
                    4 => -rng.range(0.0, 5.0),
                    _ => rng.range(0.0, 5.0),
                },
            }
        };
        let ca = mk_cap(&mut rng);
        let cb = if rng.below(4) == 0 { ca } else { mk_cap(&mut rng) };
        let bb = {
            let c = rng.vec_grid();
            c2AABB { min: c2v { x: c.x - 1.0, y: c.y - 1.0 }, max: c2v { x: c.x + 1.0, y: c.y + 1.0 } }
        };
        unsafe {
            let (rc, rr) = ((p.c.c2CapsuletoCapsule)(ca, cb), (p.rs.c2CapsuletoCapsule)(ca, cb));
            same(&format!("c2CapsuletoCapsule zero-sem #{i} {ca:?} {cb:?}"), rc, rr);
            let (rc, rr) = ((p.c.c2AABBtoCapsule)(bb, ca), (p.rs.c2AABBtoCapsule)(bb, ca));
            same(&format!("c2AABBtoCapsule zero-sem #{i} {bb:?} {ca:?}"), rc, rr);
        }
    }
    // Cross-check the wrapper against c2GJK's raw return for the same input, in
    // BOTH libraries: `wrapper == (gjk_dist == 0.0)` unless the dist is NaN.
    for i in 0..2000 {
        let a = c2Capsule { a: rng.vec_grid(), b: rng.vec_grid(), r: rng.range(0.0, 4.0) };
        let b = c2Capsule { a: rng.vec_grid(), b: rng.vec_grid(), r: rng.range(0.0, 4.0) };
        let sa = Shape::capsule(a);
        let sb = Shape::capsule(b);
        for api in [p.c, p.rs] {
            let d = run_gjk(api, &sa, None, &sb, None, 1, OutSel::ALL, None).dist;
            let w = unsafe { (api.c2CapsuletoCapsule)(a, b) };
            let expect = if d.is_nan() { 0 } else { (d == 0.0) as i32 };
            assert_eq!(w, expect, "{}: wrapper/GJK mismatch #{i}: dist={d} wrapper={w}", api.which);
        }
    }
}

/// Row 43 — `c2BBVerts` on inverted / NaN boxes writes the fields verbatim.
#[test]
fn err_bbverts_inverted() {
    let p = pair();
    let mut rng = Rng::new(1043);
    for i in 0..4000 {
        let bb = match rng.below(3) {
            0 => { let c = rng.vec_grid(); c2AABB { min: c2v { x: c.x + 3.0, y: c.y + 3.0 }, max: c } }
            1 => c2AABB { min: rng.vec_wild(), max: rng.vec_wild() },
            _ => { let v = rng.vec_any(); c2AABB { min: v, max: v } }
        };
        let poison = c2v { x: f32::from_bits(0x7F81_2345), y: f32::from_bits(0x7F81_2346) };
        let mut oc = [poison; 4];
        let mut or = [poison; 4];
        let mut bc = bb;
        let mut br = bb;
        unsafe {
            (p.c.c2BBVerts)(oc.as_mut_ptr(), &mut bc);
            (p.rs.c2BBVerts)(or.as_mut_ptr(), &mut br);
        }
        same(&format!("c2BBVerts inverted #{i} {bb:?}"), oc, or);
        same(&format!("c2BBVerts inverted #{i} input untouched"), bc, br);
    }
}

/// Row 44 — the public `capsule` entry point never returns an error code.
#[test]
fn err_capsule_extreme_args() {
    let p = pair();
    let exotic = [
        f32::NAN,
        f32::from_bits(0xFFC0_0000),
        f32::from_bits(0x7F80_0001),
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::from_bits(0x8000_0001),
        0.0,
        -0.0,
        FLT_EPSILON,
        -FLT_EPSILON,
        1e18,
        -1e18,
        1e-40,
    ];
    for &a in &exotic {
        for &b in &exotic {
            for &c in &exotic {
                let rc = unsafe { (p.c.capsule)(a, b, c, c, a) };
                let rr = unsafe { (p.rs.capsule)(a, b, c, c, a) };
                same(&format!("capsule({a},{b},{c},{c},{a})"), rc, rr);
                assert!((0..8).contains(&rc), "capsule returned {rc}, outside the 3-bit mask");
                let rc = unsafe { (p.c.capsule)(a, b, c, a, b) };
                let rr = unsafe { (p.rs.capsule)(a, b, c, a, b) };
                same(&format!("capsule({a},{b},{c},{a},{b})"), rc, rr);
                assert!((0..8).contains(&rc), "capsule returned {rc}, outside the 3-bit mask");
            }
        }
    }
    // Negative r, degenerate a == b, everything at once.
    let mut rng = Rng::new(1044);
    for i in 0..20_000 {
        let v = rng.wild();
        let r = -rng.range(0.0, 60.0);
        let rc = unsafe { (p.c.capsule)(v, v, v, v, r) };
        let rr = unsafe { (p.rs.capsule)(v, v, v, v, r) };
        same(&format!("capsule degenerate #{i} v={v} r={r}"), rc, rr);
        assert!((0..8).contains(&rc));
    }
}
