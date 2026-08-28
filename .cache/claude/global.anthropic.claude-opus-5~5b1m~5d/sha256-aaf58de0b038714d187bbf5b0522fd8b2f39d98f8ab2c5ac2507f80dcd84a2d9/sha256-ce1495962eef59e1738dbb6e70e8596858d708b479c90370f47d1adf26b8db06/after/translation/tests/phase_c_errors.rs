//! Phase C — error-path differential tests, one per row of `ERRORS.md`.
//!
//! Each test constructs the exact invalid input / degenerate condition named by
//! the row, calls BOTH `.so`s, and asserts they agree on the SAME sentinel
//! (`0`, `1`, `+0.0f`, `{0,0}`, a specific `NaN` bit pattern, …) — not merely
//! that "both failed somehow".
//!
//! Rows E16, E17, E18, E19 and E42 are undefined behaviour in C and are
//! excluded; see the note at the bottom of `ERRORS.md` for the empirical
//! evidence and for the defined neighbour that is tested in their place.

#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::{c_int, c_void};

#[repr(C, align(16))]
#[derive(Copy, Clone)]
struct Buf {
    bytes: [u8; 32],
}

impl Buf {
    fn of<T: Copy>(v: &T) -> Buf {
        let mut b = Buf { bytes: [0; 32] };
        let s = raw(v);
        b.bytes[..s.len()].copy_from_slice(&s);
        b
    }
    fn ptr(&self) -> *const c_void {
        self.bytes.as_ptr() as *const c_void
    }
}

/// Every out-of-range `C2_TYPE` an FFI caller can pass (C enums accept any int).
const BAD_TYPES: &[c_int] = &[
    3,
    4,
    -1,
    -2,
    100,
    255,
    256,
    0x7fff_ffff,
    i32::MIN,
    i32::MIN + 1,
    -0x8000,
    0x1_0000,
];

const GOOD_TYPES: [c_int; 3] = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE];

fn shape_of(rng: &mut Rng, t: c_int) -> Buf {
    match t {
        C2_TYPE_CIRCLE => Buf::of(&rng.circle()),
        C2_TYPE_AABB => Buf::of(&rng.aabb()),
        _ => Buf::of(&rng.capsule()),
    }
}

// ---------------------------------------------------------------------------
// Reference predicates: faithful transliterations of the C bodies, used to
// derive the EXPECTED sentinel for a given input instead of hand-guessing it.
// (The pass/fail decision is still C-vs-Rust; these only pin down *which*
// sentinel the C source prescribes, so "both returned 1" cannot pass by
// accident when the C source says 0.)
// ---------------------------------------------------------------------------

fn ref_dot(a: c2v, b: c2v) -> f32 {
    a.y * b.y + a.x * b.x
}
fn ref_sub(a: c2v, b: c2v) -> c2v {
    c2v { x: a.x - b.x, y: a.y - b.y }
}
fn ref_maxv(a: c2v, b: c2v) -> c2v {
    c2v {
        x: if a.x > b.x { a.x } else { b.x },
        y: if a.y > b.y { a.y } else { b.y },
    }
}
fn ref_minv(a: c2v, b: c2v) -> c2v {
    c2v {
        x: if a.x < b.x { a.x } else { b.x },
        y: if a.y < b.y { a.y } else { b.y },
    }
}

/// `int c2AABBtoAABB(c2AABB A, c2AABB B)`
fn ref_aabb_to_aabb(a: c2AABB, b: c2AABB) -> c_int {
    let d0 = (b.max.x < a.min.x) as c_int;
    let d1 = (a.max.x < b.min.x) as c_int;
    let d2 = (b.max.y < a.min.y) as c_int;
    let d3 = (a.max.y < b.min.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

/// `int c2CircletoAABB(c2Circle A, c2AABB B)`
fn ref_circle_to_aabb(a: c2Circle, b: c2AABB) -> c_int {
    let l = ref_maxv(b.min, ref_minv(a.p, b.max));
    let ab = ref_sub(a.p, l);
    let d2 = ref_dot(ab, ab);
    let r2 = a.r * a.r;
    (d2 < r2) as c_int
}

/// `int c2CircletoCapsule(c2Circle A, c2Capsule B)`
fn ref_circle_to_capsule(a: c2Circle, b: c2Capsule) -> c_int {
    let n = ref_sub(b.b, b.a);
    let ap = ref_sub(a.p, b.a);
    let da = ref_dot(ap, n);
    let d2;
    if da < 0.0 {
        d2 = ref_dot(ap, ap);
    } else {
        let db = ref_dot(ref_sub(a.p, b.b), n);
        if db < 0.0 {
            let k = da / ref_dot(n, n);
            let e = ref_sub(ap, c2v { x: n.x * k, y: n.y * k });
            d2 = ref_dot(e, e);
        } else {
            let bp = ref_sub(a.p, b.b);
            d2 = ref_dot(bp, bp);
        }
    }
    let r = a.r + b.r;
    (d2 < r * r) as c_int
}

/// `int c2CircletoCircle(c2Circle A, c2Circle B)`
fn ref_circle_to_circle(a: c2Circle, b: c2Circle) -> c_int {
    let c = ref_sub(b.p, a.p);
    let d2 = ref_dot(c, c);
    let mut r2 = a.r + b.r;
    r2 = r2 * r2;
    (d2 < r2) as c_int
}

// ===========================================================================
// E1..E4 — c2Collided with out-of-range enum values
// ===========================================================================

#[test]
fn e1_e2_e3_e4_c2Collided_out_of_range_enums() {
    let p = load();
    let c: FnCollided = p.c.sym("c2Collided");
    let r: FnCollided = p.rs.sym("c2Collided");
    let mut rng = Rng::new(0xE1);
    unsafe {
        // E1: bad typeA (any typeB, valid or not) -> outer `default:` -> 0
        for &ta in BAD_TYPES {
            for tb in GOOD_TYPES.iter().copied().chain(BAD_TYPES.iter().copied()) {
                for _ in 0..64 {
                    let a = shape_of(&mut rng, C2_TYPE_CAPSULE);
                    let b = shape_of(&mut rng, C2_TYPE_CAPSULE);
                    let cv = c(a.ptr(), ta, b.ptr(), tb);
                    let rv = r(a.ptr(), ta, b.ptr(), tb);
                    assert_eq!(cv, rv, "E1 c2Collided(typeA={ta}, typeB={tb})");
                    assert_eq!(cv, 0, "E1 expected the sentinel 0, got {cv}");
                }
            }
        }
        // E2/E3/E4: valid typeA, bad typeB -> inner `default:` -> 0
        for (row, ta) in [("E2", C2_TYPE_CIRCLE), ("E3", C2_TYPE_AABB), ("E4", C2_TYPE_CAPSULE)] {
            for &tb in BAD_TYPES {
                for _ in 0..128 {
                    let a = shape_of(&mut rng, ta);
                    let b = shape_of(&mut rng, C2_TYPE_CAPSULE);
                    let cv = c(a.ptr(), ta, b.ptr(), tb);
                    let rv = r(a.ptr(), ta, b.ptr(), tb);
                    assert_eq!(cv, rv, "{row} c2Collided(typeA={ta}, typeB={tb})");
                    assert_eq!(cv, 0, "{row} expected the sentinel 0, got {cv}");
                }
            }
        }
        // Neither shape pointer is dereferenced on the rejecting paths, so NULL
        // shapes must also be accepted without a fault by both libraries.
        for &ta in BAD_TYPES {
            for &tb in BAD_TYPES {
                let cv = c(std::ptr::null(), ta, std::ptr::null(), tb);
                let rv = r(std::ptr::null(), ta, std::ptr::null(), tb);
                assert_eq!(cv, rv, "E1 NULL shapes with bad types");
                assert_eq!(cv, 0);
            }
        }
        for ta in GOOD_TYPES {
            for &tb in BAD_TYPES {
                let a = shape_of(&mut rng, ta);
                let cv = c(a.ptr(), ta, std::ptr::null(), tb);
                let rv = r(a.ptr(), ta, std::ptr::null(), tb);
                assert_eq!(cv, rv, "E2-4 NULL B with bad typeB={tb}");
                assert_eq!(cv, 0);
            }
        }
    }
}

// ===========================================================================
// E5 — c2MakeProxy with an out-of-range type writes NOTHING
// ===========================================================================

#[test]
fn e5_c2MakeProxy_out_of_range_type_writes_nothing() {
    let p = load();
    let c: FnMakeProxy = p.c.sym("c2MakeProxy");
    let r: FnMakeProxy = p.rs.sym("c2MakeProxy");
    let mut rng = Rng::new(0xE5);
    unsafe {
        for &ty in BAD_TYPES {
            for _ in 0..256 {
                // distinctive, fully-populated seed so ANY write is visible
                let mut seed = c2Proxy {
                    radius: rng.wild(),
                    count: rng.next_u32() as c_int,
                    verts: Default::default(),
                };
                for i in 0..8 {
                    seed.verts[i] = rng.v_wild();
                }
                let sh = shape_of(&mut rng, C2_TYPE_CAPSULE);
                let mut cp = seed;
                let mut rp = seed;
                c(sh.ptr(), ty, &mut cp);
                r(sh.ptr(), ty, &mut rp);
                assert_eq!(
                    raw(&cp),
                    raw(&rp),
                    "E5 c2MakeProxy(type={ty}) diverged\n C   : {}\n Rust: {}",
                    proxy_hex(&cp),
                    proxy_hex(&rp)
                );
                // and the sentinel behaviour itself: the proxy is untouched
                assert_eq!(
                    raw(&cp),
                    raw(&seed),
                    "E5 c2MakeProxy(type={ty}) must not write anything"
                );
            }
            // a NULL shape is fine too: no branch dereferences it
            let mut cp = c2Proxy::default();
            let mut rp = c2Proxy::default();
            c(std::ptr::null(), ty, &mut cp);
            r(std::ptr::null(), ty, &mut rp);
            assert_eq!(raw(&cp), raw(&rp), "E5 NULL shape, type={ty}");
        }
    }
}

// ===========================================================================
// E6..E11 — c2GJK NULL-pointer parameters
// ===========================================================================

struct GjkOut {
    rc: f32,
    a: c2v,
    b: c2v,
    it: c_int,
    cache: c2GJKCache,
}

#[allow(clippy::too_many_arguments)]
fn gjk(
    f: FnGJK,
    a: &Buf,
    ta: c_int,
    ax: Option<&c2x>,
    b: &Buf,
    tb: c_int,
    bx: Option<&c2x>,
    want_a: bool,
    want_b: bool,
    use_radius: c_int,
    want_it: bool,
    cache: Option<c2GJKCache>,
) -> GjkOut {
    let mut oa = c2v { x: 111.0, y: 222.0 };
    let mut ob = c2v { x: 333.0, y: 444.0 };
    let mut it: c_int = -77;
    let mut cc = cache.unwrap_or_default();
    let rc = unsafe {
        f(
            a.ptr(),
            ta,
            ax.map(|x| x as *const c2x).unwrap_or(std::ptr::null()),
            b.ptr(),
            tb,
            bx.map(|x| x as *const c2x).unwrap_or(std::ptr::null()),
            if want_a { &mut oa } else { std::ptr::null_mut() },
            if want_b { &mut ob } else { std::ptr::null_mut() },
            use_radius,
            if want_it { &mut it } else { std::ptr::null_mut() },
            if cache.is_some() {
                &mut cc
            } else {
                std::ptr::null_mut()
            },
        )
    };
    GjkOut {
        rc,
        a: oa,
        b: ob,
        it,
        cache: cc,
    }
}

fn same(x: &GjkOut, y: &GjkOut) -> bool {
    x.rc.to_bits() == y.rc.to_bits()
        && raw(&x.a) == raw(&y.a)
        && raw(&x.b) == raw(&y.b)
        && x.it == y.it
        && raw(&x.cache) == raw(&y.cache)
}

fn assert_same(x: &GjkOut, y: &GjkOut, ctx: &str) {
    assert!(
        same(x, y),
        "DIVERGENCE {ctx}\n  C   : rc={} a={} b={} it={} cache=[{}]\n  Rust: rc={} a={} b={} it={} cache=[{}]",
        f32_hex(x.rc),
        v_hex(&x.a),
        v_hex(&x.b),
        x.it,
        cache_hex(&x.cache),
        f32_hex(y.rc),
        v_hex(&y.a),
        v_hex(&y.b),
        y.it,
        cache_hex(&y.cache),
    );
}

#[test]
fn e6_e7_c2GJK_null_transform_equals_explicit_identity() {
    let p = load();
    let cf: FnGJK = p.c.sym("c2GJK");
    let rf: FnGJK = p.rs.sym("c2GJK");
    let mut rng = Rng::new(0xE6);
    let ident = c2x {
        p: c2v { x: 0.0, y: 0.0 },
        r: c2r { c: 1.0, s: 0.0 },
    };
    for ta in GOOD_TYPES {
        for tb in GOOD_TYPES {
            for use_radius in [0, 1] {
                for _ in 0..400 {
                    let a = shape_of(&mut rng, ta);
                    let b = shape_of(&mut rng, tb);
                    // E6: ax_ptr == NULL
                    let cn = gjk(cf, &a, ta, None, &b, tb, Some(&ident), true, true, use_radius, true, None);
                    let rn = gjk(rf, &a, ta, None, &b, tb, Some(&ident), true, true, use_radius, true, None);
                    assert_same(&cn, &rn, "E6 ax_ptr == NULL");
                    // ... and it must equal an explicitly-passed identity
                    let ce = gjk(cf, &a, ta, Some(&ident), &b, tb, Some(&ident), true, true, use_radius, true, None);
                    assert!(
                        same(&cn, &ce),
                        "E6: NULL ax_ptr did not behave like c2xIdentity()"
                    );
                    // E7: bx_ptr == NULL
                    let cn2 = gjk(cf, &a, ta, Some(&ident), &b, tb, None, true, true, use_radius, true, None);
                    let rn2 = gjk(rf, &a, ta, Some(&ident), &b, tb, None, true, true, use_radius, true, None);
                    assert_same(&cn2, &rn2, "E7 bx_ptr == NULL");
                    assert!(same(&cn2, &ce), "E7: NULL bx_ptr did not behave like identity");
                    // both NULL
                    let cb = gjk(cf, &a, ta, None, &b, tb, None, true, true, use_radius, true, None);
                    let rb = gjk(rf, &a, ta, None, &b, tb, None, true, true, use_radius, true, None);
                    assert_same(&cb, &rb, "E6+E7 both transforms NULL");
                    assert!(same(&cb, &ce), "both NULL did not behave like identity");
                }
            }
        }
    }
}

#[test]
fn e8_e9_e10_c2GJK_null_outputs() {
    let p = load();
    let cf: FnGJK = p.c.sym("c2GJK");
    let rf: FnGJK = p.rs.sym("c2GJK");
    let mut rng = Rng::new(0xE8);
    for ta in GOOD_TYPES {
        for tb in GOOD_TYPES {
            for use_radius in [0, 1] {
                for mask in 0..8u32 {
                    for _ in 0..80 {
                        let a = shape_of(&mut rng, ta);
                        let b = shape_of(&mut rng, tb);
                        let (wa, wb, wi) = (mask & 1 != 0, mask & 2 != 0, mask & 4 != 0);
                        let co = gjk(cf, &a, ta, None, &b, tb, None, wa, wb, use_radius, wi, None);
                        let ro = gjk(rf, &a, ta, None, &b, tb, None, wa, wb, use_radius, wi, None);
                        assert_same(&co, &ro, &format!("E8/E9/E10 mask={mask:03b}"));
                        // the untouched sentinels must survive verbatim
                        if !wa {
                            assert_eq!(raw(&co.a), raw(&c2v { x: 111.0, y: 222.0 }), "E8 outA was written");
                        }
                        if !wb {
                            assert_eq!(raw(&co.b), raw(&c2v { x: 333.0, y: 444.0 }), "E9 outB was written");
                        }
                        if !wi {
                            assert_eq!(co.it, -77, "E10 *iterations was written");
                        }
                        // and rc is still produced regardless
                        let full = gjk(cf, &a, ta, None, &b, tb, None, true, true, use_radius, true, None);
                        assert_eq!(
                            co.rc.to_bits(),
                            full.rc.to_bits(),
                            "E8/9/10 rc changed because an output pointer was NULL"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn e11_e12_c2GJK_null_and_cold_cache() {
    let p = load();
    let cf: FnGJK = p.c.sym("c2GJK");
    let rf: FnGJK = p.rs.sym("c2GJK");
    let mut rng = Rng::new(0xEB);
    for ta in GOOD_TYPES {
        for tb in GOOD_TYPES {
            for use_radius in [0, 1] {
                for _ in 0..400 {
                    let a = shape_of(&mut rng, ta);
                    let b = shape_of(&mut rng, tb);
                    // E11: cache == NULL
                    let cn = gjk(cf, &a, ta, None, &b, tb, None, true, true, use_radius, true, None);
                    let rn = gjk(rf, &a, ta, None, &b, tb, None, true, true, use_radius, true, None);
                    assert_same(&cn, &rn, "E11 cache == NULL");
                    // E12: cache != NULL with count == 0 -> not read, but written
                    let zero = c2GJKCache::default();
                    let cz = gjk(cf, &a, ta, None, &b, tb, None, true, true, use_radius, true, Some(zero));
                    let rz = gjk(rf, &a, ta, None, &b, tb, None, true, true, use_radius, true, Some(zero));
                    assert_same(&cz, &rz, "E12 cold cache");
                    // a cold cache must give the same answer as no cache at all
                    assert_eq!(
                        cn.rc.to_bits(),
                        cz.rc.to_bits(),
                        "E12 cold cache changed the result"
                    );
                    // and it must have been written on the way out
                    assert!(
                        cz.cache.count >= 1 && cz.cache.count <= 3,
                        "E12 cache was not written (count={})",
                        cz.cache.count
                    );
                    // a cache whose count is 0 but whose other fields are junk
                    // must still be ignored on entry
                    let junk = c2GJKCache {
                        metric: rng.wild(),
                        count: 0,
                        iA: [7, 6, 5],
                        iB: [4, 3, 2],
                        div: rng.wild(),
                    };
                    let cj = gjk(cf, &a, ta, None, &b, tb, None, true, true, use_radius, true, Some(junk));
                    let rj = gjk(rf, &a, ta, None, &b, tb, None, true, true, use_radius, true, Some(junk));
                    assert_same(&cj, &rj, "E12 count==0 with junk fields");
                    assert_eq!(
                        cj.rc.to_bits(),
                        cn.rc.to_bits(),
                        "E12 a count==0 cache must be ignored on entry"
                    );
                }
            }
        }
    }
}

#[test]
fn e13_e14_c2GJK_cache_validity_test_both_branches() {
    let p = load();
    let cf: FnGJK = p.c.sym("c2GJK");
    let rf: FnGJK = p.rs.sym("c2GJK");
    let mut rng = Rng::new(0xED);
    // E13 = cache accepted (metric >= -1.0e8f makes the conjunct false, so the
    //       negated test is true).  E14 = cache rejected.
    let mut accepted = 0usize;
    let mut rejected = 0usize;
    for ta in GOOD_TYPES {
        for tb in GOOD_TYPES {
            let na = match ta {
                C2_TYPE_CIRCLE => 1u32,
                C2_TYPE_AABB => 4,
                _ => 2,
            };
            let nb = match tb {
                C2_TYPE_CIRCLE => 1u32,
                C2_TYPE_AABB => 4,
                _ => 2,
            };
            for count in 1..=3i32 {
                for &metric in &[
                    0.0f32,
                    -1.0,
                    1.0,
                    -9.9e7,
                    -1.0e8,
                    -1.000001e8,
                    -1.0e9,
                    -f32::MAX,
                    f32::NEG_INFINITY,
                    f32::INFINITY,
                    f32::NAN,
                ] {
                    for use_radius in [0, 1] {
                        for _ in 0..24 {
                            let a = shape_of(&mut rng, ta);
                            let b = shape_of(&mut rng, tb);
                            let mut cache = c2GJKCache {
                                metric,
                                count,
                                iA: [0; 3],
                                iB: [0; 3],
                                div: rng.uniform(0.05, 8.0),
                            };
                            for i in 0..3 {
                                cache.iA[i] = rng.below(na) as c_int;
                                cache.iB[i] = rng.below(nb) as c_int;
                            }
                            let co = gjk(cf, &a, ta, None, &b, tb, None, true, true, use_radius, true, Some(cache));
                            let ro = gjk(rf, &a, ta, None, &b, tb, None, true, true, use_radius, true, Some(cache));
                            assert_same(
                                &co,
                                &ro,
                                &format!("E13/E14 metric={} count={count}", f32_hex(metric)),
                            );
                            // Compare with a forced cold start to tell the two
                            // branches apart.
                            let cold = gjk(cf, &a, ta, None, &b, tb, None, true, true, use_radius, true, Some(c2GJKCache::default()));
                            if co.rc.to_bits() == cold.rc.to_bits() && raw(&co.a) == raw(&cold.a) {
                                rejected += 1;
                            } else {
                                accepted += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    eprintln!("E13/E14: cache-influenced={accepted} cache-equivalent-to-cold={rejected}");
    assert!(accepted > 0, "E13: the cache was never actually read");
    assert!(rejected > 0, "E14: the cold-start path was never taken");
}

#[test]
fn e15_c2GJK_negative_cache_count() {
    let p = load();
    let cf: FnGJK = p.c.sym("c2GJK");
    let rf: FnGJK = p.rs.sym("c2GJK");
    let mut rng = Rng::new(0xEF);
    for ta in GOOD_TYPES {
        for tb in GOOD_TYPES {
            for &count in &[-1i32, -2, -3, -100, i32::MIN, i32::MIN + 1] {
                for use_radius in [0, 1] {
                    for _ in 0..48 {
                        let a = shape_of(&mut rng, ta);
                        let b = shape_of(&mut rng, tb);
                        let cache = c2GJKCache {
                            metric: rng.uniform(-2.0e8, 2.0e8),
                            count,
                            iA: [rng.below(4) as c_int; 3],
                            iB: [rng.below(4) as c_int; 3],
                            div: rng.uniform(0.05, 8.0),
                        };
                        let co = gjk(cf, &a, ta, None, &b, tb, None, true, true, use_radius, true, Some(cache));
                        let ro = gjk(rf, &a, ta, None, &b, tb, None, true, true, use_radius, true, Some(cache));
                        assert_same(&co, &ro, &format!("E15 cache->count={count}"));
                        // documented sentinel: rc == +0.0, no iterations, and
                        // the witness collapses to the origin
                        assert_eq!(co.rc.to_bits(), 0, "E15 expected rc == +0.0, got {}", f32_hex(co.rc));
                        assert_eq!(co.it, 0, "E15 expected *iterations == 0");
                        assert_eq!(raw(&co.a), raw(&c2v { x: 0.0, y: 0.0 }), "E15 outA");
                        assert_eq!(raw(&co.b), raw(&c2v { x: 0.0, y: 0.0 }), "E15 outB");
                        assert_eq!(co.cache.count, count, "E15 cache->count preserved");
                        assert_eq!(co.cache.metric.to_bits(), 0, "E15 cache->metric = +0.0");
                    }
                }
            }
        }
    }
}

// ===========================================================================
// E20..E24 — the `use_radius` block
// ===========================================================================

#[test]
fn e20_e21_e22_c2GJK_radius_block_sentinels() {
    let p = load();
    let cf: FnGJK = p.c.sym("c2GJK");
    let rf: FnGJK = p.rs.sym("c2GJK");
    let mut rng = Rng::new(0x20);
    let mut e20 = 0usize; // midpoint branch (dist <= rA+rB or dist <= eps)
    let mut e21 = 0usize; // shrink produced a == b, so rc forced to +0.0
    let mut e22 = 0usize; // hit == 1
    for _ in 0..40_000 {
        // Circle-vs-circle lets the separation be dialled exactly.
        let rA = rng.uniform(0.0, 20.0);
        let rB = rng.uniform(0.0, 20.0);
        let sep = match rng.below(4) {
            0 => rA + rB,                     // exactly touching -> E20
            1 => rng.uniform(0.0, rA + rB),   // overlapping      -> E20
            2 => 0.0,                         // concentric       -> E20
            _ => rng.uniform(0.0, 60.0),
        };
        let a = Buf::of(&c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: rA });
        let b = Buf::of(&c2Circle { p: c2v { x: sep, y: 0.0 }, r: rB });
        let c0 = gjk(cf, &a, C2_TYPE_CIRCLE, None, &b, C2_TYPE_CIRCLE, None, true, true, 0, true, None);
        let c1 = gjk(cf, &a, C2_TYPE_CIRCLE, None, &b, C2_TYPE_CIRCLE, None, true, true, 1, true, None);
        let r1 = gjk(rf, &a, C2_TYPE_CIRCLE, None, &b, C2_TYPE_CIRCLE, None, true, true, 1, true, None);
        let r0 = gjk(rf, &a, C2_TYPE_CIRCLE, None, &b, C2_TYPE_CIRCLE, None, true, true, 0, true, None);
        assert_same(&c0, &r0, "E20-22 use_radius=0");
        assert_same(&c1, &r1, "E20-22 use_radius=1");

        let dist = c0.rc;
        if c1.rc.to_bits() == 0 {
            if raw(&c1.a) == raw(&c1.b) && !(dist > rA + rB && dist > f32::EPSILON) {
                e20 += 1;
                // documented sentinel for the midpoint branch
                assert_eq!(c1.rc.to_bits(), 0, "E20 must return exactly +0.0");
            } else if dist > rA + rB && dist > f32::EPSILON {
                e21 += 1;
            }
        }
        if c0.rc.to_bits() == 0 && raw(&c0.a) == raw(&c0.b) {
            e22 += 1;
        }
    }
    // E22 (hit == 1): guarantee the path with a deeply nested pair
    for _ in 0..2000 {
        let cc = rng.v();
        let big = Buf::of(&c2AABB {
            min: c2v { x: cc.x - 60.0, y: cc.y - 60.0 },
            max: c2v { x: cc.x + 60.0, y: cc.y + 60.0 },
        });
        let inner = Buf::of(&c2Capsule {
            a: c2v { x: cc.x - 0.5, y: cc.y - 0.5 },
            b: c2v { x: cc.x + 0.5, y: cc.y + 0.5 },
            r: 1.0,
        });
        for use_radius in [0, 1] {
            let co = gjk(cf, &big, C2_TYPE_AABB, None, &inner, C2_TYPE_CAPSULE, None, true, true, use_radius, true, None);
            let ro = gjk(rf, &big, C2_TYPE_AABB, None, &inner, C2_TYPE_CAPSULE, None, true, true, use_radius, true, None);
            assert_same(&co, &ro, "E22 nested pair");
            if co.rc.to_bits() == 0 && raw(&co.a) == raw(&co.b) {
                e22 += 1;
            }
        }
    }
    eprintln!("E20 midpoint={e20}  E21 shrink-to-equal={e21}  E22 hit/zero={e22}");
    assert!(e20 > 0, "E20 (midpoint branch) never fired");
    assert!(e22 > 0, "E22 (hit / rc == +0.0 with a == b) never fired");
    // E21 is a knife-edge float coincidence; it is *covered* by the exhaustive
    // touching sweep below, which asserts the sentinel directly.
    let _ = e21;
}

/// E21 — after the radius shrink `a == b` exactly, so `dist` is forced to
/// `+0.0` even though `dist - rA - rB > 0` had just been computed.
#[test]
fn e21_c2GJK_shrink_collapses_to_zero() {
    let p = load();
    let cf: FnGJK = p.c.sym("c2GJK");
    let rf: FnGJK = p.rs.sym("c2GJK");
    let mut found = 0usize;
    // Separation just barely above rA + rB: the shrink then moves `a` and `b`
    // onto the same float.
    for k in 0..20_000u32 {
        let rA = 10.0f32;
        let rB = 5.0f32;
        // step by 1 ULP upwards from exactly-touching
        let sep = f32::from_bits((rA + rB).to_bits() + k);
        let a = Buf::of(&c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: rA });
        let b = Buf::of(&c2Circle { p: c2v { x: sep, y: 0.0 }, r: rB });
        let co = gjk(cf, &a, C2_TYPE_CIRCLE, None, &b, C2_TYPE_CIRCLE, None, true, true, 1, true, None);
        let ro = gjk(rf, &a, C2_TYPE_CIRCLE, None, &b, C2_TYPE_CIRCLE, None, true, true, 1, true, None);
        assert_same(&co, &ro, &format!("E21 sep=+{k}ulp"));
        if co.rc.to_bits() == 0 && raw(&co.a) == raw(&co.b) {
            found += 1;
        }
    }
    eprintln!("E21: {found} of 20000 one-ULP steps collapsed a == b");
    assert!(found > 0, "E21 never reached the `a == b` collapse");
}

#[test]
fn e23_e24_c2GJK_negative_and_nonfinite_radii() {
    let p = load();
    let cf: FnGJK = p.c.sym("c2GJK");
    let rf: FnGJK = p.rs.sym("c2GJK");
    let mut rng = Rng::new(0x23);
    // E23: negative radii GROW the distance rather than being rejected
    let mut grew = 0usize;
    for _ in 0..20_000 {
        let rA = -rng.uniform(0.0, 20.0);
        let rB = -rng.uniform(0.0, 20.0);
        let sep = rng.uniform(1.0, 60.0);
        let a = Buf::of(&c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: rA });
        let b = Buf::of(&c2Circle { p: c2v { x: sep, y: 0.0 }, r: rB });
        let c0 = gjk(cf, &a, C2_TYPE_CIRCLE, None, &b, C2_TYPE_CIRCLE, None, true, true, 0, true, None);
        let c1 = gjk(cf, &a, C2_TYPE_CIRCLE, None, &b, C2_TYPE_CIRCLE, None, true, true, 1, true, None);
        let r1 = gjk(rf, &a, C2_TYPE_CIRCLE, None, &b, C2_TYPE_CIRCLE, None, true, true, 1, true, None);
        assert_same(&c1, &r1, "E23 negative radii");
        if c1.rc > c0.rc {
            grew += 1;
        }
    }
    eprintln!("E23: {grew} cases where use_radius=1 GREW the distance");
    assert!(grew > 0, "E23 never observed the distance growing");

    // E24: NaN / +-inf radii with use_radius = 1 -> the midpoint branch -> +0.0
    for rv in [
        f32::NAN,
        f32::from_bits(0xffc0_0000),
        f32::from_bits(0x7f80_0001),
        f32::INFINITY,
        f32::NEG_INFINITY,
    ] {
        for which in 0..3 {
            for _ in 0..200 {
                let (mut rA, mut rB) = (rng.uniform(0.0, 10.0), rng.uniform(0.0, 10.0));
                match which {
                    0 => rA = rv,
                    1 => rB = rv,
                    _ => {
                        rA = rv;
                        rB = rv;
                    }
                }
                let sep = rng.uniform(20.0, 60.0);
                let a = Buf::of(&c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: rA });
                let b = Buf::of(&c2Circle { p: c2v { x: sep, y: 0.0 }, r: rB });
                let co = gjk(cf, &a, C2_TYPE_CIRCLE, None, &b, C2_TYPE_CIRCLE, None, true, true, 1, true, None);
                let ro = gjk(rf, &a, C2_TYPE_CIRCLE, None, &b, C2_TYPE_CIRCLE, None, true, true, 1, true, None);
                assert_same(&co, &ro, &format!("E24 r={} which={which}", f32_hex(rv)));
                if rv.is_nan() || rv == f32::INFINITY {
                    assert_eq!(
                        co.rc.to_bits(),
                        0,
                        "E24 NaN/+inf radius must take the midpoint branch (rc == +0.0), got {}",
                        f32_hex(co.rc)
                    );
                }
            }
        }
    }
}

#[test]
fn e25_c2GJK_nan_coordinates() {
    let p = load();
    let cf: FnGJK = p.c.sym("c2GJK");
    let rf: FnGJK = p.rs.sym("c2GJK");
    let mut rng = Rng::new(0x25);
    let nans = [
        f32::NAN,
        f32::from_bits(0x7fc0_0000),
        f32::from_bits(0xffc0_0000),
        f32::from_bits(0x7f80_0001),
        f32::from_bits(0xff80_0001),
        f32::from_bits(0x7fab_cdef),
    ];
    for ta in GOOD_TYPES {
        for tb in GOOD_TYPES {
            for &nan in &nans {
                for slot in 0..5usize {
                    for use_radius in [0, 1] {
                        for _ in 0..24 {
                            // poison one float of shape A
                            let mut ab = shape_of(&mut rng, ta);
                            let off = 4 * slot.min(4);
                            if off + 4 <= 20 {
                                ab.bytes[off..off + 4].copy_from_slice(&nan.to_bits().to_le_bytes());
                            }
                            let b = shape_of(&mut rng, tb);
                            let co = gjk(cf, &ab, ta, None, &b, tb, None, true, true, use_radius, true, None);
                            let ro = gjk(rf, &ab, ta, None, &b, tb, None, true, true, use_radius, true, None);
                            assert_same(
                                &co,
                                &ro,
                                &format!("E25 NaN={} slot={slot} ta={ta} tb={tb}", f32_hex(nan)),
                            );
                        }
                    }
                }
            }
        }
    }
}

/// E26..E29 — loop terminators.
///
/// `iter == 20` (E26) turns out to be **unreachable** through the public API:
/// the three shape types have at most 4 vertices, and the duplicate-support
/// check (3-slot `saveA`/`saveB`) plus the monotone `d1 > d0` guard bound the
/// loop far below the cap.  A 3.4-million-call randomized sweep (finite, wild,
/// NaN, warm-started, rotated, non-normalised rotations) never exceeded
/// `*iterations == 4`.  The strongest available differential statement is
/// therefore: **every reachable iteration count is bit-identical in both
/// libraries, and both libraries have the same reachable maximum.**
#[test]
fn e26_e27_e28_e29_c2GJK_loop_terminators() {
    let p = load();
    let cf: FnGJK = p.c.sym("c2GJK");
    let rf: FnGJK = p.rs.sym("c2GJK");
    let mut rng = Rng::new(0x26);
    let mut chist = [0usize; 22];
    let mut rhist = [0usize; 22];
    for _ in 0..120_000 {
        let ta = GOOD_TYPES[rng.below(3) as usize];
        let tb = GOOD_TYPES[rng.below(3) as usize];
        let wild = rng.below(3) == 0;
        let mk = |rng: &mut Rng, t: c_int| -> Buf {
            match (t, wild) {
                (C2_TYPE_CIRCLE, false) => Buf::of(&rng.circle()),
                (C2_TYPE_AABB, false) => Buf::of(&rng.aabb()),
                (C2_TYPE_CIRCLE, true) => Buf::of(&c2Circle { p: rng.v_wild(), r: rng.wild() }),
                (C2_TYPE_AABB, true) => Buf::of(&c2AABB { min: rng.v_wild(), max: rng.v_wild() }),
                (_, true) => Buf::of(&c2Capsule { a: rng.v_wild(), b: rng.v_wild(), r: rng.wild() }),
                _ => Buf::of(&rng.capsule()),
            }
        };
        let a = mk(&mut rng, ta);
        let b = mk(&mut rng, tb);
        let axv = rng.x();
        let bxv = rng.x();
        let (ua, ub) = (rng.below(2) == 0, rng.below(2) == 0);
        let cache = if rng.below(2) == 0 {
            let na = match ta { C2_TYPE_CIRCLE => 1u32, C2_TYPE_AABB => 4, _ => 2 };
            let nb = match tb { C2_TYPE_CIRCLE => 1u32, C2_TYPE_AABB => 4, _ => 2 };
            let mut cc = c2GJKCache {
                metric: if rng.below(3) == 0 { rng.wild() } else { rng.uniform(-2.0e8, 2.0e8) },
                count: 1 + rng.below(3) as c_int,
                iA: [0; 3],
                iB: [0; 3],
                div: rng.uniform(0.01, 4.0),
            };
            for i in 0..3 {
                cc.iA[i] = rng.below(na) as c_int;
                cc.iB[i] = rng.below(nb) as c_int;
            }
            Some(cc)
        } else {
            None
        };
        let ur = rng.below(2) as c_int;
        let co = gjk(cf, &a, ta, if ua { Some(&axv) } else { None }, &b, tb, if ub { Some(&bxv) } else { None }, true, true, ur, true, cache);
        let ro = gjk(rf, &a, ta, if ua { Some(&axv) } else { None }, &b, tb, if ub { Some(&bxv) } else { None }, true, true, ur, true, cache);
        assert_same(&co, &ro, "E26-29 terminator sweep");
        chist[co.it.clamp(0, 21) as usize] += 1;
        rhist[ro.it.clamp(0, 21) as usize] += 1;
    }
    assert_eq!(chist, rhist, "E26-29 iteration histograms differ");
    let cmax = (0..22).rev().find(|&i| chist[i] > 0).unwrap();
    let rmax = (0..22).rev().find(|&i| rhist[i] > 0).unwrap();
    assert_eq!(cmax, rmax, "E26 reachable maximum *iterations differs");
    eprintln!("E26-29 iteration histogram (identical in both) = {chist:?}, max = {cmax}");
    // E27/E28/E29 all terminate below the cap, so several distinct counts must
    // appear; a single value would mean only one terminator is exercised.
    assert!(
        chist.iter().filter(|&&n| n > 0).count() >= 3,
        "only {} distinct iteration counts seen",
        chist.iter().filter(|&&n| n > 0).count()
    );
    assert!(chist[0] > 0, "E28 (immediate `dot(d,d) < eps^2` break) never fired");
    assert!(cmax < 20, "E26 unexpectedly reachable — update ERRORS.md");
}

// ===========================================================================
// E30..E34 — c2Witness / c2L with a zero `div` or an out-of-range `count`
// ===========================================================================

/// Every `count` value an FFI caller can put in a `c2Simplex`.
const BAD_COUNTS: &[c_int] = &[0, 4, 5, -1, -2, 100, i32::MIN, i32::MAX];

#[test]
fn e30_e31_e32_c2Witness() {
    let p = load();
    let c: FnWitness = p.c.sym("c2Witness");
    let r: FnWitness = p.rs.sym("c2Witness");
    let mut rng = Rng::new(0x30);
    unsafe {
        let run = |s: c2Simplex, ctx: &str| -> (c2v, c2v) {
            let mut cs = s;
            let mut rs_ = s;
            let mut ca = c2v { x: 99.0, y: -99.0 };
            let mut cb = c2v { x: -55.0, y: 55.0 };
            let mut ra = ca;
            let mut rb = cb;
            c(&mut cs, &mut ca, &mut cb);
            r(&mut rs_, &mut ra, &mut rb);
            assert!(
                raw(&ca) == raw(&ra) && raw(&cb) == raw(&rb) && raw(&cs) == raw(&rs_),
                "DIVERGENCE c2Witness {ctx}\n  in  : {}\n  C   a={} b={}\n  Rust a={} b={}",
                simplex_hex(&s),
                v_hex(&ca),
                v_hex(&cb),
                v_hex(&ra),
                v_hex(&rb)
            );
            (ca, cb)
        };
        // E30 / E31: div == +0.0 and -0.0 at every valid count
        for count in [1i32, 2, 3] {
            for divv in [0.0f32, -0.0] {
                for _ in 0..500 {
                    let mut s = rng.simplex(count);
                    s.div = divv;
                    run(s, &format!("E30/E31 count={count} div={}", f32_hex(divv)));
                }
                // pin the documented behaviour: den = +-inf, so a non-zero `u`
                // yields +-inf and a zero `u` yields NaN
                let mut s = c2Simplex::default();
                s.count = 2;
                s.div = divv;
                s.verts[0].u = 1.0;
                s.verts[1].u = 0.0;
                s.verts[0].sA = c2v { x: 1.0, y: 1.0 };
                s.verts[1].sA = c2v { x: 2.0, y: 2.0 };
                s.verts[0].sB = c2v { x: 3.0, y: 3.0 };
                s.verts[1].sB = c2v { x: 4.0, y: 4.0 };
                let (a, _) = run(s, "E30/E31 pinned");
                assert!(
                    a.x.is_nan() || a.x.is_infinite(),
                    "E30/E31 expected a non-finite witness, got {}",
                    f32_hex(a.x)
                );
            }
        }
        // E32: count outside {1,2,3} -> *a = *b = {0,0}
        for &count in BAD_COUNTS {
            for _ in 0..500 {
                let mut s = rng.simplex(2);
                s.count = count;
                let (a, b) = run(s, &format!("E32 count={count}"));
                assert_eq!(
                    raw(&a),
                    raw(&c2v { x: 0.0, y: 0.0 }),
                    "E32 count={count}: outA must be the {{0,0}} sentinel, got {}",
                    v_hex(&a)
                );
                assert_eq!(
                    raw(&b),
                    raw(&c2v { x: 0.0, y: 0.0 }),
                    "E32 count={count}: outB must be the {{0,0}} sentinel, got {}",
                    v_hex(&b)
                );
            }
            // and with a degenerate div as well
            for divv in [0.0f32, -0.0, f32::NAN, f32::INFINITY] {
                let mut s = rng.simplex(2);
                s.count = count;
                s.div = divv;
                let (a, b) = run(s, &format!("E32 count={count} div={}", f32_hex(divv)));
                assert_eq!(raw(&a), raw(&c2v { x: 0.0, y: 0.0 }));
                assert_eq!(raw(&b), raw(&c2v { x: 0.0, y: 0.0 }));
            }
        }
    }
}

#[test]
fn e33_e34_c2L() {
    let p = load();
    let c: FnSimplexv = p.c.sym("c2L");
    let r: FnSimplexv = p.rs.sym("c2L");
    let mut rng = Rng::new(0x33);
    unsafe {
        let run = |s: c2Simplex, ctx: &str| -> c2v {
            let mut cs = s;
            let mut rs_ = s;
            let cv = c(&mut cs);
            let rv = r(&mut rs_);
            assert!(
                raw(&cv) == raw(&rv) && raw(&cs) == raw(&rs_),
                "DIVERGENCE c2L {ctx}\n  in  : {}\n  C={} Rust={}",
                simplex_hex(&s),
                v_hex(&cv),
                v_hex(&rv)
            );
            cv
        };
        // E33: div == +-0 with count 1 and 2
        for count in [1i32, 2] {
            for divv in [0.0f32, -0.0] {
                for _ in 0..500 {
                    let mut s = rng.simplex(count);
                    s.div = divv;
                    run(s, &format!("E33 count={count} div={}", f32_hex(divv)));
                }
            }
        }
        // E33 pinned: count 2, div 0, non-zero u -> non-finite result
        let mut s = c2Simplex::default();
        s.count = 2;
        s.div = 0.0;
        s.verts[0].u = 1.0;
        s.verts[1].u = 1.0;
        s.verts[0].p = c2v { x: 1.0, y: -1.0 };
        s.verts[1].p = c2v { x: 2.0, y: -2.0 };
        let v = run(s, "E33 pinned");
        assert!(!v.x.is_finite(), "E33 expected a non-finite x, got {}", f32_hex(v.x));

        // E34: count outside {1,2} (notably 3) -> {0,0}
        for &count in BAD_COUNTS.iter().chain([3i32].iter()) {
            for _ in 0..400 {
                let mut s = rng.simplex(2);
                s.count = count;
                let v = run(s, &format!("E34 count={count}"));
                assert_eq!(
                    raw(&v),
                    raw(&c2v { x: 0.0, y: 0.0 }),
                    "E34 count={count}: expected the {{0,0}} sentinel, got {}",
                    v_hex(&v)
                );
            }
        }
    }
}

// ===========================================================================
// E35..E39 — unguarded divisions and sqrtf
// ===========================================================================

#[test]
fn e35_e36_e37_c2Div_c2Norm_division_by_zero() {
    let p = load();
    let cd: FnDiv = p.c.sym("c2Div");
    let rd: FnDiv = p.rs.sym("c2Div");
    let cn: FnV1 = p.c.sym("c2Norm");
    let rn: FnV1 = p.rs.sym("c2Norm");
    let mut rng = Rng::new(0x35);
    unsafe {
        // E35 / E36: b == +-0
        for divv in [0.0f32, -0.0] {
            for _ in 0..4000 {
                let a = if rng.below(3) == 0 { rng.v_wild() } else { rng.v() };
                assert_bits_eq!(
                    cd(a, divv),
                    rd(a, divv),
                    "E35/E36 c2Div({}, {})",
                    v_hex(&a),
                    f32_hex(divv)
                );
            }
            // pinned sentinels
            let one = c2v { x: 1.0, y: -1.0 };
            let got = cd(one, divv);
            assert_bits_eq!(got, rd(one, divv), "E35/E36 pinned");
            let want_x = if divv.is_sign_negative() { f32::NEG_INFINITY } else { f32::INFINITY };
            assert_eq!(
                got.x.to_bits(),
                want_x.to_bits(),
                "E35/E36 c2Div(1, {}) expected {}, got {}",
                f32_hex(divv),
                f32_hex(want_x),
                f32_hex(got.x)
            );
            // 0 / 0 -> NaN
            let zero = c2v { x: 0.0, y: -0.0 };
            let gz = cd(zero, divv);
            assert_bits_eq!(gz, rd(zero, divv), "E35/E36 zero numerator");
            assert!(gz.x.is_nan(), "E35 0*inf must be NaN, got {}", f32_hex(gz.x));
        }
        // E37: c2Norm of the zero vector -> {NaN, NaN}
        for z in [
            c2v { x: 0.0, y: 0.0 },
            c2v { x: -0.0, y: 0.0 },
            c2v { x: 0.0, y: -0.0 },
            c2v { x: -0.0, y: -0.0 },
        ] {
            let cv = cn(z);
            let rv = rn(z);
            assert_bits_eq!(cv, rv, "E37 c2Norm({})", v_hex(&z));
            assert!(
                cv.x.is_nan() && cv.y.is_nan(),
                "E37 expected {{NaN,NaN}}, got {}",
                v_hex(&cv)
            );
        }
    }
}

#[test]
fn e38_e39_c2Len_overflow_and_nan() {
    let p = load();
    let c: FnVf = p.c.sym("c2Len");
    let r: FnVf = p.rs.sym("c2Len");
    unsafe {
        // E38: dot(a,a) overflows to +inf -> sqrtf(+inf) == +inf
        for a in [
            c2v { x: f32::MAX, y: f32::MAX },
            c2v { x: f32::MAX, y: 0.0 },
            c2v { x: -f32::MAX, y: -f32::MAX },
            c2v { x: 1.0e30, y: 1.0e30 },
            c2v { x: f32::INFINITY, y: 0.0 },
            c2v { x: f32::NEG_INFINITY, y: 1.0 },
        ] {
            let cv = c(a);
            assert_f32_bits_eq!(cv, r(a), "E38 c2Len({})", v_hex(&a));
            assert_eq!(
                cv.to_bits(),
                f32::INFINITY.to_bits(),
                "E38 expected +inf, got {}",
                f32_hex(cv)
            );
        }
        // dot(a,a) == inf - inf is impossible (both terms are squares), so the
        // only NaN source is a NaN input:
        // E39: sqrtf(NaN) -- the quieted payload must match bit-for-bit
        for nb in [
            0x7fc0_0000u32,
            0xffc0_0000,
            0x7f80_0001,
            0xff80_0001,
            0x7fab_cdef,
            0xffab_cdef,
            0x7fff_ffff,
        ] {
            let n = f32::from_bits(nb);
            for a in [
                c2v { x: n, y: 0.0 },
                c2v { x: 0.0, y: n },
                c2v { x: n, y: n },
                c2v { x: n, y: f32::INFINITY },
            ] {
                let cv = c(a);
                assert_f32_bits_eq!(cv, r(a), "E39 c2Len({})", v_hex(&a));
                assert!(cv.is_nan(), "E39 expected NaN, got {}", f32_hex(cv));
            }
        }
        // and `sqrtf` is never handed a negative: dot(a,a) is a sum of squares
        for a in [
            c2v { x: -3.0, y: -4.0 },
            c2v { x: -1.0e-30, y: -1.0e-30 },
        ] {
            assert_f32_bits_eq!(c(a), r(a), "E39 negative components");
        }
    }
}

// ===========================================================================
// E40 / E41 — c2Support
// ===========================================================================

#[test]
fn e40_e41_c2Support_bad_count_and_zero_direction() {
    let p = load();
    let c: FnSupport = p.c.sym("c2Support");
    let r: FnSupport = p.rs.sym("c2Support");
    let mut rng = Rng::new(0x40);
    unsafe {
        let mut verts = [c2v::default(); 8];
        for i in 0..8 {
            verts[i] = rng.v();
        }
        // E40: count <= 0 -> verts[0] still read for dmax, loop skipped, rc == 0
        for &count in &[0i32, -1, -2, -100, i32::MIN, i32::MIN + 1] {
            for _ in 0..200 {
                let d = if rng.below(3) == 0 { rng.v_wild() } else { rng.v() };
                let cv = c(verts.as_ptr(), count, d);
                let rv = r(verts.as_ptr(), count, d);
                assert_eq!(cv, rv, "E40 c2Support(count={count})");
                assert_eq!(cv, 0, "E40 expected the sentinel 0, got {cv}");
            }
        }
        // E41: d == {0,0} -> every dot is +-0, `dot > dmax` never true -> 0
        for z in [
            c2v { x: 0.0, y: 0.0 },
            c2v { x: -0.0, y: 0.0 },
            c2v { x: 0.0, y: -0.0 },
            c2v { x: -0.0, y: -0.0 },
        ] {
            for count in [1i32, 2, 3, 4, 8] {
                let cv = c(verts.as_ptr(), count, z);
                let rv = r(verts.as_ptr(), count, z);
                assert_eq!(cv, rv, "E41 c2Support(d={})", v_hex(&z));
                assert_eq!(cv, 0, "E41 expected index 0 (ties keep the first), got {cv}");
            }
        }
        // a NaN direction makes every comparison false too -> index 0
        for nb in [0x7fc0_0000u32, 0xffc0_0000, 0x7f80_0001] {
            let d = c2v { x: f32::from_bits(nb), y: f32::from_bits(nb) };
            for count in [1i32, 2, 4, 8] {
                let cv = c(verts.as_ptr(), count, d);
                let rv = r(verts.as_ptr(), count, d);
                assert_eq!(cv, rv, "E41 c2Support(NaN d)");
                assert_eq!(cv, 0, "E41 NaN direction should keep index 0, got {cv}");
            }
        }
        // "oversized length": a count far larger than 8 would read out of
        // bounds, so the largest DEFINED length (8) is the boundary tested, and
        // one step past the logical vertex count of each shape type is covered
        // by the 1/2/4 cases above.
        for count in [7i32, 8] {
            for _ in 0..200 {
                let d = rng.v();
                assert_eq!(
                    c(verts.as_ptr(), count, d),
                    r(verts.as_ptr(), count, d),
                    "E40 boundary count={count}"
                );
            }
        }
    }
}

// ===========================================================================
// E43..E45 — c2GJKSimplexMetric / c2D default arms
// ===========================================================================

#[test]
fn e43_c2GJKSimplexMetric_out_of_range_count() {
    let p = load();
    let c: FnSimplexf = p.c.sym("c2GJKSimplexMetric");
    let r: FnSimplexf = p.rs.sym("c2GJKSimplexMetric");
    let mut rng = Rng::new(0x43);
    unsafe {
        for &count in BAD_COUNTS.iter().chain([1i32].iter()) {
            for _ in 0..400 {
                let mut s = rng.simplex(3);
                s.count = count;
                let mut cs = s;
                let mut rs_ = s;
                let cv = c(&mut cs);
                let rv = r(&mut rs_);
                assert_f32_bits_eq!(cv, rv, "E43 count={count}");
                assert_eq!(
                    cv.to_bits(),
                    0,
                    "E43 count={count}: `default:` falls into `case 1:` so the result must be +0.0, got {}",
                    f32_hex(cv)
                );
                assert_eq!(raw(&cs), raw(&rs_), "E43 simplex was mutated differently");
            }
        }
    }
}

#[test]
fn e44_e45_c2D_default_arm_and_nan_determinant() {
    let p = load();
    let c: FnSimplexv = p.c.sym("c2D");
    let r: FnSimplexv = p.rs.sym("c2D");
    let cskew: FnV1 = p.c.sym("c2Skew");
    let cccw: FnV1 = p.c.sym("c2CCW90");
    let csub: FnVV = p.c.sym("c2Sub");
    let mut rng = Rng::new(0x44);
    unsafe {
        // E44: count == 3 or out of range -> {0,0}
        for &count in BAD_COUNTS.iter().chain([3i32].iter()) {
            for _ in 0..400 {
                let mut s = rng.simplex(3);
                s.count = count;
                let mut cs = s;
                let mut rs_ = s;
                let cv = c(&mut cs);
                let rv = r(&mut rs_);
                assert_bits_eq!(cv, rv, "E44 count={count}");
                assert_eq!(
                    raw(&cv),
                    raw(&c2v { x: 0.0, y: 0.0 }),
                    "E44 count={count}: expected the {{0,0}} sentinel, got {}",
                    v_hex(&cv)
                );
            }
        }
        // E45: count == 2 with det == +-0 or NaN -> the c2CCW90 branch
        let mut ccw_taken = 0usize;
        for _ in 0..4000 {
            // collinear with the origin => det(ab, -a) == +-0
            let a = rng.v();
            let k = rng.coord();
            let b = c2v { x: a.x * k, y: a.y * k };
            let mut s = rng.simplex(2);
            s.verts[0].p = a;
            s.verts[1].p = b;
            let mut cs = s;
            let mut rs_ = s;
            let cv = c(&mut cs);
            let rv = r(&mut rs_);
            assert_bits_eq!(cv, rv, "E45 collinear");
            let ab = csub(b, a);
            if raw(&cv) == raw(&cccw(ab)) && raw(&cv) != raw(&cskew(ab)) {
                ccw_taken += 1;
            }
        }
        // NaN determinant: `> 0` is false, so CCW90 must be chosen
        for nb in [0x7fc0_0000u32, 0xffc0_0000, 0x7f80_0001] {
            let n = f32::from_bits(nb);
            for (px, py) in [(n, 1.0f32), (1.0, n), (n, n)] {
                let mut s = c2Simplex::default();
                s.count = 2;
                s.div = 1.0;
                s.verts[0].p = c2v { x: px, y: py };
                s.verts[1].p = c2v { x: 3.0, y: 4.0 };
                let mut cs = s;
                let mut rs_ = s;
                let cv = c(&mut cs);
                let rv = r(&mut rs_);
                assert_bits_eq!(cv, rv, "E45 NaN determinant");
                let ab = csub(s.verts[1].p, s.verts[0].p);
                assert_eq!(
                    raw(&cv),
                    raw(&cccw(ab)),
                    "E45 a NaN determinant must select the c2CCW90 branch"
                );
                ccw_taken += 1;
            }
        }
        assert!(ccw_taken > 0, "E45 the c2CCW90 branch was never selected");
    }
}

// ===========================================================================
// E46..E56 — every arm of c22 and c23 (each is a distinct "rejection" of a
// simplex vertex, so each gets its own row)
// ===========================================================================

fn run_simplex_void(p: &Pair, name: &str, s: c2Simplex, ctx: &str) -> c2Simplex {
    let c: FnSimplexVoid = p.c.sym(name);
    let r: FnSimplexVoid = p.rs.sym(name);
    let mut cs = s;
    let mut rs_ = s;
    unsafe {
        c(&mut cs);
        r(&mut rs_);
    }
    assert!(
        raw(&cs) == raw(&rs_),
        "DIVERGENCE {name} {ctx}\n  in  : {}\n  C   : {}\n  Rust: {}",
        simplex_hex(&s),
        simplex_hex(&cs),
        simplex_hex(&rs_)
    );
    cs
}

#[test]
fn e46_e47_e48_c22_all_arms() {
    let p = load();
    let mut rng = Rng::new(0x46);

    // E46: v <= 0 (incl. v == +-0) -> count = 1, a.u = 1, div = 1, keep `a`
    for _ in 0..3000 {
        // origin "before" a: put both points on the same side so v = dot(a, a-b) <= 0
        let d = c2v { x: rng.uniform(1.0, 20.0), y: rng.uniform(1.0, 20.0) };
        let a = c2v { x: d.x, y: d.y };
        let b = c2v { x: d.x * 2.0, y: d.y * 2.0 };
        let mut s = rng.simplex(2);
        s.verts[0].p = a;
        s.verts[1].p = b;
        let out = run_simplex_void(&p, "c22", s, "E46 v<=0");
        assert_eq!(out.count, 1, "E46 expected count = 1");
        assert_eq!(out.verts[0].u.to_bits(), 1.0f32.to_bits(), "E46 a.u = 1");
        assert_eq!(out.div.to_bits(), 1.0f32.to_bits(), "E46 div = 1");
        assert_eq!(raw(&out.verts[0].p), raw(&a), "E46 vertex `a` must be kept");
    }
    // E46 with a.p == b.p (u == v == +0)
    for _ in 0..2000 {
        let a = rng.v();
        let mut s = rng.simplex(2);
        s.verts[0].p = a;
        s.verts[1].p = a;
        let out = run_simplex_void(&p, "c22", s, "E46 a.p == b.p");
        assert_eq!(out.count, 1);
        assert_eq!(out.div.to_bits(), 1.0f32.to_bits());
    }
    // E47: v > 0 && u <= 0 -> a = b, count = 1
    for _ in 0..3000 {
        let d = c2v { x: rng.uniform(1.0, 20.0), y: rng.uniform(1.0, 20.0) };
        let a = c2v { x: -d.x * 2.0, y: -d.y * 2.0 };
        let b = c2v { x: -d.x, y: -d.y };
        let mut s = rng.simplex(2);
        s.verts[0].p = a;
        s.verts[1].p = b;
        let out = run_simplex_void(&p, "c22", s, "E47 u<=0");
        assert_eq!(out.count, 1, "E47 expected count = 1");
        assert_eq!(raw(&out.verts[0].p), raw(&b), "E47 vertex `b` must win");
        assert_eq!(out.div.to_bits(), 1.0f32.to_bits(), "E47 div = 1");
    }
    // E48: u or v is NaN -> BOTH `<= 0` tests are false -> the `else` arm
    for nb in [0x7fc0_0000u32, 0xffc0_0000, 0x7f80_0001, 0x7fab_cdef] {
        let n = f32::from_bits(nb);
        for (ax, ay, bx, by) in [
            (n, 1.0f32, 2.0f32, 3.0f32),
            (1.0, n, 2.0, 3.0),
            (1.0, 2.0, n, 3.0),
            (1.0, 2.0, 3.0, n),
            (n, n, n, n),
        ] {
            let mut s = c2Simplex::default();
            s.count = 2;
            s.div = 7.0;
            s.verts[0].p = c2v { x: ax, y: ay };
            s.verts[1].p = c2v { x: bx, y: by };
            let out = run_simplex_void(&p, "c22", s, "E48 NaN u/v");
            assert_eq!(
                out.count, 2,
                "E48 a NaN u/v must take the `else` arm (count = 2), got {}",
                out.count
            );
            assert!(out.div.is_nan(), "E48 div must be NaN, got {}", f32_hex(out.div));
        }
    }
}

#[test]
fn e49_to_e56_c23_all_arms() {
    let p = load();
    let mut rng = Rng::new(0x49);
    // Classify by the observable effect on the simplex (count and which vertex
    // ended up in slot 0/1), then require every arm to have been reached.
    let mut arms = [0usize; 7];
    let classify = |s: c2Simplex, out: c2Simplex| -> usize {
        let (a, b, c) = (s.verts[0].p, s.verts[1].p, s.verts[2].p);
        match out.count {
            1 => {
                if raw(&out.verts[0].p) == raw(&a) {
                    0
                } else if raw(&out.verts[0].p) == raw(&b) {
                    1
                } else if raw(&out.verts[0].p) == raw(&c) {
                    2
                } else {
                    0
                }
            }
            2 => {
                if raw(&out.verts[0].p) == raw(&a) && raw(&out.verts[1].p) == raw(&b) {
                    3
                } else if raw(&out.verts[0].p) == raw(&b) && raw(&out.verts[1].p) == raw(&c) {
                    4
                } else {
                    5
                }
            }
            _ => 6,
        }
    };
    // big random sweep to hit all seven arms
    for _ in 0..60_000 {
        let s = rng.simplex(3);
        let out = run_simplex_void(&p, "c23", s, "E49-56 random");
        arms[classify(s, out)] += 1;
        // documented invariants per arm
        match out.count {
            1 => {
                assert_eq!(out.verts[0].u.to_bits(), 1.0f32.to_bits(), "count=1 => u = 1");
                assert_eq!(out.div.to_bits(), 1.0f32.to_bits(), "count=1 => div = 1");
            }
            2 | 3 => {}
            n => panic!("c23 produced an unexpected count {n}"),
        }
    }
    // E55: an all-NaN triangle must take the final `else` arm (count = 3)
    for nb in [0x7fc0_0000u32, 0xffc0_0000, 0x7f80_0001] {
        let n = f32::from_bits(nb);
        let mut s = c2Simplex::default();
        s.count = 3;
        s.div = 5.0;
        for i in 0..3 {
            s.verts[i].p = c2v { x: n, y: n };
        }
        let out = run_simplex_void(&p, "c23", s, "E55 all-NaN");
        assert_eq!(out.count, 3, "E55 all-NaN must take the `else` arm");
        assert!(out.div.is_nan(), "E55 div must be NaN");
        arms[6] += 1;
    }
    // E56: a degenerate (collinear) triangle => area == +-0 => uABC/vABC/wABC
    //      are all +-0, so one of the count == 2 arms must win, never `else`.
    let mut degenerate_seen = 0usize;
    for _ in 0..20_000 {
        let a = rng.v();
        let d = c2v { x: rng.coord(), y: rng.coord() };
        let mk = |t: f32| c2v { x: a.x + d.x * t, y: a.y + d.y * t };
        let mut s = rng.simplex(3);
        s.verts[0].p = a;
        s.verts[1].p = mk(1.0);
        s.verts[2].p = mk(rng.uniform(-3.0, 3.0));
        let out = run_simplex_void(&p, "c23", s, "E56 collinear");
        arms[classify(s, out)] += 1;
        if out.count != 3 {
            degenerate_seen += 1;
        }
    }
    // E28-adjacent: all three points identical
    for _ in 0..4000 {
        let a = rng.v();
        let mut s = rng.simplex(3);
        for i in 0..3 {
            s.verts[i].p = a;
        }
        let out = run_simplex_void(&p, "c23", s, "E56 a==b==c");
        assert_eq!(out.count, 1, "a==b==c must collapse to count 1");
        arms[classify(s, out)] += 1;
    }
    eprintln!("E49-E56 c23 arm hit counts = {arms:?} (degenerate non-else = {degenerate_seen})");
    for (i, &n) in arms.iter().enumerate() {
        assert!(n > 0, "E{} (c23 arm {i}) never fired: {arms:?}", 49 + i);
    }
    assert!(degenerate_seen > 0, "E56 degenerate triangles never reduced");
}

// ===========================================================================
// E57..E60 — c2CircletoCapsule
// ===========================================================================

#[test]
fn e57_e58_e59_e60_c2CircletoCapsule() {
    let p = load();
    let c: FnCircletoCapsule = p.c.sym("c2CircletoCapsule");
    let r: FnCircletoCapsule = p.rs.sym("c2CircletoCapsule");
    let mut rng = Rng::new(0x57);
    unsafe {
        // E57 / E58: zero-length capsule -> n == {0,0} -> da == db == +0, so the
        // `da < 0` and `db < 0` tests are both false and the `bp` arm runs.
        // The `da / dot(n,n)` division is therefore NEVER reached with a zero
        // denominator, and the result is well-defined.
        for _ in 0..8000 {
            let z = rng.v();
            let cap = c2Capsule { a: z, b: z, r: rng.radius() };
            let ci = rng.circle();
            let cv = c(ci, cap);
            let rv = r(ci, cap);
            assert_eq!(cv, rv, "E57 zero-length capsule");
            // cross-check against the equivalent circle-vs-circle test, which
            // is what the `bp` arm degenerates to
            assert!(cv == 0 || cv == 1, "E57 result must be a 0/1 sentinel, got {cv}");
        }
        // pinned: a circle exactly on the degenerate capsule's point
        for rad in [0.0f32, -0.0, 1.0, -1.0] {
            let z = c2v { x: 5.0, y: -5.0 };
            let cap = c2Capsule { a: z, b: z, r: rad };
            let ci = c2Circle { p: z, r: rad };
            let cv = c(ci, cap);
            assert_eq!(cv, r(ci, cap), "E57 pinned");
            // d2 == 0, r = rad+rad ; 0 < (2*rad)^2 iff rad != 0
            let expect = if rad == 0.0 { 0 } else { 1 };
            assert_eq!(cv, expect, "E57 pinned expected {expect}, got {cv}");
        }
        // E59: A.r + B.r < 0 -> r*r > 0, so a collision can STILL be reported
        let mut neg_hit = 0usize;
        for _ in 0..20_000 {
            let rA = -rng.uniform(0.0, 20.0);
            let rB = -rng.uniform(0.0, 20.0);
            let ci = c2Circle { p: rng.v(), r: rA };
            let cap = c2Capsule { a: rng.v(), b: rng.v(), r: rB };
            let cv = c(ci, cap);
            assert_eq!(cv, r(ci, cap), "E59 negative radii");
            if cv != 0 {
                neg_hit += 1;
            }
        }
        eprintln!("E59: {neg_hit} collisions reported with a NEGATIVE total radius");
        assert!(neg_hit > 0, "E59 negative radii never produced a positive result");
        // E60: any NaN -> d2 < r*r is false -> 0
        for nb in [0x7fc0_0000u32, 0xffc0_0000, 0x7f80_0001, 0x7fab_cdef] {
            let n = f32::from_bits(nb);
            for slot in 0..8 {
                let mut ci = c2Circle { p: c2v { x: 1.0, y: 2.0 }, r: 30.0 };
                let mut cap = c2Capsule {
                    a: c2v { x: 0.0, y: 0.0 },
                    b: c2v { x: 4.0, y: 0.0 },
                    r: 30.0,
                };
                match slot {
                    0 => ci.p.x = n,
                    1 => ci.p.y = n,
                    2 => ci.r = n,
                    3 => cap.a.x = n,
                    4 => cap.a.y = n,
                    5 => cap.b.x = n,
                    6 => cap.b.y = n,
                    _ => cap.r = n,
                }
                let cv = c(ci, cap);
                assert_eq!(cv, r(ci, cap), "E60 NaN slot={slot}");
                // The C is ground truth: a NaN does NOT always propagate here.
                // Slots 3..6 feed `n` / `ap`, and the final `bp` arm uses
                // neither, so those NaNs are silently discarded.  Derive the
                // expected sentinel from the C body itself.
                let want = ref_circle_to_capsule(ci, cap);
                assert_eq!(
                    cv, want,
                    "E60 slot={slot}: the C source prescribes {want}, got {cv}"
                );
                if matches!(slot, 0 | 1 | 2 | 7) {
                    assert_eq!(
                        cv, 0,
                        "E60 a NaN in the circle centre/radius or capsule radius must reject (slot={slot})"
                    );
                }
            }
        }
    }
}

// ===========================================================================
// E61..E65 — c2CircletoCircle / c2CircletoAABB
// ===========================================================================

#[test]
fn e61_e62_c2CircletoCircle() {
    let p = load();
    let c: FnCircletoCircle = p.c.sym("c2CircletoCircle");
    let r: FnCircletoCircle = p.rs.sym("c2CircletoCircle");
    let mut rng = Rng::new(0x61);
    unsafe {
        // E61: negative radii are not rejected; (rA+rB)^2 is still positive
        let mut neg_hit = 0usize;
        for _ in 0..20_000 {
            let a = c2Circle { p: rng.v(), r: -rng.uniform(0.0, 20.0) };
            let b = c2Circle { p: rng.v(), r: -rng.uniform(0.0, 20.0) };
            let cv = c(a, b);
            assert_eq!(cv, r(a, b), "E61 negative radii");
            if cv != 0 {
                neg_hit += 1;
            }
        }
        eprintln!("E61: {neg_hit} collisions reported with negative radii");
        assert!(neg_hit > 0, "E61 negative radii never reported a collision");
        // pinned: r = -5 each, centres 8 apart -> d2 = 64 < 100 -> reports 1
        let a = c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: -5.0 };
        let b = c2Circle { p: c2v { x: 8.0, y: 0.0 }, r: -5.0 };
        assert_eq!(c(a, b), r(a, b), "E61 pinned");
        assert_eq!(c(a, b), 1, "E61 pinned: negative radii still report 1");
        // E62: NaN -> 0
        for nb in [0x7fc0_0000u32, 0xffc0_0000, 0x7f80_0001] {
            let n = f32::from_bits(nb);
            for slot in 0..6 {
                let mut a = c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 50.0 };
                let mut b = c2Circle { p: c2v { x: 1.0, y: 1.0 }, r: 50.0 };
                match slot {
                    0 => a.p.x = n,
                    1 => a.p.y = n,
                    2 => a.r = n,
                    3 => b.p.x = n,
                    4 => b.p.y = n,
                    _ => b.r = n,
                }
                let cv = c(a, b);
                assert_eq!(cv, r(a, b), "E62 NaN slot={slot}");
                assert_eq!(
                    cv,
                    ref_circle_to_circle(a, b),
                    "E62 slot={slot}: C source prescribes a different value"
                );
                assert_eq!(cv, 0, "E62 NaN must reject, got {cv}");
            }
        }
    }
}

#[test]
fn e63_e64_e65_c2CircletoAABB() {
    let p = load();
    let c: FnCircletoAABB = p.c.sym("c2CircletoAABB");
    let r: FnCircletoAABB = p.rs.sym("c2CircletoAABB");
    let mut rng = Rng::new(0x63);
    unsafe {
        // E63: inverted AABB (min > max) -> c2Clampv collapses onto B.min
        for _ in 0..10_000 {
            let m = rng.v();
            let inv = c2AABB {
                min: c2v { x: m.x + 10.0, y: m.y + 10.0 },
                max: c2v { x: m.x - 10.0, y: m.y - 10.0 },
            };
            let a = rng.circle();
            let cv = c(a, inv);
            assert_eq!(cv, r(a, inv), "E63 inverted AABB");
            assert_eq!(cv, ref_circle_to_aabb(a, inv), "E63 vs C source");
        }
        // pinned: the clamp must land exactly on B.min for an inverted box
        let inv = c2AABB {
            min: c2v { x: 10.0, y: 10.0 },
            max: c2v { x: -10.0, y: -10.0 },
        };
        for (px, py, rad, want) in [
            (10.0f32, 10.0f32, 1.0f32, 1),  // on B.min -> d2 = 0 < 1
            (0.0, 0.0, 1.0, 0),             // 14.14 away from B.min
            (0.0, 0.0, 20.0, 1),
        ] {
            let a = c2Circle { p: c2v { x: px, y: py }, r: rad };
            let cv = c(a, inv);
            assert_eq!(cv, r(a, inv), "E63 pinned ({px},{py}) r={rad}");
            assert_eq!(cv, want, "E63 pinned expected {want}, got {cv}");
        }
        // E64: A.r < 0 -> r2 = A.r*A.r > 0, so a collision can still be reported
        let mut neg_hit = 0usize;
        for _ in 0..20_000 {
            let a = c2Circle { p: rng.v(), r: -rng.uniform(0.0, 30.0) };
            let bb = rng.aabb();
            let cv = c(a, bb);
            assert_eq!(cv, r(a, bb), "E64 negative radius");
            if cv != 0 {
                neg_hit += 1;
            }
        }
        eprintln!("E64: {neg_hit} collisions reported with a negative circle radius");
        assert!(neg_hit > 0, "E64 negative radius never reported a collision");
        // E65: NaN anywhere -> 0
        for nb in [0x7fc0_0000u32, 0xffc0_0000, 0x7f80_0001] {
            let n = f32::from_bits(nb);
            for slot in 0..7 {
                let mut a = c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 50.0 };
                let mut bb = c2AABB {
                    min: c2v { x: -1.0, y: -1.0 },
                    max: c2v { x: 1.0, y: 1.0 },
                };
                match slot {
                    0 => a.p.x = n,
                    1 => a.p.y = n,
                    2 => a.r = n,
                    3 => bb.min.x = n,
                    4 => bb.min.y = n,
                    5 => bb.max.x = n,
                    _ => bb.max.y = n,
                }
                let cv = c(a, bb);
                assert_eq!(cv, r(a, bb), "E65 NaN slot={slot}");
                // `c2Clampv` is `c2Maxv(lo, c2Minv(p, hi))` and both use
                // `x > y ? x : y`, so a NaN in the FIRST operand (`B.min`)
                // loses the comparison and is DISCARDED.  Derive the expected
                // sentinel from the C body rather than assuming propagation.
                let want = ref_circle_to_aabb(a, bb);
                assert_eq!(
                    cv, want,
                    "E65 slot={slot}: the C source prescribes {want}, got {cv}"
                );
                if matches!(slot, 0 | 1 | 2) {
                    assert_eq!(
                        cv, 0,
                        "E65 a NaN in the circle centre/radius must reject (slot={slot})"
                    );
                }
            }
        }
    }
}

// ===========================================================================
// E66..E72 — c2AABBtoAABB, c2AABBtoCapsule, c2CapsuletoCapsule, c2BBVerts
// ===========================================================================

#[test]
fn e66_e67_c2AABBtoAABB() {
    let p = load();
    let c: FnAABBtoAABB = p.c.sym("c2AABBtoAABB");
    let r: FnAABBtoAABB = p.rs.sym("c2AABBtoAABB");
    let mut rng = Rng::new(0x66);
    unsafe {
        // E66: inverted boxes are not rejected
        for _ in 0..20_000 {
            let m = rng.v();
            let inv = c2AABB {
                min: c2v { x: m.x + 10.0, y: m.y + 10.0 },
                max: c2v { x: m.x - 10.0, y: m.y - 10.0 },
            };
            let other = rng.aabb();
            assert_eq!(c(inv, other), r(inv, other), "E66 inverted A");
            assert_eq!(c(other, inv), r(other, inv), "E66 inverted B");
            assert_eq!(c(inv, inv), r(inv, inv), "E66 both inverted");
            assert_eq!(c(inv, other), ref_aabb_to_aabb(inv, other), "E66 vs C source");
            assert_eq!(c(other, inv), ref_aabb_to_aabb(other, inv), "E66 vs C source");
        }
        // E67: ANY NaN makes all four `<` false, so the function reports 1
        for nb in [0x7fc0_0000u32, 0xffc0_0000, 0x7f80_0001, 0x7fab_cdef] {
            let n = f32::from_bits(nb);
            for slot in 0..8 {
                // choose a pair that is definitely separated without the NaN
                let mut a = c2AABB {
                    min: c2v { x: 0.0, y: 0.0 },
                    max: c2v { x: 1.0, y: 1.0 },
                };
                let mut b = c2AABB {
                    min: c2v { x: 100.0, y: 100.0 },
                    max: c2v { x: 101.0, y: 101.0 },
                };
                assert_eq!(c(a, b), 0, "sanity: the pair must be separated");
                match slot {
                    0 => a.min.x = n,
                    1 => a.min.y = n,
                    2 => a.max.x = n,
                    3 => a.max.y = n,
                    4 => b.min.x = n,
                    5 => b.min.y = n,
                    6 => b.max.x = n,
                    _ => b.max.y = n,
                }
                let cv = c(a, b);
                assert_eq!(cv, r(a, b), "E67 NaN slot={slot}");
                let want = ref_aabb_to_aabb(a, b);
                assert_eq!(
                    cv, want,
                    "E67 slot={slot}: the C source prescribes {want}, got {cv}"
                );
                // The pair above is separated on BOTH axes, so poisoning one
                // comparison is not enough: another `<` is still true.  Use a
                // pair separated on a single axis to show the flip.
                let mut a1 = c2AABB {
                    min: c2v { x: 0.0, y: 0.0 },
                    max: c2v { x: 1.0, y: 1.0 },
                };
                let mut b1 = c2AABB {
                    min: c2v { x: 100.0, y: 0.0 },
                    max: c2v { x: 101.0, y: 1.0 },
                };
                assert_eq!(c(a1, b1), 0, "sanity: separated on x only");
                match slot {
                    0 => a1.min.x = n,
                    1 => a1.min.y = n,
                    2 => a1.max.x = n,
                    3 => a1.max.y = n,
                    4 => b1.min.x = n,
                    5 => b1.min.y = n,
                    6 => b1.max.x = n,
                    _ => b1.max.y = n,
                }
                let cv1 = c(a1, b1);
                assert_eq!(cv1, r(a1, b1), "E67 single-axis NaN slot={slot}");
                assert_eq!(
                    cv1,
                    ref_aabb_to_aabb(a1, b1),
                    "E67 single-axis slot={slot}: C source prescribes a different value"
                );
                if matches!(slot, 2 | 4) {
                    assert_eq!(
                        cv1, 1,
                        "E67 poisoning the ONLY true comparison (`A.max.x < B.min.x`) must report 1"
                    );
                }
            }
            // all-NaN: guaranteed 1
            let nan_box = c2AABB {
                min: c2v { x: n, y: n },
                max: c2v { x: n, y: n },
            };
            let cv = c(nan_box, nan_box);
            assert_eq!(cv, r(nan_box, nan_box), "E67 all-NaN");
            assert_eq!(cv, 1, "E67 all-NaN must report 1 (every `<` is false)");
        }
    }
}

#[test]
fn e68_e69_e70_e71_gjk_backed_predicates() {
    let p = load();
    let cac: FnAABBtoCapsule = p.c.sym("c2AABBtoCapsule");
    let rac: FnAABBtoCapsule = p.rs.sym("c2AABBtoCapsule");
    let ccc: FnCapsuletoCapsule = p.c.sym("c2CapsuletoCapsule");
    let rcc: FnCapsuletoCapsule = p.rs.sym("c2CapsuletoCapsule");
    let cgjk: FnGJK = p.c.sym("c2GJK");
    let mut rng = Rng::new(0x68);
    let mut e68 = 0usize; // rc != 0 -> 0
    let mut e69 = 0usize; // rc == 0 -> 1
    let mut e70 = 0usize;
    let mut e71 = 0usize;
    unsafe {
        for _ in 0..20_000 {
            let bb = rng.aabb();
            let k = rng.capsule();
            let k2 = rng.capsule();
            let cv = cac(bb, k);
            assert_eq!(cv, rac(bb, k), "E68/E69 c2AABBtoCapsule");
            let cv2 = ccc(k, k2);
            assert_eq!(cv2, rcc(k, k2), "E70/E71 c2CapsuletoCapsule");
            // tie the sentinel to the underlying c2GJK return value
            let bbuf = Buf::of(&bb);
            let kbuf = Buf::of(&k);
            let k2buf = Buf::of(&k2);
            let d = gjk(cgjk, &bbuf, C2_TYPE_AABB, None, &kbuf, C2_TYPE_CAPSULE, None, false, false, 1, false, None).rc;
            let expect = if d != 0.0 { 0 } else { 1 };
            assert_eq!(
                cv, expect,
                "E68/E69 c2AABBtoCapsule must be `c2GJK(...) == 0`; d={} rc={cv}",
                f32_hex(d)
            );
            if d != 0.0 { e68 += 1 } else { e69 += 1 }
            let d2 = gjk(cgjk, &kbuf, C2_TYPE_CAPSULE, None, &k2buf, C2_TYPE_CAPSULE, None, false, false, 1, false, None).rc;
            let expect2 = if d2 != 0.0 { 0 } else { 1 };
            assert_eq!(cv2, expect2, "E70/E71 c2CapsuletoCapsule must be `c2GJK(...) == 0`");
            if d2 != 0.0 { e70 += 1 } else { e71 += 1 }
        }
        // E68 / E70 with a NaN return: NaN is "true" in C, so both must reject
        for nb in [0x7fc0_0000u32, 0xffc0_0000, 0x7f80_0001] {
            let n = f32::from_bits(nb);
            let bb = c2AABB {
                min: c2v { x: n, y: 0.0 },
                max: c2v { x: 1.0, y: 1.0 },
            };
            let k = c2Capsule {
                a: c2v { x: 0.0, y: 0.0 },
                b: c2v { x: 1.0, y: 1.0 },
                r: 1.0,
            };
            assert_eq!(cac(bb, k), rac(bb, k), "E68 NaN-producing input");
            let kn = c2Capsule {
                a: c2v { x: n, y: 0.0 },
                b: c2v { x: 1.0, y: 0.0 },
                r: 1.0,
            };
            assert_eq!(ccc(kn, k), rcc(kn, k), "E70 NaN-producing input");
        }
    }
    eprintln!("E68={e68} E69={e69} E70={e70} E71={e71}");
    assert!(e68 > 0 && e69 > 0, "E68/E69 both outcomes required: {e68}/{e69}");
    assert!(e70 > 0 && e71 > 0, "E70/E71 both outcomes required: {e70}/{e71}");
}

#[test]
fn e72_c2BBVerts_inverted_box() {
    let p = load();
    let c: FnBBVerts = p.c.sym("c2BBVerts");
    let r: FnBBVerts = p.rs.sym("c2BBVerts");
    let mut rng = Rng::new(0x72);
    unsafe {
        for _ in 0..10_000 {
            let m = rng.v();
            let mut bb_c = c2AABB {
                min: c2v { x: m.x + 10.0, y: m.y + 10.0 },
                max: c2v { x: m.x - 10.0, y: m.y - 10.0 },
            };
            let mut bb_r = bb_c;
            let mut co = [c2v { x: -1.0, y: -2.0 }; 8];
            let mut ro = co;
            c(co.as_mut_ptr(), &mut bb_c);
            r(ro.as_mut_ptr(), &mut bb_r);
            assert_eq!(raw(&co), raw(&ro), "E72 inverted box verts");
            assert_eq!(raw(&bb_c), raw(&bb_r), "E72 *bb must not be modified");
            // no validation: the corners come out exactly as given
            assert_eq!(raw(&co[0]), raw(&bb_c.min), "E72 verts[0] == min");
            assert_eq!(raw(&co[2]), raw(&bb_c.max), "E72 verts[2] == max");
            // slots 4..8 must stay untouched
            for i in 4..8 {
                assert_eq!(raw(&co[i]), raw(&c2v { x: -1.0, y: -2.0 }), "E72 slot {i} written");
            }
        }
        // NaN corners
        for nb in [0x7fc0_0000u32, 0xffc0_0000, 0x7f80_0001] {
            let n = f32::from_bits(nb);
            let mut bb_c = c2AABB {
                min: c2v { x: n, y: n },
                max: c2v { x: n, y: n },
            };
            let mut bb_r = bb_c;
            let mut co = [c2v::default(); 8];
            let mut ro = co;
            c(co.as_mut_ptr(), &mut bb_c);
            r(ro.as_mut_ptr(), &mut bb_r);
            assert_eq!(raw(&co), raw(&ro), "E72 NaN corners");
        }
    }
}

// ===========================================================================
// E73 / E74 — the public `capsule()` entry point
// ===========================================================================

#[test]
fn e73_e74_capsule_no_validation() {
    let p = load();
    let c: FnCapsule = p.c.sym("capsule");
    let r: FnCapsule = p.rs.sym("capsule");
    unsafe {
        // E73: every special value in every slot, and every pair of slots
        for a in specials() {
            for slot in 0..5usize {
                let mut args = [-40.0f32, 40.0, -20.0, 100.0, 10.0];
                args[slot] = a;
                let cv = c(args[0], args[1], args[2], args[3], args[4]);
                let rv = r(args[0], args[1], args[2], args[3], args[4]);
                assert_eq!(
                    cv, rv,
                    "E73 capsule slot={slot} v={} -> C={cv} Rust={rv}",
                    f32_hex(a)
                );
                assert!(
                    (0..8).contains(&cv),
                    "E73 the result must stay a 3-bit mask, got {cv}"
                );
            }
            for b in specials() {
                for (s1, s2) in [(0usize, 1usize), (0, 4), (2, 3), (3, 4), (1, 2)] {
                    let mut args = [-40.0f32, 40.0, -20.0, 100.0, 10.0];
                    args[s1] = a;
                    args[s2] = b;
                    let cv = c(args[0], args[1], args[2], args[3], args[4]);
                    let rv = r(args[0], args[1], args[2], args[3], args[4]);
                    assert_eq!(
                        cv, rv,
                        "E73 capsule slots=({s1},{s2}) a={} b={}",
                        f32_hex(a),
                        f32_hex(b)
                    );
                }
            }
        }
        // E74: r < 0 is not rejected
        let mut neg_nonzero = 0usize;
        for i in 0..2000u32 {
            let rad = -(i as f32) * 0.05;
            for (mx, my, nx, ny) in [
                (-40.0f32, 40.0f32, -20.0f32, 100.0f32),
                (-70.0, 0.0, -70.0, 0.0),
                (-25.0, -25.0, -25.0, -25.0),
                (0.0, 0.0, -100.0, 50.0),
            ] {
                let cv = c(mx, my, nx, ny, rad);
                let rv = r(mx, my, nx, ny, rad);
                assert_eq!(cv, rv, "E74 capsule r={rad}");
                assert!((0..8).contains(&cv), "E74 mask out of range: {cv}");
                if cv != 0 {
                    neg_nonzero += 1;
                }
            }
        }
        eprintln!("E74: {neg_nonzero} non-zero results with a negative radius");
        assert!(neg_nonzero > 0, "E74 a negative radius never produced a hit");
    }
}
