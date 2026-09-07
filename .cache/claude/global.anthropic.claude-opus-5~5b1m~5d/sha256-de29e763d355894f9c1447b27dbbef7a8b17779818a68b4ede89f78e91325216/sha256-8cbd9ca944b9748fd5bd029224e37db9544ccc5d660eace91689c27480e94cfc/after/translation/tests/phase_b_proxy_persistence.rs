//! Phase B — CONFIGS.md row 69: `c2Proxy` slot persistence across `c2GJK` calls.
//!
//! `c2GJK` declares `c2Proxy pA; c2Proxy pB;` as *uninitialised* locals, so they
//! occupy a pair of fixed stack slots that every call from the same call site
//! reuses.  `c2MakeProxy` writes only `verts[0 .. count)`, so a `c2GJKCache`
//! whose `iA`/`iB` name a vertex at or beyond `count` — which the C never
//! validates — reads back whatever an EARLIER call left there.
//!
//! IMPORTANT: the two libraries must be driven in separate, uninterrupted
//! sequences.  Because both `.so`s are called from the same site at the same
//! stack depth, a call into one clobbers the other's slot; interleaving them
//! makes the *C's own* answer change (verified: it collapses to `(0,0)`).  Each
//! run below therefore executes the whole priming + read sequence against one
//! library, records the results, and only then compares.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_int, c_void};

#[repr(C, align(16))]
#[derive(Copy, Clone)]
struct SB([u8; 32]);

fn sb<T: Copy>(v: &T) -> SB {
    let mut b = SB([0u8; 32]);
    unsafe {
        std::ptr::copy_nonoverlapping(
            v as *const T as *const u8,
            b.0.as_mut_ptr(),
            std::mem::size_of::<T>(),
        );
    }
    b
}

#[derive(Copy, Clone, Debug, PartialEq)]
struct Res {
    dist: u32,
    a: (u32, u32),
    b: (u32, u32),
    it: c_int,
    cache: [u8; 36],
}

#[allow(clippy::too_many_arguments)]
unsafe fn call(
    f: FnGJK,
    a: &SB,
    ta: c_int,
    b: &SB,
    tb: c_int,
    ur: c_int,
    cache: Option<&mut c2GJKCache>,
) -> Res {
    let mut oa = c2v { x: 0.0, y: 0.0 };
    let mut ob = c2v { x: 0.0, y: 0.0 };
    let mut it: c_int = -1;
    let cp = match cache {
        Some(c) => c as *mut c2GJKCache,
        None => std::ptr::null_mut(),
    };
    let dist = f(
        a.0.as_ptr() as *const c_void,
        ta,
        std::ptr::null(),
        b.0.as_ptr() as *const c_void,
        tb,
        std::ptr::null(),
        &mut oa,
        &mut ob,
        ur,
        &mut it,
        cp,
    );
    let mut cb = [0u8; 36];
    if !cp.is_null() {
        std::ptr::copy_nonoverlapping(cp as *const u8, cb.as_mut_ptr(), 36);
    }
    Res {
        dist: dist.to_bits(),
        a: (oa.x.to_bits(), oa.y.to_bits()),
        b: (ob.x.to_bits(), ob.y.to_bits()),
        it,
        cache: cb,
    }
}

/// One "scenario": prime the proxy slot with `prime`, then read it back through
/// a cache whose indices point past the second shape pair's vertex count.
/// Returns every observable of every call, in order.
unsafe fn scenario(f: FnGJK, prime: &[(SB, c_int, SB, c_int)], read: &(SB, c_int, SB, c_int), k: c2GJKCache, ur: c_int) -> Vec<Res> {
    let mut out = Vec::new();
    for (a, ta, b, tb) in prime {
        out.push(call(f, a, *ta, b, *tb, ur, None));
    }
    let mut kk = k;
    out.push(call(f, &read.0, read.1, &read.2, read.3, ur, Some(&mut kk)));
    let mut cb = [0u8; 36];
    std::ptr::copy_nonoverlapping(&kk as *const c2GJKCache as *const u8, cb.as_mut_ptr(), 36);
    out.push(Res { dist: 0, a: (0, 0), b: (0, 0), it: 0, cache: cb });
    out
}

#[test]
fn row69_proxy_slot_persists_across_calls() {
    let p = apis();
    let mut rng = Rng::new(0x6900);
    // SCOPE: only vertices that the IMMEDIATELY PRECEDING call actually wrote.
    // Prime with an AABB/AABB pair (c2MakeProxy fills verts[0..4] on both
    // sides), then read back through a CIRCLE/CIRCLE call (count 1) whose cache
    // names vertices 1..3. Those slots were written by the previous call, so the
    // value is fully determined -- unlike slots no preceding call ever wrote,
    // which hold cross-library stack residue (see the module comment).
    for case in 0..n_cases(4000) {
        let pa = sb(&rng.aabb());
        let pb = sb(&rng.aabb());
        let read = (sb(&rng.circle()), C2_TYPE_CIRCLE, sb(&rng.circle()), C2_TYPE_CIRCLE);
        let k = c2GJKCache {
            metric: rng.f32_in(-20.0, 20.0),
            count: 1 + (rng.below(3) as c_int),
            iA: [1 + (rng.below(3) as c_int), 1 + (rng.below(3) as c_int), 1 + (rng.below(3) as c_int)],
            iB: [1 + (rng.below(3) as c_int), 1 + (rng.below(3) as c_int), 1 + (rng.below(3) as c_int)],
            div: rng.f32_in(-4.0, 4.0),
        };
        for &ur in &[0i32, 1] {
            // Each sequence runs to completion against ONE library: prime, then
            // read. The read only touches slots the prime just wrote.
            let run = |api: &Api| unsafe {
                let p0 = call(api.c2GJK, &pa, C2_TYPE_AABB, &pb, C2_TYPE_AABB, ur, None);
                let mut kk = k;
                let p1 = call(api.c2GJK, &read.0, read.1, &read.2, read.3, ur, Some(&mut kk));
                (p0, p1, kk)
            };
            let (c0, c1, ck) = run(&p.c);
            let (r0, r1, rk) = run(&p.r);
            assert_eq!(c0, r0, "priming call diverges (case {case}, ur={ur})");
            assert_eq!(
                c1, r1,
                "stale-proxy read diverges (case {case}, ur={ur}, cache={k:?}).\n\
                 The C reuses one fixed stack slot for `c2Proxy pA/pB`; vertices\n\
                 the previous call wrote must still be visible to this one."
            );
            eq_bytes("row69/cache", &(case, ur), &ck, &rk);
        }
    }
}

/// The stale-proxy read must actually DIFFER from what a zero-initialised proxy
/// would give -- otherwise the test above would pass vacuously and could not
/// detect a regression back to per-call zeroing.
#[test]
fn row69b_stale_read_is_observably_not_zero() {
    let p = apis();
    // Prime with an AABB far from the origin, so its corners are unmistakable.
    let bb1 = c2AABB { min: c2v { x: 100.0, y: 200.0 }, max: c2v { x: 300.0, y: 400.0 } };
    let bb2 = c2AABB { min: c2v { x: -100.0, y: -200.0 }, max: c2v { x: -300.0, y: -400.0 } };
    let ci1 = c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 1.0 };
    let ci2 = c2Circle { p: c2v { x: 5.0, y: 5.0 }, r: 1.0 };
    let (pa, pb) = (sb(&bb1), sb(&bb2));
    let (ra, rb) = (sb(&ci1), sb(&ci2));
    let mut differed = 0;
    for idx in 1..4i32 {
        let k = c2GJKCache { metric: 0.0, count: 1, iA: [idx, 0, 0], iB: [0, 0, 0], div: 1.0 };
        let run = |api: &Api| unsafe {
            call(api.c2GJK, &pa, C2_TYPE_AABB, &pb, C2_TYPE_AABB, 0, None);
            let mut kk = k;
            call(api.c2GJK, &ra, C2_TYPE_CIRCLE, &rb, C2_TYPE_CIRCLE, 0, Some(&mut kk))
        };
        let c = run(&p.c);
        let r = run(&p.r);
        assert_eq!(c, r, "stale-proxy read diverges for iA[0]={idx}");
        // index 0 would give outA == circle centre (0,0); a stale corner must not
        let zero_answer = (0f32.to_bits(), 0f32.to_bits());
        if c.a != zero_answer {
            differed += 1;
        }
    }
    assert!(
        differed > 0,
        "every stale read produced the zero-proxy answer -- the test cannot \
         distinguish persistent slots from per-call zeroing"
    );
}
