//! Shared harness: loads the C reference `.so` and the Rust `.so` via
//! `libloading` and exposes their exported symbols.
//!
//! Nothing here calls into the Rust crate directly; every Rust invocation goes
//! through `dlopen`/`dlsym` on the built cdylib, exactly as an external C
//! caller would, so the `#[no_mangle]` wrappers are under test too.

// This module is shared by several test binaries; not every one uses every item.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::{Path, PathBuf};

/// Repository root (parent of the `translation` crate directory).
pub fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("crate dir has a parent")
        .to_path_buf()
}

/// Locates the C reference shared library produced by the CMake build.
fn c_library_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_LIB_PATH") {
        return PathBuf::from(p);
    }
    let build_dir = repo_root().join("c_src").join("build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&build_dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                found.push(p);
            }
        }
    }
    found.sort();
    found.pop().unwrap_or_else(|| {
        panic!(
            "no C .so found in {}; build it with: \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build_dir.display()
        )
    })
}

/// Locates the Rust cdylib. Prefers `RUST_LIB_PATH`, then whichever of the
/// release/debug artifacts exists (newest wins).
fn rust_library_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_LIB_PATH") {
        return PathBuf::from(p);
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut candidates: Vec<PathBuf> = Vec::new();
    for profile in ["release", "debug"] {
        let p = manifest
            .join("target")
            .join(profile)
            .join("libmemchra2_lib.so");
        if p.exists() {
            candidates.push(p);
        }
    }
    // Also handle a shared CARGO_TARGET_DIR layout.
    if let Ok(td) = std::env::var("CARGO_TARGET_DIR") {
        for profile in ["release", "debug"] {
            let p = Path::new(&td).join(profile).join("libmemchra2_lib.so");
            if p.exists() {
                candidates.push(p);
            }
        }
    }
    candidates.sort_by_key(|p| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .ok()
    });
    candidates
        .pop()
        .expect("libmemchra2_lib.so not found; run `cargo build` first")
}

/// The single exported entry point: `int memchra2(int, int, int, int)`.
pub type Memchra2Fn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

pub struct Harness {
    _c_lib: Library,
    _rust_lib: Library,
    c_memchra2: Memchra2Fn,
    rust_memchra2: Memchra2Fn,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

impl Harness {
    pub fn load() -> Self {
        let c_path = c_library_path();
        let rust_path = rust_library_path();

        // SAFETY: both paths point at shared libraries built from this repo.
        let c_lib = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", c_path.display()));
        let rust_lib = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", rust_path.display()));

        let c_memchra2: Memchra2Fn = unsafe {
            let s: Symbol<Memchra2Fn> = c_lib
                .get(b"memchra2\0")
                .expect("C .so does not export `memchra2`");
            *s
        };
        let rust_memchra2: Memchra2Fn = unsafe {
            let s: Symbol<Memchra2Fn> = rust_lib
                .get(b"memchra2\0")
                .expect("Rust .so does not export `memchra2`");
            *s
        };

        Harness {
            _c_lib: c_lib,
            _rust_lib: rust_lib,
            c_memchra2,
            rust_memchra2,
            c_path,
            rust_path,
        }
    }

    pub fn c_memchra2(&self, a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
        unsafe { (self.c_memchra2)(a, b, c, d) }
    }

    pub fn rust_memchra2(&self, a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
        unsafe { (self.rust_memchra2)(a, b, c, d) }
    }

    /// Asserts byte-for-byte equality of the two implementations' results.
    #[track_caller]
    pub fn assert_match(&self, a: c_int, b: c_int, c: c_int, d: c_int) {
        let expected = self.c_memchra2(a, b, c, d);
        let actual = self.rust_memchra2(a, b, c, d);
        assert_eq!(
            expected.to_ne_bytes(),
            actual.to_ne_bytes(),
            "memchra2({a}, {b}, {c}, {d}): C returned {expected} (0x{expected:08x}), \
             Rust returned {actual} (0x{actual:08x})"
        );
    }
}

/// Deterministic splitmix64-based generator so failures are reproducible.
pub struct Rng(u64);

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

    pub fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }

    /// Draws from a mix of distributions that hit interesting branches:
    /// full-range bit patterns, small magnitudes, and `int` bit patterns that
    /// reinterpret as floats inside the `(0.0, 1000.0)` window used by
    /// `int_to_float_bits`.
    pub fn next_interesting_i32(&mut self) -> i32 {
        match self.next_u64() % 8 {
            0 => self.next_i32(),
            1 => (self.next_u64() % 21) as i32 - 10,
            2 => (self.next_u64() % 512) as i32 - 256,
            3 => {
                // Float in (0, 1000): exponent chosen so the value lands in range.
                let f = (self.next_u64() as f64 / u64::MAX as f64) as f32 * 999.9;
                f.to_bits() as i32
            }
            4 => {
                // Subnormal / tiny positive floats (> 0.0 but truncate to 0).
                ((self.next_u64() % 0x0080_0000) as u32) as i32
            }
            5 => {
                // Near the 1000.0f boundary.
                let base = 1000.0f32.to_bits() as i64;
                (base + (self.next_u64() % 9) as i64 - 4) as i32
            }
            6 => {
                // Byte-aligned patterns for interpret_as_int / complex_iteration.
                (self.next_u64() as u32 & 0xFF00_00FF) as i32
            }
            _ => {
                // Powers of two and neighbours.
                let sh = (self.next_u64() % 32) as u32;
                let base = 1i32.wrapping_shl(sh);
                base.wrapping_add((self.next_u64() % 3) as i32 - 1)
            }
        }
    }
}

/// Values chosen to cover every branch and boundary in the C source:
/// sign changes (affecting the `-` count in the formatted buffer), `int`
/// bit patterns that reinterpret as in-range / out-of-range / NaN floats,
/// and low-byte patterns feeding `interpret_as_int`.
pub const EDGE_VALUES: &[i32] = &[
    0,
    1,
    -1,
    2,
    -2,
    9,
    10,
    -10,
    99,
    100,
    255,
    256,
    -255,
    -256,
    257,
    32767,
    -32768,
    65535,
    65536,
    1_000_000,
    -1_000_000,
    i32::MAX,
    i32::MIN,
    i32::MAX - 1,
    i32::MIN + 1,
    // Float bit patterns
    0x0000_0001,               // smallest positive subnormal
    0x007F_FFFF,               // largest subnormal
    0x0080_0000,               // smallest normal
    0x3F80_0000,               // 1.0f
    0x3F00_0000,               // 0.5f
    0x4048_F5C3,               // 3.14f
    0x437F_0000,               // 255.0f
    0x4479_FFFF,               // just below 1000.0f
    0x447A_0000,               // exactly 1000.0f (excluded by `< 1000.0f`)
    0x447A_0001,               // just above 1000.0f
    0x7F80_0000,               // +inf
    -0x0080_0000,              // negative float
    0x7FC0_0000,               // quiet NaN
    -1_073_741_824,            // 0xC0000000, negative float
];
