#![allow(dead_code, non_snake_case)]

use libloading::Library;
use std::ffi::{c_float, c_int, c_void};
use std::fmt::Debug;
use std::path::{Path, PathBuf};

pub const CIRCLE: c_int = 0;
pub const AABB: c_int = 1;
pub const CAPSULE: c_int = 2;

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct V {
    pub x: c_float,
    pub y: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct R {
    pub c: c_float,
    pub s: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct X {
    pub p: V,
    pub r: R,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct Circle {
    pub p: V,
    pub r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct Box2 {
    pub min: V,
    pub max: V,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct Capsule {
    pub a: V,
    pub b: V,
    pub r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct Cache {
    pub metric: c_float,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct Proxy {
    pub radius: c_float,
    pub count: c_int,
    pub verts: [V; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct Sv {
    pub sA: V,
    pub sB: V,
    pub p: V,
    pub u: c_float,
    pub iA: c_int,
    pub iB: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct Simplex {
    pub a: Sv,
    pub b: Sv,
    pub c: Sv,
    pub d: Sv,
    pub div: c_float,
    pub count: c_int,
}

type F2V = unsafe extern "C" fn(c_float, c_float) -> V;
type FVfV = unsafe extern "C" fn(V, c_float) -> V;
type FVVV = unsafe extern "C" fn(V, V) -> V;
type FVVVV = unsafe extern "C" fn(V, V, V) -> V;
type FVVf = unsafe extern "C" fn(V, V) -> c_float;
type FVf = unsafe extern "C" fn(V) -> c_float;
type FV = unsafe extern "C" fn(V) -> V;
type FR = unsafe extern "C" fn() -> R;
type FX = unsafe extern "C" fn() -> X;
type FRVV = unsafe extern "C" fn(R, V) -> V;
type FXVV = unsafe extern "C" fn(X, V) -> V;
type FSimplexFloat = unsafe extern "C" fn(*mut Simplex) -> c_float;
type FSimplexVoid = unsafe extern "C" fn(*mut Simplex);
type FSimplexV = unsafe extern "C" fn(*mut Simplex) -> V;

pub struct Api {
    _lib: Library,
    pub c2V: F2V,
    pub c2Mulvs: FVfV,
    pub c2Maxv: FVVV,
    pub c2Minv: FVVV,
    pub c2Clampv: FVVVV,
    pub c2Sub: FVVV,
    pub c2Dot: FVVf,
    pub c2RotIdentity: FR,
    pub c2xIdentity: FX,
    pub c2BBVerts: unsafe extern "C" fn(*mut V, *mut Box2),
    pub c2MakeProxy: unsafe extern "C" fn(*const c_void, c_int, *mut Proxy),
    pub c2Len: FVf,
    pub c2Det2: FVVf,
    pub c2GJKSimplexMetric: FSimplexFloat,
    pub c2Mulrv: FRVV,
    pub c2Add: FVVV,
    pub c2Mulxv: FXVV,
    pub c22: FSimplexVoid,
    pub c23: FSimplexVoid,
    pub c2Neg: FV,
    pub c2Skew: FV,
    pub c2CCW90: FV,
    pub c2D: FSimplexV,
    pub c2Support: unsafe extern "C" fn(*const V, c_int, V) -> c_int,
    pub c2Witness: unsafe extern "C" fn(*mut Simplex, *mut V, *mut V),
    pub c2Div: FVfV,
    pub c2Norm: FV,
    pub c2L: FSimplexV,
    pub c2MulrvT: FRVV,
    pub c2GJK: unsafe extern "C" fn(
        *const c_void,
        c_int,
        *const X,
        *const c_void,
        c_int,
        *const X,
        *mut V,
        *mut V,
        c_int,
        *mut c_int,
        *mut Cache,
    ) -> c_float,
    pub c2AABBtoAABB: unsafe extern "C" fn(Box2, Box2) -> c_int,
    pub c2AABBtoCapsule: unsafe extern "C" fn(Box2, Capsule) -> c_int,
    pub c2CapsuletoCapsule: unsafe extern "C" fn(Capsule, Capsule) -> c_int,
    pub c2CircletoCircle: unsafe extern "C" fn(Circle, Circle) -> c_int,
    pub c2CircletoAABB: unsafe extern "C" fn(Circle, Box2) -> c_int,
    pub c2CircletoCapsule: unsafe extern "C" fn(Circle, Capsule) -> c_int,
    pub c2Collided:
        unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int,
    pub aabb: unsafe extern "C" fn(c_float, c_float, c_float, c_float) -> c_int,
}

impl Api {
    pub unsafe fn load(path: &Path) -> Self {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        macro_rules! symbol {
            ($name:literal, $ty:ty) => {{
                let symbol = unsafe { lib.get::<$ty>(concat!($name, "\0").as_bytes()) }
                    .unwrap_or_else(|error| panic!("missing {}: {error}", $name));
                *symbol
            }};
        }
        Self {
            c2V: symbol!("c2V", F2V),
            c2Mulvs: symbol!("c2Mulvs", FVfV),
            c2Maxv: symbol!("c2Maxv", FVVV),
            c2Minv: symbol!("c2Minv", FVVV),
            c2Clampv: symbol!("c2Clampv", FVVVV),
            c2Sub: symbol!("c2Sub", FVVV),
            c2Dot: symbol!("c2Dot", FVVf),
            c2RotIdentity: symbol!("c2RotIdentity", FR),
            c2xIdentity: symbol!("c2xIdentity", FX),
            c2BBVerts: symbol!("c2BBVerts", unsafe extern "C" fn(*mut V, *mut Box2)),
            c2MakeProxy: symbol!(
                "c2MakeProxy",
                unsafe extern "C" fn(*const c_void, c_int, *mut Proxy)
            ),
            c2Len: symbol!("c2Len", FVf),
            c2Det2: symbol!("c2Det2", FVVf),
            c2GJKSimplexMetric: symbol!("c2GJKSimplexMetric", FSimplexFloat),
            c2Mulrv: symbol!("c2Mulrv", FRVV),
            c2Add: symbol!("c2Add", FVVV),
            c2Mulxv: symbol!("c2Mulxv", FXVV),
            c22: symbol!("c22", FSimplexVoid),
            c23: symbol!("c23", FSimplexVoid),
            c2Neg: symbol!("c2Neg", FV),
            c2Skew: symbol!("c2Skew", FV),
            c2CCW90: symbol!("c2CCW90", FV),
            c2D: symbol!("c2D", FSimplexV),
            c2Support: symbol!(
                "c2Support",
                unsafe extern "C" fn(*const V, c_int, V) -> c_int
            ),
            c2Witness: symbol!(
                "c2Witness",
                unsafe extern "C" fn(*mut Simplex, *mut V, *mut V)
            ),
            c2Div: symbol!("c2Div", FVfV),
            c2Norm: symbol!("c2Norm", FV),
            c2L: symbol!("c2L", FSimplexV),
            c2MulrvT: symbol!("c2MulrvT", FRVV),
            c2GJK: symbol!(
                "c2GJK",
                unsafe extern "C" fn(
                    *const c_void,
                    c_int,
                    *const X,
                    *const c_void,
                    c_int,
                    *const X,
                    *mut V,
                    *mut V,
                    c_int,
                    *mut c_int,
                    *mut Cache,
                ) -> c_float
            ),
            c2AABBtoAABB: symbol!("c2AABBtoAABB", unsafe extern "C" fn(Box2, Box2) -> c_int),
            c2AABBtoCapsule: symbol!(
                "c2AABBtoCapsule",
                unsafe extern "C" fn(Box2, Capsule) -> c_int
            ),
            c2CapsuletoCapsule: symbol!(
                "c2CapsuletoCapsule",
                unsafe extern "C" fn(Capsule, Capsule) -> c_int
            ),
            c2CircletoCircle: symbol!(
                "c2CircletoCircle",
                unsafe extern "C" fn(Circle, Circle) -> c_int
            ),
            c2CircletoAABB: symbol!(
                "c2CircletoAABB",
                unsafe extern "C" fn(Circle, Box2) -> c_int
            ),
            c2CircletoCapsule: symbol!(
                "c2CircletoCapsule",
                unsafe extern "C" fn(Circle, Capsule) -> c_int
            ),
            c2Collided: symbol!(
                "c2Collided",
                unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int
            ),
            aabb: symbol!(
                "aabb",
                unsafe extern "C" fn(c_float, c_float, c_float, c_float) -> c_int
            ),
            _lib: lib,
        }
    }
}

pub struct Pair {
    pub c: Api,
    pub rust: Api,
}

impl Pair {
    pub unsafe fn load() -> Self {
        Self {
            c: unsafe { Api::load(&c_library_path()) },
            rust: unsafe { Api::load(&rust_library_path()) },
        }
    }
}

pub fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../c_src/build/libharvest-work-3Nv5PV.so")
}

pub fn rust_library_path() -> PathBuf {
    if let Some(path) = std::env::var_os("RUST_SO") {
        return PathBuf::from(path);
    }
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libaabb_lib.so")
}

pub fn same<T: Copy + Debug>(left: T, right: T, context: &str) {
    let left_bytes = unsafe {
        std::slice::from_raw_parts((&left as *const T).cast::<u8>(), std::mem::size_of::<T>())
    };
    let right_bytes = unsafe {
        std::slice::from_raw_parts(
            (&right as *const T).cast::<u8>(),
            std::mem::size_of::<T>(),
        )
    };
    assert_eq!(
        left_bytes, right_bytes,
        "{context}: C={left:?} Rust={right:?}"
    );
}

pub fn same_slice<T: Copy + Debug>(left: &[T], right: &[T], context: &str) {
    assert_eq!(left.len(), right.len(), "{context}: length mismatch");
    for (index, (&a, &b)) in left.iter().zip(right).enumerate() {
        same(a, b, &format!("{context}[{index}]"));
    }
}

#[derive(Clone, Copy)]
pub struct Rng(pub u64);

impl Rng {
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x as u32
    }

    pub fn finite(&mut self) -> f32 {
        let signed = self.next_u32() as i32;
        (signed % 200_001) as f32 / 64.0
    }

    pub fn small(&mut self) -> f32 {
        let signed = self.next_u32() as i32;
        (signed % 20_001) as f32 / 256.0
    }

    pub fn positive(&mut self) -> f32 {
        (self.next_u32() % 10_000) as f32 / 128.0 + 0.125
    }

    pub fn v(&mut self) -> V {
        V {
            x: self.finite(),
            y: self.finite(),
        }
    }

    pub fn small_v(&mut self) -> V {
        V {
            x: self.small(),
            y: self.small(),
        }
    }
}

pub fn ptr<T>(value: &T) -> *const c_void {
    (value as *const T).cast()
}
