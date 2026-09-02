//! Aliasing differential tests.
//!
//! Two C functions take an output pointer AND an input pointer and interleave
//! reads of the input with writes to the output:
//!
//! ```c
//! void c2BBVerts(c2v *out, c2AABB *bb) {
//!     out[0] = bb->min;
//!     out[1] = c2V(bb->max.x, bb->min.y);   /* may already see out[0]'s write */
//!     out[2] = bb->max;
//!     out[3] = c2V(bb->min.x, bb->max.y);
//! }
//!
//! void c2Witness(c2Simplex *s, c2v *a, c2v *b) {
//!     ...
//!     *a = ...;      /* sequenced BEFORE */
//!     *b = ... s->a.sB ...;   /* these reads */
//! }
//! ```
//!
//! Neither has a restrict qualifier, so an overlapping call is a perfectly
//! well-defined C input: the writes are sequenced before the later reads. A Rust
//! translation that snapshots the input struct up front would silently disagree.
//! These tests pin that behaviour down.

mod common;
use common::*;

type FnBBVerts = unsafe extern "C" fn(*mut C2v, *mut C2AABB);
type FnWitness = unsafe extern "C" fn(*mut C2Simplex, *mut C2v, *mut C2v);

/// `c2BBVerts` called with `out` pointing at the AABB itself: `out[0] = bb->min`
/// is a no-op, but `out[1] = (bb->max.x, bb->min.y)` OVERWRITES `bb->max`, so the
/// later `out[2] = bb->max` must observe the new value.
#[test]
fn bbverts_output_aliasing_input() {
    let l = libs();
    let (c, r) = l.pair::<FnBBVerts>("c2BBVerts");
    let mut rng = Rng::new(0xA1_0001);

    // 4 c2v of output overlapping the 2 c2v of the AABB, at every offset that
    // keeps the writes in bounds.
    for shift in 0..1usize {
        let mut cases: Vec<C2AABB> = degenerate_aabbs();
        for _ in 0..4000 {
            cases.push(rng.aabb());
        }
        for _ in 0..2000 {
            cases.push(C2AABB { min: rng.v_mixed(), max: rng.v_mixed() });
        }
        for bb in cases {
            // Backing store: 4 c2v; the AABB lives in the first two slots, and
            // `out` points at the same place.
            let mut cbuf = [C2v { x: f32::from_bits(0x5eed_0001), y: f32::from_bits(0x5eed_0002) }; 4];
            let mut rbuf = cbuf;
            cbuf[0] = bb.min;
            cbuf[1] = bb.max;
            rbuf[0] = bb.min;
            rbuf[1] = bb.max;
            unsafe {
                let cout = cbuf.as_mut_ptr().add(shift);
                let cbbp = cbuf.as_mut_ptr() as *mut C2AABB;
                c(cout, cbbp);
                let rout = rbuf.as_mut_ptr().add(shift);
                let rbbp = rbuf.as_mut_ptr() as *mut C2AABB;
                r(rout, rbbp);
            }
            same("c2BBVerts out aliases bb", &(shift, bb.min, bb.max), &cbuf, &rbuf);
        }
    }
}

/// `c2Witness` with `a` and/or `b` pointing into the simplex it reads. `*a` is
/// sequenced before the operands of `*b` are read, so an aliased `a` changes the
/// value written to `*b`.
#[test]
fn witness_output_aliasing_simplex() {
    let l = libs();
    let (c, r) = l.pair::<FnWitness>("c2Witness");
    let mut rng = Rng::new(0xA1_0002);

    // Byte offsets of every c2v-aligned field inside a c2Simplex that c2Witness
    // reads: verts[k].sA @ 36k, verts[k].sB @ 36k+8.
    let targets: Vec<usize> = (0..3usize).flat_map(|k| [36 * k, 36 * k + 8]).collect();

    for count in [1i32, 2, 3, 0, 4] {
        for &oa in &targets {
            for &ob in &targets {
                for i in 0..40 {
                    let mut s = C2Simplex::default();
                    s.count = count;
                    s.div = if i % 5 == 0 { 0.0 } else { rng.range(0.1, 8.0) };
                    for k in 0..4 {
                        s.v[k] = C2sv {
                            sA: rng.v_tame(),
                            sB: rng.v_tame(),
                            p: rng.v_tame(),
                            u: rng.range(-4.0, 4.0),
                            iA: k as i32,
                            iB: 3 - k as i32,
                        };
                    }
                    let mut cs = s;
                    let mut rs = s;
                    unsafe {
                        let cp = &mut cs as *mut C2Simplex as *mut u8;
                        let rp = &mut rs as *mut C2Simplex as *mut u8;
                        c(
                            &mut cs,
                            cp.add(oa) as *mut C2v,
                            cp.add(ob) as *mut C2v,
                        );
                        r(
                            &mut rs,
                            rp.add(oa) as *mut C2v,
                            rp.add(ob) as *mut C2v,
                        );
                    }
                    same(
                        "c2Witness a/b alias the simplex",
                        &(count, oa, ob, s.div),
                        &cs,
                        &rs,
                    );
                }
            }
        }
    }
}

/// `c2Witness` with `a` and `b` pointing at the SAME c2v: the second write wins.
#[test]
fn witness_a_and_b_alias_each_other() {
    let l = libs();
    let (c, r) = l.pair::<FnWitness>("c2Witness");
    let mut rng = Rng::new(0xA1_0003);

    for count in [0i32, 1, 2, 3, 4, 99, -1] {
        for i in 0..2000 {
            let mut s = C2Simplex::default();
            s.count = count;
            s.div = if i % 7 == 0 { 0.0 } else { rng.range(-8.0, 8.0) };
            for k in 0..4 {
                s.v[k].sA = rng.v_mixed();
                s.v[k].sB = rng.v_mixed();
                s.v[k].u = rng.f32_mixed();
            }
            let mut cv = C2v { x: f32::from_bits(0x5eed_dead), y: f32::from_bits(0x5eed_beef) };
            let mut rv = cv;
            let mut cs = s;
            let mut rs = s;
            unsafe {
                c(&mut cs, &mut cv, &mut cv);
                r(&mut rs, &mut rv, &mut rv);
            }
            same("c2Witness a == b", &(count, s.div), &cv, &rv);
            same("c2Witness a == b (simplex)", &count, &cs, &rs);
        }
    }
}

/// `c2MakeProxy` with the shape pointer aliasing the proxy: the C writes
/// `p->radius` and `p->count` before reading `c->p` / `c->a` / `c->b`, so the
/// later reads see the earlier writes.
#[test]
fn makeproxy_shape_aliasing_proxy() {
    let l = libs();
    let (c, r) =
        l.pair::<unsafe extern "C" fn(*const std::ffi::c_void, i32, *mut C2Proxy)>("c2MakeProxy");
    let mut rng = Rng::new(0xA1_0004);

    for kind in [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE] {
        for i in 0..3000 {
            let blob = if i % 6 == 0 {
                ShapeBlob::degenerate(kind, i)
            } else {
                ShapeBlob::random(&mut rng, kind)
            };
            // Put the shape at the very start of the proxy, so `shape == p`.
            let mut cp = C2Proxy {
                radius: f32::from_bits(0x5eed_face),
                count: -0x5eed,
                verts: [C2v { x: 1.0, y: 2.0 }; 8],
            };
            unsafe {
                std::ptr::copy_nonoverlapping(
                    blob.bytes.as_ptr(),
                    &mut cp as *mut C2Proxy as *mut u8,
                    20,
                );
            }
            let mut rp = cp;
            unsafe {
                let cptr = &mut cp as *mut C2Proxy;
                c(cptr as *const std::ffi::c_void, kind, cptr);
                let rptr = &mut rp as *mut C2Proxy;
                r(rptr as *const std::ffi::c_void, kind, rptr);
            }
            same("c2MakeProxy shape aliases proxy", &(type_name(kind), i), &cp, &rp);
        }
    }
}

/// `c2GJK` writes `*cache`, then `*outA`, then `*outB`, then `*iterations`, and
/// reads `A`/`B`/`ax_ptr`/`bx_ptr` only at the top. Every overlap of those
/// pointers is a legal call; the write ORDER is what makes the result
/// well-defined, so this pins the order down.
#[test]
fn gjk_out_params_aliasing() {
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0xA1_0005);

    // One buffer holding a cache followed by two c2v and an int, so the
    // out-params can be pointed at overlapping offsets within it.
    #[repr(C, align(8))]
    #[derive(Copy, Clone, Debug)]
    struct Slab {
        cache: C2GJKCache,
        a: C2v,
        b: C2v,
        it: i32,
        pad: i32,
    }

    for (ta, tb) in TYPE_PAIRS {
        for ur in [0, 1] {
            for i in 0..60 {
                let a = ShapeBlob::random(&mut rng, ta);
                let b = ShapeBlob::random(&mut rng, tb);
                let base = Slab {
                    cache: C2GJKCache::default(),
                    a: C2v { x: f32::from_bits(0x5eed_0001), y: f32::from_bits(0x5eed_0002) },
                    b: C2v { x: f32::from_bits(0x5eed_0003), y: f32::from_bits(0x5eed_0004) },
                    it: -12345,
                    pad: 0,
                };
                // c2v-aligned offsets within the slab that the out-params may
                // be aimed at: cache.metric(0), cache.iA(8), cache.iB(20),
                // cache.div/a(32..), b(40), it(48).
                for &(oa, ob, oi) in &[
                    (36usize, 36usize, 44usize), // outA == outB
                    (0, 36, 44),                 // outA over cache.metric/count
                    (32, 36, 44),                // outA over cache.div
                    (36, 0, 44),                 // outB over cache.metric
                    (8, 20, 4),                  // over cache.iA / iB / count
                    (36, 44, 4),                 // iterations over cache.count
                    (36, 44, 48),                // disjoint (control)
                ] {
                    let mut cs = base;
                    let mut rs = base;
                    unsafe {
                        let cp = &mut cs as *mut Slab as *mut u8;
                        let rp = &mut rs as *mut Slab as *mut u8;
                        let cd = cf(
                            a.ptr(), ta, std::ptr::null(),
                            b.ptr(), tb, std::ptr::null(),
                            cp.add(oa) as *mut C2v,
                            cp.add(ob) as *mut C2v,
                            ur,
                            cp.add(oi) as *mut i32,
                            &mut cs.cache,
                        );
                        let rd = rf(
                            a.ptr(), ta, std::ptr::null(),
                            b.ptr(), tb, std::ptr::null(),
                            rp.add(oa) as *mut C2v,
                            rp.add(ob) as *mut C2v,
                            ur,
                            rp.add(oi) as *mut i32,
                            &mut rs.cache,
                        );
                        same_f32("c2GJK aliased out-params (return)", &(oa, ob, oi, i), cd, rd);
                    }
                    // Compare the whole slab: cache, both witnesses, iterations.
                    same("c2GJK aliased out-params (slab cache)", &(oa, ob, oi), &cs.cache, &rs.cache);
                    same("c2GJK aliased out-params (slab a)", &(oa, ob, oi), &cs.a, &rs.a);
                    same("c2GJK aliased out-params (slab b)", &(oa, ob, oi), &cs.b, &rs.b);
                    same_i32("c2GJK aliased out-params (slab it)", &(oa, ob, oi), cs.it, rs.it);
                }
            }
        }
    }
}

/// `c2GJK` with the two shape pointers aliasing each other (A == B), and with a
/// shape pointer aliasing the cache.
#[test]
fn gjk_shape_pointer_aliasing() {
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0xA1_0006);

    for (ta, tb) in TYPE_PAIRS {
        for ur in [0, 1] {
            for i in 0..80 {
                // A single blob interpreted as BOTH shapes at once.
                let blob = if i % 5 == 0 {
                    ShapeBlob::degenerate(ta, i)
                } else {
                    ShapeBlob::random(&mut rng, ta)
                };
                let opts = GjkOpts {
                    use_radius: ur,
                    cache: if i % 2 == 0 { Some(C2GJKCache::default()) } else { None },
                    ..Default::default()
                };
                let mut ca = C2v::default();
                let mut cb = C2v::default();
                let mut ra = C2v::default();
                let mut rb = C2v::default();
                let mut cit = 0i32;
                let mut rit = 0i32;
                let mut cc = opts.cache;
                let mut rc = opts.cache;
                let (cd, rd) = unsafe {
                    (
                        cf(
                            blob.ptr(), ta, std::ptr::null(),
                            blob.ptr(), tb, std::ptr::null(),
                            &mut ca, &mut cb, ur, &mut cit,
                            cc.as_mut().map_or(std::ptr::null_mut(), |c| c as *mut _),
                        ),
                        rf(
                            blob.ptr(), ta, std::ptr::null(),
                            blob.ptr(), tb, std::ptr::null(),
                            &mut ra, &mut rb, ur, &mut rit,
                            rc.as_mut().map_or(std::ptr::null_mut(), |c| c as *mut _),
                        ),
                    )
                };
                let ctx = (type_name(ta), type_name(tb), ur, i);
                same_f32("c2GJK A == B (return)", &ctx, cd, rd);
                same("c2GJK A == B (outA)", &ctx, &ca, &ra);
                same("c2GJK A == B (outB)", &ctx, &cb, &rb);
                same_i32("c2GJK A == B (iters)", &ctx, cit, rit);
                same("c2GJK A == B (cache)", &ctx, &cc, &rc);
            }
        }
    }
}
