//! Shared differential-test harness.
//!
//! Both the C and the Rust implementation are loaded as **shared objects** via
//! `libloading` and called through their exported `hdr_compare` symbol. The
//! Rust code is never called directly, so the `#[no_mangle] extern "C"` export
//! wrapper is part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::{Path, PathBuf};

/// ABI of the single exported entry point (`lib.h`):
/// `int hdr_compare(const uint8_t *h1, const uint8_t *h2);`
pub type HdrCompareFn = unsafe extern "C" fn(*const u8, *const u8) -> c_int;

pub struct Impls {
    pub c: HdrCompareFn,
    pub rust: HdrCompareFn,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/libtranslated_rust.so`, built by CMake.
fn c_so_path() -> PathBuf {
    let base = manifest_dir().join("c_src").join("build");
    // The CMake project name is derived from the parent directory name, so the
    // library file name follows the crate directory. Accept any libX.so found.
    let preferred = base.join("libtranslated_rust.so");
    if preferred.is_file() {
        return preferred;
    }
    if let Ok(rd) = std::fs::read_dir(&base) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            if name.starts_with("lib") && name.ends_with(".so") {
                return p;
            }
        }
    }
    panic!(
        "C shared library not found under {}.\nBuild it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        base.display()
    );
}

/// `target/<profile>/libhdr_compare_lib.so`, built by Cargo (cdylib).
fn rust_so_path() -> PathBuf {
    const NAME: &str = "libhdr_compare_lib.so";
    // current_exe() == target/<profile>/deps/<testbin>-<hash>
    let exe = std::env::current_exe().expect("current_exe");
    let deps: &Path = exe.parent().expect("deps dir");
    let profile: &Path = deps.parent().expect("profile dir");
    for cand in [profile.join(NAME), deps.join(NAME)] {
        if cand.is_file() {
            return cand;
        }
    }
    panic!(
        "Rust cdylib {NAME} not found in {} or {}. Build it with `cargo build`.",
        profile.display(),
        deps.display()
    );
}

/// Loads both shared objects (leaked so the function pointers stay valid for
/// the lifetime of the test process) and resolves `hdr_compare` from each.
pub fn load() -> Impls {
    let c_path = c_so_path();
    let rust_path = rust_so_path();
    unsafe {
        let c_lib: &'static Library = Box::leak(Box::new(
            Library::new(&c_path).unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display())),
        ));
        let r_lib: &'static Library = Box::leak(Box::new(
            Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_path.display())),
        ));
        let c: Symbol<HdrCompareFn> = c_lib
            .get(b"hdr_compare\0")
            .expect("C .so does not export hdr_compare");
        let r: Symbol<HdrCompareFn> = r_lib
            .get(b"hdr_compare\0")
            .expect("Rust .so does not export hdr_compare");
        Impls { c: *c, rust: *r, c_path, rust_path }
    }
}

impl Impls {
    /// Calls both implementations on the same raw pointers.
    #[inline]
    pub fn both(&self, h1: *const u8, h2: *const u8) -> (c_int, c_int) {
        unsafe { ((self.c)(h1, h2), (self.rust)(h1, h2)) }
    }

    /// Differential assertion over two byte slices (each must be >= 3 bytes,
    /// or >= 0 when the corresponding pointer is provably not dereferenced).
    #[inline]
    pub fn assert_eq_slices(&self, h1: &[u8], h2: &[u8], ctx: &str) -> c_int {
        let (c, r) = self.both(h1.as_ptr(), h2.as_ptr());
        assert_eq!(
            c, r,
            "DIVERGENCE {ctx}: h1={:02x?} h2={:02x?} -> C={c} Rust={r}",
            h1, h2
        );
        c
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed for reproducibility.
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x243F_6A88_85A3_08D3;

pub struct Rng(u64);

impl Rng {
    #[inline]
    pub fn new(seed: u64) -> Self {
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
    pub fn u8(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    /// Uniform in `0..n` (n > 0).
    #[inline]
    pub fn below(&mut self, n: u32) -> u32 {
        (self.next_u64() % n as u64) as u32
    }
    pub fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.u8();
        }
    }
}

// ---------------------------------------------------------------------------
// MP3-style header construction, mirroring the fields the C inspects.
// ---------------------------------------------------------------------------

/// Sync/version encodings that pass `hdr_valid`'s second gate.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Sync {
    /// `(h[1] & 0xF0) == 0xF0` with version bit 3 set  -> MPEG1
    Mpeg1,
    /// `(h[1] & 0xF0) == 0xF0` with version bit 3 clear -> MPEG2
    Mpeg2,
    /// `(h[1] & 0xFE) == 0xE2` -> MPEG2.5 (layer bits are fixed to `01`)
    Mpeg25,
}

/// Builds a 4-byte header. `layer` is the raw 2-bit code (`0` is the reserved
/// value the C rejects), `bitrate` the raw 4-bit index (`15` reserved, `0` is
/// free format), `srate` the raw 2-bit index (`3` reserved).
pub fn make_hdr(
    sync: Sync,
    layer: u8,
    bitrate: u8,
    srate: u8,
    padding: bool,
    private: bool,
    crc: bool,
    b3: u8,
) -> [u8; 4] {
    let b1 = match sync {
        Sync::Mpeg1 => 0xF8 | ((layer & 3) << 1) | crc as u8,
        Sync::Mpeg2 => 0xF0 | ((layer & 3) << 1) | crc as u8,
        // 0xE2 already encodes layer bits `01`; only bit 0 is free.
        Sync::Mpeg25 => 0xE2 | crc as u8,
    };
    let b2 = ((bitrate & 0x0F) << 4) | ((srate & 3) << 2) | ((padding as u8) << 1) | private as u8;
    [0xFF, b1, b2, b3]
}

/// A `h2` that satisfies every `hdr_valid` gate.
pub fn valid_h2() -> [u8; 4] {
    // FF FB 90 00 : MPEG1, Layer III, bitrate index 9, sample-rate index 0
    make_hdr(Sync::Mpeg1, 1, 9, 0, false, false, true, 0x00)
}

/// True iff `h[1]` passes `hdr_valid`'s sync gate.
#[inline]
pub fn sync_ok(b1: u8) -> bool {
    (b1 & 0xF0) == 0xF0 || (b1 & 0xFE) == 0xE2
}

/// Reference model of the C `hdr_valid` (used only to *select* inputs, never
/// as an oracle for the differential comparison).
#[inline]
pub fn model_valid(h: &[u8]) -> bool {
    h[0] == 0xFF
        && sync_ok(h[1])
        && ((h[1] >> 1) & 3) != 0
        && (h[2] >> 4) != 15
        && ((h[2] >> 2) & 3) != 3
}

/// Every `h[1]` value that passes the sync gate.
pub fn sync_passing_b1() -> Vec<u8> {
    (0u16..=255).map(|v| v as u8).filter(|&b| sync_ok(b)).collect()
}
