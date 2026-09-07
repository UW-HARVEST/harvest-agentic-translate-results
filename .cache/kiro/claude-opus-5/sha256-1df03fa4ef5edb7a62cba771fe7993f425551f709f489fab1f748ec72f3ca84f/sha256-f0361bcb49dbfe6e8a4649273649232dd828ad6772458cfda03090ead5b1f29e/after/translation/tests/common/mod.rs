//! Shared differential-test harness.
//!
//! Both implementations are loaded **as shared objects via `libloading`** and
//! called only through their exported `tritanopia` symbol, exactly as an
//! external C consumer would. The Rust crate is never called directly, so the
//! `#[no_mangle] extern "C"` wrapper is part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

/// `typedef struct cb_rgb_255 { unsigned char R, G, B; }` — 3 bytes, align 1.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Rgb { r, g, b }
    }
}

type TritFn = unsafe extern "C" fn(Rgb) -> Rgb;
/// The same entry point viewed as `u32 -> u32`. `cb_rgb_255` is 3 bytes, so
/// under x86-64 SysV it occupies a single INTEGER-class eightbyte, which is
/// how a 32-bit scalar travels too. This view lets the tests drive the
/// otherwise-inaccessible padding byte of the argument register.
type TritRawFn = unsafe extern "C" fn(u32) -> u32;

pub struct Harness {
    _c_lib: Library,
    _rust_lib: Library,
    c: TritFn,
    rust: TritFn,
    c_raw: TritRawFn,
    rust_raw: TritRawFn,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

fn find_c_so() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let build = root.join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&build) {
        for e in entries.flatten() {
            let p = e.path();
            let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            if name.starts_with("lib") && name.ends_with(".so") {
                candidates.push(p);
            }
        }
    }
    candidates.sort();
    candidates.pop().unwrap_or_else(|| {
        panic!(
            "no C shared object found in {}. Build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    // `std::env::current_exe()` is target/<profile>/deps/<test-bin>; the cdylib
    // is two directories up. Fall back to the well-known profile dirs.
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(deps) = exe.parent() {
            if let Some(profile) = deps.parent() {
                candidates.push(profile.join("libtritanopia_lib.so"));
            }
        }
    }
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    for p in ["debug", "release"] {
        candidates.push(target.join(p).join("libtritanopia_lib.so"));
    }
    for c in &candidates {
        if c.is_file() {
            assert_fresh(c);
            return c.clone();
        }
    }
    panic!(
        "no Rust cdylib found; looked at {:?}. Build it with `cargo build`.",
        candidates
    )
}

/// `cargo test` does not rebuild a `cdylib`-only lib target, so a stale `.so`
/// would silently make these tests verify old code. Refuse to run in that case.
fn assert_fresh(so: &Path) {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("lib.rs");
    let mtime = |p: &Path| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    };
    assert!(
        mtime(so) >= mtime(&src),
        "STALE Rust shared object: {} is older than {}.\n\
         `cargo test` does not rebuild a cdylib-only lib target — run\n\
         `cargo build --release` (or ./run_verification.sh) first.",
        so.display(),
        src.display()
    );
}

impl Harness {
    pub fn new() -> Self {
        let c_path = find_c_so();
        let rust_path = find_rust_so();
        unsafe {
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", c_path.display()));
            let rust_lib = Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", rust_path.display()));

            let c: Symbol<TritFn> = c_lib
                .get(b"tritanopia\0")
                .expect("C .so does not export `tritanopia`");
            let rust: Symbol<TritFn> = rust_lib
                .get(b"tritanopia\0")
                .expect("Rust .so does not export `tritanopia`");
            let c_raw: Symbol<TritRawFn> = c_lib.get(b"tritanopia\0").unwrap();
            let rust_raw: Symbol<TritRawFn> = rust_lib.get(b"tritanopia\0").unwrap();

            let (c, rust, c_raw, rust_raw) = (*c, *rust, *c_raw, *rust_raw);
            Harness {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rust,
                c_raw,
                rust_raw,
                c_path,
                rust_path,
            }
        }
    }

    #[inline]
    pub fn c(&self, x: Rgb) -> Rgb {
        unsafe { (self.c)(x) }
    }

    #[inline]
    pub fn rust(&self, x: Rgb) -> Rgb {
        unsafe { (self.rust)(x) }
    }

    #[inline]
    pub fn c_raw(&self, x: u32) -> u32 {
        unsafe { (self.c_raw)(x) }
    }

    #[inline]
    pub fn rust_raw(&self, x: u32) -> u32 {
        unsafe { (self.rust_raw)(x) }
    }

    /// Assert byte-identical results for one input.
    #[inline]
    pub fn check(&self, x: Rgb) {
        let c = self.c(x);
        let r = self.rust(x);
        assert_eq!(
            c, r,
            "DIVERGENCE for input {{R:{}, G:{}, B:{}}}: C -> {{{}, {}, {}}}, Rust -> {{{}, {}, {}}}",
            x.r, x.g, x.b, c.r, c.g, c.b, r.r, r.g, r.b
        );
    }

    /// Run a whole `CONFIGS.md` row: check every input and report the count.
    pub fn check_row(&self, row: &str, inputs: impl IntoIterator<Item = Rgb>) -> usize {
        let mut n = 0usize;
        for x in inputs {
            self.check(x);
            n += 1;
        }
        assert!(n > 0, "{row}: generated no inputs — the row would vacuously pass");
        eprintln!("{row}: {n} inputs matched byte-for-byte");
        n
    }
}

/// SplitMix64 — deterministic, fixed seed, so every run is reproducible.
pub struct Rng(u64);

pub const SEED: u64 = 0x5EED_1234_ABCD_F00D;

impl Rng {
    pub const fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn seeded() -> Self {
        Rng(SEED)
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
    pub fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }
    /// Uniform in `0..n` (n > 0).
    #[inline]
    pub fn below(&mut self, n: u32) -> u32 {
        (self.next_u64() >> 32) as u32 % n
    }
    /// Uniform in `lo..=hi`.
    #[inline]
    pub fn range(&mut self, lo: u8, hi: u8) -> u8 {
        (lo as u32 + self.below((hi - lo) as u32 + 1)) as u8
    }
    #[inline]
    pub fn byte(&mut self) -> u8 {
        self.next_u64() as u8
    }
    #[inline]
    pub fn rgb(&mut self) -> Rgb {
        let v = self.next_u64();
        Rgb::new(v as u8, (v >> 8) as u8, (v >> 16) as u8)
    }
}

// ---------------------------------------------------------------------------
// A pure-Rust mirror of the C pipeline's *branch selection* only.
//
// This is used exclusively to CLASSIFY an input (which ternary branch each
// stage takes, and whether the float->uchar cast wraps) so the CONFIGS.md rows
// can be generated mechanically. It never participates in an assertion — all
// assertions compare the two `.so`s.
// ---------------------------------------------------------------------------

pub const DEGAMMA_THRESHOLD_BYTE: u8 = 11; // first byte with byte/255 > 0.04045
pub const REGAMMA_THRESHOLD: f64 = 0.003_130_804_953_560_371_517_027_863_777_09;

fn norm(v: u8) -> f32 {
    (v as f32) / 255.0f32
}

fn remove_gamma(c: f32) -> f32 {
    let c = c as f64;
    if c > 0.04045 {
        ((c + 0.055) / 1.055).powf(2.4) as f32
    } else {
        (c / 12.92) as f32
    }
}

/// The post-matrix linear triple, for classification purposes.
pub fn post_matrix(x: Rgb) -> (f32, f32, f32) {
    let r = remove_gamma(norm(x.r));
    let g = remove_gamma(norm(x.g));
    let b = remove_gamma(norm(x.b));
    (
        r + 0.127_398_863_108_80f32 * g - 0.127_398_863_410_72f32 * b,
        -4.486E-11f32 * r + 0.873_909_299_283_61f32 * g + 0.126_090_701_015_23f32 * b,
        3.1113E-10f32 * r + 0.873_909_297_258_48f32 * g + 0.126_090_700_671_15f32 * b,
    )
}

fn apply_gamma(c: f32) -> f32 {
    let c = c as f64;
    if c > REGAMMA_THRESHOLD {
        (1.055 * c.powf(0.4166666666) - 0.055) as f32
    } else {
        (c * 12.92) as f32
    }
}

/// `y = x * 255 + 0.5` for each channel — the value handed to the C cast.
pub fn denorm_pre_cast(x: Rgb) -> (f32, f32, f32) {
    let (r, g, b) = post_matrix(x);
    (
        apply_gamma(r) * 255.0f32 + 0.5f32,
        apply_gamma(g) * 255.0f32 + 0.5f32,
        apply_gamma(b) * 255.0f32 + 0.5f32,
    )
}

/// Axis 1 branch word, e.g. `"LPP"`.
pub fn degamma_class(x: Rgb) -> String {
    [x.r, x.g, x.b]
        .iter()
        .map(|&v| if v >= DEGAMMA_THRESHOLD_BYTE { 'P' } else { 'L' })
        .collect()
}

/// Axis 2 branch word, e.g. `"lpp"`.
pub fn regamma_class(x: Rgb) -> String {
    let (r, g, b) = post_matrix(x);
    [r, g, b]
        .iter()
        .map(|&v| if (v as f64) > REGAMMA_THRESHOLD { 'p' } else { 'l' })
        .collect()
}

/// Axis 3 class per channel: `'i'` in range, `'n'` negative-wrap, `'o'` overflow-wrap.
pub fn cast_class(x: Rgb) -> String {
    let (r, g, b) = denorm_pre_cast(x);
    [r, g, b]
        .iter()
        .map(|&y| {
            if y < 0.0 {
                'n'
            } else if y >= 256.0 {
                'o'
            } else {
                'i'
            }
        })
        .collect()
}

/// Draw `count` inputs matching `pred`, giving up after a bounded number of
/// attempts so a genuinely unreachable configuration is reported, not hung on.
pub fn sample_where(
    rng: &mut Rng,
    count: usize,
    max_attempts: usize,
    pred: impl Fn(Rgb) -> bool,
) -> Vec<Rgb> {
    let mut out = Vec::with_capacity(count);
    let mut attempts = 0usize;
    while out.len() < count && attempts < max_attempts {
        attempts += 1;
        let x = rng.rgb();
        if pred(x) {
            out.push(x);
        }
    }
    out
}

/// Exhaustive scan for inputs matching `pred` (the whole 2^24 cube), capped.
pub fn scan_all_where(limit: usize, pred: impl Fn(Rgb) -> bool) -> Vec<Rgb> {
    let mut out = Vec::new();
    for r in 0..=255u8 {
        for g in 0..=255u8 {
            for b in 0..=255u8 {
                let x = Rgb::new(r, g, b);
                if pred(x) {
                    out.push(x);
                    if out.len() >= limit {
                        return out;
                    }
                }
            }
        }
    }
    out
}

pub fn all_inputs() -> impl Iterator<Item = Rgb> {
    (0u32..(1u32 << 24)).map(|i| Rgb::new(i as u8, (i >> 8) as u8, (i >> 16) as u8))
}
