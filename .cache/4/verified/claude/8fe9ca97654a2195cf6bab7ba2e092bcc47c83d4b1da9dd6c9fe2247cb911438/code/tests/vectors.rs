//! Differential tests for the vector entry points of `q_math.c`
//! (CONFIGS.md rows 18-31):
//!
//! `_DotProduct` `_VectorAdd` `_VectorSubtract` `_VectorCopy` `_VectorScale`
//! `_VectorMA` `Vector4Scale` `VectorNormalize` `VectorNormalize2`
//! `NormalizeColor` `RadiusFromBounds` `ClearBounds` `AddPointToBounds`
//! `VectorRotate` `MatrixMultiply`.
//!
//! Every call goes through `dlsym` on both shared objects (never through the
//! Rust functions directly).  Every output buffer is a *guarded* buffer: three
//! (or four) slots pre-filled with the same recognisable garbage on both sides
//! plus one trailing guard element the callee must not touch, so both "did not
//! store" and "stored one element too many" show up as a difference.

mod harness;

use harness::*;

// ---------------------------------------------------------------------------
// the C signatures (vec3_t == float[3] == *mut f32, vec4_t == float[4],
// `vec3_t x[3]` decays to `*mut [f32;3]`)
// ---------------------------------------------------------------------------

/// `vec_t _DotProduct( const vec3_t v1, const vec3_t v2 )`
type FDot = unsafe extern "C" fn(*const f32, *const f32) -> f32;
/// `void _VectorAdd/_VectorSubtract( const vec3_t, const vec3_t, vec3_t )`
type FVVV = unsafe extern "C" fn(*const f32, *const f32, *mut f32);
/// `void _VectorCopy( const vec3_t in, vec3_t out )`
type FCopy = unsafe extern "C" fn(*const f32, *mut f32);
/// `void _VectorScale( const vec3_t, vec_t, vec3_t )` / `Vector4Scale`
type FScale = unsafe extern "C" fn(*const f32, f32, *mut f32);
/// `void _VectorMA( const vec3_t, float, const vec3_t, vec3_t )`
type FMA = unsafe extern "C" fn(*const f32, f32, *const f32, *mut f32);
/// `vec_t VectorNormalize( vec3_t v )`
type FNorm = unsafe extern "C" fn(*mut f32) -> f32;
/// `vec_t VectorNormalize2( const vec3_t, vec3_t )` / `float NormalizeColor(...)`
type FNorm2 = unsafe extern "C" fn(*const f32, *mut f32) -> f32;
/// `float RadiusFromBounds( const vec3_t mins, const vec3_t maxs )`
type FRadius = unsafe extern "C" fn(*const f32, *const f32) -> f32;
/// `void ClearBounds( vec3_t mins, vec3_t maxs )`
type FClear = unsafe extern "C" fn(*mut f32, *mut f32);
/// `void AddPointToBounds( const vec3_t v, vec3_t mins, vec3_t maxs )`
type FAddPt = unsafe extern "C" fn(*const f32, *mut f32, *mut f32);
/// `void VectorRotate( vec3_t in, vec3_t matrix[3], vec3_t out )`
type FRotate = unsafe extern "C" fn(*mut f32, *mut [f32; 3], *mut f32);
/// `void MatrixMultiply(float in1[3][3], float in2[3][3], float out[3][3])`
type FMatMul = unsafe extern "C" fn(*mut [f32; 3], *mut [f32; 3], *mut [f32; 3]);

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Trailing guard element of every output buffer.  A modest finite value, so
/// that including it in a comparison can never make the NaN-payload heuristic
/// of `check_vec` fire.
const GUARD: f32 = 1.5;

/// Pre-filled `vec3_t` output buffer: 3 garbage slots + the guard.
const OUT3: [f32; 4] = [-3.456_789e-8, 1.234_5e7, -7.654_321, GUARD];
/// Pre-filled `vec4_t` output buffer: 4 garbage slots + the guard.
const OUT4: [f32; 5] = [-3.456_789e-8, 1.234_5e7, -7.654_321, 42.125, GUARD];

/// A `vec3_t` input turned into a guarded in/out buffer (for the aliased cases).
fn guarded3(v: &[f32; 3]) -> [f32; 4] {
    [v[0], v[1], v[2], GUARD]
}

/// A `vec4_t` input turned into a guarded in/out buffer.
fn guarded4(v: &[f32; 4]) -> [f32; 5] {
    [v[0], v[1], v[2], v[3], GUARD]
}

/// `[0x3f800000,...]` -- bit patterns, for diagnosable failure messages.
fn bits(v: &[f32]) -> String {
    let mut s = String::from("[");
    for (i, x) in v.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!("{:08x}", x.to_bits()));
    }
    s.push(']');
    s
}

fn flat9(m: &[[f32; 3]; 3]) -> [f32; 9] {
    [
        m[0][0], m[0][1], m[0][2], m[1][0], m[1][1], m[1][2], m[2][0], m[2][1], m[2][2],
    ]
}

/// A 3x3 matrix plus a guard row, laid out exactly like `float[3][3]`.
fn guarded_mat(m: &[[f32; 3]; 3]) -> [[f32; 3]; 4] {
    [m[0], m[1], m[2], [GUARD, GUARD, GUARD]]
}

/// The zero/garbage-filled 3x3 output matrix + guard row.
fn out_mat() -> [[f32; 3]; 4] {
    [
        [OUT3[0], OUT3[1], OUT3[2]],
        [OUT3[1], OUT3[2], OUT3[0]],
        [OUT3[2], OUT3[0], OUT3[1]],
        [GUARD, GUARD, GUARD],
    ]
}

fn flat12(m: &[[f32; 3]; 4]) -> [f32; 12] {
    [
        m[0][0], m[0][1], m[0][2], m[1][0], m[1][1], m[1][2], m[2][0], m[2][1], m[2][2], m[3][0],
        m[3][1], m[3][2],
    ]
}

/// Every input shape of CONFIGS.md axis 2, as `vec3_t`s:
/// `S0 S-0 SA SN SR SH ST SI SNaN`.
fn shape_pool(rng: &mut Rng) -> Vec<[f32; 3]> {
    let mut v: Vec<[f32; 3]> = vec![
        // S0 / S-0
        [0.0, 0.0, 0.0],
        [-0.0, -0.0, -0.0],
        [0.0, -0.0, 0.0],
        // SA -- axial unit vectors, both signs
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [-1.0, 0.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, 0.0, -1.0],
        // SN -- normalized
        [0.577_350_3, 0.577_350_3, 0.577_350_3],
        [0.6, 0.8, 0.0],
        [-0.267_261_24, 0.534_522_5, -0.801_783_7],
        // SR -- arbitrary finite
        [1.0, 2.0, 3.0],
        [3.0, 4.0, 0.0],
        [-1.5, 2.25, -3.75],
        [99999.0, -99999.0, 0.5],
        [255.0, 1.0 / 255.0, -180.0],
        // SH -- squares overflow to +inf
        [1e30, 1e30, 1e30],
        [f32::MAX, f32::MAX, f32::MAX],
        [-1e30, 1e30, -1e30],
        [1.8e19, 0.0, 0.0],  // 1.8e19^2 is still finite
        [1.85e19, 0.0, 0.0], // 1.85e19^2 == +inf
        // ST -- squares underflow to 0
        [1e-30, 0.0, 0.0],
        [1e-30, 1e-30, 1e-30],
        [1e-45, -1e-45, 1e-45],
        [f32::MIN_POSITIVE, f32::MIN_POSITIVE, f32::MIN_POSITIVE],
        // SI
        [f32::INFINITY, 0.0, 0.0],
        [f32::NEG_INFINITY, 0.0, 0.0],
        [f32::INFINITY, f32::NEG_INFINITY, 0.0],
        [f32::INFINITY, f32::INFINITY, f32::INFINITY],
        // SNaN
        [f32::NAN, 0.0, 0.0],
        [0.0, f32::NAN, 0.0],
        [0.0, 0.0, f32::NAN],
        [f32::NAN, f32::NAN, f32::NAN],
        [f32::NAN, f32::INFINITY, 1.0],
        [f32::NAN, 1e30, -1.0],
        [1.0, f32::NAN, f32::INFINITY],
    ];
    for _ in 0..6 {
        v.push(rng.dir());
    }
    for _ in 0..6 {
        v.push(rng.vec3_finite());
    }
    for _ in 0..6 {
        v.push(rng.vec3_mag(1.0));
    }
    v
}

/// `vec4_t` flavour of [`shape_pool`]: the same shapes with a rotating 4th
/// component drawn from the same set of interesting values.
fn shape_pool4(rng: &mut Rng) -> Vec<[f32; 4]> {
    let base = shape_pool(rng);
    let extra = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        1e30,
        1e-30,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::MAX,
        1e-45,
        0.5,
    ];
    base.iter()
        .enumerate()
        .map(|(i, v)| [v[0], v[1], v[2], extra[i % extra.len()]])
        .collect()
}

/// Scale factors of CONFIGS.md row 21: `0 -0 1 -1 inf NaN denormal ...`.
fn scale_pool(rng: &mut Rng) -> Vec<f32> {
    let mut s: Vec<f32> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        2.0,
        -2.0,
        255.0,
        1.0 / 255.0,
        99999.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        1e-45,
        -1e-45,
        1e-30,
        1e30,
        -1e30,
        f32::MAX,
        -f32::MAX,
    ];
    for _ in 0..8 {
        s.push(rng.f32_any());
    }
    s
}

/// Matrices for rows 30/31: identity, zero, permutation, singular, `SH`, `SNaN`.
fn matrix_pool(rng: &mut Rng) -> Vec<[[f32; 3]; 3]> {
    let mut m: Vec<[[f32; 3]; 3]> = vec![
        // identity
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        // zero / negative zero
        [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
        [[-0.0, -0.0, -0.0], [-0.0, -0.0, -0.0], [-0.0, -0.0, -0.0]],
        // permutations
        [[0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]],
        [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]],
        // mirror
        [[-1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, -1.0]],
        // singular (rank 2 and rank 1)
        [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]],
        [[1.0, 1.0, 1.0], [1.0, 1.0, 1.0], [1.0, 1.0, 1.0]],
        [[1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
        // rotation about z by 30 degrees
        [
            [0.866_025_4, 0.5, 0.0],
            [-0.5, 0.866_025_4, 0.0],
            [0.0, 0.0, 1.0],
        ],
        // SH -- products overflow
        [[1e30, 1e30, 1e30], [1e30, 1e30, 1e30], [1e30, 1e30, 1e30]],
        [
            [f32::MAX, 0.0, 0.0],
            [0.0, f32::MAX, 0.0],
            [0.0, 0.0, f32::MAX],
        ],
        // ST
        [
            [1e-30, 1e-30, 1e-30],
            [1e-45, 1e-45, 1e-45],
            [0.0, 0.0, 0.0],
        ],
        // SI
        [
            [f32::INFINITY, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, f32::NEG_INFINITY],
        ],
        // SNaN
        [[f32::NAN, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        [
            [f32::NAN, f32::NAN, f32::NAN],
            [f32::NAN, f32::NAN, f32::NAN],
            [f32::NAN, f32::NAN, f32::NAN],
        ],
        [
            [f32::NAN, f32::INFINITY, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
        ],
    ];
    for _ in 0..4 {
        m.push([rng.vec3_finite(), rng.vec3_finite(), rng.vec3_finite()]);
    }
    for _ in 0..4 {
        m.push([rng.vec3_mag(1.0), rng.vec3_mag(1.0), rng.vec3_mag(1.0)]);
    }
    for _ in 0..2 {
        m.push([rng.vec3_any(), rng.vec3_any(), rng.vec3_any()]);
    }
    m
}

// ---------------------------------------------------------------------------
// row 18 -- _DotProduct
// ---------------------------------------------------------------------------

#[test]
fn dot_product() {
    let (c, r): (FDot, FDot) = both("_DotProduct");
    let mut rng = Rng::new(18);
    let pool = shape_pool(&mut rng);

    // every ordered pair of shapes: S0 x SA x SN x SR x SH x ST x SI x SNaN
    for a in &pool {
        for b in &pool {
            let inputs = [a[0], a[1], a[2], b[0], b[1], b[2]];
            let ctx = format!(
                "_DotProduct(v1={a:?} {}, v2={b:?} {})",
                bits(a),
                bits(b)
            );
            check_f32(&ctx, &inputs, unsafe { c(a.as_ptr(), b.as_ptr()) }, unsafe {
                r(a.as_ptr(), b.as_ptr())
            });
        }
    }

    // SEQ -- the SAME pointer passed as both arguments
    for a in &pool {
        let inputs = [a[0], a[1], a[2], a[0], a[1], a[2]];
        let ctx = format!("_DotProduct(v,v) with one pointer, v={a:?} {}", bits(a));
        check_f32(&ctx, &inputs, unsafe { c(a.as_ptr(), a.as_ptr()) }, unsafe {
            r(a.as_ptr(), a.as_ptr())
        });
    }

    // 20000 random pairs
    for i in 0..20000 {
        let a = rng.vec3_any();
        let b = rng.vec3_any();
        let inputs = [a[0], a[1], a[2], b[0], b[1], b[2]];
        let ctx = format!(
            "_DotProduct #{i} v1={a:?} {} v2={b:?} {}",
            bits(&a),
            bits(&b)
        );
        check_f32(&ctx, &inputs, unsafe { c(a.as_ptr(), b.as_ptr()) }, unsafe {
            r(a.as_ptr(), b.as_ptr())
        });
        // ... and the same vector against itself
        let ctx = format!("_DotProduct #{i} (v,v) v={a:?} {}", bits(&a));
        let inputs = [a[0], a[1], a[2], a[0], a[1], a[2]];
        check_f32(&ctx, &inputs, unsafe { c(a.as_ptr(), a.as_ptr()) }, unsafe {
            r(a.as_ptr(), a.as_ptr())
        });
    }
}

// ---------------------------------------------------------------------------
// row 19 -- _VectorAdd / _VectorSubtract
// ---------------------------------------------------------------------------

/// Runs one `(const vec3, const vec3, vec3 out)` pair on guarded output buffers
/// that start from the same garbage on both sides.
fn run_vvv(ctx: &str, fc: FVVV, fr: FVVV, a: &[f32; 3], b: &[f32; 3]) {
    let mut oc = OUT3;
    let mut or_ = OUT3;
    unsafe { fc(a.as_ptr(), b.as_ptr(), oc.as_mut_ptr()) };
    unsafe { fr(a.as_ptr(), b.as_ptr(), or_.as_mut_ptr()) };
    let inputs = [a[0], a[1], a[2], b[0], b[1], b[2]];
    check_vec(ctx, &inputs, &oc, &or_);
}

#[test]
fn vector_add_sub() {
    let (add_c, add_r): (FVVV, FVVV) = both("_VectorAdd");
    let (sub_c, sub_r): (FVVV, FVVV) = both("_VectorSubtract");
    let mut rng = Rng::new(19);
    let pool = shape_pool(&mut rng);

    // every ordered pair of shapes -- SH overflows to inf, SI gives inf-inf,
    // S-0 gives 0 + -0 and 0 - 0.
    for a in &pool {
        for b in &pool {
            run_vvv(
                &format!("_VectorAdd({a:?},{b:?}) {} {}", bits(a), bits(b)),
                add_c,
                add_r,
                a,
                b,
            );
            run_vvv(
                &format!("_VectorSubtract({a:?},{b:?}) {} {}", bits(a), bits(b)),
                sub_c,
                sub_r,
                a,
                b,
            );
        }
    }

    // the signed-zero and inf-inf identities, spelled out
    let zeros: &[([f32; 3], [f32; 3])] = &[
        ([0.0, 0.0, 0.0], [-0.0, -0.0, -0.0]),
        ([-0.0, -0.0, -0.0], [0.0, 0.0, 0.0]),
        ([-0.0, -0.0, -0.0], [-0.0, -0.0, -0.0]),
        ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]),
        (
            [f32::INFINITY, f32::NEG_INFINITY, f32::INFINITY],
            [f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY],
        ),
        (
            [f32::MAX, -f32::MAX, f32::MAX],
            [f32::MAX, -f32::MAX, -f32::MAX],
        ),
        (
            [1e-45, -1e-45, f32::MIN_POSITIVE],
            [1e-45, 1e-45, -f32::MIN_POSITIVE],
        ),
    ];
    for (a, b) in zeros {
        run_vvv(
            &format!("_VectorAdd(boundary {a:?},{b:?})"),
            add_c,
            add_r,
            a,
            b,
        );
        run_vvv(
            &format!("_VectorSubtract(boundary {a:?},{b:?})"),
            sub_c,
            sub_r,
            a,
            b,
        );
    }

    // 20000 random pairs
    for i in 0..20000 {
        let a = rng.vec3_any();
        let b = rng.vec3_any();
        run_vvv(
            &format!("_VectorAdd #{i} {a:?} {b:?} {} {}", bits(&a), bits(&b)),
            add_c,
            add_r,
            &a,
            &b,
        );
        run_vvv(
            &format!(
                "_VectorSubtract #{i} {a:?} {b:?} {} {}",
                bits(&a),
                bits(&b)
            ),
            sub_c,
            sub_r,
            &a,
            &b,
        );
    }
}

// ---------------------------------------------------------------------------
// row 20 -- aliased in/out pointers (SAL)
// ---------------------------------------------------------------------------

/// The C code writes and reads its parameters in source order, so passing the
/// same raw pointer for several parameters is well defined; both translations
/// must produce the same sequence of stores.
#[test]
fn vector_ops_aliasing() {
    let (add_c, add_r): (FVVV, FVVV) = both("_VectorAdd");
    let (sub_c, sub_r): (FVVV, FVVV) = both("_VectorSubtract");
    let (scale_c, scale_r): (FScale, FScale) = both("_VectorScale");
    let (copy_c, copy_r): (FCopy, FCopy) = both("_VectorCopy");
    let (ma_c, ma_r): (FMA, FMA) = both("_VectorMA");
    let (v4_c, v4_r): (FScale, FScale) = both("Vector4Scale");
    let mut rng = Rng::new(20);

    let pool = shape_pool(&mut rng);
    let pool4 = shape_pool4(&mut rng);
    let scales = scale_pool(&mut rng);

    // deterministic combinations first, then random ones
    let mut cases: Vec<([f32; 3], [f32; 3], f32, [f32; 4])> = Vec::new();
    for i in 0..pool.len() {
        for k in 0..3usize {
            let a = pool[i];
            let b = pool[(i + 1 + k * 7) % pool.len()];
            let s = scales[(i + k) % scales.len()];
            let q = pool4[(i + k * 5) % pool4.len()];
            cases.push((a, b, s, q));
        }
    }
    for _ in 0..6000 {
        cases.push((
            rng.vec3_any(),
            rng.vec3_any(),
            rng.f32_any(),
            rng.vec4_any(),
        ));
    }

    for (n, (a, b, s, q)) in cases.iter().enumerate() {
        let in6 = [a[0], a[1], a[2], b[0], b[1], b[2]];
        let in3s = [a[0], a[1], a[2], *s];
        let in7 = [a[0], a[1], a[2], *s, b[0], b[1], b[2]];
        let in4s = [q[0], q[1], q[2], q[3], *s];
        let tag = format!(
            "#{n} a={a:?}{} b={b:?}{} s={s:?}(0x{:08x}) q={q:?}{}",
            bits(a),
            bits(b),
            s.to_bits(),
            bits(q)
        );

        for (name, fc, fr) in [
            ("_VectorAdd", add_c, add_r),
            ("_VectorSubtract", sub_c, sub_r),
        ] {
            // out == first input
            {
                let mut xc = guarded3(a);
                let mut xr = guarded3(a);
                let yc = guarded3(b);
                let pc = xc.as_mut_ptr();
                unsafe { fc(pc, yc.as_ptr(), pc) };
                let pr = xr.as_mut_ptr();
                unsafe { fr(pr, yc.as_ptr(), pr) };
                check_vec(&format!("{name}(a,b,a) {tag}"), &in6, &xc, &xr);
            }
            // out == second input
            {
                let xc = guarded3(a);
                let mut yc = guarded3(b);
                let mut yr = guarded3(b);
                let pc = yc.as_mut_ptr();
                unsafe { fc(xc.as_ptr(), pc, pc) };
                let pr = yr.as_mut_ptr();
                unsafe { fr(xc.as_ptr(), pr, pr) };
                check_vec(&format!("{name}(a,b,b) {tag}"), &in6, &yc, &yr);
            }
            // both inputs are the same pointer, separate output
            {
                let xc = guarded3(a);
                let mut oc = OUT3;
                let mut or_ = OUT3;
                unsafe { fc(xc.as_ptr(), xc.as_ptr(), oc.as_mut_ptr()) };
                unsafe { fr(xc.as_ptr(), xc.as_ptr(), or_.as_mut_ptr()) };
                let in_aa = [a[0], a[1], a[2], a[0], a[1], a[2]];
                check_vec(&format!("{name}(a,a,out) {tag}"), &in_aa, &oc, &or_);
            }
            // all three the same pointer
            {
                let mut xc = guarded3(a);
                let mut xr = guarded3(a);
                let pc = xc.as_mut_ptr();
                unsafe { fc(pc, pc, pc) };
                let pr = xr.as_mut_ptr();
                unsafe { fr(pr, pr, pr) };
                let in_aa = [a[0], a[1], a[2], a[0], a[1], a[2]];
                check_vec(&format!("{name}(a,a,a) {tag}"), &in_aa, &xc, &xr);
            }
        }

        // _VectorScale: out == in
        {
            let mut xc = guarded3(a);
            let mut xr = guarded3(a);
            let pc = xc.as_mut_ptr();
            unsafe { scale_c(pc, *s, pc) };
            let pr = xr.as_mut_ptr();
            unsafe { scale_r(pr, *s, pr) };
            check_vec(&format!("_VectorScale(a,s,a) {tag}"), &in3s, &xc, &xr);
        }
        // _VectorScale: separate out (baseline)
        {
            let xc = guarded3(a);
            let mut oc = OUT3;
            let mut or_ = OUT3;
            unsafe { scale_c(xc.as_ptr(), *s, oc.as_mut_ptr()) };
            unsafe { scale_r(xc.as_ptr(), *s, or_.as_mut_ptr()) };
            check_vec(&format!("_VectorScale(a,s,out) {tag}"), &in3s, &oc, &or_);
        }

        // _VectorCopy: out == in, and separate out
        {
            let mut xc = guarded3(a);
            let mut xr = guarded3(a);
            let pc = xc.as_mut_ptr();
            unsafe { copy_c(pc, pc) };
            let pr = xr.as_mut_ptr();
            unsafe { copy_r(pr, pr) };
            assert_vec(&format!("_VectorCopy(a,a) {tag}"), &xc, &xr);

            let mut oc = OUT3;
            let mut or_ = OUT3;
            unsafe { copy_c(xc.as_ptr(), oc.as_mut_ptr()) };
            unsafe { copy_r(xr.as_ptr(), or_.as_mut_ptr()) };
            assert_vec(&format!("_VectorCopy(a,out) {tag}"), &oc, &or_);
        }

        // _VectorMA(veca, scale, vecb, vecc)
        {
            // vecc == veca
            let mut xc = guarded3(a);
            let mut xr = guarded3(a);
            let yc = guarded3(b);
            let pc = xc.as_mut_ptr();
            unsafe { ma_c(pc, *s, yc.as_ptr(), pc) };
            let pr = xr.as_mut_ptr();
            unsafe { ma_r(pr, *s, yc.as_ptr(), pr) };
            check_vec(&format!("_VectorMA(a,s,b,a) {tag}"), &in7, &xc, &xr);
        }
        {
            // vecc == vecb
            let xc = guarded3(a);
            let mut yc = guarded3(b);
            let mut yr = guarded3(b);
            let pc = yc.as_mut_ptr();
            unsafe { ma_c(xc.as_ptr(), *s, pc, pc) };
            let pr = yr.as_mut_ptr();
            unsafe { ma_r(xc.as_ptr(), *s, pr, pr) };
            check_vec(&format!("_VectorMA(a,s,b,b) {tag}"), &in7, &yc, &yr);
        }
        {
            // veca == vecb, separate output
            let xc = guarded3(a);
            let mut oc = OUT3;
            let mut or_ = OUT3;
            unsafe { ma_c(xc.as_ptr(), *s, xc.as_ptr(), oc.as_mut_ptr()) };
            unsafe { ma_r(xc.as_ptr(), *s, xc.as_ptr(), or_.as_mut_ptr()) };
            let in_aa = [a[0], a[1], a[2], *s, a[0], a[1], a[2]];
            check_vec(&format!("_VectorMA(a,s,a,out) {tag}"), &in_aa, &oc, &or_);
        }
        {
            // all three the same pointer
            let mut xc = guarded3(a);
            let mut xr = guarded3(a);
            let pc = xc.as_mut_ptr();
            unsafe { ma_c(pc, *s, pc, pc) };
            let pr = xr.as_mut_ptr();
            unsafe { ma_r(pr, *s, pr, pr) };
            let in_aa = [a[0], a[1], a[2], *s, a[0], a[1], a[2]];
            check_vec(&format!("_VectorMA(a,s,a,a) {tag}"), &in_aa, &xc, &xr);
        }

        // Vector4Scale: out == in, and separate out
        {
            let mut xc = guarded4(q);
            let mut xr = guarded4(q);
            let pc = xc.as_mut_ptr();
            unsafe { v4_c(pc, *s, pc) };
            let pr = xr.as_mut_ptr();
            unsafe { v4_r(pr, *s, pr) };
            check_vec(&format!("Vector4Scale(q,s,q) {tag}"), &in4s, &xc, &xr);
        }
        {
            let xc = guarded4(q);
            let mut oc = OUT4;
            let mut or_ = OUT4;
            unsafe { v4_c(xc.as_ptr(), *s, oc.as_mut_ptr()) };
            unsafe { v4_r(xc.as_ptr(), *s, or_.as_mut_ptr()) };
            check_vec(&format!("Vector4Scale(q,s,out) {tag}"), &in4s, &oc, &or_);
        }
    }
}

// ---------------------------------------------------------------------------
// row 21 -- _VectorScale / Vector4Scale
// ---------------------------------------------------------------------------

#[test]
fn vector_scale() {
    let (c3, r3): (FScale, FScale) = both("_VectorScale");
    let (c4, r4): (FScale, FScale) = both("Vector4Scale");
    let mut rng = Rng::new(21);

    let pool = shape_pool(&mut rng);
    let pool4 = shape_pool4(&mut rng);
    let scales = scale_pool(&mut rng);

    for v in &pool {
        for &s in &scales {
            let inputs = [v[0], v[1], v[2], s];
            let mut oc = OUT3;
            let mut or_ = OUT3;
            unsafe { c3(v.as_ptr(), s, oc.as_mut_ptr()) };
            unsafe { r3(v.as_ptr(), s, or_.as_mut_ptr()) };
            check_vec(
                &format!(
                    "_VectorScale({v:?}{}, {s:?}(0x{:08x}))",
                    bits(v),
                    s.to_bits()
                ),
                &inputs,
                &oc,
                &or_,
            );
        }
    }

    for v in &pool4 {
        for &s in &scales {
            let inputs = [v[0], v[1], v[2], v[3], s];
            let mut oc = OUT4;
            let mut or_ = OUT4;
            unsafe { c4(v.as_ptr(), s, oc.as_mut_ptr()) };
            unsafe { r4(v.as_ptr(), s, or_.as_mut_ptr()) };
            check_vec(
                &format!(
                    "Vector4Scale({v:?}{}, {s:?}(0x{:08x}))",
                    bits(v),
                    s.to_bits()
                ),
                &inputs,
                &oc,
                &or_,
            );
        }
    }

    for i in 0..20000 {
        let v = rng.vec3_any();
        let q = rng.vec4_any();
        let s = rng.f32_any();

        let mut oc = OUT3;
        let mut or_ = OUT3;
        unsafe { c3(v.as_ptr(), s, oc.as_mut_ptr()) };
        unsafe { r3(v.as_ptr(), s, or_.as_mut_ptr()) };
        check_vec(
            &format!(
                "_VectorScale #{i} {v:?}{} * {s:?}(0x{:08x})",
                bits(&v),
                s.to_bits()
            ),
            &[v[0], v[1], v[2], s],
            &oc,
            &or_,
        );

        let mut oc = OUT4;
        let mut or_ = OUT4;
        unsafe { c4(q.as_ptr(), s, oc.as_mut_ptr()) };
        unsafe { r4(q.as_ptr(), s, or_.as_mut_ptr()) };
        check_vec(
            &format!(
                "Vector4Scale #{i} {q:?}{} * {s:?}(0x{:08x})",
                bits(&q),
                s.to_bits()
            ),
            &[q[0], q[1], q[2], q[3], s],
            &oc,
            &or_,
        );
    }
}

// ---------------------------------------------------------------------------
// row 22 -- _VectorMA
// ---------------------------------------------------------------------------

#[test]
fn vector_ma() {
    let (c, r): (FMA, FMA) = both("_VectorMA");
    let mut rng = Rng::new(22);

    let pool = shape_pool(&mut rng);
    let scales = scale_pool(&mut rng);

    let run = |ctx: String, a: &[f32; 3], s: f32, b: &[f32; 3]| {
        let mut oc = OUT3;
        let mut or_ = OUT3;
        unsafe { c(a.as_ptr(), s, b.as_ptr(), oc.as_mut_ptr()) };
        unsafe { r(a.as_ptr(), s, b.as_ptr(), or_.as_mut_ptr()) };
        let inputs = [a[0], a[1], a[2], s, b[0], b[1], b[2]];
        check_vec(&ctx, &inputs, &oc, &or_);
    };

    // scale x veca x vecb, sampled across the whole cross product
    for &s in &scales {
        for i in 0..pool.len() {
            for k in 0..3usize {
                let a = &pool[i];
                let b = &pool[(i + 1 + k * 11) % pool.len()];
                run(
                    format!(
                        "_VectorMA({a:?}{}, {s:?}(0x{:08x}), {b:?}{})",
                        bits(a),
                        s.to_bits(),
                        bits(b)
                    ),
                    a,
                    s,
                    b,
                );
            }
        }
    }

    // the 0 * inf cases spelled out (scale zero x infinite vector and the
    // reverse), which manufacture the default NaN inside the multiplication
    let inf = f32::INFINITY;
    for (a, s, b) in [
        ([0.0f32, 0.0, 0.0], 0.0f32, [inf, -inf, inf]),
        ([1.0, -1.0, 0.0], -0.0, [inf, inf, -inf]),
        ([inf, -inf, 0.0], 0.0, [inf, -inf, inf]),
        ([0.0, 0.0, 0.0], inf, [0.0, -0.0, 0.0]),
        ([-0.0, 0.0, 1.0], inf, [0.0, 0.0, -0.0]),
        ([1.0, 2.0, 3.0], 0.0, [f32::NAN, f32::NAN, f32::NAN]),
        ([f32::NAN, 0.0, 1.0], 0.0, [inf, 0.0, 0.0]),
        ([1e30, 1e30, 1e30], 1e30, [1e30, -1e30, 1e30]),
        ([f32::MAX, -f32::MAX, 0.0], 2.0, [f32::MAX, f32::MAX, 0.0]),
        ([1e-30, 1e-30, 0.0], 1e-30, [1e-30, -1e-30, 1e-45]),
    ] {
        run(
            format!("_VectorMA(0*inf case {a:?}, {s:?}, {b:?})"),
            &a,
            s,
            &b,
        );
    }

    for i in 0..20000 {
        let a = rng.vec3_any();
        let b = rng.vec3_any();
        let s = rng.f32_any();
        run(
            format!(
                "_VectorMA #{i} {a:?}{} {s:?}(0x{:08x}) {b:?}{}",
                bits(&a),
                s.to_bits(),
                bits(&b)
            ),
            &a,
            s,
            &b,
        );
    }
}

// ---------------------------------------------------------------------------
// row 23 -- _VectorCopy copies bits verbatim
// ---------------------------------------------------------------------------

/// No arithmetic happens in `_VectorCopy`, so *every* bit pattern -- including
/// arbitrary NaN payloads and signalling NaNs -- must survive unchanged.  This
/// is compared with `assert_vec`, never with the NaN-tolerant `check_vec`.
#[test]
fn vector_copy_bits() {
    let (c, r): (FCopy, FCopy) = both("_VectorCopy");
    let mut rng = Rng::new(23);

    for i in 0..20000 {
        let v = [
            f32::from_bits(rng.next_u32()),
            f32::from_bits(rng.next_u32()),
            f32::from_bits(rng.next_u32()),
        ];
        let mut oc = OUT3;
        let mut or_ = OUT3;
        unsafe { c(v.as_ptr(), oc.as_mut_ptr()) };
        unsafe { r(v.as_ptr(), or_.as_mut_ptr()) };
        assert_vec(&format!("_VectorCopy #{i} in={}", bits(&v)), &oc, &or_);
        // the copy must actually equal the input, bit for bit
        assert_vec(
            &format!("_VectorCopy #{i} in={} vs C out", bits(&v)),
            &v,
            &oc[0..3],
        );
    }

    // every NaN payload class, both signs, plus the exponent extremes
    let mut patterns: Vec<u32> = vec![
        0x0000_0000,
        0x8000_0000,
        0x0000_0001,
        0x8000_0001,
        0x007f_ffff,
        0x0080_0000,
        0x3f80_0000,
        0xbf80_0000,
        0x7f7f_ffff,
        0xff7f_ffff,
        0x7f80_0000, // +inf
        0xff80_0000, // -inf
        0x7f80_0001, // smallest signalling NaN
        0xff80_0001,
        0x7fbf_ffff, // largest signalling NaN
        0x7fc0_0000, // canonical quiet NaN
        0xffc0_0000, // x86 "real indefinite"
        0x7fff_ffff,
        0xffff_ffff,
        0x7fca_fe42,
        0xffde_ad00,
    ];
    patterns.extend((0..32).map(|b| 1u32 << b));
    for &p in &patterns {
        for &q in &patterns {
            let v = [f32::from_bits(p), f32::from_bits(q), f32::from_bits(p ^ q)];
            let mut oc = OUT3;
            let mut or_ = OUT3;
            unsafe { c(v.as_ptr(), oc.as_mut_ptr()) };
            unsafe { r(v.as_ptr(), or_.as_mut_ptr()) };
            assert_vec(&format!("_VectorCopy(pattern {})", bits(&v)), &oc, &or_);
            assert_vec(
                &format!("_VectorCopy(pattern {}) vs C out", bits(&v)),
                &v,
                &oc[0..3],
            );
        }
    }
}

// ---------------------------------------------------------------------------
// row 24 -- VectorNormalize (in place)
// ---------------------------------------------------------------------------

/// The three length classes: `length != 0`, `length == 0` (both the zero vector
/// and `{1e-30,0,0}`, whose square underflows to `0`), and a NaN length -- for
/// which `if (length)` is *true*, so the division is performed.
fn normalize_cases(rng: &mut Rng) -> Vec<[f32; 3]> {
    let mut v = shape_pool(rng);
    v.extend_from_slice(&[
        // length == 0 through underflow
        [1e-30, 0.0, 0.0],
        [0.0, 1e-30, 0.0],
        [0.0, 0.0, 1e-30],
        [1e-30, 1e-30, 1e-30],
        [1e-45, 1e-45, 1e-45],
        [-1e-30, 0.0, 0.0],
        [1.0e-23, 0.0, 0.0], // 1e-46 -> underflows to 0
        // right at the edge where the square is the smallest normal
        [1.084_202_2e-19, 0.0, 0.0],
        [1.084_202_3e-19, 0.0, 0.0],
        // exactly unit length
        [1.0, 0.0, 0.0],
        [0.6, 0.8, 0.0],
        // length that is a power of two
        [4.0, 0.0, 0.0],
        [2.0, 2.0, 2.0],
        // NaN / inf lengths
        [f32::NAN, 0.0, 0.0],
        [f32::INFINITY, 1.0, 0.0],
        [f32::NEG_INFINITY, f32::INFINITY, 0.0],
        [1e30, 1e30, 1e30],
    ]);
    for _ in 0..40 {
        v.push(rng.vec3_mag(1.0));
    }
    for _ in 0..40 {
        v.push(rng.vec3_finite());
    }
    v
}

#[test]
fn vector_normalize() {
    let (c, r): (FNorm, FNorm) = both("VectorNormalize");
    let mut rng = Rng::new(24);

    let mut cases = normalize_cases(&mut rng);
    for _ in 0..20000 {
        cases.push(rng.vec3_any());
    }

    for (i, v) in cases.iter().enumerate() {
        // guarded, identically pre-filled in/out buffers (the function works in
        // place, so `SAL` is the only mode there is)
        let mut vc = guarded3(v);
        let mut vr = guarded3(v);
        let lc = unsafe { c(vc.as_mut_ptr()) };
        let lr = unsafe { r(vr.as_mut_ptr()) };
        let ctx = format!("VectorNormalize #{i} v={v:?}{}", bits(v));
        check_f32(&format!("{ctx} -- returned length"), v, lc, lr);
        check_vec(&format!("{ctx} -- vector"), v, &vc, &vr);
    }
}

// ---------------------------------------------------------------------------
// row 25 -- VectorNormalize2 (separate output, and out == v)
// ---------------------------------------------------------------------------

#[test]
fn vector_normalize2() {
    let (c, r): (FNorm2, FNorm2) = both("VectorNormalize2");
    let mut rng = Rng::new(25);

    let mut cases = normalize_cases(&mut rng);
    for _ in 0..20000 {
        cases.push(rng.vec3_any());
    }

    for (i, v) in cases.iter().enumerate() {
        // separate output buffer, pre-filled identically on both sides
        {
            let mut oc = OUT3;
            let mut or_ = OUT3;
            let lc = unsafe { c(v.as_ptr(), oc.as_mut_ptr()) };
            let lr = unsafe { r(v.as_ptr(), or_.as_mut_ptr()) };
            let ctx = format!("VectorNormalize2 #{i} v={v:?}{} -> out", bits(v));
            check_f32(&format!("{ctx} -- returned length"), v, lc, lr);
            check_vec(&format!("{ctx} -- out"), v, &oc, &or_);
        }
        // SAL: out == v
        {
            let mut vc = guarded3(v);
            let mut vr = guarded3(v);
            let pc = vc.as_mut_ptr();
            let lc = unsafe { c(pc, pc) };
            let pr = vr.as_mut_ptr();
            let lr = unsafe { r(pr, pr) };
            let ctx = format!("VectorNormalize2 #{i} v={v:?}{} aliased", bits(v));
            check_f32(&format!("{ctx} -- returned length"), v, lc, lr);
            check_vec(&format!("{ctx} -- vector"), v, &vc, &vr);
        }
    }
}

// ---------------------------------------------------------------------------
// row 26 -- NormalizeColor
// ---------------------------------------------------------------------------

#[test]
fn normalize_color() {
    let (c, r): (FNorm2, FNorm2) = both("NormalizeColor");
    let mut rng = Rng::new(26);

    let mut cases: Vec<[f32; 3]> = vec![
        // max comes from in[0] / in[1] / in[2]
        [3.0, 2.0, 1.0],
        [1.0, 3.0, 2.0],
        [1.0, 2.0, 3.0],
        [1.0, 1.0, 1.0],
        // max == 0 (both signed zeroes -- `!max` is true for -0.0 too)
        [0.0, 0.0, 0.0],
        [-0.0, -0.0, -0.0],
        [-0.0, -1.0, -2.0],
        [-1.0, -0.0, -2.0],
        [-1.0, -2.0, -0.0],
        [0.0, -0.0, -0.0],
        [-1.0, 0.0, -1.0],
        // max < 0
        [-1.0, -2.0, -3.0],
        [-3.0, -2.0, -1.0],
        [-1e-45, -1e30, -1.0],
        // the usual colour range
        [1.0, 0.5, 0.25],
        [0.75, 0.75, 0.75],
        [255.0, 128.0, 0.0],
        [1.0 / 255.0, 0.0, 0.0],
        // SNaN -- every `>` comparison against a NaN is false
        [f32::NAN, 1.0, 2.0],
        [1.0, f32::NAN, 2.0],
        [1.0, 2.0, f32::NAN],
        [f32::NAN, f32::NAN, f32::NAN],
        [f32::NAN, 0.0, 0.0],
        [0.0, f32::NAN, 0.0],
        // SI / SH / ST
        [f32::INFINITY, 1.0, 2.0],
        [f32::NEG_INFINITY, -1.0, -2.0],
        [f32::INFINITY, f32::NEG_INFINITY, 0.0],
        [1e30, 1e-30, -1e30],
        [f32::MAX, f32::MIN_POSITIVE, -f32::MAX],
        [1e-45, 1e-45, 1e-45],
        [f32::MIN_POSITIVE, 0.0, -f32::MIN_POSITIVE],
    ];
    for _ in 0..20000 {
        cases.push(rng.vec3_any());
    }
    for _ in 0..2000 {
        cases.push(rng.vec3_mag(1.0));
    }

    for (i, v) in cases.iter().enumerate() {
        // separate output
        {
            let mut oc = OUT3;
            let mut or_ = OUT3;
            let mc = unsafe { c(v.as_ptr(), oc.as_mut_ptr()) };
            let mr = unsafe { r(v.as_ptr(), or_.as_mut_ptr()) };
            let ctx = format!("NormalizeColor #{i} in={v:?}{}", bits(v));
            check_f32(&format!("{ctx} -- returned max"), v, mc, mr);
            check_vec(&format!("{ctx} -- out"), v, &oc, &or_);
        }
        // SAL: out == in
        {
            let mut vc = guarded3(v);
            let mut vr = guarded3(v);
            let pc = vc.as_mut_ptr();
            let mc = unsafe { c(pc, pc) };
            let pr = vr.as_mut_ptr();
            let mr = unsafe { r(pr, pr) };
            let ctx = format!("NormalizeColor #{i} in={v:?}{} aliased", bits(v));
            check_f32(&format!("{ctx} -- returned max"), v, mc, mr);
            check_vec(&format!("{ctx} -- out"), v, &vc, &vr);
        }
    }
}

// ---------------------------------------------------------------------------
// row 27 -- RadiusFromBounds
// ---------------------------------------------------------------------------

#[test]
fn radius_from_bounds() {
    let (c, r): (FRadius, FRadius) = both("RadiusFromBounds");
    let mut rng = Rng::new(27);

    let pool = shape_pool(&mut rng);

    // every ordered pair: covers mins > maxs, mins < maxs, mixed signs,
    // S0 SH SI SNaN, and both the `a > b` and the `!(a > b)` arm of the
    // conditional (including the NaN case, where `a > b` is false).
    for mins in &pool {
        for maxs in &pool {
            let inputs = [
                mins[0], mins[1], mins[2], maxs[0], maxs[1], maxs[2],
            ];
            let ctx = format!(
                "RadiusFromBounds(mins={mins:?}{}, maxs={maxs:?}{})",
                bits(mins),
                bits(maxs)
            );
            check_f32(
                &ctx,
                &inputs,
                unsafe { c(mins.as_ptr(), maxs.as_ptr()) },
                unsafe { r(mins.as_ptr(), maxs.as_ptr()) },
            );
        }
    }

    // SAL: mins and maxs are the same pointer (mins == maxs)
    for v in &pool {
        let inputs = [v[0], v[1], v[2], v[0], v[1], v[2]];
        let ctx = format!("RadiusFromBounds(v,v) one pointer v={v:?}{}", bits(v));
        check_f32(&ctx, &inputs, unsafe { c(v.as_ptr(), v.as_ptr()) }, unsafe {
            r(v.as_ptr(), v.as_ptr())
        });
    }

    // explicit `mins == maxs` by value, and boxes built the way real callers do
    for (mins, maxs) in [
        ([0.0f32, 0.0, 0.0], [0.0f32, 0.0, 0.0]),
        ([-1.0, -1.0, -1.0], [1.0, 1.0, 1.0]),
        ([1.0, 1.0, 1.0], [-1.0, -1.0, -1.0]), // mins > maxs
        ([99999.0, 99999.0, 99999.0], [-99999.0, -99999.0, -99999.0]), // ClearBounds
        ([-15.0, -15.0, -24.0], [15.0, 15.0, 32.0]), // a player bbox
        ([-0.0, 0.0, -0.0], [0.0, -0.0, 0.0]),
        ([f32::MAX, 0.0, 0.0], [-f32::MAX, 0.0, 0.0]),
        (
            [f32::NAN, 1.0, 2.0],
            [1.0, f32::NAN, 2.0], // a > b with a NaN picks b
        ),
    ] {
        let inputs = [
            mins[0], mins[1], mins[2], maxs[0], maxs[1], maxs[2],
        ];
        let ctx = format!("RadiusFromBounds(boundary {mins:?},{maxs:?})");
        check_f32(
            &ctx,
            &inputs,
            unsafe { c(mins.as_ptr(), maxs.as_ptr()) },
            unsafe { r(mins.as_ptr(), maxs.as_ptr()) },
        );
    }

    for i in 0..20000 {
        let mins = rng.vec3_any();
        let maxs = rng.vec3_any();
        let inputs = [
            mins[0], mins[1], mins[2], maxs[0], maxs[1], maxs[2],
        ];
        let ctx = format!(
            "RadiusFromBounds #{i} mins={mins:?}{} maxs={maxs:?}{}",
            bits(&mins),
            bits(&maxs)
        );
        check_f32(
            &ctx,
            &inputs,
            unsafe { c(mins.as_ptr(), maxs.as_ptr()) },
            unsafe { r(mins.as_ptr(), maxs.as_ptr()) },
        );
    }
}

// ---------------------------------------------------------------------------
// row 28 -- ClearBounds
// ---------------------------------------------------------------------------

/// `ClearBounds` only writes constants, so the comparison is bit exact; the
/// chained assignment `mins[0] = mins[1] = mins[2] = 99999` reads back what it
/// wrote, which matters for the aliased and the overlapping cases.
#[test]
fn clear_bounds() {
    let (c, r): (FClear, FClear) = both("ClearBounds");
    let mut rng = Rng::new(28);

    // fresh, guarded buffers pre-filled with the same garbage
    for i in 0..2000 {
        let mut mc = OUT3;
        let mut xc = OUT3;
        let mut mr = OUT3;
        let mut xr = OUT3;
        if i > 0 {
            // ... and with random garbage, so a missing store is always visible
            for k in 0..4 {
                let g = f32::from_bits(rng.next_u32());
                mc[k] = g;
                mr[k] = g;
                let g = f32::from_bits(rng.next_u32());
                xc[k] = g;
                xr[k] = g;
            }
        }
        unsafe { c(mc.as_mut_ptr(), xc.as_mut_ptr()) };
        unsafe { r(mr.as_mut_ptr(), xr.as_mut_ptr()) };
        assert_vec(&format!("ClearBounds #{i} mins"), &mc, &mr);
        assert_vec(&format!("ClearBounds #{i} maxs"), &xc, &xr);
    }

    // SAL: mins and maxs are the SAME pointer -- the second chain overwrites
    // the first, so the whole buffer ends up at -99999.
    for i in 0..2000 {
        let mut bc = OUT3;
        let mut br = OUT3;
        for k in 0..4 {
            let g = f32::from_bits(rng.next_u32());
            bc[k] = g;
            br[k] = g;
        }
        let pc = bc.as_mut_ptr();
        unsafe { c(pc, pc) };
        let pr = br.as_mut_ptr();
        unsafe { r(pr, pr) };
        assert_vec(&format!("ClearBounds #{i} aliased mins==maxs"), &bc, &br);
    }

    // partially overlapping buffers: maxs = mins + 1 and mins = maxs + 2
    for i in 0..2000 {
        let mut bc = [0.0f32; 8];
        let mut br = [0.0f32; 8];
        for k in 0..8 {
            let g = f32::from_bits(rng.next_u32());
            bc[k] = g;
            br[k] = g;
        }
        let off = (i % 3) + 1;
        let (pc, qc) = unsafe { (bc.as_mut_ptr(), bc.as_mut_ptr().add(off)) };
        unsafe { c(pc, qc) };
        let (pr, qr) = unsafe { (br.as_mut_ptr(), br.as_mut_ptr().add(off)) };
        unsafe { r(pr, qr) };
        assert_vec(
            &format!("ClearBounds #{i} overlapping (maxs = mins+{off})"),
            &bc,
            &br,
        );
    }
}

// ---------------------------------------------------------------------------
// row 29 -- AddPointToBounds
// ---------------------------------------------------------------------------

/// `AddPointToBounds` only compares and copies, so NaN payloads survive
/// verbatim and the comparison is bit exact (`assert_vec`, not `check_vec`).
#[test]
fn add_point_to_bounds() {
    let (clear_c, clear_r): (FClear, FClear) = both("ClearBounds");
    let (c, r): (FAddPt, FAddPt) = both("AddPointToBounds");
    let mut rng = Rng::new(29);

    // A deterministic walk: start from ClearBounds, then add points that are
    // inside, and points that stick out on each of the 6 faces, in both orders.
    let inf = f32::INFINITY;
    let scripted: Vec<[f32; 3]> = vec![
        [0.0, 0.0, 0.0],   // sets mins == maxs == origin
        [1.0, 2.0, 3.0],   // grows all three maxs
        [-1.0, -2.0, -3.0], // grows all three mins
        [0.5, 0.5, 0.5],   // strictly inside: nothing changes
        [-0.5, -0.5, -0.5],
        [-2.0, 0.0, 0.0], // -x face
        [2.0, 0.0, 0.0],  // +x face
        [0.0, -3.0, 0.0], // -y face
        [0.0, 3.0, 0.0],  // +y face
        [0.0, 0.0, -4.0], // -z face
        [0.0, 0.0, 4.0],  // +z face
        [-2.0, 3.0, -4.0], // exactly on three faces: no `<`/`>` fires
        [2.0, -3.0, 4.0],
        [-0.0, 0.0, -0.0],           // signed zero: neither `<` nor `>`
        [f32::NAN, 0.0, 0.0],        // SNaN: both comparisons false
        [0.0, f32::NAN, 0.0],
        [0.0, 0.0, f32::NAN],
        [f32::NAN, f32::NAN, f32::NAN],
        [1.0, 2.0, 3.0], // still inside afterwards
        [-inf, 0.0, 0.0],
        [inf, 0.0, 0.0],
        [0.0, -inf, inf],
        [inf, inf, inf], // no change: already infinite
        [f32::NAN, inf, -inf],
        [1e30, -1e30, 1e-45],
    ];

    // fresh buffers
    {
        let mut mc = OUT3;
        let mut xc = OUT3;
        let mut mr = OUT3;
        let mut xr = OUT3;
        unsafe { clear_c(mc.as_mut_ptr(), xc.as_mut_ptr()) };
        unsafe { clear_r(mr.as_mut_ptr(), xr.as_mut_ptr()) };
        assert_vec("AddPointToBounds: ClearBounds mins", &mc, &mr);
        assert_vec("AddPointToBounds: ClearBounds maxs", &xc, &xr);
        for (i, p) in scripted.iter().enumerate() {
            unsafe { c(p.as_ptr(), mc.as_mut_ptr(), xc.as_mut_ptr()) };
            unsafe { r(p.as_ptr(), mr.as_mut_ptr(), xr.as_mut_ptr()) };
            assert_vec(
                &format!("AddPointToBounds scripted step {i} p={p:?}{} mins", bits(p)),
                &mc,
                &mr,
            );
            assert_vec(
                &format!("AddPointToBounds scripted step {i} p={p:?}{} maxs", bits(p)),
                &xc,
                &xr,
            );
        }
    }

    // the same walk with mins and maxs aliased to a single buffer
    {
        let mut bc = OUT3;
        let mut br = OUT3;
        let pc = bc.as_mut_ptr();
        unsafe { clear_c(pc, pc) };
        let pr = br.as_mut_ptr();
        unsafe { clear_r(pr, pr) };
        assert_vec("AddPointToBounds aliased: after ClearBounds", &bc, &br);
        for (i, p) in scripted.iter().enumerate() {
            let pc = bc.as_mut_ptr();
            unsafe { c(p.as_ptr(), pc, pc) };
            let pr = br.as_mut_ptr();
            unsafe { r(p.as_ptr(), pr, pr) };
            assert_vec(
                &format!("AddPointToBounds aliased step {i} p={p:?}{}", bits(p)),
                &bc,
                &br,
            );
        }
    }

    // 20000 random points, in fresh sequences of 8 so both branches of every
    // comparison keep firing
    for seq in 0..2500 {
        let mut mc = OUT3;
        let mut xc = OUT3;
        let mut mr = OUT3;
        let mut xr = OUT3;
        unsafe { clear_c(mc.as_mut_ptr(), xc.as_mut_ptr()) };
        unsafe { clear_r(mr.as_mut_ptr(), xr.as_mut_ptr()) };
        for step in 0..8 {
            let p = match seq % 4 {
                0 => rng.vec3_any(),
                1 => rng.vec3_mag(10.0),
                2 => rng.vec3_finite(),
                _ => rng.vec3_mag(1e6),
            };
            unsafe { c(p.as_ptr(), mc.as_mut_ptr(), xc.as_mut_ptr()) };
            unsafe { r(p.as_ptr(), mr.as_mut_ptr(), xr.as_mut_ptr()) };
            assert_vec(
                &format!(
                    "AddPointToBounds seq {seq} step {step} p={p:?}{} mins",
                    bits(&p)
                ),
                &mc,
                &mr,
            );
            assert_vec(
                &format!(
                    "AddPointToBounds seq {seq} step {step} p={p:?}{} maxs",
                    bits(&p)
                ),
                &xc,
                &xr,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// row 30 -- VectorRotate
// ---------------------------------------------------------------------------

#[test]
fn vector_rotate() {
    let (c, r): (FRotate, FRotate) = both("VectorRotate");
    let mut rng = Rng::new(30);

    let vectors = shape_pool(&mut rng);
    let matrices = matrix_pool(&mut rng);

    let check = |ctx: String, v: &[f32; 3], m: &[[f32; 3]; 3]| {
        let inputs = [
            v[0], v[1], v[2], m[0][0], m[0][1], m[0][2], m[1][0], m[1][1], m[1][2], m[2][0],
            m[2][1], m[2][2],
        ];
        // separate output buffer
        {
            let mut vc = guarded3(v);
            let mut vr = guarded3(v);
            let mut mc = guarded_mat(m);
            let mut mr = guarded_mat(m);
            let mut oc = OUT3;
            let mut or_ = OUT3;
            unsafe { c(vc.as_mut_ptr(), mc.as_mut_ptr(), oc.as_mut_ptr()) };
            unsafe { r(vr.as_mut_ptr(), mr.as_mut_ptr(), or_.as_mut_ptr()) };
            check_vec(&format!("{ctx} -- out"), &inputs, &oc, &or_);
            check_vec(&format!("{ctx} -- in untouched"), &inputs, &vc, &vr);
            check_vec(
                &format!("{ctx} -- matrix untouched"),
                &inputs,
                &flat12(&mc),
                &flat12(&mr),
            );
        }
        // SAL: out == in (the C code reads `in` after writing `out[0]`)
        {
            let mut vc = guarded3(v);
            let mut vr = guarded3(v);
            let mut mc = guarded_mat(m);
            let mut mr = guarded_mat(m);
            let pc = vc.as_mut_ptr();
            unsafe { c(pc, mc.as_mut_ptr(), pc) };
            let pr = vr.as_mut_ptr();
            unsafe { r(pr, mr.as_mut_ptr(), pr) };
            check_vec(&format!("{ctx} -- aliased out==in"), &inputs, &vc, &vr);
        }
    };

    for (vi, v) in vectors.iter().enumerate() {
        for (mi, m) in matrices.iter().enumerate() {
            check(
                format!("VectorRotate(v#{vi}={v:?}{}, m#{mi}={m:?})", bits(v)),
                v,
                m,
            );
        }
    }

    for i in 0..8000 {
        let v = rng.vec3_any();
        let m = [rng.vec3_any(), rng.vec3_any(), rng.vec3_any()];
        check(
            format!("VectorRotate #{i} v={v:?}{} m={m:?}", bits(&v)),
            &v,
            &m,
        );
    }
    for i in 0..8000 {
        let v = rng.vec3_mag(1.0);
        let m = [rng.vec3_mag(1.0), rng.vec3_mag(1.0), rng.vec3_mag(1.0)];
        check(
            format!("VectorRotate unit #{i} v={v:?}{} m={m:?}", bits(&v)),
            &v,
            &m,
        );
    }
}

// ---------------------------------------------------------------------------
// row 31 -- MatrixMultiply
// ---------------------------------------------------------------------------

#[test]
fn matrix_multiply() {
    let (c, r): (FMatMul, FMatMul) = both("MatrixMultiply");
    let mut rng = Rng::new(31);

    let pool = matrix_pool(&mut rng);

    let check = |ctx: String, m1: &[[f32; 3]; 3], m2: &[[f32; 3]; 3]| {
        let f1 = flat9(m1);
        let f2 = flat9(m2);
        let mut inputs = [0.0f32; 18];
        inputs[..9].copy_from_slice(&f1);
        inputs[9..].copy_from_slice(&f2);

        // separate output
        {
            let mut ac = guarded_mat(m1);
            let mut bc = guarded_mat(m2);
            let mut oc = out_mat();
            let mut ar = guarded_mat(m1);
            let mut br = guarded_mat(m2);
            let mut or_ = out_mat();
            unsafe { c(ac.as_mut_ptr(), bc.as_mut_ptr(), oc.as_mut_ptr()) };
            unsafe { r(ar.as_mut_ptr(), br.as_mut_ptr(), or_.as_mut_ptr()) };
            check_vec(
                &format!("{ctx} -- out"),
                &inputs,
                &flat12(&oc),
                &flat12(&or_),
            );
            check_vec(
                &format!("{ctx} -- in1 untouched"),
                &inputs,
                &flat12(&ac),
                &flat12(&ar),
            );
            check_vec(
                &format!("{ctx} -- in2 untouched"),
                &inputs,
                &flat12(&bc),
                &flat12(&br),
            );
        }
        // SAL: out == in1
        {
            let mut ac = guarded_mat(m1);
            let mut bc = guarded_mat(m2);
            let mut ar = guarded_mat(m1);
            let mut br = guarded_mat(m2);
            let pc = ac.as_mut_ptr();
            unsafe { c(pc, bc.as_mut_ptr(), pc) };
            let pr = ar.as_mut_ptr();
            unsafe { r(pr, br.as_mut_ptr(), pr) };
            check_vec(
                &format!("{ctx} -- aliased out==in1"),
                &inputs,
                &flat12(&ac),
                &flat12(&ar),
            );
            check_vec(
                &format!("{ctx} -- aliased out==in1, in2 untouched"),
                &inputs,
                &flat12(&bc),
                &flat12(&br),
            );
        }
        // SAL: out == in2
        {
            let mut ac = guarded_mat(m1);
            let mut bc = guarded_mat(m2);
            let mut ar = guarded_mat(m1);
            let mut br = guarded_mat(m2);
            let pc = bc.as_mut_ptr();
            unsafe { c(ac.as_mut_ptr(), pc, pc) };
            let pr = br.as_mut_ptr();
            unsafe { r(ar.as_mut_ptr(), pr, pr) };
            check_vec(
                &format!("{ctx} -- aliased out==in2"),
                &inputs,
                &flat12(&bc),
                &flat12(&br),
            );
            check_vec(
                &format!("{ctx} -- aliased out==in2, in1 untouched"),
                &inputs,
                &flat12(&ac),
                &flat12(&ar),
            );
        }
        // SAL: in1 == in2, separate output
        {
            let mut ac = guarded_mat(m1);
            let mut oc = out_mat();
            let mut ar = guarded_mat(m1);
            let mut or_ = out_mat();
            let pc = ac.as_mut_ptr();
            unsafe { c(pc, pc, oc.as_mut_ptr()) };
            let pr = ar.as_mut_ptr();
            unsafe { r(pr, pr, or_.as_mut_ptr()) };
            let mut sq = [0.0f32; 18];
            sq[..9].copy_from_slice(&f1);
            sq[9..].copy_from_slice(&f1);
            check_vec(
                &format!("{ctx} -- aliased in1==in2"),
                &sq,
                &flat12(&oc),
                &flat12(&or_),
            );
        }
        // SAL: all three the same pointer
        {
            let mut ac = guarded_mat(m1);
            let mut ar = guarded_mat(m1);
            let pc = ac.as_mut_ptr();
            unsafe { c(pc, pc, pc) };
            let pr = ar.as_mut_ptr();
            unsafe { r(pr, pr, pr) };
            let mut sq = [0.0f32; 18];
            sq[..9].copy_from_slice(&f1);
            sq[9..].copy_from_slice(&f1);
            check_vec(
                &format!("{ctx} -- aliased in1==in2==out"),
                &sq,
                &flat12(&ac),
                &flat12(&ar),
            );
        }
    };

    for (i, m1) in pool.iter().enumerate() {
        for (j, m2) in pool.iter().enumerate() {
            check(
                format!("MatrixMultiply(pool #{i} {m1:?}, pool #{j} {m2:?})"),
                m1,
                m2,
            );
        }
    }

    for i in 0..5000 {
        let m1 = [rng.vec3_any(), rng.vec3_any(), rng.vec3_any()];
        let m2 = [rng.vec3_any(), rng.vec3_any(), rng.vec3_any()];
        check(format!("MatrixMultiply #{i} {m1:?} x {m2:?}"), &m1, &m2);
    }
    for i in 0..5000 {
        let m1 = [rng.vec3_mag(1.0), rng.vec3_mag(1.0), rng.vec3_mag(1.0)];
        let m2 = [rng.vec3_finite(), rng.vec3_finite(), rng.vec3_finite()];
        check(
            format!("MatrixMultiply finite #{i} {m1:?} x {m2:?}"),
            &m1,
            &m2,
        );
    }
}
