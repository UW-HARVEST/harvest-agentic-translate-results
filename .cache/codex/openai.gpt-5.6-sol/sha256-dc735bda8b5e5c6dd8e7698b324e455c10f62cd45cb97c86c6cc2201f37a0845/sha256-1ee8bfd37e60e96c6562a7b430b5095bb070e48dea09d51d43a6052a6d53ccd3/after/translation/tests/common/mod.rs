#![allow(dead_code, non_snake_case)]

use libloading::Library;
use std::ffi::{c_float, c_int, c_uint, c_void};
use std::fmt::Debug;
use std::path::PathBuf;

pub const CIRCLE: c_uint = 0;
pub const AABB: c_uint = 1;
pub const CAPSULE: c_uint = 2;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct V {
    pub x: c_float,
    pub y: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct R {
    pub c: c_float,
    pub s: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct X {
    pub p: V,
    pub r: R,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Circle {
    pub p: V,
    pub r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Bb {
    pub min: V,
    pub max: V,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Capsule {
    pub a: V,
    pub b: V,
    pub r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Cache {
    pub metric: c_float,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Proxy {
    pub radius: c_float,
    pub count: c_int,
    pub verts: [V; 8],
}

impl Default for Proxy {
    fn default() -> Self {
        Self {
            radius: 0.0,
            count: 0,
            verts: [V::default(); 8],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Sv {
    pub sA: V,
    pub sB: V,
    pub p: V,
    pub u: c_float,
    pub iA: c_int,
    pub iB: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Simplex {
    pub a: Sv,
    pub b: Sv,
    pub c: Sv,
    pub d: Sv,
    pub div: c_float,
    pub count: c_int,
}

pub type FnV = unsafe extern "C" fn(c_float, c_float) -> V;
pub type FnMulvs = unsafe extern "C" fn(V, c_float) -> V;
pub type FnVvV = unsafe extern "C" fn(V, V) -> V;
pub type FnVvvV = unsafe extern "C" fn(V, V, V) -> V;
pub type FnVvF = unsafe extern "C" fn(V, V) -> c_float;
pub type FnVoidR = unsafe extern "C" fn() -> R;
pub type FnVoidX = unsafe extern "C" fn() -> X;
pub type FnBbVerts = unsafe extern "C" fn(*mut V, *mut Bb);
pub type FnMakeProxy = unsafe extern "C" fn(*const c_void, c_uint, *mut Proxy);
pub type FnVF = unsafe extern "C" fn(V) -> c_float;
pub type FnSimplexF = unsafe extern "C" fn(*mut Simplex) -> c_float;
pub type FnRvV = unsafe extern "C" fn(R, V) -> V;
pub type FnSimplexVoid = unsafe extern "C" fn(*mut Simplex);
pub type FnVV = unsafe extern "C" fn(V) -> V;
pub type FnSimplexV = unsafe extern "C" fn(*mut Simplex) -> V;
pub type FnSupport = unsafe extern "C" fn(*const V, c_int, V) -> c_int;
pub type FnWitness = unsafe extern "C" fn(*mut Simplex, *mut V, *mut V);
pub type FnDiv = unsafe extern "C" fn(V, c_float) -> V;
pub type FnXvV = unsafe extern "C" fn(X, V) -> V;
pub type FnGjk = unsafe extern "C" fn(
    *const c_void,
    c_uint,
    *const X,
    *const c_void,
    c_uint,
    *const X,
    *mut V,
    *mut V,
    c_int,
    *mut c_int,
    *mut Cache,
) -> c_float;
pub type FnBbBb = unsafe extern "C" fn(Bb, Bb) -> c_int;
pub type FnBbCapsule = unsafe extern "C" fn(Bb, Capsule) -> c_int;
pub type FnCapsuleCapsule = unsafe extern "C" fn(Capsule, Capsule) -> c_int;
pub type FnCircleCircle = unsafe extern "C" fn(Circle, Circle) -> c_int;
pub type FnCircleBb = unsafe extern "C" fn(Circle, Bb) -> c_int;
pub type FnCircleCapsule = unsafe extern "C" fn(Circle, Capsule) -> c_int;
pub type FnCollided = unsafe extern "C" fn(*const c_void, c_uint, *const c_void, c_uint) -> c_int;
pub type FnAabb = unsafe extern "C" fn(c_float, c_float, c_float, c_float) -> c_int;

pub struct Api {
    _lib: Library,
    pub aabb: FnAabb,
    pub c22: FnSimplexVoid,
    pub c23: FnSimplexVoid,
    pub c2AABBtoAABB: FnBbBb,
    pub c2AABBtoCapsule: FnBbCapsule,
    pub c2Add: FnVvV,
    pub c2BBVerts: FnBbVerts,
    pub c2CCW90: FnVV,
    pub c2CapsuletoCapsule: FnCapsuleCapsule,
    pub c2CircletoAABB: FnCircleBb,
    pub c2CircletoCapsule: FnCircleCapsule,
    pub c2CircletoCircle: FnCircleCircle,
    pub c2Clampv: FnVvvV,
    pub c2Collided: FnCollided,
    pub c2D: FnSimplexV,
    pub c2Det2: FnVvF,
    pub c2Div: FnDiv,
    pub c2Dot: FnVvF,
    pub c2GJK: FnGjk,
    pub c2GJKSimplexMetric: FnSimplexF,
    pub c2L: FnSimplexV,
    pub c2Len: FnVF,
    pub c2MakeProxy: FnMakeProxy,
    pub c2Maxv: FnVvV,
    pub c2Minv: FnVvV,
    pub c2Mulrv: FnRvV,
    pub c2MulrvT: FnRvV,
    pub c2Mulvs: FnMulvs,
    pub c2Mulxv: FnXvV,
    pub c2Neg: FnVV,
    pub c2Norm: FnVV,
    pub c2RotIdentity: FnVoidR,
    pub c2Skew: FnVV,
    pub c2Sub: FnVvV,
    pub c2Support: FnSupport,
    pub c2V: FnV,
    pub c2Witness: FnWitness,
    pub c2xIdentity: FnVoidX,
}

impl Api {
    pub unsafe fn load(path: PathBuf) -> Self {
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));

        macro_rules! symbol {
            ($name:ident, $ty:ty) => {{
                let value = unsafe {
                    *lib.get::<$ty>(concat!(stringify!($name), "\0").as_bytes())
                        .unwrap_or_else(|error| {
                            panic!(
                                "failed to load {} from {}: {error}",
                                stringify!($name),
                                path.display()
                            )
                        })
                };
                value
            }};
        }

        Self {
            aabb: symbol!(aabb, FnAabb),
            c22: symbol!(c22, FnSimplexVoid),
            c23: symbol!(c23, FnSimplexVoid),
            c2AABBtoAABB: symbol!(c2AABBtoAABB, FnBbBb),
            c2AABBtoCapsule: symbol!(c2AABBtoCapsule, FnBbCapsule),
            c2Add: symbol!(c2Add, FnVvV),
            c2BBVerts: symbol!(c2BBVerts, FnBbVerts),
            c2CCW90: symbol!(c2CCW90, FnVV),
            c2CapsuletoCapsule: symbol!(c2CapsuletoCapsule, FnCapsuleCapsule),
            c2CircletoAABB: symbol!(c2CircletoAABB, FnCircleBb),
            c2CircletoCapsule: symbol!(c2CircletoCapsule, FnCircleCapsule),
            c2CircletoCircle: symbol!(c2CircletoCircle, FnCircleCircle),
            c2Clampv: symbol!(c2Clampv, FnVvvV),
            c2Collided: symbol!(c2Collided, FnCollided),
            c2D: symbol!(c2D, FnSimplexV),
            c2Det2: symbol!(c2Det2, FnVvF),
            c2Div: symbol!(c2Div, FnDiv),
            c2Dot: symbol!(c2Dot, FnVvF),
            c2GJK: symbol!(c2GJK, FnGjk),
            c2GJKSimplexMetric: symbol!(c2GJKSimplexMetric, FnSimplexF),
            c2L: symbol!(c2L, FnSimplexV),
            c2Len: symbol!(c2Len, FnVF),
            c2MakeProxy: symbol!(c2MakeProxy, FnMakeProxy),
            c2Maxv: symbol!(c2Maxv, FnVvV),
            c2Minv: symbol!(c2Minv, FnVvV),
            c2Mulrv: symbol!(c2Mulrv, FnRvV),
            c2MulrvT: symbol!(c2MulrvT, FnRvV),
            c2Mulvs: symbol!(c2Mulvs, FnMulvs),
            c2Mulxv: symbol!(c2Mulxv, FnXvV),
            c2Neg: symbol!(c2Neg, FnVV),
            c2Norm: symbol!(c2Norm, FnVV),
            c2RotIdentity: symbol!(c2RotIdentity, FnVoidR),
            c2Skew: symbol!(c2Skew, FnVV),
            c2Sub: symbol!(c2Sub, FnVvV),
            c2Support: symbol!(c2Support, FnSupport),
            c2V: symbol!(c2V, FnV),
            c2Witness: symbol!(c2Witness, FnWitness),
            c2xIdentity: symbol!(c2xIdentity, FnVoidX),
            _lib: lib,
        }
    }
}

pub fn apis() -> (Api, Api) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_path = root
        .parent()
        .unwrap()
        .join("c_src/build/libharvest-work-OooqjH.so");
    let rust_path = root.join("target/release/libaabb_lib.so");
    assert!(
        c_path.is_file(),
        "missing C shared library: {}",
        c_path.display()
    );
    assert!(
        rust_path.is_file(),
        "missing Rust shared library: {}; run cargo build --release first",
        rust_path.display()
    );
    unsafe { (Api::load(c_path), Api::load(rust_path)) }
}

pub fn bytes<T>(value: &T) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts((value as *const T).cast::<u8>(), std::mem::size_of::<T>())
    }
}

pub fn assert_same<T: Debug>(context: &str, c: &T, rust: &T) {
    assert_eq!(bytes(c), bytes(rust), "{context}: C={c:?}, Rust={rust:?}");
}

pub fn assert_f32(context: &str, c: f32, rust: f32) {
    assert_eq!(
        c.to_bits(),
        rust.to_bits(),
        "{context}: C={c:?} ({:#010x}), Rust={rust:?} ({:#010x})",
        c.to_bits(),
        rust.to_bits()
    );
}

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x as u32
    }

    pub fn f32(&mut self) -> f32 {
        let magnitude = (self.u32() % 200_001) as f32 / 1000.0;
        if self.u32() & 1 == 0 {
            magnitude
        } else {
            -magnitude
        }
    }

    pub fn positive(&mut self) -> f32 {
        (self.u32() % 50_000 + 1) as f32 / 1000.0
    }

    pub fn v(&mut self) -> V {
        V {
            x: self.f32(),
            y: self.f32(),
        }
    }
}

pub fn transformed(base: V, scale: f32, quarter_turns: u32) -> V {
    let scaled = V {
        x: base.x * scale,
        y: base.y * scale,
    };
    match quarter_turns % 4 {
        0 => scaled,
        1 => V {
            x: -scaled.y,
            y: scaled.x,
        },
        2 => V {
            x: -scaled.x,
            y: -scaled.y,
        },
        _ => V {
            x: scaled.y,
            y: -scaled.x,
        },
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Shape {
    Circle(Circle),
    Bb(Bb),
    Capsule(Capsule),
}

impl Shape {
    pub fn kind(&self) -> c_uint {
        match self {
            Shape::Circle(_) => CIRCLE,
            Shape::Bb(_) => AABB,
            Shape::Capsule(_) => CAPSULE,
        }
    }

    pub fn ptr(&self) -> *const c_void {
        match self {
            Shape::Circle(value) => (value as *const Circle).cast(),
            Shape::Bb(value) => (value as *const Bb).cast(),
            Shape::Capsule(value) => (value as *const Capsule).cast(),
        }
    }
}

pub fn random_shape(rng: &mut Rng, kind: c_uint) -> Shape {
    match kind {
        CIRCLE => Shape::Circle(Circle {
            p: rng.v(),
            r: rng.positive(),
        }),
        AABB => {
            let center = rng.v();
            let half = V {
                x: rng.positive(),
                y: rng.positive(),
            };
            Shape::Bb(Bb {
                min: V {
                    x: center.x - half.x,
                    y: center.y - half.y,
                },
                max: V {
                    x: center.x + half.x,
                    y: center.y + half.y,
                },
            })
        }
        CAPSULE => Shape::Capsule(Capsule {
            a: rng.v(),
            b: rng.v(),
            r: rng.positive(),
        }),
        _ => unreachable!(),
    }
}
