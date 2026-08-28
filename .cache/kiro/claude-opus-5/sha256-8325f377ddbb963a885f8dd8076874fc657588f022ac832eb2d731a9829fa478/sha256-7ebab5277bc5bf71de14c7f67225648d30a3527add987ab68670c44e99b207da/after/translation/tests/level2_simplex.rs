//! Level 2: proxy construction, support mapping and the simplex routines.
//!
//! Every function here mutates a caller-supplied struct and leaves some of its
//! fields untouched, so both libraries are handed an identically pre-filled
//! struct and the whole struct is compared afterwards.

#![allow(non_snake_case)]

mod harness;
use harness::*;

const N: u32 = 20_000;

type FnBBVerts = unsafe extern "C" fn(*mut V, *mut AABB);
type FnMakeProxy = unsafe extern "C" fn(*const std::ffi::c_void, i32, *mut Proxy);
type FnSupport = unsafe extern "C" fn(*const V, i32, V) -> i32;
type FnSimplexF = unsafe extern "C" fn(*mut Simplex) -> f32;
type FnSimplexV = unsafe extern "C" fn(*mut Simplex) -> V;
type FnSimplexVoid = unsafe extern "C" fn(*mut Simplex);
type FnWitness = unsafe extern "C" fn(*mut Simplex, *mut V, *mut V);

fn rand_proxy(rng: &mut Rng) -> Proxy {
    let mut p = Proxy {
        radius: rng.float(),
        count: rng.next_u32() as i32,
        verts: [V::default(); 8],
    };
    for v in p.verts.iter_mut() {
        *v = rng.v();
    }
    p
}

fn rand_simplex(rng: &mut Rng) -> Simplex {
    let mut s = Simplex {
        verts: [Sv::default(); 4],
        div: rng.float(),
        count: 0,
    };
    for v in s.verts.iter_mut() {
        v.sA = rng.v();
        v.sB = rng.v();
        v.p = rng.v();
        v.u = rng.float();
        v.iA = rng.below(8) as i32;
        v.iB = rng.below(8) as i32;
    }
    // Mostly legal counts, occasionally an out-of-range one to reach the
    // `default:` arms of the C switches.
    s.count = match rng.below(10) {
        0 => 0,
        1 => 4,
        2 => -1,
        3 => 7,
        n => (n as i32) % 3 + 1,
    };
    s
}

/// A simplex whose `p` values are small and related, so that the sign tests in
/// `c22`/`c23` land in every branch rather than always the same one.
fn rand_simplex_realistic(rng: &mut Rng) -> Simplex {
    let mut s = rand_simplex(rng);
    let scale = match rng.below(4) {
        0 => 1.0e-7,
        1 => 1.0,
        2 => 1.0e3,
        _ => 1.0e-3,
    };
    let base = V {
        x: rng.unit() * scale,
        y: rng.unit() * scale,
    };
    for v in s.verts.iter_mut() {
        v.p = V {
            x: base.x + rng.unit() * scale,
            y: base.y + rng.unit() * scale,
        };
        v.u = rng.unit() * scale;
    }
    s.div = match rng.below(4) {
        0 => 1.0,
        1 => 0.0,
        _ => rng.unit() * scale,
    };
    s.count = (rng.below(3) as i32) + 1;
    s
}

#[test]
fn c2BBVerts_matches() {
    let (c, r) = pair::<FnBBVerts>("c2BBVerts");
    let mut rng = Rng::new(11);
    for _ in 0..volume(N) {
        let mut bb = AABB {
            min: rng.v(),
            max: rng.v(),
        };
        let mut oc = [V::default(); 4];
        let mut or = [V::default(); 4];
        for i in 0..4 {
            let v = rng.v();
            oc[i] = v;
            or[i] = v;
        }
        unsafe { c(oc.as_mut_ptr(), &mut bb) };
        unsafe { r(or.as_mut_ptr(), &mut bb) };
        for i in 0..4 {
            assert_v("c2BBVerts", &(bb, i), oc[i], or[i]);
        }
    }
}

#[test]
fn c2MakeProxy_matches() {
    let (c, r) = pair::<FnMakeProxy>("c2MakeProxy");
    let mut rng = Rng::new(12);
    for _ in 0..volume(N) {
        // Pick a shape and pass an identically pre-filled proxy to both sides;
        // the C switch leaves untouched fields alone, so the full struct must
        // still agree.
        let ty = match rng.below(8) {
            0..=1 => C2_TYPE_CIRCLE,
            2..=3 => C2_TYPE_AABB,
            4..=5 => C2_TYPE_CAPSULE,
            6 => 3,      // no matching case in C
            _ => -7,     // ditto
        };
        let circle = Circle {
            p: rng.v(),
            r: rng.float(),
        };
        let bb = AABB {
            min: rng.v(),
            max: rng.v(),
        };
        let cap = Capsule {
            a: rng.v(),
            b: rng.v(),
            r: rng.float(),
        };
        let shape: *const std::ffi::c_void = match ty {
            C2_TYPE_CIRCLE => &circle as *const Circle as *const _,
            C2_TYPE_AABB => &bb as *const AABB as *const _,
            C2_TYPE_CAPSULE => &cap as *const Capsule as *const _,
            // Unmatched type: C dereferences nothing, so any pointer will do.
            _ => &bb as *const AABB as *const _,
        };
        let base = rand_proxy(&mut rng);
        let mut pc = base;
        let mut pr = base;
        unsafe { c(shape, ty, &mut pc) };
        unsafe { r(shape, ty, &mut pr) };
        assert!(
            proxy_eq(&pc, &pr),
            "c2MakeProxy(type={ty}):\n  C   ={pc:?}\n  Rust={pr:?}"
        );
    }
}

#[test]
fn c2Support_matches() {
    let (c, r) = pair::<FnSupport>("c2Support");
    let mut rng = Rng::new(13);
    for _ in 0..volume(N) {
        let mut verts = [V::default(); 8];
        for v in verts.iter_mut() {
            *v = rng.v();
        }
        let count = match rng.below(12) {
            0 => 0,
            1 => -4,
            n => (n as i32) % 8 + 1,
        };
        let d = rng.v();
        let a = unsafe { c(verts.as_ptr(), count, d) };
        let b = unsafe { r(verts.as_ptr(), count, d) };
        assert_eq!(a, b, "c2Support(count={count}, d={d:?}, verts={verts:?})");
    }
}

/// Vertex sets shaped like the ones the algorithm actually produces (AABB
/// corners, capsule endpoints) with many exactly-equal dot products, which is
/// where the strict `>` in the C loop matters.
#[test]
fn c2Support_matches_for_degenerate_sets() {
    let (c, r) = pair::<FnSupport>("c2Support");
    let mut rng = Rng::new(14);
    for _ in 0..volume(N) {
        let mut verts = [V::default(); 8];
        let q = V {
            x: (rng.below(5) as f32) - 2.0,
            y: (rng.below(5) as f32) - 2.0,
        };
        for v in verts.iter_mut() {
            *v = if rng.below(2) == 0 {
                q
            } else {
                V {
                    x: (rng.below(5) as f32) - 2.0,
                    y: (rng.below(5) as f32) - 2.0,
                }
            };
        }
        let count = (rng.below(8) as i32) + 1;
        let d = V {
            x: (rng.below(5) as f32) - 2.0,
            y: (rng.below(5) as f32) - 2.0,
        };
        let a = unsafe { c(verts.as_ptr(), count, d) };
        let b = unsafe { r(verts.as_ptr(), count, d) };
        assert_eq!(a, b, "c2Support(count={count}, d={d:?}, verts={verts:?})");
    }
}

#[test]
fn c2GJKSimplexMetric_matches() {
    let (c, r) = pair::<FnSimplexF>("c2GJKSimplexMetric");
    let mut rng = Rng::new(15);
    for i in 0..volume(N) {
        let base = if i % 2 == 0 {
            rand_simplex(&mut rng)
        } else {
            rand_simplex_realistic(&mut rng)
        };
        let mut sc = base;
        let mut sr = base;
        let a = unsafe { c(&mut sc) };
        let b = unsafe { r(&mut sr) };
        assert_f("c2GJKSimplexMetric", &base, a, b);
        assert_simplex("c2GJKSimplexMetric (struct untouched)", &base, &sc, &sr);
    }
}

#[test]
fn simplex_returning_vector_fns_match() {
    let mut rng = Rng::new(16);
    let names = ["c2D", "c2L"];
    let fns: Vec<_> = names.iter().map(|n| (n, pair::<FnSimplexV>(n))).collect();
    for i in 0..volume(N) {
        let base = if i % 2 == 0 {
            rand_simplex(&mut rng)
        } else {
            rand_simplex_realistic(&mut rng)
        };
        for (name, (c, r)) in &fns {
            let mut sc = base;
            let mut sr = base;
            let a = unsafe { c(&mut sc) };
            let b = unsafe { r(&mut sr) };
            assert_v(name, &base, a, b);
            assert_simplex(name, &base, &sc, &sr);
        }
    }
}

#[test]
fn c22_and_c23_match() {
    let mut rng = Rng::new(17);
    let names = ["c22", "c23"];
    let fns: Vec<_> = names.iter().map(|n| (n, pair::<FnSimplexVoid>(n))).collect();
    for i in 0..volume(N) {
        let base = if i % 2 == 0 {
            rand_simplex(&mut rng)
        } else {
            rand_simplex_realistic(&mut rng)
        };
        for (name, (c, r)) in &fns {
            let mut sc = base;
            let mut sr = base;
            unsafe { c(&mut sc) };
            unsafe { r(&mut sr) };
            assert_simplex(name, &base, &sc, &sr);
        }
    }
}

#[test]
fn c2Witness_matches() {
    let (c, r) = pair::<FnWitness>("c2Witness");
    let mut rng = Rng::new(18);
    for i in 0..volume(N) {
        let base = if i % 2 == 0 {
            rand_simplex(&mut rng)
        } else {
            rand_simplex_realistic(&mut rng)
        };
        let mut sc = base;
        let mut sr = base;
        let (mut ac, mut bc) = (rng.v(), rng.v());
        let (mut ar, mut br) = (ac, bc);
        unsafe { c(&mut sc, &mut ac, &mut bc) };
        unsafe { r(&mut sr, &mut ar, &mut br) };
        assert_v("c2Witness outA", &base, ac, ar);
        assert_v("c2Witness outB", &base, bc, br);
        assert_simplex("c2Witness (struct untouched)", &base, &sc, &sr);
    }
}
