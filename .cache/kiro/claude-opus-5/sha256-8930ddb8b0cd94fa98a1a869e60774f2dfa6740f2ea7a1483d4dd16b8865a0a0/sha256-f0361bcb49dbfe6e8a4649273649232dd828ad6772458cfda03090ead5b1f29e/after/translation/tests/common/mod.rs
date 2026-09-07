//! Differential test harness: loads BOTH the C `.so` and the Rust `.so` via
//! `libloading` and compares `crc16` results through the FFI boundary.
//!
//! The Rust implementation is NEVER called directly — always through the
//! `cdylib`'s exported `crc16` symbol, so the `#[no_mangle] extern "C"`
//! wrapper is under test too.

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

/// `tflac_u16 crc16(const tflac_u8 *d, tflac_u32 len, tflac_u16 crc16)`
pub type Crc16Fn = unsafe extern "C" fn(*const u8, u32, u16) -> u16;

pub struct Lib {
    // Field order matters: `func` borrows from `lib`, so we keep the raw
    // pointer instead and hold the library alive for the process lifetime.
    _lib: &'static Library,
    pub crc16: Crc16Fn,
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                candidates.push(p);
            }
        }
    }
    candidates.sort();
    candidates.pop().unwrap_or_else(|| {
        panic!(
            "no .so found in {}; build the C library first:\n  cd c_src && mkdir -p build && cd build \
             && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    let target = workspace_root().join("translation").join("target");
    // Prefer the profile the tests were built under, then fall back.
    let mut dirs = vec![target.join("release"), target.join("debug")];
    if cfg!(debug_assertions) {
        dirs.reverse();
    }
    for d in dirs {
        let p = d.join("libcrc16_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("libcrc16_lib.so not found under {}; run `cargo build --release`", target.display())
}

fn load(path: &Path) -> Lib {
    unsafe {
        let lib: &'static Library = Box::leak(Box::new(
            Library::new(path).unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display())),
        ));
        let sym: Symbol<Crc16Fn> = lib
            .get(b"crc16\0")
            .unwrap_or_else(|e| panic!("dlsym crc16 in {}: {e}", path.display()));
        let f = *sym;
        Lib { _lib: lib, crc16: f }
    }
}

/// Both libraries, loaded once per test process.
pub struct Pair {
    pub c: Lib,
    pub rs: Lib,
}

pub fn pair() -> &'static Pair {
    use std::sync::OnceLock;
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| Pair { c: load(&find_c_so()), rs: load(&find_rust_so()) })
}

/// Deterministic PRNG (SplitMix64) — fixed seed, reproducible.
pub struct Rng(u64);

// Not every test binary uses every helper.
#[allow(dead_code)]
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
    pub fn u8(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    pub fn u16(&mut self) -> u16 {
        (self.next_u64() >> 24) as u16
    }
    /// Uniform-ish in `0..n` (n > 0).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    pub fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.u8();
        }
    }
}

/// Call both `.so`s with identical arguments and assert byte-identical results.
///
/// `d` may be null only when `len == 0`.
#[track_caller]
pub fn assert_same(ctx: &str, d: *const u8, len: u32, crc: u16) -> u16 {
    let p = pair();
    let (a, b) = unsafe { ((p.c.crc16)(d, len, crc), (p.rs.crc16)(d, len, crc)) };
    assert_eq!(
        a.to_le_bytes(),
        b.to_le_bytes(),
        "{ctx}: C=0x{a:04x} RUST=0x{b:04x} (len={len}, seed_crc=0x{crc:04x})"
    );
    a
}

/// Convenience wrapper over a slice.
#[track_caller]
pub fn assert_same_slice(ctx: &str, data: &[u8], crc: u16) -> u16 {
    assert_same(ctx, data.as_ptr(), data.len() as u32, crc)
}
