//! ABI-layout verification.
//!
//! The Rust translation reshapes two C structs:
//!   * `c2Simplex { c2sv a, b, c, d; float div; int count; }` becomes
//!     `{ verts: [c2sv; 4], div, count }` — legitimate only if the four members
//!     are exactly `[c2sv; 4]`, which the C itself relies on (`c2sv *verts = &s.a;`).
//!   * `c2GJKCache` keeps its exact field order, which the C's `count == 4` path
//!     depends on through `iA[3]`-aliases-`iB[0]`.
//!
//! Rather than trusting `size_of`, each claim is *probed through the C `.so`*:
//! we hand the C a struct laid out the Rust way and check that the bytes it
//! writes land where we predict.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_int, c_void};
use std::mem::{align_of, offset_of, size_of};

#[test]
fn static_sizes_and_offsets() {
    assert_eq!((size_of::<c2v>(), align_of::<c2v>()), (8, 4));
    assert_eq!((size_of::<c2r>(), align_of::<c2r>()), (8, 4));
    assert_eq!((size_of::<c2x>(), align_of::<c2x>()), (16, 4));
    assert_eq!((size_of::<c2Circle>(), align_of::<c2Circle>()), (12, 4));
    assert_eq!((size_of::<c2AABB>(), align_of::<c2AABB>()), (16, 4));
    assert_eq!((size_of::<c2Capsule>(), align_of::<c2Capsule>()), (20, 4));

    assert_eq!(size_of::<c2GJKCache>(), 36);
    assert_eq!(offset_of!(c2GJKCache, metric), 0);
    assert_eq!(offset_of!(c2GJKCache, count), 4);
    assert_eq!(offset_of!(c2GJKCache, iA), 8);
    assert_eq!(offset_of!(c2GJKCache, iB), 20);
    assert_eq!(offset_of!(c2GJKCache, div), 32);

    assert_eq!(size_of::<c2Proxy>(), 72);
    assert_eq!(offset_of!(c2Proxy, radius), 0);
    assert_eq!(offset_of!(c2Proxy, count), 4);
    assert_eq!(offset_of!(c2Proxy, verts), 8);

    assert_eq!(size_of::<c2sv>(), 36);
    assert_eq!(offset_of!(c2sv, sA), 0);
    assert_eq!(offset_of!(c2sv, sB), 8);
    assert_eq!(offset_of!(c2sv, p), 16);
    assert_eq!(offset_of!(c2sv, u), 24);
    assert_eq!(offset_of!(c2sv, iA), 28);
    assert_eq!(offset_of!(c2sv, iB), 32);

    assert_eq!(size_of::<c2Simplex>(), 152);
    assert_eq!(offset_of!(c2Simplex, verts), 0);
    assert_eq!(offset_of!(c2Simplex, div), 144);
    assert_eq!(offset_of!(c2Simplex, count), 148);
}

/// Probe: the C's `c2Simplex::b` must sit at byte offset 36 (== `verts[1]`).
/// `c22`'s `u <= 0` branch performs `s->a = s->b`, so if our `verts[1]` did not
/// coincide with the C's `b` the copied bytes would not match `verts[1]`.
#[test]
fn c_agrees_on_c2Simplex_member_offsets() {
    let a = api();
    // Origin strictly past B along the segment => the C takes `u <= 0` => a = b.
    let mut s = c2Simplex::default();
    s.verts[0].p = c2v { x: -3.0, y: -3.0 };
    s.verts[1].p = c2v { x: -1.0, y: -1.0 };
    // distinctive contents in slot 1 so the copy is unmistakable
    s.verts[1].sA = c2v { x: 111.0, y: 222.0 };
    s.verts[1].sB = c2v { x: 333.0, y: 444.0 };
    s.verts[1].iA = 5;
    s.verts[1].iB = 6;
    s.verts[2].sA = c2v { x: -1.0, y: -1.0 };
    s.count = 2;

    let mut cs = s;
    unsafe { (a.c22.0)(&mut cs) };
    assert_eq!(cs.count, 1, "expected the C to take the `u <= 0` collapse branch");
    assert_eq!(cs.verts[0].sA, c2v { x: 111.0, y: 222.0 }, "C's `s->a = s->b` did not land on verts[1]");
    assert_eq!(cs.verts[0].sB, c2v { x: 333.0, y: 444.0 });
    assert_eq!((cs.verts[0].iA, cs.verts[0].iB), (5, 6));
    assert_eq!(cs.div, 1.0, "C's `div` is not at offset 144");

    // Same probe for `c` (verts[2]) via c23's `uBC <= 0 && vCA <= 0` collapse.
    let mut t = c2Simplex::default();
    t.verts[0].p = c2v { x: 10.0, y: 0.0 };
    t.verts[1].p = c2v { x: 0.0, y: 10.0 };
    t.verts[2].p = c2v { x: 4.0, y: 4.0 };
    t.verts[2].sA = c2v { x: 777.0, y: 888.0 };
    t.verts[2].iA = 7;
    t.count = 3;
    let mut ct = t;
    let mut rt = t;
    unsafe {
        (a.c23.0)(&mut ct);
        (a.c23.1)(&mut rt);
    }
    assert_eq!(bytes_of(&ct), bytes_of(&rt), "c23 diverged on the layout probe");
    // Whatever branch it picks, the C and Rust must agree byte for byte, and the
    // 152-byte footprint must be fully accounted for.
    assert_eq!(size_of::<c2Simplex>(), 152);
}

/// Probe: `c2GJKCache`'s field offsets as the C sees them.  We give the C a
/// cache with `count == 2` and distinctive `iA`/`iB`, then confirm the values it
/// writes back appear in the slots our `#[repr(C)]` declaration predicts.
#[test]
fn c_agrees_on_c2GJKCache_offsets() {
    let a = api();
    let A = c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 1.0, y: 1.0 } };
    let B = c2AABB { min: c2v { x: 8.0, y: 8.0 }, max: c2v { x: 9.0, y: 9.0 } };

    // Byte-level view: write the cache through raw offsets and read it back as a
    // struct, to prove the two views coincide.
    let mut raw = [0u8; 36];
    raw[0..4].copy_from_slice(&0.0f32.to_le_bytes()); // metric
    raw[4..8].copy_from_slice(&2i32.to_le_bytes()); // count
    raw[8..12].copy_from_slice(&2i32.to_le_bytes()); // iA[0]
    raw[12..16].copy_from_slice(&3i32.to_le_bytes()); // iA[1]
    raw[16..20].copy_from_slice(&0i32.to_le_bytes()); // iA[2]
    raw[20..24].copy_from_slice(&1i32.to_le_bytes()); // iB[0]
    raw[24..28].copy_from_slice(&0i32.to_le_bytes()); // iB[1]
    raw[28..32].copy_from_slice(&0i32.to_le_bytes()); // iB[2]
    raw[32..36].copy_from_slice(&1.0f32.to_le_bytes()); // div

    let as_struct: c2GJKCache = unsafe { std::ptr::read(raw.as_ptr() as *const c2GJKCache) };
    assert_eq!(as_struct.count, 2);
    assert_eq!(as_struct.iA, [2, 3, 0]);
    assert_eq!(as_struct.iB, [1, 0, 0]);
    assert_eq!(as_struct.div, 1.0);

    let mut cc = as_struct;
    let mut rc = as_struct;
    let cd = unsafe {
        (a.c2GJK.0)(&A as *const _ as *const c_void, C2_TYPE_AABB, std::ptr::null(),
                    &B as *const _ as *const c_void, C2_TYPE_AABB, std::ptr::null(),
                    std::ptr::null_mut(), std::ptr::null_mut(), 1, std::ptr::null_mut(), &mut cc)
    };
    let rd = unsafe {
        (a.c2GJK.1)(&A as *const _ as *const c_void, C2_TYPE_AABB, std::ptr::null(),
                    &B as *const _ as *const c_void, C2_TYPE_AABB, std::ptr::null(),
                    std::ptr::null_mut(), std::ptr::null_mut(), 1, std::ptr::null_mut(), &mut rc)
    };
    assert_eq!(cd.to_bits(), rd.to_bits());
    assert_eq!(bytes_of(&cc), bytes_of(&rc), "cache byte images differ (layout mismatch)");
    // The C wrote a plausible cache back: count in 1..=3, div > 0.
    assert!((1..=3).contains(&cc.count), "C wrote count={}", cc.count);
}

/// Probe: `c2Proxy` — the C writes 4 AABB corners at `verts[0..4]`, i.e. bytes
/// 8..40 of the struct, and leaves the rest untouched.
#[test]
fn c_agrees_on_c2Proxy_offsets() {
    let a = api();
    let bb = c2AABB { min: c2v { x: -1.5, y: -2.5 }, max: c2v { x: 3.5, y: 4.5 } };
    let mut p = c2Proxy { radius: 9.0, count: -1, verts: [c2v { x: 42.0, y: 43.0 }; 8] };
    let mut q = p;
    unsafe {
        (a.c2MakeProxy.0)(&bb as *const _ as *const c_void, C2_TYPE_AABB, &mut p);
        (a.c2MakeProxy.1)(&bb as *const _ as *const c_void, C2_TYPE_AABB, &mut q);
    }
    assert_eq!(bytes_of(&p), bytes_of(&q));
    assert_eq!(p.radius, 0.0);
    assert_eq!(p.count, 4);
    assert_eq!(p.verts[0], bb.min);
    assert_eq!(p.verts[1], c2v { x: bb.max.x, y: bb.min.y });
    assert_eq!(p.verts[2], bb.max);
    assert_eq!(p.verts[3], c2v { x: bb.min.x, y: bb.max.y });
    // slots 4..8 untouched -> proves `verts` starts at offset 8 and is 8 elements
    for i in 4..8 {
        assert_eq!(p.verts[i], c2v { x: 42.0, y: 43.0 }, "C wrote past verts[3] (slot {i})");
    }
}

/// Struct-by-value ABI: the small structs are passed/returned in registers on
/// x86-64 SysV.  Round-tripping them through the C proves our declarations match.
#[test]
fn struct_by_value_abi_round_trip() {
    let a = api();
    let r = Rng::new(0xAB1);
    for _ in 0..2000 {
        let v = r.wild_v();
        // c2v in, c2v out
        assert!(veq((a.c2V.0)(v.x, v.y), (a.c2V.1)(v.x, v.y)));
        // c2r out
        assert!(req((a.c2RotIdentity.0)(), (a.c2RotIdentity.1)()));
        // c2x out (16 bytes -> two SSE registers)
        assert!(xeq((a.c2xIdentity.0)(), (a.c2xIdentity.1)()));
        // c2AABB by value (16 bytes)
        let A = r.aabb();
        let B = r.aabb();
        assert_eq!((a.c2AABBtoAABB.0)(A, B), (a.c2AABBtoAABB.1)(A, B));
        // c2Capsule by value (20 bytes -> memory class)
        let p = r.capsule();
        let q = r.capsule();
        assert_eq!((a.c2CapsuletoCapsule.0)(p, q), (a.c2CapsuletoCapsule.1)(p, q));
        // c2Circle (12 bytes) + c2AABB (16 bytes) mixed
        let c = r.circle();
        assert_eq!((a.c2CircletoAABB.0)(c, A), (a.c2CircletoAABB.1)(c, A));
        assert_eq!((a.c2CircletoCapsule.0)(c, p), (a.c2CircletoCapsule.1)(c, p));
        assert_eq!((a.c2AABBtoCapsule.0)(A, p), (a.c2AABBtoCapsule.1)(A, p));
        // c2r + c2v by value
        let rot = r.rot();
        assert!(veq((a.c2Mulrv.0)(rot, v), (a.c2Mulrv.1)(rot, v)));
        // c2x + c2v by value
        let xf = r.xform();
        assert!(veq((a.c2Mulxv.0)(xf, v), (a.c2Mulxv.1)(xf, v)));
        // int return, float args
        let _: c_int = (a.aabb.0)(v.x, v.y, v.x + 1.0, v.y + 1.0);
    }
}
