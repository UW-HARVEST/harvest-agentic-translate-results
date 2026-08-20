//! Differential tests for `c_src/inc/q_shared.h` itself: the function-like
//! macros and the `static ID_INLINE` functions, plus the constants and the
//! struct layout (CONFIGS.md rows 50-60, ERRORS.md rows 26 and 27).
//!
//! Neither the macros nor the `static` functions have external linkage in C, so
//! they cannot be reached with `dlsym` directly.  Every call therefore goes
//! through the `w_*` entry points of `tests/csupport/wrappers.c`
//! (`cbuild/libcwrap.so`, which expands the *real* header) and their
//! counterparts in `src/wrappers.rs` (`target/<profile>/libdriver.so`).
//!
//! The `_`-prefixed function forms (`_DotProduct`, `_VectorAdd`, ...) come from
//! `cbuild/libcdriver.so` / the Rust cdylib and are cross-checked against the
//! macro forms: `#if 1` in q_shared.h:363 makes the macros the ones the library
//! actually uses, and both spellings must agree bit-for-bit.

mod harness;

use core::ffi::c_int;
use harness::*;

// ---------------------------------------------------------------------------
// the shapes of the `w_*` entry points (see tests/csupport/wrappers.c)
// ---------------------------------------------------------------------------

/// `vec_t w_VectorLength(const vec3_t)`, `w_VectorLengthSquared`
type VecToF = unsafe extern "C" fn(*const f32) -> f32;
/// `vec_t w_DotProduct(const vec3_t, const vec3_t)`, `w_Distance`, `w_DistanceSquared`
type Vec2ToF = unsafe extern "C" fn(*const f32, *const f32) -> f32;
/// `int w_VectorCompare(const vec3_t, const vec3_t)`
type Vec2ToI = unsafe extern "C" fn(*const f32, *const f32) -> c_int;
/// `int w_PlaneTypeForNormal(const vec3_t)`
type VecToI = unsafe extern "C" fn(*const f32) -> c_int;
/// `void w_SnapVector(vec3_t)`, `w_VectorNormalizeFast`, `w_VectorInverse`, `w_VectorClear`
type VecInPlace = unsafe extern "C" fn(*mut f32);
/// `void w_VectorCopy(const vec3_t, vec3_t)`, `w_VectorNegate`, `w_Vector4Copy`
type VecCopy = unsafe extern "C" fn(*const f32, *mut f32);
/// `void w_VectorAdd(const vec3_t, const vec3_t, vec3_t)`, `w_VectorSubtract`
type Vec3Op = unsafe extern "C" fn(*const f32, *const f32, *mut f32);
/// `void w_VectorScale(const vec3_t, vec_t, vec3_t)`
type VecScale = unsafe extern "C" fn(*const f32, f32, *mut f32);
/// `void w_VectorMA(const vec3_t, vec_t, const vec3_t, vec3_t)`
type VecMA = unsafe extern "C" fn(*const f32, f32, *const f32, *mut f32);
/// `void w_VectorSet(vec3_t, vec_t, vec_t, vec_t)`, `w_MAKERGB`
type Set3 = unsafe extern "C" fn(*mut f32, f32, f32, f32);
/// `void w_MAKERGBA(vec4_t, vec_t, vec_t, vec_t, vec_t)`
type Set4 = unsafe extern "C" fn(*mut f32, f32, f32, f32, f32);
/// `float w_SQRTFAST(float)`, `w_Square`
type F32ToF32 = unsafe extern "C" fn(f32) -> f32;
/// `double w_DEG2RAD(float)`, `w_RAD2DEG`
type F32ToF64 = unsafe extern "C" fn(f32) -> f64;
/// `int w_IS_NAN(float)`, `w_ANGLE2SHORT`
type F32ToI = unsafe extern "C" fn(f32) -> c_int;
/// `double w_SHORT2ANGLE(int)`
type IToF64 = unsafe extern "C" fn(c_int) -> f64;
/// `int w_ColorIndex(int)`
type IToI = unsafe extern "C" fn(c_int) -> c_int;
/// `void w_layout(int *)`, `void w_angle_indexes(int *)`
type IntsOut = unsafe extern "C" fn(*mut c_int);
/// `double w_M_PI(void)`
type ToF64 = unsafe extern "C" fn() -> f64;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// A NaN with a payload no computation in this library can produce, used to
/// prove that an output slot really was written (and that neighbouring slots
/// were not).
const SENTINEL: f32 = f32::from_bits(0x7fab_cdef);

fn bits(v: &[f32]) -> String {
    let s: Vec<String> = v.iter().map(|x| format!("0x{:08x}", x.to_bits())).collect();
    format!("[{}]", s.join(","))
}

/// A unit-length (as far as `f32` allows) version of `v`; falls back to an
/// axial vector for degenerate input.
fn normalized(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if l == 0.0 || !l.is_finite() {
        return [1.0, 0.0, 0.0];
    }
    [v[0] / l, v[1] / l, v[2] / l]
}

/// The vector shapes of CONFIGS.md axis 2: `S0` `S-0` `SA` `SN` `SR` `SH`
/// (squares overflow) `ST` (squares underflow) `SI` `SNaN`.
fn boundary_vecs() -> Vec<[f32; 3]> {
    let mut v: Vec<[f32; 3]> = vec![
        // S0 / S-0
        [0.0, 0.0, 0.0],
        [-0.0, -0.0, -0.0],
        [0.0, -0.0, 0.0],
        [-0.0, 0.0, -0.0],
        // SA -- all six axial unit vectors
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [-1.0, 0.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, 0.0, -1.0],
        // SN -- normalized
        [0.577_350_26, 0.577_350_26, 0.577_350_26],
        [0.6, 0.8, 0.0],
        [-0.267_261_24, 0.534_522_5, 0.801_783_7],
        [0.0, 0.707_106_77, -0.707_106_77],
        // SR -- ordinary finite values
        [1.0, 2.0, 3.0],
        [3.0, -4.0, 12.0],
        [-1.5, 2.25, -3.75],
        [255.0, 1.0 / 255.0, -255.0],
        [0.999_999_94, 1.000_000_1, 1.0],
        // SH -- squares overflow to +inf
        [1e30, 1e30, 1e30],
        [-1e30, 1e30, -1e30],
        [f32::MAX, 0.0, 0.0],
        [f32::MAX, f32::MAX, f32::MAX],
        [-f32::MAX, 1.0, 0.0],
        [1.8e19, 0.0, 0.0],
        [1.9e19, 1.9e19, 0.0],
        // ST -- squares underflow to 0 / to a denormal
        [1e-30, 1e-30, 1e-30],
        [1e-45, -1e-45, 1e-45],
        [f32::MIN_POSITIVE, -f32::MIN_POSITIVE, f32::MIN_POSITIVE],
        [1e-23, 1e-23, 0.0],
        // SI
        [f32::INFINITY, 0.0, 0.0],
        [0.0, f32::INFINITY, 0.0],
        [0.0, 0.0, f32::INFINITY],
        [f32::NEG_INFINITY, 0.0, 0.0],
        [f32::INFINITY, f32::NEG_INFINITY, 0.0],
        [f32::INFINITY, f32::INFINITY, f32::INFINITY],
        // SNaN
        [f32::NAN, 0.0, 0.0],
        [0.0, f32::NAN, 0.0],
        [0.0, 0.0, f32::NAN],
        [f32::NAN, f32::NAN, f32::NAN],
        [f32::NAN, f32::INFINITY, 1e30],
        [f32::NAN, 1.0, -1.0],
        // mixed magnitudes
        [1e30, 1e-30, 1.0],
        [1e-30, 1e30, -1.0],
    ];
    // every INTERESTING value, uniform and in each single slot
    for &x in INTERESTING {
        v.push([x, x, x]);
        v.push([x, 0.0, 0.0]);
        v.push([0.0, x, 0.0]);
        v.push([0.0, 0.0, x]);
        v.push([x, 1.0, -1.0]);
    }
    v
}

// ---------------------------------------------------------------------------
// row 50 -- VectorCompare
// ---------------------------------------------------------------------------

#[test]
fn vector_compare() {
    let (c, r): (Vec2ToI, Vec2ToI) = both_w("w_VectorCompare");
    let mut rng = Rng::new(5001); // row 50

    let call = |f: Vec2ToI, a: &[f32; 3], b: &[f32; 3]| -> c_int {
        unsafe { f(a.as_ptr(), b.as_ptr()) }
    };

    // ---- equal vectors (also: the same vector compared with itself) --------
    for v in boundary_vecs() {
        let ctx = format!("w_VectorCompare({v:?} {}, itself)", bits(&v));
        let (vc, vr) = (call(c, &v, &v), call(r, &v, &v));
        assert_int(&ctx, vc, vr);
        // `v1[0] != v2[0]` is *true* for a NaN, so a vector holding a NaN never
        // compares equal -- not even to itself.
        let expect: c_int = if v.iter().any(|x| x.is_nan()) { 0 } else { 1 };
        assert_int(&format!("{ctx} == {expect}"), vc, expect);

        // a separate but bit-identical copy must behave the same
        let w = v;
        assert_int(&format!("{ctx} (copy)"), call(c, &v, &w), call(r, &v, &w));
    }

    // ---- +0.0 vs -0.0: compares EQUAL ------------------------------------
    for i in 0..3 {
        let mut a = [0.0f32; 3];
        let mut b = [0.0f32; 3];
        b[i] = -0.0;
        let ctx = format!("w_VectorCompare(+0 vs -0 in slot {i})");
        let (vc, vr) = (call(c, &a, &b), call(r, &a, &b));
        assert_int(&ctx, vc, vr);
        assert_int(&format!("{ctx} == 1"), vc, 1);
        a[i] = -0.0;
        b[i] = 0.0;
        assert_int(
            &format!("w_VectorCompare(-0 vs +0 in slot {i})"),
            call(c, &a, &b),
            call(r, &a, &b),
        );
    }
    let allneg = [-0.0f32, -0.0, -0.0];
    let allpos = [0.0f32, 0.0, 0.0];
    assert_int(
        "w_VectorCompare({-0,-0,-0},{0,0,0})",
        call(c, &allneg, &allpos),
        call(r, &allneg, &allpos),
    );
    assert_int(
        "w_VectorCompare({-0,-0,-0},{0,0,0}) == 1",
        call(c, &allneg, &allpos),
        1,
    );

    // ---- NaN vs itself: compares UNEQUAL ----------------------------------
    for i in 0..3 {
        let mut a = [1.0f32, 2.0, 3.0];
        a[i] = f32::NAN;
        let b = a;
        let ctx = format!("w_VectorCompare(NaN in slot {i}, itself)");
        let (vc, vr) = (call(c, &a, &b), call(r, &a, &b));
        assert_int(&ctx, vc, vr);
        assert_int(&format!("{ctx} == 0"), vc, 0);
        // every NaN payload behaves the same way
        for payload in [0x7f80_0001u32, 0x7fc0_0000, 0x7fff_ffff, 0xffc0_0000, 0xffff_ffff] {
            let mut a = [1.0f32, 2.0, 3.0];
            a[i] = f32::from_bits(payload);
            let b = a;
            let ctx = format!("w_VectorCompare(NaN 0x{payload:08x} in slot {i}, itself)");
            let (vc, vr) = (call(c, &a, &b), call(r, &a, &b));
            assert_int(&ctx, vc, vr);
            assert_int(&format!("{ctx} == 0"), vc, 0);
        }
    }

    // ---- differing in exactly one component ------------------------------
    for base in boundary_vecs() {
        for i in 0..3 {
            for &other in &[0.0f32, 1.0, -1.0, f32::INFINITY, f32::NAN, 1e30, -0.0] {
                let mut b = base;
                b[i] = other;
                let ctx = format!(
                    "w_VectorCompare({} , {} differing in slot {i})",
                    bits(&base),
                    bits(&b)
                );
                assert_int(&ctx, call(c, &base, &b), call(r, &base, &b));
            }
            // one ulp apart
            let mut b = base;
            b[i] = f32::from_bits(base[i].to_bits() ^ 1);
            let ctx = format!("w_VectorCompare({}, one ulp in slot {i})", bits(&base));
            assert_int(&ctx, call(c, &base, &b), call(r, &base, &b));
        }
    }

    // ---- random pairs -----------------------------------------------------
    for _ in 0..20000 {
        let a = rng.vec3_any();
        let mut b = rng.vec3_any();
        // half the time make them mostly equal, so both branches are hit often
        if rng.bool() {
            b = a;
            let i = rng.below(3) as usize;
            if rng.bool() {
                b[i] = rng.f32_any();
            }
        }
        let ctx = format!("w_VectorCompare({}, {})", bits(&a), bits(&b));
        assert_int(&ctx, call(c, &a, &b), call(r, &a, &b));
    }
}

// ---------------------------------------------------------------------------
// row 51 -- VectorLength / VectorLengthSquared / Distance / DistanceSquared
// ---------------------------------------------------------------------------

#[test]
fn lengths_and_distances() {
    let (len_c, len_r): (VecToF, VecToF) = both_w("w_VectorLength");
    let (lsq_c, lsq_r): (VecToF, VecToF) = both_w("w_VectorLengthSquared");
    let (dst_c, dst_r): (Vec2ToF, Vec2ToF) = both_w("w_Distance");
    let (dsq_c, dsq_r): (Vec2ToF, Vec2ToF) = both_w("w_DistanceSquared");
    let mut rng = Rng::new(5101); // row 51

    let mut vecs = boundary_vecs();
    for _ in 0..4000 {
        vecs.push(rng.vec3_any());
    }
    for _ in 0..2000 {
        vecs.push(rng.vec3_finite());
    }
    for _ in 0..1000 {
        vecs.push(normalized(rng.vec3_mag(1.0)));
    }
    for _ in 0..1000 {
        vecs.push(rng.vec3_mag(1e30)); // SH: squares overflow
    }
    for _ in 0..1000 {
        vecs.push(rng.vec3_mag(1e-30)); // ST: squares underflow
    }

    // ---- lengths ---------------------------------------------------------
    for v in &vecs {
        let ctx = format!("w_VectorLength({v:?} {})", bits(v));
        check_f32(&ctx, v, unsafe { len_c(v.as_ptr()) }, unsafe {
            len_r(v.as_ptr())
        });
        let ctx = format!("w_VectorLengthSquared({v:?} {})", bits(v));
        check_f32(&ctx, v, unsafe { lsq_c(v.as_ptr()) }, unsafe {
            lsq_r(v.as_ptr())
        });
    }

    // ---- distances between pairs -----------------------------------------
    let mut pairs: Vec<([f32; 3], [f32; 3])> = Vec::new();
    let bs = boundary_vecs();
    for (i, a) in bs.iter().enumerate() {
        // every boundary shape against a handful of others (a full cross
        // product of ~230 shapes would be 50k pairs; take a diagonal band)
        for k in 0..6 {
            let b = bs[(i * 7 + k * 31 + 1) % bs.len()];
            pairs.push((*a, b));
        }
        pairs.push((*a, *a)); // SEQ, two equal (but distinct) vectors
    }
    for _ in 0..8000 {
        pairs.push((rng.vec3_any(), rng.vec3_any()));
    }
    for _ in 0..2000 {
        pairs.push((rng.vec3_finite(), rng.vec3_finite()));
    }
    for _ in 0..1000 {
        let a = rng.vec3_mag(1e3);
        let mut b = a;
        b[rng.below(3) as usize] += 1.0; // nearly equal
        pairs.push((a, b));
    }

    for (p1, p2) in &pairs {
        let inputs = [p1[0], p1[1], p1[2], p2[0], p2[1], p2[2]];
        let ctx = format!("w_Distance({}, {})", bits(p1), bits(p2));
        check_f32(&ctx, &inputs, unsafe { dst_c(p1.as_ptr(), p2.as_ptr()) }, unsafe {
            dst_r(p1.as_ptr(), p2.as_ptr())
        });
        let ctx = format!("w_DistanceSquared({}, {})", bits(p1), bits(p2));
        check_f32(&ctx, &inputs, unsafe { dsq_c(p1.as_ptr(), p2.as_ptr()) }, unsafe {
            dsq_r(p1.as_ptr(), p2.as_ptr())
        });
    }

    // ---- SEQ: the *same pointer* passed twice ----------------------------
    for v in &vecs {
        let inputs = [v[0], v[1], v[2], v[0], v[1], v[2]];
        let ctx = format!("w_Distance({} , same pointer)", bits(v));
        let (dc, dr) = unsafe { (dst_c(v.as_ptr(), v.as_ptr()), dst_r(v.as_ptr(), v.as_ptr())) };
        check_f32(&ctx, &inputs, dc, dr);
        let ctx2 = format!("w_DistanceSquared({} , same pointer)", bits(v));
        let (sc, sr) = unsafe { (dsq_c(v.as_ptr(), v.as_ptr()), dsq_r(v.as_ptr(), v.as_ptr())) };
        check_f32(&ctx2, &inputs, sc, sr);
        // v - v is exactly +0 for every finite v, so both are exactly +0.0
        if v.iter().all(|x| x.is_finite()) {
            assert_f32(&format!("{ctx} == +0.0"), 0.0, dc);
            assert_f32(&format!("{ctx2} == +0.0"), 0.0, sc);
        }
    }
}

// ---------------------------------------------------------------------------
// row 52 -- VectorNormalizeFast (the path main.c takes)
// ---------------------------------------------------------------------------

#[test]
fn vector_normalize_fast() {
    let (c, r): (VecInPlace, VecInPlace) = both_w("w_VectorNormalizeFast");
    let mut rng = Rng::new(5201); // row 52

    let mut vecs = boundary_vecs();
    for _ in 0..8000 {
        vecs.push(rng.vec3_any());
    }
    for _ in 0..4000 {
        vecs.push(rng.vec3_finite());
    }
    for _ in 0..2000 {
        vecs.push(normalized(rng.vec3_mag(1.0)));
    }
    for _ in 0..2000 {
        vecs.push(rng.vec3_mag(1e30));
    }
    for _ in 0..2000 {
        vecs.push(rng.vec3_mag(1e-30));
    }

    for v in &vecs {
        let mut a = *v;
        let mut b = *v;
        unsafe { c(a.as_mut_ptr()) };
        unsafe { r(b.as_mut_ptr()) };
        check_vec(
            &format!("w_VectorNormalizeFast({v:?} {})", bits(v)),
            v,
            &a,
            &b,
        );
    }
}

// ---------------------------------------------------------------------------
// row 53 -- VectorInverse (in place) and VectorNegate (a -> b)
// ---------------------------------------------------------------------------

#[test]
fn inverse_and_negate() {
    let (inv_c, inv_r): (VecInPlace, VecInPlace) = both_w("w_VectorInverse");
    let (neg_c, neg_r): (VecCopy, VecCopy) = both_w("w_VectorNegate");
    let mut rng = Rng::new(5301); // row 53

    let mut vecs = boundary_vecs();
    // arbitrary bit patterns, so every NaN payload and every denormal shows up
    for _ in 0..20000 {
        vecs.push([
            f32::from_bits(rng.next_u32()),
            f32::from_bits(rng.next_u32()),
            f32::from_bits(rng.next_u32()),
        ]);
    }
    // both zeroes and both infinities in every slot
    for &x in &[0.0f32, -0.0, f32::INFINITY, f32::NEG_INFINITY] {
        for i in 0..3 {
            let mut v = [1.0f32, -2.0, 3.0];
            v[i] = x;
            vecs.push(v);
        }
    }

    for v in &vecs {
        // ---- in place ----
        let mut a = *v;
        let mut b = *v;
        unsafe { inv_c(a.as_mut_ptr()) };
        unsafe { inv_r(b.as_mut_ptr()) };
        let ctx = format!("w_VectorInverse({})", bits(v));
        assert_vec(&ctx, &a, &b);
        // `-x` is a pure sign flip (xorps), so the payload of a NaN survives
        for i in 0..3 {
            assert_int(
                &format!("{ctx} slot {i} sign bit flipped"),
                a[i].to_bits(),
                v[i].to_bits() ^ 0x8000_0000,
            );
        }

        // ---- a -> b, separate output ----
        let mut oc = [SENTINEL; 4];
        let mut or_ = [SENTINEL; 4];
        unsafe { neg_c(v.as_ptr(), oc.as_mut_ptr()) };
        unsafe { neg_r(v.as_ptr(), or_.as_mut_ptr()) };
        let ctx = format!("w_VectorNegate({})", bits(v));
        assert_vec(&ctx, &oc[..3], &or_[..3]);
        assert_int(
            &format!("{ctx} wrote past the end"),
            oc[3].to_bits(),
            SENTINEL.to_bits(),
        );
        assert_int(
            &format!("{ctx} (Rust) wrote past the end"),
            or_[3].to_bits(),
            SENTINEL.to_bits(),
        );
        // the two spellings of the same operation must agree
        assert_vec(
            &format!("w_VectorInverse vs w_VectorNegate on {}", bits(v)),
            &a,
            &oc[..3],
        );
    }
}

// ---------------------------------------------------------------------------
// row 54 -- SnapVector
// ---------------------------------------------------------------------------

/// `(int)f` as gcc compiles it (`cvttss2si`): truncation toward zero, and the
/// "integer indefinite" value `0x80000000` for NaN and for everything outside
/// `[-2^31, 2^31)`.  Used to check the *C* result against ERRORS.md row 27
/// independently of the Rust translation.
fn snap_model(x: f32) -> f32 {
    if x.is_nan() || x >= 2147483648.0f32 || x < -2147483648.0f32 {
        -2147483648.0f32
    } else {
        (x as i32) as f32
    }
}

fn snap_values() -> Vec<f32> {
    let mut v: Vec<f32> = vec![
        0.0,
        -0.0,
        0.25,
        -0.25,
        0.5,
        -0.5,
        0.999_999_94,
        -0.999_999_94,
        1.0,
        -1.0,
        1.5,
        -1.5,
        2.5,
        -2.5,
        42.9,
        -42.9,
        -0.75,
        3.999_999_8,
        -3.999_999_8,
        123456.789,
        -123456.789,
        16777215.0,
        -16777215.0,
        16777216.0,
        // exactly +-2^31 and one ulp inside / outside
        2147483648.0,   // 2^31 -- OUT of int range
        -2147483648.0,  // -2^31 -- the smallest *valid* int
        2147483520.0,   // 2^31 - 128, the largest f32 below 2^31
        -2147483520.0,
        2147483904.0, // 2^31 + 256, the first f32 above 2^31
        -2147483904.0,
        // out of range
        1e30,
        -1e30,
        f32::MAX,
        -f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        // denormals and tiny values -- truncate to +0.0
        1e-45,
        -1e-45,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        1e-30,
        -1e-30,
    ];
    v.extend_from_slice(INTERESTING);
    v
}

#[test]
fn snap_vector() {
    let (c, r): (VecInPlace, VecInPlace) = both_w("w_SnapVector");
    let mut rng = Rng::new(5401); // row 54

    let vals = snap_values();

    // every boundary value in every slot, against a benign background
    let mut vecs: Vec<[f32; 3]> = Vec::new();
    for &x in &vals {
        vecs.push([x, x, x]);
        for i in 0..3 {
            let mut v = [1.75f32, -2.25, 3.5];
            v[i] = x;
            vecs.push(v);
        }
    }
    // triples of boundary values
    for (i, &x) in vals.iter().enumerate() {
        vecs.push([x, vals[(i + 1) % vals.len()], vals[(i + 2) % vals.len()]]);
    }
    // random: in range, around the range boundary, and anything at all
    for _ in 0..6000 {
        vecs.push(rng.vec3_mag(1e6));
    }
    for _ in 0..6000 {
        vecs.push(rng.vec3_mag(2.147_483_6e9));
    }
    for _ in 0..6000 {
        vecs.push(rng.vec3_any());
    }
    for _ in 0..2000 {
        vecs.push([
            f32::from_bits(rng.next_u32()),
            f32::from_bits(rng.next_u32()),
            f32::from_bits(rng.next_u32()),
        ]);
    }

    for v in &vecs {
        let mut a = *v;
        let mut b = *v;
        unsafe { c(a.as_mut_ptr()) };
        unsafe { r(b.as_mut_ptr()) };
        let ctx = format!("w_SnapVector({v:?} {})", bits(v));
        // the result is always an integral float, never a NaN, so it is
        // compared bit-for-bit even for NaN input
        assert_vec(&ctx, &a, &b);
        let model = [snap_model(v[0]), snap_model(v[1]), snap_model(v[2])];
        assert_vec(&format!("{ctx} vs cvttss2si model"), &a, &model);
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 27 -- the (int) cast of an out-of-range or NaN component
// ---------------------------------------------------------------------------

#[test]
fn snap_vector_overflow() {
    let (c, r): (VecInPlace, VecInPlace) = both_w("w_SnapVector");
    let mut rng = Rng::new(2701); // ERRORS.md row 27

    /// what `cvttss2si` returns for NaN / out-of-range, converted back to float
    const INDEFINITE: f32 = -2147483648.0f32; // 0xcf000000

    assert_int(
        "INT_MIN as f32 is 0xcf000000",
        INDEFINITE.to_bits(),
        0xcf00_0000u32,
    );

    // Values whose `(int)` conversion is undefined in C and yields the x86
    // "integer indefinite" value 0x80000000 with gcc.
    let mut bad: Vec<f32> = vec![
        2147483648.0, // exactly 2^31: INT_MAX is 2^31 - 1, so this overflows
        2147483904.0, // one ulp above 2^31
        -2147483904.0,
        2147484000.0,
        3e9,
        -3e9,
        1e30,
        -1e30,
        1e38,
        f32::MAX,
        -f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
    ];
    // every NaN payload class
    for payload in [
        0x7f80_0001u32,
        0x7fbf_ffff,
        0x7fc0_0000,
        0x7fff_ffff,
        0xff80_0001,
        0xffc0_0000,
        0xffff_ffff,
    ] {
        bad.push(f32::from_bits(payload));
    }
    for _ in 0..2000 {
        // random magnitudes beyond the int range
        let m = 2147483648.0f32 * (1.0 + (rng.next_u32() >> 8) as f32 / (1u32 << 24) as f32);
        bad.push(if rng.bool() { m } else { -m });
    }

    for &x in &bad {
        for i in 0..3 {
            let mut v = [1.5f32, -2.5, 3.5];
            v[i] = x;
            let mut a = v;
            let mut b = v;
            unsafe { c(a.as_mut_ptr()) };
            unsafe { r(b.as_mut_ptr()) };
            let ctx = format!(
                "w_SnapVector with 0x{:08x} ({x:?}) in slot {i}",
                x.to_bits()
            );
            assert_vec(&ctx, &a, &b);
            assert_f32(
                &format!("{ctx}: the component is the integer-indefinite value"),
                INDEFINITE,
                a[i],
            );
            assert_f32(
                &format!("{ctx}: the component is the integer-indefinite value (Rust)"),
                INDEFINITE,
                b[i],
            );
            // the in-range neighbours are untouched by the overflow
            for j in 0..3 {
                if j != i {
                    assert_f32(
                        &format!("{ctx}: slot {j} still truncates normally"),
                        snap_model(v[j]),
                        a[j],
                    );
                }
            }
        }
    }

    // -2^31 itself is *in* range (INT_MIN is representable), so the conversion
    // is well defined -- and produces the very same float.
    let v = [-2147483648.0f32, -2147483648.0, -2147483648.0];
    let mut a = v;
    let mut b = v;
    unsafe { c(a.as_mut_ptr()) };
    unsafe { r(b.as_mut_ptr()) };
    assert_vec("w_SnapVector(-2^31) (a valid conversion)", &a, &b);
    assert_vec(
        "w_SnapVector(-2^31) == -2147483648.0",
        &a,
        &[INDEFINITE, INDEFINITE, INDEFINITE],
    );
}

// ---------------------------------------------------------------------------
// row 55 -- the macro forms, and the `_`-prefixed function forms
// ---------------------------------------------------------------------------

#[test]
fn macro_forms() {
    // the macro forms, via the header expansion in tests/csupport/wrappers.c
    let (dot_c, dot_r): (Vec2ToF, Vec2ToF) = both_w("w_DotProduct");
    let (sub_c, sub_r): (Vec3Op, Vec3Op) = both_w("w_VectorSubtract");
    let (add_c, add_r): (Vec3Op, Vec3Op) = both_w("w_VectorAdd");
    let (cpy_c, cpy_r): (VecCopy, VecCopy) = both_w("w_VectorCopy");
    let (scl_c, scl_r): (VecScale, VecScale) = both_w("w_VectorScale");
    let (ma_c, ma_r): (VecMA, VecMA) = both_w("w_VectorMA");
    let (clr_c, clr_r): (VecInPlace, VecInPlace) = both_w("w_VectorClear");
    let (set_c, set_r): (Set3, Set3) = both_w("w_VectorSet");
    let (v4_c, v4_r): (VecCopy, VecCopy) = both_w("w_Vector4Copy");
    let (rgb_c, rgb_r): (Set3, Set3) = both_w("w_MAKERGB");
    let (rgba_c, rgba_r): (Set4, Set4) = both_w("w_MAKERGBA");

    // the `_`-prefixed function forms, from libcdriver.so / libdriver.so
    let (fdot_c, fdot_r): (Vec2ToF, Vec2ToF) = both("_DotProduct");
    let (fsub_c, fsub_r): (Vec3Op, Vec3Op) = both("_VectorSubtract");
    let (fadd_c, fadd_r): (Vec3Op, Vec3Op) = both("_VectorAdd");
    let (fcpy_c, fcpy_r): (VecCopy, VecCopy) = both("_VectorCopy");
    let (fscl_c, fscl_r): (VecScale, VecScale) = both("_VectorScale");
    let (fma_c, fma_r): (VecMA, VecMA) = both("_VectorMA");

    let mut rng = Rng::new(5501); // row 55

    // (a, b, s) triples: 20000 random ones plus the boundary shapes
    let mut cases: Vec<([f32; 3], [f32; 3], f32)> = Vec::new();
    let bs = boundary_vecs();
    for (i, a) in bs.iter().enumerate() {
        for k in 0..4 {
            let b = bs[(i * 5 + k * 37 + 3) % bs.len()];
            cases.push((*a, b, INTERESTING[(i + k) % INTERESTING.len()]));
        }
    }
    for _ in 0..20000 {
        cases.push((rng.vec3_any(), rng.vec3_any(), rng.f32_any()));
    }
    for _ in 0..4000 {
        cases.push((rng.vec3_finite(), rng.vec3_finite(), rng.f32_finite()));
    }

    for (a, b, s) in &cases {
        let (a, b, s) = (*a, *b, *s);
        let inputs = [a[0], a[1], a[2], b[0], b[1], b[2], s];
        let vecs_only = [a[0], a[1], a[2], b[0], b[1], b[2]];
        let tag = format!("a={} b={} s=0x{:08x}", bits(&a), bits(&b), s.to_bits());

        // ---- DotProduct -------------------------------------------------
        let (mc, mr) = unsafe { (dot_c(a.as_ptr(), b.as_ptr()), dot_r(a.as_ptr(), b.as_ptr())) };
        let (fc, fr) = unsafe { (fdot_c(a.as_ptr(), b.as_ptr()), fdot_r(a.as_ptr(), b.as_ptr())) };
        check_f32(&format!("w_DotProduct({tag})"), &vecs_only, mc, mr);
        check_f32(
            &format!("DotProduct macro vs _DotProduct, C side ({tag})"),
            &vecs_only,
            mc,
            fc,
        );
        check_f32(
            &format!("DotProduct macro vs _DotProduct, Rust side ({tag})"),
            &vecs_only,
            mr,
            fr,
        );

        // ---- VectorAdd / VectorSubtract ---------------------------------
        for (name, mac_c, mac_r, fun_c, fun_r) in [
            ("VectorAdd", add_c, add_r, fadd_c, fadd_r),
            ("VectorSubtract", sub_c, sub_r, fsub_c, fsub_r),
        ] {
            let mut om_c = [SENTINEL; 4];
            let mut om_r = [SENTINEL; 4];
            let mut of_c = [SENTINEL; 4];
            let mut of_r = [SENTINEL; 4];
            unsafe { mac_c(a.as_ptr(), b.as_ptr(), om_c.as_mut_ptr()) };
            unsafe { mac_r(a.as_ptr(), b.as_ptr(), om_r.as_mut_ptr()) };
            unsafe { fun_c(a.as_ptr(), b.as_ptr(), of_c.as_mut_ptr()) };
            unsafe { fun_r(a.as_ptr(), b.as_ptr(), of_r.as_mut_ptr()) };
            check_vec(
                &format!("w_{name}({tag})"),
                &vecs_only,
                &om_c[..3],
                &om_r[..3],
            );
            check_vec(
                &format!("{name} macro vs _{name}, C side ({tag})"),
                &vecs_only,
                &om_c[..3],
                &of_c[..3],
            );
            check_vec(
                &format!("{name} macro vs _{name}, Rust side ({tag})"),
                &vecs_only,
                &om_r[..3],
                &of_r[..3],
            );
            for (who, out) in [("C", &om_c), ("Rust", &om_r)] {
                assert_int(
                    &format!("w_{name} ({who}) wrote past the end ({tag})"),
                    out[3].to_bits(),
                    SENTINEL.to_bits(),
                );
            }
        }

        // ---- VectorScale ------------------------------------------------
        {
            let mut om_c = [SENTINEL; 4];
            let mut om_r = [SENTINEL; 4];
            let mut of_c = [SENTINEL; 4];
            let mut of_r = [SENTINEL; 4];
            unsafe { scl_c(a.as_ptr(), s, om_c.as_mut_ptr()) };
            unsafe { scl_r(a.as_ptr(), s, om_r.as_mut_ptr()) };
            unsafe { fscl_c(a.as_ptr(), s, of_c.as_mut_ptr()) };
            unsafe { fscl_r(a.as_ptr(), s, of_r.as_mut_ptr()) };
            let ins = [a[0], a[1], a[2], s];
            check_vec(&format!("w_VectorScale({tag})"), &ins, &om_c[..3], &om_r[..3]);
            check_vec(
                &format!("VectorScale macro vs _VectorScale, C side ({tag})"),
                &ins,
                &om_c[..3],
                &of_c[..3],
            );
            check_vec(
                &format!("VectorScale macro vs _VectorScale, Rust side ({tag})"),
                &ins,
                &om_r[..3],
                &of_r[..3],
            );
            assert_int(
                &format!("w_VectorScale wrote past the end ({tag})"),
                om_c[3].to_bits(),
                SENTINEL.to_bits(),
            );
        }

        // ---- VectorMA ---------------------------------------------------
        {
            let mut om_c = [SENTINEL; 4];
            let mut om_r = [SENTINEL; 4];
            let mut of_c = [SENTINEL; 4];
            let mut of_r = [SENTINEL; 4];
            unsafe { ma_c(a.as_ptr(), s, b.as_ptr(), om_c.as_mut_ptr()) };
            unsafe { ma_r(a.as_ptr(), s, b.as_ptr(), om_r.as_mut_ptr()) };
            unsafe { fma_c(a.as_ptr(), s, b.as_ptr(), of_c.as_mut_ptr()) };
            unsafe { fma_r(a.as_ptr(), s, b.as_ptr(), of_r.as_mut_ptr()) };
            check_vec(&format!("w_VectorMA({tag})"), &inputs, &om_c[..3], &om_r[..3]);
            // note: the macro computes `(b)[i]*(s)`, the function `scale*vecb[i]`
            check_vec(
                &format!("VectorMA macro vs _VectorMA, C side ({tag})"),
                &inputs,
                &om_c[..3],
                &of_c[..3],
            );
            check_vec(
                &format!("VectorMA macro vs _VectorMA, Rust side ({tag})"),
                &inputs,
                &om_r[..3],
                &of_r[..3],
            );
            assert_int(
                &format!("w_VectorMA wrote past the end ({tag})"),
                om_c[3].to_bits(),
                SENTINEL.to_bits(),
            );
        }

        // ---- VectorCopy -------------------------------------------------
        {
            let mut om_c = [SENTINEL; 4];
            let mut om_r = [SENTINEL; 4];
            let mut of_c = [SENTINEL; 4];
            let mut of_r = [SENTINEL; 4];
            unsafe { cpy_c(a.as_ptr(), om_c.as_mut_ptr()) };
            unsafe { cpy_r(a.as_ptr(), om_r.as_mut_ptr()) };
            unsafe { fcpy_c(a.as_ptr(), of_c.as_mut_ptr()) };
            unsafe { fcpy_r(a.as_ptr(), of_r.as_mut_ptr()) };
            // a copy is bit-exact, NaN payloads included
            assert_vec(&format!("w_VectorCopy({tag})"), &om_c[..3], &om_r[..3]);
            assert_vec(&format!("w_VectorCopy({tag}) == input"), &om_c[..3], &a);
            assert_vec(
                &format!("VectorCopy macro vs _VectorCopy, C side ({tag})"),
                &om_c[..3],
                &of_c[..3],
            );
            assert_vec(
                &format!("VectorCopy macro vs _VectorCopy, Rust side ({tag})"),
                &om_r[..3],
                &of_r[..3],
            );
            assert_int(
                &format!("w_VectorCopy wrote past the end ({tag})"),
                om_c[3].to_bits(),
                SENTINEL.to_bits(),
            );
        }

        // ---- VectorClear ------------------------------------------------
        {
            let mut oc = [a[0], a[1], a[2], SENTINEL];
            let mut or_ = [a[0], a[1], a[2], SENTINEL];
            unsafe { clr_c(oc.as_mut_ptr()) };
            unsafe { clr_r(or_.as_mut_ptr()) };
            assert_vec(&format!("w_VectorClear({tag})"), &oc[..3], &or_[..3]);
            assert_vec(
                &format!("w_VectorClear({tag}) == +0"),
                &oc[..3],
                &[0.0, 0.0, 0.0],
            );
            assert_int(
                &format!("w_VectorClear wrote past the end ({tag})"),
                oc[3].to_bits(),
                SENTINEL.to_bits(),
            );
        }

        // ---- VectorSet / MAKERGB / MAKERGBA / Vector4Copy ---------------
        {
            let mut oc = [SENTINEL; 4];
            let mut or_ = [SENTINEL; 4];
            unsafe { set_c(oc.as_mut_ptr(), a[0], a[1], a[2]) };
            unsafe { set_r(or_.as_mut_ptr(), a[0], a[1], a[2]) };
            assert_vec(&format!("w_VectorSet({tag})"), &oc[..3], &or_[..3]);
            assert_vec(&format!("w_VectorSet({tag}) == inputs"), &oc[..3], &a);
            assert_int(
                &format!("w_VectorSet wrote past the end ({tag})"),
                oc[3].to_bits(),
                SENTINEL.to_bits(),
            );

            let mut oc = [SENTINEL; 4];
            let mut or_ = [SENTINEL; 4];
            unsafe { rgb_c(oc.as_mut_ptr(), a[0], a[1], a[2]) };
            unsafe { rgb_r(or_.as_mut_ptr(), a[0], a[1], a[2]) };
            assert_vec(&format!("w_MAKERGB({tag})"), &oc[..3], &or_[..3]);
            assert_vec(&format!("w_MAKERGB({tag}) == inputs"), &oc[..3], &a);
            assert_int(
                &format!("w_MAKERGB wrote past the end ({tag})"),
                oc[3].to_bits(),
                SENTINEL.to_bits(),
            );

            let mut oc = [SENTINEL; 5];
            let mut or_ = [SENTINEL; 5];
            unsafe { rgba_c(oc.as_mut_ptr(), a[0], a[1], a[2], s) };
            unsafe { rgba_r(or_.as_mut_ptr(), a[0], a[1], a[2], s) };
            assert_vec(&format!("w_MAKERGBA({tag})"), &oc[..4], &or_[..4]);
            assert_vec(
                &format!("w_MAKERGBA({tag}) == inputs"),
                &oc[..4],
                &[a[0], a[1], a[2], s],
            );
            assert_int(
                &format!("w_MAKERGBA wrote past the end ({tag})"),
                oc[4].to_bits(),
                SENTINEL.to_bits(),
            );

            let src = [a[0], a[1], a[2], s];
            let mut oc = [SENTINEL; 5];
            let mut or_ = [SENTINEL; 5];
            unsafe { v4_c(src.as_ptr(), oc.as_mut_ptr()) };
            unsafe { v4_r(src.as_ptr(), or_.as_mut_ptr()) };
            assert_vec(&format!("w_Vector4Copy({tag})"), &oc[..4], &or_[..4]);
            assert_vec(&format!("w_Vector4Copy({tag}) == input"), &oc[..4], &src);
            assert_int(
                &format!("w_Vector4Copy wrote past the end ({tag})"),
                oc[4].to_bits(),
                SENTINEL.to_bits(),
            );
        }
    }

    // The pure-move macros must preserve *every* bit pattern, so they get their
    // own sweep over completely arbitrary words (all NaN payloads included).
    for _ in 0..20000 {
        let src = [
            f32::from_bits(rng.next_u32()),
            f32::from_bits(rng.next_u32()),
            f32::from_bits(rng.next_u32()),
            f32::from_bits(rng.next_u32()),
        ];
        let tag = bits(&src);

        let mut oc = [SENTINEL; 4];
        let mut or_ = [SENTINEL; 4];
        unsafe { cpy_c(src.as_ptr(), oc.as_mut_ptr()) };
        unsafe { cpy_r(src.as_ptr(), or_.as_mut_ptr()) };
        assert_vec(&format!("w_VectorCopy(bits {tag})"), &oc[..3], &or_[..3]);
        assert_vec(&format!("w_VectorCopy(bits {tag}) == input"), &oc[..3], &src[..3]);

        let mut oc = [SENTINEL; 5];
        let mut or_ = [SENTINEL; 5];
        unsafe { v4_c(src.as_ptr(), oc.as_mut_ptr()) };
        unsafe { v4_r(src.as_ptr(), or_.as_mut_ptr()) };
        assert_vec(&format!("w_Vector4Copy(bits {tag})"), &oc[..4], &or_[..4]);
        assert_vec(&format!("w_Vector4Copy(bits {tag}) == input"), &oc[..4], &src);

        let mut oc = [SENTINEL; 4];
        let mut or_ = [SENTINEL; 4];
        unsafe { set_c(oc.as_mut_ptr(), src[0], src[1], src[2]) };
        unsafe { set_r(or_.as_mut_ptr(), src[0], src[1], src[2]) };
        assert_vec(&format!("w_VectorSet(bits {tag})"), &oc[..3], &or_[..3]);

        let mut oc = [SENTINEL; 5];
        let mut or_ = [SENTINEL; 5];
        unsafe { rgba_c(oc.as_mut_ptr(), src[0], src[1], src[2], src[3]) };
        unsafe { rgba_r(or_.as_mut_ptr(), src[0], src[1], src[2], src[3]) };
        assert_vec(&format!("w_MAKERGBA(bits {tag})"), &oc[..4], &or_[..4]);
        assert_vec(&format!("w_MAKERGBA(bits {tag}) == input"), &oc[..4], &src);
    }
}

// ---------------------------------------------------------------------------
// row 56 -- SQRTFAST / IS_NAN / Square
// ---------------------------------------------------------------------------

#[test]
fn sqrtfast_isnan_square() {
    let (sqrt_c, sqrt_r): (F32ToF32, F32ToF32) = both_w("w_SQRTFAST");
    let (isnan_c, isnan_r): (F32ToI, F32ToI) = both_w("w_IS_NAN");
    let (square_c, square_r): (F32ToF32, F32ToF32) = both_w("w_Square");
    let mut rng = Rng::new(5601); // row 56

    // ---- IS_NAN: every exponent / mantissa class, both signs -------------
    // `IS_NAN(x)` only tests the exponent field, so it reports +-inf as "NaN"
    // as well.
    for exp in 0u32..=255 {
        for mant in [
            0u32, 1, 2, 0x0f_ffff, 0x3f_ffff, 0x40_0000, 0x40_0001, 0x7f_fffe, 0x7f_ffff,
        ] {
            for sign in [0u32, 1] {
                let x = f32::from_bits((sign << 31) | (exp << 23) | mant);
                let ctx = format!("w_IS_NAN(0x{:08x})", x.to_bits());
                let (ic, ir) = unsafe { (isnan_c(x), isnan_r(x)) };
                assert_int(&ctx, ic, ir);
                let expect: c_int = if exp == 255 { 1 } else { 0 };
                assert_int(&format!("{ctx} == {expect}"), ic, expect);
            }
        }
    }
    for &x in INTERESTING {
        let ctx = format!("w_IS_NAN({x:?} 0x{:08x})", x.to_bits());
        assert_int(&ctx, unsafe { isnan_c(x) }, unsafe { isnan_r(x) });
    }
    // all NaN payloads (and every other bit pattern) at random
    for _ in 0..20000 {
        let x = f32::from_bits(rng.next_u32());
        let ctx = format!("w_IS_NAN(0x{:08x})", x.to_bits());
        let (ic, ir) = unsafe { (isnan_c(x), isnan_r(x)) };
        assert_int(&ctx, ic, ir);
        let expect: c_int = if (x.to_bits() >> 23) & 0xff == 0xff { 1 } else { 0 };
        assert_int(&format!("{ctx} == {expect}"), ic, expect);
    }
    // the +-inf and NaN extremes spelled out
    for (x, expect) in [
        (f32::INFINITY, 1),
        (f32::NEG_INFINITY, 1),
        (f32::NAN, 1),
        (f32::from_bits(0x7f80_0001), 1), // signalling NaN
        (f32::from_bits(0xffff_ffff), 1),
        (f32::MAX, 0),
        (-f32::MAX, 0),
        (0.0, 0),
        (-0.0, 0),
        (1e-45, 0),
        (f32::MIN_POSITIVE, 0),
    ] {
        let ctx = format!("w_IS_NAN(0x{:08x})", x.to_bits());
        let (ic, ir) = unsafe { (isnan_c(x), isnan_r(x)) };
        assert_int(&ctx, ic, ir);
        assert_int(&format!("{ctx} == {expect}"), ic, expect as c_int);
    }

    // ---- SQRTFAST and Square --------------------------------------------
    let mut vals: Vec<f32> = INTERESTING.to_vec();
    vals.extend([
        4.0, 9.0, 16.0, 100.0, 2.0, 3.0, 0.25, 1e-30, 1e30, 1.8e19, -4.0, -1e30, 1e-45,
        f32::MIN_POSITIVE, 16777216.0, 3.4e38,
    ]);
    for exp in 0u32..=255 {
        for mant in [0u32, 1, 0x40_0000, 0x7f_ffff] {
            for sign in [0u32, 1] {
                vals.push(f32::from_bits((sign << 31) | (exp << 23) | mant));
            }
        }
    }
    for _ in 0..20000 {
        vals.push(rng.f32_any());
    }
    for _ in 0..10000 {
        vals.push(f32::from_bits(rng.next_u32()));
    }

    for &x in &vals {
        let ctx = format!("w_SQRTFAST({x:?} 0x{:08x})", x.to_bits());
        check_f32(&ctx, &[x], unsafe { sqrt_c(x) }, unsafe { sqrt_r(x) });
        let ctx = format!("w_Square({x:?} 0x{:08x})", x.to_bits());
        let (sc, sr) = unsafe { (square_c(x), square_r(x)) };
        check_f32(&ctx, &[x], sc, sr);
        // Square is exactly x*x
        check_f32(&format!("{ctx} == x*x"), &[x], x * x, sc);
    }
}

// ---------------------------------------------------------------------------
// row 57 -- DEG2RAD / RAD2DEG / ANGLE2SHORT / SHORT2ANGLE / ColorIndex
// ---------------------------------------------------------------------------

#[test]
fn deg_rad_and_short_angles() {
    let (deg_c, deg_r): (F32ToF64, F32ToF64) = both_w("w_DEG2RAD");
    let (rad_c, rad_r): (F32ToF64, F32ToF64) = both_w("w_RAD2DEG");
    let (a2s_c, a2s_r): (F32ToI, F32ToI) = both_w("w_ANGLE2SHORT");
    let (s2a_c, s2a_r): (IToF64, IToF64) = both_w("w_SHORT2ANGLE");
    let (ci_c, ci_r): (IToI, IToI) = both_w("w_ColorIndex");
    let mut rng = Rng::new(5701); // row 57

    // the angle pool: the constants the library compares against, the
    // ANGLE2SHORT overflow boundary, and random values
    let mut angles: Vec<f32> = INTERESTING.to_vec();
    angles.extend([
        0.0, -0.0, 1.0, -1.0, 45.0, 90.0, 180.0, 270.0, 360.0, -360.0, 359.999_97, 0.017_453_292,
        57.295_78, 11796480.0, -11796480.0, 11796479.0, -11796479.0, 11796481.0, 2e7, -2e7, 1e-45,
        f32::MIN_POSITIVE, 1e30, -1e30, f32::MAX, -f32::MAX, 3.141_592_7, -3.141_592_7,
        6.283_185_5, 1.570_796_4,
    ]);
    for _ in 0..20000 {
        angles.push(match rng.below(4) {
            0 => rng.f32_any(),
            1 => rng.f32_mag(360.0),
            2 => rng.f32_mag(1.2e7),
            _ => rng.f32_finite(),
        });
    }

    // ---- DEG2RAD: `((a) * M_PI) / 180.0F`, computed in f64 --------------
    for &a in &angles {
        let ctx = format!("w_DEG2RAD({a:?} 0x{:08x})", a.to_bits());
        assert_f64(&ctx, unsafe { deg_c(a) }, unsafe { deg_r(a) });
    }

    // ---- ANGLE2SHORT: `(int)((x)*65536/360) & 65535`, computed in f32 ----
    for &a in &angles {
        let ctx = format!("w_ANGLE2SHORT({a:?} 0x{:08x})", a.to_bits());
        let (ic, ir) = unsafe { (a2s_c(a), a2s_r(a)) };
        assert_int(&ctx, ic, ir);
        assert!(
            (0..=65535).contains(&ic),
            "{ctx}: `& 65535` must leave 0..=65535, got {ic}"
        );
    }

    // ---- SHORT2ANGLE: `(x)*(360.0/65536)`, an f64 result ----------------
    let mut shorts: Vec<i32> = (0..=65535).step_by(7).collect();
    shorts.extend([
        0,
        1,
        -1,
        32767,
        32768,
        -32768,
        65535,
        65536,
        -65536,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
    ]);
    for _ in 0..20000 {
        shorts.push(rng.i32_any());
    }
    for &x in &shorts {
        let ctx = format!("w_SHORT2ANGLE({x})");
        assert_f64(&ctx, unsafe { s2a_c(x) }, unsafe { s2a_r(x) });
    }

    // ---- ColorIndex: `((c) - '0') & 7` ---------------------------------
    // Every one of the 256 `char` values (both as an unsigned byte and as the
    // sign-extended `char` a `char*` deref would produce), then negative ints.
    let mut colors: Vec<i32> = Vec::new();
    for b in 0u32..256 {
        colors.push(b as i32);
        colors.push(b as u8 as i8 as i32);
    }
    colors.extend([-1, -8, -9, -48, -49, -256, -1000, -32768, -65536, -1_000_000]);
    colors.extend([i32::MAX, i32::MAX - 1, 65535, 65536]);
    // `(c) - '0'` overflows for c < INT_MIN + 48: signed overflow, i.e. UB in C,
    // which gcc wraps.  `q_shared::ColorIndex` uses `wrapping_sub` for exactly
    // this reason, so the whole int range is fair game (a plain `-` would panic
    // in a debug-built Rust cdylib instead of wrapping).
    colors.extend([
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 7,
        i32::MIN + 8,
        i32::MIN + 47,
        i32::MIN + 48,
        i32::MIN + 49,
        i32::MIN + 100,
    ]);
    for _ in 0..20000 {
        colors.push(rng.i32_any());
    }
    for &v in &colors {
        let ctx = format!("w_ColorIndex({v})");
        let (ic, ir) = unsafe { (ci_c(v), ci_r(v)) };
        assert_int(&ctx, ic, ir);
        assert!(
            (0..=7).contains(&ic),
            "{ctx}: `& 7` must leave 0..=7, got {ic}"
        );
    }
    for (ch, idx) in [
        ('0', 0),
        ('1', 1),
        ('2', 2),
        ('3', 3),
        ('4', 4),
        ('5', 5),
        ('6', 6),
        ('7', 7),
        ('8', 0),
    ] {
        let v = ch as i32;
        assert_int(
            &format!("w_ColorIndex('{ch}') == {idx}"),
            unsafe { ci_c(v) },
            idx as c_int,
        );
    }

    // ---- RAD2DEG: `((a) * 180.0f) / M_PI` ------------------------------
    // Checked last on purpose: `(a) * 180.0f` is a *float* multiplication in C
    // (both operands are `float`, and FLT_EVAL_METHOD == 0 on x86-64), so the
    // product is rounded to f32 before the f64 division by M_PI.
    for &a in &angles {
        let ctx = format!("w_RAD2DEG({a:?} 0x{:08x})", a.to_bits());
        assert_f64(&ctx, unsafe { rad_c(a) }, unsafe { rad_r(a) });
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 26 -- the (int) overflow inside ANGLE2SHORT
// ---------------------------------------------------------------------------

#[test]
fn angle2short_overflow() {
    let (c, r): (F32ToI, F32ToI) = both_w("w_ANGLE2SHORT");
    let mut rng = Rng::new(2601); // ERRORS.md row 26

    // `(int)(x*65536/360)` overflows once |x| >= 2^31 * 360 / 65536 = 11796480.
    // gcc's cvttss2si then yields 0x80000000, and 0x80000000 & 65535 == 0.
    let mut bad: Vec<f32> = vec![
        11796480.0, // exactly 2^31*360/65536: x*65536/360 == 2^31 exactly
        11796481.0,
        11796520.0,
        12000000.0,
        2e7,
        1e30,
        f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7f80_0001),
        f32::from_bits(0xffc0_0000),
        f32::from_bits(0x7fff_ffff),
    ];
    for _ in 0..2000 {
        let m = 11796480.0f32 * (1.0 + (rng.next_u32() >> 8) as f32 / (1u32 << 24) as f32);
        bad.push(if rng.bool() { m } else { -m });
    }

    for &x in &bad {
        let ctx = format!("w_ANGLE2SHORT({x:?} 0x{:08x})", x.to_bits());
        let (ic, ir) = unsafe { (c(x), r(x)) };
        assert_int(&ctx, ic, ir);
        let scaled = x * 65536.0f32 / 360.0f32;
        if scaled.is_nan() || scaled >= 2147483648.0 || scaled < -2147483648.0 {
            // 0x80000000 & 65535 == 0
            assert_int(&format!("{ctx}: indefinite & 65535 == 0"), ic, 0);
            assert_int(&format!("{ctx}: indefinite & 65535 == 0 (Rust)"), ir, 0);
        }
    }

    // -11796480.0 scales to exactly -2^31, which *is* a valid int, and
    // 0x80000000 & 65535 is 0 as well -- same answer, different reason.
    for &x in &[-11796480.0f32] {
        let ctx = format!("w_ANGLE2SHORT({x:?}) (a valid conversion to INT_MIN)");
        let (ic, ir) = unsafe { (c(x), r(x)) };
        assert_int(&ctx, ic, ir);
        assert_int(&format!("{ctx} == 0"), ic, 0);
    }

    // just inside the boundary the conversion is well defined and non-zero
    for &x in &[11796479.0f32, -11796479.0, 1e6, -1e6, 359.0, 180.0] {
        let ctx = format!("w_ANGLE2SHORT({x:?}) (in range)");
        let (ic, ir) = unsafe { (c(x), r(x)) };
        assert_int(&ctx, ic, ir);
        let scaled = x * 65536.0f32 / 360.0f32;
        assert!(
            scaled < 2147483648.0 && scaled >= -2147483648.0,
            "{ctx}: expected an in-range scaled value, got {scaled}"
        );
    }
}

// ---------------------------------------------------------------------------
// row 58 -- PlaneTypeForNormal
// ---------------------------------------------------------------------------

#[test]
fn plane_type_for_normal() {
    let (c, r): (VecToI, VecToI) = both_w("w_PlaneTypeForNormal");
    let mut rng = Rng::new(5801); // row 58

    /// `PLANE_X`/`PLANE_Y`/`PLANE_Z`/`PLANE_NON_AXIAL` as the macro computes them
    fn model(v: &[f32; 3]) -> c_int {
        if v[0] as f64 == 1.0 {
            0
        } else if v[1] as f64 == 1.0 {
            1
        } else if v[2] as f64 == 1.0 {
            2
        } else {
            3
        }
    }

    let mut vecs: Vec<[f32; 3]> = boundary_vecs();
    // {1, y, z} -> PLANE_X, {x, 1, z} -> PLANE_Y, {x, y, 1} -> PLANE_Z
    let others = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        0.999_999_94, // the largest f32 below 1.0
        1.000_000_1,  // the smallest f32 above 1.0
        0.5,
        2.0,
        1e30,
        1e-45,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ];
    for &y in &others {
        for &z in &others {
            vecs.push([1.0, y, z]);
            vecs.push([y, 1.0, z]);
            vecs.push([y, z, 1.0]);
            vecs.push([y, z, -1.0]);
            vecs.push([y, y, z]);
        }
    }
    for _ in 0..20000 {
        let mut v = rng.vec3_any();
        // make an exact 1.0 turn up often enough to hit all four results
        if rng.below(3) == 0 {
            v[rng.below(3) as usize] = 1.0;
        }
        vecs.push(v);
    }
    for _ in 0..2000 {
        vecs.push(normalized(rng.vec3_mag(1.0)));
    }

    for v in &vecs {
        let ctx = format!("w_PlaneTypeForNormal({v:?} {})", bits(v));
        let (ic, ir) = unsafe { (c(v.as_ptr()), r(v.as_ptr())) };
        assert_int(&ctx, ic, ir);
        assert_int(&format!("{ctx} vs the macro model"), ic, model(v));
    }

    // the four documented results, spelled out
    for (v, expect) in [
        ([1.0f32, 0.0, 0.0], 0),
        ([1.0f32, 1.0, 1.0], 0),
        ([0.0f32, 1.0, 0.0], 1),
        ([2.0f32, 1.0, 1.0], 1),
        ([0.0f32, 0.0, 1.0], 2),
        ([-1.0f32, -1.0, 1.0], 2),
        ([0.0f32, 0.0, 0.0], 3),
        ([-1.0f32, -1.0, -1.0], 3),
        ([0.999_999_94f32, 0.999_999_94, 0.999_999_94], 3),
        ([f32::NAN, f32::NAN, f32::NAN], 3),
    ] {
        let ctx = format!("w_PlaneTypeForNormal({v:?}) == {expect}");
        let (ic, ir) = unsafe { (c(v.as_ptr()), r(v.as_ptr())) };
        assert_int(&ctx, ic, ir);
        assert_int(&ctx, ic, expect as c_int);
    }
}

// ---------------------------------------------------------------------------
// row 60 -- struct layout and constants
// ---------------------------------------------------------------------------

#[test]
fn layout_and_constants() {
    let (layout_c, layout_r): (IntsOut, IntsOut) = both_w("w_layout");
    let (idx_c, idx_r): (IntsOut, IntsOut) = both_w("w_angle_indexes");
    let (pi_c, pi_r): (ToF64, ToF64) = both_w("w_M_PI");

    // ---- w_layout(int out[9]) ------------------------------------------
    // 0 sizeof(cplane_t), 1..5 offsetof(normal, dist, type, signbits, pad),
    // 6 NUMVERTEXNORMALS, 7 nanmask, 8 sizeof(vec_t)
    let mut lc = [-1i32; 9];
    let mut lr = [-1i32; 9];
    unsafe { layout_c(lc.as_mut_ptr()) };
    unsafe { layout_r(lr.as_mut_ptr()) };
    let names = [
        "sizeof(cplane_t)",
        "offsetof(cplane_t, normal)",
        "offsetof(cplane_t, dist)",
        "offsetof(cplane_t, type)",
        "offsetof(cplane_t, signbits)",
        "offsetof(cplane_t, pad)",
        "NUMVERTEXNORMALS",
        "nanmask",
        "sizeof(vec_t)",
    ];
    let expect_layout = [20i32, 0, 12, 16, 17, 18, 162, 255 << 23, 4];
    for i in 0..9 {
        assert_int(&format!("w_layout[{i}] = {}", names[i]), lc[i], lr[i]);
        assert_int(
            &format!("w_layout[{i}] = {} == {}", names[i], expect_layout[i]),
            lc[i],
            expect_layout[i],
        );
    }
    assert_int("nanmask == 255<<23 == 0x7f800000", lc[7], 0x7f80_0000i32);

    // ---- w_angle_indexes(int out[10]) ----------------------------------
    let mut ic = [-1i32; 10];
    let mut ir = [-1i32; 10];
    unsafe { idx_c(ic.as_mut_ptr()) };
    unsafe { idx_r(ir.as_mut_ptr()) };
    let idx_names = [
        "PITCH",
        "YAW",
        "ROLL",
        "PLANE_X",
        "PLANE_Y",
        "PLANE_Z",
        "PLANE_NON_AXIAL",
        "qfalse",
        "qtrue",
        "sizeof(qboolean)",
    ];
    let expect_idx = [0i32, 1, 2, 0, 1, 2, 3, 0, 1, 4];
    for i in 0..10 {
        assert_int(
            &format!("w_angle_indexes[{i}] = {}", idx_names[i]),
            ic[i],
            ir[i],
        );
        assert_int(
            &format!(
                "w_angle_indexes[{i}] = {} == {}",
                idx_names[i], expect_idx[i]
            ),
            ic[i],
            expect_idx[i],
        );
    }

    // ---- M_PI ----------------------------------------------------------
    // q_shared.h guards its own M_PI with `#ifndef M_PI` and <math.h> is
    // included first, so the `double` M_PI of libm wins.
    let (pc, pr) = unsafe { (pi_c(), pi_r()) };
    assert_f64("w_M_PI()", pc, pr);
    assert_f64("w_M_PI() == std::f64::consts::PI", pc, std::f64::consts::PI);
    assert_int(
        "w_M_PI() bit pattern",
        pc.to_bits(),
        std::f64::consts::PI.to_bits(),
    );
    assert!(
        pc != 3.14159265358979323846f32 as f64,
        "w_M_PI() must be the f64 M_PI, not the f32 literal from q_shared.h:277"
    );
}

// ---------------------------------------------------------------------------
// the remaining executable macros of q_shared.h: Q_IsColorString, random,
// crandom (CONFIGS.md rows 66, 67)
// ---------------------------------------------------------------------------

extern "C" {
    /// libc's `srand`, so that the `rand()`-based macros can be compared from a
    /// known state.  The test binary, `libcwrap.so` and `libdriver.so` all share
    /// one glibc, hence one `rand()` state.
    fn srand(seed: core::ffi::c_uint);
}

/// `#define Q_IsColorString(p) ( p && *(p) == '^' && *((p)+1) && *((p)+1) != '^' )`
#[test]
fn q_is_color_string() {
    type F = unsafe extern "C" fn(*const core::ffi::c_char) -> c_int;
    let (c, r): (F, F) = both_w("w_Q_IsColorString");

    // NULL is explicitly allowed by the macro's first operand
    assert_int("w_Q_IsColorString(NULL)", unsafe { c(std::ptr::null()) }, unsafe {
        r(std::ptr::null())
    });
    assert_eq!(unsafe { c(std::ptr::null()) }, 0);

    let cases: &[&[u8]] = &[
        b"\0",
        b"^\0",     // escape but nothing after it
        b"^^\0",    // escaped escape -> not a colour string
        b"^0\0",
        b"^1rest\0",
        b"^9\0",
        b"^a\0",
        b"^ \0",
        b"^\x01\0",
        b"^\x7f\0",
        b"^\xff\0", // high bit set: `char` is signed on x86-64
        b"0^\0",
        b"a^1\0",
        b" ^1\0",
        b"x\0",
        b"^^1\0",
        b"^^^\0",
    ];
    for s in cases {
        let p = s.as_ptr() as *const core::ffi::c_char;
        let ctx = format!("w_Q_IsColorString({:?})", String::from_utf8_lossy(s));
        let (vc, vr) = unsafe { (c(p), r(p)) };
        assert_int(&ctx, vc, vr);
        assert!((0..=1).contains(&vc), "{ctx}: must be 0 or 1, got {vc}");
    }
    // every possible byte after the escape
    for b in 0..=255u8 {
        let buf = [b'^', b, 0u8];
        let p = buf.as_ptr() as *const core::ffi::c_char;
        let (vc, vr) = unsafe { (c(p), r(p)) };
        assert_int(&format!("w_Q_IsColorString(^{b:#04x})"), vc, vr);
    }
}

/// `#define random() ((rand () & 0x7fff) / ((float)0x7fff))` and
/// `#define crandom() (2.0 * (random() - 0.5))`.
///
/// Both expand to libc's `rand()`, whose state is process global and therefore
/// shared by the two libraries; `srand` is called with the same seed before each
/// side so that both consume the same stream.
#[test]
fn random_and_crandom_macros() {
    type F = unsafe extern "C" fn() -> f32;
    let (rand_c, rand_r): (F, F) = both_w("w_random");
    let (crand_c, crand_r): (F, F) = both_w("w_crandom");

    for seed in [0u32, 1, 2, 42, 12345, 0x7fff_ffff, u32::MAX, 69069] {
        unsafe { srand(seed) };
        let cs: Vec<f32> = (0..200).map(|_| unsafe { rand_c() }).collect();
        unsafe { srand(seed) };
        let rs: Vec<f32> = (0..200).map(|_| unsafe { rand_r() }).collect();
        assert_vec(&format!("w_random(srand({seed}))"), &cs, &rs);
        for (i, v) in cs.iter().enumerate() {
            assert!(
                (0.0..=1.0).contains(v),
                "w_random step {i} out of range: {v}"
            );
        }

        unsafe { srand(seed) };
        let cs: Vec<f32> = (0..200).map(|_| unsafe { crand_c() }).collect();
        unsafe { srand(seed) };
        let rs: Vec<f32> = (0..200).map(|_| unsafe { crand_r() }).collect();
        assert_vec(&format!("w_crandom(srand({seed}))"), &cs, &rs);
        for (i, v) in cs.iter().enumerate() {
            assert!(
                (-1.0..=1.0).contains(v),
                "w_crandom step {i} out of range: {v}"
            );
        }
    }
}
