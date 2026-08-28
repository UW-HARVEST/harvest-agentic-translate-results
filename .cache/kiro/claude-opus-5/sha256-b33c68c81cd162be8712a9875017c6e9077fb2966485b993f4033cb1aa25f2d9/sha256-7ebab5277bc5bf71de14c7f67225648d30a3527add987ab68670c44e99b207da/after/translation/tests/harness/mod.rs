//! Shared loader + FFI type definitions for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded as shared objects through `libloading`, so
//! every call — including the Rust one — crosses a real FFI boundary and
//! exercises the `#[no_mangle]` export wrappers.

#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// Referencing the crate makes the lib target a dependency of this test binary,
// which is what forces cargo to (re)build it — and cargo emits every declared
// crate-type in one rustc invocation, so the `cdylib` we dlopen below is
// rebuilt too. Nothing from the rlib is actually called: every call in these
// tests goes through the shared object.
use agglom_lib as _;

/* ---------------------------- FFI types ---------------------------- */

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct C2Circle {
    pub p: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct C2Aabb {
    pub min: C2v,
    pub max: C2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct LmVec2 {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CnRnd {
    pub state: [u64; 2],
}

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;

/* ------------------------- function pointers ----------------------- */

pub type FnC2V = unsafe extern "C" fn(f32, f32) -> C2v;
pub type FnV2V = unsafe extern "C" fn(C2v, C2v) -> C2v;
pub type FnClampv = unsafe extern "C" fn(C2v, C2v, C2v) -> C2v;
pub type FnDot = unsafe extern "C" fn(C2v, C2v) -> f32;
pub type FnCirCir = unsafe extern "C" fn(C2Circle, C2Circle) -> c_int;
pub type FnCirAabb = unsafe extern "C" fn(C2Circle, C2Aabb) -> c_int;
pub type FnAabbAabb = unsafe extern "C" fn(C2Aabb, C2Aabb) -> c_int;
pub type FnF2 = unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int;
pub type FnF3 = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type FnF4 = unsafe extern "C" fn(*mut CnRnd) -> f64;
pub type FnF5 = unsafe extern "C" fn(u32) -> u32;
pub type FnF7 = unsafe extern "C" fn(u32, u32, u32) -> u32;
pub type FnF9 = unsafe extern "C" fn(LmVec2, LmVec2, LmVec2, LmVec2) -> LmVec2;
pub type FnF10 = unsafe extern "C" fn(u16) -> f32;
pub type FnTriple = unsafe extern "C" fn(*mut f32, *const f32);
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

/* ----------------------------- loading ----------------------------- */

pub struct Impls {
    pub c: Library,
    pub rust: Library,
}

impl Impls {
    pub fn sym<T>(&self, name: &str) -> (Symbol<'_, T>, Symbol<'_, T>) {
        let cname = format!("{name}\0");
        let bytes = cname.as_bytes();
        let c = unsafe { self.c.get::<T>(bytes) }
            .unwrap_or_else(|e| panic!("C .so missing `{name}`: {e}"));
        let r = unsafe { self.rust.get::<T>(bytes) }
            .unwrap_or_else(|e| panic!("Rust .so missing `{name}`: {e}"));
        (c, r)
    }
}

/// The C build directory contains exactly one shared object; its name is
/// derived from the checkout directory name, so discover it rather than
/// hard-coding it.
fn find_so(dir: &Path) -> Option<PathBuf> {
    let mut best: Option<PathBuf> = None;
    for entry in std::fs::read_dir(dir).ok()? {
        let p = entry.ok()?.path();
        if p.extension().and_then(|e| e.to_str()) == Some("so") && p.is_file() {
            best = Some(p);
        }
    }
    best
}

fn workspace_root() -> PathBuf {
    // translation/ -> parent is the working directory holding c_src/ and translation/
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

/// Locate the freshly built Rust cdylib.
///
/// `cargo test` compiles the lib target into `target/<profile>/deps/` but only
/// "uplifts" a copy to `target/<profile>/` for `cargo build`. The test binary
/// itself lives in `deps/`, so looking next to it first always finds the .so
/// that matches the current sources; the uplifted copy is only a fallback.
fn rust_so_path() -> PathBuf {
    const NAME: &str = "libagglom_lib.so";
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("test binary has a parent dir");
    let candidate = deps.join(NAME);
    if candidate.is_file() {
        return candidate;
    }
    if let Some(profile) = deps.parent() {
        let candidate = profile.join(NAME);
        if candidate.is_file() {
            return candidate;
        }
    }
    panic!("{NAME} not found near {}", deps.display());
}

pub fn impls() -> &'static Impls {
    static IMPLS: OnceLock<Impls> = OnceLock::new();
    IMPLS.get_or_init(|| {
        let root = workspace_root();
        let c_build = root.join("c_src").join("build");
        let c_path = find_so(&c_build).unwrap_or_else(|| {
            panic!(
                "no .so found in {}; build the C library first \
                 (cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .)",
                c_build.display()
            )
        });
        let r_path = rust_so_path();

        let c = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
        let rust = unsafe { Library::new(&r_path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", r_path.display()));
        Impls { c, rust }
    })
}

/* --------------------------- comparisons --------------------------- */

/// Bit-exact f32 comparison (so NaN payloads and -0.0/+0.0 are distinguished).
#[track_caller]
pub fn eq_f32(ctx: &str, c: f32, r: f32) {
    assert_eq!(
        c.to_bits(),
        r.to_bits(),
        "{ctx}: C={c:?} (0x{:08x}) != Rust={r:?} (0x{:08x})",
        c.to_bits(),
        r.to_bits()
    );
}

#[track_caller]
pub fn eq_f64(ctx: &str, c: f64, r: f64) {
    assert_eq!(
        c.to_bits(),
        r.to_bits(),
        "{ctx}: C={c:?} (0x{:016x}) != Rust={r:?} (0x{:016x})",
        c.to_bits(),
        r.to_bits()
    );
}

#[track_caller]
pub fn eq_vec2(ctx: &str, c: (f32, f32), r: (f32, f32)) {
    eq_f32(&format!("{ctx}.x"), c.0, r.0);
    eq_f32(&format!("{ctx}.y"), c.1, r.1);
}

/* ------------------------- input generation ------------------------ */

/// Deterministic splitmix64 so every run compares the same inputs.
pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
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
    /// Uniform random bit pattern reinterpreted as f32 — covers NaN/inf/denormals.
    pub fn next_f32_bits(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    /// "Tame" float in [-range, range].
    pub fn next_f32_in(&mut self, range: f32) -> f32 {
        let u = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32; // [0,1)
        (u * 2.0 - 1.0) * range
    }
}

/// Float values worth hitting exactly on every axis.
///
/// The NaN entries deliberately include both signs, non-default payloads and
/// signaling NaNs: every real mismatch found while verifying this translation
/// was a NaN sign/payload difference, driven by which SSE operand ends up as
/// the instruction destination and by x86's negative "QNaN indefinite".
pub const EDGE_F32: &[f32] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    1e-45, // smallest denormal
    -1e-45,
    f32::MAX,
    f32::MIN,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
    -f32::NAN,
    60.0,
    120.0,
    180.0,
    240.0,
    300.0,
    360.0,
    359.999_97,
    2.0,
    -2.0,
    255.0,
    16_777_216.0,
    -16_777_216.0,
    2_147_483_648.0,
    -2_147_483_648.0,
    2_147_483_520.0,
    -2_147_483_520.0,
    1e30,
    -1e30,
];

/// NaN bit patterns: quiet/signaling, both signs, default and custom payloads.
pub const NAN_BITS: &[u32] = &[
    0x7FC0_0000, // default quiet NaN, positive
    0xFFC0_0000, // x86 "QNaN indefinite" (what an invalid op produces)
    0x7FC0_DEAD,
    0xFFC0_DEAD,
    0x7FFF_FFFF,
    0xFFFF_FFFF,
    0x7F80_0001, // signaling NaN, positive
    0xFF80_0001, // signaling NaN, negative
    0x7FBF_FFFF,
    0xFFBF_FFFF,
];

pub fn nan_pool() -> Vec<f32> {
    NAN_BITS.iter().map(|&b| f32::from_bits(b)).collect()
}

/// `EDGE_F32` plus every NaN pattern and both infinities.
pub fn edge_plus_nans() -> Vec<f32> {
    let mut v = EDGE_F32.to_vec();
    v.extend(nan_pool());
    v
}

pub const EDGE_I32: &[i32] = &[
    0,
    1,
    -1,
    2,
    -2,
    3,
    -3,
    7,
    -7,
    i32::MAX,
    i32::MIN,
    i32::MAX - 1,
    i32::MIN + 1,
    1_000_000,
    -1_000_000,
    0x4000_0000,
    -0x4000_0000,
];

pub const EDGE_U32: &[u32] = &[
    0,
    1,
    2,
    3,
    4,
    7,
    8,
    16,
    31,
    32,
    33,
    255,
    256,
    0xFFFF,
    0x1_0000,
    0x7FFF_FFFF,
    0x8000_0000,
    0xFFFF_FFFF,
    0xAAAA_AAAA,
    0x5555_5555,
    4096,
    4608,
];

pub const EDGE_U64: &[u64] = &[
    0,
    1,
    2,
    u64::MAX,
    u64::MAX - 1,
    0x8000_0000_0000_0000,
    0x1234_5678_9ABC_DEF0,
    0xDEAD_BEEF_CAFE_BABE,
    0xFFFF_FFFF_0000_0000,
    0x0000_0000_FFFF_FFFF,
    0xAAAA_AAAA_AAAA_AAAA,
    0x5555_5555_5555_5555,
];
