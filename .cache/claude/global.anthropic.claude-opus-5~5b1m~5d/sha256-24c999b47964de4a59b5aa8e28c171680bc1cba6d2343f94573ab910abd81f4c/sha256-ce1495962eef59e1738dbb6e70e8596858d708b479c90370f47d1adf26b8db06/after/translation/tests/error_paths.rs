//! Phase C — one differential test per row of `ERRORS.md` (E01..E93).
//!
//! The library has no error-code channel at all (no `assert`, no `return -1`,
//! no `errno`), so its rejection surface is made of null-pointer sentinels,
//! `switch` fallbacks for out-of-range values, and unguarded division /
//! `sqrtf`.  Each test constructs the exact invalid input and asserts that both
//! `.so`s reject it identically — same sentinel behaviour, same returned
//! `inf`/`NaN` bit pattern, same untouched output bytes.
//!
//! The handful of rows whose C behaviour is genuine undefined behaviour (reads
//! of uninitialised stack) are marked `UB:` in the test name; those assert the
//! property both libraries really do share (the call returns normally) and the
//! measured divergence is documented in ERRORS.md.

mod common;
use common::*;

use std::os::unix::process::ExitStatusExt;
use std::ptr;

// ===========================================================================
// E01..E09 — c2GJK null optional pointers
// ===========================================================================

const POISON_A: C2v = C2v {
    x: -12345.5,
    y: 6789.25,
};
const POISON_B: C2v = C2v {
    x: 24680.125,
    y: -1357.75,
};
const POISON_IT: i32 = -987654321;

struct GjkOut {
    dist: f32,
    a: C2v,
    b: C2v,
    it: i32,
    cache: C2GJKCache,
    cache_used: bool,
}

#[allow(clippy::too_many_arguments)]
fn gjk_call(
    f: FnGJK,
    sa: &ShapeBlob,
    ta: u32,
    ax: *const C2x,
    sb: &ShapeBlob,
    tb: u32,
    bx: *const C2x,
    pass_a: bool,
    pass_b: bool,
    pass_it: bool,
    use_radius: i32,
    cache0: Option<C2GJKCache>,
) -> GjkOut {
    let mut a = POISON_A;
    let mut b = POISON_B;
    let mut it = POISON_IT;
    let mut cache = cache0.unwrap_or(C2GJKCache {
        metric: 0.0,
        count: 0,
        iA: [0; 3],
        iB: [0; 3],
        div: 0.0,
    });
    let cache_ptr = if cache0.is_some() {
        &mut cache as *mut C2GJKCache
    } else {
        ptr::null_mut()
    };
    let dist = unsafe {
        f(
            sa.as_ptr(),
            ta,
            ax,
            sb.as_ptr(),
            tb,
            bx,
            if pass_a { &mut a } else { ptr::null_mut() },
            if pass_b { &mut b } else { ptr::null_mut() },
            use_radius,
            if pass_it { &mut it } else { ptr::null_mut() },
            cache_ptr,
        )
    };
    GjkOut {
        dist,
        a,
        b,
        it,
        cache,
        cache_used: cache0.is_some(),
    }
}

#[track_caller]
fn gjk_same(c: &GjkOut, r: &GjkOut, ctx: &str) {
    assert!(
        f32_same(c.dist, r.dist),
        "[{ctx}] return: C {} Rust {}",
        fmt_f32(c.dist),
        fmt_f32(r.dist)
    );
    assert!(
        v_same(c.a, r.a),
        "[{ctx}] outA: C {} Rust {}",
        fmt_v(c.a),
        fmt_v(r.a)
    );
    assert!(
        v_same(c.b, r.b),
        "[{ctx}] outB: C {} Rust {}",
        fmt_v(c.b),
        fmt_v(r.b)
    );
    assert_eq!(c.it, r.it, "[{ctx}] iterations");
    assert_eq!(c.cache_used, r.cache_used);
    if c.cache_used {
        assert!(
            raw_same(&c.cache, &r.cache),
            "[{ctx}] cache: C {} Rust {}",
            fmt_cache(&c.cache),
            fmt_cache(&r.cache)
        );
    }
}

fn sample_shapes(rng: &mut Rng) -> Vec<(u32, ShapeBlob)> {
    vec![
        (
            C2_TYPE_CIRCLE,
            ShapeBlob::circle(rand_circle(rng, 60.0)),
        ),
        (C2_TYPE_AABB, ShapeBlob::aabb(rand_aabb(rng, 60.0))),
        (
            C2_TYPE_CAPSULE,
            ShapeBlob::capsule(rand_capsule(rng, 60.0)),
        ),
    ]
}

#[test]
fn e01_e09_gjk_null_optional_pointers() {
    let (c, r): (FnGJK, FnGJK) = sym(b"c2GJK");
    let mut rng = Rng::new(0xE01);
    for iter in 0..256u32 {
        let sa_list = sample_shapes(&mut rng);
        let sb_list = sample_shapes(&mut rng);
        let axv = rng.x_unit(120.0);
        let bxv = rng.x_unit(120.0);
        for (ta, sa) in &sa_list {
            for (tb, sb) in &sb_list {
                // E01/E02/E03: transform pointers
                for (ax, bx, tag) in [
                    (ptr::null(), ptr::null(), "E03 both NULL"),
                    (&axv as *const C2x, ptr::null(), "E02 bx NULL"),
                    (ptr::null(), &bxv as *const C2x, "E01 ax NULL"),
                    (&axv as *const C2x, &bxv as *const C2x, "both given"),
                ] {
                    // E04..E07/E09: every out-pointer mask
                    for mask in 0..8u32 {
                        let pass_a = mask & 1 != 0;
                        let pass_b = mask & 2 != 0;
                        let pass_it = mask & 4 != 0;
                        // E08: cache NULL vs given
                        for cache0 in [
                            None,
                            Some(C2GJKCache {
                                metric: 3.5,
                                count: 0,
                                iA: [9, 9, 9],
                                iB: [8, 8, 8],
                                div: 7.5,
                            }),
                        ] {
                            let ctx = format!(
                                "{tag} mask={mask} cache={} {}/{} #{iter}",
                                cache0.is_some(),
                                type_name(*ta),
                                type_name(*tb)
                            );
                            let co = gjk_call(
                                c, sa, *ta, ax, sb, *tb, bx, pass_a, pass_b, pass_it, 1, cache0,
                            );
                            let ro = gjk_call(
                                r, sa, *ta, ax, sb, *tb, bx, pass_a, pass_b, pass_it, 1, cache0,
                            );
                            gjk_same(&co, &ro, &ctx);
                            // NULL out-pointers must leave the caller's value alone.
                            if !pass_a {
                                assert!(v_same(co.a, POISON_A), "[{ctx}] C wrote through NULL outA");
                                assert!(
                                    v_same(ro.a, POISON_A),
                                    "[{ctx}] Rust wrote through NULL outA"
                                );
                            }
                            if !pass_b {
                                assert!(v_same(co.b, POISON_B), "[{ctx}] C wrote through NULL outB");
                                assert!(
                                    v_same(ro.b, POISON_B),
                                    "[{ctx}] Rust wrote through NULL outB"
                                );
                            }
                            if !pass_it {
                                assert_eq!(co.it, POISON_IT, "[{ctx}] C wrote NULL iterations");
                                assert_eq!(ro.it, POISON_IT, "[{ctx}] Rust wrote NULL iterations");
                            }
                        }
                    }
                }
            }
        }
    }
}

// ===========================================================================
// E10/E11 — gjk_cache never touches a9/b9  (see also tests/gjk_cache_entry.rs)
// ===========================================================================

#[test]
fn e10_e11_gjk_cache_out_params_untouched() {
    let (c, r): (FnGjkCache, FnGjkCache) = sym(b"gjk_cache");
    let mut rng = Rng::new(0xE10);
    for i in 0..512u32 {
        let p: [f32; 9] = std::array::from_fn(|_| rng.range(-200.0, 200.0));
        let sentinel_a = C2v {
            x: f32::from_bits(0x1234_5678),
            y: f32::from_bits(0x9ABC_DEF0),
        };
        let sentinel_b = C2v {
            x: f32::from_bits(0x0F0F_0F0F),
            y: f32::from_bits(0xF0F0_F0F0),
        };
        for rev in [0i8, 1] {
            // E11: non-NULL
            let mut ca = sentinel_a;
            let mut cb = sentinel_b;
            let mut ra = sentinel_a;
            let mut rb = sentinel_b;
            unsafe {
                c(
                    rev as core::ffi::c_char,
                    &mut ca,
                    &mut cb,
                    p[0],
                    p[1],
                    p[2],
                    p[3],
                    p[4],
                    p[5],
                    p[6],
                    p[7],
                    p[8],
                )
            };
            unsafe {
                r(
                    rev as core::ffi::c_char,
                    &mut ra,
                    &mut rb,
                    p[0],
                    p[1],
                    p[2],
                    p[3],
                    p[4],
                    p[5],
                    p[6],
                    p[7],
                    p[8],
                )
            };
            assert!(v_same(ca, sentinel_a) && v_same(cb, sentinel_b), "C wrote a9/b9");
            assert!(
                v_same(ra, sentinel_a) && v_same(rb, sentinel_b),
                "Rust wrote a9/b9 (#{i} rev={rev})"
            );
            assert!(v_same(ca, ra) && v_same(cb, rb));
            // E10: NULL
            unsafe {
                c(
                    rev as core::ffi::c_char,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    p[0],
                    p[1],
                    p[2],
                    p[3],
                    p[4],
                    p[5],
                    p[6],
                    p[7],
                    p[8],
                )
            };
            unsafe {
                r(
                    rev as core::ffi::c_char,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    p[0],
                    p[1],
                    p[2],
                    p[3],
                    p[4],
                    p[5],
                    p[6],
                    p[7],
                    p[8],
                )
            };
        }
    }
}

// ===========================================================================
// E12..E15 — out-of-range C2_TYPE into c2MakeProxy
// ===========================================================================

const BAD_TYPES: [u32; 10] = [
    3,
    4,
    5,
    100,
    0x7FFF_FFFF,
    0x8000_0000,
    0xFFFF_FFFF, // -1
    0xFFFF_FFFE, // -2
    0x0000_FFFF,
    0xDEAD_BEEF,
];

#[test]
fn e12_e14_makeproxy_invalid_enum_writes_nothing() {
    let (c, r): (FnMakeProxy, FnMakeProxy) = sym(b"c2MakeProxy");
    let mut rng = Rng::new(0xE12);
    for &ty in &BAD_TYPES {
        for _ in 0..64 {
            let shape = ShapeBlob::circle(rand_circle(&mut rng, 100.0));
            let mut base = C2Proxy {
                radius: rng.spicy(),
                count: rng.next_u32() as i32,
                verts: [C2v::default(); 8],
            };
            for v in base.verts.iter_mut() {
                *v = rng.v_spicy();
            }
            let mut pc = base;
            let mut pr = base;
            unsafe { c(shape.as_ptr(), ty, &mut pc) };
            unsafe { r(shape.as_ptr(), ty, &mut pr) };
            assert_raw(&pc, &pr, &format!("c2MakeProxy type={ty:#x}"));
            assert_raw(
                &base,
                &pc,
                &format!("C modified *p for invalid type {ty:#x} (unexpected)"),
            );
            assert_raw(
                &base,
                &pr,
                &format!("Rust modified *p for invalid type {ty:#x}"),
            );
        }
    }
}

#[test]
fn e15_makeproxy_valid_type_leaves_other_fields() {
    let (c, r): (FnMakeProxy, FnMakeProxy) = sym(b"c2MakeProxy");
    let mut rng = Rng::new(0xE15);
    for ty in ALL_TYPES {
        for _ in 0..256 {
            let shape = rand_shape(&mut rng, ty, 100.0);
            let mut base = C2Proxy {
                radius: rng.spicy(),
                count: rng.next_u32() as i32,
                verts: [C2v::default(); 8],
            };
            for v in base.verts.iter_mut() {
                *v = rng.v_spicy();
            }
            let mut pc = base;
            let mut pr = base;
            unsafe { c(shape.as_ptr(), ty, &mut pc) };
            unsafe { r(shape.as_ptr(), ty, &mut pr) };
            assert_raw(&pc, &pr, &format!("c2MakeProxy {}", type_name(ty)));
            // Vertices the arm does not assign must keep the poisoned bytes.
            let written = match ty {
                C2_TYPE_CIRCLE => 1,
                C2_TYPE_AABB => 4,
                _ => 2,
            };
            for k in written..8 {
                assert!(
                    v_same(pc.verts[k], base.verts[k]),
                    "C overwrote verts[{k}] for {}",
                    type_name(ty)
                );
                assert!(
                    v_same(pr.verts[k], base.verts[k]),
                    "Rust overwrote verts[{k}] for {}",
                    type_name(ty)
                );
            }
        }
    }
}

// ===========================================================================
// E16/E17 — UB: out-of-range C2_TYPE into c2GJK
// ===========================================================================


// ===========================================================================
// E18..E26 — switch(count) fallbacks
// ===========================================================================

const BAD_COUNTS: [i32; 9] = [0, 4, 5, 6, -1, -2, 1000, i32::MIN, i32::MAX];

fn filled_simplex(rng: &mut Rng, count: i32, div: f32) -> C2Simplex {
    let rng = &mut Rng::new(rng.next_u64());
    let mut s = C2Simplex {
        verts: [C2sv::default(); 4],
        div,
        count,
    };
    for i in 0..4 {
        s.verts[i] = C2sv {
            sA: rng.v_range(-100.0, 100.0),
            sB: rng.v_range(-100.0, 100.0),
            p: rng.v_range(-100.0, 100.0),
            u: rng.range(-5.0, 5.0),
            iA: 11 + i as i32,
            iB: 71 + i as i32,
        };
    }
    s
}

#[test]
fn e18_e19_metric_bad_count_returns_zero() {
    let (c, r): (FnFSimplex, FnFSimplex) = sym(b"c2GJKSimplexMetric");
    let mut rng = Rng::new(0xE18);
    for &count in &BAD_COUNTS {
        for _ in 0..64 {
            let mut sc = { let d = rng.spicy(); filled_simplex(&mut rng, count, d) };
            let mut sr = sc;
            let mc = unsafe { c(&mut sc) };
            let mr = unsafe { r(&mut sr) };
            assert_f32(mc, mr, &format!("metric count={count}"));
            assert!(
                f32_same(mc, 0.0),
                "expected exactly +0.0 from the default arm, got {}",
                fmt_f32(mc)
            );
        }
    }
    // count == 1 also returns 0 (the `default:` falls through to `case 1:`)
    for _ in 0..64 {
        let mut sc = { let d = rng.spicy(); filled_simplex(&mut rng, 1, d) };
        let mut sr = sc;
        assert_f32(unsafe { c(&mut sc) }, unsafe { r(&mut sr) }, "metric count=1");
    }
}

#[test]
fn e20_e21_c2d_bad_count_returns_zero_vec() {
    let (c, r): (FnVSimplex, FnVSimplex) = sym(b"c2D");
    let mut rng = Rng::new(0xE20);
    for &count in BAD_COUNTS.iter().chain([3i32].iter()) {
        for _ in 0..64 {
            let mut sc = { let d = rng.spicy(); filled_simplex(&mut rng, count, d) };
            let mut sr = sc;
            let dc = unsafe { c(&mut sc) };
            let dr = unsafe { r(&mut sr) };
            assert_v(dc, dr, &format!("c2D count={count}"));
            assert!(
                v_same(dc, C2v { x: 0.0, y: 0.0 }),
                "expected (+0,+0) from the default arm, got {}",
                fmt_v(dc)
            );
            assert_raw(&sc, &sr, "c2D must not mutate");
        }
    }
}

#[test]
fn e22_e23_c2l_bad_count_returns_zero_vec() {
    let (c, r): (FnVSimplex, FnVSimplex) = sym(b"c2L");
    let mut rng = Rng::new(0xE22);
    for &count in BAD_COUNTS.iter().chain([3i32].iter()) {
        for div in [1.0f32, 0.0, -0.0, f32::NAN, f32::INFINITY, FLT_MAX] {
            for _ in 0..16 {
                let mut sc = filled_simplex(&mut rng, count, div);
                let mut sr = sc;
                let vc = unsafe { c(&mut sc) };
                let vr = unsafe { r(&mut sr) };
                assert_v(vc, vr, &format!("c2L count={count} div={}", fmt_f32(div)));
                assert!(
                    v_same(vc, C2v { x: 0.0, y: 0.0 }),
                    "expected (+0,+0), got {} (div={})",
                    fmt_v(vc),
                    fmt_f32(div)
                );
            }
        }
    }
}

#[test]
fn e24_witness_bad_count_writes_zero_vecs() {
    let (c, r): (FnWitness, FnWitness) = sym(b"c2Witness");
    let mut rng = Rng::new(0xE24);
    for &count in &BAD_COUNTS {
        for div in [1.0f32, 0.0, f32::NAN] {
            for _ in 0..16 {
                let mut sc = filled_simplex(&mut rng, count, div);
                let mut sr = sc;
                let (mut ac, mut bc) = (POISON_A, POISON_B);
                let (mut ar, mut br) = (POISON_A, POISON_B);
                unsafe { c(&mut sc, &mut ac, &mut bc) };
                unsafe { r(&mut sr, &mut ar, &mut br) };
                assert_v(ac, ar, &format!("c2Witness a count={count}"));
                assert_v(bc, br, &format!("c2Witness b count={count}"));
                assert!(
                    v_same(ac, C2v { x: 0.0, y: 0.0 }) && v_same(bc, C2v { x: 0.0, y: 0.0 }),
                    "expected zeros from the default arm"
                );
            }
        }
    }
}

#[test]
fn e25_e26_gjk_cache_forces_simplex_counts() {
    // A cache with count 1..3 makes `c2GJK` enter the loop with that simplex
    // size, i.e. it reaches `case 3: c23(&s)` on the very first iteration.
    // A cache with count 4 makes `switch (s.count)` fall through with no case.
    let (c, r): (FnGJK, FnGJK) = sym(b"c2GJK");
    let mut rng = Rng::new(0xE25);
    for count in [1i32, 2, 3, 4] {
        for _ in 0..128 {
            let bb = ShapeBlob::aabb(rand_aabb(&mut rng, 80.0));
            let cap = ShapeBlob::capsule(rand_capsule(&mut rng, 80.0));
            // Indices must stay inside the proxies (AABB has 4 verts, capsule 2),
            // and for count == 4 the aliased reads `iA[3] -> iB[0]` and
            // `iB[3] -> div` must also be valid small indices.
            let cache0 = C2GJKCache {
                metric: rng.range(-10.0, 10.0),
                count,
                iA: [0, 1, 2],
                iB: [0, 1, 0],
                div: f32::from_bits(1), // reads back as vertex index 1
            };
            let co = gjk_call(
                c,
                &bb,
                C2_TYPE_AABB,
                ptr::null(),
                &cap,
                C2_TYPE_CAPSULE,
                ptr::null(),
                true,
                true,
                true,
                1,
                Some(cache0),
            );
            let ro = gjk_call(
                r,
                &bb,
                C2_TYPE_AABB,
                ptr::null(),
                &cap,
                C2_TYPE_CAPSULE,
                ptr::null(),
                true,
                true,
                true,
                1,
                Some(cache0),
            );
            gjk_same(&co, &ro, &format!("cache count={count}"));
        }
    }
}

// ===========================================================================
// E27..E36 — division by zero / degenerate float "rejections"
// ===========================================================================

#[test]
fn e27_e29_c2div_by_zero_and_nan() {
    let (c, r): (FnVvf, FnVvf) = sym(b"c2Div");
    let numerators = [
        C2v { x: 1.0, y: -1.0 },
        C2v { x: 0.0, y: -0.0 },
        C2v { x: FLT_MAX, y: FLT_MIN },
        C2v {
            x: f32::INFINITY,
            y: f32::NEG_INFINITY,
        },
        C2v { x: f32::NAN, y: 1.0 },
        C2v { x: 1e-45, y: -1e-45 },
    ];
    for &b in &[0.0f32, -0.0, f32::NAN, -f32::NAN] {
        for &a in &numerators {
            let cv = unsafe { c(a, b) };
            let rv = unsafe { r(a, b) };
            assert_v(cv, rv, &format!("c2Div {} / {}", fmt_v(a), fmt_f32(b)));
            // sanity: the C really does produce non-finite output here
            assert!(
                !cv.x.is_finite() || !cv.y.is_finite() || (a.x == 0.0 && a.y == 0.0),
                "expected a non-finite component"
            );
        }
    }
}

#[test]
fn e30_e33_c2norm_and_c2len_degenerate() {
    let (cn, rn): (FnVv, FnVv) = sym(b"c2Norm");
    let (cl, rl): (FnFv, FnFv) = sym(b"c2Len");
    let cases = [
        C2v { x: 0.0, y: 0.0 },
        C2v { x: -0.0, y: 0.0 },
        C2v { x: 0.0, y: -0.0 },
        C2v { x: -0.0, y: -0.0 },
        C2v {
            x: f32::INFINITY,
            y: 0.0,
        },
        C2v {
            x: f32::INFINITY,
            y: f32::INFINITY,
        },
        C2v {
            x: f32::INFINITY,
            y: f32::NEG_INFINITY,
        },
        C2v { x: f32::NAN, y: 0.0 },
        C2v { x: 0.0, y: f32::NAN },
        C2v {
            x: f32::NAN,
            y: f32::INFINITY,
        },
        C2v { x: FLT_MAX, y: FLT_MAX },
        C2v { x: 1e-45, y: 1e-45 },
        C2v { x: FLT_MIN, y: FLT_MIN },
    ];
    for &a in &cases {
        assert_v(unsafe { cn(a) }, unsafe { rn(a) }, &format!("c2Norm {}", fmt_v(a)));
        assert_f32(unsafe { cl(a) }, unsafe { rl(a) }, &format!("c2Len {}", fmt_v(a)));
    }
    // E30 specifically: the zero vector must normalise to (NaN, NaN)
    let z = C2v { x: 0.0, y: 0.0 };
    let got = unsafe { cn(z) };
    assert!(
        got.x.is_nan() && got.y.is_nan(),
        "expected NaN from c2Norm({{0,0}}), got {}",
        fmt_v(got)
    );
    assert_v(got, unsafe { rn(z) }, "c2Norm zero vector");
}

#[test]
fn e34_e36_witness_and_l_div_zero() {
    let (cw, rw): (FnWitness, FnWitness) = sym(b"c2Witness");
    let (cl, rl): (FnVSimplex, FnVSimplex) = sym(b"c2L");
    let mut rng = Rng::new(0xE34);
    for div in [0.0f32, -0.0, f32::NAN, -f32::NAN, f32::INFINITY, 1e-45] {
        for count in [1i32, 2, 3] {
            for _ in 0..64 {
                let mut sc = filled_simplex(&mut rng, count, div);
                let mut sr = sc;
                let (mut ac, mut bc) = (POISON_A, POISON_B);
                let (mut ar, mut br) = (POISON_A, POISON_B);
                unsafe { cw(&mut sc, &mut ac, &mut bc) };
                unsafe { rw(&mut sr, &mut ar, &mut br) };
                assert_v(
                    ac,
                    ar,
                    &format!("c2Witness a count={count} div={}", fmt_f32(div)),
                );
                assert_v(
                    bc,
                    br,
                    &format!("c2Witness b count={count} div={}", fmt_f32(div)),
                );

                let mut sc = filled_simplex(&mut rng, count, div);
                let mut sr = sc;
                assert_v(
                    unsafe { cl(&mut sc) },
                    unsafe { rl(&mut sr) },
                    &format!("c2L count={count} div={}", fmt_f32(div)),
                );
            }
        }
    }
}

#[test]
fn e37_e40_c22_c23_degenerate_arms() {
    let (c22c, c22r): (FnSimplex, FnSimplex) = sym(b"c22");
    let (c23c, c23r): (FnSimplex, FnSimplex) = sym(b"c23");
    let mut rng = Rng::new(0xE37);

    // E37: a.p == b.p -> the `v <= 0` arm (count = 1, div = 1)
    for _ in 0..256 {
        let p = rng.v_range(-100.0, 100.0);
        let mut sc = { let d = rng.range(0.5, 4.0); filled_simplex(&mut rng, 2, d) };
        sc.verts[0].p = p;
        sc.verts[1].p = p;
        let mut sr = sc;
        unsafe { c22c(&mut sc) };
        unsafe { c22r(&mut sr) };
        assert_raw(&sc, &sr, "E37 c22 a.p==b.p");
        assert_eq!(sc.count, 1);
        assert!(f32_same(sc.div, 1.0));
    }

    // E38: NaN in a.p / b.p -> the final `else` arm (count = 2, div = NaN)
    for _ in 0..256 {
        let mut sc = { let d = rng.range(0.5, 4.0); filled_simplex(&mut rng, 2, d) };
        sc.verts[0].p = C2v {
            x: f32::NAN,
            y: rng.range(-1.0, 1.0),
        };
        sc.verts[1].p = rng.v_range(-10.0, 10.0);
        let mut sr = sc;
        unsafe { c22c(&mut sc) };
        unsafe { c22r(&mut sr) };
        assert_raw(&sc, &sr, "E38 c22 NaN");
        assert_eq!(sc.count, 2, "NaN must fall through to the `else` arm");
        assert!(sc.div.is_nan());
    }

    // E39: fully degenerate triangle -> arm 1 (count = 1, div = 1)
    for _ in 0..256 {
        let p = rng.v_range(-100.0, 100.0);
        let mut sc = { let d = rng.range(0.5, 4.0); filled_simplex(&mut rng, 3, d) };
        sc.verts[0].p = p;
        sc.verts[1].p = p;
        sc.verts[2].p = p;
        let mut sr = sc;
        unsafe { c23c(&mut sc) };
        unsafe { c23r(&mut sr) };
        assert_raw(&sc, &sr, "E39 c23 all-equal");
        assert_eq!(sc.count, 1);
        assert!(f32_same(sc.div, 1.0));
    }

    // E40: NaN triangle -> the final `else` arm (count = 3, div = NaN)
    for _ in 0..256 {
        let mut sc = { let d = rng.range(0.5, 4.0); filled_simplex(&mut rng, 3, d) };
        sc.verts[0].p = C2v {
            x: f32::NAN,
            y: f32::NAN,
        };
        sc.verts[1].p = rng.v_range(-10.0, 10.0);
        sc.verts[2].p = rng.v_range(-10.0, 10.0);
        let mut sr = sc;
        unsafe { c23c(&mut sc) };
        unsafe { c23r(&mut sr) };
        assert_raw(&sc, &sr, "E40 c23 NaN");
        assert_eq!(sc.count, 3, "NaN must fall through to the `else` arm");
        assert!(sc.div.is_nan());
    }
}

// ===========================================================================
// E41..E45 — c2GJK radius / NaN / overflow branches
// ===========================================================================

#[test]
fn e41_e43_radius_branches() {
    let (c, r): (FnGJK, FnGJK) = sym(b"c2GJK");
    let mut rng = Rng::new(0xE41);
    let mut collapsed = 0u32;
    let mut shrunk = 0u32;
    let mut forced_zero = 0u32;
    for _ in 0..4096 {
        // Vary the radii and the separation so both sides of
        // `dist > rA + rB && dist > FLT_EPSILON` are taken.
        let sep = match rng.below(3) {
            0 => rng.range(0.0, 1.0),
            1 => rng.range(0.0, 60.0),
            _ => rng.range(0.0, 1e-6),
        };
        let ra = rng.range(0.0, 30.0);
        let rb = rng.range(0.0, 30.0);
        let a = ShapeBlob::circle(C2Circle {
            p: C2v { x: 0.0, y: 0.0 },
            r: ra,
        });
        let b = ShapeBlob::circle(C2Circle {
            p: C2v { x: sep, y: 0.0 },
            r: rb,
        });
        let co = gjk_call(
            c,
            &a,
            C2_TYPE_CIRCLE,
            ptr::null(),
            &b,
            C2_TYPE_CIRCLE,
            ptr::null(),
            true,
            true,
            true,
            1,
            Some(C2GJKCache {
                metric: 0.0,
                count: 0,
                iA: [0; 3],
                iB: [0; 3],
                div: 0.0,
            }),
        );
        let ro = gjk_call(
            r,
            &a,
            C2_TYPE_CIRCLE,
            ptr::null(),
            &b,
            C2_TYPE_CIRCLE,
            ptr::null(),
            true,
            true,
            true,
            1,
            Some(C2GJKCache {
                metric: 0.0,
                count: 0,
                iA: [0; 3],
                iB: [0; 3],
                div: 0.0,
            }),
        );
        gjk_same(&co, &ro, "E41..E43 radius branches");
        if f32_same(co.dist, 0.0) {
            if v_same(co.a, co.b) {
                collapsed += 1;
            } else {
                forced_zero += 1;
            }
        } else {
            shrunk += 1;
        }
    }
    println!("E41/E42 collapsed={collapsed} E43 forced_zero={forced_zero} shrunk={shrunk}");
    assert!(collapsed > 0, "the midpoint-collapse branch was never taken");
    assert!(shrunk > 0, "the radius-shrink branch was never taken");
}

/// ERRORS.md E43 — after shrinking by the radii the C re-tests
/// `a.x == b.x && a.y == b.y` and forces `dist` back to `0`.  Reaching that
/// line needs `dist > rA + rB` to hold while the shrink still lands both
/// witness points on the same float: `rA = FLT_MAX`, `rB = -FLT_MAX` gives
/// `rA + rB == 0` and saturates both points to `FLT_MAX`.
#[test]
fn e43_forced_zero_after_radius_shrink() {
    let (c, r): (FnGJK, FnGJK) = sym(b"c2GJK");
    let mut rng = Rng::new(0xE43);
    let mut reached = 0u32;
    for i in 0..512u32 {
        let sep = if i == 0 { 100.0 } else { rng.range(1.0, 1e6) };
        let y = if i == 0 { 0.0 } else { rng.range(-1e3, 1e3) };
        let a = ShapeBlob::circle(C2Circle {
            p: C2v { x: 0.0, y },
            r: FLT_MAX,
        });
        let b = ShapeBlob::circle(C2Circle {
            p: C2v { x: sep, y },
            r: -FLT_MAX,
        });
        let co = gjk_call(
            c, &a, C2_TYPE_CIRCLE, ptr::null(), &b, C2_TYPE_CIRCLE, ptr::null(),
            true, true, true, 1, None,
        );
        let ro = gjk_call(
            r, &a, C2_TYPE_CIRCLE, ptr::null(), &b, C2_TYPE_CIRCLE, ptr::null(),
            true, true, true, 1, None,
        );
        gjk_same(&co, &ro, &format!("E43 forced zero #{i}"));
        if f32_same(co.dist, 0.0) && v_same(co.a, co.b) && co.a.x.is_finite() {
            reached += 1;
        }
    }
    println!("E43: the `a == b => dist = 0` re-check was reached {reached}/512 times");
    assert!(
        reached > 0,
        "the E43 branch (dist forced to 0 after the shrink) was never reached"
    );
}

/// ERRORS.md E46 — the `while (iter < 20)` cap.  Both libraries carry the same
/// literal bound; this test asserts `*iterations` agrees bit-for-bit across a
/// wide randomized space and reports the largest value actually reachable.
#[test]
fn e46_iteration_counts_agree() {
    let (c, r): (FnGJK, FnGJK) = sym(b"c2GJK");
    let mut rng = Rng::new(0xE46B);
    let mut histogram = [0u32; 21];
    for i in 0..200_000u32 {
        let ta = ALL_TYPES[rng.below(3) as usize];
        let tb = ALL_TYPES[rng.below(3) as usize];
        let sa = match ta {
            C2_TYPE_CIRCLE => ShapeBlob::circle(C2Circle { p: rng.v_spicy(), r: rng.spicy() }),
            C2_TYPE_AABB => ShapeBlob::aabb(C2AABB { min: rng.v_spicy(), max: rng.v_spicy() }),
            _ => ShapeBlob::capsule(C2Capsule { a: rng.v_spicy(), b: rng.v_spicy(), r: rng.spicy() }),
        };
        let sb = match tb {
            C2_TYPE_CIRCLE => ShapeBlob::circle(C2Circle { p: rng.v_spicy(), r: rng.spicy() }),
            C2_TYPE_AABB => ShapeBlob::aabb(C2AABB { min: rng.v_spicy(), max: rng.v_spicy() }),
            _ => ShapeBlob::capsule(C2Capsule { a: rng.v_spicy(), b: rng.v_spicy(), r: rng.spicy() }),
        };
        let ax = rng.x_spicy();
        let bx = rng.x_spicy();
        let (ua, ub) = (rng.bool(), rng.bool());
        let axp = if ua { &ax as *const C2x } else { ptr::null() };
        let bxp = if ub { &bx as *const C2x } else { ptr::null() };
        let ur = if rng.bool() { 1 } else { 0 };
        let co = gjk_call(c, &sa, ta, axp, &sb, tb, bxp, true, true, true, ur, None);
        let ro = gjk_call(r, &sa, ta, axp, &sb, tb, bxp, true, true, true, ur, None);
        gjk_same(&co, &ro, &format!("E46 iteration agreement #{i}"));
        if (0..=20).contains(&co.it) {
            histogram[co.it as usize] += 1;
        }
    }
    println!("E46 iteration histogram: {histogram:?}");
    let max = (0..21).rev().find(|&k| histogram[k] > 0).unwrap();
    println!("E46 largest reachable iteration count = {max} (the C caps at 20)");
    assert!(histogram[0] > 0 && histogram[1] > 0 && histogram[2] > 0);
}

#[test]
fn e44_e45_nan_and_overflow_geometry() {
    let (c, r): (FnGJK, FnGJK) = sym(b"c2GJK");
    let mut rng = Rng::new(0xE44);
    for ta in ALL_TYPES {
        for tb in ALL_TYPES {
            for i in 0..192u32 {
                // E44: NaN coordinates.  E45: +-inf / FLT_MAX coordinates.
                let (sa, sb) = if i % 2 == 0 {
                    (
                        nan_shape(&mut rng, ta),
                        nan_shape(&mut rng, tb),
                    )
                } else {
                    (
                        extreme_shape(&mut rng, ta),
                        extreme_shape(&mut rng, tb),
                    )
                };
                for ur in [0i32, 1] {
                    let cache0 = Some(C2GJKCache {
                        metric: 0.0,
                        count: 0,
                        iA: [0; 3],
                        iB: [0; 3],
                        div: 0.0,
                    });
                    let co = gjk_call(
                        c,
                        &sa,
                        ta,
                        ptr::null(),
                        &sb,
                        tb,
                        ptr::null(),
                        true,
                        true,
                        true,
                        ur,
                        cache0,
                    );
                    let ro = gjk_call(
                        r,
                        &sa,
                        ta,
                        ptr::null(),
                        &sb,
                        tb,
                        ptr::null(),
                        true,
                        true,
                        true,
                        ur,
                        cache0,
                    );
                    gjk_same(
                        &co,
                        &ro,
                        &format!(
                            "E44/E45 {} vs {} ur={ur} #{i}",
                            type_name(ta),
                            type_name(tb)
                        ),
                    );
                }
            }
        }
    }
}

fn nan_shape(rng: &mut Rng, ty: u32) -> ShapeBlob {
    let nan = |rng: &mut Rng| {
        if rng.bool() {
            f32::from_bits(0x7FC0_0000 | rng.next_u32() & 0x7FFF)
        } else {
            rng.range(-50.0, 50.0)
        }
    };
    match ty {
        C2_TYPE_CIRCLE => ShapeBlob::circle(C2Circle {
            p: C2v {
                x: nan(rng),
                y: nan(rng),
            },
            r: nan(rng),
        }),
        C2_TYPE_AABB => ShapeBlob::aabb(C2AABB {
            min: C2v {
                x: nan(rng),
                y: nan(rng),
            },
            max: C2v {
                x: nan(rng),
                y: nan(rng),
            },
        }),
        _ => ShapeBlob::capsule(C2Capsule {
            a: C2v {
                x: nan(rng),
                y: nan(rng),
            },
            b: C2v {
                x: nan(rng),
                y: nan(rng),
            },
            r: nan(rng),
        }),
    }
}

fn extreme_shape(rng: &mut Rng, ty: u32) -> ShapeBlob {
    const BIG: [f32; 6] = [
        f32::INFINITY,
        f32::NEG_INFINITY,
        FLT_MAX,
        -FLT_MAX,
        1e30,
        -1e30,
    ];
    let big = |rng: &mut Rng| BIG[rng.below(BIG.len() as u32) as usize];
    match ty {
        C2_TYPE_CIRCLE => ShapeBlob::circle(C2Circle {
            p: C2v {
                x: big(rng),
                y: big(rng),
            },
            r: big(rng),
        }),
        C2_TYPE_AABB => ShapeBlob::aabb(C2AABB {
            min: C2v {
                x: big(rng),
                y: big(rng),
            },
            max: C2v {
                x: big(rng),
                y: big(rng),
            },
        }),
        _ => ShapeBlob::capsule(C2Capsule {
            a: C2v {
                x: big(rng),
                y: big(rng),
            },
            b: C2v {
                x: big(rng),
                y: big(rng),
            },
            r: big(rng),
        }),
    }
}

// ===========================================================================
// E46..E50 — loop exit conditions, with coverage assertions
// ===========================================================================

#[test]
fn e46_e50_loop_exit_conditions() {
    let (c, r): (FnGJK, FnGJK) = sym(b"c2GJK");
    let mut rng = Rng::new(0xE46);
    let mut hit_count = 0u32; // E50: s.count == 3 -> hit
    let mut zero_iter = 0u32; // early break on the first pass
    let mut multi_iter = 0u32;
    let mut max_iter = 0i32;
    for _ in 0..8192 {
        let ta = ALL_TYPES[rng.below(3) as usize];
        let tb = ALL_TYPES[rng.below(3) as usize];
        // Overlapping shapes drive `hit`; distant ones drive the other exits.
        let scale = match rng.below(3) {
            0 => 1.0f32,
            1 => 1e-6,
            _ => 1e6,
        };
        let sa = shape_scaled(&mut rng, ta, C2v { x: 0.0, y: 0.0 }, scale);
        let off = match rng.below(3) {
            0 => 0.0,
            1 => scale * 0.5,
            _ => scale * 20.0,
        };
        let sb = shape_scaled(&mut rng, tb, C2v { x: off, y: off * 0.5 }, scale);
        for ur in [0i32, 1] {
            let co = gjk_call(
                c,
                &sa,
                ta,
                ptr::null(),
                &sb,
                tb,
                ptr::null(),
                true,
                true,
                true,
                ur,
                Some(C2GJKCache {
                    metric: 0.0,
                    count: 0,
                    iA: [0; 3],
                    iB: [0; 3],
                    div: 0.0,
                }),
            );
            let ro = gjk_call(
                r,
                &sa,
                ta,
                ptr::null(),
                &sb,
                tb,
                ptr::null(),
                true,
                true,
                true,
                ur,
                Some(C2GJKCache {
                    metric: 0.0,
                    count: 0,
                    iA: [0; 3],
                    iB: [0; 3],
                    div: 0.0,
                }),
            );
            gjk_same(&co, &ro, "E46..E50 loop exits");
            if co.cache.count == 3 {
                hit_count += 1;
            }
            if co.it == 0 {
                zero_iter += 1;
            } else {
                multi_iter += 1;
            }
            max_iter = max_iter.max(co.it);
        }
    }
    println!(
        "E46..E50 coverage: hit(count==3)={hit_count} it==0:{zero_iter} it>0:{multi_iter} max_it={max_iter}"
    );
    assert!(hit_count > 0, "the `hit` (count == 3) exit was never reached");
    assert!(zero_iter > 0 && multi_iter > 0, "iteration counts not varied");
    assert!(max_iter >= 2, "GJK never ran more than one iteration");
}

fn shape_scaled(rng: &mut Rng, ty: u32, at: C2v, scale: f32) -> ShapeBlob {
    match ty {
        C2_TYPE_CIRCLE => ShapeBlob::circle(C2Circle {
            p: at,
            r: rng.range(0.0, scale),
        }),
        C2_TYPE_AABB => ShapeBlob::aabb(C2AABB {
            min: C2v {
                x: at.x - scale,
                y: at.y - scale,
            },
            max: C2v {
                x: at.x + scale,
                y: at.y + scale,
            },
        }),
        _ => ShapeBlob::capsule(C2Capsule {
            a: C2v {
                x: at.x - scale,
                y: at.y,
            },
            b: C2v {
                x: at.x + scale,
                y: at.y + scale * 0.25,
            },
            r: rng.range(0.0, scale * 0.5),
        }),
    }
}

// ===========================================================================
// E51..E53, E58..E60 — invalid c2GJKCache contents (value-reproducible rows)
// ===========================================================================

#[test]
fn e51_e53_e58_e60_cache_contents() {
    let (c, r): (FnGJK, FnGJK) = sym(b"c2GJK");
    let mut rng = Rng::new(0xE51);
    let mut accepted = 0u32;
    let mut rejected = 0u32;
    for _ in 0..1024 {
        let bb = ShapeBlob::aabb(rand_aabb(&mut rng, 80.0));
        let cap = ShapeBlob::capsule(rand_capsule(&mut rng, 80.0));
        let metrics = [
            0.0f32,
            -0.0,
            1.0,
            -1.0,
            f32::NAN,
            -f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
            -1.0e8,
            -1.0e8 - 1.0,
            -1.0e9,
            -FLT_MAX,
            FLT_MAX,
            rng.range(-1e10, 1e10),
        ];
        let counts = [0i32, 1, 2, 3, -1, -2, i32::MIN];
        let divs = [1.0f32, 0.0, -0.0, f32::NAN, f32::INFINITY, 2.5, -3.5];
        for &metric in &metrics {
            for &count in &counts {
                for &div in &divs {
                    let cache0 = C2GJKCache {
                        metric,
                        count,
                        // valid indices for AABB (4 verts) / capsule (2 verts)
                        iA: [0, 1, 2],
                        iB: [0, 1, 0],
                        div,
                    };
                    let co = gjk_call(
                        c,
                        &bb,
                        C2_TYPE_AABB,
                        ptr::null(),
                        &cap,
                        C2_TYPE_CAPSULE,
                        ptr::null(),
                        true,
                        true,
                        true,
                        1,
                        Some(cache0),
                    );
                    let ro = gjk_call(
                        r,
                        &bb,
                        C2_TYPE_AABB,
                        ptr::null(),
                        &cap,
                        C2_TYPE_CAPSULE,
                        ptr::null(),
                        true,
                        true,
                        true,
                        1,
                        Some(cache0),
                    );
                    gjk_same(
                        &co,
                        &ro,
                        &format!(
                            "cache metric={} count={count} div={}",
                            fmt_f32(metric),
                            fmt_f32(div)
                        ),
                    );
                    if count != 0 {
                        // `iterations == 0` with a replayed 3-simplex means the
                        // cache was used; a cold restart runs at least one step
                        // from vertex 0.  Just tally the two outcomes.
                        if co.it == 0 {
                            accepted += 1;
                        } else {
                            rejected += 1;
                        }
                    }
                }
            }
        }
    }
    println!("E52/E59 cache accept/reject tally: it==0:{accepted} it>0:{rejected}");
    assert!(accepted + rejected > 0);
}

#[test]
fn e59_cache_rejected_by_huge_negative_metric() {
    // The only way to make `min_metric < max_metric * 2.0f && metric < -1.0e8f`
    // true (and therefore leave `cache_was_read == 0`) is a hugely negative
    // metric.  Reaching it needs metric_old and metric both very negative.
    let (c, r): (FnGJK, FnGJK) = sym(b"c2GJK");
    let mut rng = Rng::new(0xE59);
    let mut restarted = 0u32;
    for _ in 0..2048 {
        // A 3-vertex cache over a huge AABB gives `c2Det2` a hugely negative
        // determinant, i.e. metric < -1e8.
        let s = rng.range(1e5, 1e6);
        let bb = ShapeBlob::aabb(C2AABB {
            min: C2v { x: -s, y: -s },
            max: C2v { x: s, y: s },
        });
        let cap = ShapeBlob::capsule(C2Capsule {
            a: C2v { x: s * 2.0, y: 0.0 },
            b: C2v { x: s * 3.0, y: s },
            r: 1.0,
        });
        for (ia, ib) in [([0, 1, 2], [0, 1, 0]), ([2, 1, 0], [1, 0, 1])] {
            for metric in [-1.0e9f32, -1.0e12, -FLT_MAX, -1.0e8 - 1.0] {
                let cache0 = C2GJKCache {
                    metric,
                    count: 3,
                    iA: ia,
                    iB: ib,
                    div: 1.0,
                };
                let co = gjk_call(
                    c,
                    &bb,
                    C2_TYPE_AABB,
                    ptr::null(),
                    &cap,
                    C2_TYPE_CAPSULE,
                    ptr::null(),
                    true,
                    true,
                    true,
                    1,
                    Some(cache0),
                );
                let ro = gjk_call(
                    r,
                    &bb,
                    C2_TYPE_AABB,
                    ptr::null(),
                    &cap,
                    C2_TYPE_CAPSULE,
                    ptr::null(),
                    true,
                    true,
                    true,
                    1,
                    Some(cache0),
                );
                gjk_same(&co, &ro, "E59 huge negative metric");
                if co.it > 0 {
                    restarted += 1;
                }
            }
        }
    }
    println!("E59: {restarted} calls restarted from a cold simplex");
}

// ===========================================================================
// E54..E57 — UB rows: cache count >= 5 / out-of-range vertex indices
// ===========================================================================

#[repr(C)]
#[derive(Copy, Clone)]
struct PaddedCache {
    cache: C2GJKCache,
    pad: [u32; 16],
}

#[test]
fn e54_cache_count_four_matches_exactly() {
    // count == 4 keeps every aliased access inside the 36-byte struct, so this
    // row IS value-reproducible and is asserted as such.
    let (c, r): (FnGJK, FnGJK) = sym(b"c2GJK");
    let mut rng = Rng::new(0xE54);
    for _ in 0..1024 {
        let bb = ShapeBlob::aabb(rand_aabb(&mut rng, 80.0));
        let cap = ShapeBlob::capsule(rand_capsule(&mut rng, 80.0));
        for div_bits in [0u32, 1] {
            let base = PaddedCache {
                cache: C2GJKCache {
                    metric: rng.range(-5.0, 5.0),
                    count: 4,
                    iA: [0, 1, 2],
                    iB: [0, 1, 0],
                    div: f32::from_bits(div_bits),
                },
                pad: [0x1111_1111; 16],
            };
            let mut cc = base;
            let mut rc = base;
            let (mut ca, mut cb, mut ci) = (POISON_A, POISON_B, POISON_IT);
            let (mut ra, mut rb, mut ri) = (POISON_A, POISON_B, POISON_IT);
            let dc = unsafe {
                c(
                    bb.as_ptr(),
                    C2_TYPE_AABB,
                    ptr::null(),
                    cap.as_ptr(),
                    C2_TYPE_CAPSULE,
                    ptr::null(),
                    &mut ca,
                    &mut cb,
                    1,
                    &mut ci,
                    &mut cc.cache,
                )
            };
            let dr = unsafe {
                r(
                    bb.as_ptr(),
                    C2_TYPE_AABB,
                    ptr::null(),
                    cap.as_ptr(),
                    C2_TYPE_CAPSULE,
                    ptr::null(),
                    &mut ra,
                    &mut rb,
                    1,
                    &mut ri,
                    &mut rc.cache,
                )
            };
            assert_f32(dc, dr, "E54 return");
            assert_v(ca, ra, "E54 outA");
            assert_v(cb, rb, "E54 outB");
            assert_eq!(ci, ri, "E54 iterations");
            assert_raw(&cc, &rc, "E54 cache image (incl. padding)");
        }
    }
}



// ===========================================================================
// E61..E67 — c2Support boundaries
// ===========================================================================

#[test]
fn e61_e67_support_boundaries() {
    let (c, r): (FnSupport, FnSupport) = sym(b"c2Support");
    let mut rng = Rng::new(0xE61);
    for _ in 0..1024 {
        let mut verts = [C2v::default(); 8];
        for v in verts.iter_mut() {
            *v = rng.v_range(-100.0, 100.0);
        }
        let d = rng.v_range(-1.0, 1.0);
        // E61/E62/E63: count 0, 1 and negative -> always index 0
        for count in [0i32, 1, -1, -100, i32::MIN] {
            let cc = unsafe { c(verts.as_ptr(), count, d) };
            let rr = unsafe { r(verts.as_ptr(), count, d) };
            assert_eq!(cc, rr, "c2Support count={count}");
            assert_eq!(cc, 0, "count={count} must yield index 0");
        }
        // E64: d == {0,0} -> ties -> index 0
        for z in [
            C2v { x: 0.0, y: 0.0 },
            C2v { x: -0.0, y: -0.0 },
            C2v { x: 0.0, y: -0.0 },
        ] {
            let cc = unsafe { c(verts.as_ptr(), 8, z) };
            let rr = unsafe { r(verts.as_ptr(), 8, z) };
            assert_eq!(cc, rr, "c2Support d=0");
            assert_eq!(cc, 0, "all dots are +-0, so index 0 wins the tie");
        }
        // E65: NaN vertices never displace imax
        let mut nv = verts;
        nv[3] = C2v {
            x: f32::NAN,
            y: f32::NAN,
        };
        nv[5] = C2v {
            x: f32::NAN,
            y: 1.0,
        };
        let cc = unsafe { c(nv.as_ptr(), 8, d) };
        let rr = unsafe { r(nv.as_ptr(), 8, d) };
        assert_eq!(cc, rr, "c2Support NaN verts");
        assert!(cc != 3 && cc != 5, "a NaN dot must never become the max");
        // E66: verts[0] NaN -> dmax is NaN -> nothing is > NaN -> 0
        let mut n0 = verts;
        n0[0] = C2v {
            x: f32::NAN,
            y: 0.0,
        };
        let cc = unsafe { c(n0.as_ptr(), 8, d) };
        let rr = unsafe { r(n0.as_ptr(), 8, d) };
        assert_eq!(cc, rr, "c2Support NaN verts[0]");
        assert_eq!(cc, 0, "dmax == NaN means index 0 is returned");
        // E67: `count` larger than the caller "meant" but with all 8 verts
        // initialised, so it IS value-comparable.
        for count in [2i32, 4, 8] {
            assert_eq!(
                unsafe { c(verts.as_ptr(), count, d) },
                unsafe { r(verts.as_ptr(), count, d) },
                "c2Support oversized count={count}"
            );
        }
    }
}

// ===========================================================================
// E68..E70 — c2BBVerts has no validation
// ===========================================================================

#[test]
fn e68_e70_bbverts_no_validation() {
    let (c, r): (FnBBVerts, FnBBVerts) = sym(b"c2BBVerts");
    let mut rng = Rng::new(0xE68);
    for _ in 0..1024 {
        let good = rand_aabb(&mut rng, 200.0);
        let boxes = [
            C2AABB {
                min: good.max,
                max: good.min,
            }, // E68 inverted
            C2AABB {
                min: good.min,
                max: good.min,
            }, // E69 empty
            C2AABB {
                min: C2v {
                    x: f32::NAN,
                    y: f32::INFINITY,
                },
                max: C2v {
                    x: f32::NEG_INFINITY,
                    y: -f32::NAN,
                },
            }, // E70
            C2AABB {
                min: rng.v_spicy(),
                max: rng.v_spicy(),
            },
        ];
        for mut bb in boxes {
            let expect_bits = [
                bb.min,
                C2v { x: bb.max.x, y: bb.min.y },
                bb.max,
                C2v { x: bb.min.x, y: bb.max.y },
            ];
            let mut oc = [C2v { x: 99.0, y: -99.0 }; 4];
            let mut or_ = oc;
            unsafe { c(oc.as_mut_ptr(), &mut bb) };
            unsafe { r(or_.as_mut_ptr(), &mut bb) };
            assert_raw(&oc, &or_, "c2BBVerts");
            // The C copies the corner bit patterns verbatim, NaN payloads
            // included -- no clamping, no reordering.
            assert_raw(&expect_bits, &oc, "c2BBVerts corner order/bits");
        }
    }
}

// ===========================================================================
// E71..E77 — min / max / clamp NaN and signed-zero ordering
// ===========================================================================

#[test]
fn e71_e77_minmax_clamp_ordering() {
    let (cx, rx): (FnVvv, FnVvv) = sym(b"c2Maxv");
    let (cm, rm): (FnVvv, FnVvv) = sym(b"c2Minv");
    let (cc, rc): (FnVvvv, FnVvvv) = sym(b"c2Clampv");
    let nan = f32::NAN;
    let nnan = -f32::NAN;

    // E71: a.x NaN -> Maxv picks b.x
    let a = C2v { x: nan, y: 1.0 };
    let b = C2v { x: 2.0, y: 2.0 };
    let got = unsafe { cx(a, b) };
    assert_v(got, unsafe { rx(a, b) }, "E71");
    assert!(f32_same(got.x, 2.0), "E71: expected b.x, got {}", fmt_f32(got.x));

    // E72: b.x NaN -> Maxv still picks b.x (the NaN wins)
    let a = C2v { x: 2.0, y: 1.0 };
    let b = C2v { x: nan, y: 2.0 };
    let got = unsafe { cx(a, b) };
    assert_v(got, unsafe { rx(a, b) }, "E72");
    assert!(got.x.is_nan(), "E72: the NaN must win");

    // E73: b.x NaN -> Minv picks b.x
    let got = unsafe { cm(a, b) };
    assert_v(got, unsafe { rm(a, b) }, "E73");
    assert!(got.x.is_nan(), "E73: the NaN must win");

    // E74: a.x NaN -> Minv picks b.x
    let a = C2v { x: nan, y: 1.0 };
    let b = C2v { x: 2.0, y: 2.0 };
    let got = unsafe { cm(a, b) };
    assert_v(got, unsafe { rm(a, b) }, "E74");
    assert!(f32_same(got.x, 2.0), "E74: expected b.x");

    // E75: +0 vs -0 -> b's zero always wins, sign included
    for (az, bz) in [(0.0f32, -0.0f32), (-0.0, 0.0), (0.0, 0.0), (-0.0, -0.0)] {
        let a = C2v { x: az, y: az };
        let b = C2v { x: bz, y: bz };
        let gx = unsafe { cx(a, b) };
        let gm = unsafe { cm(a, b) };
        assert_v(gx, unsafe { rx(a, b) }, "E75 max");
        assert_v(gm, unsafe { rm(a, b) }, "E75 min");
        assert!(
            f32_same(gx.x, bz) && f32_same(gm.x, bz),
            "E75: expected b's zero (bits {:08x}), got max {:08x} min {:08x}",
            bz.to_bits(),
            gx.x.to_bits(),
            gm.x.to_bits()
        );
    }

    // E76/E77: inverted range and NaN in every slot
    let mut rng = Rng::new(0xE76);
    for _ in 0..2048 {
        let lo = rng.v_range(-50.0, 50.0);
        let hi = rng.v_range(-50.0, 50.0);
        let a = rng.v_range(-100.0, 100.0);
        // E76 -- deliberately inverted (lo > hi)
        let (l, h) = (
            C2v {
                x: lo.x.max(hi.x),
                y: lo.y.max(hi.y),
            },
            C2v {
                x: lo.x.min(hi.x),
                y: lo.y.min(hi.y),
            },
        );
        let got = unsafe { cc(a, l, h) };
        assert_v(got, unsafe { rc(a, l, h) }, "E76 inverted clamp");
        assert!(
            f32_same(got.x, l.x) || f32_same(got.x, h.x) || f32_same(got.x, a.x),
            "E76 unexpected clamp result"
        );
        // E77 -- NaN in each of the three arguments
        for (aa, ll, hh) in [
            (C2v { x: nan, y: a.y }, l, h),
            (a, C2v { x: nan, y: l.y }, h),
            (a, l, C2v { x: nan, y: h.y }),
            (
                C2v { x: nnan, y: nan },
                C2v { x: nan, y: nnan },
                C2v { x: nnan, y: nan },
            ),
        ] {
            assert_v(
                unsafe { cc(aa, ll, hh) },
                unsafe { rc(aa, ll, hh) },
                "E77 NaN clamp",
            );
        }
    }
}

// ===========================================================================
// E78..E83 — sign-of-NaN / signed-zero traps
// ===========================================================================

#[test]
fn e78_e80_rotation_sign_traps() {
    let (ct, rt): (FnMulrv, FnMulrv) = sym(b"c2MulrvT");
    let (cf, rf): (FnMulrv, FnMulrv) = sym(b"c2Mulrv");
    // Distinct NaN payloads so the destination-register choice is observable.
    let payloads = [
        0x7FC0_0001u32,
        0xFFC0_0002,
        0x7FC0_5555,
        0xFFCA_AAAA,
        0x7F80_0001,
    ];
    let plain = [0.0f32, -0.0, 1.0, -1.0, f32::INFINITY, f32::NEG_INFINITY, 2.0];
    let mut vals: Vec<f32> = payloads.iter().map(|&b| f32::from_bits(b)).collect();
    vals.extend_from_slice(&plain);
    for &rc in &vals {
        for &rs in &vals {
            for &bx in &vals {
                for &by in &vals {
                    let m = C2r { c: rc, s: rs };
                    let v = C2v { x: bx, y: by };
                    let gt = unsafe { ct(m, v) };
                    assert_v(
                        gt,
                        unsafe { rt(m, v) },
                        &format!(
                            "E78/E79 c2MulrvT rot=({},{}) v={}",
                            fmt_f32(rc),
                            fmt_f32(rs),
                            fmt_v(v)
                        ),
                    );
                    let gf = unsafe { cf(m, v) };
                    assert_v(
                        gf,
                        unsafe { rf(m, v) },
                        &format!(
                            "E80 c2Mulrv rot=({},{}) v={}",
                            fmt_f32(rc),
                            fmt_f32(rs),
                            fmt_v(v)
                        ),
                    );
                }
            }
        }
    }
    // E79 specifically: the `xorps` sign flip must happen on `a.s` BEFORE the
    // multiply, so `(-a.s) * b.x` is `(-0.0) * 0.0 == -0.0`.  Adding a second
    // `-0.0` keeps the result at `-0.0`; had the negation been folded into a
    // subtraction (`a.c*b.y - a.s*b.x`) the result would be `+0.0`.
    let m = C2r { c: 1.0, s: 0.0 };
    let v = C2v { x: 0.0, y: -0.0 };
    let got = unsafe { ct(m, v) };
    assert_v(got, unsafe { rt(m, v) }, "E79");
    assert!(
        f32_same(got.y, -0.0),
        "E79: expected -0.0 from (-0.0)*0.0 + 1.0*(-0.0), got {}",
        fmt_f32(got.y)
    );
    // and the plain all-`+0` case really does yield `+0.0` (-0.0 + +0.0)
    let m = C2r { c: 0.0, s: 0.0 };
    let v = C2v { x: 0.0, y: 0.0 };
    let got = unsafe { ct(m, v) };
    assert_v(got, unsafe { rt(m, v) }, "E79 all-zero");
    assert!(f32_same(got.y, 0.0), "expected +0.0, got {}", fmt_f32(got.y));
}

#[test]
fn e81_e83_unary_sign_helpers() {
    let (cn, rn): (FnVv, FnVv) = sym(b"c2Neg");
    let (ck, rk): (FnVv, FnVv) = sym(b"c2Skew");
    let (cw, rw): (FnVv, FnVv) = sym(b"c2CCW90");
    let specials = [
        0x0000_0000u32,
        0x8000_0000,
        0x7FC0_0001,
        0xFFC0_0001,
        0x7F80_0000,
        0xFF80_0000,
        0x7F80_0001,
        0x0000_0001,
        0x8000_0001,
        0x3F80_0000,
        0xBF80_0000,
    ];
    for &xb in &specials {
        for &yb in &specials {
            let a = C2v {
                x: f32::from_bits(xb),
                y: f32::from_bits(yb),
            };
            let gn = unsafe { cn(a) };
            assert_v(gn, unsafe { rn(a) }, "E81/E82 c2Neg");
            assert_eq!(
                gn.x.to_bits(),
                xb ^ 0x8000_0000,
                "c2Neg must flip exactly the sign bit"
            );
            assert_eq!(gn.y.to_bits(), yb ^ 0x8000_0000);
            let gk = unsafe { ck(a) };
            assert_v(gk, unsafe { rk(a) }, "E83 c2Skew");
            assert_eq!(gk.x.to_bits(), yb ^ 0x8000_0000);
            assert_eq!(gk.y.to_bits(), xb);
            let gw = unsafe { cw(a) };
            assert_v(gw, unsafe { rw(a) }, "E83 c2CCW90");
            assert_eq!(gw.x.to_bits(), yb);
            assert_eq!(gw.y.to_bits(), xb ^ 0x8000_0000);
        }
    }
}

// ===========================================================================
// E84..E87 — non-boolean scalar parameters
// ===========================================================================

#[test]
fn e84_use_radius_non_boolean() {
    let (c, r): (FnGJK, FnGJK) = sym(b"c2GJK");
    let mut rng = Rng::new(0xE84);
    for _ in 0..512 {
        let ta = ALL_TYPES[rng.below(3) as usize];
        let tb = ALL_TYPES[rng.below(3) as usize];
        let sa = rand_shape(&mut rng, ta, 60.0);
        let sb = rand_shape(&mut rng, tb, 60.0);
        let mut prev_nonzero: Option<(f32, C2v, C2v)> = None;
        for ur in [0i32, 1, -1, 2, 7, -7, i32::MIN, i32::MAX] {
            let co = gjk_call(
                c,
                &sa,
                ta,
                ptr::null(),
                &sb,
                tb,
                ptr::null(),
                true,
                true,
                true,
                ur,
                None,
            );
            let ro = gjk_call(
                r,
                &sa,
                ta,
                ptr::null(),
                &sb,
                tb,
                ptr::null(),
                true,
                true,
                true,
                ur,
                None,
            );
            gjk_same(&co, &ro, &format!("E84 use_radius={ur}"));
            if ur != 0 {
                // every non-zero value must behave identically to `1`
                if let Some((d, a, b)) = prev_nonzero {
                    assert!(
                        f32_same(d, co.dist) && v_same(a, co.a) && v_same(b, co.b),
                        "E84: use_radius={ur} differs from another non-zero value"
                    );
                }
                prev_nonzero = Some((co.dist, co.a, co.b));
            }
        }
    }
}

#[test]
fn e85_e87_gjk_cache_param_edges() {
    let (c, r): (FnGjkCache, FnGjkCache) = sym(b"gjk_cache");
    let mut rng = Rng::new(0xE85);
    let revs: [i8; 8] = [0, 1, -1, 2, 0x7F, -128, 0x10, -3];
    for _ in 0..256 {
        let x = rng.range(-100.0, 100.0);
        let y = rng.range(-100.0, 100.0);
        let cases: [[f32; 9]; 8] = [
            // E87 degenerate
            [x + 9.0, y + 9.0, x - 9.0, y - 9.0, x, y, x + 1.0, y, 2.0],
            [x, y, x, y, x, y, x, y, 0.0],
            [x, y, x + 1.0, y + 1.0, x, y, x, y, -5.0],
            // E86 extreme
            [
                f32::NAN,
                f32::INFINITY,
                f32::NEG_INFINITY,
                -f32::NAN,
                FLT_MAX,
                -FLT_MAX,
                FLT_MIN,
                1e-45,
                f32::NAN,
            ],
            [
                f32::INFINITY,
                f32::INFINITY,
                f32::INFINITY,
                f32::INFINITY,
                f32::INFINITY,
                f32::INFINITY,
                f32::INFINITY,
                f32::INFINITY,
                f32::INFINITY,
            ],
            [0.0, -0.0, 0.0, -0.0, 0.0, -0.0, 0.0, -0.0, -0.0],
            [FLT_MAX; 9],
            std::array::from_fn(|_| rng.spicy()),
        ];
        for p in cases {
            for &rev in &revs {
                let sa = C2v {
                    x: f32::from_bits(0x5555_5555),
                    y: f32::from_bits(0xAAAA_AAAA),
                };
                let sb = C2v {
                    x: f32::from_bits(0x3333_3333),
                    y: f32::from_bits(0xCCCC_CCCC),
                };
                let (mut ca, mut cb) = (sa, sb);
                let (mut ra, mut rb) = (sa, sb);
                unsafe {
                    c(
                        rev as core::ffi::c_char,
                        &mut ca,
                        &mut cb,
                        p[0],
                        p[1],
                        p[2],
                        p[3],
                        p[4],
                        p[5],
                        p[6],
                        p[7],
                        p[8],
                    )
                };
                unsafe {
                    r(
                        rev as core::ffi::c_char,
                        &mut ra,
                        &mut rb,
                        p[0],
                        p[1],
                        p[2],
                        p[3],
                        p[4],
                        p[5],
                        p[6],
                        p[7],
                        p[8],
                    )
                };
                assert!(
                    v_same(ca, sa) && v_same(cb, sb),
                    "C wrote a9/b9 for reverse={rev}"
                );
                assert!(
                    v_same(ra, sa) && v_same(rb, sb),
                    "Rust wrote a9/b9 for reverse={rev}"
                );
            }
        }
    }
}

// ===========================================================================
// E88..E93 — genuine null-pointer dereferences (out-of-process)
// ===========================================================================

/// Cases that must crash identically in both libraries, plus a few that must
/// NOT crash in either.
const CRASH_CASES: [&str; 19] = [
    "bbverts_null_out",
    "bbverts_null_bb",
    "makeproxy_null_p_circle",
    "makeproxy_null_p_aabb",
    "makeproxy_null_p_capsule",
    "makeproxy_null_shape_circle",
    "support_null_verts_count0",
    "support_null_verts_count1",
    "support_null_verts_count4",
    "support_null_verts_count_neg",
    "gjk_null_shape_a",
    "gjk_null_shape_b",
    "metric_null_s",
    "c22_null_s",
    "c23_null_s",
    "witness_null_s",
    "witness_null_out",
    "c2d_null_s",
    "c2l_null_s",
];

/// Cases that must complete normally in both libraries.
const NO_CRASH_CASES: [&str; 3] = [
    "makeproxy_null_p_invalid_type",
    "makeproxy_null_shape_invalid_type",
    "gjk_null_shapes_invalid_type",
];

fn run_case(lib: &str, case: &str) {
    let d = duo();
    let handle = if lib == "c" { &d.c } else { &d.r };
    macro_rules! g {
        ($name:literal, $ty:ty) => {{
            #[allow(unused_unsafe)]
            let s: libloading::Symbol<$ty> = unsafe { handle.get($name) }.unwrap();
            *s
        }};
    }
    let mut bb = C2AABB {
        min: C2v { x: 1.0, y: 2.0 },
        max: C2v { x: 3.0, y: 4.0 },
    };
    let mut out = [C2v::default(); 4];
    let circle = ShapeBlob::circle(C2Circle {
        p: C2v { x: 1.0, y: 1.0 },
        r: 2.0,
    });
    let mut proxy = C2Proxy::default();
    let mut simplex = C2Simplex::default();
    simplex.count = 2;
    simplex.div = 1.0;
    let mut wa = C2v::default();
    match case {
        "bbverts_null_out" => unsafe { g!(b"c2BBVerts", FnBBVerts)(ptr::null_mut(), &mut bb) },
        "bbverts_null_bb" => unsafe {
            g!(b"c2BBVerts", FnBBVerts)(out.as_mut_ptr(), ptr::null_mut())
        },
        "makeproxy_null_p_circle" => unsafe {
            g!(b"c2MakeProxy", FnMakeProxy)(circle.as_ptr(), C2_TYPE_CIRCLE, ptr::null_mut())
        },
        "makeproxy_null_p_aabb" => unsafe {
            g!(b"c2MakeProxy", FnMakeProxy)(circle.as_ptr(), C2_TYPE_AABB, ptr::null_mut())
        },
        "makeproxy_null_p_capsule" => unsafe {
            g!(b"c2MakeProxy", FnMakeProxy)(circle.as_ptr(), C2_TYPE_CAPSULE, ptr::null_mut())
        },
        "makeproxy_null_shape_circle" => unsafe {
            g!(b"c2MakeProxy", FnMakeProxy)(ptr::null(), C2_TYPE_CIRCLE, &mut proxy)
        },
        "makeproxy_null_p_invalid_type" => unsafe {
            g!(b"c2MakeProxy", FnMakeProxy)(circle.as_ptr(), 3, ptr::null_mut())
        },
        "makeproxy_null_shape_invalid_type" => unsafe {
            g!(b"c2MakeProxy", FnMakeProxy)(ptr::null(), 0xFFFF_FFFF, &mut proxy)
        },
        "support_null_verts_count0" => {
            let n = unsafe {
                g!(b"c2Support", FnSupport)(ptr::null(), 0, C2v { x: 1.0, y: 1.0 })
            };
            println!("support returned {n}");
        }
        "support_null_verts_count1" => {
            let n = unsafe {
                g!(b"c2Support", FnSupport)(ptr::null(), 1, C2v { x: 1.0, y: 1.0 })
            };
            println!("support returned {n}");
        }
        "support_null_verts_count_neg" => {
            let n = unsafe {
                g!(b"c2Support", FnSupport)(ptr::null(), -5, C2v { x: 1.0, y: 1.0 })
            };
            println!("support returned {n}");
        }
        "c2d_null_s" => {
            let v = unsafe { g!(b"c2D", FnVSimplex)(ptr::null_mut()) };
            println!("c2D returned {} {}", v.x, v.y);
        }
        "c2l_null_s" => {
            let v = unsafe { g!(b"c2L", FnVSimplex)(ptr::null_mut()) };
            println!("c2L returned {} {}", v.x, v.y);
        }
        "support_null_verts_count4" => {
            let n = unsafe {
                g!(b"c2Support", FnSupport)(ptr::null(), 4, C2v { x: 1.0, y: 1.0 })
            };
            println!("support returned {n}");
        }
        "gjk_null_shape_a" => {
            let d = unsafe {
                g!(b"c2GJK", FnGJK)(
                    ptr::null(),
                    C2_TYPE_CIRCLE,
                    ptr::null(),
                    circle.as_ptr(),
                    C2_TYPE_CIRCLE,
                    ptr::null(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    1,
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
            };
            println!("gjk returned {d}");
        }
        "gjk_null_shape_b" => {
            let d = unsafe {
                g!(b"c2GJK", FnGJK)(
                    circle.as_ptr(),
                    C2_TYPE_CIRCLE,
                    ptr::null(),
                    ptr::null(),
                    C2_TYPE_AABB,
                    ptr::null(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    1,
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
            };
            println!("gjk returned {d}");
        }
        "gjk_null_shapes_invalid_type" => {
            // With an out-of-range type `c2MakeProxy` never dereferences the
            // shape pointer, so passing NULL is safe -- but the C then reads an
            // uninitialised proxy, which may or may not fault.  Both libraries
            // must at least agree that the *shape* NULL alone is not the fault.
            let d = unsafe {
                g!(b"c2GJK", FnGJK)(
                    ptr::null(),
                    7,
                    ptr::null(),
                    ptr::null(),
                    7,
                    ptr::null(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    0,
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
            };
            println!("gjk returned {d}");
        }
        "metric_null_s" => {
            let v = unsafe { g!(b"c2GJKSimplexMetric", FnFSimplex)(ptr::null_mut()) };
            println!("metric returned {v}");
        }
        "c22_null_s" => unsafe { g!(b"c22", FnSimplex)(ptr::null_mut()) },
        "c23_null_s" => unsafe { g!(b"c23", FnSimplex)(ptr::null_mut()) },
        "witness_null_s" => unsafe {
            g!(b"c2Witness", FnWitness)(ptr::null_mut(), &mut wa, &mut wa)
        },
        "witness_null_out" => unsafe {
            g!(b"c2Witness", FnWitness)(&mut simplex, ptr::null_mut(), ptr::null_mut())
        },
        other => panic!("unknown crash case {other}"),
    }
}

fn spawn_in_child(test: &str, env_key: &str, lib: &str, case: &str) -> std::process::ExitStatus {
    let exe = std::env::current_exe().unwrap();
    std::process::Command::new(exe)
        .args([test, "--exact", "--nocapture", "--test-threads=1"])
        .env(env_key, case)
        .env("GJK_CHILD_LIB", lib)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("failed to spawn the child test process")
}

fn spawn_case(lib: &str, case: &str) -> std::process::ExitStatus {
    spawn_in_child(
        "e88_e93_null_pointer_dereferences",
        "GJK_CRASH_CASE",
        lib,
        case,
    )
}

#[test]
fn e88_e93_null_pointer_dereferences() {
    // Child mode: perform the null dereference and (if it survives) exit 0.
    if let Ok(case) = std::env::var("GJK_CRASH_CASE") {
        let lib = std::env::var("GJK_CHILD_LIB").unwrap();
        run_case(&lib, &case);
        std::process::exit(0);
    }

    for case in CRASH_CASES {
        let cs = spawn_case("c", case);
        let rs = spawn_case("rust", case);
        assert_eq!(
            cs.signal(),
            rs.signal(),
            "[{case}] fatal-signal mismatch: C {cs:?} Rust {rs:?}"
        );
        assert_eq!(
            cs.code(),
            rs.code(),
            "[{case}] exit-code mismatch: C {cs:?} Rust {rs:?}"
        );
        println!("[{case}] both: signal={:?} code={:?}", cs.signal(), cs.code());
    }

    for case in NO_CRASH_CASES {
        let cs = spawn_case("c", case);
        let rs = spawn_case("rust", case);
        assert_eq!(
            cs.signal(),
            None,
            "[{case}] the C crashed but was expected to survive"
        );
        assert_eq!(
            rs.signal(),
            None,
            "[{case}] the Rust crashed but the C survived"
        );
        assert_eq!(cs.code(), Some(0), "[{case}] the C child failed");
        assert_eq!(rs.code(), Some(0), "[{case}] the Rust child failed");
    }
}


// ===========================================================================
// E16/E17, E55, E56/E57 — UB rows, run OUT OF PROCESS
//
// For these inputs the C reads uninitialised stack (`c2Proxy pA;` for an
// out-of-range `C2_TYPE`, `c2Proxy::verts[k]` for an out-of-range cache index,
// and the bytes past `c2Simplex` for `cache->count >= 5`).  The C's behaviour is
// therefore not merely different from the Rust's -- it is not even stable from
// run to run, and it can fault outright (a garbage `pA.count` makes
// `c2Support` walk off the end of the vertex array).  Each case is consequently
// executed in a forked child so a C-side fault cannot take the test runner with
// it.  What is asserted: the RUST library, which is deterministic, always
// returns normally.  The C's measured outcome is printed and documented in
// ERRORS.md.
// ===========================================================================

const UB_CASES: [&str; 6] = [
    "ub_invalid_type_a",
    "ub_invalid_type_b",
    "ub_invalid_type_both",
    "ub_cache_count_ge5",
    "ub_cache_index_in_proxy",
    "ub_cache_index_past_proxy",
];

fn run_ub_case(lib: &str, case: &str) {
    let d = duo();
    let handle = if lib == "c" { &d.c } else { &d.r };
    let gjk: FnGJK = {
        #[allow(unused_unsafe)]
        let s: libloading::Symbol<FnGJK> = unsafe { handle.get(b"c2GJK") }.unwrap();
        *s
    };
    let mut rng = Rng::new(0xB1);
    let circle = ShapeBlob::circle(rand_circle(&mut rng, 40.0));
    let bb = ShapeBlob::aabb(rand_aabb(&mut rng, 80.0));
    let cap = ShapeBlob::capsule(rand_capsule(&mut rng, 80.0));

    let call = |ta: u32, tb: u32, sa: &ShapeBlob, sb: &ShapeBlob, cache: *mut C2GJKCache| {
        let mut a = POISON_A;
        let mut b = POISON_B;
        let mut it = POISON_IT;
        let dist = unsafe {
            gjk(
                sa.as_ptr(),
                ta,
                ptr::null(),
                sb.as_ptr(),
                tb,
                ptr::null(),
                &mut a,
                &mut b,
                1,
                &mut it,
                cache,
            )
        };
        println!(
            "  [{lib}] dist={} a={} b={} it={it}",
            fmt_f32(dist),
            fmt_v(a),
            fmt_v(b)
        );
    };

    match case {
        "ub_invalid_type_a" => {
            for &bad in &BAD_TYPES {
                call(bad, C2_TYPE_CIRCLE, &circle, &circle, ptr::null_mut());
            }
        }
        "ub_invalid_type_b" => {
            for &bad in &BAD_TYPES {
                call(C2_TYPE_CIRCLE, bad, &circle, &circle, ptr::null_mut());
            }
        }
        "ub_invalid_type_both" => {
            for &bad in &BAD_TYPES {
                call(bad, bad, &circle, &circle, ptr::null_mut());
            }
        }
        "ub_cache_count_ge5" => {
            for count in [5i32, 6, 7, 8] {
                let mut pc = PaddedCache {
                    cache: C2GJKCache {
                        metric: 1.5,
                        count,
                        iA: [0, 1, 1],
                        iB: [0, 1, 0],
                        div: f32::from_bits(1),
                    },
                    pad: [1u32; 16],
                };
                call(
                    C2_TYPE_AABB,
                    C2_TYPE_CAPSULE,
                    &bb,
                    &cap,
                    &mut pc.cache,
                );
                println!("  [{lib}] count={count} cache={} pad={:x?}", fmt_cache(&pc.cache), pc.pad);
            }
        }
        "ub_cache_index_in_proxy" => {
            for idx in [2i32, 3, 4, 5, 6, 7] {
                let mut cache = C2GJKCache {
                    metric: 1.5,
                    count: 1,
                    iA: [idx, 0, 0],
                    iB: [idx, 0, 0],
                    div: 1.0,
                };
                call(C2_TYPE_CIRCLE, C2_TYPE_CAPSULE, &circle, &cap, &mut cache);
                println!("  [{lib}] idx={idx} cache={}", fmt_cache(&cache));
            }
        }
        "ub_cache_index_past_proxy" => {
            for idx in [8i32, 9, -1, -2] {
                let mut cache = C2GJKCache {
                    metric: 1.5,
                    count: 1,
                    iA: [idx, 0, 0],
                    iB: [idx, 0, 0],
                    div: 1.0,
                };
                call(C2_TYPE_CIRCLE, C2_TYPE_CAPSULE, &circle, &cap, &mut cache);
                println!("  [{lib}] idx={idx} cache={}", fmt_cache(&cache));
            }
        }
        other => panic!("unknown UB case {other}"),
    }
}

#[test]
fn e16_e17_e55_e57_ub_cases_out_of_process() {
    if let Ok(case) = std::env::var("GJK_UB_CASE") {
        let lib = std::env::var("GJK_CHILD_LIB").unwrap();
        run_ub_case(&lib, &case);
        std::process::exit(0);
    }
    for case in UB_CASES {
        let cs = spawn_in_child(
            "e16_e17_e55_e57_ub_cases_out_of_process",
            "GJK_UB_CASE",
            "c",
            case,
        );
        let rs = spawn_in_child(
            "e16_e17_e55_e57_ub_cases_out_of_process",
            "GJK_UB_CASE",
            "rust",
            case,
        );
        println!(
            "[{case}] C: signal={:?} code={:?} | Rust: signal={:?} code={:?}",
            cs.signal(),
            cs.code(),
            rs.signal(),
            rs.code()
        );
        // The Rust translation is deterministic and must never fault here.
        assert_eq!(
            rs.signal(),
            None,
            "[{case}] the Rust library faulted on a UB input"
        );
        assert_eq!(rs.code(), Some(0), "[{case}] the Rust child failed");
    }
}
