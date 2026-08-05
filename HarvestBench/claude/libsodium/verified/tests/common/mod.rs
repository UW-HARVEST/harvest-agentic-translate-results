//! Shared differential-testing harness.
//!
//! Loads BOTH the C `.so` and the Rust `.so` via `libloading` and exposes
//! helpers to fetch matching symbols from each. Tests then call the same
//! symbol on both libraries and compare results byte-for-byte.
//!
//! We NEVER call Rust functions directly — every call goes through the
//! exported `#[no_mangle]` symbol loaded from the cdylib, exactly as an
//! external C consumer would.

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::sync::OnceLock;

pub struct Libs {
    pub c: Library,
    pub rust: Library,
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    crate_root().join("c_src/build/libsodium.so")
}

fn rust_so_path() -> PathBuf {
    // The cdylib produced by this crate.
    let mut p = crate_root().join("target");
    // Prefer release, fall back to debug.
    let rel = p.join("release/liblibsodium.so");
    if rel.exists() {
        return rel;
    }
    p.push("debug/liblibsodium.so");
    p
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c = unsafe { Library::new(c_so_path()).expect("load C .so") };
        let rust = unsafe { Library::new(rust_so_path()).expect("load Rust .so") };
        // Both libraries need sodium_init() called once.
        unsafe {
            let ci: Symbol<unsafe extern "C" fn() -> i32> =
                c.get(b"sodium_init").expect("C sodium_init");
            let ri: Symbol<unsafe extern "C" fn() -> i32> =
                rust.get(b"sodium_init").expect("Rust sodium_init");
            ci();
            ri();
        }
        Libs { c, rust }
    })
}

/// Fetch a symbol of type `T` from both libraries.
#[macro_export]
macro_rules! sympair {
    ($libs:expr, $name:expr, $t:ty) => {{
        let cs: libloading::Symbol<$t> =
            unsafe { $libs.c.get($name).expect(concat!("C symbol ", stringify!($name))) };
        let rs: libloading::Symbol<$t> =
            unsafe { $libs.rust.get($name).expect(concat!("Rust symbol ", stringify!($name))) };
        (cs, rs)
    }};
}

/// A small deterministic PRNG (xorshift64*) for reproducible fuzzing.
pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
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
    pub fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = (self.next_u64() & 0xff) as u8;
        }
    }
    pub fn range(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn vec(&mut self, len: usize) -> Vec<u8> {
        let mut v = vec![0u8; len];
        self.fill(&mut v);
        v
    }
}
