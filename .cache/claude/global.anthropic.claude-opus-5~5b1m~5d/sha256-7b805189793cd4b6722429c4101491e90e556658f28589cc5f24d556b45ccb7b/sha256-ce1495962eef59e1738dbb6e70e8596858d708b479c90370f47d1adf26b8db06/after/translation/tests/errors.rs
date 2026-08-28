//! Phase C — one differential test per row of `ERRORS.md`.
//!
//! The C library has **no error channel at all** (no `RETURN_ERROR`, no
//! `assert`, no `return -1`, no error enum — verified by grep, see
//! `ERRORS.md`). Its rejection surface is therefore:
//!
//!   * explicit null-pointer guards          -> rows 1..7, 11, 49
//!   * `switch` statements with no matching  -> rows 8..21, 65
//!     `case` (out-of-range enum / count)
//!   * numeric guards and boundary constants -> rows 22..48
//!   * unchecked dereferences (UB/SIGSEGV)   -> rows 50..64
//!
//! Rows 50..64 are compared by re-executing this test binary as a child process
//! and asserting that BOTH libraries die from the SAME signal — "both failed
//! somehow" is not good enough.

mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::c_int;

const N: usize = 2_000;

/// Counts that are invalid for at least one of the simplex `switch`es, plus the
/// generic "one past the end" and extreme-int boundaries.
const BAD_COUNTS: [c_int; 8] = [0, -1, 4, 5, 100, i32::MIN, i32::MAX, -2];

/// Out-of-range `C2_TYPE` values. A C enum is just `int`, so these are all real
/// inputs across the FFI boundary.
const BAD_TYPES: [c_int; 9] = [3, 4, -1, -2, 100, i32::MIN, i32::MAX, 0x7fff_fffe, -0x8000_0000];

// ===========================================================================
// Rows 1..7 — the null-pointer guards c2GJK actually performs
// ===========================================================================

/// Row 1 — `ax_ptr == NULL` must behave exactly like passing `c2xIdentity()`.
#[test]
fn err_gjk_null_ax_is_identity() {
    let ident = c2x {
        p: c2v { x: 0.0, y: 0.0 },
        r: c2r { c: 1.0, s: 0.0 },
    };
    let mut rng = Rng::new(101);
    for i in 0..N {
        let ta = *pick(&ALL_TYPES, &mut rng);
        let tb = *pick(&ALL_TYPES, &mut rng);
        let a = Shape::random(&mut rng, ta);
        let b = Shape::random(&mut rng, tb);
        let (c, r) = both();

        let null_o = GjkOpts {
            ax: None,
            ..Default::default()
        };
        let ident_o = GjkOpts {
            ax: Some(ident),
            ..Default::default()
        };
        let cn = call_gjk(c, &a, &b, &null_o);
        let rn = call_gjk(r, &a, &b, &null_o);
        assert_gjk_eq(&format!("ax=NULL #{i}"), &cn, &rn);
        // ... and the C's own NULL result must equal its explicit-identity one.
        let ci = call_gjk(c, &a, &b, &ident_o);
        assert_gjk_eq(&format!("ax=NULL == ax=identity #{i}"), &cn, &ci);
        let ri = call_gjk(r, &a, &b, &ident_o);
        assert_gjk_eq(&format!("ax=identity #{i}"), &ci, &ri);
    }
}

/// Row 2 — same for `bx_ptr`.
#[test]
fn err_gjk_null_bx_is_identity() {
    let ident = c2x {
        p: c2v { x: 0.0, y: 0.0 },
        r: c2r { c: 1.0, s: 0.0 },
    };
    let mut rng = Rng::new(102);
    for i in 0..N {
        let a = Shape::any(&mut rng);
        let b = Shape::any(&mut rng);
        let (c, r) = both();
        let null_o = GjkOpts {
            bx: None,
            ..Default::default()
        };
        let ident_o = GjkOpts {
            bx: Some(ident),
            ..Default::default()
        };
        let cn = call_gjk(c, &a, &b, &null_o);
        let rn = call_gjk(r, &a, &b, &null_o);
        assert_gjk_eq(&format!("bx=NULL #{i}"), &cn, &rn);
        let ci = call_gjk(c, &a, &b, &ident_o);
        assert_gjk_eq(&format!("bx=NULL == bx=identity #{i}"), &cn, &ci);
        let ri = call_gjk(r, &a, &b, &ident_o);
        assert_gjk_eq(&format!("bx=identity #{i}"), &ci, &ri);
    }
}

/// Rows 3, 4 — `cache == NULL` skips both the read and the write-back, and must
/// give the same distance as a cold (`count == 0`) cache.
#[test]
fn err_gjk_null_cache() {
    let mut rng = Rng::new(103);
    for i in 0..N {
        let a = Shape::any(&mut rng);
        let b = Shape::any(&mut rng);
        let (c, r) = both();
        let o = GjkOpts {
            cache: None,
            ..Default::default()
        };
        let cn = call_gjk(c, &a, &b, &o);
        let rn = call_gjk(r, &a, &b, &o);
        assert_gjk_eq(&format!("cache=NULL #{i}"), &cn, &rn);
        // A NULL cache must not be written: our local copy stays all-zero.
        assert_bits_eq(
            &format!("cache=NULL not written #{i}"),
            &c2GJKCache::default(),
            &cn.cache,
        );
        assert_bits_eq(
            &format!("cache=NULL not written (rust) #{i}"),
            &c2GJKCache::default(),
            &rn.cache,
        );
    }
}

#[test]
fn err_gjk_cache_count_zero() {
    let mut rng = Rng::new(104);
    for i in 0..N {
        let a = Shape::any(&mut rng);
        let b = Shape::any(&mut rng);
        let (c, r) = both();
        // count == 0 -> cache_was_good == 0, so the garbage in metric/div/iA/iB
        // must be ignored entirely.
        let cold = c2GJKCache {
            metric: rng.any_f32(),
            count: 0,
            iA: [999, -7, 3],
            iB: [-1, 42, 8],
            div: rng.any_f32(),
        };
        let co = GjkOpts {
            cache: Some(cold),
            ..Default::default()
        };
        let cn = call_gjk(c, &a, &b, &co);
        let rn = call_gjk(r, &a, &b, &co);
        assert_gjk_eq(&format!("cache count=0 #{i}"), &cn, &rn);

        // The distance must match the cache==NULL call exactly.
        let nn = call_gjk(
            c,
            &a,
            &b,
            &GjkOpts {
                cache: None,
                ..Default::default()
            },
        );
        assert_f32_bits_eq(
            &format!("cache count=0 == cache NULL #{i}"),
            nn.dist,
            cn.dist,
        );
    }
}

/// Rows 5, 6, 7 — NULL `outA` / `outB` / `iterations`, in all 8 combinations.
#[test]
fn err_gjk_null_outputs() {
    let mut rng = Rng::new(105);
    for i in 0..N {
        let a = Shape::any(&mut rng);
        let b = Shape::any(&mut rng);
        let (c, r) = both();
        let full = call_gjk(c, &a, &b, &GjkOpts::default());
        for mask in 0..8u8 {
            let o = GjkOpts {
                want_a: mask & 1 != 0,
                want_b: mask & 2 != 0,
                want_iters: mask & 4 != 0,
                ..Default::default()
            };
            let cn = call_gjk(c, &a, &b, &o);
            let rn = call_gjk(r, &a, &b, &o);
            assert_gjk_eq(&format!("null outputs mask={mask} #{i}"), &cn, &rn);
            // A NULL pointer must leave the caller's buffer untouched...
            assert_eq_ctx(
                &format!("mask={mask} wrote_a #{i}"),
                o.want_a,
                cn.wrote_a,
            );
            assert_eq_ctx(
                &format!("mask={mask} wrote_b #{i}"),
                o.want_b,
                cn.wrote_b,
            );
            assert_eq_ctx(
                &format!("mask={mask} wrote_iters #{i}"),
                o.want_iters,
                cn.wrote_iters,
            );
            // ... and must not change the return value.
            assert_f32_bits_eq(
                &format!("mask={mask} dist unaffected #{i}"),
                full.dist,
                cn.dist,
            );
        }
    }
}

// ===========================================================================
// Rows 8..11 — c2MakeProxy with an out-of-range C2_TYPE
// ===========================================================================

/// Rows 8, 9, 10 — no `case` matches and there is no `default:`, so the whole
/// 72-byte `c2Proxy` must be left exactly as the caller had it.
#[test]
fn err_makeproxy_out_of_range_enum() {
    let (c, r) = both();
    let mut rng = Rng::new(108);
    for i in 0..N {
        // A real shape is supplied, so a stray `default:` arm would be visible.
        let circle = rng.circle();
        let aabb = rng.aabb();
        let capsule = rng.capsule();
        let shapes: [*const c_void; 3] = [
            &circle as *const _ as *const c_void,
            &aabb as *const _ as *const c_void,
            &capsule as *const _ as *const c_void,
        ];
        for &ty in &BAD_TYPES {
            for &sp in &shapes {
                let mut orig = c2Proxy::default();
                orig.radius = f32::from_bits(0x1234_5678);
                orig.count = -0x7f7f_7f7f;
                for (k, v) in orig.verts.iter_mut().enumerate() {
                    v.x = f32::from_bits(0xa000_0000 + k as u32);
                    v.y = f32::from_bits(0xb000_0000 + k as u32);
                }
                let mut cp = orig;
                let mut rp = orig;
                unsafe { (c.c2MakeProxy)(sp, ty, &mut cp) };
                unsafe { (r.c2MakeProxy)(sp, ty, &mut rp) };
                assert_bits_eq(&format!("c2MakeProxy ty={ty} #{i}"), &cp, &rp);
                // The strong claim: nothing at all was written.
                assert_bits_eq(
                    &format!("c2MakeProxy ty={ty} C left proxy untouched #{i}"),
                    &orig,
                    &cp,
                );
                assert_bits_eq(
                    &format!("c2MakeProxy ty={ty} Rust left proxy untouched #{i}"),
                    &orig,
                    &rp,
                );
            }
        }
    }
}

/// Row 11 — with an out-of-range type the `shape` pointer is never
/// dereferenced, so even NULL must not fault.
#[test]
fn err_makeproxy_null_shape_invalid_type() {
    let (c, r) = both();
    for &ty in &BAD_TYPES {
        let mut orig = c2Proxy::default();
        orig.radius = 42.5;
        orig.count = 7;
        let mut cp = orig;
        let mut rp = orig;
        unsafe { (c.c2MakeProxy)(std::ptr::null(), ty, &mut cp) };
        unsafe { (r.c2MakeProxy)(std::ptr::null(), ty, &mut rp) };
        assert_bits_eq(&format!("c2MakeProxy NULL shape ty={ty}"), &cp, &rp);
        assert_bits_eq(
            &format!("c2MakeProxy NULL shape ty={ty} untouched"),
            &orig,
            &cp,
        );
    }
}

// ===========================================================================
// Rows 12..21 — invalid `s->count` for each simplex switch
// ===========================================================================

/// Rows 12..15 — `default:` falls through into `case 1:` and returns `0.0f`.
#[test]
fn err_simplexmetric_bad_count() {
    let mut rng = Rng::new(112);
    for i in 0..N {
        let vs = [rng.sv(), rng.sv(), rng.sv(), rng.sv()];
        for &count in BAD_COUNTS.iter().chain([1i32].iter()) {
            let s = simplex(count, rng.coord(), &vs);
            let (cv, rv) = diff_simplex(
                &format!("c2GJKSimplexMetric count={count} #{i}"),
                &s,
                |a, p| unsafe { (a.c2GJKSimplexMetric)(p) },
            );
            assert_f32_bits_eq(
                &format!("c2GJKSimplexMetric count={count} #{i}"),
                cv,
                rv,
            );
            // The documented C result for every non-{2,3} count.
            assert_eq!(
                cv.to_bits(),
                0u32,
                "C must return +0.0f for count={count}, got {cv:?}"
            );
        }
    }
}

/// Rows 16, 17 — `case 3:` shares `default:`; both give `c2V(0,0)`.
#[test]
fn err_c2d_bad_count() {
    let mut rng = Rng::new(116);
    for i in 0..N {
        let vs = [rng.sv(), rng.sv(), rng.sv(), rng.sv()];
        for &count in BAD_COUNTS.iter().chain([3i32].iter()) {
            let s = simplex(count, rng.coord(), &vs);
            let (cv, rv) = diff_simplex(&format!("c2D count={count} #{i}"), &s, |a, p| unsafe {
                (a.c2D)(p)
            });
            assert_bits_eq(&format!("c2D count={count} #{i}"), &cv, &rv);
            assert_bits_eq(
                &format!("c2D count={count} must be (0,0) #{i}"),
                &c2v { x: 0.0, y: 0.0 },
                &cv,
            );
        }
    }
}

/// Rows 18, 19 — `cmp $3 / jg default` means count > 3 also lands in default.
/// `den = 1.0f/div` is still evaluated first, which is unobservable but must
/// not, for example, make the Rust panic on a zero divisor.
#[test]
fn err_witness_bad_count() {
    let (c, r) = both();
    let mut rng = Rng::new(118);
    for i in 0..N {
        let vs = [rng.sv(), rng.sv(), rng.sv(), rng.sv()];
        for &count in &BAD_COUNTS {
            for div in [1.0f32, 0.0, -0.0, f32::NAN, f32::INFINITY, rng.coord()] {
                let s = simplex(count, div, &vs);
                let mut cs = s;
                let mut rs = s;
                let mut ca = c2v { x: 11.0, y: 22.0 };
                let mut cb = c2v { x: 33.0, y: 44.0 };
                let mut ra = ca;
                let mut rb = cb;
                unsafe { (c.c2Witness)(&mut cs, &mut ca, &mut cb) };
                unsafe { (r.c2Witness)(&mut rs, &mut ra, &mut rb) };
                let ctx = format!("c2Witness count={count} div={div:?} #{i}");
                assert_bits_eq(&format!("{ctx} / a"), &ca, &ra);
                assert_bits_eq(&format!("{ctx} / b"), &cb, &rb);
                assert_bits_eq(&format!("{ctx} / simplex"), &cs, &rs);
                assert_bits_eq(&format!("{ctx} / a is (0,0)"), &c2v { x: 0.0, y: 0.0 }, &ca);
                assert_bits_eq(&format!("{ctx} / b is (0,0)"), &c2v { x: 0.0, y: 0.0 }, &cb);
            }
        }
    }
}

/// Row 20 — `c2L` has no `case 3:`, so 3 is invalid here even though it is
/// valid for `c2Witness`.
#[test]
fn err_c2l_bad_count() {
    let mut rng = Rng::new(120);
    for i in 0..N {
        let vs = [rng.sv(), rng.sv(), rng.sv(), rng.sv()];
        for &count in BAD_COUNTS.iter().chain([3i32].iter()) {
            for div in [1.0f32, 0.0, f32::NAN, rng.coord()] {
                let s = simplex(count, div, &vs);
                let (cv, rv) = diff_simplex(
                    &format!("c2L count={count} div={div:?} #{i}"),
                    &s,
                    |a, p| unsafe { (a.c2L)(p) },
                );
                assert_bits_eq(&format!("c2L count={count} div={div:?} #{i}"), &cv, &rv);
                assert_bits_eq(
                    &format!("c2L count={count} must be (0,0) #{i}"),
                    &c2v { x: 0.0, y: 0.0 },
                    &cv,
                );
            }
        }
    }
}

/// Rows 21, 65 — `cache->count == 4`, the largest value that stays inside both
/// structs: `cache->iA[3]` aliases `cache->iB[0]`, `cache->iB[3]` aliases the
/// bits of `cache->div` read as an `int`, and `verts + 3` is `s.d`. The loop
/// `switch` then has no matching `case`, so neither `c22` nor `c23` runs.
#[test]
fn err_gjk_cache_count_four() {
    let mut rng = Rng::new(121);
    for i in 0..N {
        let ta = *pick(&ALL_TYPES, &mut rng);
        let tb = *pick(&ALL_TYPES, &mut rng);
        let a = Shape::random(&mut rng, ta);
        let b = Shape::random(&mut rng, tb);
        let na = a.vert_count() as u32;
        let nb = b.vert_count() as u32;
        let (c, r) = both();

        // iB[3] IS cache.div reinterpreted as i32, so choose div's bit pattern
        // to be a small in-range index; that keeps every proxy read in bounds.
        let idx_b3 = rng.below(nb) as i32;
        let div_as_index = f32::from_bits(idx_b3 as u32);
        // iA[3] IS iB[0], so iB[0] must be a valid index for BOTH proxies.
        let shared = rng.below(na.min(nb)) as c_int;

        let cache = c2GJKCache {
            metric: *pick(&[0.0f32, -1.0e9, 1.0, -5.0], &mut rng),
            count: 4,
            iA: [
                rng.below(na) as c_int,
                rng.below(na) as c_int,
                rng.below(na) as c_int,
            ],
            iB: [shared, rng.below(nb) as c_int, rng.below(nb) as c_int],
            div: div_as_index,
        };
        let o = GjkOpts {
            cache: Some(cache),
            ..Default::default()
        };
        let cn = call_gjk(c, &a, &b, &o);
        let rn = call_gjk(r, &a, &b, &o);
        assert_gjk_eq(
            &format!("cache count=4 #{i} {cache:?} A={a:?} B={b:?}"),
            &cn,
            &rn,
        );
    }
}

// ===========================================================================
// Rows 22..29 — degenerate divisors (the C never guards `1.0f / x`)
// ===========================================================================

/// Rows 22, 23, 24 — `c2Div` computes `1.0f / b` unconditionally.
#[test]
fn err_div_by_zero() {
    let (c, r) = both();
    let mut rng = Rng::new(122);
    let divisors = [
        0.0f32,
        -0.0,
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7f80_0001), // sNaN
        f32::from_bits(0xff80_0001), // negative sNaN
        f32::from_bits(0x7fc0_dead), // qNaN with payload
        f32::from_bits(0xffc0_beef),
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(1), // smallest subnormal -> 1/x overflows
        -f32::from_bits(1),
        FLT_MAX,
    ];
    for i in 0..N {
        let vs = [
            rng.any_v(),
            c2v { x: 0.0, y: -0.0 },
            c2v {
                x: f32::INFINITY,
                y: f32::NEG_INFINITY,
            },
            c2v { x: 1.0, y: -1.0 },
            c2v {
                x: rng.nan(),
                y: rng.nan(),
            },
        ];
        for v in vs {
            for &d in &divisors {
                assert_bits_eq(
                    &format!("c2Div #{i} {v:?} / {d:?} (0x{:08x})", d.to_bits()),
                    &unsafe { (c.c2Div)(v, d) },
                    &unsafe { (r.c2Div)(v, d) },
                );
            }
        }
    }
}

/// Rows 25, 26 — `c2Norm` divides by `c2Len(a)`, which is 0 for the zero
/// vector and `inf` for an infinite one.
#[test]
fn err_norm_degenerate() {
    let (c, r) = both();
    let mut rng = Rng::new(125);
    let mut cases: Vec<c2v> = vec![
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: 0.0 },
        c2v { x: 0.0, y: -0.0 },
        c2v { x: -0.0, y: -0.0 },
        c2v {
            x: f32::INFINITY,
            y: 0.0,
        },
        c2v {
            x: f32::INFINITY,
            y: f32::INFINITY,
        },
        c2v {
            x: f32::NEG_INFINITY,
            y: f32::INFINITY,
        },
        c2v {
            x: f32::INFINITY,
            y: 1.0,
        },
        c2v { x: FLT_MAX, y: FLT_MAX },
        c2v {
            x: f32::from_bits(1),
            y: f32::from_bits(1),
        },
        c2v {
            x: f32::from_bits(1),
            y: 0.0,
        },
    ];
    for _ in 0..500 {
        cases.push(c2v {
            x: rng.nan(),
            y: rng.nan(),
        });
        cases.push(c2v { x: rng.nan(), y: 0.0 });
        cases.push(c2v { x: 0.0, y: rng.nan() });
    }
    for (i, v) in cases.iter().enumerate() {
        assert_bits_eq(
            &format!("c2Norm degenerate #{i} {v:?}"),
            &unsafe { (c.c2Norm)(*v) },
            &unsafe { (r.c2Norm)(*v) },
        );
    }
}

/// Rows 27, 28 — `c2Witness` computes `den = 1.0f / s->div` *before* the switch,
/// so a zero or NaN `div` poisons every weight in the valid counts too.
#[test]
fn err_witness_div_degenerate() {
    let (c, r) = both();
    let mut rng = Rng::new(127);
    let divs = [
        0.0f32,
        -0.0,
        f32::NAN,
        f32::from_bits(0x7f80_0001),
        f32::from_bits(0xffc0_1234),
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(1),
        FLT_MAX,
    ];
    for i in 0..N {
        for count in [1i32, 2, 3] {
            let mut vs = [rng.sv(), rng.sv(), rng.sv(), rng.sv()];
            // Weights that make 0*inf and inf*0 happen.
            if rng.bool() {
                vs[0].u = 0.0;
            }
            if rng.bool() {
                vs[1].u = f32::INFINITY;
            }
            for &div in &divs {
                let s = simplex(count, div, &vs);
                let mut cs = s;
                let mut rs = s;
                let mut ca = c2v { x: 7.0, y: 8.0 };
                let mut cb = c2v { x: 9.0, y: 10.0 };
                let mut ra = ca;
                let mut rb = cb;
                unsafe { (c.c2Witness)(&mut cs, &mut ca, &mut cb) };
                unsafe { (r.c2Witness)(&mut rs, &mut ra, &mut rb) };
                let ctx = format!("c2Witness div={div:?} count={count} #{i}");
                assert_bits_eq(&format!("{ctx} / a"), &ca, &ra);
                assert_bits_eq(&format!("{ctx} / b"), &cb, &rb);
                assert_bits_eq(&format!("{ctx} / simplex"), &cs, &rs);
            }
        }
    }
}

/// Row 29 — same `1.0f / s->div` degeneracy in `c2L`.
#[test]
fn err_c2l_div_degenerate() {
    let mut rng = Rng::new(129);
    let divs = [
        0.0f32,
        -0.0,
        f32::NAN,
        f32::from_bits(0x7f80_0001),
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(1),
        FLT_MAX,
    ];
    for i in 0..N {
        for count in [1i32, 2] {
            let mut vs = [rng.sv(), rng.sv(), rng.sv(), rng.sv()];
            if rng.bool() {
                vs[0].u = 0.0;
            }
            if rng.bool() {
                vs[1].u = f32::INFINITY;
            }
            for &div in &divs {
                let s = simplex(count, div, &vs);
                let (cv, rv) = diff_simplex(
                    &format!("c2L div={div:?} count={count} #{i}"),
                    &s,
                    |a, p| unsafe { (a.c2L)(p) },
                );
                assert_bits_eq(&format!("c2L div={div:?} count={count} #{i}"), &cv, &rv);
            }
        }
    }
}

// ===========================================================================
// Rows 30..32 — c2Support boundaries
// ===========================================================================

/// Rows 30, 31 — a non-positive `count` skips the loop entirely but `verts[0]`
/// is still dereferenced, and the function returns `0`.
#[test]
fn err_support_nonpositive_count() {
    let (c, r) = both();
    let mut rng = Rng::new(130);
    for i in 0..N {
        let mut verts = [c2v::default(); 8];
        for v in verts.iter_mut() {
            *v = rng.any_v();
        }
        let d = rng.any_v();
        for count in [0i32, -1, -2, -100, i32::MIN] {
            let cv = unsafe { (c.c2Support)(verts.as_ptr(), count, d) };
            let rv = unsafe { (r.c2Support)(verts.as_ptr(), count, d) };
            assert_eq_ctx(&format!("c2Support count={count} #{i}"), cv, rv);
            assert_eq!(cv, 0, "C must return 0 for count={count}");
        }
    }
}

/// Row 32 — when every dot product is NaN, `dot > dmax` is false for all of
/// them (unordered), so index 0 wins.
#[test]
fn err_support_all_nan() {
    let (c, r) = both();
    let mut rng = Rng::new(132);
    for i in 0..N {
        let mut verts = [c2v::default(); 8];
        for v in verts.iter_mut() {
            *v = rng.v();
        }
        for d in [
            c2v {
                x: rng.nan(),
                y: rng.nan(),
            },
            c2v { x: f32::NAN, y: 0.0 },
            c2v {
                x: f32::INFINITY,
                y: f32::NEG_INFINITY,
            },
        ] {
            for count in [1i32, 2, 4, 8] {
                let cv = unsafe { (c.c2Support)(verts.as_ptr(), count, d) };
                let rv = unsafe { (r.c2Support)(verts.as_ptr(), count, d) };
                assert_eq_ctx(&format!("c2Support NaN d #{i} count={count}"), cv, rv);
            }
        }
        // Also make the vertices NaN rather than the direction.
        let nverts = [c2v {
            x: rng.nan(),
            y: rng.nan(),
        }; 8];
        let d = rng.v();
        for count in [1i32, 2, 4, 8] {
            assert_eq_ctx(
                &format!("c2Support NaN verts #{i} count={count}"),
                unsafe { (c.c2Support)(nverts.as_ptr(), count, d) },
                unsafe { (r.c2Support)(nverts.as_ptr(), count, d) },
            );
        }
    }
}

// ===========================================================================
// Rows 33..35 — NaN vs the `<= 0` / `> 0` guards
// ===========================================================================

/// Row 33 — with `u`/`v` NaN, `v <= 0` and `u <= 0` are both false, so `c22`
/// falls all the way through to the `else` arm and sets `count = 2`.
#[test]
fn err_c22_nan_guards() {
    let mut rng = Rng::new(133);
    for i in 0..N {
        let mut vs = [rng.sv(), rng.sv(), rng.sv(), rng.sv()];
        // NaN in a.p and/or b.p makes u and v NaN.
        match i % 4 {
            0 => vs[0].p = c2v { x: rng.nan(), y: rng.nan() },
            1 => vs[1].p = c2v { x: rng.nan(), y: rng.nan() },
            2 => {
                vs[0].p = c2v { x: rng.nan(), y: rng.coord() };
                vs[1].p = c2v { x: rng.coord(), y: rng.nan() };
            }
            _ => {
                vs[0].p = c2v {
                    x: f32::INFINITY,
                    y: f32::NEG_INFINITY,
                };
                vs[1].p = c2v {
                    x: f32::INFINITY,
                    y: f32::INFINITY,
                };
            }
        }
        let s = simplex(2, rng.coord(), &vs);
        diff_simplex(&format!("c22 NaN guards #{i} case{}", i % 4), &s, |a, p| {
            unsafe { (a.c22)(p) }
        });
    }
}

/// Row 34 — all six guarded arms of `c23` fail for unordered compares, so the
/// final `else` (count = 3) runs.
#[test]
fn err_c23_nan_guards() {
    let mut rng = Rng::new(134);
    for i in 0..N {
        let mut vs = [rng.sv(), rng.sv(), rng.sv(), rng.sv()];
        let n = |rng: &mut Rng| c2v {
            x: rng.nan(),
            y: rng.nan(),
        };
        match i % 5 {
            0 => vs[0].p = n(&mut rng),
            1 => vs[1].p = n(&mut rng),
            2 => vs[2].p = n(&mut rng),
            3 => {
                vs[0].p = n(&mut rng);
                vs[1].p = n(&mut rng);
                vs[2].p = n(&mut rng);
            }
            _ => {
                vs[0].p = c2v {
                    x: f32::INFINITY,
                    y: 0.0,
                };
                vs[1].p = c2v {
                    x: f32::NEG_INFINITY,
                    y: 0.0,
                };
                vs[2].p = c2v {
                    x: 0.0,
                    y: f32::INFINITY,
                };
            }
        }
        let s = simplex(3, rng.coord(), &vs);
        diff_simplex(&format!("c23 NaN guards #{i} case{}", i % 5), &s, |a, p| {
            unsafe { (a.c23)(p) }
        });
    }
}

/// Row 35 — `c2Maxv`/`c2Minv`/`c2Clampv` are `comiss`+`jbe`, so an unordered
/// compare always takes the else-branch and returns `b`. That means
/// `c2Maxv(NaN, 1.0) == 1.0` but `c2Maxv(1.0, NaN) == NaN`, and `c2Clampv`
/// inverts its clamping semantics when `hi` is NaN.
#[test]
fn err_minmax_nan() {
    let (c, r) = both();
    let mut rng = Rng::new(135);
    for i in 0..N {
        let n1 = rng.nan();
        let n2 = rng.nan();
        let k = rng.coord();
        let pairs = [
            (c2v { x: n1, y: n1 }, c2v { x: k, y: k }),
            (c2v { x: k, y: k }, c2v { x: n1, y: n1 }),
            (c2v { x: n1, y: k }, c2v { x: k, y: n2 }),
            (c2v { x: n1, y: n2 }, c2v { x: n2, y: n1 }),
        ];
        for (a, b) in pairs {
            let cmax = unsafe { (c.c2Maxv)(a, b) };
            let rmax = unsafe { (r.c2Maxv)(a, b) };
            assert_bits_eq(&format!("c2Maxv NaN #{i}"), &cmax, &rmax);
            let cmin = unsafe { (c.c2Minv)(a, b) };
            let rmin = unsafe { (r.c2Minv)(a, b) };
            assert_bits_eq(&format!("c2Minv NaN #{i}"), &cmin, &rmin);
            // The documented quirk: with either operand NaN the result is `b`.
            if a.x.is_nan() || b.x.is_nan() {
                assert_eq!(
                    cmax.x.to_bits(),
                    b.x.to_bits(),
                    "c2Maxv must return b.x when unordered"
                );
                assert_eq!(
                    cmin.x.to_bits(),
                    b.x.to_bits(),
                    "c2Minv must return b.x when unordered"
                );
            }
        }
        // c2Clampv with NaN in each slot.
        for slot in 0..3 {
            let mut v = [rng.v(), rng.v(), rng.v()];
            v[slot] = c2v { x: n1, y: n2 };
            assert_bits_eq(
                &format!("c2Clampv NaN slot{slot} #{i}"),
                &unsafe { (c.c2Clampv)(v[0], v[1], v[2]) },
                &unsafe { (r.c2Clampv)(v[0], v[1], v[2]) },
            );
        }
    }
}

// ===========================================================================
// Rows 36, 37 — the cache-validity guard at lib.c:400
//
//   if (!(min_metric < max_metric * 2.0f && metric < -1.0e8f))
//           cache_was_read = 1;
//
// `metric` here is the FRESHLY COMPUTED simplex metric, not `cache->metric`.
// ===========================================================================

/// Row 36 — a NaN metric makes `min < 2*max` false, so the whole conjunction is
/// false and the cache is *accepted*.
#[test]
fn err_gjk_cache_nan_metric() {
    let mut rng = Rng::new(136);
    for i in 0..N {
        let a = Shape::any(&mut rng);
        let b = Shape::any(&mut rng);
        let na = a.vert_count() as u32;
        let nb = b.vert_count() as u32;
        let (c, r) = both();
        for metric in [
            f32::NAN,
            -f32::NAN,
            f32::from_bits(0x7f80_0001),
            f32::from_bits(0xffc0_5555),
            f32::INFINITY,
            f32::NEG_INFINITY,
        ] {
            for count in [1i32, 2, 3] {
                let mut cache = c2GJKCache {
                    metric,
                    count,
                    iA: [0; 3],
                    iB: [0; 3],
                    div: 1.0,
                };
                for k in 0..count as usize {
                    cache.iA[k] = (rng.below(na)) as c_int;
                    cache.iB[k] = (rng.below(nb)) as c_int;
                }
                let o = GjkOpts {
                    cache: Some(cache),
                    ..Default::default()
                };
                assert_gjk_eq(
                    &format!("cache NaN metric={metric:?} count={count} #{i}"),
                    &call_gjk(c, &a, &b, &o),
                    &call_gjk(r, &a, &b, &o),
                );
            }
        }
    }
}

/// Row 37 — the `metric < -1.0e8f` half of the guard at lib.c:400, driven to
/// the threshold **exactly**.
///
/// This is the only comparison in the library whose `<` vs `<=` distinction
/// needs an input landing precisely on a constant. Two things must hold at once
/// or the test is vacuous:
///
///  1. the **freshly computed** simplex metric (not `cache->metric`) must be
///     exactly `-1.0e8f`, and
///  2. the resulting value of `cache_was_read` must be **observable**. GJK is
///     self-correcting, so for many shape pairs a warm and a cold start
///     converge to the same witness points and the guard's outcome cannot be
///     seen at all.
///
/// Construction:
///   * `A` is a **circle**, so it has exactly one proxy vertex and `iA = [0,0,0]`
///     is always in range (no uninitialised proxy slot is ever read).
///   * `B` is `AABB{(bx,by), (bx+W, by+H)}`, whose proxy vertices are
///     `[(bx,by), (bx+W,by), (bx+W,by+H), (bx,by+H)]`.
///   * `iB = [0,1,3]` gives simplex points `p_k = B.verts[k] - A.p`, so
///     `p1-p0 = (W,0)` and `p2-p0 = (0,H)` and
///     `metric = c2Det2((W,0),(0,H)) = W*H`.
///   * **Every coordinate is an integer** of magnitude < 2^24, so all of those
///     subtractions are exact in `f32`. The metric is therefore exactly `W*H`
///     *regardless of where the two shapes sit*, which decouples requirement 1
///     from requirement 2 and lets the position be swept freely to satisfy 2.
///   * `W = 10000, H = -10000` gives exactly `-1.0e8f` (both factors and the
///     product are exactly representable). `W = 9999 / 10001` bracket it.
///   * `cache->metric = 0` keeps the other half of the conjunction
///     (`min_metric < 2*max_metric`) true, so nothing short-circuits.
#[test]
fn err_gjk_cache_metric_threshold() {
    let (c, r) = both();
    let threshold = -1.0e8f32;
    let mut rng = Rng::new(137);

    // W values bracketing and hitting the threshold exactly.
    let widths: [(f32, f32); 3] = [
        (9999.0, -10000.0),  // metric = -99_990_000  (> -1e8, guard false)
        (10000.0, -10000.0), // metric = -100_000_000 (== -1e8, the boundary)
        (10001.0, -10000.0), // metric = -100_010_000 (< -1e8, guard true)
    ];

    let mut saw_exact = false;
    let mut observable = 0usize;
    let mut checked = 0usize;

    for iter in 0..3000 {
        // Integral positions keep every subtraction exact.
        let ax = (rng.below(40001) as i32 - 20000) as f32;
        let ay = (rng.below(40001) as i32 - 20000) as f32;
        let bx = (rng.below(40001) as i32 - 20000) as f32;
        let by = (rng.below(40001) as i32 - 20000) as f32;

        for &(w, h) in &widths {
            let a = Shape::Circle(c2Circle {
                p: c2v { x: ax, y: ay },
                r: 0.0,
            });
            let b = Shape::Aabb(c2AABB {
                min: c2v { x: bx, y: by },
                max: c2v { x: bx + w, y: by + h },
            });

            // Recompute the metric the way c2GJK will, via the C's own exports.
            let p0 = c2v { x: bx - ax, y: by - ay };
            let p1 = c2v { x: (bx + w) - ax, y: by - ay };
            let p2 = c2v { x: bx - ax, y: (by + h) - ay };
            let metric = unsafe { (c.c2Det2)((c.c2Sub)(p1, p0), (c.c2Sub)(p2, p0)) };
            assert_eq!(
                metric.to_bits(),
                (w * h).to_bits(),
                "construction broken: metric {metric:?} != W*H {:?} for \
                 W={w} H={h} at ({ax},{ay})/({bx},{by})",
                w * h
            );
            if metric.to_bits() == threshold.to_bits() {
                saw_exact = true;
            }

            let warm = c2GJKCache {
                metric: 0.0,
                count: 3,
                iA: [0, 0, 0],
                iB: [0, 1, 3],
                div: 1.0,
            };
            for ur in [0, 1] {
                let o = GjkOpts {
                    cache: Some(warm),
                    use_radius: ur,
                    ..Default::default()
                };
                let cv = call_gjk(c, &a, &b, &o);
                let rv = call_gjk(r, &a, &b, &o);
                assert_gjk_eq(
                    &format!(
                        "cache metric threshold metric={metric:?} \
                         (0x{:08x}) W={w} H={h} A=({ax},{ay}) B=({bx},{by}) ur={ur}",
                        metric.to_bits()
                    ),
                    &cv,
                    &rv,
                );

                // Is `cache_was_read` observable here at all? Compare against
                // the rejected-cache behaviour (count == 0 takes the same
                // re-seed path). Only if these differ does this input actually
                // discriminate `<` from `<=`.
                if metric.to_bits() == threshold.to_bits() && ur == 1 {
                    let mut cold = warm;
                    cold.count = 0;
                    let dv = call_gjk(
                        c,
                        &a,
                        &b,
                        &GjkOpts {
                            cache: Some(cold),
                            use_radius: ur,
                            ..Default::default()
                        },
                    );
                    checked += 1;
                    if cv.dist.to_bits() != dv.dist.to_bits()
                        || bytes_of(&cv.a) != bytes_of(&dv.a)
                        || bytes_of(&cv.b) != bytes_of(&dv.b)
                        || cv.iters != dv.iters
                        || bytes_of(&cv.cache) != bytes_of(&dv.cache)
                    {
                        observable += 1;
                    }
                }
            }
        }
        if iter == 0 {
            println!("first sample: A=({ax},{ay}) B=({bx},{by})");
        }
    }

    println!(
        "metric-threshold: {observable}/{checked} exact--1e8 inputs make \
         cache_was_read observable"
    );
    assert!(
        saw_exact,
        "construction broken: no candidate produced a computed metric of \
         exactly -1.0e8f, so `<` vs `<=` at lib.c:400 would be untested"
    );
    assert!(
        observable > 0,
        "no input was found where the metric is exactly -1.0e8f AND the value \
         of cache_was_read is observable; row 37 would be vacuous"
    );
}

/// Row 37b — the OTHER half of the same conjunction, `min_metric <
/// max_metric * 2.0f`, at exact equality.
///
/// `min_metric == 2*max_metric` needs `metric == 2*metric_old` with
/// `metric < metric_old`, AND `metric < -1.0e8f` must hold so the second
/// conjunct does not mask the first. The same exact-integer construction as
/// `err_gjk_cache_metric_threshold` supplies it: `W = 20000, H = -10000` gives a
/// computed metric of exactly `-2.0e8f`, paired with `cache->metric = -1.0e8f`.
/// Then `min = -2e8`, `max = -1e8`, `2*max = -2e8` — exactly equal — so `<`
/// yields false (cache accepted) while `<=` would yield true (cache rejected).
#[test]
fn err_gjk_cache_metric_double_boundary() {
    let (c, r) = both();
    let mut rng = Rng::new(1372);

    // (W, H, cache->metric) triples with metric == 2 * metric_old exactly.
    let cases: [(f32, f32, f32); 4] = [
        (20000.0, -10000.0, -1.0e8),      // metric -2e8, 2*max == min exactly
        (20000.0, -20000.0, -2.0e8),      // metric -4e8, ditto
        (20000.0, -10000.0, -1.000_1e8),  // just off equality (control)
        (20000.0, -10000.0, -0.999_9e8),  // just off the other way (control)
    ];

    let mut observable = 0usize;
    let mut at_equality = 0usize;

    for _ in 0..1500 {
        let ax = (rng.below(40001) as i32 - 20000) as f32;
        let ay = (rng.below(40001) as i32 - 20000) as f32;
        let bx = (rng.below(40001) as i32 - 20000) as f32;
        let by = (rng.below(40001) as i32 - 20000) as f32;

        for &(w, h, old) in &cases {
            let a = Shape::Circle(c2Circle {
                p: c2v { x: ax, y: ay },
                r: 0.0,
            });
            let b = Shape::Aabb(c2AABB {
                min: c2v { x: bx, y: by },
                max: c2v { x: bx + w, y: by + h },
            });
            let p0 = c2v { x: bx - ax, y: by - ay };
            let p1 = c2v { x: (bx + w) - ax, y: by - ay };
            let p2 = c2v { x: bx - ax, y: (by + h) - ay };
            let metric = unsafe { (c.c2Det2)((c.c2Sub)(p1, p0), (c.c2Sub)(p2, p0)) };

            let min_m = if metric < old { metric } else { old };
            let max_m = if metric > old { metric } else { old };
            if min_m.to_bits() == (max_m + max_m).to_bits() && metric < -1.0e8 {
                at_equality += 1;
            }

            let warm = c2GJKCache {
                metric: old,
                count: 3,
                iA: [0, 0, 0],
                iB: [0, 1, 3],
                div: 1.0,
            };
            let o = GjkOpts {
                cache: Some(warm),
                ..Default::default()
            };
            let cv = call_gjk(c, &a, &b, &o);
            let rv = call_gjk(r, &a, &b, &o);
            assert_gjk_eq(
                &format!(
                    "min<2max boundary metric={metric:?} old={old:?} \
                     W={w} H={h} A=({ax},{ay}) B=({bx},{by})"
                ),
                &cv,
                &rv,
            );

            let mut cold = warm;
            cold.count = 0;
            let dv = call_gjk(
                c,
                &a,
                &b,
                &GjkOpts {
                    cache: Some(cold),
                    ..Default::default()
                },
            );
            if cv.dist.to_bits() != dv.dist.to_bits()
                || bytes_of(&cv.a) != bytes_of(&dv.a)
                || bytes_of(&cv.b) != bytes_of(&dv.b)
                || cv.iters != dv.iters
                || bytes_of(&cv.cache) != bytes_of(&dv.cache)
            {
                observable += 1;
            }
        }
    }
    println!(
        "min<2max boundary: {at_equality} inputs sat exactly on \
         min_metric == 2*max_metric with metric < -1e8; \
         cache_was_read observable in {observable} cases"
    );
    assert!(
        at_equality > 0,
        "no input reached min_metric == 2*max_metric exactly with metric < -1e8"
    );
    assert!(observable > 0, "cache_was_read was never observable");
}

// ===========================================================================
// Rows 40, 41, 43, 45 — the terminal numeric guards of c2GJK
// ===========================================================================

/// Row 40 — `0 < dist <= FLT_EPSILON` must take the midpoint arm even though
/// `dist > rA + rB` holds (both radii are 0).
#[test]
fn err_gjk_dist_below_epsilon() {
    let (c, r) = both();
    let gaps = [
        f32::from_bits(1),
        1.0e-30,
        1.0e-8,
        FLT_EPSILON * 0.5,
        FLT_EPSILON,             // exactly at the boundary -> `>` is false
        FLT_EPSILON * 1.000_001, // one hair above -> shrink arm
        1.0e-6,
    ];
    for &gap in &gaps {
        let a = Shape::Circle(c2Circle {
            p: c2v { x: 0.0, y: 0.0 },
            r: 0.0,
        });
        let b = Shape::Circle(c2Circle {
            p: c2v { x: gap, y: 0.0 },
            r: 0.0,
        });
        let with = GjkOpts::default();
        let without = GjkOpts {
            use_radius: 0,
            ..Default::default()
        };
        let cw = call_gjk(c, &a, &b, &with);
        let rw = call_gjk(r, &a, &b, &with);
        assert_gjk_eq(&format!("dist<=eps gap={gap:e} ur=1"), &cw, &rw);
        let cwo = call_gjk(c, &a, &b, &without);
        let rwo = call_gjk(r, &a, &b, &without);
        assert_gjk_eq(&format!("dist<=eps gap={gap:e} ur=0"), &cwo, &rwo);

        // With use_radius=0 the raw distance is `c2Len` of the centre offset.
        // (Note this is NOT always `gap`: for a subnormal gap, `gap*gap`
        // underflows to zero inside c2Dot, so c2Len returns exactly 0.)
        let raw = unsafe {
            (c.c2Len)((c.c2Sub)(
                c2v { x: 0.0, y: 0.0 },
                c2v { x: gap, y: 0.0 },
            ))
        };
        assert_eq!(
            cwo.dist.to_bits(),
            raw.to_bits(),
            "use_radius=0 must return the raw witness distance for gap={gap:e}"
        );
        // Both radii are 0, so `dist > rA + rB` is true whenever raw > 0; the
        // arm is therefore decided purely by the FLT_EPSILON test.
        if raw <= FLT_EPSILON {
            assert_eq!(
                cw.dist.to_bits(),
                0,
                "raw dist {raw:e} <= FLT_EPSILON must collapse dist to +0.0"
            );
        } else {
            assert_eq!(
                cw.dist.to_bits(),
                raw.to_bits(),
                "raw dist {raw:e} > FLT_EPSILON with zero radii must pass through"
            );
        }
    }
}

/// Row 41 — after the radius shift, `a == b` forces `dist` back to 0
/// (lib.c:486). The condition is searched for rather than assumed reachable:
/// the search recomputes the C's own arithmetic through the exported leaf
/// symbols, so a hit is a genuine hit.
#[test]
fn err_gjk_radius_collapse() {
    let (c, r) = both();
    let mut rng = Rng::new(141);
    let mut found = 0usize;
    let mut checked = 0usize;

    for _ in 0..200_000 {
        // Circle vs circle keeps the witness points equal to the two centres,
        // so the post-shift arithmetic is fully predictable.
        let a0 = c2v { x: 0.0, y: 0.0 };
        let b0 = c2v {
            x: rng.unit() * 100.0,
            y: rng.unit() * 100.0,
        };
        let dist = unsafe { (c.c2Len)((c.c2Sub)(a0, b0)) };
        if !(dist > FLT_EPSILON) {
            continue;
        }
        // Put rA + rB just below dist so the shifted points nearly coincide.
        let t = rng.unit().abs();
        let slack = dist * (f32::from_bits(rng.below(0x0080_0000) + 0x2000_0000));
        let r_a = dist * t;
        let r_b = dist - r_a - slack;
        if !(r_b > 0.0) {
            continue;
        }
        let rsum = r_a + r_b;
        if !(dist > rsum) {
            continue;
        }
        let n = unsafe { (c.c2Norm)((c.c2Sub)(b0, a0)) };
        let a1 = unsafe { (c.c2Add)(a0, (c.c2Mulvs)(n, r_a)) };
        let b1 = unsafe { (c.c2Sub)(b0, (c.c2Mulvs)(n, r_b)) };
        checked += 1;
        if !(a1.x == b1.x && a1.y == b1.y) {
            continue;
        }
        found += 1;

        let sa = Shape::Circle(c2Circle { p: a0, r: r_a });
        let sb = Shape::Circle(c2Circle { p: b0, r: r_b });
        let o = GjkOpts::default();
        let cv = call_gjk(c, &sa, &sb, &o);
        let rv = call_gjk(r, &sa, &sb, &o);
        assert_gjk_eq(
            &format!("radius collapse rA={r_a:?} rB={r_b:?} b0={b0:?}"),
            &cv,
            &rv,
        );
        // This is the arm's whole point: dist is forced to exactly +0.0 even
        // though dist > rA + rB was true.
        assert_eq!(
            cv.dist.to_bits(),
            0,
            "collapse arm must zero dist (dist={dist:?} rsum={rsum:?})"
        );
        if found >= 200 {
            break;
        }
    }
    println!("radius-collapse: {found} hits out of {checked} candidates");
    assert!(
        found > 0,
        "never reached the `a == b` collapse arm in 200k candidates \
         ({checked} passed the dist > rA+rB gate)"
    );
}

/// Rows 43, 45 — the `iter < 20` cap and the `c2Dot(d,d) < FLT_EPSILON*FLT_EPSILON`
/// break (the squared constant is exactly `2^-46`, `0x28800000`).
#[test]
fn err_gjk_iteration_cap() {
    let (c, r) = both();
    let mut rng = Rng::new(143);
    let mut max_iters = -1;

    // Coincident shapes: the search direction is exactly (0,0) on the very
    // first pass, so the FLT_EPSILON^2 test breaks with iter == 0.
    for i in 0..N {
        let p = rng.v();
        for s in [
            Shape::Circle(c2Circle { p, r: rng.radius() }),
            Shape::Aabb(c2AABB { min: p, max: p }),
            Shape::Capsule(c2Capsule { a: p, b: p, r: rng.radius() }),
        ] {
            let o = GjkOpts::default();
            let cv = call_gjk(c, &s, &s, &o);
            let rv = call_gjk(r, &s, &s, &o);
            assert_gjk_eq(&format!("eps2 break #{i}"), &cv, &rv);
            assert_eq!(
                cv.iters, 0,
                "coincident shapes must break on the first pass"
            );
        }
    }

    // The cap itself, over an adversarial mix of shapes and transforms.
    for i in 0..N * 4 {
        let a = Shape::any(&mut rng);
        let b = Shape::any(&mut rng);
        let o = GjkOpts {
            ax: if rng.bool() { Some(rng.xform()) } else { None },
            bx: if rng.bool() { Some(rng.xform()) } else { None },
            use_radius: rng.below(2) as c_int,
            ..Default::default()
        };
        let cv = call_gjk(c, &a, &b, &o);
        let rv = call_gjk(r, &a, &b, &o);
        assert_gjk_eq(&format!("iteration cap #{i}"), &cv, &rv);
        assert!(
            cv.iters >= 0 && cv.iters <= 20,
            "iterations {} outside [0,20] for {a:?} {b:?}",
            cv.iters
        );
        max_iters = max_iters.max(cv.iters);
    }
    println!("max iterations observed: {max_iters} (cap is 20)");
}

/// Rows 44, 73 — NaN / infinite shape coordinates. Every loop guard
/// (`d1 > d0`, `c2Dot(d,d) < eps^2`) is an ordered compare and so is false when
/// unordered, which changes the control flow rather than just the values.
///
/// The coordinates use DISTINCT NaN payloads per field: `c2Len(c2Sub(a,b))` and
/// `c2Len(c2Sub(b,a))` are bit-identical for every finite input, so only
/// differing payloads can pin the argument order of that call.
#[test]
fn err_gjk_nan_shape_coords() {
    let (c, r) = both();
    let mut rng = Rng::new(144);

    // Distinct, recognisable payloads so no two fields share a NaN.
    let payload = |k: u32, sign: u32| f32::from_bits((sign << 31) | 0x7f80_0000 | (k & 0x7f_ffff));

    for i in 0..N * 2 {
        let mut nxt = 0u32;
        let mut np = |quiet: bool, sign: u32| {
            nxt += 1;
            let base = if quiet { 0x0040_0000 } else { 0 };
            payload(base | (nxt * 7 + 1), sign)
        };
        let shapes: Vec<Shape> = vec![
            Shape::Aabb(c2AABB {
                min: c2v { x: np(true, 0), y: np(true, 1) },
                max: c2v { x: np(false, 0), y: np(false, 1) },
            }),
            Shape::Capsule(c2Capsule {
                a: c2v { x: np(true, 1), y: np(true, 0) },
                b: c2v { x: np(false, 1), y: np(false, 0) },
                r: np(true, 0),
            }),
            Shape::Circle(c2Circle {
                p: c2v { x: np(true, 0), y: np(false, 1) },
                r: np(true, 1),
            }),
            // Mixed: some coordinates NaN, some finite.
            Shape::Aabb(c2AABB {
                min: c2v { x: rng.coord(), y: np(true, 1) },
                max: c2v { x: np(false, 0), y: rng.coord() },
            }),
            Shape::Capsule(c2Capsule {
                a: c2v { x: f32::INFINITY, y: rng.coord() },
                b: c2v { x: f32::NEG_INFINITY, y: rng.coord() },
                r: rng.radius(),
            }),
            Shape::Circle(c2Circle {
                p: c2v { x: rng.coord(), y: rng.coord() },
                r: f32::INFINITY,
            }),
            // Finite control, so a divergence is attributable to the NaNs.
            Shape::Circle(c2Circle {
                p: c2v { x: rng.coord(), y: rng.coord() },
                r: rng.radius(),
            }),
        ];
        for (j, a) in shapes.iter().enumerate() {
            for (k, b) in shapes.iter().enumerate() {
                for ur in [0, 1] {
                    let o = GjkOpts {
                        use_radius: ur,
                        ..Default::default()
                    };
                    assert_gjk_eq(
                        &format!("NaN coords #{i} {j}x{k} ur={ur} A={a:?} B={b:?}"),
                        &call_gjk(c, a, b, &o),
                        &call_gjk(r, a, b, &o),
                    );
                }
            }
        }
    }
}

/// Row 49 — the wrapper forwards `a`/`b` straight into `c2GJK`'s `outA`/`outB`
/// guards, so NULL must be handled, not faulted on.
#[test]
fn err_gjk_wrapper_null_outputs() {
    let (c, r) = both();
    let mut rng = Rng::new(149);
    for i in 0..N {
        let f = [
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.radius(),
        ];
        for rev in [0i8, 1] {
            for mask in 0..4u8 {
                let sentinel = c2v { x: -3.5, y: 7.25 };
                let mut ca = sentinel;
                let mut cb = sentinel;
                let mut ra = sentinel;
                let mut rb = sentinel;
                let cap = if mask & 1 != 0 { &mut ca as *mut c2v } else { std::ptr::null_mut() };
                let cbp = if mask & 2 != 0 { &mut cb as *mut c2v } else { std::ptr::null_mut() };
                let rap = if mask & 1 != 0 { &mut ra as *mut c2v } else { std::ptr::null_mut() };
                let rbp = if mask & 2 != 0 { &mut rb as *mut c2v } else { std::ptr::null_mut() };
                unsafe {
                    (c.gjk)(rev, cap, cbp, f[0], f[1], f[2], f[3], f[4], f[5], f[6], f[7], f[8]);
                    (r.gjk)(rev, rap, rbp, f[0], f[1], f[2], f[3], f[4], f[5], f[6], f[7], f[8]);
                }
                let ctx = format!("gjk NULL out mask={mask} rev={rev} #{i}");
                assert_bits_eq(&format!("{ctx} / a"), &ca, &ra);
                assert_bits_eq(&format!("{ctx} / b"), &cb, &rb);
                if mask & 1 == 0 {
                    assert_bits_eq(&format!("{ctx} / a untouched"), &sentinel, &ca);
                }
                if mask & 2 == 0 {
                    assert_bits_eq(&format!("{ctx} / b untouched"), &sentinel, &cb);
                }
            }
        }
    }
}

// ===========================================================================
// Rows 50..64 — unchecked dereferences
//
// The C performs no null check in these functions, so the input is UB and both
// libraries fault. "Both crashed somehow" would be a worthless assertion, so
// each case is executed in a CHILD PROCESS and the two libraries are required
// to die from the SAME signal.
// ===========================================================================

/// One row of the table each. The name is passed to the child through the
/// environment.
const CRASH_CASES: &[&str] = &[
    "bbverts_out_null",             // row 50
    "bbverts_bb_null",              // row 51
    "makeproxy_shape_null_circle",  // row 52
    "makeproxy_shape_null_aabb",    // row 52
    "makeproxy_shape_null_capsule", // row 52
    "makeproxy_p_null_circle",      // row 53
    "makeproxy_p_null_aabb",        // row 53
    "makeproxy_p_null_capsule",     // row 53
    "simplexmetric_null",           // row 54
    "c22_null",                     // row 55
    "c23_null",                     // row 56
    "c2d_null",                     // row 57
    "c2l_null",                     // row 58
    "witness_s_null",               // row 59
    "witness_a_null",               // row 60
    "witness_b_null",               // row 61
    "support_null_count0",          // row 62
    "support_null_count4",          // row 62
    "gjk_a_null_circle",            // row 63
    "gjk_a_null_aabb",              // row 63
    "gjk_a_null_capsule",           // row 63
    "gjk_b_null_circle",            // row 64
    "gjk_b_null_aabb",              // row 64
];

fn perform_crash(api: &Api, case: &str) {
    let mut bb = c2AABB {
        min: c2v { x: 1.0, y: 2.0 },
        max: c2v { x: 3.0, y: 4.0 },
    };
    let mut out = [c2v::default(); 4];
    let circle = c2Circle {
        p: c2v { x: 1.0, y: 2.0 },
        r: 3.0,
    };
    let capsule = c2Capsule {
        a: c2v { x: 0.0, y: 0.0 },
        b: c2v { x: 1.0, y: 1.0 },
        r: 0.5,
    };
    let mut proxy = c2Proxy::default();
    let mut s = simplex(1, 1.0, &[c2sv::default(), c2sv::default()]);
    let mut va = c2v::default();
    let mut vb = c2v::default();
    let d = c2v { x: 1.0, y: 0.0 };
    let nullv: *mut c2v = std::ptr::null_mut();
    let nulls: *mut c2Simplex = std::ptr::null_mut();
    let cptr = &circle as *const _ as *const c_void;
    let capptr = &capsule as *const _ as *const c_void;
    let bbptr = &bb as *const _ as *const c_void;

    unsafe {
        match case {
            "bbverts_out_null" => (api.c2BBVerts)(nullv, &mut bb),
            "bbverts_bb_null" => (api.c2BBVerts)(out.as_mut_ptr(), std::ptr::null_mut()),
            "makeproxy_shape_null_circle" => {
                (api.c2MakeProxy)(std::ptr::null(), C2_TYPE_CIRCLE, &mut proxy)
            }
            "makeproxy_shape_null_aabb" => {
                (api.c2MakeProxy)(std::ptr::null(), C2_TYPE_AABB, &mut proxy)
            }
            "makeproxy_shape_null_capsule" => {
                (api.c2MakeProxy)(std::ptr::null(), C2_TYPE_CAPSULE, &mut proxy)
            }
            "makeproxy_p_null_circle" => {
                (api.c2MakeProxy)(cptr, C2_TYPE_CIRCLE, std::ptr::null_mut())
            }
            "makeproxy_p_null_aabb" => {
                (api.c2MakeProxy)(bbptr, C2_TYPE_AABB, std::ptr::null_mut())
            }
            "makeproxy_p_null_capsule" => {
                (api.c2MakeProxy)(capptr, C2_TYPE_CAPSULE, std::ptr::null_mut())
            }
            "simplexmetric_null" => {
                let _ = (api.c2GJKSimplexMetric)(nulls);
            }
            "c22_null" => (api.c22)(nulls),
            "c23_null" => (api.c23)(nulls),
            "c2d_null" => {
                let _ = (api.c2D)(nulls);
            }
            "c2l_null" => {
                let _ = (api.c2L)(nulls);
            }
            "witness_s_null" => (api.c2Witness)(nulls, &mut va, &mut vb),
            "witness_a_null" => (api.c2Witness)(&mut s, nullv, &mut vb),
            "witness_b_null" => (api.c2Witness)(&mut s, &mut va, nullv),
            "support_null_count0" => {
                let _ = (api.c2Support)(std::ptr::null(), 0, d);
            }
            "support_null_count4" => {
                let _ = (api.c2Support)(std::ptr::null(), 4, d);
            }
            "gjk_a_null_circle" => {
                let _ = (api.c2GJK)(
                    std::ptr::null(),
                    C2_TYPE_CIRCLE,
                    std::ptr::null(),
                    cptr,
                    C2_TYPE_CIRCLE,
                    std::ptr::null(),
                    &mut va,
                    &mut vb,
                    1,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
            }
            "gjk_a_null_aabb" => {
                let _ = (api.c2GJK)(
                    std::ptr::null(),
                    C2_TYPE_AABB,
                    std::ptr::null(),
                    cptr,
                    C2_TYPE_CIRCLE,
                    std::ptr::null(),
                    &mut va,
                    &mut vb,
                    1,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
            }
            "gjk_a_null_capsule" => {
                let _ = (api.c2GJK)(
                    std::ptr::null(),
                    C2_TYPE_CAPSULE,
                    std::ptr::null(),
                    cptr,
                    C2_TYPE_CIRCLE,
                    std::ptr::null(),
                    &mut va,
                    &mut vb,
                    1,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
            }
            "gjk_b_null_circle" => {
                let _ = (api.c2GJK)(
                    cptr,
                    C2_TYPE_CIRCLE,
                    std::ptr::null(),
                    std::ptr::null(),
                    C2_TYPE_CIRCLE,
                    std::ptr::null(),
                    &mut va,
                    &mut vb,
                    1,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
            }
            "gjk_b_null_aabb" => {
                let _ = (api.c2GJK)(
                    cptr,
                    C2_TYPE_CIRCLE,
                    std::ptr::null(),
                    std::ptr::null(),
                    C2_TYPE_AABB,
                    std::ptr::null(),
                    &mut va,
                    &mut vb,
                    1,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
            }
            other => panic!("unknown crash case {other}"),
        }
    }
    // Reached only if the call did NOT fault, which is itself a finding.
    println!("NO_FAULT {case}");
}

/// The child half. A no-op unless `GJK_CRASH_CASE` is set, so it costs nothing
/// during a normal run.
#[test]
fn null_deref_child() {
    let Ok(case) = std::env::var("GJK_CRASH_CASE") else {
        return;
    };
    let which = std::env::var("GJK_CRASH_LIB").unwrap_or_default();
    let api = match which.as_str() {
        "c" => c(),
        "rust" => r(),
        other => panic!("GJK_CRASH_LIB must be c|rust, got {other:?}"),
    };
    perform_crash(api, &case);
}

/// Rows 50..64 — the parent half: run each case against each library in its own
/// process and require identical termination.
#[test]
fn err_null_deref_signals() {
    use std::os::unix::process::ExitStatusExt;
    use std::process::{Command, Stdio};

    if std::env::var_os("GJK_CRASH_CASE").is_some() {
        return; // we are the child
    }

    let exe = std::env::current_exe().expect("current_exe");
    let c_so = c_so_path();
    let rust_so = rust_so_path();

    let run = |case: &str, lib: &str| -> (Option<i32>, Option<i32>) {
        let st = Command::new(&exe)
            .args([
                "--exact",
                "null_deref_child",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("GJK_CRASH_CASE", case)
            .env("GJK_CRASH_LIB", lib)
            // Resolve the libraries directly so the child never re-runs cargo.
            .env("GJK_C_SO", &c_so)
            .env("GJK_RUST_SO", &rust_so)
            .env("GJK_NO_BUILD", "1")
            .env("RUST_BACKTRACE", "0")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("spawning the child test process failed");
        (st.signal(), st.code())
    };

    let mut report = Vec::new();
    for case in CRASH_CASES {
        let (csig, ccode) = run(case, "c");
        let (rsig, rcode) = run(case, "rust");
        assert_eq!(
            csig, rsig,
            "row `{case}`: C died from signal {csig:?} but Rust from {rsig:?} \
             (exit codes {ccode:?} / {rcode:?}) — the two must reject this \
             input identically"
        );
        assert_eq!(
            ccode, rcode,
            "row `{case}`: C exited with code {ccode:?} but Rust with {rcode:?}"
        );
        assert!(
            csig.is_some(),
            "row `{case}`: expected a fatal signal (the C dereferences a null \
             pointer here) but the process exited normally with {ccode:?}"
        );
        // SIGSEGV (11) or SIGBUS (7) are the only sane outcomes.
        assert!(
            matches!(csig, Some(11) | Some(7)),
            "row `{case}`: unexpected signal {csig:?}, expected SIGSEGV/SIGBUS"
        );
        report.push(format!("{case}: signal {}", csig.unwrap()));
    }
    println!(
        "null-deref parity verified for {} rows:\n  {}",
        CRASH_CASES.len(),
        report.join("\n  ")
    );
}

// ===========================================================================
// Rows 66..68 — distinct NaN payloads in the simplex primitives
//
// Several sites inside the library multiply or add two values that can BOTH be
// NaN at the same time (`c2Det2(b,c) * area` in `c23`, `den * u` in `c2Witness`
// and `c2L`, `c2Sub(b.p, a.p)` in `c2GJKSimplexMetric`). An SSE arithmetic
// instruction returns the DESTINATION operand's NaN, so those sites are only
// pinned down when the two operands carry DIFFERENT payloads. Randomised NaNs
// hit that combination too rarely to rely on, so this drives every float field
// of the simplex with its own distinct, non-canonical payload.
// ===========================================================================

/// Give every field a unique NaN so that no two operands can accidentally
/// agree: payload `k`, alternating sign and alternating quiet/signalling.
fn distinct_nan(k: u32) -> f32 {
    let sign = (k & 1) << 31;
    let quiet = if k & 2 != 0 { 0x0040_0000 } else { 0 };
    let mut payload = (k.wrapping_mul(2_654_435_761)) & 0x003f_ffff;
    if payload == 0 && quiet == 0 {
        payload = 1; // a zero payload with quiet=0 would be infinity
    }
    f32::from_bits(sign | 0x7f80_0000 | quiet | payload)
}

/// Build a simplex whose float fields selected by `mask` are distinct NaNs and
/// whose remaining fields are ordinary finite values.
fn nan_simplex(count: c_int, seed: u32, mask: u32, rng: &mut Rng) -> c2Simplex {
    let mut s = c2Simplex::default();
    let mut k = seed;
    let mut next = |on: bool, rng: &mut Rng| -> f32 {
        k = k.wrapping_add(1);
        if on { distinct_nan(k) } else { rng.coord() }
    };
    for i in 0..4 {
        let on = mask & (1 << i) != 0;
        s.verts[i].sA = c2v {
            x: next(on, rng),
            y: next(on, rng),
        };
        s.verts[i].sB = c2v {
            x: next(on, rng),
            y: next(on, rng),
        };
        s.verts[i].p = c2v {
            x: next(on, rng),
            y: next(on, rng),
        };
        s.verts[i].u = next(mask & (1 << (i + 4)) != 0, rng);
        s.verts[i].iA = (i as c_int) % 4;
        s.verts[i].iB = (3 - i as c_int) % 4;
    }
    s.div = next(mask & (1 << 8) != 0, rng);
    s.count = count;
    s
}

/// Row 66 — `c2GJKSimplexMetric` with `count == 2` computes
/// `c2Len(c2Sub(b.p, a.p))`. `|v|` and `|-v|` are bit-identical for every finite
/// input, so only two differing NaN payloads can pin the argument order.
/// `count == 3` likewise pins `c2Det2`'s two `c2Sub` arguments.
#[test]
fn err_simplexmetric_distinct_nan_payloads() {
    let mut rng = Rng::new(166);
    for seed in 0..600u32 {
        for count in [1i32, 2, 3, 4] {
            for mask in [0b0000_0011u32, 0b0000_0111, 0b1_0000_1111, 0b0000_0101, 0x1ff] {
                let s = nan_simplex(count, seed * 97, mask, &mut rng);
                let (cv, rv) = diff_simplex(
                    &format!("metric distinct-NaN count={count} mask={mask:#b} seed={seed}"),
                    &s,
                    |a, p| unsafe { (a.c2GJKSimplexMetric)(p) },
                );
                assert_f32_bits_eq(
                    &format!(
                        "c2GJKSimplexMetric distinct-NaN count={count} \
                         mask={mask:#b} seed={seed} p0={:?} p1={:?} p2={:?}",
                        s.verts[0].p, s.verts[1].p, s.verts[2].p
                    ),
                    cv,
                    rv,
                );
            }
        }
    }
}

/// Row 67 — `c23`'s `uABC/vABC/wABC` are `c2Det2(..) * area`, and the final
/// `else` arm (the only one with no positivity guard on its operands) stores
/// them straight into the simplex, so both factors can be NaN at once there.
/// Also re-checks `c22`, whose `else` arm is likewise reachable with NaN `u`/`v`
/// because `x <= 0` is false when unordered.
#[test]
fn err_c22_c23_distinct_nan_payloads() {
    let mut rng = Rng::new(167);
    for seed in 0..800u32 {
        for mask in [
            0b0000_0011u32,
            0b0000_0111,
            0b0000_0101,
            0b0000_0110,
            0b1_0000_0111,
            0x1ff,
        ] {
            let s2 = nan_simplex(2, seed * 131, mask, &mut rng);
            diff_simplex(
                &format!("c22 distinct-NaN mask={mask:#b} seed={seed}"),
                &s2,
                |a, p| unsafe { (a.c22)(p) },
            );
            let s3 = nan_simplex(3, seed * 131 + 7, mask, &mut rng);
            diff_simplex(
                &format!("c23 distinct-NaN mask={mask:#b} seed={seed}"),
                &s3,
                |a, p| unsafe { (a.c23)(p) },
            );
        }
    }
}

/// Row 68 — `c2Witness` and `c2L` both compute `den * u` with
/// `den = 1.0f / s->div`. When `div` is NaN, `den` is NaN too, so a NaN weight
/// makes both operands of that `mulss` NaN simultaneously — the only way to pin
/// which one is the destination.
#[test]
fn err_witness_c2l_distinct_nan_payloads() {
    let (c, r) = both();
    let mut rng = Rng::new(168);
    for seed in 0..800u32 {
        // Force `div` (bit 8) AND the weights (bits 4..7) to be distinct NaNs.
        for mask in [0x1f0u32, 0x1ff, 0b1_0001_0000, 0b1_0011_0000, 0b1_0111_0000] {
            for count in [1i32, 2, 3, 4] {
                let s = nan_simplex(count, seed * 211, mask, &mut rng);
                // c2Witness
                let mut cs = s;
                let mut rs = s;
                let mut ca = c2v { x: 1.0, y: 2.0 };
                let mut cb = c2v { x: 3.0, y: 4.0 };
                let mut ra = ca;
                let mut rb = cb;
                unsafe { (c.c2Witness)(&mut cs, &mut ca, &mut cb) };
                unsafe { (r.c2Witness)(&mut rs, &mut ra, &mut rb) };
                let ctx = format!(
                    "c2Witness distinct-NaN count={count} mask={mask:#x} seed={seed} \
                     div={:?} u0={:?} u1={:?}",
                    s.div, s.verts[0].u, s.verts[1].u
                );
                assert_bits_eq(&format!("{ctx} / a"), &ca, &ra);
                assert_bits_eq(&format!("{ctx} / b"), &cb, &rb);
                assert_bits_eq(&format!("{ctx} / simplex"), &cs, &rs);

                // c2L
                let (cv, rv) = diff_simplex(
                    &format!("c2L distinct-NaN count={count} mask={mask:#x} seed={seed}"),
                    &s,
                    |a, p| unsafe { (a.c2L)(p) },
                );
                assert_bits_eq(
                    &format!("c2L distinct-NaN count={count} mask={mask:#x} seed={seed}"),
                    &cv,
                    &rv,
                );
            }
        }
    }
}
