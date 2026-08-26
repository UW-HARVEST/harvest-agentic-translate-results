//! Differential tests for the plane / box / direction-table entry points of
//! `q_math.c`: CONFIGS.md rows 39-44 and 48-49, plus ERRORS.md rows 21-23.
//!
//! Covered entry points
//!
//! * `CrossProduct` (through the `w_CrossProduct` hook of
//!   `tests/csupport/wrappers.c`, because it is a `static ID_INLINE` function of
//!   `q_shared.h` and therefore has no exported symbol of its own),
//! * `PlaneFromPoints`, `SetPlaneSignbits`, `BoxOnPlaneSide`,
//! * `DirToByte` / `ByteToDir` and the `bytedirs` table they index.
//!
//! Every call goes through `dlsym` on the two shared objects; nothing in
//! `src/` is called directly.

mod harness;

use core::ffi::c_int;
use harness::*;

// ---------------------------------------------------------------------------
// types
// ---------------------------------------------------------------------------

/// ```c
/// typedef struct cplane_s {
///     vec3_t  normal;
///     float   dist;
///     byte    type;      // for fast side tests: 0,1,2 = axial, 3 = nonaxial
///     byte    signbits;  // signx + (signy<<1) + (signz<<2)
///     byte    pad[2];
/// } cplane_t;
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct CPlane {
    normal: [f32; 3],
    dist: f32,
    type_: u8,
    signbits: u8,
    pad: [u8; 2],
}

/// `sizeof(cplane_t)` -- the whole struct is compared after every call, so the
/// layout must match the C one exactly (row 60 checks this against the C
/// `offsetof`s; here it is only asserted locally).
const PLANE_SIZE: usize = 20;
const _: () = assert!(core::mem::size_of::<CPlane>() == PLANE_SIZE);
const _: () = assert!(core::mem::align_of::<CPlane>() == 4);

type CrossFn = unsafe extern "C" fn(*const f32, *const f32, *mut f32);
type PlaneFromPointsFn =
    unsafe extern "C" fn(*mut f32, *const f32, *const f32, *const f32) -> c_int;
type SignbitsFn = unsafe extern "C" fn(*mut CPlane);
type BoxFn = unsafe extern "C" fn(*mut f32, *mut f32, *mut CPlane) -> c_int;
type DirToByteFn = unsafe extern "C" fn(*mut f32) -> c_int;
type ByteToDirFn = unsafe extern "C" fn(c_int, *mut f32);

/// Distinct, finite, easily recognisable bit patterns (exponent 0xFB, so no
/// NaN/inf semantics sneak in) used to pre-fill every output buffer.  A slot
/// that still holds its sentinel after the call was *not* written.
const SENTINEL: [f32; 5] = [
    f32::from_bits(0x7DEA_DBE0),
    f32::from_bits(0x7DEA_DBE1),
    f32::from_bits(0x7DEA_DBE2),
    f32::from_bits(0x7DEA_DBE3),
    f32::from_bits(0x7DEA_DBE4),
];

fn plane_bytes(p: &CPlane) -> [u8; PLANE_SIZE] {
    let mut out = [0u8; PLANE_SIZE];
    unsafe {
        core::ptr::copy_nonoverlapping(p as *const CPlane as *const u8, out.as_mut_ptr(), PLANE_SIZE)
    };
    out
}

// ---------------------------------------------------------------------------
// row 39 -- CrossProduct (q_shared.h `static ID_INLINE`, via w_CrossProduct)
// ---------------------------------------------------------------------------

#[test]
fn cross_product() {
    let (c, r): (CrossFn, CrossFn) = both_w("w_CrossProduct");
    let mut rng = Rng::new(39_0001);

    // out == a fresh buffer
    let plain = |v1: [f32; 3], v2: [f32; 3], tag: &str| {
        let mut oc = SENTINEL;
        let mut or_ = SENTINEL;
        unsafe { c(v1.as_ptr(), v2.as_ptr(), oc.as_mut_ptr()) };
        unsafe { r(v1.as_ptr(), v2.as_ptr(), or_.as_mut_ptr()) };
        let inputs = [v1[0], v1[1], v1[2], v2[0], v2[1], v2[2]];
        check_vec(
            &format!("CrossProduct({tag} v1={v1:?}, v2={v2:?})"),
            &inputs,
            &oc,
            &or_,
        );
        // the 5th slot is a guard: neither side may write past cross[2]
        assert_vec(
            &format!("CrossProduct({tag} v1={v1:?}, v2={v2:?}) guard slots"),
            &oc[3..],
            &SENTINEL[3..],
        );
    };

    // all 9 ordered pairs of the axis unit vectors (incl. the 3 SEQ diagonals)
    let axes = [[1.0f32, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    for (i, a) in axes.iter().enumerate() {
        for (j, b) in axes.iter().enumerate() {
            plain(*a, *b, &format!("axes {i}x{j}"));
        }
    }
    // negated axes too: 6x6 signed axial pairs
    for si in 0..6 {
        for sj in 0..6 {
            let mut a = axes[si % 3];
            let mut b = axes[sj % 3];
            if si >= 3 {
                a[si % 3] = -1.0;
            }
            if sj >= 3 {
                b[sj % 3] = -1.0;
            }
            plain(a, b, &format!("signed axes {si}x{sj}"));
        }
    }

    // zero, negative zero, parallel and anti-parallel vectors (result 0 / -0)
    let bases = [
        [0.0f32, 0.0, 0.0],
        [-0.0f32, -0.0, -0.0],
        [1.0, 2.0, 3.0],
        [-1.0, 2.0, -3.0],
        [1e30, 1e30, 1e30],
        [1e-30, 1e-30, 1e-30],
        [f32::MAX, f32::MAX, f32::MAX],
        [f32::MIN_POSITIVE, 1e-45, 0.0],
        [f32::INFINITY, 0.0, 0.0],
        [f32::NEG_INFINITY, f32::INFINITY, 0.0],
        [f32::NAN, 1.0, 2.0],
        [1.0, f32::NAN, 2.0],
        [1.0, 2.0, f32::NAN],
    ];
    for v in bases {
        plain(v, v, "SEQ by value");
        for k in [0.0f32, -0.0, 1.0, -1.0, 2.0, -2.0, 0.5, 1e30, f32::INFINITY, f32::NAN] {
            let w = [v[0] * k, v[1] * k, v[2] * k];
            plain(v, w, &format!("parallel k={k:?}"));
            plain(w, v, &format!("parallel k={k:?} swapped"));
        }
        for w in bases {
            plain(v, w, "pairs of interesting vectors");
        }
    }

    // literally the same pointer for v1 and v2 (SEQ)
    for v in bases {
        let a = v;
        let mut oc = SENTINEL;
        let mut or_ = SENTINEL;
        let pa = a.as_ptr();
        unsafe { c(pa, pa, oc.as_mut_ptr()) };
        unsafe { r(pa, pa, or_.as_mut_ptr()) };
        let inputs = [a[0], a[1], a[2]];
        check_vec(
            &format!("CrossProduct(same pointer twice, v={a:?})"),
            &inputs,
            &oc,
            &or_,
        );
    }

    // random: finite, then the full weird pool
    for _ in 0..8000 {
        let (v1, v2) = (rng.vec3_mag(1e3), rng.vec3_mag(1e3));
        plain(v1, v2, "random finite");
    }
    for _ in 0..8000 {
        let (v1, v2) = (rng.vec3_any(), rng.vec3_any());
        plain(v1, v2, "random any");
    }
    for _ in 0..4000 {
        let (v1, v2) = (rng.dir(), rng.dir());
        plain(v1, v2, "random dir");
    }

    // SAL: `cross` aliases v1, v2, or both (the C code reads v1/v2 *after*
    // writing cross[0], so aliasing is observable)
    let mut alias_cases: Vec<([f32; 3], [f32; 3])> = Vec::new();
    for a in bases {
        for b in bases {
            alias_cases.push((a, b));
        }
    }
    for _ in 0..3000 {
        alias_cases.push((rng.vec3_mag(1e3), rng.vec3_mag(1e3)));
    }
    for _ in 0..1000 {
        alias_cases.push((rng.vec3_any(), rng.vec3_any()));
    }
    for (v1, v2) in alias_cases {
        let inputs = [v1[0], v1[1], v1[2], v2[0], v2[1], v2[2]];

        // cross == v1
        let (mut bc, mut br) = (v1, v1);
        let (pc, pr) = (bc.as_mut_ptr(), br.as_mut_ptr());
        unsafe { c(pc, v2.as_ptr(), pc) };
        unsafe { r(pr, v2.as_ptr(), pr) };
        check_vec(
            &format!("CrossProduct(cross==v1, v1={v1:?}, v2={v2:?})"),
            &inputs,
            &bc,
            &br,
        );

        // cross == v2
        let (mut bc, mut br) = (v2, v2);
        let (pc, pr) = (bc.as_mut_ptr(), br.as_mut_ptr());
        unsafe { c(v1.as_ptr(), pc, pc) };
        unsafe { r(v1.as_ptr(), pr, pr) };
        check_vec(
            &format!("CrossProduct(cross==v2, v1={v1:?}, v2={v2:?})"),
            &inputs,
            &bc,
            &br,
        );

        // cross == v1 == v2 (all three the same buffer)
        let (mut bc, mut br) = (v1, v1);
        let (pc, pr) = (bc.as_mut_ptr(), br.as_mut_ptr());
        unsafe { c(pc, pc, pc) };
        unsafe { r(pr, pr, pr) };
        check_vec(
            &format!("CrossProduct(all aliased, v={v1:?})"),
            &[v1[0], v1[1], v1[2]],
            &bc,
            &br,
        );
    }
}

// ---------------------------------------------------------------------------
// row 40 -- PlaneFromPoints
// ---------------------------------------------------------------------------

/// `plane[3]` is written only when `VectorNormalize` returned non-zero, so the
/// buffer is pre-filled with [`SENTINEL`]: a surviving sentinel in slot 3 is
/// exactly the "not written" case, and it must survive in *both* libraries.
#[test]
fn plane_from_points() {
    let (c, r): (PlaneFromPointsFn, PlaneFromPointsFn) = both("PlaneFromPoints");
    let mut rng = Rng::new(40_0002);

    let mut non_degenerate_seen = 0usize;
    let mut degenerate_seen = 0usize;

    let run = |a: [f32; 3], b: [f32; 3], cc: [f32; 3], tag: &str| -> c_int {
        let mut pc = SENTINEL;
        let mut pr = SENTINEL;
        let rc = unsafe { c(pc.as_mut_ptr(), a.as_ptr(), b.as_ptr(), cc.as_ptr()) };
        let rr = unsafe { r(pr.as_mut_ptr(), a.as_ptr(), b.as_ptr(), cc.as_ptr()) };
        let ctx = format!("PlaneFromPoints({tag}: a={a:?}, b={b:?}, c={cc:?})");
        assert_int(&format!("{ctx} return"), rc, rr);
        let inputs = [a[0], a[1], a[2], b[0], b[1], b[2], cc[0], cc[1], cc[2]];
        check_vec(&format!("{ctx} plane"), &inputs, &pc, &pr);
        // vec4_t: nothing may be written past plane[3]
        assert_vec(&format!("{ctx} guard slot"), &pc[4..], &SENTINEL[4..]);
        // "plane[3] written only on success"
        let untouched_c = pc[3].to_bits() == SENTINEL[3].to_bits();
        assert_eq!(
            untouched_c,
            rc == 0,
            "{ctx}: C returned {rc} but plane[3] untouched = {untouched_c} \
             (0x{:08x}) -- plane[3] must be written exactly when the result is qtrue",
            pc[3].to_bits()
        );
        rc
    };

    // --- guaranteed non-degenerate triangles: must return qtrue -------------
    for _ in 0..3000 {
        let a = rng.vec3_mag(1e3);
        // two edges of length >= 1 along two different axes -> the cross
        // product has exactly one non-zero component, so it can never
        // normalize to zero
        let i = rng.below(3) as usize;
        let mut j = rng.below(3) as usize;
        if j == i {
            j = (i + 1) % 3;
        }
        let (mut b, mut cv) = (a, a);
        let s = 1.0 + (rng.below(1000) as f32);
        let t = 1.0 + (rng.below(1000) as f32);
        b[i] += if rng.bool() { s } else { -s };
        cv[j] += if rng.bool() { t } else { -t };
        let got = run(a, b, cv, "non-degenerate axis-aligned triangle");
        assert_int(
            &format!("PlaneFromPoints(non-degenerate a={a:?}, b={b:?}, c={cv:?}) must be qtrue"),
            got,
            1,
        );
        non_degenerate_seen += 1;
    }
    // a couple of hand-written non-degenerate ones
    for (a, b, cv) in [
        ([0.0f32, 0.0, 0.0], [1.0f32, 0.0, 0.0], [0.0f32, 1.0, 0.0]),
        ([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),
        ([-1.0, -1.0, -1.0], [1.0, -1.0, -1.0], [-1.0, 1.0, -1.0]),
        ([3.0, 4.0, 5.0], [3.0, 4.0, 6.0], [4.0, 4.0, 5.0]),
        ([1e-3, 0.0, 0.0], [0.0, 1e-3, 0.0], [0.0, 0.0, 1e-3]),
    ] {
        let got = run(a, b, cv, "hand-written non-degenerate");
        assert_int(
            &format!("PlaneFromPoints(hand-written a={a:?}) must be qtrue"),
            got,
            1,
        );
    }

    // --- degenerate: must return qfalse, plane[3] untouched -----------------
    //
    // For two equal points the cross product computes `x*y - y*x`, which is
    // exactly 0 in IEEE arithmetic *unless* the product overflows to infinity
    // (inf - inf = NaN, and `if (length)` is true for NaN, so the function then
    // reports success).  These points are all small enough that no product can
    // overflow, so every degenerate shape must give qfalse.
    let pts = [
        [0.0f32, 0.0, 0.0],
        [1.0, 2.0, 3.0],
        [-4.0, 5.5, 1e-8],
        [1e10, -1e9, 7.0],
        [f32::MIN_POSITIVE, 1e-45, -1e-45],
    ];
    for p in pts {
        for q in pts {
            // a == b
            let got = run(p, p, q, "degenerate a==b");
            assert_int(&format!("PlaneFromPoints(a==b={p:?},c={q:?}) qfalse"), got, 0);
            // b == c
            let got = run(p, q, q, "degenerate b==c");
            assert_int(&format!("PlaneFromPoints(a={p:?},b==c={q:?}) qfalse"), got, 0);
            // a == c
            let got = run(p, q, p, "degenerate a==c");
            assert_int(&format!("PlaneFromPoints(a==c={p:?},b={q:?}) qfalse"), got, 0);
            // all three equal
            let got = run(p, p, p, "degenerate all equal");
            assert_int(&format!("PlaneFromPoints(all=={p:?}) qfalse"), got, 0);
            degenerate_seen += 4;
        }
    }
    // The same degenerate shapes with coordinates whose products *do* overflow:
    // `b == c` then makes CrossProduct compute inf - inf = NaN, VectorNormalize
    // returns NaN, `if (length)` is true and the "degenerate" triangle is
    // reported as valid.  Only C-vs-Rust agreement is asserted here.
    let wild = [
        [1e20f32, -1e20, 7.0],
        [1e30, 1e30, 1e30],
        [f32::MAX, f32::MAX, 0.0],
        [f32::MAX, -f32::MAX, 1.0],
        [1.9e19, 1.9e19, 1.9e19],
    ];
    for p in wild {
        for q in wild {
            run(p, p, q, "overflowing degenerate a==b");
            run(p, q, q, "overflowing degenerate b==c");
            run(p, q, p, "overflowing degenerate a==c");
            run(p, p, p, "overflowing degenerate all equal");
        }
        for q in pts {
            run(p, p, q, "mixed-scale degenerate a==b");
            run(p, q, q, "mixed-scale degenerate b==c");
            run(q, p, p, "mixed-scale degenerate b==c (huge pair)");
            run(p, q, p, "mixed-scale degenerate a==c");
        }
    }
    // collinear: c = a + t*(b - a)
    for _ in 0..2000 {
        let a = rng.vec3_mag(100.0);
        let d = rng.vec3_mag(100.0);
        let b = [a[0] + d[0], a[1] + d[1], a[2] + d[2]];
        let t = match rng.below(6) {
            0 => 2.0f32,
            1 => -1.0,
            2 => 0.5,
            3 => 3.0,
            4 => -0.25,
            _ => rng.f32_mag(4.0),
        };
        let cv = [a[0] + d[0] * t, a[1] + d[1] * t, a[2] + d[2] * t];
        // exactly collinear only up to rounding, so the return value is
        // whatever the C code says -- but it must agree bit-for-bit.
        let got = run(a, b, cv, &format!("collinear t={t:?}"));
        if got == 0 {
            degenerate_seen += 1;
        }
    }
    // exactly collinear along one axis (no rounding at all)
    for k in [1.0f32, 2.0, 3.0, -1.0, 0.5, 1e10] {
        let a = [0.0f32, 0.0, 0.0];
        let b = [1.0f32, 0.0, 0.0];
        let cv = [k, 0.0, 0.0];
        let got = run(a, b, cv, &format!("exactly collinear on x, k={k:?}"));
        assert_int(
            &format!("PlaneFromPoints(collinear on x, k={k:?}) qfalse"),
            got,
            0,
        );
        degenerate_seen += 1;
    }

    // --- overflow / inf / NaN ----------------------------------------------
    let extreme = [
        [1e30f32, 1e30, 1e30],
        [-1e30f32, 1e30, -1e30],
        [f32::MAX, f32::MAX, f32::MAX],
        [f32::MAX, -f32::MAX, 0.0],
        [1e38, 0.0, 0.0],
        [f32::INFINITY, 0.0, 0.0],
        [f32::NEG_INFINITY, f32::INFINITY, 0.0],
        [f32::NAN, 0.0, 0.0],
        [0.0, f32::NAN, 0.0],
        [0.0, 0.0, f32::NAN],
        [1e-30, 1e-30, 1e-30],
        [1e-45, 0.0, -1e-45],
        [0.0, 0.0, 0.0],
        [1.0, 2.0, 3.0],
    ];
    for a in extreme {
        for b in extreme {
            for cv in extreme {
                run(a, b, cv, "extreme");
            }
        }
    }

    // --- random ------------------------------------------------------------
    for _ in 0..8000 {
        let (a, b, cv) = (rng.vec3_mag(1e3), rng.vec3_mag(1e3), rng.vec3_mag(1e3));
        if run(a, b, cv, "random finite triangle") == 1 {
            non_degenerate_seen += 1;
        }
    }
    for _ in 0..6000 {
        let (a, b, cv) = (rng.vec3_any(), rng.vec3_any(), rng.vec3_any());
        run(a, b, cv, "random any");
    }
    for _ in 0..3000 {
        // tiny triangles whose cross product underflows to zero
        let scale = match rng.below(3) {
            0 => 1e-20f32,
            1 => 1e-30,
            _ => 1e-40,
        };
        let a = rng.vec3_mag(scale);
        let b = rng.vec3_mag(scale);
        let cv = rng.vec3_mag(scale);
        if run(a, b, cv, &format!("underflowing triangle scale={scale:?}")) == 0 {
            degenerate_seen += 1;
        }
    }

    assert!(
        non_degenerate_seen > 1000,
        "expected plenty of qtrue results, saw {non_degenerate_seen}"
    );
    assert!(
        degenerate_seen > 100,
        "expected plenty of qfalse results, saw {degenerate_seen}"
    );
}

// ---------------------------------------------------------------------------
// row 41 / ERRORS.md row 23 -- SetPlaneSignbits
// ---------------------------------------------------------------------------

/// The whole 20-byte struct is compared afterwards, so `normal`, `dist`,
/// `type` and `pad` are also checked for being left alone, and `signbits` is
/// pre-set to a value the function must overwrite.
#[test]
fn set_plane_signbits() {
    let (c, r): (SignbitsFn, SignbitsFn) = both("SetPlaneSignbits");
    let mut rng = Rng::new(41_0003);

    let run = |p: CPlane, tag: &str| {
        let mut pc = p;
        let mut pr = p;
        unsafe { c(&mut pc as *mut CPlane) };
        unsafe { r(&mut pr as *mut CPlane) };
        let ctx = format!("SetPlaneSignbits({tag}, in={p:?})");
        // whole-struct byte compare (catches signbits, and any stray write)
        assert_int(
            &format!("{ctx} full struct bytes"),
            plane_bytes(&pc),
            plane_bytes(&pr),
        );
        // separate, friendlier assertions on the individual fields
        assert_vec(&format!("{ctx} normal"), &pc.normal, &pr.normal);
        assert_f32(&format!("{ctx} dist"), pc.dist, pr.dist);
        assert_int(&format!("{ctx} type"), pc.type_, pr.type_);
        assert_int(&format!("{ctx} signbits"), pc.signbits, pr.signbits);
        assert_int(&format!("{ctx} pad"), pc.pad, pr.pad);
        // the input's non-signbits fields must be untouched by C as well
        assert_vec(&format!("{ctx} normal preserved"), &p.normal, &pc.normal);
        assert_f32(&format!("{ctx} dist preserved"), p.dist, pc.dist);
        assert_int(&format!("{ctx} type preserved"), p.type_, pc.type_);
        assert_int(&format!("{ctx} pad preserved"), p.pad, pc.pad);
        pc.signbits
    };

    // pre-set signbits values that must all be overwritten, plus other junk
    // in the fields the function does not touch
    let presets: [(u8, u8, [u8; 2], f32); 6] = [
        (0x00, 0x00, [0x00, 0x00], 0.0),
        (0xFF, 0xFF, [0xFF, 0xFF], -1.0),
        (0x07, 0x05, [0xAA, 0xBB], 12.5),
        (0x03, 0xF0, [0x12, 0x34], f32::NAN),
        (0x02, 0x08, [0x7F, 0x80], f32::INFINITY),
        (0x2A, 0x03, [0xDE, 0xAD], -1e30),
    ];

    // all 8 sign combinations of a unit normal: signbits must come out as the
    // combination itself
    for bits in 0u8..8 {
        for &(sb, ty, pad, dist) in &presets {
            let normal = [
                if bits & 1 != 0 { -1.0f32 } else { 1.0 },
                if bits & 2 != 0 { -1.0f32 } else { 1.0 },
                if bits & 4 != 0 { -1.0f32 } else { 1.0 },
            ];
            let p = CPlane { normal, dist, type_: ty, signbits: sb, pad };
            let got = run(p, &format!("sign combination {bits}"));
            assert_int(
                &format!("SetPlaneSignbits(normal={normal:?}, preset signbits=0x{sb:02x})"),
                got,
                bits,
            );
        }
    }

    // -0.0 is NOT negative for `< 0` (ERRORS.md row 23), and neither is NaN
    let comps = [0.0f32, -0.0, 1.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 1e-45, -1e-45];
    for &x in &comps {
        for &y in &comps {
            for &z in &comps {
                for &(sb, ty, pad, dist) in &presets {
                    let p = CPlane {
                        normal: [x, y, z],
                        dist,
                        type_: ty,
                        signbits: sb,
                        pad,
                    };
                    let got = run(p, "zero/NaN/inf components");
                    // -0.0 and NaN must leave their bit clear
                    let expect = (if x < 0.0 { 1 } else { 0 })
                        | (if y < 0.0 { 2 } else { 0 })
                        | (if z < 0.0 { 4 } else { 0 });
                    assert_int(
                        &format!("SetPlaneSignbits(normal=[{x:?},{y:?},{z:?}]) signbits"),
                        got,
                        expect as u8,
                    );
                }
            }
        }
    }

    // random normals with random junk in every other field
    for _ in 0..20000 {
        let p = CPlane {
            normal: match rng.below(3) {
                0 => rng.vec3_any(),
                1 => rng.vec3_finite(),
                _ => rng.dir(),
            },
            dist: rng.f32_any(),
            type_: (rng.next_u32() & 0xff) as u8,
            signbits: (rng.next_u32() & 0xff) as u8,
            pad: [(rng.next_u32() & 0xff) as u8, (rng.next_u32() & 0xff) as u8],
        };
        run(p, "random");
    }
}

// ---------------------------------------------------------------------------
// BoxOnPlaneSide -- shared driver
// ---------------------------------------------------------------------------

/// Calls both implementations with identical, freshly copied arguments and
/// compares the return value *and* all three argument buffers (nothing may be
/// written through them).
#[track_caller]
fn box_both(
    c: BoxFn,
    r: BoxFn,
    emins: [f32; 3],
    emaxs: [f32; 3],
    p: CPlane,
    ctx: &str,
) -> c_int {
    let (mut mnc, mut mxc, mut pc) = (emins, emaxs, p);
    let (mut mnr, mut mxr, mut pr) = (emins, emaxs, p);
    let rc = unsafe { c(mnc.as_mut_ptr(), mxc.as_mut_ptr(), &mut pc as *mut CPlane) };
    let rr = unsafe { r(mnr.as_mut_ptr(), mxr.as_mut_ptr(), &mut pr as *mut CPlane) };
    let ctx = format!("BoxOnPlaneSide({ctx}: emins={emins:?}, emaxs={emaxs:?}, p={p:?})");
    assert_int(&format!("{ctx} return"), rc, rr);
    assert_vec(&format!("{ctx} emins after"), &mnc, &mnr);
    assert_vec(&format!("{ctx} emaxs after"), &mxc, &mxr);
    assert_int(
        &format!("{ctx} plane after"),
        plane_bytes(&pc),
        plane_bytes(&pr),
    );
    // the C function is read-only in all three arguments
    assert_vec(&format!("{ctx} emins unmodified"), &emins, &mnc);
    assert_vec(&format!("{ctx} emaxs unmodified"), &emaxs, &mxc);
    assert_int(
        &format!("{ctx} plane unmodified"),
        plane_bytes(&p),
        plane_bytes(&pc),
    );
    assert!(
        (0..=3).contains(&rc),
        "{ctx}: C returned {rc}, outside 0..=3"
    );
    rc
}

// ---------------------------------------------------------------------------
// row 42 -- BoxOnPlaneSide, fast axial cases (type 0, 1, 2)
// ---------------------------------------------------------------------------

#[test]
fn box_on_plane_side_axial() {
    let (c, r): (BoxFn, BoxFn) = both("BoxOnPlaneSide");
    let mut rng = Rng::new(42_0004);

    let mut seen = [0usize; 4];

    // For type t: `dist <= emins[t]` -> 1, else `dist >= emaxs[t]` -> 2, else 3.
    for t in 0u8..3 {
        // the three returns, hand-arranged, incl. the exact boundaries
        let cases: [(f32, f32, f32, c_int, &str); 9] = [
            (0.0, 10.0, -1.0, 1, "dist below emins"),
            (0.0, 10.0, 0.0, 1, "dist exactly emins (<= is inclusive)"),
            (0.0, 10.0, 20.0, 2, "dist above emaxs"),
            (0.0, 10.0, 10.0, 2, "dist exactly emaxs (>= is inclusive)"),
            (0.0, 10.0, 5.0, 3, "dist strictly inside"),
            (0.0, 0.0, 0.0, 1, "empty box, dist == both"),
            (5.0, 5.0, 5.0, 1, "degenerate box, dist == both"),
            // an inverted box can never reach `return 3`: `dist <= emins` is
            // tested first, so anything below the (larger) emins gives 1.
            (10.0, 0.0, 5.0, 1, "inverted box, dist between (emins wins)"),
            (10.0, 0.0, 20.0, 2, "inverted box, dist above emins and emaxs"),
        ];
        for (mn, mx, dist, expect, tag) in cases {
            // fill the other two components with values that would give a
            // *different* answer, to prove only index `type` is read
            let mut emins = [1e6f32; 3];
            let mut emaxs = [-1e6f32; 3];
            emins[t as usize] = mn;
            emaxs[t as usize] = mx;
            let p = CPlane {
                normal: [f32::NAN, f32::NAN, f32::NAN], // never read on this path
                dist,
                type_: t,
                signbits: 0xFF, // never read on this path
                pad: [0xAA, 0xBB],
            };
            let got = box_both(c, r, emins, emaxs, p, &format!("axial type={t} {tag}"));
            assert_int(
                &format!("BoxOnPlaneSide(axial type={t} {tag}) expected {expect}"),
                got,
                expect,
            );
            seen[got as usize] += 1;
        }

        // random axial boxes/dists
        for _ in 0..4000 {
            let emins = rng.vec3_mag(100.0);
            let emaxs = rng.vec3_mag(100.0);
            let dist = match rng.below(4) {
                0 => emins[t as usize],
                1 => emaxs[t as usize],
                2 => rng.f32_mag(100.0),
                _ => rng.f32_any(),
            };
            let p = CPlane {
                normal: rng.vec3_any(),
                dist,
                type_: t,
                signbits: (rng.next_u32() & 0xff) as u8,
                pad: [0, 0],
            };
            let got = box_both(c, r, emins, emaxs, p, &format!("axial random type={t}"));
            seen[got as usize] += 1;
        }

        // NaN / inf in the interesting slot
        for &mn in INTERESTING {
            for &mx in INTERESTING {
                for &dist in &[0.0f32, -0.0, 1.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY]
                {
                    let mut emins = [0.0f32; 3];
                    let mut emaxs = [0.0f32; 3];
                    emins[t as usize] = mn;
                    emaxs[t as usize] = mx;
                    let p = CPlane {
                        normal: [0.0; 3],
                        dist,
                        type_: t,
                        signbits: 0,
                        pad: [0, 0],
                    };
                    let got = box_both(c, r, emins, emaxs, p, &format!("axial extremes type={t}"));
                    seen[got as usize] += 1;
                }
            }
        }
    }

    for want in [1usize, 2, 3] {
        assert!(
            seen[want] > 0,
            "axial return value {want} never occurred: counts {seen:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// row 43 -- BoxOnPlaneSide, general case (type 3 x signbits 0..7)
// ---------------------------------------------------------------------------

#[test]
fn box_on_plane_side_general() {
    let (c, r): (BoxFn, BoxFn) = both("BoxOnPlaneSide");
    let mut rng = Rng::new(43_0005);

    let mut seen = [0usize; 4];
    let mut seen_per_signbits = [[0usize; 4]; 8];

    // Hand-arranged: with normal = {1,0,0} the general expression collapses to
    // dist1 = A, dist2 = B where {A,B} = {emins[0], emaxs[0]} selected by bit 0
    // of signbits.  Driving (A, B) directly reaches all four `sides` values for
    // every one of the 8 switch arms.
    for signbits in 0u8..8 {
        for (a, b, expect, tag) in [
            (10.0f32, 10.0f32, 1 as c_int, "dist1>=dist, dist2>=dist"),
            (-10.0, -10.0, 2, "dist1<dist, dist2<dist"),
            (10.0, -10.0, 3, "dist1>=dist, dist2<dist"),
            (-10.0, 10.0, 0, "dist1<dist, dist2>=dist (impossible for a sane box)"),
        ] {
            let (mn0, mx0) = if signbits & 1 != 0 { (a, b) } else { (b, a) };
            let emins = [mn0, 0.0, 0.0];
            let emaxs = [mx0, 0.0, 0.0];
            let p = CPlane {
                normal: [1.0, 0.0, 0.0],
                dist: 0.0,
                type_: 3,
                signbits,
                pad: [0, 0],
            };
            let got = box_both(
                c,
                r,
                emins,
                emaxs,
                p,
                &format!("general signbits={signbits} {tag}"),
            );
            assert_int(
                &format!("BoxOnPlaneSide(general signbits={signbits} {tag}) expected {expect}"),
                got,
                expect,
            );
            seen[got as usize] += 1;
            seen_per_signbits[signbits as usize][got as usize] += 1;
        }

        // the same four shapes, but driving every component (so bits 1 and 2
        // of signbits are exercised too)
        for &(lo, hi) in &[(-10.0f32, 10.0f32), (10.0, -10.0), (0.0, 0.0)] {
            for _ in 0..200 {
                let normal = rng.dir();
                let mut emins = [0.0f32; 3];
                let mut emaxs = [0.0f32; 3];
                for i in 0..3 {
                    if signbits & (1 << i) != 0 {
                        emins[i] = hi;
                        emaxs[i] = lo;
                    } else {
                        emins[i] = lo;
                        emaxs[i] = hi;
                    }
                }
                let p = CPlane {
                    normal,
                    dist: rng.f32_mag(30.0),
                    type_: 3,
                    signbits,
                    pad: [0, 0],
                };
                let got = box_both(
                    c,
                    r,
                    emins,
                    emaxs,
                    p,
                    &format!("general signbits={signbits} lo/hi swap"),
                );
                seen[got as usize] += 1;
                seen_per_signbits[signbits as usize][got as usize] += 1;
            }
        }

        // random boxes and planes, sometimes with a properly ordered box,
        // sometimes inverted, sometimes with the "correct" signbits for the
        // normal, sometimes not
        for _ in 0..3000 {
            let normal = match rng.below(3) {
                0 => rng.dir(),
                1 => rng.vec3_mag(10.0),
                _ => rng.vec3_finite(),
            };
            let mut emins = rng.vec3_mag(100.0);
            let mut emaxs = rng.vec3_mag(100.0);
            if rng.bool() {
                for i in 0..3 {
                    if emins[i] > emaxs[i] {
                        let t = emins[i];
                        emins[i] = emaxs[i];
                        emaxs[i] = t;
                    }
                }
            }
            let dist = match rng.below(4) {
                0 => 0.0,
                1 => rng.f32_mag(100.0),
                2 => rng.f32_mag(1e4),
                _ => rng.f32_finite(),
            };
            let p = CPlane {
                normal,
                dist,
                type_: 3,
                signbits,
                pad: [0, 0],
            };
            let got = box_both(
                c,
                r,
                emins,
                emaxs,
                p,
                &format!("general random signbits={signbits}"),
            );
            seen[got as usize] += 1;
            seen_per_signbits[signbits as usize][got as usize] += 1;
        }

        // extreme magnitudes: products overflow to inf, inf-inf -> NaN
        for _ in 0..500 {
            let normal = [rng.f32_mag(1e30), rng.f32_mag(1e30), rng.f32_mag(1e30)];
            let emins = [rng.f32_mag(1e30), rng.f32_mag(1e30), rng.f32_mag(1e30)];
            let emaxs = [rng.f32_mag(1e30), rng.f32_mag(1e30), rng.f32_mag(1e30)];
            let p = CPlane {
                normal,
                dist: rng.f32_any(),
                type_: 3,
                signbits,
                pad: [0, 0],
            };
            let got = box_both(
                c,
                r,
                emins,
                emaxs,
                p,
                &format!("general overflow signbits={signbits}"),
            );
            seen[got as usize] += 1;
            seen_per_signbits[signbits as usize][got as usize] += 1;
        }
    }

    for want in 0usize..4 {
        assert!(
            seen[want] > 0,
            "general-case return value {want} never occurred: counts {seen:?}"
        );
    }
    for signbits in 0..8 {
        for want in 0usize..4 {
            assert!(
                seen_per_signbits[signbits][want] > 0,
                "signbits={signbits}: return value {want} never occurred: {:?}",
                seen_per_signbits[signbits]
            );
        }
    }
}

// ---------------------------------------------------------------------------
// row 44a / ERRORS.md row 21 -- type out of range (3..255) takes the general case
// ---------------------------------------------------------------------------

#[test]
fn box_on_plane_side_type_out_of_range() {
    let (c, r): (BoxFn, BoxFn) = both("BoxOnPlaneSide");
    let mut rng = Rng::new(44_0006);

    // A handful of fixed configurations, reused for every (type, signbits)
    // pair so that the *only* thing that varies is `type`.
    let mut configs: Vec<([f32; 3], [f32; 3], [f32; 3], f32)> = vec![
        ([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], [1.0, 0.0, 0.0], 0.5),
        ([-1.0, -2.0, -3.0], [4.0, 5.0, 6.0], [0.577, -0.577, 0.577], -2.0),
        ([7.0, 7.0, 7.0], [-7.0, -7.0, -7.0], [-1.0, -1.0, -1.0], 0.0),
        ([1e6, 0.0, -1e6], [1e6, 0.0, -1e6], [1e-6, 1e6, -1.0], 1e3),
    ];
    for _ in 0..4 {
        configs.push((
            rng.vec3_mag(100.0),
            rng.vec3_mag(100.0),
            rng.dir(),
            rng.f32_mag(100.0),
        ));
    }

    for ty in 3u16..=255 {
        for signbits in 0u8..8 {
            for &(emins, emaxs, normal, dist) in &configs {
                let p = CPlane {
                    normal,
                    dist,
                    type_: ty as u8,
                    signbits,
                    pad: [0, 0],
                };
                let got = box_both(
                    c,
                    r,
                    emins,
                    emaxs,
                    p,
                    &format!("type={ty} signbits={signbits}"),
                );
                // `type` only selects the fast path when < 3, so every type in
                // 3..=255 must give the same answer as type == 3.
                let p3 = CPlane { type_: 3, ..p };
                let got3 = box_both(
                    c,
                    r,
                    emins,
                    emaxs,
                    p3,
                    &format!("type=3 reference for signbits={signbits}"),
                );
                assert_int(
                    &format!(
                        "BoxOnPlaneSide(type={ty}, signbits={signbits}) must match type=3 \
                         (emins={emins:?}, emaxs={emaxs:?}, normal={normal:?}, dist={dist:?})"
                    ),
                    got,
                    got3,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// row 44b -- invalid signbits (8..255) hit the `default:` arm, dist1 = dist2 = 0
// ---------------------------------------------------------------------------

#[test]
fn box_on_plane_side_invalid_signbits() {
    let (c, r): (BoxFn, BoxFn) = both("BoxOnPlaneSide");
    let mut rng = Rng::new(44_0007);

    let mut seen = [0usize; 4];

    // dist1 = dist2 = 0, so:  dist < 0 -> 1, dist == 0 -> 1, dist > 0 -> 2,
    // dist NaN -> 0 (both comparisons false).
    let dists: [(f32, c_int, &str); 9] = [
        (-1.0, 1, "dist < 0"),
        (-1e30, 1, "dist very negative"),
        (f32::NEG_INFINITY, 1, "dist -inf"),
        (-0.0, 1, "dist -0.0 (0 >= -0.0 is true)"),
        (0.0, 1, "dist == 0"),
        (1.0, 2, "dist > 0"),
        (1e-45, 2, "dist smallest denormal > 0"),
        (f32::INFINITY, 2, "dist +inf"),
        (f32::NAN, 0, "dist NaN -> both comparisons false"),
    ];

    for signbits in 8u16..=255 {
        for ty in [3u8, 4, 128, 255] {
            for &(dist, expect, tag) in &dists {
                // the box and the normal are irrelevant on this path: feed
                // values that would give a different answer if they were read
                let emins = [1e30f32, f32::NAN, -1e30];
                let emaxs = [-1e30f32, f32::INFINITY, f32::NAN];
                let p = CPlane {
                    normal: [f32::NAN, 1e30, f32::NEG_INFINITY],
                    dist,
                    type_: ty,
                    signbits: signbits as u8,
                    pad: [0, 0],
                };
                let got = box_both(
                    c,
                    r,
                    emins,
                    emaxs,
                    p,
                    &format!("default arm signbits={signbits} type={ty} {tag}"),
                );
                assert_int(
                    &format!(
                        "BoxOnPlaneSide(signbits={signbits}, type={ty}, {tag}) expected {expect}"
                    ),
                    got,
                    expect,
                );
                seen[got as usize] += 1;
            }
        }
    }

    // random junk everywhere, signbits always >= 8
    for _ in 0..8000 {
        let signbits = (8 + rng.below(248)) as u8;
        let p = CPlane {
            normal: rng.vec3_any(),
            dist: rng.f32_any(),
            type_: (3 + rng.below(253)) as u8,
            signbits,
            pad: [0, 0],
        };
        let got = box_both(
            c,
            r,
            rng.vec3_any(),
            rng.vec3_any(),
            p,
            &format!("default arm random signbits={signbits}"),
        );
        seen[got as usize] += 1;
    }

    for want in [0usize, 1, 2] {
        assert!(
            seen[want] > 0,
            "default-arm return value {want} never occurred: counts {seen:?}"
        );
    }
    assert_eq!(
        seen[3], 0,
        "the `default:` arm sets dist1 == dist2 == 0, so 3 is unreachable: counts {seen:?}"
    );
}

// ---------------------------------------------------------------------------
// ERRORS.md row 22 -- NaN anywhere makes both comparisons false -> returns 0
// ---------------------------------------------------------------------------

#[test]
fn box_on_plane_side_nan() {
    let (c, r): (BoxFn, BoxFn) = both("BoxOnPlaneSide");
    let mut rng = Rng::new(22_0008);

    let mut zero_seen_general = 0usize;

    // --- general case, both dist1 and dist2 poisoned -> must return 0 -------
    //
    // Each `switch` arm builds dist1 from one corner and dist2 from the other,
    // so a NaN in a *single* slot of emins or emaxs only poisons one of the two
    // and the other comparison still decides (that case is covered below, with
    // C-vs-Rust agreement only).  Putting the NaN into `normal`, into `dist`, or
    // into the same slot of both corners poisons both sums, which is the
    // ERRORS.md row 22 shape: `dist1 >= dist` and `dist2 < dist` are both false
    // and the function returns 0.
    for signbits in 0u8..8 {
        for slot in 0..3usize {
            for which in 0..3 {
                let mut emins = [-1.0f32, -1.0, -1.0];
                let mut emaxs = [1.0f32, 1.0, 1.0];
                let mut normal = [1.0f32, 1.0, 1.0];
                let mut dist = 0.0f32;
                let tag = match which {
                    0 => {
                        emins[slot] = f32::NAN;
                        emaxs[slot] = f32::NAN;
                        "emins[slot] and emaxs[slot]"
                    }
                    1 => {
                        normal[slot] = f32::NAN;
                        "normal[slot]"
                    }
                    _ => {
                        dist = f32::NAN;
                        "dist"
                    }
                };
                let p = CPlane {
                    normal,
                    dist,
                    type_: 3,
                    signbits,
                    pad: [0, 0],
                };
                let got = box_both(
                    c,
                    r,
                    emins,
                    emaxs,
                    p,
                    &format!("NaN general signbits={signbits} slot={slot} in {tag}"),
                );
                assert_int(
                    &format!(
                        "BoxOnPlaneSide(NaN in {tag}, slot {slot}, signbits={signbits}) must be 0"
                    ),
                    got,
                    0,
                );
                zero_seen_general += 1;
            }
        }
    }

    // A NaN in only one corner poisons only one of dist1/dist2; the surviving
    // comparison still decides, so the result is 0, 1 or 2 -- never 3, because
    // one of the two `if`s can never fire.
    for signbits in 0u8..8 {
        for slot in 0..3usize {
            for which in 0..2 {
                let mut emins = [-1.0f32, -1.0, -1.0];
                let mut emaxs = [1.0f32, 1.0, 1.0];
                if which == 0 {
                    emins[slot] = f32::NAN;
                } else {
                    emaxs[slot] = f32::NAN;
                }
                let p = CPlane {
                    normal: [1.0, 1.0, 1.0],
                    dist: 0.0,
                    type_: 3,
                    signbits,
                    pad: [0, 0],
                };
                let got = box_both(
                    c,
                    r,
                    emins,
                    emaxs,
                    p,
                    &format!(
                        "NaN in one corner, signbits={signbits} slot={slot} \
                         which={}",
                        ["emins", "emaxs"][which]
                    ),
                );
                assert!(
                    got != 3,
                    "BoxOnPlaneSide(NaN in one corner, signbits={signbits}, slot={slot}) \
                     returned 3, but one of dist1/dist2 is NaN"
                );
                if got == 0 {
                    zero_seen_general += 1;
                }
            }
        }
    }

    // `dist` NaN alone is enough, whatever the box is
    for signbits in 0u8..8 {
        for _ in 0..200 {
            let p = CPlane {
                normal: rng.dir(),
                dist: f32::NAN,
                type_: 3,
                signbits,
                pad: [0, 0],
            };
            let got = box_both(
                c,
                r,
                rng.vec3_mag(100.0),
                rng.vec3_mag(100.0),
                p,
                &format!("NaN dist, random box, signbits={signbits}"),
            );
            assert_int(
                &format!("BoxOnPlaneSide(dist=NaN, signbits={signbits}) must be 0"),
                got,
                0,
            );
            zero_seen_general += 1;
        }
    }

    // 0*inf and inf-inf manufacture NaNs from non-NaN inputs
    for signbits in 0u8..8 {
        for &(n, m) in &[
            (f32::INFINITY, 0.0f32),
            (0.0f32, f32::INFINITY),
            (f32::INFINITY, f32::NEG_INFINITY),
            (1e30f32, 1e30f32),
        ] {
            let emins = [m, m, m];
            let emaxs = [-m, -m, -m];
            let p = CPlane {
                normal: [n, n, n],
                dist: 0.0,
                type_: 3,
                signbits,
                pad: [0, 0],
            };
            let got = box_both(
                c,
                r,
                emins,
                emaxs,
                p,
                &format!("manufactured NaN signbits={signbits} n={n:?} m={m:?}"),
            );
            if got == 0 {
                zero_seen_general += 1;
            }
        }
    }

    // --- axial case: `dist` NaN makes both comparisons false -> returns 3 ----
    for t in 0u8..3 {
        for which in 0..3 {
            let mut emins = [-1.0f32; 3];
            let mut emaxs = [1.0f32; 3];
            let mut dist = 0.0f32;
            match which {
                0 => emins[t as usize] = f32::NAN,
                1 => emaxs[t as usize] = f32::NAN,
                _ => dist = f32::NAN,
            }
            let p = CPlane {
                normal: [0.0; 3],
                dist,
                type_: t,
                signbits: 0,
                pad: [0, 0],
            };
            let got = box_both(
                c,
                r,
                emins,
                emaxs,
                p,
                &format!("NaN axial type={t} which={which}"),
            );
            // On the axial path a NaN comparison falls through to `return 3`
            // (there is no `sides` accumulator), so 0 is impossible here.
            assert_int(
                &format!("BoxOnPlaneSide(NaN axial type={t} which={which}) falls through to 3"),
                got,
                3,
            );
        }
    }

    // --- fully random NaN soup ---------------------------------------------
    for _ in 0..10000 {
        let p = CPlane {
            normal: rng.vec3_any(),
            dist: rng.f32_any(),
            type_: match rng.below(4) {
                0 => 0,
                1 => 1,
                2 => 2,
                _ => (3 + rng.below(253)) as u8,
            },
            signbits: (rng.next_u32() & 0xff) as u8,
            pad: [0, 0],
        };
        box_both(c, r, rng.vec3_any(), rng.vec3_any(), p, "NaN soup");
    }

    assert!(
        zero_seen_general > 0,
        "expected the general case to return 0 for NaN inputs"
    );
}

// ---------------------------------------------------------------------------
// row 48 -- DirToByte over the whole bytedirs table
// ---------------------------------------------------------------------------

#[test]
fn dir_to_byte() {
    let (c, r): (DirToByteFn, DirToByteFn) = both("DirToByte");
    let mut rng = Rng::new(48_0009);

    // the table the C library itself indexes, read straight out of its .so
    let bytedirs: [[f32; 3]; 162] = unsafe { *cdata::<[[f32; 3]; 162]>("bytedirs") };
    // the Rust library's copy must be identical for the results to be
    // comparable at all (row 59 checks this on its own, too)
    let rbytedirs: [[f32; 3]; 162] = unsafe { *rdata::<[[f32; 3]; 162]>("bytedirs") };
    for i in 0..162 {
        assert_vec(
            &format!("bytedirs[{i}]"),
            &bytedirs[i][..],
            &rbytedirs[i][..],
        );
    }

    let run = |dir: [f32; 3], tag: &str| -> c_int {
        let (mut dc, mut dr) = (dir, dir);
        let rc = unsafe { c(dc.as_mut_ptr()) };
        let rr = unsafe { r(dr.as_mut_ptr()) };
        let ctx = format!("DirToByte({tag}, dir={dir:?})");
        assert_int(&format!("{ctx} return"), rc, rr);
        // DirToByte must not write through `dir`
        assert_vec(&format!("{ctx} dir after"), &dc, &dr);
        assert_vec(&format!("{ctx} dir unmodified"), &dir, &dc);
        assert!(
            (0..162).contains(&rc),
            "{ctx}: C returned {rc}, not a valid bytedirs index"
        );
        rc
    };

    // NULL is explicitly handled (`if (!dir) return 0;`)
    {
        let rc = unsafe { c(core::ptr::null_mut()) };
        let rr = unsafe { r(core::ptr::null_mut()) };
        assert_int("DirToByte(NULL) return", rc, rr);
        assert_int("DirToByte(NULL) must be 0", rc, 0);
    }

    // every table entry must map back to its own index (the dot product with
    // itself is the unique maximum, and `d > bestd` keeps the first maximum)
    for i in 0..162 {
        let got = run(bytedirs[i], &format!("bytedirs[{i}]"));
        assert_int(&format!("DirToByte(bytedirs[{i}]) must be {i}"), got, i as c_int);
    }
    // ... and the negated entries (all dot products <= 0 against the entry
    // itself, so a different index -- or 0 -- wins)
    for i in 0..162 {
        let d = [-bytedirs[i][0], -bytedirs[i][1], -bytedirs[i][2]];
        run(d, &format!("-bytedirs[{i}]"));
    }
    // scaled entries: scaling by a positive factor cannot change the winner,
    // scaling by a negative one flips it like the negation above
    for i in (0..162).step_by(7) {
        for k in [1e-30f32, 1e-6, 0.5, 2.0, 1e6, 1e30, -1.0, -1e30, 0.0, -0.0] {
            let d = [bytedirs[i][0] * k, bytedirs[i][1] * k, bytedirs[i][2] * k];
            run(d, &format!("bytedirs[{i}] * {k:?}"));
        }
    }

    // zero vector: every dot product is 0, never > bestd == 0, so 0 is returned
    for z in [[0.0f32, 0.0, 0.0], [-0.0f32, -0.0, -0.0], [0.0, -0.0, 0.0]] {
        let got = run(z, "zero vector");
        assert_int(&format!("DirToByte({z:?}) must be 0"), got, 0);
    }

    // inf / NaN / denormal / huge
    let odd = [
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        1e30,
        -1e30,
        f32::MAX,
        -f32::MAX,
        1e-45,
        -1e-45,
        f32::MIN_POSITIVE,
        0.0,
        -0.0,
        1.0,
        -1.0,
    ];
    for &x in &odd {
        for &y in &odd {
            for &z in &odd {
                run([x, y, z], "extreme components");
            }
            // and mixed with a table entry
            for i in (0..162).step_by(31) {
                run([x, y, bytedirs[i][2]], "half-extreme");
            }
        }
    }

    // 20000 random directions
    for _ in 0..20000 {
        let d = match rng.below(4) {
            0 => rng.vec3_any(),
            1 => rng.vec3_finite(),
            2 => rng.dir(),
            _ => rng.vec3_mag(1.0),
        };
        run(d, "random");
    }
    // random directions very close to table entries (the `>` vs `>=` boundary)
    for _ in 0..5000 {
        let i = rng.below(162) as usize;
        let eps = match rng.below(3) {
            0 => 0.0f32,
            1 => 1e-7,
            _ => rng.f32_mag(1e-3),
        };
        let d = [
            bytedirs[i][0] + eps,
            bytedirs[i][1] - eps,
            bytedirs[i][2] + eps,
        ];
        run(d, &format!("near bytedirs[{i}]"));
    }
}

// ---------------------------------------------------------------------------
// row 49 -- ByteToDir
// ---------------------------------------------------------------------------

#[test]
fn byte_to_dir() {
    let (c, r): (ByteToDirFn, ByteToDirFn) = both("ByteToDir");
    let mut rng = Rng::new(49_0010);

    let bytedirs: [[f32; 3]; 162] = unsafe { *cdata::<[[f32; 3]; 162]>("bytedirs") };

    let run = |b: c_int| -> [f32; 3] {
        // 4 slots: the 4th is a guard, ByteToDir writes a vec3_t only
        let mut dc = SENTINEL;
        let mut dr = SENTINEL;
        unsafe { c(b, dc.as_mut_ptr()) };
        unsafe { r(b, dr.as_mut_ptr()) };
        let ctx = format!("ByteToDir({b})");
        assert_vec(&format!("{ctx} dir"), &dc, &dr);
        assert_vec(&format!("{ctx} guard slots"), &dc[3..], &SENTINEL[3..]);
        assert!(
            dc[0].to_bits() != SENTINEL[0].to_bits()
                || dc[1].to_bits() != SENTINEL[1].to_bits()
                || dc[2].to_bits() != SENTINEL[2].to_bits()
                || dc[0].to_bits() == 0,
            "{ctx}: dir was not written at all: {dc:?}"
        );
        [dc[0], dc[1], dc[2]]
    };

    // every valid index copies the table entry verbatim
    for b in 0..162 {
        let got = run(b as c_int);
        assert_vec(
            &format!("ByteToDir({b}) must copy bytedirs[{b}]"),
            &bytedirs[b][..],
            &got[..],
        );
    }

    // out of range -> vec3_origin
    for b in [
        -1i32,
        -2,
        162,
        163,
        164,
        255,
        256,
        1000,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
        -161,
        -162,
        -163,
    ] {
        let got = run(b);
        assert_vec(
            &format!("ByteToDir({b}) out of range must be vec3_origin"),
            &[0.0f32, 0.0, 0.0],
            &got[..],
        );
    }

    // random ints, mostly in and around the valid range
    for _ in 0..20000 {
        let b = match rng.below(4) {
            0 => rng.i32_any(),
            1 => rng.below(324) as i32 - 162,
            2 => rng.below(162) as i32,
            _ => rng.next_u32() as i32,
        };
        run(b);
    }
}
