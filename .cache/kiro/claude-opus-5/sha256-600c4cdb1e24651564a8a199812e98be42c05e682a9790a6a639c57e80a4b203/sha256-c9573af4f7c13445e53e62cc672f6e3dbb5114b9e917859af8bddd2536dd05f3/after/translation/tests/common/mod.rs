//! Shared differential-test harness.
//!
//! Loads BOTH shared objects with `libloading` and exposes an identical `Api`
//! view over each, so every test calls the C library and the Rust library
//! through the exact same FFI path an external consumer would use. Rust
//! functions are NEVER called directly — only through `libcapsule_lib.so`.

#![allow(non_snake_case, non_camel_case_types, dead_code)]

use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// C types (layout must match c_src/src/lib.c exactly)
// ---------------------------------------------------------------------------

pub const C2_TYPE_CIRCLE: i32 = 0;
pub const C2_TYPE_AABB: i32 = 1;
pub const C2_TYPE_CAPSULE: i32 = 2;

pub const FLT_EPSILON: f32 = 1.192_092_895_507_812_5e-7;
pub const FLT_MAX: f32 = f32::MAX;

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
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
pub struct c2Proxy {
    pub radius: f32,
    pub count: i32,
    pub verts: [c2v; 8],
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

/// `typedef struct { c2sv a, b, c, d; float div; int count; } c2Simplex;`
#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct c2Simplex {
    pub v: [c2sv; 4],
    pub div: f32,
    pub count: i32,
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

pub trait Bits {
    type Out: PartialEq + std::fmt::Debug;
    fn bits(&self) -> Self::Out;
}

/// Canonical bit pattern for a `f32`.
///
/// Bit-exact, with ONE documented exception: all NaNs map to a single value.
///
/// Rationale (verified experimentally, see `NOTES-nan.md`): when *both*
/// operands of an SSE `mulss`/`addss`/`subss` are NaN, x86 propagates the
/// *destination* operand. GCC at `-O0` makes the right-hand operand the
/// destination; GCC at `-O1`+ and LLVM at every level make the left-hand
/// operand the destination. So the resulting NaN *sign/payload* for the very
/// same C source flips with `-O0` vs `-O2` — it is unspecified instruction
/// selection, not program semantics. Matching `-O0` byte-for-byte would
/// require writing the Rust operands backwards, which would then diverge from
/// an `-O1`+ build of the identical C. Every order-*independent* case is still
/// compared strictly: single-NaN propagation, `inf - inf`, `0 * inf`, `±0`,
/// subnormals, overflow to `±inf`. Use [`same_strict`] to compare raw bits
/// including the NaN payload.
pub fn canon_f32(v: f32) -> u32 {
    if v.is_nan() { 0x7FC0_0000 } else { v.to_bits() }
}

impl Bits for f32 {
    type Out = u32;
    fn bits(&self) -> u32 {
        canon_f32(*self)
    }
}
impl Bits for i32 {
    type Out = i32;
    fn bits(&self) -> i32 {
        *self
    }
}
impl Bits for c2v {
    type Out = (u32, u32);
    fn bits(&self) -> (u32, u32) {
        (canon_f32(self.x), canon_f32(self.y))
    }
}
impl Bits for c2r {
    type Out = (u32, u32);
    fn bits(&self) -> (u32, u32) {
        (canon_f32(self.c), canon_f32(self.s))
    }
}
impl Bits for c2x {
    type Out = ((u32, u32), (u32, u32));
    fn bits(&self) -> Self::Out {
        (self.p.bits(), self.r.bits())
    }
}
impl Bits for c2AABB {
    type Out = ((u32, u32), (u32, u32));
    fn bits(&self) -> Self::Out {
        (self.min.bits(), self.max.bits())
    }
}
impl Bits for c2Circle {
    type Out = ((u32, u32), u32);
    fn bits(&self) -> Self::Out {
        (self.p.bits(), canon_f32(self.r))
    }
}
impl Bits for c2Capsule {
    type Out = ((u32, u32), (u32, u32), u32);
    fn bits(&self) -> Self::Out {
        (self.a.bits(), self.b.bits(), canon_f32(self.r))
    }
}
impl Bits for c2GJKCache {
    type Out = (u32, i32, [i32; 3], [i32; 3], u32);
    fn bits(&self) -> Self::Out {
        (
            canon_f32(self.metric),
            self.count,
            self.iA,
            self.iB,
            canon_f32(self.div),
        )
    }
}
impl Bits for c2Proxy {
    type Out = (u32, i32, Vec<(u32, u32)>);
    fn bits(&self) -> Self::Out {
        (
            canon_f32(self.radius),
            self.count,
            self.verts.iter().map(|v| v.bits()).collect(),
        )
    }
}
impl Bits for c2sv {
    type Out = ((u32, u32), (u32, u32), (u32, u32), u32, i32, i32);
    fn bits(&self) -> Self::Out {
        (
            self.sA.bits(),
            self.sB.bits(),
            self.p.bits(),
            canon_f32(self.u),
            self.iA,
            self.iB,
        )
    }
}
impl Bits for c2Simplex {
    type Out = (Vec<<c2sv as Bits>::Out>, u32, i32);
    fn bits(&self) -> Self::Out {
        (
            self.v.iter().map(|x| x.bits()).collect(),
            canon_f32(self.div),
            self.count,
        )
    }
}
impl<T: Bits> Bits for Option<T> {
    type Out = Option<T::Out>;
    fn bits(&self) -> Self::Out {
        self.as_ref().map(|x| x.bits())
    }
}
impl<A: Bits, B: Bits> Bits for (A, B) {
    type Out = (A::Out, B::Out);
    fn bits(&self) -> Self::Out {
        (self.0.bits(), self.1.bits())
    }
}
impl<A: Bits, B: Bits, C: Bits> Bits for (A, B, C) {
    type Out = (A::Out, B::Out, C::Out);
    fn bits(&self) -> Self::Out {
        (self.0.bits(), self.1.bits(), self.2.bits())
    }
}
impl<A: Bits, B: Bits, C: Bits, D: Bits> Bits for (A, B, C, D) {
    type Out = (A::Out, B::Out, C::Out, D::Out);
    fn bits(&self) -> Self::Out {
        (self.0.bits(), self.1.bits(), self.2.bits(), self.3.bits())
    }
}
impl<A: Bits, B: Bits, C: Bits, D: Bits, E: Bits> Bits for (A, B, C, D, E) {
    type Out = (A::Out, B::Out, C::Out, D::Out, E::Out);
    fn bits(&self) -> Self::Out {
        (
            self.0.bits(),
            self.1.bits(),
            self.2.bits(),
            self.3.bits(),
            self.4.bits(),
        )
    }
}
impl<T: Bits, const N: usize> Bits for [T; N] {
    type Out = Vec<T::Out>;
    fn bits(&self) -> Self::Out {
        self.iter().map(|x| x.bits()).collect()
    }
}

// ---------------------------------------------------------------------------
// Function-pointer types
// ---------------------------------------------------------------------------

pub type FnVV = unsafe extern "C" fn(c2v) -> c2v;
pub type FnVVV = unsafe extern "C" fn(c2v, c2v) -> c2v;
pub type FnFVV = unsafe extern "C" fn(c2v, c2v) -> f32;
pub type FnGjk = unsafe extern "C" fn(
    *const std::ffi::c_void,
    i32,
    *const c2x,
    *const std::ffi::c_void,
    i32,
    *const c2x,
    *mut c2v,
    *mut c2v,
    i32,
    *mut i32,
    *mut c2GJKCache,
) -> f32;

macro_rules! define_api {
    ( $( $name:ident : $ty:ty ),* $(,)? ) => {
        pub struct Api {
            pub which: &'static str,
            $( pub $name: $ty, )*
            _lib: &'static libloading::Library,
        }
        impl Api {
            fn from_lib(lib: &'static libloading::Library, which: &'static str) -> Api {
                unsafe {
                    Api {
                        which,
                        $( $name: *lib
                            .get::<$ty>(concat!(stringify!($name), "\0").as_bytes())
                            .unwrap_or_else(|e| panic!(
                                "{which}: missing symbol `{}`: {e}", stringify!($name))), )*
                        _lib: lib,
                    }
                }
            }
        }
    };
}

define_api! {
    c2V: unsafe extern "C" fn(f32, f32) -> c2v,
    c2Mulvs: unsafe extern "C" fn(c2v, f32) -> c2v,
    c2Maxv: FnVVV,
    c2Minv: FnVVV,
    c2Clampv: unsafe extern "C" fn(c2v, c2v, c2v) -> c2v,
    c2Sub: FnVVV,
    c2Dot: FnFVV,
    c2RotIdentity: unsafe extern "C" fn() -> c2r,
    c2xIdentity: unsafe extern "C" fn() -> c2x,
    c2BBVerts: unsafe extern "C" fn(*mut c2v, *mut c2AABB),
    c2MakeProxy: unsafe extern "C" fn(*const std::ffi::c_void, i32, *mut c2Proxy),
    c2Len: unsafe extern "C" fn(c2v) -> f32,
    c2Det2: FnFVV,
    c2GJKSimplexMetric: unsafe extern "C" fn(*mut c2Simplex) -> f32,
    c2Mulrv: unsafe extern "C" fn(c2r, c2v) -> c2v,
    c2Add: FnVVV,
    c2Mulxv: unsafe extern "C" fn(c2x, c2v) -> c2v,
    c22: unsafe extern "C" fn(*mut c2Simplex),
    c23: unsafe extern "C" fn(*mut c2Simplex),
    c2Neg: FnVV,
    c2Skew: FnVV,
    c2CCW90: FnVV,
    c2D: unsafe extern "C" fn(*mut c2Simplex) -> c2v,
    c2Support: unsafe extern "C" fn(*const c2v, i32, c2v) -> i32,
    c2Witness: unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v),
    c2Div: unsafe extern "C" fn(c2v, f32) -> c2v,
    c2Norm: FnVV,
    c2L: unsafe extern "C" fn(*mut c2Simplex) -> c2v,
    c2MulrvT: unsafe extern "C" fn(c2r, c2v) -> c2v,
    c2GJK: FnGjk,
    c2AABBtoAABB: unsafe extern "C" fn(c2AABB, c2AABB) -> i32,
    c2AABBtoCapsule: unsafe extern "C" fn(c2AABB, c2Capsule) -> i32,
    c2CapsuletoCapsule: unsafe extern "C" fn(c2Capsule, c2Capsule) -> i32,
    c2CircletoCircle: unsafe extern "C" fn(c2Circle, c2Circle) -> i32,
    c2CircletoAABB: unsafe extern "C" fn(c2Circle, c2AABB) -> i32,
    c2CircletoCapsule: unsafe extern "C" fn(c2Circle, c2Capsule) -> i32,
    c2Collided: unsafe extern "C" fn(*const std::ffi::c_void, i32, *const std::ffi::c_void, i32) -> i32,
    capsule: unsafe extern "C" fn(f32, f32, f32, f32, f32) -> i32,
}

// ---------------------------------------------------------------------------
// Library discovery / loading
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_PATH") {
        return PathBuf::from(p);
    }
    let dir = manifest_dir().parent().unwrap().join("c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so")
                && p.file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| s.starts_with("lib"))
            {
                found.push(p);
            }
        }
    }
    found.sort();
    found.pop().unwrap_or_else(|| {
        panic!(
            "no C .so found in {}; build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            dir.display()
        )
    })
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        return PathBuf::from(p);
    }
    // The test binary lives in <target>/<profile>/deps/, so the cdylib is our
    // grand-parent directory.
    if let Ok(exe) = std::env::current_exe() {
        let mut d: Option<&Path> = exe.parent();
        while let Some(dir) = d {
            let cand = dir.join("libcapsule_lib.so");
            if cand.is_file() {
                return cand;
            }
            d = dir.parent();
        }
    }
    for profile in ["debug", "release"] {
        let cand = manifest_dir()
            .join("target")
            .join(profile)
            .join("libcapsule_lib.so");
        if cand.is_file() {
            return cand;
        }
    }
    panic!("libcapsule_lib.so not found; run `cargo build` first");
}

fn leak_lib(path: &Path) -> &'static libloading::Library {
    let lib = unsafe { libloading::Library::new(path) }
        .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
    Box::leak(Box::new(lib))
}

/// Both implementations, loaded through `dlopen`.
pub struct Pair {
    pub c: &'static Api,
    pub rs: &'static Api,
}

static PAIR: std::sync::OnceLock<Pair> = std::sync::OnceLock::new();

pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| {
        let c = Api::from_lib(leak_lib(&c_so_path()), "C");
        let rs = Api::from_lib(leak_lib(&rust_so_path()), "Rust");
        Pair {
            c: Box::leak(Box::new(c)),
            rs: Box::leak(Box::new(rs)),
        }
    })
}

impl<T: Bits> Bits for Vec<T> {
    type Out = Vec<T::Out>;
    fn bits(&self) -> Self::Out {
        self.iter().map(|x| x.bits()).collect()
    }
}

/// Assert that two values are bit-identical.
#[track_caller]
pub fn same<T: Bits>(what: &str, c: T, rs: T) {
    let (cb, rb) = (c.bits(), rs.bits());
    assert!(
        cb == rb,
        "DIVERGENCE in {what}\n  C    = {cb:?}\n  Rust = {rb:?}"
    );
}

/// Assert raw bit equality, INCLUDING the NaN sign/payload. Only valid where
/// the result cannot come from an operation with two NaN operands (see
/// [`canon_f32`]).
#[track_caller]
pub fn same_strict(what: &str, c: f32, rs: f32) {
    assert!(
        c.to_bits() == rs.to_bits(),
        "STRICT DIVERGENCE in {what}\n  C    = {:#010x} ({c})\n  Rust = {:#010x} ({rs})",
        c.to_bits(),
        rs.to_bits()
    );
}

/// Same, for `c2v`.
#[track_caller]
pub fn same_strict_v(what: &str, c: c2v, rs: c2v) {
    same_strict(&format!("{what}.x"), c.x, rs.x);
    same_strict(&format!("{what}.y"), c.y, rs.y);
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64) + float generators
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5DEE_CE66_D000_0001;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ SEED)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// Uniform in `[-1, 1)`.
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() as f32 / 4_294_967_296.0) * 2.0 - 1.0
    }
    /// Uniform in `[lo, hi)`.
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (self.next_u32() as f32 / 4_294_967_296.0) * (hi - lo)
    }
    /// A "tame" coordinate: finite, magnitudes spread over many decades.
    pub fn coord(&mut self) -> f32 {
        let m = self.unit();
        match self.below(6) {
            0 => m,
            1 => m * 100.0,
            2 => m * 10_000.0,
            3 => m * 0.01,
            4 => m * 1e-6,
            _ => m * 1e6,
        }
    }
    /// Small integral-ish coordinate — makes exact ties / equal values likely.
    pub fn grid(&mut self) -> f32 {
        (self.below(21) as i32 - 10) as f32
    }
    /// Any float, including non-finite and subnormal values.
    pub fn wild(&mut self) -> f32 {
        match self.below(16) {
            0 => 0.0,
            1 => -0.0,
            2 => f32::NAN,
            3 => f32::INFINITY,
            4 => f32::NEG_INFINITY,
            5 => f32::MAX,
            6 => f32::MIN,
            7 => f32::MIN_POSITIVE,
            8 => -f32::MIN_POSITIVE,
            9 => f32::from_bits(1),           // smallest subnormal
            10 => f32::from_bits(0x8000_0001), // -smallest subnormal
            11 => 1.0,
            12 => -1.0,
            13 => FLT_EPSILON,
            14 => self.grid(),
            _ => self.coord(),
        }
    }
    pub fn vec_coord(&mut self) -> c2v {
        c2v {
            x: self.coord(),
            y: self.coord(),
        }
    }
    pub fn vec_grid(&mut self) -> c2v {
        c2v {
            x: self.grid(),
            y: self.grid(),
        }
    }
    pub fn vec_wild(&mut self) -> c2v {
        c2v {
            x: self.wild(),
            y: self.wild(),
        }
    }
    /// Mix of tame, grid and wild vectors.
    pub fn vec_any(&mut self) -> c2v {
        match self.below(4) {
            0 => self.vec_grid(),
            1 => self.vec_wild(),
            _ => self.vec_coord(),
        }
    }
    pub fn rot_unit(&mut self) -> c2r {
        let t = self.range(-6.283_185_5, 6.283_185_5);
        c2r {
            c: t.cos(),
            s: t.sin(),
        }
    }
    pub fn rot_any(&mut self) -> c2r {
        match self.below(4) {
            0 => c2r { c: 1.0, s: 0.0 },
            1 => c2r {
                c: self.coord(),
                s: self.coord(),
            },
            _ => self.rot_unit(),
        }
    }
    pub fn xform_translation(&mut self) -> c2x {
        c2x {
            p: self.vec_coord(),
            r: c2r { c: 1.0, s: 0.0 },
        }
    }
    pub fn xform_rotation(&mut self) -> c2x {
        c2x {
            p: c2v { x: 0.0, y: 0.0 },
            r: self.rot_unit(),
        }
    }
    pub fn xform_full(&mut self) -> c2x {
        c2x {
            p: self.vec_coord(),
            r: self.rot_unit(),
        }
    }
    pub fn xform_unnormalised(&mut self) -> c2x {
        c2x {
            p: self.vec_coord(),
            r: c2r {
                c: self.range(-3.0, 3.0),
                s: self.range(-3.0, 3.0),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Shape generators (geometry classes referenced by CONFIGS.md)
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Class {
    /// Widely separated.
    Far,
    /// A few units apart.
    Near,
    /// Overlapping.
    Overlap,
    /// A inside B / B inside A.
    Contained,
    /// Exactly the same position.
    Coincident,
    /// Zero-extent shapes.
    Degenerate,
    /// Integral coordinates in a small box — exact ties and equalities.
    Grid,
    /// Huge coordinates.
    Huge,
    /// Subnormal coordinates.
    Tiny,
    /// Inverted AABBs / negative radii.
    Malformed,
}

pub const ALL_CLASSES: [Class; 10] = [
    Class::Far,
    Class::Near,
    Class::Overlap,
    Class::Contained,
    Class::Coincident,
    Class::Degenerate,
    Class::Grid,
    Class::Huge,
    Class::Tiny,
    Class::Malformed,
];

/// Raw bytes of a shape, plus its `C2_TYPE`, ready to hand to `c2GJK`.
#[derive(Clone, Debug)]
pub struct Shape {
    pub ty: i32,
    pub bytes: Vec<u8>,
}

impl Shape {
    pub fn circle(c: c2Circle) -> Shape {
        Shape {
            ty: C2_TYPE_CIRCLE,
            bytes: as_bytes(&c),
        }
    }
    pub fn aabb(b: c2AABB) -> Shape {
        Shape {
            ty: C2_TYPE_AABB,
            bytes: as_bytes(&b),
        }
    }
    pub fn capsule(c: c2Capsule) -> Shape {
        Shape {
            ty: C2_TYPE_CAPSULE,
            bytes: as_bytes(&c),
        }
    }
    pub fn ptr(&self) -> *const std::ffi::c_void {
        self.bytes.as_ptr() as *const std::ffi::c_void
    }
}

pub fn as_bytes<T: Copy>(v: &T) -> Vec<u8> {
    let mut out = vec![0u8; std::mem::size_of::<T>()];
    unsafe {
        std::ptr::copy_nonoverlapping(v as *const T as *const u8, out.as_mut_ptr(), out.len());
    }
    out
}

/// Scale/offset pair used to place a shape for a geometry class.
fn class_center(rng: &mut Rng, class: Class, second: bool) -> c2v {
    match class {
        Class::Far => {
            let base = if second { 500.0 } else { -500.0 };
            c2v {
                x: base + rng.range(-20.0, 20.0),
                y: rng.range(-20.0, 20.0),
            }
        }
        Class::Near => {
            let base = if second { 3.0 } else { -3.0 };
            c2v {
                x: base + rng.range(-1.5, 1.5),
                y: rng.range(-1.5, 1.5),
            }
        }
        Class::Overlap | Class::Contained | Class::Degenerate => c2v {
            x: rng.range(-1.0, 1.0),
            y: rng.range(-1.0, 1.0),
        },
        Class::Coincident => c2v { x: 0.0, y: 0.0 },
        Class::Grid => rng.vec_grid(),
        Class::Huge => c2v {
            x: rng.range(-1e18, 1e18),
            y: rng.range(-1e18, 1e18),
        },
        Class::Tiny => c2v {
            x: rng.range(-1e-38, 1e-38),
            y: rng.range(-1e-38, 1e-38),
        },
        Class::Malformed => rng.vec_coord(),
    }
}

fn class_extent(rng: &mut Rng, class: Class) -> f32 {
    match class {
        Class::Far | Class::Near | Class::Overlap => rng.range(0.25, 4.0),
        Class::Contained => rng.range(0.1, 1.0),
        Class::Coincident => rng.range(0.5, 2.0),
        Class::Degenerate => 0.0,
        Class::Grid => rng.below(4) as f32,
        Class::Huge => rng.range(1e15, 1e17),
        Class::Tiny => rng.range(0.0, 1e-38),
        Class::Malformed => rng.range(-4.0, 4.0),
    }
}

pub fn gen_shape(rng: &mut Rng, ty: i32, class: Class, second: bool) -> Shape {
    let c = class_center(rng, class, second);
    let e = class_extent(rng, class);
    match ty {
        C2_TYPE_CIRCLE => Shape::circle(c2Circle { p: c, r: e }),
        C2_TYPE_AABB => {
            let (dx, dy) = (e, class_extent(rng, class));
            let bb = if class == Class::Malformed {
                // deliberately inverted
                c2AABB {
                    min: c2v {
                        x: c.x + dx.abs(),
                        y: c.y + dy.abs(),
                    },
                    max: c2v {
                        x: c.x - dx.abs(),
                        y: c.y - dy.abs(),
                    },
                }
            } else {
                c2AABB {
                    min: c2v {
                        x: c.x - dx.abs(),
                        y: c.y - dy.abs(),
                    },
                    max: c2v {
                        x: c.x + dx.abs(),
                        y: c.y + dy.abs(),
                    },
                }
            };
            Shape::aabb(bb)
        }
        C2_TYPE_CAPSULE => {
            let half = class_extent(rng, class);
            let dir = rng.rot_unit();
            let a = c2v {
                x: c.x - dir.c * half,
                y: c.y - dir.s * half,
            };
            let b = if class == Class::Degenerate {
                a
            } else {
                c2v {
                    x: c.x + dir.c * half,
                    y: c.y + dir.s * half,
                }
            };
            Shape::capsule(c2Capsule { a, b, r: e })
        }
        _ => unreachable!("invalid type {ty}"),
    }
}

pub const TYPES: [i32; 3] = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE];

pub fn type_name(t: i32) -> &'static str {
    match t {
        C2_TYPE_CIRCLE => "CIRCLE",
        C2_TYPE_AABB => "AABB",
        C2_TYPE_CAPSULE => "CAPSULE",
        _ => "INVALID",
    }
}

// ---------------------------------------------------------------------------
// c2GJK differential driver
// ---------------------------------------------------------------------------

/// Everything `c2GJK` can observably produce.
#[derive(Clone, Debug)]
pub struct GjkOut {
    pub dist: f32,
    pub a: Option<c2v>,
    pub b: Option<c2v>,
    pub iters: Option<i32>,
    pub cache: Option<c2GJKCache>,
}

impl Bits for GjkOut {
    type Out = (
        u32,
        Option<(u32, u32)>,
        Option<(u32, u32)>,
        Option<i32>,
        Option<<c2GJKCache as Bits>::Out>,
    );
    fn bits(&self) -> Self::Out {
        (
            canon_f32(self.dist),
            self.a.bits(),
            self.b.bits(),
            self.iters,
            self.cache.bits(),
        )
    }
}

/// Which optional out-params to pass.
#[derive(Copy, Clone, Debug)]
pub struct OutSel {
    pub a: bool,
    pub b: bool,
    pub iters: bool,
}

impl OutSel {
    pub const ALL: OutSel = OutSel {
        a: true,
        b: true,
        iters: true,
    };
}

#[allow(clippy::too_many_arguments)]
pub fn run_gjk(
    api: &Api,
    sa: &Shape,
    ax: Option<c2x>,
    sb: &Shape,
    bx: Option<c2x>,
    use_radius: i32,
    sel: OutSel,
    cache_in: Option<c2GJKCache>,
) -> GjkOut {
    // Poison the out-params so a missing write is detectable.
    let mut a = c2v {
        x: f32::from_bits(0xDEAD_BEEF),
        y: f32::from_bits(0xDEAD_BEEE),
    };
    let mut b = a;
    let mut iters: i32 = -12345;
    let mut cache = cache_in.unwrap_or_default();

    // The C code casts away const and writes nothing, but be safe and give it
    // its own mutable copies of the shape bytes.
    let mut abytes = sa.bytes.clone();
    let mut bbytes = sb.bytes.clone();

    let axp = ax.as_ref().map_or(std::ptr::null(), |x| x as *const c2x);
    let bxp = bx.as_ref().map_or(std::ptr::null(), |x| x as *const c2x);

    let dist = unsafe {
        (api.c2GJK)(
            abytes.as_mut_ptr() as *const std::ffi::c_void,
            sa.ty,
            axp,
            bbytes.as_mut_ptr() as *const std::ffi::c_void,
            sb.ty,
            bxp,
            if sel.a {
                &mut a as *mut c2v
            } else {
                std::ptr::null_mut()
            },
            if sel.b {
                &mut b as *mut c2v
            } else {
                std::ptr::null_mut()
            },
            use_radius,
            if sel.iters {
                &mut iters as *mut i32
            } else {
                std::ptr::null_mut()
            },
            if cache_in.is_some() {
                &mut cache as *mut c2GJKCache
            } else {
                std::ptr::null_mut()
            },
        )
    };

    GjkOut {
        dist,
        a: if sel.a { Some(a) } else { None },
        b: if sel.b { Some(b) } else { None },
        iters: if sel.iters { Some(iters) } else { None },
        cache: if cache_in.is_some() { Some(cache) } else { None },
    }
}

/// Run `c2GJK` on both libraries and assert bit-identical observable output.
#[allow(clippy::too_many_arguments)]
#[track_caller]
pub fn diff_gjk(
    p: &Pair,
    what: &str,
    sa: &Shape,
    ax: Option<c2x>,
    sb: &Shape,
    bx: Option<c2x>,
    use_radius: i32,
    sel: OutSel,
    cache_in: Option<c2GJKCache>,
) {
    let oc = run_gjk(p.c, sa, ax, sb, bx, use_radius, sel, cache_in);
    let or = run_gjk(p.rs, sa, ax, sb, bx, use_radius, sel, cache_in);
    if oc.bits() != or.bits() {
        panic!(
            "DIVERGENCE in {what}\n  A: ty={} bytes={:?}\n  B: ty={} bytes={:?}\n  \
             ax={ax:?} bx={bx:?} use_radius={use_radius} sel={sel:?} cache_in={cache_in:?}\n  \
             C    = {oc:?}\n  Rust = {or:?}",
            type_name(sa.ty),
            sa.bytes,
            type_name(sb.ty),
            sb.bytes,
        );
    }
}

/// Read a shape's bytes back as a typed value (for the boolean wrappers).
pub fn read_circle(s: &Shape) -> c2Circle {
    read_as(s)
}
pub fn read_aabb(s: &Shape) -> c2AABB {
    read_as(s)
}
pub fn read_capsule(s: &Shape) -> c2Capsule {
    read_as(s)
}

fn read_as<T: Copy>(s: &Shape) -> T {
    assert!(s.bytes.len() >= std::mem::size_of::<T>());
    unsafe { std::ptr::read_unaligned(s.bytes.as_ptr() as *const T) }
}
