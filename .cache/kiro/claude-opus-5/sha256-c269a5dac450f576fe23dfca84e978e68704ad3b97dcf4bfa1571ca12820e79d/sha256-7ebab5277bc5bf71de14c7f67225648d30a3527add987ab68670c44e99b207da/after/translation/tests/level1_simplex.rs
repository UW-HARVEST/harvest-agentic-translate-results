//! Level 1: proxy construction and simplex routines.
//!
//! These take out-pointers, so the whole destination struct is filled with an
//! identical pattern on both sides and compared byte-for-byte afterwards —
//! that also catches any field the C code leaves untouched but Rust writes
//! (or vice versa).

#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::c_int;

const N: usize = 3000;

// c2Proxy / c2Simplex are pure 4-byte fields, so they have no padding and a
// raw byte comparison is well defined. Verified against the C compiler:
//   c2v=8 c2x=16 c2Proxy=72 c2sv=36 c2Simplex=152 c2GJKCache=36 c2Capsule=20
//   c2Simplex offsets: b=36 c=72 d=108 div=144 count=148
const _: () = assert!(std::mem::size_of::<c2Proxy>() == 72);
const _: () = assert!(std::mem::size_of::<c2Simplex>() == 152);
const _: () = assert!(std::mem::size_of::<c2sv>() == 36);
const _: () = assert!(std::mem::size_of::<c2v>() == 8);
const _: () = assert!(std::mem::size_of::<c2x>() == 16);
const _: () = assert!(std::mem::size_of::<c2GJKCache>() == 36);
const _: () = assert!(std::mem::size_of::<c2Capsule>() == 20);
const _: () = assert!(std::mem::offset_of!(c2Simplex, div) == 144);
const _: () = assert!(std::mem::offset_of!(c2Simplex, count) == 148);

fn rand_sv(rng: &mut Rng) -> c2sv {
    c2sv {
        sA: rng.spicy_vec(),
        sB: rng.spicy_vec(),
        p: rng.spicy_vec(),
        u: rng.spicy(),
        iA: (rng.next_u32() % 8) as c_int,
        iB: (rng.next_u32() % 8) as c_int,
    }
}

fn rand_sv_tame(rng: &mut Rng) -> c2sv {
    c2sv {
        sA: rng.vec(),
        sB: rng.vec(),
        p: rng.vec(),
        u: rng.unit(),
        iA: (rng.next_u32() % 8) as c_int,
        iB: (rng.next_u32() % 8) as c_int,
    }
}

fn rand_simplex(rng: &mut Rng, count: c_int, tame: bool) -> c2Simplex {
    let mut s = c2Simplex::default();
    for i in 0..4 {
        s.verts[i] = if tame { rand_sv_tame(rng) } else { rand_sv(rng) };
    }
    s.div = if tame { rng.unit() * 10.0 } else { rng.spicy() };
    s.count = count;
    s
}

/// A simplex whose `p` values are consistent with `sB - sA`, as GJK maintains.
fn rand_simplex_coherent(rng: &mut Rng, count: c_int) -> c2Simplex {
    let mut s = rand_simplex(rng, count, true);
    for i in 0..4 {
        s.verts[i].p = c2v {
            x: s.verts[i].sB.x - s.verts[i].sA.x,
            y: s.verts[i].sB.y - s.verts[i].sA.y,
        };
    }
    s
}

/// Counts covering every switch arm plus out-of-range values that hit
/// `default:` in the C source.
const COUNTS: [c_int; 8] = [0, 1, 2, 3, 4, 5, -1, 100];

// ---------------------------------------------------------------------------
// c2BBVerts
// ---------------------------------------------------------------------------

#[test]
fn t_c2BBVerts() {
    let l = libs();
    let (c, r) = l.sym::<unsafe extern "C" fn(*mut c2v, *mut c2AABB) -> ()>(b"c2BBVerts");
    let mut rng = Rng::new(20);
    for i in 0..N {
        let mut bb = c2AABB {
            min: rng.spicy_vec(),
            max: rng.spicy_vec(),
        };
        // 8 slots, pre-filled identically; only the first 4 must be written.
        let fill = c2v {
            x: f32::from_bits(0xDEAD_BEEF),
            y: f32::from_bits(0xCAFE_BABE),
        };
        let mut co = [fill; 8];
        let mut ro = [fill; 8];
        unsafe { c(co.as_mut_ptr(), &mut bb) };
        unsafe { r(ro.as_mut_ptr(), &mut bb) };
        assert_raw_eq!(co, ro, "i={i} bb={bb:?}");
    }
}

// ---------------------------------------------------------------------------
// c2MakeProxy
// ---------------------------------------------------------------------------

/// Fill pattern for out-params: distinctive, and not a NaN-producing value in
/// any arithmetic we perform on it (it is never read).
fn proxy_fill() -> c2Proxy {
    let mut p = c2Proxy::default();
    p.radius = f32::from_bits(0x1234_5678);
    p.count = 0x7EEE_EEEE;
    for (i, v) in p.verts.iter_mut().enumerate() {
        v.x = f32::from_bits(0x4000_0000 + i as u32);
        v.y = f32::from_bits(0x4100_0000 + i as u32);
    }
    p
}

#[test]
fn t_c2MakeProxy() {
    let l = libs();
    let (c, r) =
        l.sym::<unsafe extern "C" fn(*const u8, c_int, *mut c2Proxy) -> ()>(b"c2MakeProxy");
    let mut rng = Rng::new(21);

    for i in 0..N {
        // Circle
        let circle = c2Circle {
            p: rng.spicy_vec(),
            r: rng.spicy(),
        };
        // AABB
        let bb = c2AABB {
            min: rng.spicy_vec(),
            max: rng.spicy_vec(),
        };
        // Capsule
        let cap = c2Capsule {
            a: rng.spicy_vec(),
            b: rng.spicy_vec(),
            r: rng.spicy(),
        };

        let cases: [(*const u8, c_int, String); 3] = [
            (
                (&circle as *const c2Circle).cast(),
                C2_TYPE_CIRCLE,
                format!("{circle:?}"),
            ),
            ((&bb as *const c2AABB).cast(), C2_TYPE_AABB, format!("{bb:?}")),
            (
                (&cap as *const c2Capsule).cast(),
                C2_TYPE_CAPSULE,
                format!("{cap:?}"),
            ),
        ];
        for (ptr, ty, desc) in cases {
            let mut cp = proxy_fill();
            let mut rp = proxy_fill();
            unsafe { c(ptr, ty, &mut cp) };
            unsafe { r(ptr, ty, &mut rp) };
            assert_raw_eq!(cp, rp, "i={i} ty={ty} shape={desc}");
        }
    }

    // Out-of-range type: the C switch has no default, so the proxy is left
    // completely untouched.
    let shape = c2Circle {
        p: c2v { x: 1.0, y: 2.0 },
        r: 3.0,
    };
    for ty in [3i32, 4, -1, 99] {
        let mut cp = proxy_fill();
        let mut rp = proxy_fill();
        unsafe { c((&shape as *const c2Circle).cast(), ty, &mut cp) };
        unsafe { r((&shape as *const c2Circle).cast(), ty, &mut rp) };
        assert_raw_eq!(cp, rp, "unknown ty={ty}");
        assert_raw_eq!(proxy_fill(), rp, "unknown ty={ty} must be untouched");
    }
}

// ---------------------------------------------------------------------------
// c2GJKSimplexMetric
// ---------------------------------------------------------------------------

#[test]
fn t_c2GJKSimplexMetric() {
    let l = libs();
    let (c, r) = l.sym::<unsafe extern "C" fn(*mut c2Simplex) -> f32>(b"c2GJKSimplexMetric");
    let mut rng = Rng::new(22);
    for &count in COUNTS.iter() {
        for i in 0..N {
            let mut cs = rand_simplex(&mut rng, count, false);
            let mut rs = cs;
            let cv = unsafe { c(&mut cs) };
            let rv = unsafe { r(&mut rs) };
            assert_f32_bits_eq!(cv, rv, "count={count} i={i}");
            // The function must not mutate the simplex.
            assert_raw_eq!(cs, rs, "count={count} i={i} simplex mutated differently");
        }
    }
}

// ---------------------------------------------------------------------------
// c22 / c23
// ---------------------------------------------------------------------------

#[test]
fn t_c22() {
    let l = libs();
    let (c, r) = l.sym::<unsafe extern "C" fn(*mut c2Simplex) -> ()>(b"c22");
    for (seed, tame) in [(23u64, true), (24, false)] {
        let mut rng = Rng::new(seed);
        for &count in COUNTS.iter() {
            for i in 0..N {
                let mut cs = rand_simplex(&mut rng, count, tame);
                let mut rs = cs;
                unsafe { c(&mut cs) };
                unsafe { r(&mut rs) };
                assert_raw_eq!(cs, rs, "tame={tame} count={count} i={i}");
            }
        }
    }
}

#[test]
fn t_c23() {
    let l = libs();
    let (c, r) = l.sym::<unsafe extern "C" fn(*mut c2Simplex) -> ()>(b"c23");
    for (seed, tame) in [(25u64, true), (26, false)] {
        let mut rng = Rng::new(seed);
        for &count in COUNTS.iter() {
            for i in 0..N {
                let mut cs = rand_simplex(&mut rng, count, tame);
                let mut rs = cs;
                unsafe { c(&mut cs) };
                unsafe { r(&mut rs) };
                assert_raw_eq!(cs, rs, "tame={tame} count={count} i={i}");
            }
        }
    }
}

/// `c23` has seven mutually exclusive branches; drive it with coherent
/// simplices so all of them are reached, and record which fired.
#[test]
fn t_c23_branch_coverage() {
    let l = libs();
    let (c, r) = l.sym::<unsafe extern "C" fn(*mut c2Simplex) -> ()>(b"c23");
    let mut rng = Rng::new(27);
    let mut counts_seen = [0usize; 4]; // by resulting s.count
    for i in 0..50_000 {
        let mut cs = rand_simplex_coherent(&mut rng, 3);
        // Scale the points around the origin so the barycentric regions are
        // all reachable.
        let scale = [0.01f32, 0.5, 1.0, 30.0][i % 4];
        for v in cs.verts.iter_mut() {
            v.p.x *= scale;
            v.p.y *= scale;
        }
        let mut rs = cs;
        unsafe { c(&mut cs) };
        unsafe { r(&mut rs) };
        assert_raw_eq!(cs, rs, "i={i}");
        if (0..4).contains(&cs.count) {
            counts_seen[cs.count as usize] += 1;
        }
    }
    assert!(counts_seen[1] > 0, "no count==1 outcome from c23");
    assert!(counts_seen[2] > 0, "no count==2 outcome from c23");
    assert!(counts_seen[3] > 0, "no count==3 outcome from c23");
}

#[test]
fn t_c22_branch_coverage() {
    let l = libs();
    let (c, r) = l.sym::<unsafe extern "C" fn(*mut c2Simplex) -> ()>(b"c22");
    let mut rng = Rng::new(28);
    let mut counts_seen = [0usize; 4];
    for i in 0..20_000 {
        let mut cs = rand_simplex_coherent(&mut rng, 2);
        let mut rs = cs;
        unsafe { c(&mut cs) };
        unsafe { r(&mut rs) };
        assert_raw_eq!(cs, rs, "i={i}");
        if (0..4).contains(&cs.count) {
            counts_seen[cs.count as usize] += 1;
        }
    }
    assert!(counts_seen[1] > 0, "no count==1 outcome from c22");
    assert!(counts_seen[2] > 0, "no count==2 outcome from c22");
}

// ---------------------------------------------------------------------------
// c2D / c2L
// ---------------------------------------------------------------------------

#[test]
fn t_c2D_and_c2L() {
    let l = libs();
    for name in [&b"c2D"[..], &b"c2L"[..]] {
        let (c, r) = l.sym::<unsafe extern "C" fn(*mut c2Simplex) -> c2v>(name);
        for (seed, tame) in [(29u64, true), (30, false)] {
            let mut rng = Rng::new(seed);
            for &count in COUNTS.iter() {
                for i in 0..N {
                    let mut cs = rand_simplex(&mut rng, count, tame);
                    let mut rs = cs;
                    let cv = unsafe { c(&mut cs) };
                    let rv = unsafe { r(&mut rs) };
                    assert_raw_eq!(
                        cv,
                        rv,
                        "{} tame={tame} count={count} i={i}",
                        String::from_utf8_lossy(name)
                    );
                    assert_raw_eq!(cs, rs, "{} mutated", String::from_utf8_lossy(name));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// c2Witness
// ---------------------------------------------------------------------------

#[test]
fn t_c2Witness() {
    let l = libs();
    let (c, r) =
        l.sym::<unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v) -> ()>(b"c2Witness");
    for (seed, tame) in [(31u64, true), (32, false)] {
        let mut rng = Rng::new(seed);
        for &count in COUNTS.iter() {
            for i in 0..N {
                let mut cs = rand_simplex(&mut rng, count, tame);
                let mut rs = cs;
                let fill = c2v {
                    x: f32::from_bits(0x0BAD_F00D),
                    y: f32::from_bits(0x0000_0BAD),
                };
                let (mut ca, mut cb) = (fill, fill);
                let (mut ra, mut rb) = (fill, fill);
                unsafe { c(&mut cs, &mut ca, &mut cb) };
                unsafe { r(&mut rs, &mut ra, &mut rb) };
                assert_raw_eq!(ca, ra, "witness a tame={tame} count={count} i={i}");
                assert_raw_eq!(cb, rb, "witness b tame={tame} count={count} i={i}");
                assert_raw_eq!(cs, rs, "witness simplex tame={tame} count={count} i={i}");
            }
        }
    }
}

/// `div == 0` makes `den` infinite; make sure both sides propagate that
/// identically for every count.
#[test]
fn t_c2Witness_zero_div() {
    let l = libs();
    let (c, r) =
        l.sym::<unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v) -> ()>(b"c2Witness");
    let (cl, rl) = l.sym::<unsafe extern "C" fn(*mut c2Simplex) -> c2v>(b"c2L");
    let mut rng = Rng::new(33);
    for &count in COUNTS.iter() {
        for div in [0.0f32, -0.0, f32::INFINITY, f32::NEG_INFINITY] {
            for i in 0..200 {
                let mut cs = rand_simplex(&mut rng, count, true);
                cs.div = div;
                let mut rs = cs;
                let (mut ca, mut cb, mut ra, mut rb) = (
                    c2v::default(),
                    c2v::default(),
                    c2v::default(),
                    c2v::default(),
                );
                unsafe { c(&mut cs, &mut ca, &mut cb) };
                unsafe { r(&mut rs, &mut ra, &mut rb) };
                assert!(
                    c2v_eq_nan_ok(ca, ra) && c2v_eq_nan_ok(cb, rb),
                    "div={div} count={count} i={i}: C=({ca:?},{cb:?}) Rust=({ra:?},{rb:?})"
                );

                let mut cs2 = cs;
                let mut rs2 = cs;
                let lv = unsafe { cl(&mut cs2) };
                let lr = unsafe { rl(&mut rs2) };
                assert!(
                    c2v_eq_nan_ok(lv, lr),
                    "c2L div={div} count={count} i={i}: C={lv:?} Rust={lr:?}"
                );
            }
        }
    }
}
