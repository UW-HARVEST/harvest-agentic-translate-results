//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and exposes a symmetric pair
//! of vtables so every test can call the C implementation and the Rust
//! implementation through the identical FFI path. The Rust crate is *never*
//! called directly — only through `libagglom_lib.so`'s `#[no_mangle]` exports.

#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_uint, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// ABI-compatible mirrors of the C types
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct C2Circle {
    pub p: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct C2Aabb {
    pub min: C2v,
    pub max: C2v,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct LmVec2 {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct CnRnd {
    pub state: [u64; 2],
}

pub const C2_TYPE_CIRCLE: c_uint = 0;
pub const C2_TYPE_AABB: c_uint = 1;

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

/// Bit-level equality for `f32` (so NaN payload and signed zero are compared).
pub trait BitEq {
    type Repr: std::fmt::Debug + PartialEq;
    fn repr(&self) -> Self::Repr;
}

impl BitEq for f32 {
    type Repr = u32;
    fn repr(&self) -> u32 {
        self.to_bits()
    }
}
impl BitEq for f64 {
    type Repr = u64;
    fn repr(&self) -> u64 {
        self.to_bits()
    }
}
impl BitEq for C2v {
    type Repr = (u32, u32);
    fn repr(&self) -> (u32, u32) {
        (self.x.to_bits(), self.y.to_bits())
    }
}
impl BitEq for LmVec2 {
    type Repr = (u32, u32);
    fn repr(&self) -> (u32, u32) {
        (self.x.to_bits(), self.y.to_bits())
    }
}
impl BitEq for [f32; 3] {
    type Repr = (u32, u32, u32);
    fn repr(&self) -> (u32, u32, u32) {
        (self[0].to_bits(), self[1].to_bits(), self[2].to_bits())
    }
}
impl BitEq for i32 {
    type Repr = i32;
    fn repr(&self) -> i32 {
        *self
    }
}
impl BitEq for u32 {
    type Repr = u32;
    fn repr(&self) -> u32 {
        *self
    }
}
impl BitEq for u64 {
    type Repr = u64;
    fn repr(&self) -> u64 {
        *self
    }
}
impl BitEq for [u64; 2] {
    type Repr = [u64; 2];
    fn repr(&self) -> [u64; 2] {
        *self
    }
}
impl BitEq for (u32, u32, u32) {
    type Repr = (u32, u32, u32);
    fn repr(&self) -> (u32, u32, u32) {
        *self
    }
}
impl<const K: usize> BitEq for [u32; K] {
    type Repr = [u32; K];
    fn repr(&self) -> [u32; K] {
        *self
    }
}

// ---------------------------------------------------------------------------
// Function-pointer types
// ---------------------------------------------------------------------------

pub type FnC2V = unsafe extern "C" fn(f32, f32) -> C2v;
pub type FnV2 = unsafe extern "C" fn(C2v, C2v) -> C2v;
pub type FnV3 = unsafe extern "C" fn(C2v, C2v, C2v) -> C2v;
pub type FnDot = unsafe extern "C" fn(C2v, C2v) -> f32;
pub type FnCC = unsafe extern "C" fn(C2Circle, C2Circle) -> c_int;
pub type FnCA = unsafe extern "C" fn(C2Circle, C2Aabb) -> c_int;
pub type FnAA = unsafe extern "C" fn(C2Aabb, C2Aabb) -> c_int;
pub type FnF2 = unsafe extern "C" fn(*const c_void, c_uint, *const c_void, c_uint) -> c_int;
pub type FnF3 = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type FnF4 = unsafe extern "C" fn(*mut CnRnd) -> f64;
pub type FnF5 = unsafe extern "C" fn(u32) -> u32;
pub type FnF7 = unsafe extern "C" fn(u32, u32, u32) -> u32;
pub type FnF9 = unsafe extern "C" fn(LmVec2, LmVec2, LmVec2, LmVec2) -> LmVec2;
pub type FnF10 = unsafe extern "C" fn(u16) -> f32;
pub type FnF1x = unsafe extern "C" fn(*mut f32, *const f32);
#[rustfmt::skip]
pub type FnAgglom = unsafe extern "C" fn(
    f32, f32, f32, f32, f32, f32, f32,
    c_int, c_int,
    u64, u64,
    u32,
    u32, u32, u32,
    f32, f32, f32, f32, f32, f32, f32, f32,
    u16,
    f32, f32, f32,
    f32, f32, f32,
    f32, f32, f32,
) -> f64;

/// The complete public surface of one implementation.
pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    pub c2V: FnC2V,
    pub c2Maxv: FnV2,
    pub c2Minv: FnV2,
    pub c2Clampv: FnV3,
    pub c2Sub: FnV2,
    pub c2Dot: FnDot,
    pub c2CircletoCircle: FnCC,
    pub c2CircletoAABB: FnCA,
    pub c2AABBtoAABB: FnAA,
    pub f2: FnF2,
    pub f3: FnF3,
    pub f4: FnF4,
    pub f5: FnF5,
    pub f7: FnF7,
    pub f9: FnF9,
    pub f10: FnF10,
    pub f11: FnF1x,
    pub f12: FnF1x,
    pub f13: FnF1x,
    pub agglom: FnAgglom,
}

unsafe fn sym<T: Copy>(lib: &Library, name: &[u8]) -> T {
    let s: Symbol<T> = unsafe {
        lib.get(name).unwrap_or_else(|e| {
            panic!(
                "missing symbol `{}`: {e}",
                String::from_utf8_lossy(&name[..name.len() - 1])
            )
        })
    };
    *s
}

impl Impl {
    unsafe fn load(name: &'static str, path: &PathBuf) -> Impl {
        let lib = unsafe {
            Library::new(path).unwrap_or_else(|e| panic!("cannot load {}: {e}", path.display()))
        };
        unsafe {
            Impl {
                name,
                c2V: sym(&lib, b"c2V\0"),
                c2Maxv: sym(&lib, b"c2Maxv\0"),
                c2Minv: sym(&lib, b"c2Minv\0"),
                c2Clampv: sym(&lib, b"c2Clampv\0"),
                c2Sub: sym(&lib, b"c2Sub\0"),
                c2Dot: sym(&lib, b"c2Dot\0"),
                c2CircletoCircle: sym(&lib, b"c2CircletoCircle\0"),
                c2CircletoAABB: sym(&lib, b"c2CircletoAABB\0"),
                c2AABBtoAABB: sym(&lib, b"c2AABBtoAABB\0"),
                f2: sym(&lib, b"f2\0"),
                f3: sym(&lib, b"f3\0"),
                f4: sym(&lib, b"f4\0"),
                f5: sym(&lib, b"f5\0"),
                f7: sym(&lib, b"f7\0"),
                f9: sym(&lib, b"f9\0"),
                f10: sym(&lib, b"f10\0"),
                f11: sym(&lib, b"f11\0"),
                f12: sym(&lib, b"f12\0"),
                f13: sym(&lib, b"f13\0"),
                agglom: sym(&lib, b"agglom\0"),
                _lib: lib,
            }
        }
    }
}

pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src").join("build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                found.push(p);
            }
        }
    }
    found.sort();
    found.pop().unwrap_or_else(|| {
        panic!(
            "no .so in {}. Build the C library first:\n  cd c_src && mkdir -p build && cd build \
             && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    // The integration-test executable lives in target/<profile>/deps/, so the
    // cdylib we must load is its sibling in target/<profile>/.
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile_dir = deps.parent().expect("profile dir");
    let direct = profile_dir.join("libagglom_lib.so");
    if direct.exists() {
        return direct;
    }
    for cand in ["release", "debug"] {
        let p = workspace_root()
            .join("translation")
            .join("target")
            .join(cand)
            .join("libagglom_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libagglom_lib.so not found near {}. Run `cargo build` for this profile first.",
        profile_dir.display()
    );
}

static PAIR: OnceLock<Pair> = OnceLock::new();

pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| {
        let c_path = find_c_so();
        let rs_path = find_rust_so();
        eprintln!("[harness] C   .so: {}", c_path.display());
        eprintln!("[harness] Rust .so: {}", rs_path.display());
        unsafe {
            Pair {
                c: Impl::load("C", &c_path),
                rs: Impl::load("Rust", &rs_path),
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Assertion helper
// ---------------------------------------------------------------------------

#[track_caller]
pub fn same<T: BitEq>(what: &str, ctx: impl std::fmt::Debug, c: T, rs: T) {
    let (a, b) = (c.repr(), rs.repr());
    assert!(
        a == b,
        "DIVERGENCE in {what}\n  input : {ctx:?}\n  C     : {a:?}\n  Rust  : {b:?}"
    );
}

// ---------------------------------------------------------------------------
// Deterministic RNG (splitmix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x243F_6A88_85A3_08D3;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
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
    pub fn next_u16(&mut self) -> u16 {
        (self.next_u64() >> 48) as u16
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `[0, n)`.
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }

    /// A completely arbitrary 32-bit pattern reinterpreted as `f32`
    /// (NaNs, infinities, subnormals and huge values all appear).
    pub fn raw_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }

    /// A "reasonable" float in `[-scale, scale]`, occasionally a special value.
    pub fn nice_f32(&mut self, scale: f32) -> f32 {
        let r = self.next_u32();
        match r % 32 {
            0 => 0.0,
            1 => -0.0,
            2 => f32::INFINITY,
            3 => f32::NEG_INFINITY,
            4 => f32::NAN,
            5 => f32::from_bits(0xFFC0_0001), // negative NaN, payload 1
            6 => f32::from_bits(0x7F80_0001), // "signaling" NaN pattern
            7 => f32::MAX,
            8 => f32::MIN,
            9 => f32::MIN_POSITIVE,
            10 => f32::from_bits(1), // smallest subnormal
            11 => f32::from_bits(0x8000_0001),
            12 => 1.0,
            13 => -1.0,
            _ => {
                let u = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32; // [0,1)
                (u * 2.0 - 1.0) * scale
            }
        }
    }

    /// A float that is finite and in `[-scale, scale]`.
    pub fn finite_f32(&mut self, scale: f32) -> f32 {
        let u = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32;
        (u * 2.0 - 1.0) * scale
    }

    /// A finite float in `[lo, hi)`.
    pub fn range_f32(&mut self, lo: f32, hi: f32) -> f32 {
        let u = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32;
        lo + u * (hi - lo)
    }

    pub fn c2v(&mut self, scale: f32) -> C2v {
        C2v {
            x: self.nice_f32(scale),
            y: self.nice_f32(scale),
        }
    }
    pub fn raw_c2v(&mut self) -> C2v {
        C2v {
            x: self.raw_f32(),
            y: self.raw_f32(),
        }
    }
    pub fn circle(&mut self, scale: f32) -> C2Circle {
        C2Circle {
            p: self.c2v(scale),
            r: self.nice_f32(scale),
        }
    }
    pub fn aabb(&mut self, scale: f32) -> C2Aabb {
        C2Aabb {
            min: self.c2v(scale),
            max: self.c2v(scale),
        }
    }
    pub fn lmv(&mut self, scale: f32) -> LmVec2 {
        LmVec2 {
            x: self.nice_f32(scale),
            y: self.nice_f32(scale),
        }
    }
    pub fn raw_lmv(&mut self) -> LmVec2 {
        LmVec2 {
            x: self.raw_f32(),
            y: self.raw_f32(),
        }
    }

    /// Boundary-biased `i32`.
    pub fn edgy_i32(&mut self) -> i32 {
        let r = self.next_u32();
        match r % 24 {
            0 => 0,
            1 => 1,
            2 => -1,
            3 => i32::MIN,
            4 => i32::MAX,
            5 => i32::MIN + 1,
            6 => i32::MAX - 1,
            7 => 2,
            8 => -2,
            9 => 3,
            10 => -3,
            11 => 1 << 30,
            12 => -(1 << 30),
            _ => self.next_i32(),
        }
    }

    /// Boundary-biased `u32`.
    pub fn edgy_u32(&mut self) -> u32 {
        let r = self.next_u32();
        match r % 20 {
            0 => 0,
            1 => 1,
            2 => 2,
            3 => u32::MAX,
            4 => u32::MAX - 1,
            5 => 0xFFFF,
            6 => 0x1_0000,
            7 => 0x8000_0000,
            8 => 8,
            9 => 16,
            10 => 24,
            11 => 32,
            12 => 4096,
            _ => self.next_u32(),
        }
    }

    /// Boundary-biased `u64`.
    pub fn edgy_u64(&mut self) -> u64 {
        let r = self.next_u32();
        match r % 12 {
            0 => 0,
            1 => 1,
            2 => u64::MAX,
            3 => 1u64 << 63,
            4 => u64::MAX >> 1,
            _ => self.next_u64(),
        }
    }
}

// ---------------------------------------------------------------------------
// Interesting scalar corpora used by many tests
// ---------------------------------------------------------------------------

/// Every "class" of `f32` a caller can pass across the FFI boundary.
pub const SPECIAL_F32: &[u32] = &[
    0x0000_0000, // +0.0
    0x8000_0000, // -0.0
    0x0000_0001, // smallest positive subnormal
    0x8000_0001, // smallest negative subnormal
    0x007F_FFFF, // largest subnormal
    0x0080_0000, // FLT_MIN
    0x3F80_0000, // 1.0
    0xBF80_0000, // -1.0
    0x3F00_0000, // 0.5
    0x4270_0000, // 60.0
    0x42F0_0000, // 120.0
    0x4334_0000, // 180.0
    0x4370_0000, // 240.0
    0x4396_0000, // 300.0
    0x43B4_0000, // 360.0
    0x7F7F_FFFF, // FLT_MAX
    0xFF7F_FFFF, // -FLT_MAX
    0x7F80_0000, // +inf
    0xFF80_0000, // -inf
    0x7FC0_0000, // qNaN
    0xFFC0_0000, // -qNaN
    0x7FC0_0001, // qNaN payload 1
    0x7F80_0001, // sNaN
    0xFFFF_FFFF, // -NaN all ones
    0x4B00_0000, // 2^23
    0x4F00_0000, // 2^31
    0xCF00_0000, // -2^31
    0x5F00_0000, // 2^63
];

pub fn special_f32_values() -> Vec<f32> {
    SPECIAL_F32.iter().map(|&b| f32::from_bits(b)).collect()
}

pub const SPECIAL_I32: &[i32] = &[
    i32::MIN,
    i32::MIN + 1,
    i32::MIN + 2,
    -1073741824,
    -100000,
    -1000,
    -100,
    -7,
    -3,
    -2,
    -1,
    0,
    1,
    2,
    3,
    7,
    100,
    1000,
    100000,
    1073741824,
    i32::MAX - 2,
    i32::MAX - 1,
    i32::MAX,
];

pub const SPECIAL_U32: &[u32] = &[
    0,
    1,
    2,
    3,
    4,
    5,
    7,
    8,
    16,
    24,
    31,
    32,
    33,
    64,
    255,
    256,
    4096,
    65535,
    65536,
    0x00FF_FFFF,
    0x0100_0000,
    0x7FFF_FFFF,
    0x8000_0000,
    0xFFFF_FFFE,
    0xFFFF_FFFF,
];

pub const SPECIAL_U64: &[u64] = &[
    0,
    1,
    2,
    3,
    0xFF,
    0xFFFF,
    0xFFFF_FFFF,
    0x1_0000_0000,
    0x0123_4567_89AB_CDEF,
    0xFEDC_BA98_7654_3210,
    1u64 << 22,
    1u64 << 23,
    1u64 << 26,
    1u64 << 63,
    u64::MAX >> 1,
    u64::MAX - 1,
    u64::MAX,
];

// ---------------------------------------------------------------------------
// Convenience wrappers around the pointer-taking triples
// ---------------------------------------------------------------------------

pub fn call_f1x(f: FnF1x, src: [f32; 3]) -> [f32; 3] {
    let mut dest = [0.0f32; 3];
    unsafe { f(dest.as_mut_ptr(), src.as_ptr()) };
    dest
}

/// Call with `dest == src` (full aliasing).
pub fn call_f1x_aliased(f: FnF1x, src: [f32; 3]) -> [f32; 3] {
    let mut buf = src;
    let p = buf.as_mut_ptr();
    unsafe { f(p, p) };
    buf
}
