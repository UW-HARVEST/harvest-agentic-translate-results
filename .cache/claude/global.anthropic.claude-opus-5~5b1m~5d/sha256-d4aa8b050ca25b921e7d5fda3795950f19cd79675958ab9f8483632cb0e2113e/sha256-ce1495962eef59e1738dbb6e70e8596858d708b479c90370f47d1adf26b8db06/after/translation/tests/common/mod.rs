//! Shared differential-test harness.
//!
//! Loads **both** shared objects through `libloading` and calls `crc16` only
//! through its exported C symbol. The Rust implementation is never called
//! directly as a Rust function, so the `#[unsafe(no_mangle)] extern "C"` wrapper
//! is under test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The exact ABI from `c_src/include/lib.h:282`:
/// `tflac_u16 crc16(const tflac_u8 *d, tflac_u32 len, tflac_u16 crc16);`
pub type Crc16Fn = unsafe extern "C" fn(*const u8, u32, u16) -> u16;

/// One loaded implementation (C or Rust), addressed only via its `.so`.
pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: Library,
    crc16: Crc16Fn,
}

impl Impl {
    /// Call the library's exported `crc16` over a real byte slice.
    pub fn crc16(&self, data: &[u8], seed: u16) -> u16 {
        assert!(data.len() <= u32::MAX as usize, "len must fit tflac_u32");
        unsafe { (self.crc16)(data.as_ptr(), data.len() as u32, seed) }
    }

    /// Call with a fully explicit pointer/length pair, for boundary cases where
    /// `len` must not be derived from a slice (null pointer, wild pointer,
    /// `len == 0`, ...).
    pub unsafe fn crc16_raw(&self, ptr: *const u8, len: u32, seed: u16) -> u16 {
        unsafe { (self.crc16)(ptr, len, seed) }
    }
}

/// Both implementations, loaded once per test binary.
pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/lib<parent-dir-name>.so` — the name is derived from the parent
/// directory by `CMakeLists.txt`, so discover it rather than hardcoding it.
pub fn c_so_path() -> PathBuf {
    let build_dir = manifest_dir().join("../c_src/build");

    if let Some(p) = find_so(&build_dir) {
        return p;
    }

    // Not built yet: build it. Existence-guarded so concurrent test binaries do
    // not fight over the directory.
    let c_src = manifest_dir().join("../c_src");
    let _ = std::process::Command::new("cmake")
        .args(["-S", ".", "-B", "build", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
        .current_dir(&c_src)
        .status();
    let _ = std::process::Command::new("cmake")
        .args(["--build", "build"])
        .current_dir(&c_src)
        .status();

    find_so(&build_dir).unwrap_or_else(|| {
        panic!(
            "could not find or build the C shared library in {}.\n\
             Build it with:\n  cd c_src && cmake -S . -B build \
             -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build build",
            build_dir.display()
        )
    })
}

fn find_so(dir: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut found: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    found.sort();
    found.into_iter().next()
}

/// `target/<profile>/libcrc16_lib.so`, located relative to the running test
/// executable so it works under both `--release` and the dev profile.
pub fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test-bin> -> .../target/<profile>
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("test exe should live in target/<profile>/deps")
        .to_path_buf();

    let candidate = profile_dir.join("libcrc16_lib.so");
    if candidate.exists() {
        return candidate;
    }
    // Fall back to the sibling profile dirs.
    for prof in ["release", "debug"] {
        let alt = manifest_dir().join("target").join(prof).join("libcrc16_lib.so");
        if alt.exists() {
            return alt;
        }
    }
    panic!(
        "could not find libcrc16_lib.so near {} — build it with `cargo build`",
        profile_dir.display()
    );
}

/// Guard against silently verifying a **stale** `.so`.
///
/// The crate is `crate-type = ["cdylib"]` only, so `cargo test` does *not*
/// (re)build a `.so` for the test profile — the harness necessarily loads the
/// artifact produced by a previous `cargo build [--release]`. If that artifact
/// predates the newest source file, the whole test run would be meaningless, so
/// fail loudly instead.
fn assert_so_fresh(so: &Path) {
    let so_mtime = match std::fs::metadata(so).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return,
    };

    let mut newest_src: Option<(std::time::SystemTime, PathBuf)> = None;
    let src_dir = manifest_dir().join("src");
    let mut stack = vec![src_dir];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.filter_map(|e| e.ok()) {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().map(|x| x == "rs").unwrap_or(false)
                && let Ok(t) = e.metadata().and_then(|m| m.modified())
                && newest_src.as_ref().map(|(bt, _)| t > *bt).unwrap_or(true)
            {
                newest_src = Some((t, p));
            }
        }
    }

    if let Some((t, p)) = newest_src {
        assert!(
            so_mtime >= t,
            "STALE ARTIFACT: {} is older than {}.\n\
             This crate is cdylib-only, so `cargo test` cannot rebuild it.\n\
             Run `cargo build --release` (or ./run_all.sh) before `cargo test`.",
            so.display(),
            p.display()
        );
    }
}

fn load(name: &'static str, path: PathBuf) -> Impl {
    let lib = unsafe { Library::new(&path) }
        .unwrap_or_else(|e| panic!("dlopen {} ({}) failed: {e}", name, path.display()));
    let crc16 = {
        let sym: Symbol<Crc16Fn> = unsafe { lib.get(b"crc16\0") }
            .unwrap_or_else(|e| panic!("{name} does not export `crc16`: {e}"));
        *sym
    };
    Impl { name, path, _lib: lib, crc16 }
}

/// Load (once) and return both implementations.
pub fn pair() -> &'static Pair {
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| {
        let rust_so = rust_so_path();
        assert_so_fresh(&rust_so);
        Pair { c: load("C", c_so_path()), rust: load("Rust", rust_so) }
    })
}

// ---------------------------------------------------------------------------
// Differential assertions
// ---------------------------------------------------------------------------

/// Core differential check: identical result from both `.so`s, byte-for-byte.
#[track_caller]
pub fn assert_same(data: &[u8], seed: u16, ctx: &str) -> u16 {
    let p = pair();
    let c = p.c.crc16(data, seed);
    let r = p.rust.crc16(data, seed);
    assert_eq!(
        c, r,
        "MISMATCH [{ctx}]: C=0x{c:04x} Rust=0x{r:04x} \
         (len={}, seed=0x{seed:04x}, first bytes={:02x?})",
        data.len(),
        &data[..data.len().min(16)]
    );
    // Byte-for-byte on the little-endian encoding as well, so a differing
    // high/low byte can never be masked by an integer comparison.
    assert_eq!(c.to_le_bytes(), r.to_le_bytes(), "byte repr mismatch [{ctx}]");
    c
}

/// Differential check on a raw pointer/length pair.
#[track_caller]
pub unsafe fn assert_same_raw(ptr: *const u8, len: u32, seed: u16, ctx: &str) -> u16 {
    let p = pair();
    let c = unsafe { p.c.crc16_raw(ptr, len, seed) };
    let r = unsafe { p.rust.crc16_raw(ptr, len, seed) };
    assert_eq!(
        c, r,
        "MISMATCH [{ctx}]: C=0x{c:04x} Rust=0x{r:04x} (ptr={ptr:?}, len={len}, seed=0x{seed:04x})"
    );
    c
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

/// The one fixed seed used by every randomized test in this suite.
pub const RNG_SEED: u64 = 0x5EED_C0DE_D00D_F00D;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    /// Fresh generator at the suite's canonical fixed seed, perturbed by `salt`
    /// so different tests get different (but still reproducible) streams.
    pub fn fixed(salt: u64) -> Self {
        Rng(RNG_SEED ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15))
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u16(&mut self) -> u16 {
        (self.next_u64() >> 32) as u16
    }
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 40) as u8
    }
    /// Uniform-ish in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.next_u8()).collect()
    }
    pub fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.next_u8();
        }
    }
}

/// Seed values that sit on every interesting `u16` boundary.
pub const SEED_EXTREMES: [u16; 12] = [
    0x0000, 0x0001, 0x00FF, 0x0100, 0x0101, 0x7FFF, 0x8000, 0x8001, 0xFF00, 0xFEFF, 0xFFFE, 0xFFFF,
];
