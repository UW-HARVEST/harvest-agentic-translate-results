//! Shared differential-test harness.
//!
//! BOTH implementations are loaded as shared objects via `libloading` and
//! called only through their exported `tritanopia` symbol.  The Rust functions
//! are *never* called directly, so the `#[no_mangle] extern "C"` wrapper and
//! the struct-by-value ABI are part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;

/// Mirrors `cb_rgb_255` from `c_src/include/lib.h`:
/// `struct { unsigned char R, G, B; }` -> size 3, align 1.
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Rgb { r, g, b }
    }
}

/// `cb_rgb_255 tritanopia(cb_rgb_255 RGB)`
pub type TritFn = unsafe extern "C" fn(Rgb) -> Rgb;

/// Same symbol, but viewed as taking/returning a raw 64-bit register so that
/// the *unspecified* upper bytes of `RDI`/`RAX` can be driven and inspected
/// (ERRORS.md rows E14/E15, CONFIGS.md rows 24/25).
pub type TritRawFn = unsafe extern "C" fn(u64) -> u64;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Locate the C reference `.so`. Override with `TRIT_C_SO` (used to point the
/// suite at the extra `-O2` build).
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("TRIT_C_SO") {
        return PathBuf::from(p);
    }
    let p = manifest_dir().join("c_src/build/libtranslated_rust.so");
    assert!(
        p.exists(),
        "C shared library not found at {}\nBuild it with:\n  cd c_src && mkdir -p build && cd build \
         && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// Locate the Rust `.so`. Override with `TRIT_RUST_SO`. By default prefer the
/// build profile the tests themselves were built with, then fall back.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("TRIT_RUST_SO") {
        return PathBuf::from(p);
    }
    let base = manifest_dir().join("target");
    // The test executable lives in target/<profile>/deps/, so derive <profile>.
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(deps) = exe.parent() {
            if let Some(profile_dir) = deps.parent() {
                candidates.push(profile_dir.join("libtritanopia_lib.so"));
            }
        }
    }
    candidates.push(base.join("release/libtritanopia_lib.so"));
    candidates.push(base.join("debug/libtritanopia_lib.so"));
    for c in &candidates {
        if c.exists() {
            assert_not_stale(c);
            return c.clone();
        }
    }
    panic!(
        "Rust shared library (libtritanopia_lib.so) not found; looked in:\n{}",
        candidates
            .iter()
            .map(|c| format!("  {}", c.display()))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// Guard against the single most dangerous failure mode of this suite: loading
/// a **stale** `.so` and therefore passing vacuously.
///
/// With `crate-type = ["cdylib"]` alone, `cargo test` does not rebuild the
/// cdylib (the integration tests have no dependency on it), so an edit to
/// `src/lib.rs` would be invisible to every test. `"rlib"` is now also in
/// `crate-type` to force the rebuild, and this check is the belt-and-braces
/// verification that it actually happened.
fn assert_not_stale(so: &std::path::Path) {
    let newest_src = newest_mtime(&manifest_dir().join("src"));
    let so_mtime = std::fs::metadata(so).and_then(|m| m.modified()).ok();
    if let (Some(src), Some(obj)) = (newest_src, so_mtime) {
        assert!(
            obj >= src,
            "STALE Rust .so: {} is older than the newest file in src/.\n\
             The differential tests would compare the C library against an \
             out-of-date translation and pass vacuously.\n\
             Rebuild first:  cargo build --release   (or run ./run_all.sh)",
            so.display()
        );
    }
}

fn newest_mtime(dir: &std::path::Path) -> Option<std::time::SystemTime> {
    let mut newest: Option<std::time::SystemTime> = None;
    let entries = std::fs::read_dir(dir).ok()?;
    for e in entries.flatten() {
        let path = e.path();
        let m = if path.is_dir() {
            newest_mtime(&path)
        } else {
            std::fs::metadata(&path).and_then(|m| m.modified()).ok()
        };
        if let Some(m) = m {
            if newest.is_none_or(|n| m > n) {
                newest = Some(m);
            }
        }
    }
    newest
}

/// Both libraries loaded side by side, ready for differential calls.
pub struct Pair {
    _c_lib: Library,
    _rust_lib: Library,
    pub c: TritFn,
    pub rust: TritFn,
    pub c_raw: TritRawFn,
    pub rust_raw: TritRawFn,
}

impl Pair {
    pub fn load() -> Pair {
        unsafe {
            let c_lib = Library::new(c_so_path()).expect("failed to dlopen the C .so");
            let rust_lib = Library::new(rust_so_path()).expect("failed to dlopen the Rust .so");

            // Resolve the exported symbol in each library. If the Rust `.so`
            // were missing the `#[no_mangle]` export this would fail here.
            let c_sym: Symbol<TritFn> = c_lib
                .get(b"tritanopia\0")
                .expect("C .so does not export `tritanopia`");
            let rust_sym: Symbol<TritFn> = rust_lib
                .get(b"tritanopia\0")
                .expect("Rust .so does not export `tritanopia`");
            let c = *c_sym;
            let rust = *rust_sym;

            let c_raw_sym: Symbol<TritRawFn> = c_lib.get(b"tritanopia\0").unwrap();
            let rust_raw_sym: Symbol<TritRawFn> = rust_lib.get(b"tritanopia\0").unwrap();
            let c_raw = *c_raw_sym;
            let rust_raw = *rust_raw_sym;

            Pair {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rust,
                c_raw,
                rust_raw,
            }
        }
    }

    #[inline]
    pub fn call_c(&self, i: Rgb) -> Rgb {
        unsafe { (self.c)(i) }
    }

    #[inline]
    pub fn call_rust(&self, i: Rgb) -> Rgb {
        unsafe { (self.rust)(i) }
    }

    /// Raw function-pointer addresses, for handing to worker threads.
    pub fn raw_addrs(&self) -> (usize, usize) {
        (self.c as usize, self.rust as usize)
    }
}

/// Assert C and Rust agree byte-for-byte on one input.
#[inline]
#[track_caller]
pub fn assert_same(p: &Pair, i: Rgb) {
    let c = p.call_c(i);
    let r = p.call_rust(i);
    assert_eq!(
        c, r,
        "DIVERGENCE for input ({},{},{}): C = ({},{},{}) but Rust = ({},{},{})",
        i.r, i.g, i.b, c.r, c.g, c.b, r.r, r.g, r.b
    );
}

/// Run a whole row of `CONFIGS.md`: compare every input the iterator yields.
/// Returns the number of inputs compared.
#[track_caller]
pub fn check_all<I: Iterator<Item = Rgb>>(row: &str, inputs: I) -> usize {
    let p = Pair::load();
    let mut n = 0usize;
    let mut mismatches: Vec<(Rgb, Rgb, Rgb)> = Vec::new();
    for i in inputs {
        let c = p.call_c(i);
        let r = p.call_rust(i);
        if c != r {
            if mismatches.len() < 20 {
                mismatches.push((i, c, r));
            }
        }
        n += 1;
    }
    assert!(n > 0, "{row}: no inputs were generated (test is vacuous!)");
    if !mismatches.is_empty() {
        let detail = mismatches
            .iter()
            .map(|(i, c, r)| {
                format!(
                    "  in=({:3},{:3},{:3})  C=({:3},{:3},{:3})  RUST=({:3},{:3},{:3})",
                    i.r, i.g, i.b, c.r, c.g, c.b, r.r, r.g, r.b
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        panic!(
            "{row}: {} of {} inputs DIVERGED (first {} shown):\n{}",
            mismatches.len(),
            n,
            mismatches.len().min(20),
            detail
        );
    }
    n
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) -- fixed seed => reproducible runs.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

pub const SEED: u64 = 0x5EED_1234;

impl Rng {
    pub fn new(seed: u64) -> Rng {
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
    /// Uniform in `lo..=hi` (inclusive), for u8 ranges.
    #[inline]
    pub fn range_u8(&mut self, lo: u8, hi: u8) -> u8 {
        debug_assert!(lo <= hi);
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as u8
    }
}

/// `n` random inputs whose channels are drawn from the given inclusive ranges.
/// This is the property-style driver used by most `CONFIGS.md` rows.
pub fn random_in_ranges(
    seed: u64,
    n: usize,
    r: (u8, u8),
    g: (u8, u8),
    b: (u8, u8),
) -> impl Iterator<Item = Rgb> {
    let mut rng = Rng::new(seed);
    (0..n).map(move |_| {
        Rgb::new(
            rng.range_u8(r.0, r.1),
            rng.range_u8(g.0, g.1),
            rng.range_u8(b.0, b.1),
        )
    })
}

/// Exhaustively enumerate the cuboid `r0..=r1 x g0..=g1 x b0..=b1`.
pub fn cuboid(r: (u8, u8), g: (u8, u8), b: (u8, u8)) -> impl Iterator<Item = Rgb> {
    let (r0, r1) = r;
    let (g0, g1) = g;
    let (b0, b1) = b;
    (r0..=r1).flat_map(move |rr| {
        (g0..=g1).flat_map(move |gg| (b0..=b1).map(move |bb| Rgb::new(rr, gg, bb)))
    })
}

/// Number of samples used for each randomized `CONFIGS.md` row.
pub const SAMPLES: usize = 20_000;
