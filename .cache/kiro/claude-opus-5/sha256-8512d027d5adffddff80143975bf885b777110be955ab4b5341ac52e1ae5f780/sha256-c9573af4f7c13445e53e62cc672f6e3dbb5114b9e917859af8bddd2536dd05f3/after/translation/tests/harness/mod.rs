//! Differential test harness: loads BOTH the C `.so` and the Rust `.so` through
//! `libloading` and compares their exported `half2float` symbol.
//!
//! The Rust implementation is NEVER called directly — every call goes through
//! `dlopen`/`dlsym` on the built `cdylib`, exactly as an external C consumer
//! would, so the `#[no_mangle] extern "C"` wrapper is under test too.

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

/// `float half2float(uint16_t)` as an ABI-correct function pointer.
pub type Half2Float = unsafe extern "C" fn(u16) -> f32;

/// Same symbol, but with the argument declared as a full 32-bit word, to probe
/// what each side does when a caller fails to zero-extend the `uint16_t`
/// (ERRORS.md row 11).
pub type Half2FloatWide = unsafe extern "C" fn(u32) -> f32;

/// Same symbol again with a 64-bit argument, to probe garbage in the upper half
/// of the full argument register (`rdi` vs `edi`).
pub type Half2FloatWide64 = unsafe extern "C" fn(u64) -> f32;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn first_existing(candidates: &[PathBuf], what: &str) -> PathBuf {
    for c in candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "could not locate the {what} shared object; looked in:\n{}\n\
         Build both libraries first:\n  \
         (cd c_src && mkdir -p build && cd build && cmake .. \
         -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .)\n  \
         (cd translation && cargo build --release)",
        candidates
            .iter()
            .map(|p| format!("  {}", p.display()))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// Locate the C `.so`. The CMake project name is derived from the *parent*
/// directory name, so glob rather than hardcode it.
fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HALF2FLOAT_C_SO") {
        return PathBuf::from(p);
    }
    let build_dir = workspace_root().join("c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&build_dir) {
        for e in entries.flatten() {
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
    first_existing(&found, "C")
}

/// Locate the Rust `cdylib`. Overridable so the suite can be run against both
/// the debug and the release artifact.
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HALF2FLOAT_RUST_SO") {
        return PathBuf::from(p);
    }
    let t = workspace_root().join("translation/target");
    first_existing(
        &[
            t.join("release/libhalf2float_lib.so"),
            t.join("debug/libhalf2float_lib.so"),
        ],
        "Rust",
    )
}

/// Both libraries, leaked so the extracted function pointers stay valid for the
/// whole process (including inside spawned threads).
pub struct Pair {
    pub c: Half2Float,
    pub rust: Half2Float,
    pub c_wide: Half2FloatWide,
    pub rust_wide: Half2FloatWide,
    pub c_wide64: Half2FloatWide64,
    pub rust_wide64: Half2FloatWide64,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

pub fn load() -> Pair {
    let c_path = c_so_path();
    let rust_path = rust_so_path();

    // SAFETY: both paths point at shared objects we just built; loading them
    // runs their (empty) initialisers. Leaked on purpose: the function pointers
    // below must outlive any borrow of the `Library`.
    let c_lib: &'static Library = Box::leak(Box::new(unsafe {
        Library::new(&c_path).unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()))
    }));
    let rust_lib: &'static Library = Box::leak(Box::new(unsafe {
        Library::new(&rust_path).unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_path.display()))
    }));

    // SAFETY: `half2float` has C ABI `float(uint16_t)` in both objects, as
    // declared by c_src/include/lib.h.
    unsafe {
        let c: Symbol<Half2Float> = c_lib
            .get(b"half2float\0")
            .expect("C .so must export half2float");
        let rust: Symbol<Half2Float> = rust_lib
            .get(b"half2float\0")
            .expect("Rust .so must export half2float");
        let c_wide: Symbol<Half2FloatWide> = c_lib.get(b"half2float\0").unwrap();
        let rust_wide: Symbol<Half2FloatWide> = rust_lib.get(b"half2float\0").unwrap();
        let c_wide64: Symbol<Half2FloatWide64> = c_lib.get(b"half2float\0").unwrap();
        let rust_wide64: Symbol<Half2FloatWide64> = rust_lib.get(b"half2float\0").unwrap();
        Pair {
            c: *c,
            rust: *rust,
            c_wide: *c_wide,
            rust_wide: *rust_wide,
            c_wide64: *c_wide64,
            rust_wide64: *rust_wide64,
            c_path,
            rust_path,
        }
    }
}

impl Pair {
    /// Call both and compare the **raw bit patterns**. Comparing `f32` with `==`
    /// would let every NaN divergence through (and would conflate `+0.0`/`-0.0`),
    /// so `to_bits` is the only correct comparison here.
    #[track_caller]
    pub fn assert_same(&self, h: u16) {
        // SAFETY: scalar-in / scalar-out C ABI, no pointers involved.
        let (cv, rv) = unsafe { ((self.c)(h), (self.rust)(h)) };
        let (cb, rb) = (cv.to_bits(), rv.to_bits());
        assert_eq!(
            cb,
            rb,
            "half2float divergence for h=0x{h:04X} (n={}, mant_idx_lo=0x{:03X}): \
             C=0x{cb:08X} ({cv:?})  Rust=0x{rb:08X} ({rv:?})",
            h >> 10,
            h & 0x3ff,
        );
    }

    pub fn c_bits(&self, h: u16) -> u32 {
        unsafe { (self.c)(h) }.to_bits()
    }
    pub fn rust_bits(&self, h: u16) -> u32 {
        unsafe { (self.rust)(h) }.to_bits()
    }
}

/// xorshift64* — a fixed-seed PRNG so every "randomized" run is reproducible
/// without pulling in a dependency.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        // 0 is a fixed point of xorshift; forbid it.
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform-ish in `0..n` (n > 0).
    pub fn below(&mut self, n: u32) -> u32 {
        (self.next_u64() % u64::from(n)) as u32
    }
    pub fn u16(&mut self) -> u16 {
        (self.next_u64() >> 48) as u16
    }
}
