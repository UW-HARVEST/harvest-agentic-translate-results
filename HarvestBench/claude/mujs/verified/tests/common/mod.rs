//! Shared helpers for differential tests: load BOTH the C .so and the Rust .so
//! via libloading and expose symbol lookups. We NEVER call Rust functions
//! directly — always through the loaded .so, exactly as an external C caller.

use libloading::{Library, Symbol};
use std::path::PathBuf;

pub struct Libs {
    pub c: Library,
    pub rust: Library,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

impl Libs {
    pub fn load() -> Libs {
        let base = manifest_dir();
        let c_path = base.join("c_src/build/libmujs.so");
        let rust_path = base.join("target/debug/libmujs.so");
        assert!(c_path.exists(), "C .so not built: {:?}", c_path);
        assert!(rust_path.exists(), "Rust .so not built: {:?}", rust_path);
        unsafe {
            // The C .so does NOT link libm, so math symbols (ceil, pow, ...)
            // must be made globally available before we dlopen it. Load libm
            // with RTLD_GLOBAL and leak it so the symbols stay resolvable.
            use libloading::os::unix::{Library as UnixLib, RTLD_GLOBAL, RTLD_NOW};
            if let Ok(libm) = UnixLib::open(Some("libm.so.6"), RTLD_NOW | RTLD_GLOBAL) {
                std::mem::forget(libm);
            }
            let c = Library::new(&c_path).expect("load C .so");
            let rust = Library::new(&rust_path).expect("load Rust .so");
            Libs { c, rust }
        }
    }

    pub unsafe fn c_sym<T>(&self, name: &[u8]) -> Symbol<T> {
        self.c.get(name).unwrap_or_else(|e| panic!("C sym {:?}: {}", String::from_utf8_lossy(name), e))
    }
    pub unsafe fn rust_sym<T>(&self, name: &[u8]) -> Symbol<T> {
        self.rust.get(name).unwrap_or_else(|e| panic!("Rust sym {:?}: {}", String::from_utf8_lossy(name), e))
    }
}

/// Simple deterministic xorshift PRNG for reproducible property tests.
pub struct Rng(pub u64);
impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E3779B97F4A7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 { 0 } else { self.next_u32() % n }
    }
    pub fn f64(&mut self) -> f64 {
        // uniform-ish in [0,1)
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}
