//! Shared harness: loads BOTH the C `.so` and the Rust `.so` through
//! `libloading` and calls `hdr_bitrate` only through the exported symbol, so
//! the `#[no_mangle]` / `extern "C"` wrapper is under test too. No Rust
//! function is ever called directly.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;

/// `unsigned hdr_bitrate(const uint8_t *h)`
pub type HdrBitrateFn = unsafe extern "C" fn(*const u8) -> std::ffi::c_uint;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Locate the C shared library built from `c_src/`.
fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_PATH") {
        return PathBuf::from(p);
    }
    let build = manifest_dir().parent().unwrap().join("c_src/build");
    let mut found = None;
    if let Ok(entries) = std::fs::read_dir(&build) {
        for e in entries.flatten() {
            let p = e.path();
            let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            if name.starts_with("lib") && name.ends_with(".so") {
                found = Some(p);
                break;
            }
        }
    }
    found.unwrap_or_else(|| {
        panic!(
            "no C .so found in {}. Build it first:\n  cd c_src && mkdir -p build && cd build \
             && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

/// Locate the Rust cdylib. Prefers whichever of debug/release is newest so the
/// test reflects the build that was just produced.
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        return PathBuf::from(p);
    }
    let target = manifest_dir().join("target");
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for profile in ["release", "debug"] {
        let p = target.join(profile).join("libhdr_bitrate_lib.so");
        if let Ok(md) = std::fs::metadata(&p) {
            let t = md.modified().unwrap_or(std::time::UNIX_EPOCH);
            if best.as_ref().map(|(bt, _)| t > *bt).unwrap_or(true) {
                best = Some((t, p));
            }
        }
    }
    best.map(|(_, p)| p).unwrap_or_else(|| {
        panic!(
            "libhdr_bitrate_lib.so not found under {}. Run `cargo build --release` first.",
            target.display()
        )
    })
}

/// Both libraries, kept alive for the lifetime of the test.
pub struct Pair {
    _c_lib: Library,
    _rust_lib: Library,
    pub c: HdrBitrateFn,
    pub rust: HdrBitrateFn,
}

impl Pair {
    pub fn load() -> Self {
        unsafe {
            let c_lib = Library::new(c_so_path())
                .unwrap_or_else(|e| panic!("failed to dlopen C .so: {e}"));
            let rust_lib = Library::new(rust_so_path())
                .unwrap_or_else(|e| panic!("failed to dlopen Rust .so: {e}"));

            let c_sym: Symbol<HdrBitrateFn> = c_lib
                .get(b"hdr_bitrate\0")
                .expect("C .so does not export `hdr_bitrate`");
            let rust_sym: Symbol<HdrBitrateFn> = rust_lib
                .get(b"hdr_bitrate\0")
                .expect("Rust .so does not export `hdr_bitrate` (missing #[no_mangle]?)");

            let c = *c_sym;
            let rust = *rust_sym;
            Pair { _c_lib: c_lib, _rust_lib: rust_lib, c, rust }
        }
    }

    /// Call both exports on the same buffer and require byte-identical results.
    #[track_caller]
    pub fn assert_same(&self, buf: &[u8], ctx: &str) -> u32 {
        assert!(buf.len() >= 3, "test buffers must have >= 3 bytes");
        let cv = unsafe { (self.c)(buf.as_ptr()) } as u32;
        let rv = unsafe { (self.rust)(buf.as_ptr()) } as u32;
        assert_eq!(
            cv, rv,
            "DIVERGENCE [{ctx}]: h = [{:#04x}, {:#04x}, {:#04x}] -> C returned {cv}, Rust returned {rv}",
            buf[0], buf[1], buf[2]
        );
        cv
    }
}

/// Deterministic PRNG (xorshift64*) so every run uses the same inputs.
pub struct Rng(u64);

impl Rng {
    pub const fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
}
