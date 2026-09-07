#![allow(dead_code)]

use libloading::Library;
use std::ffi::{c_int, c_void};
use std::path::{Path, PathBuf};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct C2Circle {
    pub p: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct C2Aabb {
    pub min: C2v,
    pub max: C2v,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct C2Capsule {
    pub a: C2v,
    pub b: C2v,
    pub r: f32,
}

pub type C2VFn = unsafe extern "C" fn(f32, f32) -> C2v;
pub type C2MulvsFn = unsafe extern "C" fn(C2v, f32) -> C2v;
pub type C2PairFn = unsafe extern "C" fn(C2v, C2v) -> C2v;
pub type C2ClampvFn = unsafe extern "C" fn(C2v, C2v, C2v) -> C2v;
pub type C2DotFn = unsafe extern "C" fn(C2v, C2v) -> f32;
pub type C2CircleCircleFn = unsafe extern "C" fn(C2Circle, C2Circle) -> c_int;
pub type C2CircleAabbFn = unsafe extern "C" fn(C2Circle, C2Aabb) -> c_int;
pub type C2CircleCapsuleFn = unsafe extern "C" fn(C2Circle, C2Capsule) -> c_int;
pub type C2CollidedFn = unsafe extern "C" fn(*const c_void, *const c_void, c_int) -> c_int;
pub type CircleCollideFn = unsafe extern "C" fn(f32, f32, f32) -> c_int;

pub struct Api {
    _library: Library,
    pub c2v: C2VFn,
    pub mulvs: C2MulvsFn,
    pub maxv: C2PairFn,
    pub minv: C2PairFn,
    pub clampv: C2ClampvFn,
    pub sub: C2PairFn,
    pub dot: C2DotFn,
    pub circle_circle: C2CircleCircleFn,
    pub circle_aabb: C2CircleAabbFn,
    pub circle_capsule: C2CircleCapsuleFn,
    pub collided: C2CollidedFn,
    pub circle_collide: CircleCollideFn,
}

impl Api {
    pub unsafe fn load(path: &Path) -> Self {
        // SAFETY: The test keeps the library alive in the returned Api and
        // requests signatures mechanically copied from the C implementation.
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        macro_rules! symbol {
            ($name:literal, $ty:ty) => {{
                // SAFETY: The symbol names and signatures are the tested ABI.
                unsafe { *library.get::<$ty>(concat!($name, "\0").as_bytes()).unwrap() }
            }};
        }
        let c2v = symbol!("c2V", C2VFn);
        let mulvs = symbol!("c2Mulvs", C2MulvsFn);
        let maxv = symbol!("c2Maxv", C2PairFn);
        let minv = symbol!("c2Minv", C2PairFn);
        let clampv = symbol!("c2Clampv", C2ClampvFn);
        let sub = symbol!("c2Sub", C2PairFn);
        let dot = symbol!("c2Dot", C2DotFn);
        let circle_circle = symbol!("c2CircletoCircle", C2CircleCircleFn);
        let circle_aabb = symbol!("c2CircletoAABB", C2CircleAabbFn);
        let circle_capsule = symbol!("c2CircletoCapsule", C2CircleCapsuleFn);
        let collided = symbol!("c2Collided", C2CollidedFn);
        let circle_collide = symbol!("circle_collide", CircleCollideFn);
        Self {
            _library: library,
            c2v,
            mulvs,
            maxv,
            minv,
            clampv,
            sub,
            dot,
            circle_circle,
            circle_aabb,
            circle_capsule,
            collided,
            circle_collide,
        }
    }
}

pub struct Pair {
    pub c: Api,
    pub rust: Api,
}

impl Pair {
    pub fn load() -> Self {
        // SAFETY: Api::load validates that every required dynamic symbol exists.
        unsafe {
            Self {
                c: Api::load(&c_library_path()),
                rust: Api::load(&rust_library_path()),
            }
        }
    }
}

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_library_path() -> PathBuf {
    let build = manifest_dir().join("../c_src/build");
    let mut candidates: Vec<_> = std::fs::read_dir(&build)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", build.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "so"))
        .collect();
    candidates.sort();
    assert_eq!(
        candidates.len(),
        1,
        "expected one C shared library in {}, got {candidates:?}",
        build.display()
    );
    candidates.pop().unwrap()
}

pub fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/release/libcircle_collide_lib.so")
}

pub fn v(x: f32, y: f32) -> C2v {
    C2v { x, y }
}

pub fn add(a: C2v, b: C2v) -> C2v {
    v(a.x + b.x, a.y + b.y)
}

pub fn scale(a: C2v, scalar: f32) -> C2v {
    v(a.x * scalar, a.y * scalar)
}

pub fn perpendicular(a: C2v) -> C2v {
    v(-a.y, a.x)
}

pub fn assert_f32(row: &str, case: usize, c: f32, rust: f32) {
    assert_eq!(
        c.to_bits(),
        rust.to_bits(),
        "{row} case {case}: C={c:?} ({:#010x}), Rust={rust:?} ({:#010x})",
        c.to_bits(),
        rust.to_bits()
    );
}

pub fn assert_v(row: &str, case: usize, c: C2v, rust: C2v) {
    assert_f32(&format!("{row}.x"), case, c.x, rust.x);
    assert_f32(&format!("{row}.y"), case, c.y, rust.y);
}

pub fn assert_i32(row: &str, case: usize, c: c_int, rust: c_int) {
    assert_eq!(c, rust, "{row} case {case}: C={c}, Rust={rust}");
}

#[derive(Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x as u32
    }

    pub fn bool(&mut self) -> bool {
        self.next_u32() & 1 != 0
    }

    pub fn unit(&mut self) -> f32 {
        (self.next_u32() as f64 / u32::MAX as f64) as f32
    }

    pub fn range(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.unit()
    }

    pub fn finite(&mut self) -> f32 {
        self.range(-10_000.0, 10_000.0)
    }

    pub fn nonzero_vector(&mut self) -> C2v {
        loop {
            let result = v(self.range(-100.0, 100.0), self.range(-100.0, 100.0));
            if result.x.abs() + result.y.abs() > 1.0 {
                return result;
            }
        }
    }
}

pub const SPECIAL: [f32; 14] = [
    0.0,
    -0.0,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::MAX,
    -f32::MAX,
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    f32::from_bits(1),
    f32::from_bits(0x8000_0001),
    f32::from_bits(0x7fc0_0001),
    f32::from_bits(0x7fc1_2345),
    f32::from_bits(0xffc0_0001),
    f32::from_bits(0x7f80_0001),
];
