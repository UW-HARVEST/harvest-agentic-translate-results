//! Shared differential-test harness.
//!
//! Loads BOTH shared libraries through `libloading` and calls
//! `to_barycentric` across the real FFI boundary in both. The Rust function is
//! never called directly from Rust — it is always reached via the `.so`'s
//! exported symbol, so the `#[no_mangle] extern "C"` wrapper and the SysV
//! struct-passing convention are under test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::sync::OnceLock;

/// Mirror of `typedef struct lm_vec2 { float x, y; } lm_vec2;`
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const fn new(x: f32, y: f32) -> Self {
        Vec2 { x, y }
    }
    pub const fn from_bits(x: u32, y: u32) -> Self {
        Vec2 {
            x: f32::from_bits(x),
            y: f32::from_bits(y),
        }
    }
    pub fn bits(self) -> (u32, u32) {
        (self.x.to_bits(), self.y.to_bits())
    }
}

pub type ToBarycentric = unsafe extern "C" fn(Vec2, Vec2, Vec2, Vec2) -> Vec2;

// ---------------------------------------------------------------------------
// library discovery
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // .../<root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn glob_so(dir: &PathBuf, prefix: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut hits: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            let name = match p.file_name().and_then(|n| n.to_str()) {
                Some(n) => n,
                None => return false,
            };
            name.starts_with(prefix) && name.ends_with(".so")
        })
        .collect();
    hits.sort();
    hits.pop()
}

/// `c_src/build/lib<project>.so`, built by CMake.
fn c_library_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_LIB_PATH") {
        return PathBuf::from(p);
    }
    let build_dir = workspace_root().join("c_src").join("build");
    glob_so(&build_dir, "lib").unwrap_or_else(|| {
        panic!(
            "no C shared library found in {}.\n\
             Build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build_dir.display()
        )
    })
}

/// `translation/target/<profile>/libto_barycentric_lib.so`.
///
/// Derived from the running test executable's own path
/// (`target/<profile>/deps/<test>`), so it always picks the `.so` from the same
/// profile the tests were compiled in.
fn rust_library_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_LIB_PATH") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent() // deps/
        .and_then(|p| p.parent()) // <profile>/
        .expect("test exe layout")
        .to_path_buf();
    let direct = profile_dir.join("libto_barycentric_lib.so");
    if direct.exists() {
        return direct;
    }
    glob_so(&profile_dir, "libto_barycentric_lib").unwrap_or_else(|| {
        panic!(
            "no Rust cdylib found in {}.\nBuild it with: cargo build (same profile as the tests)",
            profile_dir.display()
        )
    })
}

struct Libs {
    _c: Library,
    _rust: Library,
    c_fn: ToBarycentric,
    rust_fn: ToBarycentric,
}

// SAFETY: the loaded function pointers are plain leaf math routines with no
// interior mutability and no global state; the `Library` handles are kept alive
// for the whole process lifetime and never unloaded.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

static LIBS: OnceLock<Libs> = OnceLock::new();

fn libs() -> &'static Libs {
    LIBS.get_or_init(|| unsafe {
        let c_path = c_library_path();
        let rust_path = rust_library_path();

        let c = Library::new(&c_path)
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", c_path.display()));
        let rust = Library::new(&rust_path)
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", rust_path.display()));

        let c_sym: Symbol<ToBarycentric> = c
            .get(b"to_barycentric\0")
            .unwrap_or_else(|e| panic!("to_barycentric missing from C .so: {e}"));
        let rust_sym: Symbol<ToBarycentric> = rust
            .get(b"to_barycentric\0")
            .unwrap_or_else(|e| panic!("to_barycentric missing from Rust .so: {e}"));

        let c_fn = *c_sym;
        let rust_fn = *rust_sym;

        eprintln!("  C   .so: {}", c_path.display());
        eprintln!("  Rust.so: {}", rust_path.display());

        Libs {
            c_fn,
            rust_fn,
            _c: c,
            _rust: rust,
        }
    })
}

pub fn call_c(p1: Vec2, p2: Vec2, p3: Vec2, p: Vec2) -> Vec2 {
    unsafe { (libs().c_fn)(p1, p2, p3, p) }
}

pub fn call_rust(p1: Vec2, p2: Vec2, p3: Vec2, p: Vec2) -> Vec2 {
    unsafe { (libs().rust_fn)(p1, p2, p3, p) }
}

// ---------------------------------------------------------------------------
// comparison
// ---------------------------------------------------------------------------

fn fmt(v: Vec2) -> String {
    format!(
        "{{x: 0x{:08x} ({}), y: 0x{:08x} ({})}}",
        v.x.to_bits(),
        v.x,
        v.y.to_bits(),
        v.y
    )
}

fn fmt_args(a: [Vec2; 4]) -> String {
    let names = ["p1", "p2", "p3", "p "];
    a.iter()
        .zip(names)
        .map(|(v, n)| format!("    {n} = {}", fmt(*v)))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Calls both `.so`s and asserts the returned `lm_vec2` is bit-identical.
/// Returns the (shared) result so callers can do extra sanity checks.
#[track_caller]
pub fn assert_same(row: &str, args: [Vec2; 4]) -> Vec2 {
    let [p1, p2, p3, p] = args;
    let c = call_c(p1, p2, p3, p);
    let r = call_rust(p1, p2, p3, p);
    if c.bits() != r.bits() {
        panic!(
            "[{row}] C/Rust divergence\n{}\n  C    -> {}\n  Rust -> {}",
            fmt_args(args),
            fmt(c),
            fmt(r)
        );
    }
    c
}

// ---------------------------------------------------------------------------
// deterministic PRNG (SplitMix64) — fixed seed per row for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub const fn new(seed: u64) -> Self {
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

    /// Uniform in `[0, n)`.
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }

    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    /// Uniform in `[0, 1)`, 24-bit mantissa.
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Uniform in `[lo, hi)`.
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.unit() * (hi - lo)
    }

    /// Any of the 2^32 bit patterns (NaN, inf, subnormal, zero included).
    pub fn any_bits_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }

    /// A finite "nice" normal float, magnitude in `[2^-12, 2^12)`, random sign.
    pub fn normal_f32(&mut self) -> f32 {
        let exp: i32 = self.below(25) as i32 - 12;
        let mant = 1.0f32 + self.unit();
        let v = mant * (exp as f32).exp2();
        if self.bool() {
            -v
        } else {
            v
        }
    }

    /// A finite float scaled into a chosen power-of-two decade.
    pub fn scaled_f32(&mut self, exp2_lo: i32, exp2_hi: i32) -> f32 {
        let span = (exp2_hi - exp2_lo + 1) as u32;
        let exp = exp2_lo + self.below(span) as i32;
        let mant = 1.0f32 + self.unit();
        let v = mant * f32::from_bits(((exp + 127) as u32) << 23);
        if self.bool() {
            -v
        } else {
            v
        }
    }

    /// A subnormal `f32` (`exp == 0`, `mantissa != 0`), random sign.
    pub fn subnormal_f32(&mut self) -> f32 {
        let mant = 1 + self.below(0x007f_ffff);
        let sign = if self.bool() { 0x8000_0000u32 } else { 0 };
        f32::from_bits(sign | mant)
    }

    /// A quiet NaN with a random non-zero payload, random sign.
    pub fn qnan_f32(&mut self) -> f32 {
        let payload = 1 + self.below(0x003f_ffff); // 22-bit payload, non-zero
        let sign = if self.bool() { 0x8000_0000u32 } else { 0 };
        f32::from_bits(sign | 0x7f80_0000 | 0x0040_0000 | payload)
    }

    /// A signaling NaN (`exp == 0xff`, quiet bit CLEAR, payload non-zero).
    pub fn snan_f32(&mut self) -> f32 {
        let payload = 1 + self.below(0x003f_ffff);
        let sign = if self.bool() { 0x8000_0000u32 } else { 0 };
        f32::from_bits(sign | 0x7f80_0000 | payload)
    }

    pub fn inf_f32(&mut self) -> f32 {
        if self.bool() {
            f32::NEG_INFINITY
        } else {
            f32::INFINITY
        }
    }

    pub fn zero_f32(&mut self) -> f32 {
        if self.bool() {
            -0.0
        } else {
            0.0
        }
    }

    pub fn normal_vec(&mut self) -> Vec2 {
        Vec2::new(self.normal_f32(), self.normal_f32())
    }

    pub fn any_vec(&mut self) -> Vec2 {
        Vec2::new(self.any_bits_f32(), self.any_bits_f32())
    }
}

/// Read/write one of the 8 scalar coordinate slots of the argument list.
pub fn slot_mut(args: &mut [Vec2; 4], slot: usize) -> &mut f32 {
    let v = &mut args[slot / 2];
    if slot % 2 == 0 {
        &mut v.x
    } else {
        &mut v.y
    }
}

pub const SLOT_NAMES: [&str; 8] = ["p1.x", "p1.y", "p2.x", "p2.y", "p3.x", "p3.y", "p.x", "p.y"];
