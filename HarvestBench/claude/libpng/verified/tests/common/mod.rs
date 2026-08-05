//! Shared helpers for differential tests. Loads BOTH the C `.so` and the Rust
//! `.so` via libloading and exposes them for symbol-level comparison.

use libloading::Library;
use std::path::PathBuf;

pub fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    crate_root().join("c_src/build/libpng.so")
}

pub fn rust_so_path() -> PathBuf {
    // The cdylib is built into target/<profile>/liblibpng.so. Tests run in
    // debug by default; try debug then release.
    let root = crate_root();
    let debug = root.join("target/debug/liblibpng.so");
    if debug.exists() {
        return debug;
    }
    root.join("target/release/liblibpng.so")
}

pub struct Libs {
    pub c: Library,
    pub rust: Library,
}

impl Libs {
    pub fn load() -> Libs {
        unsafe {
            let c = Library::new(c_so_path())
                .unwrap_or_else(|e| panic!("failed to load C .so {:?}: {e}", c_so_path()));
            let rust = Library::new(rust_so_path())
                .unwrap_or_else(|e| panic!("failed to load Rust .so {:?}: {e}", rust_so_path()));
            Libs { c, rust }
        }
    }
}

/// A tiny deterministic PRNG (xorshift64*) so tests are reproducible without a
/// dependency.
pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E3779B97F4A7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_u16(&mut self) -> u16 {
        (self.next_u64() >> 48) as u16
    }
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        // inclusive [lo, hi]
        lo + (self.next_u32() % (hi - lo + 1))
    }
}
