//! Shared harness for the differential tests.
//!
//! Both implementations are loaded as shared objects through `libloading` and
//! invoked purely through their exported `contrast_ratio` symbol. The Rust
//! implementation is *never* called directly as a Rust function, so the
//! `#[no_mangle] extern "C"` wrapper and the by-value struct ABI are part of
//! what gets tested.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

/// Mirror of the C `cb_rgb_255`.
///
/// ```c
/// typedef struct cb_rgb_255 { unsigned char R, G, B; } cb_rgb_255;
/// ```
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
    pub const BLACK: Rgb = Rgb::new(0, 0, 0);
    pub const WHITE: Rgb = Rgb::new(255, 255, 255);
}

/// Same layout plus an explicit 4th byte, used by the ABI padding tests: on
/// SysV AMD64 both a 3-byte and a 4-byte all-`char` struct are class INTEGER and
/// travel in the low bytes of one general-purpose register, so calling through
/// this signature injects a chosen "garbage" byte into the register bits the
/// 3-byte struct does not define.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Rgb4 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub pad: u8,
}

pub type ContrastFn = unsafe extern "C" fn(Rgb, Rgb) -> f32;
pub type ContrastFn4 = unsafe extern "C" fn(Rgb4, Rgb4) -> f32;

/// One loaded implementation.
pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: Library,
    f: ContrastFn,
    f4: ContrastFn4,
}

impl Impl {
    /// `contrast_ratio(A, B)` through the `.so` export.
    #[inline]
    pub fn call(&self, a: Rgb, b: Rgb) -> f32 {
        unsafe { (self.f)(a, b) }
    }

    /// Same symbol, invoked through the 4-byte-struct signature so the register
    /// padding byte is attacker-controlled.
    #[inline]
    pub fn call_padded(&self, a: Rgb4, b: Rgb4) -> f32 {
        unsafe { (self.f4)(a, b) }
    }

    fn load(name: &'static str, path: PathBuf) -> Impl {
        assert!(
            path.is_file(),
            "shared object for `{name}` not found at {}\n\
             (build it first: C -> cmake, Rust -> cargo build)",
            path.display()
        );
        unsafe {
            let lib = Library::new(&path)
                .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
            let f: Symbol<ContrastFn> = lib
                .get(b"contrast_ratio\0")
                .unwrap_or_else(|e| panic!("`contrast_ratio` missing from {}: {e}", path.display()));
            let f = *f;
            let f4: Symbol<ContrastFn4> = lib.get(b"contrast_ratio\0").unwrap();
            let f4 = *f4;
            Impl { name, path, _lib: lib, f, f4 }
        }
    }
}

/// The crate root (`translation/`).
fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Locate the C `.so`. Its file name is derived by CMake from the *parent
/// directory name* of `c_src`, so it is discovered rather than hard-coded.
fn find_c_so() -> PathBuf {
    let build_dir = manifest_dir().join("../c_src/build");
    let entries = std::fs::read_dir(&build_dir).unwrap_or_else(|e| {
        panic!(
            "cannot read {} ({e}).\nBuild the C library first:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build_dir.display()
        )
    });
    let mut found: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one lib*.so in {}, got {:?}",
        build_dir.display(),
        found
    );
    found.pop().unwrap()
}

/// Locate the Rust `cdylib` matching the profile this test binary was built in.
fn find_rust_so() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test>-<hash>  ->  .../target/<profile>/
    let profile_dir: &Path = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("target/<profile> dir");
    let direct = profile_dir.join("libcontrast_ratio_lib.so");
    if direct.is_file() {
        return direct;
    }
    // Fall back to any sibling profile that has been built.
    let target = profile_dir.parent().expect("target dir");
    for p in ["release", "debug"] {
        let cand = target.join(p).join("libcontrast_ratio_lib.so");
        if cand.is_file() {
            return cand;
        }
    }
    panic!(
        "libcontrast_ratio_lib.so not found under {} — run `cargo build` (and/or \
         `cargo build --release`) before `cargo test`",
        target.display()
    );
}

/// Load both implementations.
pub fn load_pair() -> (Impl, Impl) {
    let c = Impl::load("C", find_c_so());
    let r = Impl::load("Rust", find_rust_so());
    (c, r)
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seeds keep every test reproducible.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub const fn new(seed: u64) -> Self {
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
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    /// Uniform in `lo..=hi`.
    #[inline]
    pub fn range_u8(&mut self, lo: u8, hi: u8) -> u8 {
        debug_assert!(lo <= hi);
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as u8
    }
    #[inline]
    pub fn rgb(&mut self) -> Rgb {
        Rgb::new(self.next_u8(), self.next_u8(), self.next_u8())
    }
    #[inline]
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[(self.next_u64() % xs.len() as u64) as usize]
    }
}

// ---------------------------------------------------------------------------
// Bit-exact comparison
// ---------------------------------------------------------------------------

/// The sRGB linear-branch threshold: `n/255 > 0.04045` is false for `n <= 10`
/// and true for `n >= 11`.
pub const LIN_MAX: u8 = 10;
pub const POW_MIN: u8 = 11;

/// Compare one input pair bit-for-bit. Returns `Err(message)` on divergence.
#[inline]
pub fn diff_one(c: &Impl, r: &Impl, a: Rgb, b: Rgb) -> Result<f32, String> {
    let cv = c.call(a, b);
    let rv = r.call(a, b);
    if cv.to_bits() == rv.to_bits() {
        Ok(cv)
    } else {
        Err(format!(
            "DIVERGENCE contrast_ratio(({},{},{}), ({},{},{})):\n  \
             C    = {cv:?}  bits=0x{:08X}\n  Rust = {rv:?}  bits=0x{:08X}",
            a.r, a.g, a.b, b.r, b.g, b.b,
            cv.to_bits(),
            rv.to_bits()
        ))
    }
}

/// Accumulates divergences so a failing test reports a useful sample rather
/// than dying on the first mismatch.
pub struct Checker<'a> {
    c: &'a Impl,
    r: &'a Impl,
    pub checked: u64,
    pub failures: Vec<String>,
    /// Set when `LumA < LumB` (the swap branch) was provably taken, and when it
    /// provably was not, so tests can assert both sides of X7 were reached.
    pub saw_ratio_gt_one: bool,
    pub saw_ratio_eq_one: bool,
    pub saw_non_finite: bool,
}

impl<'a> Checker<'a> {
    pub fn new(c: &'a Impl, r: &'a Impl) -> Self {
        Checker {
            c,
            r,
            checked: 0,
            failures: Vec::new(),
            saw_ratio_gt_one: false,
            saw_ratio_eq_one: false,
            saw_non_finite: false,
        }
    }

    #[inline]
    pub fn check(&mut self, a: Rgb, b: Rgb) {
        self.checked += 1;
        match diff_one(self.c, self.r, a, b) {
            Ok(v) => {
                if !v.is_finite() {
                    self.saw_non_finite = true;
                } else if v > 1.0 {
                    self.saw_ratio_gt_one = true;
                } else if v == 1.0 {
                    self.saw_ratio_eq_one = true;
                }
            }
            Err(m) => {
                if self.failures.len() < 20 {
                    self.failures.push(m);
                }
            }
        }
    }

    /// Panic with a summary if anything diverged.
    pub fn finish(self, label: &str) {
        assert!(self.checked > 0, "{label}: no inputs were checked");
        if !self.failures.is_empty() {
            panic!(
                "{label}: {} of {} inputs diverged; first {} shown:\n{}",
                self.failures.len(),
                self.checked,
                self.failures.len(),
                self.failures.join("\n")
            );
        }
    }
}
