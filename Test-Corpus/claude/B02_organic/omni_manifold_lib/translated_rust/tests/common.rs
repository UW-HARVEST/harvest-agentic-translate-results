// Common helpers for FFI integration tests.
use libloading::{Library, Symbol};
use std::path::PathBuf;

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct c2h {
    pub n: c2v,
    pub d: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2Poly {
    pub count: i32,
    pub verts: [c2v; 8],
    pub norms: [c2v; 8],
}

impl Default for c2Poly {
    fn default() -> Self {
        c2Poly {
            count: 0,
            verts: [c2v::default(); 8],
            norms: [c2v::default(); 8],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct c2Manifold {
    pub count: i32,
    pub depths: [f32; 2],
    pub contact_points: [c2v; 2],
    pub n: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: i32,
    pub iA: [i32; 3],
    pub iB: [i32; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: f32,
    pub iA: i32,
    pub iB: i32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct c2Simplex {
    pub a: c2sv,
    pub b: c2sv,
    pub c: c2sv,
    pub d: c2sv,
    pub div: f32,
    pub count: i32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: i32,
    pub verts: [c2v; 8],
}

// C2_TYPE matches the C enum
pub const C2_TYPE_CAPSULE: i32 = 0;
pub const C2_TYPE_CIRCLE: i32 = 1;
pub const C2_TYPE_AABB: i32 = 2;
pub const C2_TYPE_POLY: i32 = 3;

pub fn c_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src/build/libtranslated_rust.so");
    p
}

pub fn rust_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/release/libomni_manifold_lib.so");
    p
}

pub fn load_libs() -> (Library, Library) {
    unsafe {
        let c = Library::new(c_lib_path()).expect("failed to load C .so");
        let r = Library::new(rust_lib_path()).expect("failed to load Rust .so");
        (c, r)
    }
}

pub fn assert_v_eq(a: c2v, b: c2v, ctx: &str) {
    assert_eq!(a.x.to_bits(), b.x.to_bits(), "{}: x differs ({} vs {})", ctx, a.x, b.x);
    assert_eq!(a.y.to_bits(), b.y.to_bits(), "{}: y differs ({} vs {})", ctx, a.y, b.y);
}

pub fn assert_f_eq(a: f32, b: f32, ctx: &str) {
    assert_eq!(a.to_bits(), b.to_bits(), "{}: f32 differs ({} vs {})", ctx, a, b);
}

pub fn assert_manifold_eq(a: c2Manifold, b: c2Manifold, ctx: &str) {
    assert_eq!(a.count, b.count, "{}: count differs", ctx);
    assert_f_eq(a.depths[0], b.depths[0], &format!("{}: depths[0]", ctx));
    assert_f_eq(a.depths[1], b.depths[1], &format!("{}: depths[1]", ctx));
    assert_v_eq(a.contact_points[0], b.contact_points[0], &format!("{}: contact_points[0]", ctx));
    assert_v_eq(a.contact_points[1], b.contact_points[1], &format!("{}: contact_points[1]", ctx));
    assert_v_eq(a.n, b.n, &format!("{}: n", ctx));
}

pub fn get<'a, T>(lib: &'a Library, name: &[u8]) -> Symbol<'a, T> {
    unsafe { lib.get(name).unwrap_or_else(|e| panic!("missing symbol {:?}: {}", std::str::from_utf8(name).unwrap(), e)) }
}
