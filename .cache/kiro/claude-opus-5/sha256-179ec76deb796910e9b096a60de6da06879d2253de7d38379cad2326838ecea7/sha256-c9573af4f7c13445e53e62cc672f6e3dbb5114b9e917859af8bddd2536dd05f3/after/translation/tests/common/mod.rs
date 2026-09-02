//! Shared harness: loads BOTH the C `.so` and the Rust `.so` with `libloading`
//! and exposes every exported symbol as a typed function pointer, so all
//! comparisons go through the real dynamic-linking boundary.
#![allow(non_snake_case, non_camel_case_types, dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// C-layout structs (mirrors of c_src/src/lib.c)
// ---------------------------------------------------------------------------

pub type C2_TYPE = i32;
pub const C2_TYPE_CAPSULE: C2_TYPE = 0;
pub const C2_TYPE_CIRCLE: C2_TYPE = 1;
pub const C2_TYPE_AABB: C2_TYPE = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: i32,
    pub iA: [i32; 3],
    pub iB: [i32; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: i32,
    pub verts: [c2v; 8],
}

impl Default for c2Proxy {
    fn default() -> Self {
        c2Proxy {
            radius: 0.0,
            count: 0,
            verts: [c2v::default(); 8],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: f32,
    pub iA: i32,
    pub iB: i32,
}

/// `{ c2sv a, b, c, d; float div; int count; }`
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2Simplex {
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: i32,
}

// ---------------------------------------------------------------------------
// Bitwise equality helpers (NaN payloads and signed zeros must match exactly)
// ---------------------------------------------------------------------------

pub trait Bits {
    fn bits(&self) -> Vec<u32>;
}

impl Bits for f32 {
    fn bits(&self) -> Vec<u32> {
        vec![self.to_bits()]
    }
}
impl Bits for i32 {
    fn bits(&self) -> Vec<u32> {
        vec![*self as u32]
    }
}
impl Bits for c2v {
    fn bits(&self) -> Vec<u32> {
        vec![self.x.to_bits(), self.y.to_bits()]
    }
}
impl Bits for c2r {
    fn bits(&self) -> Vec<u32> {
        vec![self.c.to_bits(), self.s.to_bits()]
    }
}
impl Bits for c2x {
    fn bits(&self) -> Vec<u32> {
        let mut v = self.p.bits();
        v.extend(self.r.bits());
        v
    }
}
impl Bits for c2GJKCache {
    fn bits(&self) -> Vec<u32> {
        let mut v = vec![self.metric.to_bits(), self.count as u32];
        v.extend(self.iA.iter().map(|x| *x as u32));
        v.extend(self.iB.iter().map(|x| *x as u32));
        v.push(self.div.to_bits());
        v
    }
}
impl Bits for c2Proxy {
    fn bits(&self) -> Vec<u32> {
        let mut v = vec![self.radius.to_bits(), self.count as u32];
        for e in self.verts.iter() {
            v.extend(e.bits());
        }
        v
    }
}
impl Bits for c2sv {
    fn bits(&self) -> Vec<u32> {
        let mut v = self.sA.bits();
        v.extend(self.sB.bits());
        v.extend(self.p.bits());
        v.push(self.u.to_bits());
        v.push(self.iA as u32);
        v.push(self.iB as u32);
        v
    }
}
impl Bits for c2Simplex {
    fn bits(&self) -> Vec<u32> {
        let mut v = Vec::new();
        for e in self.verts.iter() {
            v.extend(e.bits());
        }
        v.push(self.div.to_bits());
        v.push(self.count as u32);
        v
    }
}
impl<T: Bits> Bits for Vec<T> {
    fn bits(&self) -> Vec<u32> {
        self.iter().flat_map(|e| e.bits()).collect()
    }
}
impl<T: Bits> Bits for &[T] {
    fn bits(&self) -> Vec<u32> {
        self.iter().flat_map(|e| e.bits()).collect()
    }
}
impl<A: Bits, B: Bits> Bits for (A, B) {
    fn bits(&self) -> Vec<u32> {
        let mut v = self.0.bits();
        v.extend(self.1.bits());
        v
    }
}
impl<A: Bits, B: Bits, C: Bits> Bits for (A, B, C) {
    fn bits(&self) -> Vec<u32> {
        let mut v = self.0.bits();
        v.extend(self.1.bits());
        v.extend(self.2.bits());
        v
    }
}
impl<A: Bits, B: Bits, C: Bits, D: Bits> Bits for (A, B, C, D) {
    fn bits(&self) -> Vec<u32> {
        let mut v = self.0.bits();
        v.extend(self.1.bits());
        v.extend(self.2.bits());
        v.extend(self.3.bits());
        v
    }
}

#[track_caller]
pub fn same<T: Bits + std::fmt::Debug>(what: &str, c: &T, r: &T) {
    let cb = c.bits();
    let rb = r.bits();
    if cb != rb {
        panic!(
            "MISMATCH in {what}\n  C    = {c:?}\n  Rust = {r:?}\n  C bits    = {cb:08x?}\n  Rust bits = {rb:08x?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Library location
// ---------------------------------------------------------------------------

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so() -> PathBuf {
    let build = crate_root().join("../c_src/build");
    let mut found: Option<PathBuf> = None;
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                found = Some(p);
                break;
            }
        }
    }
    found.unwrap_or_else(|| {
        panic!(
            "no C .so found in {}; build it with: cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    let root = crate_root();
    // Allow pointing the harness at a specific artifact (e.g. the debug-profile
    // cdylib) without touching the source.
    if let Ok(p) = std::env::var("RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "RUST_SO={} does not exist", p.display());
        return p;
    }
    let candidates = [
        root.join("target/release/libomni_collide_lib.so"),
        root.join("target/debug/libomni_collide_lib.so"),
    ];
    if !candidates.iter().any(|p| p.exists()) {
        // `cargo test` does not build a cdylib-only lib target, so build it.
        let st = std::process::Command::new(env!("CARGO"))
            .args(["build", "--release"])
            .current_dir(&root)
            .status();
        assert!(
            matches!(st, Ok(s) if s.success()),
            "failed to build the Rust cdylib; run `cargo build --release` first"
        );
    }
    let so = candidates
        .iter()
        .find(|p| p.exists())
        .cloned()
        .expect("Rust cdylib missing; run `cargo build --release`");

    // Guard against a stale artifact: `cargo test` will happily run against an
    // out-of-date `.so` because it never rebuilds a cdylib-only lib target, so a
    // forgotten `cargo build --release` would silently test the previous code.
    let so_time = std::fs::metadata(&so).and_then(|m| m.modified()).ok();
    let mut newest_src = None;
    if let Ok(rd) = std::fs::read_dir(root.join("src")) {
        for e in rd.flatten() {
            if e.path().extension().and_then(|s| s.to_str()) == Some("rs") {
                if let Ok(t) = e.metadata().and_then(|m| m.modified()) {
                    if newest_src.map_or(true, |n| t > n) {
                        newest_src = Some(t);
                    }
                }
            }
        }
    }
    if let (Some(so_t), Some(src_t)) = (so_time, newest_src) {
        assert!(
            so_t >= src_t,
            "{} is older than src/*.rs — run `cargo build --release` before `cargo test`",
            so.display()
        );
    }
    so
}

// ---------------------------------------------------------------------------
// Typed symbol table
// ---------------------------------------------------------------------------

macro_rules! api {
    ( $( $name:ident : $ty:ty ),* $(,)? ) => {
        pub struct Api {
            _lib: Library,
            $( pub $name: $ty, )*
        }
        impl Api {
            unsafe fn load(path: &Path) -> Api {
                let lib = unsafe { Library::new(path) }
                    .unwrap_or_else(|e| panic!("cannot dlopen {}: {e}", path.display()));
                $(
                    let $name: $ty = unsafe {
                        let s: Symbol<$ty> = lib
                            .get(concat!(stringify!($name), "\0").as_bytes())
                            .unwrap_or_else(|e| panic!(
                                "symbol `{}` missing from {}: {e}",
                                stringify!($name), path.display()));
                        *s
                    };
                )*
                Api { _lib: lib, $( $name, )* }
            }
        }
    };
}

api! {
    c2V: unsafe extern "C" fn(f32, f32) -> c2v,
    c2Mulvs: unsafe extern "C" fn(c2v, f32) -> c2v,
    c2Maxv: unsafe extern "C" fn(c2v, c2v) -> c2v,
    c2Minv: unsafe extern "C" fn(c2v, c2v) -> c2v,
    c2Clampv: unsafe extern "C" fn(c2v, c2v, c2v) -> c2v,
    c2Sub: unsafe extern "C" fn(c2v, c2v) -> c2v,
    c2Dot: unsafe extern "C" fn(c2v, c2v) -> f32,
    c2RotIdentity: unsafe extern "C" fn() -> c2r,
    c2xIdentity: unsafe extern "C" fn() -> c2x,
    c2BBVerts: unsafe extern "C" fn(*mut c2v, *mut c2AABB),
    c2MakeProxy: unsafe extern "C" fn(*const std::ffi::c_void, C2_TYPE, *mut c2Proxy),
    c2Len: unsafe extern "C" fn(c2v) -> f32,
    c2Det2: unsafe extern "C" fn(c2v, c2v) -> f32,
    c2GJKSimplexMetric: unsafe extern "C" fn(*mut c2Simplex) -> f32,
    c2Mulrv: unsafe extern "C" fn(c2r, c2v) -> c2v,
    c2Add: unsafe extern "C" fn(c2v, c2v) -> c2v,
    c2Mulxv: unsafe extern "C" fn(c2x, c2v) -> c2v,
    c22: unsafe extern "C" fn(*mut c2Simplex),
    c23: unsafe extern "C" fn(*mut c2Simplex),
    c2Neg: unsafe extern "C" fn(c2v) -> c2v,
    c2Skew: unsafe extern "C" fn(c2v) -> c2v,
    c2CCW90: unsafe extern "C" fn(c2v) -> c2v,
    c2D: unsafe extern "C" fn(*mut c2Simplex) -> c2v,
    c2Support: unsafe extern "C" fn(*const c2v, i32, c2v) -> i32,
    c2Witness: unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v),
    c2Div: unsafe extern "C" fn(c2v, f32) -> c2v,
    c2Norm: unsafe extern "C" fn(c2v) -> c2v,
    c2L: unsafe extern "C" fn(*mut c2Simplex) -> c2v,
    c2MulrvT: unsafe extern "C" fn(c2r, c2v) -> c2v,
    c2GJK: unsafe extern "C" fn(
        *const std::ffi::c_void, C2_TYPE, *const c2x,
        *const std::ffi::c_void, C2_TYPE, *const c2x,
        *mut c2v, *mut c2v, i32, *mut i32, *mut c2GJKCache) -> f32,
    c2AABBtoAABB: unsafe extern "C" fn(c2AABB, c2AABB) -> i32,
    c2AABBtoCapsule: unsafe extern "C" fn(c2AABB, c2Capsule) -> i32,
    c2CapsuletoCapsule: unsafe extern "C" fn(c2Capsule, c2Capsule) -> i32,
    c2CircletoCircle: unsafe extern "C" fn(c2Circle, c2Circle) -> i32,
    c2CircletoAABB: unsafe extern "C" fn(c2Circle, c2AABB) -> i32,
    c2CircletoCapsule: unsafe extern "C" fn(c2Circle, c2Capsule) -> i32,
    c2Collided: unsafe extern "C" fn(*const std::ffi::c_void, C2_TYPE, *const std::ffi::c_void, C2_TYPE) -> i32,
    ptr_from_parts: unsafe extern "C" fn(C2_TYPE, f32, f32, f32, f32, f32) -> *mut std::ffi::c_void,
    omni_collide: unsafe extern "C" fn(C2_TYPE, f32, f32, f32, f32, f32, C2_TYPE, f32, f32, f32, f32, f32) -> i32,
}

pub struct Both {
    pub c: Api,
    pub rs: Api,
}

static BOTH: OnceLock<Both> = OnceLock::new();

pub fn both() -> &'static Both {
    BOTH.get_or_init(|| {
        let cpath = find_c_so();
        let rpath = find_rust_so();
        eprintln!("C   .so: {}", cpath.display());
        eprintln!("Rust.so: {}", rpath.display());
        Both {
            c: unsafe { Api::load(&cpath) },
            rs: unsafe { Api::load(&rpath) },
        }
    })
}

// ---------------------------------------------------------------------------
// Deterministic PRNG + float generators
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E3779B97F4A7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        // SplitMix64
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in [0,1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform in [-r, r].
    pub fn range(&mut self, r: f32) -> f32 {
        (self.unit() * 2.0 - 1.0) * r
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    /// A "well-behaved" coordinate around the given scale.
    pub fn coord(&mut self, scale: f32) -> f32 {
        self.range(scale)
    }

    /// A coordinate drawn from the full float zoo: normals of wildly different
    /// magnitudes, signed zeros, subnormals, infinities and NaN.
    pub fn wild(&mut self) -> f32 {
        match self.below(14) {
            0 => 0.0,
            1 => -0.0,
            2 => f32::INFINITY,
            3 => f32::NEG_INFINITY,
            4 => f32::NAN,
            5 => -f32::NAN,
            6 => f32::MAX,
            7 => f32::MIN,
            8 => f32::MIN_POSITIVE,
            9 => f32::from_bits(self.next_u32() & 0x807F_FFFF), // subnormal
            10 => self.range(1.0e-30),
            11 => self.range(1.0e30),
            12 => f32::from_bits(self.next_u32()),
            _ => self.range(10.0),
        }
    }

    pub fn vec_coord(&mut self, scale: f32) -> c2v {
        c2v {
            x: self.coord(scale),
            y: self.coord(scale),
        }
    }
    pub fn vec_wild(&mut self) -> c2v {
        c2v {
            x: self.wild(),
            y: self.wild(),
        }
    }
    pub fn rot(&mut self) -> c2r {
        let a = self.range(std::f32::consts::PI);
        c2r {
            c: a.cos(),
            s: a.sin(),
        }
    }
    pub fn xform(&mut self, scale: f32) -> c2x {
        c2x {
            p: self.vec_coord(scale),
            r: self.rot(),
        }
    }
    pub fn circle(&mut self, scale: f32) -> c2Circle {
        c2Circle {
            p: self.vec_coord(scale),
            r: self.unit() * scale,
        }
    }
    pub fn aabb(&mut self, scale: f32) -> c2AABB {
        let a = self.vec_coord(scale);
        let b = self.vec_coord(scale);
        c2AABB {
            min: c2v {
                x: a.x.min(b.x),
                y: a.y.min(b.y),
            },
            max: c2v {
                x: a.x.max(b.x),
                y: a.y.max(b.y),
            },
        }
    }
    pub fn capsule(&mut self, scale: f32) -> c2Capsule {
        c2Capsule {
            a: self.vec_coord(scale),
            b: self.vec_coord(scale),
            r: self.unit() * scale,
        }
    }
}

/// Hand-picked boundary floats used to widen every randomized sweep.
pub const EDGE_F32: &[f32] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    2.0,
    3.0,
    1.0e-45, // smallest subnormal
    -1.0e-45,
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    1.19209290e-7, // FLT_EPSILON
    -1.19209290e-7,
    1.0e-30,
    1.0e30,
    f32::MAX,
    f32::MIN,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
];

pub const TYPES: [C2_TYPE; 3] = [C2_TYPE_CAPSULE, C2_TYPE_CIRCLE, C2_TYPE_AABB];

pub fn type_name(t: C2_TYPE) -> &'static str {
    match t {
        C2_TYPE_CAPSULE => "CAPSULE",
        C2_TYPE_CIRCLE => "CIRCLE",
        C2_TYPE_AABB => "AABB",
        _ => "INVALID",
    }
}

/// Five floats describing a shape of the given type, drawn at random.
pub fn shape_parts(rng: &mut Rng, t: C2_TYPE, scale: f32) -> [f32; 5] {
    match t {
        C2_TYPE_CIRCLE => {
            let c = rng.circle(scale);
            [c.p.x, c.p.y, c.r, rng.coord(scale), rng.coord(scale)]
        }
        C2_TYPE_AABB => {
            let b = rng.aabb(scale);
            [b.min.x, b.min.y, b.max.x, b.max.y, rng.coord(scale)]
        }
        _ => {
            let c = rng.capsule(scale);
            [c.a.x, c.a.y, c.b.x, c.b.y, c.r]
        }
    }
}
