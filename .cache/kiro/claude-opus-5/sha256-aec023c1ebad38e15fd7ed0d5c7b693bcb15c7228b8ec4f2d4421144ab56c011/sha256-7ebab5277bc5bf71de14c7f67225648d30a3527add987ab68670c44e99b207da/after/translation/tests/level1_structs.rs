//! Level 1: functions that take pointers to aggregates — proxy construction,
//! the simplex reduction routines, `c2Support` and `c2Witness`.
//!
//! Both sides get an identically pre-filled output buffer so that fields the C
//! code leaves untouched are still compared (this is how the "unknown
//! `C2_TYPE`" behaviour of `c2MakeProxy` gets pinned down).

#![allow(non_snake_case)]

mod common;

use common::*;

fn n() -> usize {
    common::scale(3000)
}

// ---------------------------------------------------------------------------
// c2BBVerts
// ---------------------------------------------------------------------------

#[test]
fn c2BBVerts_matches() {
    let l = libs();
    let (c, r) = l.pair::<unsafe extern "C" fn(*mut c2v, *mut c2AABB)>("c2BBVerts");
    let mut rng = Rng::new(0x11_0001);
    for i in 0..n() {
        let mut bb = rng.aabb();
        // Sentinel-filled so an unwritten slot is visible.
        let fill = c2v { x: -12345.5, y: 98765.25 };
        let mut cv = [fill; 4];
        let mut rv = [fill; 4];
        let mut bb_c = bb;
        let mut bb_r = bb;
        unsafe {
            c(cv.as_mut_ptr(), &mut bb_c);
            r(rv.as_mut_ptr(), &mut bb_r);
        }
        assert_bytes_eq(&cv, &rv, &format!("c2BBVerts #{i} bb={bb:?} verts"));
        assert_bytes_eq(&bb_c, &bb_r, &format!("c2BBVerts #{i} input aliasing"));
        let _ = &mut bb;
    }
}

// ---------------------------------------------------------------------------
// c2MakeProxy
// ---------------------------------------------------------------------------

#[test]
fn c2MakeProxy_matches() {
    let l = libs();
    let (c, r) =
        l.pair::<unsafe extern "C" fn(*const std::ffi::c_void, i32, *mut c2Proxy)>("c2MakeProxy");
    let mut rng = Rng::new(0x11_0002);

    // A recognisable pre-fill: whatever the callee does not write must survive
    // identically on both sides.
    let seed_proxy = || c2Proxy {
        radius: -777.5,
        count: -9,
        verts: [c2v { x: 1.5, y: -2.5 }; 8],
    };

    for i in 0..n() {
        // Include out-of-range type tags: the C switch has no `default`.
        let ty = match rng.below(6) {
            0 => C2_TYPE_CIRCLE,
            1 => C2_TYPE_AABB,
            2 => C2_TYPE_CAPSULE,
            3 => 3,
            4 => -1,
            _ => 12345,
        };
        let circle = rng.circle();
        let aabb = rng.aabb();
        let cap = rng.capsule();

        // The shape actually pointed at is chosen independently of `ty` only
        // for the in-range tags; for the bogus tags nothing is dereferenced.
        let (ptr, desc): (*const std::ffi::c_void, String) = match ty {
            C2_TYPE_CIRCLE => (&circle as *const _ as *const _, format!("{circle:?}")),
            C2_TYPE_AABB => (&aabb as *const _ as *const _, format!("{aabb:?}")),
            C2_TYPE_CAPSULE => (&cap as *const _ as *const _, format!("{cap:?}")),
            _ => (&circle as *const _ as *const _, format!("{circle:?}")),
        };

        let mut pc = seed_proxy();
        let mut pr = seed_proxy();
        unsafe {
            c(ptr, ty, &mut pc);
            r(ptr, ty, &mut pr);
        }
        assert_bytes_eq(&pc, &pr, &format!("c2MakeProxy #{i} ty={ty} shape={desc}"));
    }
}

#[test]
fn c2MakeProxy_leaves_unknown_type_untouched() {
    let l = libs();
    let (c, r) =
        l.pair::<unsafe extern "C" fn(*const std::ffi::c_void, i32, *mut c2Proxy)>("c2MakeProxy");
    let circle = c2Circle { p: c2v { x: 3.0, y: 4.0 }, r: 5.0 };
    let seed = c2Proxy { radius: 42.0, count: 7, verts: [c2v { x: 9.0, y: 8.0 }; 8] };
    for ty in [-2147483648i32, -1, 3, 4, 2147483647] {
        let mut pc = seed;
        let mut pr = seed;
        unsafe {
            c(&circle as *const _ as *const _, ty, &mut pc);
            r(&circle as *const _ as *const _, ty, &mut pr);
        }
        assert_bytes_eq(&pc, &pr, &format!("c2MakeProxy ty={ty}"));
        assert_bytes_eq(&pc, &seed, &format!("c2MakeProxy ty={ty} should be a no-op"));
    }
}

// ---------------------------------------------------------------------------
// Simplex readers / mutators
// ---------------------------------------------------------------------------

/// `c2GJKSimplexMetric` — pure reader, returns a float.
#[test]
fn c2GJKSimplexMetric_matches() {
    let l = libs();
    let (c, r) = l.pair::<unsafe extern "C" fn(*mut c2Simplex) -> f32>("c2GJKSimplexMetric");
    let mut rng = Rng::new(0x11_0003);
    for i in 0..n() {
        for count in [-1i32, 0, 1, 2, 3, 4, 99] {
            let s = rng.simplex(count);
            let mut sc = s;
            let mut sr = s;
            let (cv, rv) = unsafe { (c(&mut sc), r(&mut sr)) };
            assert_f32_eq(cv, rv, &format!("c2GJKSimplexMetric #{i} count={count} {s:?}"));
            assert_bytes_eq(&sc, &sr, &format!("c2GJKSimplexMetric #{i} simplex mutation"));
        }
    }
}

/// `c22` and `c23` rewrite the simplex in place.
#[test]
fn simplex_reduction_matches() {
    let l = libs();
    let (c2, r2) = l.pair::<unsafe extern "C" fn(*mut c2Simplex)>("c22");
    let (c3, r3) = l.pair::<unsafe extern "C" fn(*mut c2Simplex)>("c23");
    let mut rng = Rng::new(0x11_0004);
    for i in 0..n() {
        let s = rng.simplex(2);
        let mut sc = s;
        let mut sr = s;
        unsafe {
            c2(&mut sc);
            r2(&mut sr);
        }
        assert_bytes_eq(&sc, &sr, &format!("c22 #{i} in={s:?}"));

        let s = rng.simplex(3);
        let mut sc = s;
        let mut sr = s;
        unsafe {
            c3(&mut sc);
            r3(&mut sr);
        }
        assert_bytes_eq(&sc, &sr, &format!("c23 #{i} in={s:?}"));
    }
}

/// `c22`/`c23` never read `count`, so they are also exercised on simplices with
/// nonsense counts to prove that.
#[test]
fn simplex_reduction_ignores_count() {
    let l = libs();
    let (c2, r2) = l.pair::<unsafe extern "C" fn(*mut c2Simplex)>("c22");
    let (c3, r3) = l.pair::<unsafe extern "C" fn(*mut c2Simplex)>("c23");
    let mut rng = Rng::new(0x11_0005);
    for i in 0..scale(500) {
        for count in [-5i32, 0, 1, 4, 7] {
            let s = rng.simplex(count);
            let (mut a, mut b) = (s, s);
            unsafe {
                c2(&mut a);
                r2(&mut b);
            }
            assert_bytes_eq(&a, &b, &format!("c22 #{i} count={count}"));
            let (mut a, mut b) = (s, s);
            unsafe {
                c3(&mut a);
                r3(&mut b);
            }
            assert_bytes_eq(&a, &b, &format!("c23 #{i} count={count}"));
        }
    }
}

/// `c2D` / `c2L` — readers returning a vector.
#[test]
fn simplex_direction_and_lambda_match() {
    let l = libs();
    for name in ["c2D", "c2L"] {
        let (c, r) = l.pair::<unsafe extern "C" fn(*mut c2Simplex) -> c2v>(name);
        let mut rng = Rng::new(0x11_0006);
        for i in 0..n() {
            for count in [-3i32, 0, 1, 2, 3, 4, 77] {
                let s = rng.simplex(count);
                let (mut a, mut b) = (s, s);
                let (cv, rv) = unsafe { (c(&mut a), r(&mut b)) };
                assert_bytes_eq(&cv, &rv, &format!("{name} #{i} count={count} s={s:?}"));
                assert_bytes_eq(&a, &b, &format!("{name} #{i} count={count} mutation"));
            }
        }
    }
}

/// `c2Witness` writes two out-params.
#[test]
fn c2Witness_matches() {
    let l = libs();
    let (c, r) =
        l.pair::<unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v)>("c2Witness");
    let mut rng = Rng::new(0x11_0007);
    let fill = c2v { x: -4444.0, y: 3333.0 };
    for i in 0..n() {
        for count in [-1i32, 0, 1, 2, 3, 4, 9] {
            let s = rng.simplex(count);
            let (mut sc, mut sr) = (s, s);
            let (mut ac, mut bc) = (fill, fill);
            let (mut ar, mut br) = (fill, fill);
            unsafe {
                c(&mut sc, &mut ac, &mut bc);
                r(&mut sr, &mut ar, &mut br);
            }
            assert_bytes_eq(&ac, &ar, &format!("c2Witness #{i} count={count} a; s={s:?}"));
            assert_bytes_eq(&bc, &br, &format!("c2Witness #{i} count={count} b; s={s:?}"));
            assert_bytes_eq(&sc, &sr, &format!("c2Witness #{i} count={count} mutation"));
        }
    }
}

// ---------------------------------------------------------------------------
// c2Support
// ---------------------------------------------------------------------------

#[test]
fn c2Support_matches() {
    let l = libs();
    let (c, r) = l.pair::<unsafe extern "C" fn(*const c2v, i32, c2v) -> i32>("c2Support");
    let mut rng = Rng::new(0x11_0008);
    for i in 0..n() {
        // Always hand over a full 8-slot buffer so any `count` in 0..=8 is
        // memory-safe on both sides.
        let mut verts = [c2v::default(); 8];
        for v in verts.iter_mut() {
            *v = rng.vec();
        }
        // Duplicated vertices exercise the strict `>` tie-break.
        if rng.below(3) == 0 {
            let (a, b) = (rng.below(8) as usize, rng.below(8) as usize);
            verts[a] = verts[b];
        }
        let count = rng.below(9) as i32;
        let d = rng.vec();
        let (cv, rv) = unsafe { (c(verts.as_ptr(), count, d), r(verts.as_ptr(), count, d)) };
        assert_eq!(
            cv, rv,
            "c2Support #{i} count={count} d={d:?} verts={:?}",
            &verts[..8]
        );
    }
}

#[test]
fn c2Support_tie_break_prefers_first() {
    let l = libs();
    let (c, r) = l.pair::<unsafe extern "C" fn(*const c2v, i32, c2v) -> i32>("c2Support");
    // All four vertices project identically onto d; C keeps index 0.
    let verts = [
        c2v { x: 1.0, y: 0.0 },
        c2v { x: 1.0, y: 5.0 },
        c2v { x: 1.0, y: -5.0 },
        c2v { x: 1.0, y: 100.0 },
    ];
    let d = c2v { x: 1.0, y: 0.0 };
    let (cv, rv) = unsafe { (c(verts.as_ptr(), 4, d), r(verts.as_ptr(), 4, d)) };
    assert_eq!(cv, rv, "c2Support tie-break");
    assert_eq!(cv, 0, "C tie-break should keep the first index");
}
