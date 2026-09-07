//! Shared harness: loads BOTH the C `.so` and the Rust `.so` through
//! `libloading` and exposes their `crc16` exports.
//!
//! Nothing here calls the Rust implementation directly — every Rust-side call
//! goes through the dynamic symbol, exactly like an external C consumer, so the
//! `#[no_mangle] extern "C"` wrapper is under test too.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub type Crc16Fn = unsafe extern "C" fn(*const u8, u32, u16) -> u16;

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

fn find_so(dir: &Path, prefer: &[&str]) -> PathBuf {
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                found.push(p);
            }
        }
    }
    for want in prefer {
        if let Some(p) = found.iter().find(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .map(|s| s.contains(want))
                .unwrap_or(false)
        }) {
            return p.clone();
        }
    }
    found.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no .so found in {} — build it first (see README/task instructions)",
            dir.display()
        )
    })
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let dir = repo_root().join("c_src/build");
    find_so(&dir, &[])
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let root = repo_root().join("translation/target");
    for prof in ["release", "debug"] {
        let d = root.join(prof);
        let p = d.join("libcrc16_lib.so");
        if p.exists() {
            return p;
        }
    }
    find_so(&root.join("release"), &["crc16_lib"])
}

/// Both libraries, kept alive for the lifetime of the test.
pub struct Pair {
    _c_lib: libloading::Library,
    _r_lib: libloading::Library,
    pub c: Crc16Fn,
    pub r: Crc16Fn,
}

impl Pair {
    pub fn load() -> Pair {
        unsafe {
            let c_lib = libloading::Library::new(c_so_path())
                .unwrap_or_else(|e| panic!("loading C .so {:?}: {e}", c_so_path()));
            let r_lib = libloading::Library::new(rust_so_path())
                .unwrap_or_else(|e| panic!("loading Rust .so {:?}: {e}", rust_so_path()));
            let c: libloading::Symbol<Crc16Fn> =
                c_lib.get(b"crc16\0").expect("C .so exports crc16");
            let r: libloading::Symbol<Crc16Fn> =
                r_lib.get(b"crc16\0").expect("Rust .so exports crc16");
            let c = *c;
            let r = *r;
            Pair {
                _c_lib: c_lib,
                _r_lib: r_lib,
                c,
                r,
            }
        }
    }

    /// Call both exports on the same input; assert byte-identical results and
    /// return the common value.
    #[track_caller]
    pub fn both(&self, data: &[u8], len: u32, seed: u16) -> u16 {
        let p = if data.is_empty() {
            std::ptr::null()
        } else {
            data.as_ptr()
        };
        self.both_ptr(p, len, seed)
    }

    #[track_caller]
    pub fn both_ptr(&self, p: *const u8, len: u32, seed: u16) -> u16 {
        let cv = unsafe { (self.c)(p, len, seed) };
        let rv = unsafe { (self.r)(p, len, seed) };
        assert_eq!(
            cv, rv,
            "divergence: len={len} seed={seed:#06x} ptr={p:?} C={cv:#06x} Rust={rv:#06x}"
        );
        cv
    }
}

/// Deterministic xorshift64* PRNG — fixed seed, reproducible across runs.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
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
    pub fn next_u16(&mut self) -> u16 {
        (self.next_u64() >> 48) as u16
    }
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.next_u8()).collect()
    }
}

/// Seeds worth sweeping: extremes plus interesting bit patterns.
pub const EXTREME_SEEDS: &[u16] = &[
    0x0000, 0xFFFF, 0xFF00, 0x00FF, 0x8000, 0x0001, 0x8005, 0x7FFF, 0x0100, 0x00AA, 0xAA00, 0x5555,
];
