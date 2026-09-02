//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both libraries are loaded with `libloading` and called **only** through their
//! exported `float2half` symbol. The Rust implementation is never called
//! directly from the test binary, so the `#[no_mangle] extern "C"` wrapper and
//! the cdylib export are part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

pub type Float2Half = unsafe extern "C" fn(f32) -> u16;

/// Workspace root: the directory containing both `c_src/` and `translation/`.
fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Locate the C shared object produced by `c_src/build`.
///
/// The CMake project name is derived from the *parent directory name*, so the
/// file name is not fixed; discover it instead of hard-coding it.
fn c_so_path() -> PathBuf {
    let build = workspace_root().join("c_src").join("build");
    assert!(
        build.is_dir(),
        "c_src/build not found — build the C library first:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );

    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .expect("read c_src/build")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib") && n.ends_with(".so"))
                    .unwrap_or(false)
        })
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one lib*.so in c_src/build, found {found:?}"
    );
    found.pop().unwrap()
}

/// Locate the Rust cdylib for the *same* profile the test binary was built in.
///
/// `current_exe()` is `<target>/<profile>/deps/<test>-<hash>`, so the cdylib
/// sits two levels up. This keeps `cargo test` and `cargo test --release`
/// honest instead of silently testing a stale artifact.
fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("<target>/<profile>")
        .to_path_buf();

    let candidate = profile_dir.join("libfloat2half_lib.so");
    assert!(
        candidate.is_file(),
        "Rust cdylib not found at {candidate:?} — run `cargo build` for this profile"
    );
    candidate
}

/// Both libraries, loaded, with their `float2half` symbols resolved.
pub struct Pair {
    // Kept alive so the resolved function pointers stay valid.
    _c_lib: Library,
    _rust_lib: Library,
    pub c: Float2Half,
    pub rust: Float2Half,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

impl Pair {
    pub fn load() -> Pair {
        let c_path = c_so_path();
        let rust_path = rust_so_path();

        // SAFETY: both paths point at shared objects we just built; loading them
        // runs their (trivial) initialisers.
        let c_lib = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("dlopen {c_path:?}: {e}"));
        let rust_lib = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|e| panic!("dlopen {rust_path:?}: {e}"));

        // SAFETY: `float2half` has signature `uint16_t(float)` in lib.h and the
        // matching `extern "C" fn(f32) -> u16` in the Rust cdylib.
        let c = unsafe {
            let s: Symbol<Float2Half> = c_lib
                .get(b"float2half\0")
                .expect("C .so must export float2half");
            *s
        };
        let rust = unsafe {
            let s: Symbol<Float2Half> = rust_lib
                .get(b"float2half\0")
                .expect("Rust .so must export float2half");
            *s
        };

        Pair {
            _c_lib: c_lib,
            _rust_lib: rust_lib,
            c,
            rust,
            c_path,
            rust_path,
        }
    }

    /// Call both implementations on the raw bit pattern `bits` and require the
    /// two `u16` results to be identical.
    #[inline]
    pub fn check_bits(&self, bits: u32, ctx: &str) {
        let x = f32::from_bits(bits);
        // SAFETY: plain scalar-in / scalar-out FFI calls.
        let (got_c, got_rust) = unsafe { ((self.c)(x), (self.rust)(x)) };
        assert_eq!(
            got_c, got_rust,
            "MISMATCH [{ctx}] input bits {bits:#010x} (as f32 = {x:e}): \
             C returned {got_c:#06x}, Rust returned {got_rust:#06x}"
        );
    }

    /// Same, but starting from an actual `f32` value rather than a bit pattern.
    #[inline]
    pub fn check_value(&self, x: f32, ctx: &str) {
        // SAFETY: as above.
        let (got_c, got_rust) = unsafe { ((self.c)(x), (self.rust)(x)) };
        assert_eq!(
            got_c, got_rust,
            "MISMATCH [{ctx}] input {x:e} (bits {:#010x}): \
             C returned {got_c:#06x}, Rust returned {got_rust:#06x}",
            x.to_bits()
        );
    }

    /// Call C only (used by invariant probes that assert a concrete C result).
    #[inline]
    pub fn c_of_bits(&self, bits: u32) -> u16 {
        unsafe { (self.c)(f32::from_bits(bits)) }
    }

    #[inline]
    pub fn rust_of_bits(&self, bits: u32) -> u16 {
        unsafe { (self.rust)(f32::from_bits(bits)) }
    }
}

/// Deterministic PRNG (SplitMix64) so every "randomized" row is reproducible.
pub struct Rng(u64);

pub const SEED: u64 = 0x2545_F491_4F6C_DD1D;

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform in `0..n` (n > 0).
    #[inline]
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
}

/// Assemble a float bit pattern from a 9-bit table index `j` and a 23-bit
/// mantissa. `j` is exactly `(bits >> 23) & 0x1ff`, i.e. sign+exponent.
#[inline]
pub fn bits_from(j: u32, mantissa: u32) -> u32 {
    assert!(j < 512);
    (j << 23) | (mantissa & 0x007f_ffff)
}

/// The six mantissa shapes every configuration row is exercised with.
/// `shift` is the `m__shift[j]` value for the row, used to build the
/// "only bits the shift discards" shape.
pub fn mantissa_shapes(shift: u32) -> [u32; 6] {
    let discarded = if shift >= 23 {
        0x007f_ffff
    } else {
        (1u32 << shift) - 1
    };
    [
        0x0000_0000, // empty
        0x0000_0001, // lowest bit only
        0x007f_ffff, // all 23 bits
        0x0040_0000, // top mantissa bit only
        discarded,   // only bits the shift throws away
        0x0055_5555, // alternating
    ]
}
