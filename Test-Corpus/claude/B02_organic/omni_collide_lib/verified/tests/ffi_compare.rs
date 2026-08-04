// Integration tests that load both the C and Rust shared libraries via
// libloading and compare their outputs through the FFI boundary.
//
// We never call the Rust functions directly: every Rust function under test is
// accessed via its exported symbol name in the cdylib, exactly like an external
// caller would.

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::os::raw::c_void;
use std::path::PathBuf;

// ---- Mirrored C structs (must be #[repr(C)] to match the C ABI) ----

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct C2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct C2x {
    pub p: C2v,
    pub r: C2r,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct C2Circle {
    pub p: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct C2AABB {
    pub min: C2v,
    pub max: C2v,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct C2Capsule {
    pub a: C2v,
    pub b: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct C2GJKCache {
    pub metric: f32,
    pub count: c_int,
    pub i_a: [c_int; 3],
    pub i_b: [c_int; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct C2sv {
    pub s_a: C2v,
    pub s_b: C2v,
    pub p: C2v,
    pub u: f32,
    pub i_a: c_int,
    pub i_b: c_int,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct C2Simplex {
    pub a: C2sv,
    pub b: C2sv,
    pub c: C2sv,
    pub d: C2sv,
    pub div: f32,
    pub count: c_int,
}

const C2_TYPE_CAPSULE: c_int = 0;
const C2_TYPE_CIRCLE: c_int = 1;
const C2_TYPE_AABB: c_int = 2;

// ---- Library loading helpers ----

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    manifest_dir().join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    // Pick whichever profile has been built.
    let release = manifest_dir().join("target/release/libomni_collide_lib.so");
    let debug = manifest_dir().join("target/debug/libomni_collide_lib.so");
    if release.exists() {
        release
    } else if debug.exists() {
        debug
    } else {
        // Fall back to release path (will produce a clear error).
        release
    }
}

unsafe fn load_libs() -> (Library, Library) {
    let c = Library::new(c_lib_path()).expect("failed to load C .so");
    let r = Library::new(rust_lib_path()).expect("failed to load Rust .so");
    (c, r)
}

unsafe fn sym<'a, T>(lib: &'a Library, name: &[u8]) -> Symbol<'a, T> {
    lib.get(name)
        .unwrap_or_else(|e| panic!("symbol {:?} missing: {}", std::str::from_utf8(name).unwrap_or("?"), e))
}

// Compare floats by exact bit pattern — the task requires byte-identical output.
fn bits_eq_f32(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits()
}

fn assert_f32_bits_eq(a: f32, b: f32, label: &str) {
    assert!(
        bits_eq_f32(a, b),
        "{}: C={:?}/{:#x} != Rust={:?}/{:#x}",
        label,
        a,
        a.to_bits(),
        b,
        b.to_bits()
    );
}

fn assert_v_bits_eq(a: C2v, b: C2v, label: &str) {
    assert_f32_bits_eq(a.x, b.x, &format!("{}.x", label));
    assert_f32_bits_eq(a.y, b.y, &format!("{}.y", label));
}

// ---- Tests ----

#[test]
fn test_c2v() {
    unsafe {
        let (c_lib, r_lib) = load_libs();
        type Fn_ = unsafe extern "C" fn(f32, f32) -> C2v;
        let cf: Symbol<Fn_> = sym(&c_lib, b"c2V\0");
        let rf: Symbol<Fn_> = sym(&r_lib, b"c2V\0");
        for &(x, y) in &[(0.0, 0.0), (1.0, 2.0), (-1.5, 3.25), (f32::NAN, 0.0)] {
            let a = cf(x, y);
            let b = rf(x, y);
            assert_eq!(a.x.to_bits(), b.x.to_bits());
            assert_eq!(a.y.to_bits(), b.y.to_bits());
        }
    }
}

#[test]
fn test_basic_vec_ops() {
    unsafe {
        let (c_lib, r_lib) = load_libs();
        type Fn_vv_v = unsafe extern "C" fn(C2v, C2v) -> C2v;
        type Fn_vv_f = unsafe extern "C" fn(C2v, C2v) -> f32;
        type Fn_vf_v = unsafe extern "C" fn(C2v, f32) -> C2v;
        type Fn_v_v = unsafe extern "C" fn(C2v) -> C2v;
        type Fn_v_f = unsafe extern "C" fn(C2v) -> f32;

        let inputs = [
            (C2v { x: 1.0, y: 2.0 }, C2v { x: 3.0, y: 4.0 }),
            (C2v { x: -1.0, y: 0.0 }, C2v { x: 0.0, y: -1.0 }),
            (C2v { x: 1e6, y: -1e6 }, C2v { x: 1e-6, y: -1e-6 }),
            (C2v { x: 0.0, y: 0.0 }, C2v { x: 0.0, y: 0.0 }),
        ];

        for &(a, b) in &inputs {
            for (name_bytes, label) in [
                (&b"c2Add\0"[..], "Add"),
                (&b"c2Sub\0"[..], "Sub"),
                (&b"c2Maxv\0"[..], "Maxv"),
                (&b"c2Minv\0"[..], "Minv"),
            ] {
                let cf: Symbol<Fn_vv_v> = sym(&c_lib, name_bytes);
                let rf: Symbol<Fn_vv_v> = sym(&r_lib, name_bytes);
                assert_v_bits_eq(cf(a, b), rf(a, b), label);
            }
            for (name_bytes, label) in [
                (&b"c2Dot\0"[..], "Dot"),
                (&b"c2Det2\0"[..], "Det2"),
            ] {
                let cf: Symbol<Fn_vv_f> = sym(&c_lib, name_bytes);
                let rf: Symbol<Fn_vv_f> = sym(&r_lib, name_bytes);
                assert_f32_bits_eq(cf(a, b), rf(a, b), label);
            }
            for s in [-2.0_f32, -0.0, 0.0, 1.0, 2.5, 1e-6] {
                for (name_bytes, label) in [
                    (&b"c2Mulvs\0"[..], "Mulvs"),
                    (&b"c2Div\0"[..], "Div"),
                ] {
                    let cf: Symbol<Fn_vf_v> = sym(&c_lib, name_bytes);
                    let rf: Symbol<Fn_vf_v> = sym(&r_lib, name_bytes);
                    if label == "Div" && s == 0.0 {
                        // Both divide-by-zero — both produce inf, which has a deterministic bit pattern.
                    }
                    assert_v_bits_eq(cf(a, s), rf(a, s), label);
                }
            }
            for (name_bytes, label) in [
                (&b"c2Neg\0"[..], "Neg"),
                (&b"c2Skew\0"[..], "Skew"),
                (&b"c2CCW90\0"[..], "CCW90"),
            ] {
                let cf: Symbol<Fn_v_v> = sym(&c_lib, name_bytes);
                let rf: Symbol<Fn_v_v> = sym(&r_lib, name_bytes);
                assert_v_bits_eq(cf(a), rf(a), label);
            }
            for (name_bytes, label) in [(&b"c2Len\0"[..], "Len")] {
                let cf: Symbol<Fn_v_f> = sym(&c_lib, name_bytes);
                let rf: Symbol<Fn_v_f> = sym(&r_lib, name_bytes);
                assert_f32_bits_eq(cf(a), rf(a), label);
            }
        }
    }
}

#[test]
fn test_c2norm_nonzero() {
    unsafe {
        let (c_lib, r_lib) = load_libs();
        type Fn_v_v = unsafe extern "C" fn(C2v) -> C2v;
        let cf: Symbol<Fn_v_v> = sym(&c_lib, b"c2Norm\0");
        let rf: Symbol<Fn_v_v> = sym(&r_lib, b"c2Norm\0");
        for v in [
            C2v { x: 1.0, y: 0.0 },
            C2v { x: 3.0, y: 4.0 },
            C2v { x: -7.5, y: 0.25 },
            C2v { x: 1e-6, y: 1e-6 },
        ] {
            assert_v_bits_eq(cf(v), rf(v), "Norm");
        }
    }
}

#[test]
fn test_clamp() {
    unsafe {
        let (c_lib, r_lib) = load_libs();
        type Fn_ = unsafe extern "C" fn(C2v, C2v, C2v) -> C2v;
        let cf: Symbol<Fn_> = sym(&c_lib, b"c2Clampv\0");
        let rf: Symbol<Fn_> = sym(&r_lib, b"c2Clampv\0");
        let cases = [
            (C2v { x: 5.0, y: 5.0 }, C2v { x: 0.0, y: 0.0 }, C2v { x: 10.0, y: 10.0 }),
            (C2v { x: -1.0, y: 11.0 }, C2v { x: 0.0, y: 0.0 }, C2v { x: 10.0, y: 10.0 }),
        ];
        for &(v, lo, hi) in &cases {
            assert_v_bits_eq(cf(v, lo, hi), rf(v, lo, hi), "Clampv");
        }
    }
}

#[test]
fn test_rot_identity_and_xidentity() {
    unsafe {
        let (c_lib, r_lib) = load_libs();
        type Fr = unsafe extern "C" fn() -> C2r;
        type Fx = unsafe extern "C" fn() -> C2x;
        let cr: Symbol<Fr> = sym(&c_lib, b"c2RotIdentity\0");
        let rr: Symbol<Fr> = sym(&r_lib, b"c2RotIdentity\0");
        let a = cr();
        let b = rr();
        assert_f32_bits_eq(a.c, b.c, "RotIdentity.c");
        assert_f32_bits_eq(a.s, b.s, "RotIdentity.s");

        let cx: Symbol<Fx> = sym(&c_lib, b"c2xIdentity\0");
        let rx: Symbol<Fx> = sym(&r_lib, b"c2xIdentity\0");
        let a = cx();
        let b = rx();
        assert_v_bits_eq(a.p, b.p, "xIdentity.p");
        assert_f32_bits_eq(a.r.c, b.r.c, "xIdentity.r.c");
        assert_f32_bits_eq(a.r.s, b.r.s, "xIdentity.r.s");
    }
}

#[test]
fn test_mulrv_etc() {
    unsafe {
        let (c_lib, r_lib) = load_libs();
        type Fn_ = unsafe extern "C" fn(C2r, C2v) -> C2v;
        for &name in &[&b"c2Mulrv\0"[..], &b"c2MulrvT\0"[..]] {
            let cf: Symbol<Fn_> = sym(&c_lib, name);
            let rf: Symbol<Fn_> = sym(&r_lib, name);
            for &(a, b) in &[
                (C2r { c: 1.0, s: 0.0 }, C2v { x: 1.0, y: 2.0 }),
                (C2r { c: 0.0, s: 1.0 }, C2v { x: 1.0, y: 2.0 }),
                (C2r { c: 0.5, s: 0.866 }, C2v { x: -3.0, y: 4.0 }),
            ] {
                assert_v_bits_eq(cf(a, b), rf(a, b), "Mulrv*");
            }
        }
    }
}

#[test]
fn test_mulxv() {
    unsafe {
        let (c_lib, r_lib) = load_libs();
        type Fn_ = unsafe extern "C" fn(C2x, C2v) -> C2v;
        let cf: Symbol<Fn_> = sym(&c_lib, b"c2Mulxv\0");
        let rf: Symbol<Fn_> = sym(&r_lib, b"c2Mulxv\0");
        let inputs = [
            (
                C2x {
                    p: C2v { x: 1.0, y: 2.0 },
                    r: C2r { c: 1.0, s: 0.0 },
                },
                C2v { x: 3.0, y: 4.0 },
            ),
            (
                C2x {
                    p: C2v { x: -1.0, y: 0.5 },
                    r: C2r { c: 0.5, s: 0.5 },
                },
                C2v { x: 1.0, y: -2.0 },
            ),
        ];
        for &(a, b) in &inputs {
            assert_v_bits_eq(cf(a, b), rf(a, b), "Mulxv");
        }
    }
}

#[test]
fn test_bbverts_and_makeproxy() {
    unsafe {
        let (c_lib, r_lib) = load_libs();

        // c2BBVerts(c2v *out, c2AABB *bb)
        type Fn_bbverts = unsafe extern "C" fn(*mut C2v, *mut C2AABB);
        let cf: Symbol<Fn_bbverts> = sym(&c_lib, b"c2BBVerts\0");
        let rf: Symbol<Fn_bbverts> = sym(&r_lib, b"c2BBVerts\0");
        let mut bb = C2AABB {
            min: C2v { x: -1.0, y: -2.0 },
            max: C2v { x: 3.0, y: 4.0 },
        };
        let mut c_out = [C2v::default(); 4];
        let mut r_out = [C2v::default(); 4];
        cf(c_out.as_mut_ptr(), &mut bb);
        rf(r_out.as_mut_ptr(), &mut bb);
        for i in 0..4 {
            assert_v_bits_eq(c_out[i], r_out[i], &format!("BBVerts[{}]", i));
        }

        // c2MakeProxy(const void *shape, C2_TYPE type, c2Proxy *p)
        // c2Proxy is { float radius; int count; c2v verts[8]; } -- 4 + 4 + 64 = 72 bytes
        // Use raw bytes to mirror the C layout.
        #[repr(C)]
        struct Proxy {
            radius: f32,
            count: c_int,
            verts: [C2v; 8],
        }
        type Fn_proxy = unsafe extern "C" fn(*const c_void, c_int, *mut Proxy);
        let cf: Symbol<Fn_proxy> = sym(&c_lib, b"c2MakeProxy\0");
        let rf: Symbol<Fn_proxy> = sym(&r_lib, b"c2MakeProxy\0");

        // Circle
        let circ = C2Circle { p: C2v { x: 1.0, y: -1.0 }, r: 0.5 };
        let mut cp = Proxy { radius: 0.0, count: 0, verts: [C2v::default(); 8] };
        let mut rp = Proxy { radius: 0.0, count: 0, verts: [C2v::default(); 8] };
        cf(&circ as *const _ as *const c_void, C2_TYPE_CIRCLE, &mut cp);
        rf(&circ as *const _ as *const c_void, C2_TYPE_CIRCLE, &mut rp);
        assert_f32_bits_eq(cp.radius, rp.radius, "circle proxy.radius");
        assert_eq!(cp.count, rp.count);
        assert_v_bits_eq(cp.verts[0], rp.verts[0], "circle proxy.verts[0]");

        // AABB
        let mut cp = Proxy { radius: 0.0, count: 0, verts: [C2v::default(); 8] };
        let mut rp = Proxy { radius: 0.0, count: 0, verts: [C2v::default(); 8] };
        cf(&bb as *const _ as *const c_void, C2_TYPE_AABB, &mut cp);
        rf(&bb as *const _ as *const c_void, C2_TYPE_AABB, &mut rp);
        assert_f32_bits_eq(cp.radius, rp.radius, "aabb proxy.radius");
        assert_eq!(cp.count, rp.count);
        for i in 0..4 {
            assert_v_bits_eq(cp.verts[i], rp.verts[i], &format!("aabb proxy.verts[{}]", i));
        }

        // Capsule
        let cap = C2Capsule {
            a: C2v { x: 0.0, y: 0.0 },
            b: C2v { x: 1.0, y: 1.0 },
            r: 0.25,
        };
        let mut cp = Proxy { radius: 0.0, count: 0, verts: [C2v::default(); 8] };
        let mut rp = Proxy { radius: 0.0, count: 0, verts: [C2v::default(); 8] };
        cf(&cap as *const _ as *const c_void, C2_TYPE_CAPSULE, &mut cp);
        rf(&cap as *const _ as *const c_void, C2_TYPE_CAPSULE, &mut rp);
        assert_f32_bits_eq(cp.radius, rp.radius, "capsule proxy.radius");
        assert_eq!(cp.count, rp.count);
        assert_v_bits_eq(cp.verts[0], rp.verts[0], "capsule proxy.verts[0]");
        assert_v_bits_eq(cp.verts[1], rp.verts[1], "capsule proxy.verts[1]");
    }
}

#[test]
fn test_support() {
    unsafe {
        let (c_lib, r_lib) = load_libs();
        type Fn_ = unsafe extern "C" fn(*const C2v, c_int, C2v) -> c_int;
        let cf: Symbol<Fn_> = sym(&c_lib, b"c2Support\0");
        let rf: Symbol<Fn_> = sym(&r_lib, b"c2Support\0");
        let verts = [
            C2v { x: -1.0, y: -1.0 },
            C2v { x: 1.0, y: -1.0 },
            C2v { x: 1.0, y: 1.0 },
            C2v { x: -1.0, y: 1.0 },
        ];
        for d in [
            C2v { x: 1.0, y: 0.0 },
            C2v { x: 0.0, y: 1.0 },
            C2v { x: -1.0, y: 0.0 },
            C2v { x: -1.0, y: -1.0 },
        ] {
            assert_eq!(
                cf(verts.as_ptr(), 4, d),
                rf(verts.as_ptr(), 4, d),
                "Support direction {:?}",
                d
            );
        }
    }
}

#[test]
fn test_aabb_to_aabb() {
    unsafe {
        let (c_lib, r_lib) = load_libs();
        type Fn_ = unsafe extern "C" fn(C2AABB, C2AABB) -> c_int;
        let cf: Symbol<Fn_> = sym(&c_lib, b"c2AABBtoAABB\0");
        let rf: Symbol<Fn_> = sym(&r_lib, b"c2AABBtoAABB\0");

        let cases = [
            (
                C2AABB { min: C2v { x: 0.0, y: 0.0 }, max: C2v { x: 1.0, y: 1.0 } },
                C2AABB { min: C2v { x: 0.5, y: 0.5 }, max: C2v { x: 1.5, y: 1.5 } },
            ),
            (
                C2AABB { min: C2v { x: 0.0, y: 0.0 }, max: C2v { x: 1.0, y: 1.0 } },
                C2AABB { min: C2v { x: 2.0, y: 2.0 }, max: C2v { x: 3.0, y: 3.0 } },
            ),
            (
                C2AABB { min: C2v { x: -1.0, y: -1.0 }, max: C2v { x: 1.0, y: 1.0 } },
                C2AABB { min: C2v { x: -0.5, y: -0.5 }, max: C2v { x: 0.5, y: 0.5 } },
            ),
        ];
        for &(a, b) in &cases {
            assert_eq!(cf(a, b), rf(a, b), "AABBtoAABB");
        }
    }
}

#[test]
fn test_circle_to_circle() {
    unsafe {
        let (c_lib, r_lib) = load_libs();
        type Fn_ = unsafe extern "C" fn(C2Circle, C2Circle) -> c_int;
        let cf: Symbol<Fn_> = sym(&c_lib, b"c2CircletoCircle\0");
        let rf: Symbol<Fn_> = sym(&r_lib, b"c2CircletoCircle\0");
        let cases = [
            (C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: 1.0 }, C2Circle { p: C2v { x: 0.5, y: 0.0 }, r: 1.0 }),
            (C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: 1.0 }, C2Circle { p: C2v { x: 3.0, y: 0.0 }, r: 1.0 }),
        ];
        for &(a, b) in &cases {
            assert_eq!(cf(a, b), rf(a, b), "CircletoCircle");
        }
    }
}

#[test]
fn test_circle_to_aabb() {
    unsafe {
        let (c_lib, r_lib) = load_libs();
        type Fn_ = unsafe extern "C" fn(C2Circle, C2AABB) -> c_int;
        let cf: Symbol<Fn_> = sym(&c_lib, b"c2CircletoAABB\0");
        let rf: Symbol<Fn_> = sym(&r_lib, b"c2CircletoAABB\0");
        let cases = [
            (
                C2Circle { p: C2v { x: 0.5, y: 0.5 }, r: 0.25 },
                C2AABB { min: C2v { x: 0.0, y: 0.0 }, max: C2v { x: 1.0, y: 1.0 } },
            ),
            (
                C2Circle { p: C2v { x: 5.0, y: 5.0 }, r: 0.25 },
                C2AABB { min: C2v { x: 0.0, y: 0.0 }, max: C2v { x: 1.0, y: 1.0 } },
            ),
        ];
        for &(a, b) in &cases {
            assert_eq!(cf(a, b), rf(a, b), "CircletoAABB");
        }
    }
}

#[test]
fn test_circle_to_capsule() {
    unsafe {
        let (c_lib, r_lib) = load_libs();
        type Fn_ = unsafe extern "C" fn(C2Circle, C2Capsule) -> c_int;
        let cf: Symbol<Fn_> = sym(&c_lib, b"c2CircletoCapsule\0");
        let rf: Symbol<Fn_> = sym(&r_lib, b"c2CircletoCapsule\0");
        let cases = [
            (
                C2Circle { p: C2v { x: 0.5, y: 0.5 }, r: 0.25 },
                C2Capsule {
                    a: C2v { x: 0.0, y: 0.0 },
                    b: C2v { x: 1.0, y: 1.0 },
                    r: 0.1,
                },
            ),
            (
                C2Circle { p: C2v { x: 5.0, y: 5.0 }, r: 0.25 },
                C2Capsule {
                    a: C2v { x: 0.0, y: 0.0 },
                    b: C2v { x: 1.0, y: 0.0 },
                    r: 0.1,
                },
            ),
        ];
        for &(a, b) in &cases {
            assert_eq!(cf(a, b), rf(a, b), "CircletoCapsule");
        }
    }
}

#[test]
fn test_capsule_capsule_aabb_capsule() {
    unsafe {
        let (c_lib, r_lib) = load_libs();
        type FnCC = unsafe extern "C" fn(C2Capsule, C2Capsule) -> c_int;
        type FnAC = unsafe extern "C" fn(C2AABB, C2Capsule) -> c_int;

        let cf: Symbol<FnCC> = sym(&c_lib, b"c2CapsuletoCapsule\0");
        let rf: Symbol<FnCC> = sym(&r_lib, b"c2CapsuletoCapsule\0");
        let a = C2Capsule { a: C2v { x: 0.0, y: 0.0 }, b: C2v { x: 1.0, y: 0.0 }, r: 0.2 };
        let b = C2Capsule { a: C2v { x: 0.5, y: 0.1 }, b: C2v { x: 1.5, y: 0.1 }, r: 0.2 };
        assert_eq!(cf(a, b), rf(a, b), "CapsuletoCapsule overlap");
        let c = C2Capsule { a: C2v { x: 5.0, y: 5.0 }, b: C2v { x: 6.0, y: 5.0 }, r: 0.2 };
        assert_eq!(cf(a, c), rf(a, c), "CapsuletoCapsule miss");

        let cf: Symbol<FnAC> = sym(&c_lib, b"c2AABBtoCapsule\0");
        let rf: Symbol<FnAC> = sym(&r_lib, b"c2AABBtoCapsule\0");
        let bb = C2AABB { min: C2v { x: 0.0, y: 0.0 }, max: C2v { x: 1.0, y: 1.0 } };
        let cap = C2Capsule { a: C2v { x: 0.5, y: 0.5 }, b: C2v { x: 0.6, y: 0.6 }, r: 0.1 };
        assert_eq!(cf(bb, cap), rf(bb, cap), "AABBtoCapsule overlap");
        let cap2 = C2Capsule { a: C2v { x: 5.0, y: 5.0 }, b: C2v { x: 6.0, y: 6.0 }, r: 0.1 };
        assert_eq!(cf(bb, cap2), rf(bb, cap2), "AABBtoCapsule miss");
    }
}

#[test]
fn test_collided_dispatch() {
    unsafe {
        let (c_lib, r_lib) = load_libs();
        type Fn_ = unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int;
        let cf: Symbol<Fn_> = sym(&c_lib, b"c2Collided\0");
        let rf: Symbol<Fn_> = sym(&r_lib, b"c2Collided\0");

        let circle = C2Circle { p: C2v { x: 0.5, y: 0.5 }, r: 0.4 };
        let aabb = C2AABB { min: C2v { x: 0.0, y: 0.0 }, max: C2v { x: 1.0, y: 1.0 } };
        let cap = C2Capsule { a: C2v { x: 0.0, y: 0.5 }, b: C2v { x: 1.0, y: 0.5 }, r: 0.05 };

        let pairs: &[(C2_PTR, c_int, C2_PTR, c_int)] = &[
            (C2_PTR::Circle(&circle), C2_TYPE_CIRCLE, C2_PTR::AABB(&aabb), C2_TYPE_AABB),
            (C2_PTR::AABB(&aabb), C2_TYPE_AABB, C2_PTR::Circle(&circle), C2_TYPE_CIRCLE),
            (C2_PTR::Capsule(&cap), C2_TYPE_CAPSULE, C2_PTR::AABB(&aabb), C2_TYPE_AABB),
            (C2_PTR::Capsule(&cap), C2_TYPE_CAPSULE, C2_PTR::Capsule(&cap), C2_TYPE_CAPSULE),
            (C2_PTR::Circle(&circle), C2_TYPE_CIRCLE, C2_PTR::Capsule(&cap), C2_TYPE_CAPSULE),
        ];
        for (a, ta, b, tb) in pairs {
            let pa = a.as_ptr();
            let pb = b.as_ptr();
            let ca = cf(pa, *ta, pb, *tb);
            let ra = rf(pa, *ta, pb, *tb);
            assert_eq!(ca, ra, "Collided ({},{})", ta, tb);
        }
    }
}

enum C2_PTR<'a> {
    Circle(&'a C2Circle),
    AABB(&'a C2AABB),
    Capsule(&'a C2Capsule),
}
impl<'a> C2_PTR<'a> {
    fn as_ptr(&self) -> *const c_void {
        match self {
            C2_PTR::Circle(c) => *c as *const _ as *const c_void,
            C2_PTR::AABB(c) => *c as *const _ as *const c_void,
            C2_PTR::Capsule(c) => *c as *const _ as *const c_void,
        }
    }
}

#[test]
fn test_omni_collide_full_matrix() {
    unsafe {
        let (c_lib, r_lib) = load_libs();
        type Fn_ = unsafe extern "C" fn(
            c_int, f32, f32, f32, f32, f32, c_int, f32, f32, f32, f32, f32,
        ) -> c_int;
        let cf: Symbol<Fn_> = sym(&c_lib, b"omni_collide\0");
        let rf: Symbol<Fn_> = sym(&r_lib, b"omni_collide\0");

        let cases: &[(c_int, [f32; 5], c_int, [f32; 5])] = &[
            // circle-circle hit
            (C2_TYPE_CIRCLE, [0.0, 0.0, 1.0, 0.0, 0.0],
             C2_TYPE_CIRCLE, [0.5, 0.0, 1.0, 0.0, 0.0]),
            // circle-circle miss
            (C2_TYPE_CIRCLE, [0.0, 0.0, 1.0, 0.0, 0.0],
             C2_TYPE_CIRCLE, [3.0, 0.0, 1.0, 0.0, 0.0]),
            // aabb-aabb
            (C2_TYPE_AABB, [0.0, 0.0, 1.0, 1.0, 0.0],
             C2_TYPE_AABB, [0.5, 0.5, 1.5, 1.5, 0.0]),
            // capsule-capsule
            (C2_TYPE_CAPSULE, [0.0, 0.0, 1.0, 0.0, 0.2],
             C2_TYPE_CAPSULE, [0.5, 0.1, 1.5, 0.1, 0.2]),
            // circle-aabb
            (C2_TYPE_CIRCLE, [0.5, 0.5, 0.4, 0.0, 0.0],
             C2_TYPE_AABB, [0.0, 0.0, 1.0, 1.0, 0.0]),
            // aabb-circle
            (C2_TYPE_AABB, [0.0, 0.0, 1.0, 1.0, 0.0],
             C2_TYPE_CIRCLE, [0.5, 0.5, 0.4, 0.0, 0.0]),
            // capsule-aabb
            (C2_TYPE_CAPSULE, [0.0, 0.5, 1.0, 0.5, 0.05],
             C2_TYPE_AABB, [0.0, 0.0, 1.0, 1.0, 0.0]),
        ];
        for (ta, av, tb, bv) in cases {
            let c = cf(*ta, av[0], av[1], av[2], av[3], av[4],
                       *tb, bv[0], bv[1], bv[2], bv[3], bv[4]);
            let r = rf(*ta, av[0], av[1], av[2], av[3], av[4],
                       *tb, bv[0], bv[1], bv[2], bv[3], bv[4]);
            assert_eq!(c, r, "omni_collide({},{})", ta, tb);
        }
    }
}

#[test]
fn test_simplex_helpers() {
    // Test c22, c23, c2D, c2L, c2GJKSimplexMetric, c2Witness on simple inputs.
    unsafe {
        let (c_lib, r_lib) = load_libs();

        // We mirror the C c2Simplex layout exactly.
        let make_simplex = |count: c_int, pts: &[(C2v, C2v, C2v, f32)]| -> C2Simplex {
            let mut s = C2Simplex::default();
            for (i, &(s_a, s_b, p, u)) in pts.iter().enumerate() {
                let v = C2sv { s_a, s_b, p, u, i_a: 0, i_b: 0 };
                match i {
                    0 => s.a = v,
                    1 => s.b = v,
                    2 => s.c = v,
                    3 => s.d = v,
                    _ => {}
                }
            }
            s.div = 1.0;
            s.count = count;
            s
        };

        type Fn_metric = unsafe extern "C" fn(*mut C2Simplex) -> f32;
        type Fn_d = unsafe extern "C" fn(*mut C2Simplex) -> C2v;
        type Fn_l = unsafe extern "C" fn(*mut C2Simplex) -> C2v;
        type Fn_c2 = unsafe extern "C" fn(*mut C2Simplex);
        type Fn_witness = unsafe extern "C" fn(*mut C2Simplex, *mut C2v, *mut C2v);

        let cm: Symbol<Fn_metric> = sym(&c_lib, b"c2GJKSimplexMetric\0");
        let rm: Symbol<Fn_metric> = sym(&r_lib, b"c2GJKSimplexMetric\0");
        let cd: Symbol<Fn_d> = sym(&c_lib, b"c2D\0");
        let rd: Symbol<Fn_d> = sym(&r_lib, b"c2D\0");
        let cl: Symbol<Fn_l> = sym(&c_lib, b"c2L\0");
        let rl: Symbol<Fn_l> = sym(&r_lib, b"c2L\0");
        let c22c: Symbol<Fn_c2> = sym(&c_lib, b"c22\0");
        let c22r: Symbol<Fn_c2> = sym(&r_lib, b"c22\0");
        let c23c: Symbol<Fn_c2> = sym(&c_lib, b"c23\0");
        let c23r: Symbol<Fn_c2> = sym(&r_lib, b"c23\0");
        let cw: Symbol<Fn_witness> = sym(&c_lib, b"c2Witness\0");
        let rw: Symbol<Fn_witness> = sym(&r_lib, b"c2Witness\0");

        // Count=1
        let pts = [(C2v { x: 1.0, y: 2.0 }, C2v { x: 3.0, y: 4.0 }, C2v { x: 0.5, y: -0.5 }, 1.0)];
        let mut sc = make_simplex(1, &pts);
        let mut sr = make_simplex(1, &pts);
        assert_f32_bits_eq(cm(&mut sc), rm(&mut sr), "metric c=1");
        assert_v_bits_eq(cd(&mut sc), rd(&mut sr), "D c=1");
        assert_v_bits_eq(cl(&mut sc), rl(&mut sr), "L c=1");
        let mut a_c = C2v::default(); let mut b_c = C2v::default();
        let mut a_r = C2v::default(); let mut b_r = C2v::default();
        cw(&mut sc, &mut a_c, &mut b_c);
        rw(&mut sr, &mut a_r, &mut b_r);
        assert_v_bits_eq(a_c, a_r, "witness a c=1");
        assert_v_bits_eq(b_c, b_r, "witness b c=1");

        // Count=2: simulate two simplex points and call c22
        let pts = [
            (C2v { x: 1.0, y: 2.0 }, C2v { x: 3.0, y: 4.0 }, C2v { x: 0.5, y: -0.5 }, 1.0),
            (C2v { x: 2.0, y: 3.0 }, C2v { x: 4.0, y: 5.0 }, C2v { x: -0.5, y: 0.5 }, 1.0),
        ];
        let mut sc = make_simplex(2, &pts);
        let mut sr = make_simplex(2, &pts);
        c22c(&mut sc);
        c22r(&mut sr);
        assert_eq!(sc.count, sr.count, "c22 count");
        assert_f32_bits_eq(sc.div, sr.div, "c22 div");
        assert_f32_bits_eq(sc.a.u, sr.a.u, "c22 a.u");
        assert_f32_bits_eq(sc.b.u, sr.b.u, "c22 b.u");

        // Count=3: call c23
        let pts = [
            (C2v { x: 0.0, y: 0.0 }, C2v { x: 0.0, y: 0.0 }, C2v { x: -1.0, y: -1.0 }, 1.0),
            (C2v { x: 0.0, y: 0.0 }, C2v { x: 0.0, y: 0.0 }, C2v { x: 1.0, y: -1.0 }, 1.0),
            (C2v { x: 0.0, y: 0.0 }, C2v { x: 0.0, y: 0.0 }, C2v { x: 0.0, y: 1.0 }, 1.0),
        ];
        let mut sc = make_simplex(3, &pts);
        let mut sr = make_simplex(3, &pts);
        c23c(&mut sc);
        c23r(&mut sr);
        assert_eq!(sc.count, sr.count, "c23 count");
        assert_f32_bits_eq(sc.div, sr.div, "c23 div");
    }
}

#[test]
fn test_gjk_distance_and_witness() {
    unsafe {
        let (c_lib, r_lib) = load_libs();
        type Fn_ = unsafe extern "C" fn(
            *const c_void, c_int, *const C2x,
            *const c_void, c_int, *const C2x,
            *mut C2v, *mut C2v,
            c_int, *mut c_int, *mut C2GJKCache,
        ) -> f32;
        let cf: Symbol<Fn_> = sym(&c_lib, b"c2GJK\0");
        let rf: Symbol<Fn_> = sym(&r_lib, b"c2GJK\0");

        let circle = C2Circle { p: C2v { x: 5.0, y: 0.0 }, r: 1.0 };
        let aabb = C2AABB { min: C2v { x: 0.0, y: 0.0 }, max: C2v { x: 1.0, y: 1.0 } };

        for use_radius in [0, 1] {
            let mut a_c = C2v::default(); let mut b_c = C2v::default();
            let mut a_r = C2v::default(); let mut b_r = C2v::default();
            let mut iters_c: c_int = 0;
            let mut iters_r: c_int = 0;
            let dc = cf(
                &circle as *const _ as *const c_void, C2_TYPE_CIRCLE, std::ptr::null(),
                &aabb as *const _ as *const c_void, C2_TYPE_AABB, std::ptr::null(),
                &mut a_c, &mut b_c, use_radius, &mut iters_c, std::ptr::null_mut(),
            );
            let dr = rf(
                &circle as *const _ as *const c_void, C2_TYPE_CIRCLE, std::ptr::null(),
                &aabb as *const _ as *const c_void, C2_TYPE_AABB, std::ptr::null(),
                &mut a_r, &mut b_r, use_radius, &mut iters_r, std::ptr::null_mut(),
            );
            assert_f32_bits_eq(dc, dr, &format!("GJK dist (use_radius={})", use_radius));
            assert_v_bits_eq(a_c, a_r, &format!("GJK outA (use_radius={})", use_radius));
            assert_v_bits_eq(b_c, b_r, &format!("GJK outB (use_radius={})", use_radius));
            assert_eq!(iters_c, iters_r, "GJK iterations (use_radius={})", use_radius);
        }
    }
}

#[test]
fn test_gjk_with_cache() {
    unsafe {
        let (c_lib, r_lib) = load_libs();
        type Fn_ = unsafe extern "C" fn(
            *const c_void, c_int, *const C2x,
            *const c_void, c_int, *const C2x,
            *mut C2v, *mut C2v,
            c_int, *mut c_int, *mut C2GJKCache,
        ) -> f32;
        let cf: Symbol<Fn_> = sym(&c_lib, b"c2GJK\0");
        let rf: Symbol<Fn_> = sym(&r_lib, b"c2GJK\0");

        let cap_a = C2Capsule { a: C2v { x: 0.0, y: 0.0 }, b: C2v { x: 1.0, y: 0.0 }, r: 0.2 };
        let cap_b = C2Capsule { a: C2v { x: 2.0, y: 0.0 }, b: C2v { x: 3.0, y: 0.0 }, r: 0.2 };

        let mut cache_c = C2GJKCache::default();
        let mut cache_r = C2GJKCache::default();

        for _ in 0..3 {
            let mut a_c = C2v::default(); let mut b_c = C2v::default();
            let mut a_r = C2v::default(); let mut b_r = C2v::default();
            let dc = cf(
                &cap_a as *const _ as *const c_void, C2_TYPE_CAPSULE, std::ptr::null(),
                &cap_b as *const _ as *const c_void, C2_TYPE_CAPSULE, std::ptr::null(),
                &mut a_c, &mut b_c, 1, std::ptr::null_mut(), &mut cache_c,
            );
            let dr = rf(
                &cap_a as *const _ as *const c_void, C2_TYPE_CAPSULE, std::ptr::null(),
                &cap_b as *const _ as *const c_void, C2_TYPE_CAPSULE, std::ptr::null(),
                &mut a_r, &mut b_r, 1, std::ptr::null_mut(), &mut cache_r,
            );
            assert_f32_bits_eq(dc, dr, "GJK cached dist");
            assert_v_bits_eq(a_c, a_r, "GJK cached outA");
            assert_v_bits_eq(b_c, b_r, "GJK cached outB");
            assert_eq!(cache_c.count, cache_r.count, "cache.count");
            assert_f32_bits_eq(cache_c.metric, cache_r.metric, "cache.metric");
            assert_f32_bits_eq(cache_c.div, cache_r.div, "cache.div");
            for i in 0..3 {
                assert_eq!(cache_c.i_a[i], cache_r.i_a[i], "cache.iA[{}]", i);
                assert_eq!(cache_c.i_b[i], cache_r.i_b[i], "cache.iB[{}]", i);
            }
        }
    }
}
