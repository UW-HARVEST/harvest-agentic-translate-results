//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and exposes one struct per
//! library holding every exported symbol.  Nothing in here calls a Rust
//! function directly — every call crosses the `.so` boundary, exactly as an
//! external C consumer would, so the `#[no_mangle]` wrappers and the struct
//! ABI are under test too.

#![allow(non_snake_case, dead_code)]

use libloading::{Library, Symbol};
use std::os::raw::{c_int, c_uint, c_void};
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// C-compatible types (mirrors of the definitions in c_src/src/lib.c)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct C2Circle {
    pub p: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct C2Aabb {
    pub min: C2v,
    pub max: C2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct LmVec2 {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct CnRnd {
    pub state: [u64; 2],
}

pub const C2_TYPE_CIRCLE: c_uint = 0;
pub const C2_TYPE_AABB: c_uint = 1;

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

pub fn bits32(v: f32) -> u32 {
    v.to_bits()
}
pub fn bits64(v: f64) -> u64 {
    v.to_bits()
}

/// Bit-for-bit equality of two `c2v`/`lm_vec2`-shaped values.
pub fn v2_bits(x: f32, y: f32) -> (u32, u32) {
    (x.to_bits(), y.to_bits())
}

// ---------------------------------------------------------------------------
// Symbol table
// ---------------------------------------------------------------------------

type FnC2V = unsafe extern "C" fn(f32, f32) -> C2v;
type FnC2Bin = unsafe extern "C" fn(C2v, C2v) -> C2v;
type FnC2Clamp = unsafe extern "C" fn(C2v, C2v, C2v) -> C2v;
type FnC2Dot = unsafe extern "C" fn(C2v, C2v) -> f32;
type FnCC = unsafe extern "C" fn(C2Circle, C2Circle) -> c_int;
type FnCA = unsafe extern "C" fn(C2Circle, C2Aabb) -> c_int;
type FnAA = unsafe extern "C" fn(C2Aabb, C2Aabb) -> c_int;
type FnF2 = unsafe extern "C" fn(*const c_void, c_uint, *const c_void, c_uint) -> c_int;
type FnF3 = unsafe extern "C" fn(c_int, c_int) -> c_int;
type FnF4 = unsafe extern "C" fn(*mut CnRnd) -> f64;
type FnF5 = unsafe extern "C" fn(u32) -> u32;
type FnF7 = unsafe extern "C" fn(u32, u32, u32) -> u32;
type FnF9 = unsafe extern "C" fn(LmVec2, LmVec2, LmVec2, LmVec2) -> LmVec2;
type FnF10 = unsafe extern "C" fn(u16) -> f32;
type FnTriple = unsafe extern "C" fn(*mut f32, *const f32);

#[allow(clippy::type_complexity)]
type FnAgglom = unsafe extern "C" fn(
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    c_int,
    c_int,
    u64,
    u64,
    u32,
    u32,
    u32,
    u32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    u16,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
) -> f64;

/// All 33 `agglom` arguments in declaration order.
#[derive(Copy, Clone, Debug)]
pub struct AgglomArgs {
    pub f2_1: f32,
    pub f2_2: f32,
    pub f2_3: f32,
    pub f2_7: f32,
    pub f2_8: f32,
    pub f2_9: f32,
    pub f2_10: f32,
    pub f3_1: c_int,
    pub f3_2: c_int,
    pub f4_1: u64,
    pub f4_2: u64,
    pub f5_1: u32,
    pub f7_1: u32,
    pub f7_2: u32,
    pub f7_3: u32,
    pub f9_1: f32,
    pub f9_2: f32,
    pub f9_4: f32,
    pub f9_5: f32,
    pub f9_7: f32,
    pub f9_8: f32,
    pub f9_10: f32,
    pub f9_11: f32,
    pub f10_1: u16,
    pub f11_2: f32,
    pub f11_3: f32,
    pub f11_4: f32,
    pub f12_2: f32,
    pub f12_3: f32,
    pub f12_4: f32,
    pub f13_2: f32,
    pub f13_3: f32,
    pub f13_4: f32,
}

impl Default for AgglomArgs {
    /// A "tame" baseline: every sub-function gets ordinary, in-range values so
    /// that a single-axis sweep isolates one sub-function at a time.
    fn default() -> Self {
        AgglomArgs {
            f2_1: 0.5,
            f2_2: 0.5,
            f2_3: 1.0,
            f2_7: 0.0,
            f2_8: 0.0,
            f2_9: 2.0,
            f2_10: 2.0,
            f3_1: 7,
            f3_2: 3,
            f4_1: 0x1234_5678_9abc_def0,
            f4_2: 0x0fed_cba9_8765_4321,
            f5_1: 0x0000_1234,
            f7_1: 4096,
            f7_2: 2,
            f7_3: 16,
            f9_1: 0.0,
            f9_2: 0.0,
            f9_4: 1.0,
            f9_5: 0.0,
            f9_7: 0.0,
            f9_8: 1.0,
            f9_10: 0.25,
            f9_11: 0.25,
            f10_1: 0x3c00,
            f11_2: 30.0,
            f11_3: 0.5,
            f11_4: 0.5,
            f12_2: 30.0,
            f12_3: 0.5,
            f12_4: 0.5,
            f13_2: 0.75,
            f13_3: 0.5,
            f13_4: 0.25,
        }
    }
}

pub struct Lib {
    pub name: &'static str,
    _lib: Library,
    pub c2V: FnC2V,
    pub c2Maxv: FnC2Bin,
    pub c2Minv: FnC2Bin,
    pub c2Clampv: FnC2Clamp,
    pub c2Sub: FnC2Bin,
    pub c2Dot: FnC2Dot,
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
    pub f11: FnTriple,
    pub f12: FnTriple,
    pub f13: FnTriple,
    pub agglom: FnAgglom,
}

macro_rules! get {
    ($lib:expr, $ty:ty, $name:expr) => {{
        let s: Symbol<$ty> = unsafe {
            $lib.get($name)
                .unwrap_or_else(|e| panic!("symbol {:?} missing: {e}", $name))
        };
        *s
    }};
}

impl Lib {
    fn open(name: &'static str, path: &PathBuf) -> Lib {
        let lib = unsafe {
            Library::new(path)
                .unwrap_or_else(|e| panic!("cannot dlopen {}: {e}", path.display()))
        };
        let l = Lib {
            name,
            c2V: get!(lib, FnC2V, b"c2V\0"),
            c2Maxv: get!(lib, FnC2Bin, b"c2Maxv\0"),
            c2Minv: get!(lib, FnC2Bin, b"c2Minv\0"),
            c2Clampv: get!(lib, FnC2Clamp, b"c2Clampv\0"),
            c2Sub: get!(lib, FnC2Bin, b"c2Sub\0"),
            c2Dot: get!(lib, FnC2Dot, b"c2Dot\0"),
            c2CircletoCircle: get!(lib, FnCC, b"c2CircletoCircle\0"),
            c2CircletoAABB: get!(lib, FnCA, b"c2CircletoAABB\0"),
            c2AABBtoAABB: get!(lib, FnAA, b"c2AABBtoAABB\0"),
            f2: get!(lib, FnF2, b"f2\0"),
            f3: get!(lib, FnF3, b"f3\0"),
            f4: get!(lib, FnF4, b"f4\0"),
            f5: get!(lib, FnF5, b"f5\0"),
            f7: get!(lib, FnF7, b"f7\0"),
            f9: get!(lib, FnF9, b"f9\0"),
            f10: get!(lib, FnF10, b"f10\0"),
            f11: get!(lib, FnTriple, b"f11\0"),
            f12: get!(lib, FnTriple, b"f12\0"),
            f13: get!(lib, FnTriple, b"f13\0"),
            agglom: get!(lib, FnAgglom, b"agglom\0"),
            _lib: lib,
        };
        l
    }

    /// `f11`/`f12`/`f13` share the `(dest, src)` shape.
    pub fn call_triple(&self, which: Triple, src: [f32; 3]) -> [f32; 3] {
        let f = match which {
            Triple::F11 => self.f11,
            Triple::F12 => self.f12,
            Triple::F13 => self.f13,
        };
        // Pre-fill with a recognisable pattern so a "not written" slot is
        // distinguishable from a legitimately-written value.
        let mut dest: [f32; 3] = [0.0, 0.0, 0.0];
        unsafe { f(dest.as_mut_ptr(), src.as_ptr()) };
        dest
    }

    pub fn call_agglom(&self, a: &AgglomArgs) -> f64 {
        unsafe {
            (self.agglom)(
                a.f2_1, a.f2_2, a.f2_3, a.f2_7, a.f2_8, a.f2_9, a.f2_10, a.f3_1, a.f3_2, a.f4_1,
                a.f4_2, a.f5_1, a.f7_1, a.f7_2, a.f7_3, a.f9_1, a.f9_2, a.f9_4, a.f9_5, a.f9_7,
                a.f9_8, a.f9_10, a.f9_11, a.f10_1, a.f11_2, a.f11_3, a.f11_4, a.f12_2, a.f12_3,
                a.f12_4, a.f13_2, a.f13_3, a.f13_4,
            )
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Triple {
    F11,
    F12,
    F13,
}

impl Triple {
    pub fn name(self) -> &'static str {
        match self {
            Triple::F11 => "f11",
            Triple::F12 => "f12",
            Triple::F13 => "f13",
        }
    }
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

fn find_c_so() -> PathBuf {
    let dir = workspace_root().join("c_src/build");
    let entries = std::fs::read_dir(&dir).unwrap_or_else(|e| {
        panic!(
            "c_src/build not found ({e}); build the C library first:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
        )
    });
    let mut found = None;
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().map(|x| x == "so").unwrap_or(false) {
            found = Some(p);
        }
    }
    found.unwrap_or_else(|| panic!("no .so in {}", dir.display()))
}

/// `cargo test` builds the library as an rlib for the test binaries; it does
/// NOT regenerate the `cdylib`.  Loading `target/<profile>/libagglom_lib.so`
/// would therefore silently test a STALE `.so` (verified: a deliberately
/// injected bug in `f5` went undetected).  So build the cdylib explicitly here,
/// into a separate target directory to avoid the lock cargo holds on `target/`.
fn build_rust_so() -> PathBuf {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT
        .get_or_init(|| {
            let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let target_dir = manifest.join("target/so-under-test");
            let profile_dir = if cfg!(debug_assertions) {
                "debug"
            } else {
                "release"
            };
            let mut cmd = Command::new(env!("CARGO"));
            cmd.current_dir(&manifest)
                .arg("build")
                .arg("--lib")
                .arg("--target-dir")
                .arg(&target_dir);
            if !cfg!(debug_assertions) {
                cmd.arg("--release");
            }
            // Inherit the feature selection the test binary was compiled with,
            // so `--no-default-features --features X` tests the matching `.so`.
            if let Ok(f) = std::env::var("AGGLOM_TEST_FEATURES") {
                cmd.arg("--no-default-features");
                if !f.is_empty() {
                    cmd.arg("--features").arg(&f);
                }
            }
            // Do not let the parent cargo's env confuse the child build.
            for k in [
                "RUSTC_WORKSPACE_WRAPPER",
                "CARGO_MAKEFLAGS",
                "RUSTUP_TOOLCHAIN_ARG",
            ] {
                cmd.env_remove(k);
            }
            let out = cmd
                .output()
                .unwrap_or_else(|e| panic!("failed to spawn cargo to build the cdylib: {e}"));
            if !out.status.success() {
                panic!(
                    "building the Rust cdylib failed:\n--- stdout ---\n{}\n--- stderr ---\n{}",
                    String::from_utf8_lossy(&out.stdout),
                    String::from_utf8_lossy(&out.stderr)
                );
            }
            let so = target_dir.join(profile_dir).join("libagglom_lib.so");
            assert!(
                so.exists(),
                "cargo build succeeded but {} does not exist",
                so.display()
            );
            so
        })
        .clone()
}

/// The two loaded libraries, C first.
pub struct Pair {
    pub c: Lib,
    pub r: Lib,
}

pub fn load() -> Pair {
    Pair {
        c: Lib::open("C", &find_c_so()),
        r: Lib::open("Rust", &build_rust_so()),
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64) — fixed seed, reproducible
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

pub struct Rng(pub u64);

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
    /// A float drawn from the *entire* 32-bit bit space: NaNs (all payloads),
    /// infinities, subnormals and zeros all appear with their natural density.
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    /// A "tame" float in `[-scale, scale]` — no NaN/inf.
    pub fn tame_f32(&mut self, scale: f32) -> f32 {
        let u = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32; // [0,1)
        (u * 2.0 - 1.0) * scale
    }
    pub fn range_f32(&mut self, lo: f32, hi: f32) -> f32 {
        let u = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32;
        lo + u * (hi - lo)
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[(self.next_u32() as usize) % xs.len()]
    }
}

/// Interesting f32 values: zeros, subnormals, boundaries, infinities, NaNs
/// (both quiet and signalling, both signs, several payloads).
pub const SPECIAL_F32: &[f32] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    2.0,
    -2.0,
    60.0,
    120.0,
    180.0,
    240.0,
    300.0,
    360.0,
    -60.0,
    359.999_97,
    1e-45,  // smallest positive subnormal
    -1e-45,
    1.175_494_4e-38, // FLT_MIN
    3.402_823_5e38,  // FLT_MAX
    -3.402_823_5e38,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
];

/// NaN bit patterns worth passing across the boundary explicitly.
pub const NAN_BITS: &[u32] = &[
    0x7FC0_0000, // canonical quiet NaN
    0xFFC0_0000, // negative quiet NaN
    0x7F80_0001, // signalling NaN, payload 1
    0xFF80_0001, // negative signalling NaN
    0x7FFF_FFFF, // quiet NaN, all payload bits set
    0xFFAA_5555, // negative NaN, mixed payload
];

pub fn all_special_f32() -> Vec<f32> {
    let mut v: Vec<f32> = SPECIAL_F32.to_vec();
    for &b in NAN_BITS {
        v.push(f32::from_bits(b));
    }
    v
}
