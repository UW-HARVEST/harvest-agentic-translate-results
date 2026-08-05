// Common helpers for differential testing: load BOTH the C .so and Rust .so
// via libloading and compare their outputs across the FFI boundary.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;

pub struct Libs {
    pub c: Library,
    pub rust: Library,
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    crate_root().join("c_src/build/liblz4.so")
}

pub fn rust_so_path() -> PathBuf {
    // The dylib produced by `cargo test`/`cargo build` lands in target/<profile>/deps
    // but the stable path is target/{debug,release}/liblz4.so
    let root = crate_root();
    let candidates = [
        root.join("target/debug/liblz4.so"),
        root.join("target/release/liblz4.so"),
    ];
    for c in candidates.iter() {
        if c.exists() {
            return c.clone();
        }
    }
    // default
    root.join("target/debug/liblz4.so")
}

impl Libs {
    pub fn load() -> Libs {
        let cp = c_so_path();
        let rp = rust_so_path();
        assert!(cp.exists(), "C .so not found at {:?} — build it first", cp);
        assert!(rp.exists(), "Rust .so not found at {:?} — build it first", rp);
        unsafe {
            Libs {
                c: Library::new(&cp).expect("load C .so"),
                rust: Library::new(&rp).expect("load Rust .so"),
            }
        }
    }
}

/// Fetch a symbol of type F from the C library.
pub unsafe fn csym<'a, F>(libs: &'a Libs, name: &[u8]) -> Symbol<'a, F> {
    libs.c
        .get(name)
        .unwrap_or_else(|_| panic!("C symbol {} missing", String::from_utf8_lossy(name)))
}

/// Fetch a symbol of type F from the Rust library.
pub unsafe fn rsym<'a, F>(libs: &'a Libs, name: &[u8]) -> Symbol<'a, F> {
    libs.rust
        .get(name)
        .unwrap_or_else(|_| panic!("Rust symbol {} missing", String::from_utf8_lossy(name)))
}

/// A tiny deterministic xorshift PRNG so tests are reproducible without extra deps.
pub struct Rng {
    s: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng {
            s: seed ^ 0x9E3779B97F4A7C15,
        }
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.s;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.s = x;
        x
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn range(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % (n as u64)) as usize
        }
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    /// Compressible-ish data: bytes from a small alphabet with runs.
    pub fn compressible(&mut self, len: usize) -> Vec<u8> {
        // Reserve extra capacity so `as_ptr()` is a valid, non-dangling pointer
        // even for len==0 (LZ4 dereferences near the start pointer regardless).
        let mut v = Vec::with_capacity(len.max(16));
        while v.len() < len {
            let sym = (self.next_u64() % 8) as u8 + b'A';
            let run = 1 + self.range(20);
            for _ in 0..run {
                if v.len() >= len {
                    break;
                }
                v.push(sym);
            }
        }
        v
    }
    /// Fully random (incompressible) data.
    pub fn random(&mut self, len: usize) -> Vec<u8> {
        let mut v = Vec::with_capacity(len.max(16));
        for _ in 0..len {
            v.push(self.byte());
        }
        v
    }
}
