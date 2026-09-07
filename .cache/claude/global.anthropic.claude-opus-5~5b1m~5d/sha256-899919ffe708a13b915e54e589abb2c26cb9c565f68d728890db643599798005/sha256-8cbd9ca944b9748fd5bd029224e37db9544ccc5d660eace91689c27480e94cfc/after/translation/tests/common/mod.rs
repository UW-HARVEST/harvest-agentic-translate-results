//! Shared differential-test harness.
//!
//! Loads BOTH shared objects with `libloading` and calls `hdr_compare` through
//! the FFI boundary in each. The Rust implementation is NEVER called directly —
//! it is always reached via the exported `hdr_compare` symbol of
//! `libhdr_compare_lib.so`, exactly as an external C consumer would.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::{Path, PathBuf};

type HdrCompareFn = unsafe extern "C" fn(*const u8, *const u8) -> c_int;

pub struct Libs {
    _c_lib: Library,
    _rs_lib: Library,
    c_fn: HdrCompareFn,
    rs_fn: HdrCompareFn,
    pub c_path: PathBuf,
    pub rs_path: PathBuf,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workspace_root() -> PathBuf {
    manifest_dir()
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Find the single `.so` produced by the C CMake build.
fn find_c_so() -> PathBuf {
    let build_dir = workspace_root().join("c_src").join("build");
    let entries = std::fs::read_dir(&build_dir).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e}\n\
             Build the C library first:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build_dir.display()
        )
    });

    let mut found: Vec<PathBuf> = Vec::new();
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) == Some("so") {
            found.push(p);
        }
    }
    found.sort();
    assert!(
        !found.is_empty(),
        "no .so found in {} — build the C library first",
        build_dir.display()
    );
    found.remove(0)
}

/// Find the Rust `cdylib`. Prefer the profile the tests were built with, but
/// accept either `debug/` or `release/`.
fn find_rust_so() -> PathBuf {
    let name = "libhdr_compare_lib.so";

    // The test binary lives in `<target>/<profile>/deps/`, so the cdylib is at
    // `<target>/<profile>/<name>`. Derive that from argv[0] when possible so we
    // always match the profile under test.
    if let Ok(exe) = std::env::current_exe() {
        let mut dir: Option<&Path> = exe.parent();
        while let Some(d) = dir {
            let cand = d.join(name);
            if cand.is_file() {
                return cand;
            }
            dir = d.parent();
        }
    }

    for profile in ["release", "debug"] {
        let cand = manifest_dir().join("target").join(profile).join(name);
        if cand.is_file() {
            return cand;
        }
    }

    panic!(
        "{name} not found under {}/target — run `cargo build --release` first",
        manifest_dir().display()
    );
}

impl Libs {
    pub fn load() -> Libs {
        let c_path = find_c_so();
        let rs_path = find_rust_so();

        unsafe {
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
            let rs_lib = Library::new(&rs_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", rs_path.display()));

            let c_sym: Symbol<HdrCompareFn> = c_lib
                .get(b"hdr_compare\0")
                .expect("C .so does not export `hdr_compare`");
            let rs_sym: Symbol<HdrCompareFn> = rs_lib
                .get(b"hdr_compare\0")
                .expect("Rust .so does not export `hdr_compare` (missing #[no_mangle]?)");

            let c_fn = *c_sym;
            let rs_fn = *rs_sym;

            Libs {
                _c_lib: c_lib,
                _rs_lib: rs_lib,
                c_fn,
                rs_fn,
                c_path,
                rs_path,
            }
        }
    }

    /// Call the C implementation through its exported symbol.
    pub unsafe fn c_compare(&self, h1: *const u8, h2: *const u8) -> c_int {
        (self.c_fn)(h1, h2)
    }

    /// Call the Rust implementation through its exported symbol.
    pub unsafe fn rs_compare(&self, h1: *const u8, h2: *const u8) -> c_int {
        (self.rs_fn)(h1, h2)
    }

    /// Differential check on two raw pointers. Returns the (agreed) result.
    pub unsafe fn diff_raw(&self, h1: *const u8, h2: *const u8, ctx: &str) -> c_int {
        let c = self.c_compare(h1, h2);
        let r = self.rs_compare(h1, h2);
        assert_eq!(
            c, r,
            "DIVERGENCE [{ctx}]: C hdr_compare -> {c}, Rust hdr_compare -> {r}"
        );
        c
    }

    /// Differential check on two byte slices (each must be >= 3 bytes).
    pub fn diff(&self, h1: &[u8], h2: &[u8]) -> c_int {
        assert!(h1.len() >= 3 && h2.len() >= 3, "buffers must hold 3 bytes");
        let ctx = format!("h1={:02x?} h2={:02x?}", &h1[..3.min(h1.len())], &h2[..3]);
        unsafe { self.diff_raw(h1.as_ptr(), h2.as_ptr(), &ctx) }
    }

    /// Differential check where each buffer is heap-allocated to EXACTLY its
    /// length, so any read past the end is a real out-of-bounds access that
    /// ASAN / valgrind / the allocator can catch.
    pub fn diff_exact(&self, h1: &[u8], h2: &[u8]) -> c_int {
        let a: Box<[u8]> = h1.to_vec().into_boxed_slice();
        let b: Box<[u8]> = h2.to_vec().into_boxed_slice();
        let ctx = format!("exact h1={:02x?} h2={:02x?}", h1, h2);
        unsafe { self.diff_raw(a.as_ptr(), b.as_ptr(), &ctx) }
    }
}

/// Deterministic SplitMix64 PRNG — fixed seed keeps every run reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
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
        (self.next_u64() >> 33) as u8
    }

    /// Uniform in `[0, n)`.
    pub fn below(&mut self, n: u32) -> u32 {
        assert!(n > 0);
        (self.next_u64() % u64::from(n)) as u32
    }

    /// Uniform in `[lo, hi]`.
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        assert!(lo <= hi);
        lo + self.below(hi - lo + 1)
    }

    pub fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.u8();
        }
    }
}

pub const SEED: u64 = 0x5EED_1234_ABCD_0001;

// ---------------------------------------------------------------------------
// Header builders derived from the branch conditions in `c_src/src/lib.c`.
// ---------------------------------------------------------------------------

/// Build a `h[1]` byte that passes `hdr_valid`'s byte-1 checks.
///
/// * `form_f == true`  -> `(h1 & 0xF0) == 0xF0`
/// * `form_f == false` -> `(h1 & 0xFE) == 0xE2`  (i.e. `0xE2` or `0xE3`)
///
/// `layer` must be in `1..=3` (0 is rejected by `((h[1] >> 1) & 3) != 0`).
pub fn make_byte1(form_f: bool, layer: u8, prot: u8) -> u8 {
    assert!((1..=3).contains(&layer));
    if form_f {
        // bits 7..4 = 0xF, bits 3..1 hold version(1 bit) + layer(2 bits),
        // bit 0 = protection. Layer occupies bits 2..1.
        let version_bit = (prot >> 1) & 1; // reuse spare entropy for bit 3
        0xF0 | (version_bit << 3) | (layer << 1) | (prot & 1)
    } else {
        // `(h1 & 0xFE) == 0xE2` forces bits 7..1 = 0b1110001, i.e. layer field
        // `(h1 >> 1) & 3 == 1`. Only bit 0 is free.
        0xE2 | (prot & 1)
    }
}

/// Build a `h[2]` byte that passes `hdr_valid`'s byte-2 checks.
///
/// * `bitrate` must be in `0..=14` (15 is rejected by `(h[2] >> 4) != 15`)
/// * `srate` must be in `0..=2`   (3 is rejected by `((h[2] >> 2) & 3) != 3`)
/// * `low` supplies the two never-inspected low bits.
pub fn make_byte2(bitrate: u8, srate: u8, low: u8) -> u8 {
    assert!(bitrate <= 14);
    assert!(srate <= 2);
    (bitrate << 4) | (srate << 2) | (low & 3)
}

/// A canonical valid 4-byte `h2` plus a matching `h1`.
pub fn valid_pair(rng: &mut Rng) -> ([u8; 4], [u8; 4]) {
    let form_f = rng.below(2) == 0;
    let layer = rng.range(1, 3) as u8;
    let b1 = make_byte1(form_f, layer, rng.u8());
    let b2 = make_byte2(rng.range(1, 14) as u8, rng.range(0, 2) as u8, rng.u8());
    let h2 = [0xff, b1, b2, rng.u8()];
    let h1 = [rng.u8(), b1, b2, rng.u8()];
    (h1, h2)
}
