//! Shared harness: loads both the C `.so` and the Rust `.so` and exposes
//! typed symbol lookups so every call crosses the FFI boundary.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Types mirroring the C declarations
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
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
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [c2v; 8],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: f32,
    pub iA: c_int,
    pub iB: c_int,
}

/// C layout: `c2sv a, b, c, d; float div; int count;`
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2Simplex {
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: c_int,
}

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

// ---------------------------------------------------------------------------
// Bitwise comparison helpers
// ---------------------------------------------------------------------------

/// Byte-for-byte equality of any `Copy` value (compares raw representation).
pub fn raw_eq<T: Copy>(a: &T, b: &T) -> bool {
    let sa = unsafe {
        std::slice::from_raw_parts(a as *const T as *const u8, std::mem::size_of::<T>())
    };
    let sb = unsafe {
        std::slice::from_raw_parts(b as *const T as *const u8, std::mem::size_of::<T>())
    };
    sa == sb
}

pub fn hex<T: Copy>(a: &T) -> String {
    let s = unsafe {
        std::slice::from_raw_parts(a as *const T as *const u8, std::mem::size_of::<T>())
    };
    s.iter().map(|b| format!("{b:02x}")).collect()
}

/// Bitwise f32 equality (NaN payload and signed-zero sensitive).
pub fn f32_bits_eq(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits()
}

/// Bit-exact, except that any NaN equals any other NaN.
///
/// Needed only for inputs that are themselves NaN: on x86, `mulss`/`addss`
/// return the *destination* operand when either source is NaN, so the payload
/// and sign of the result depend on register allocation, which differs between
/// GCC -O0 and rustc. No non-NaN input can reach this ambiguity (SSE
/// synthesises the fixed "QNaN indefinite" 0xffc00000 for invalid operations),
/// so bit-exactness is asserted strictly everywhere else.
pub fn f32_eq_nan_ok(a: f32, b: f32) -> bool {
    if a.is_nan() && b.is_nan() {
        return true;
    }
    a.to_bits() == b.to_bits()
}

pub fn c2v_eq_nan_ok(a: c2v, b: c2v) -> bool {
    f32_eq_nan_ok(a.x, b.x) && f32_eq_nan_ok(a.y, b.y)
}

#[macro_export]
macro_rules! assert_raw_eq {
    ($c:expr, $r:expr, $($ctx:tt)*) => {{
        let cv = $c;
        let rv = $r;
        assert!(
            $crate::common::raw_eq(&cv, &rv),
            "mismatch: C={} ({:?}) Rust={} ({:?}) | {}",
            $crate::common::hex(&cv), cv,
            $crate::common::hex(&rv), rv,
            format!($($ctx)*)
        );
    }};
}

#[macro_export]
macro_rules! assert_f32_bits_eq {
    ($c:expr, $r:expr, $($ctx:tt)*) => {{
        let cv: f32 = $c;
        let rv: f32 = $r;
        assert!(
            cv.to_bits() == rv.to_bits(),
            "float mismatch: C={cv:?} (0x{:08x}) Rust={rv:?} (0x{:08x}) | {}",
            cv.to_bits(), rv.to_bits(), format!($($ctx)*)
        );
    }};
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation has a parent dir")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    // Allow pointing the suite at an alternative C build (e.g. a different
    // optimisation level) to confirm the reference is itself bit-stable.
    if let Some(p) = std::env::var_os("C_SO_PATH") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "C_SO_PATH={} does not exist", p.display());
        return p;
    }
    let build = workspace_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C library first.", build.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    candidates.sort();
    candidates
        .pop()
        .unwrap_or_else(|| panic!("no .so found in {}", build.display()))
}

fn find_rust_so() -> PathBuf {
    // The test binary lives in target/<profile>/deps/, so walk up to the
    // profile directory and pick up the cdylib built alongside it.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf();
    let target_dir = profile_dir.parent().expect("target/").to_path_buf();

    let mut tried = Vec::new();
    for cand in [
        profile_dir.join("libaabb_lib.so"),
        target_dir.join("debug").join("libaabb_lib.so"),
        target_dir.join("release").join("libaabb_lib.so"),
    ] {
        if cand.exists() {
            return cand;
        }
        tried.push(cand.display().to_string());
    }
    panic!(
        "Rust cdylib libaabb_lib.so not found (tried: {}). \
         Run `cargo build` (which builds the cdylib) before `cargo test`.",
        tried.join(", ")
    );
}

pub struct Libs {
    pub c: Library,
    pub rust: Library,
}

impl Libs {
    pub fn sym<T>(&self, name: &[u8]) -> (Symbol<'_, T>, Symbol<'_, T>) {
        let cn = String::from_utf8_lossy(name).to_string();
        let cs: Symbol<T> = unsafe { self.c.get(name) }
            .unwrap_or_else(|e| panic!("C .so missing symbol {cn}: {e}"));
        let rs: Symbol<T> = unsafe { self.rust.get(name) }
            .unwrap_or_else(|e| panic!("Rust .so missing symbol {cn}: {e}"));
        (cs, rs)
    }
}

static LIBS: OnceLock<Libs> = OnceLock::new();

/// Path to the C shared object under test.
pub fn c_so_path() -> PathBuf {
    find_c_so()
}

/// Path to the Rust cdylib under test.
pub fn rust_so_path() -> PathBuf {
    find_rust_so()
}

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = find_c_so();
        let r_path = find_rust_so();
        let c = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("load {}: {e}", c_path.display()));
        let rust = unsafe { Library::new(&r_path) }
            .unwrap_or_else(|e| panic!("load {}: {e}", r_path.display()));
        Libs { c, rust }
    })
}

// ---------------------------------------------------------------------------
// Deterministic value generation
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in [-1, 1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() as f64 / u32::MAX as f64) as f32 * 2.0 - 1.0
    }
    /// A "reasonable" geometry coordinate.
    pub fn coord(&mut self) -> f32 {
        self.unit() * 150.0
    }
    /// A float drawn from a pool that mixes ordinary values with edge cases.
    /// Never NaN: NaN inputs make result payloads register-allocation
    /// dependent, so they are exercised separately by `nanny()`.
    pub fn spicy(&mut self) -> f32 {
        const SPECIALS: [f32; 14] = [
            0.0,
            -0.0,
            1.0,
            -1.0,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::MIN_POSITIVE,
            -f32::MIN_POSITIVE,
            f32::MAX,
            f32::MIN,
            1.192_092_895_507_812_5e-7,
            -1.192_092_895_507_812_5e-7,
            1e-30,
            -1e-30,
        ];
        let r = self.next_u32();
        match r % 4 {
            0 => SPECIALS[(self.next_u32() as usize) % SPECIALS.len()],
            1 => {
                // Arbitrary bit pattern, resampled until it is not a NaN.
                loop {
                    let f = f32::from_bits(self.next_u32());
                    if !f.is_nan() {
                        return f;
                    }
                }
            }
            _ => self.coord(),
        }
    }

    /// Like `spicy`, but NaNs (with assorted payloads and signs) are included.
    pub fn nanny(&mut self) -> f32 {
        if self.next_u32() % 3 == 0 {
            let payload = self.next_u32() & 0x007f_ffff;
            let sign = (self.next_u32() & 1) << 31;
            // Force a quiet NaN with a non-zero mantissa.
            f32::from_bits(sign | 0x7f80_0000 | (payload | 0x0040_0000))
        } else {
            self.spicy()
        }
    }
    pub fn vec(&mut self) -> c2v {
        c2v {
            x: self.coord(),
            y: self.coord(),
        }
    }
    pub fn spicy_vec(&mut self) -> c2v {
        c2v {
            x: self.spicy(),
            y: self.spicy(),
        }
    }
    pub fn nanny_vec(&mut self) -> c2v {
        c2v {
            x: self.nanny(),
            y: self.nanny(),
        }
    }
}
