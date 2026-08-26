//! Shared differential-test harness.
//!
//! Both implementations are loaded as shared objects through `libloading` and
//! invoked purely through their exported `contrast_ratio` symbol. The Rust
//! implementation is **never** called directly as a Rust function, so the
//! `#[no_mangle] extern "C"` wrapper and the `#[repr(C)]` struct ABI are part of
//! what is under test.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// `typedef struct cb_rgb_255 { unsigned char R, G, B; } cb_rgb_255;`
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
    /// Low 24 bits of an argument register, as the SysV ABI lays this struct out.
    pub fn as_reg_bits(self) -> u64 {
        (self.r as u64) | ((self.g as u64) << 8) | ((self.b as u64) << 16)
    }
}

/// `float contrast_ratio(cb_rgb_255 A, cb_rgb_255 B)`
pub type ContrastRatioFn = unsafe extern "C" fn(Rgb, Rgb) -> f32;

/// The same entry point viewed as taking two raw argument registers, so a test
/// can put junk in the 5 padding bytes of each 3-byte struct.
pub type ContrastRatioRawFn = unsafe extern "C" fn(u64, u64) -> f32;

pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    pub contrast_ratio: ContrastRatioFn,
    pub contrast_ratio_raw: ContrastRatioRawFn,
}

fn dylib_name(stem: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{stem}.dll")
    } else if cfg!(target_os = "macos") {
        format!("lib{stem}.dylib")
    } else {
        format!("lib{stem}.so")
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/<profile>/` — derived from the running test binary
/// (`target/<profile>/deps/<test>-<hash>`), so it is correct for debug, release
/// and any custom profile / target dir.
fn target_profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let mut dir = exe.parent().expect("test binary has a parent dir").to_path_buf();
    if dir.file_name().map(|n| n == "deps").unwrap_or(false) {
        dir.pop();
    }
    dir
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_LIB_PATH") {
        return PathBuf::from(p);
    }
    let base = manifest_dir().join("c_src").join("build");
    // The CMake project name is derived from the parent directory name, so do
    // not hard-code it: pick up whatever shared object the build produced.
    let mut candidates: Vec<PathBuf> = Vec::new();
    for stem in ["translated_rust", "c_src"] {
        candidates.push(base.join(dylib_name(stem)));
    }
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    if let Ok(rd) = std::fs::read_dir(&base) {
        let mut found: Vec<PathBuf> = rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.is_file()
                    && p.extension()
                        .map(|x| x == "so" || x == "dylib" || x == "dll")
                        .unwrap_or(false)
            })
            .collect();
        found.sort();
        if let Some(p) = found.into_iter().next() {
            return p;
        }
    }
    panic!(
        "could not locate the C shared object under {}.\n\
         Build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        base.display()
    );
}

/// Build the Rust `cdylib` freshly, into a **dedicated** target directory.
///
/// This is essential: `cargo test` builds the test harnesses and the *unit-test*
/// binary for the lib target, but it does **not** rebuild the `cdylib` artifact.
/// Relying on whatever `target/debug/libcontrast_ratio_lib.so` happened to be
/// lying around silently tests a stale binary (verified: mutating `src/lib.rs`
/// and re-running `cargo test` left the `.so` untouched, and every test still
/// "passed"). A separate `CARGO_TARGET_DIR` also avoids contending for the outer
/// cargo invocation's build lock.
///
/// Feature selection is inherited from the environment (`DIFF_FEATURES`,
/// `DIFF_NO_DEFAULT_FEATURES`) so the runner script can exercise every feature
/// combination.
fn build_rust_so() -> PathBuf {
    let manifest = manifest_dir();
    let target_dir = manifest.join("target").join("difftest");

    let mut cmd = std::process::Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    cmd.current_dir(&manifest)
        .env("CARGO_TARGET_DIR", &target_dir)
        // Do not inherit the parent cargo's per-invocation state.
        .env_remove("RUSTC_WORKSPACE_WRAPPER")
        .arg("build")
        .arg("--lib");

    // Mirror the profile the tests themselves were built with.
    let release = !cfg!(debug_assertions);
    if release {
        cmd.arg("--release");
    }
    if std::env::var("DIFF_NO_DEFAULT_FEATURES").is_ok() {
        cmd.arg("--no-default-features");
    }
    if let Ok(f) = std::env::var("DIFF_FEATURES") {
        if !f.trim().is_empty() {
            cmd.arg("--features").arg(f);
        }
    }

    let out = cmd.output().expect("failed to spawn `cargo build --lib`");
    if !out.status.success() {
        panic!(
            "`cargo build --lib` failed while producing the Rust cdylib:\n--- stdout ---\n{}\n--- stderr ---\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        );
    }

    let p = target_dir
        .join(if release { "release" } else { "debug" })
        .join(dylib_name("contrast_ratio_lib"));
    assert!(
        p.is_file(),
        "cargo reported success but no cdylib appeared at {}",
        p.display()
    );

    // Staleness guard: the artifact must be at least as new as every source file.
    let so_mtime = std::fs::metadata(&p).and_then(|m| m.modified()).ok();
    if let Some(so_mtime) = so_mtime {
        let src_dir = manifest.join("src");
        if let Ok(rd) = std::fs::read_dir(&src_dir) {
            for e in rd.flatten() {
                if let Ok(m) = e.metadata().and_then(|m| m.modified()) {
                    assert!(
                        so_mtime >= m,
                        "the Rust cdylib at {} is OLDER than {} — the harness would be \
                         testing a stale binary",
                        p.display(),
                        e.path().display()
                    );
                }
            }
        }
    }
    p
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_LIB_PATH") {
        return PathBuf::from(p);
    }
    // [lib] name = "contrast_ratio_lib", crate-type = ["cdylib"]
    build_rust_so()
}

fn load(name: &'static str, path: &Path) -> Impl {
    // The Library is intentionally leaked: the resolved function pointers must
    // stay valid for the whole process lifetime.
    let lib = unsafe { libloading::Library::new(path) }
        .unwrap_or_else(|e| panic!("failed to dlopen {} ({name}): {e}", path.display()));
    let lib: &'static libloading::Library = Box::leak(Box::new(lib));

    let sym: libloading::Symbol<'static, ContrastRatioFn> = unsafe {
        lib.get(b"contrast_ratio\0")
            .unwrap_or_else(|e| panic!("{name}: missing exported symbol `contrast_ratio`: {e}"))
    };
    let contrast_ratio: ContrastRatioFn = *sym;

    let sym_raw: libloading::Symbol<'static, ContrastRatioRawFn> =
        unsafe { lib.get(b"contrast_ratio\0").unwrap() };
    let contrast_ratio_raw: ContrastRatioRawFn = *sym_raw;

    Impl {
        name,
        path: path.to_path_buf(),
        contrast_ratio,
        contrast_ratio_raw,
    }
}

pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

pub fn pair() -> &'static Pair {
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| {
        let c_path = find_c_so();
        let rust_path = find_rust_so();
        eprintln!("C   .so: {}", c_path.display());
        eprintln!("Rust.so: {}", rust_path.display());
        Pair {
            c: load("C", &c_path),
            rust: load("Rust", &rust_path),
        }
    })
}

// ---------------------------------------------------------------------------
// Comparison
// ---------------------------------------------------------------------------

fn describe(x: f32) -> String {
    format!("{x:?} (bits 0x{:08X})", x.to_bits())
}

/// Strict bit-for-bit comparison of the two `f32` results.
///
/// `NaN` payloads are compared too: the C `0.0f/0.0f` on x86 produces the
/// default QNaN `0xFFC00000` and the Rust translation must reproduce the exact
/// same bit pattern, not merely "some NaN".
#[track_caller]
pub fn assert_same(p: &Pair, a: Rgb, b: Rgb, ctx: &str) -> f32 {
    let cv = unsafe { (p.c.contrast_ratio)(a, b) };
    let rv = unsafe { (p.rust.contrast_ratio)(a, b) };
    if cv.to_bits() != rv.to_bits() {
        panic!(
            "MISMATCH [{ctx}]\n  A = {{R:{},G:{},B:{}}}  B = {{R:{},G:{},B:{}}}\n  \
             C    = {}\n  Rust = {}",
            a.r,
            a.g,
            a.b,
            b.r,
            b.g,
            b.b,
            describe(cv),
            describe(rv),
        );
    }
    cv
}

/// Same, but invoked through the raw two-register signature so the caller can
/// control the struct padding bits.
#[track_caller]
pub fn assert_same_raw(p: &Pair, areg: u64, breg: u64, ctx: &str) -> f32 {
    let cv = unsafe { (p.c.contrast_ratio_raw)(areg, breg) };
    let rv = unsafe { (p.rust.contrast_ratio_raw)(areg, breg) };
    if cv.to_bits() != rv.to_bits() {
        panic!(
            "MISMATCH [{ctx}]\n  Areg = 0x{areg:016X}  Breg = 0x{breg:016X}\n  \
             C    = {}\n  Rust = {}",
            describe(cv),
            describe(rv),
        );
    }
    cv
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seeds for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub const fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    /// Uniform in `lo..=hi`.
    pub fn range_u8(&mut self, lo: u8, hi: u8) -> u8 {
        debug_assert!(lo <= hi);
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as u8
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[(self.next_u64() % xs.len() as u64) as usize]
    }
    pub fn color(&mut self) -> Rgb {
        Rgb::new(self.next_u8(), self.next_u8(), self.next_u8())
    }
}

// ---------------------------------------------------------------------------
// Branch-mask helpers (axis L of CONFIGS.md)
// ---------------------------------------------------------------------------

/// A channel byte `n` takes the `pow` branch iff `n/255.f > 0.04045`, i.e. iff
/// `n >= 11`. Verified numerically: `10/255.f = 0.039215688`, `11/255.f =
/// 0.043137256`.
pub const LAST_LINEAR: u8 = 10;
pub const FIRST_POW: u8 = 11;

/// Draw a channel byte that takes the `pow` branch (`true`) or the linear
/// branch (`false`).
pub fn chan_for_branch(rng: &mut Rng, pow_branch: bool) -> u8 {
    if pow_branch {
        rng.range_u8(FIRST_POW, 255)
    } else {
        rng.range_u8(0, LAST_LINEAR)
    }
}

/// `mask` bit0 -> R, bit1 -> G, bit2 -> B; set bit = `pow` branch.
pub fn color_for_mask(rng: &mut Rng, mask: u8) -> Rgb {
    Rgb::new(
        chan_for_branch(rng, mask & 1 != 0),
        chan_for_branch(rng, mask & 2 != 0),
        chan_for_branch(rng, mask & 4 != 0),
    )
}

/// The interesting channel-byte values (axis V of CONFIGS.md): the `> 0.04045`
/// boundary, the domain ends, and the mid-range.
pub const BOUNDARY_BYTES: [u8; 12] = [0, 1, 2, 9, 10, 11, 12, 127, 128, 253, 254, 255];

/// The 8 corners of the RGB cube.
pub fn corners() -> Vec<Rgb> {
    let mut v = Vec::with_capacity(8);
    for &r in &[0u8, 255] {
        for &g in &[0u8, 255] {
            for &b in &[0u8, 255] {
                v.push(Rgb::new(r, g, b));
            }
        }
    }
    v
}

pub const BLACK: Rgb = Rgb::new(0, 0, 0);
pub const WHITE: Rgb = Rgb::new(255, 255, 255);
pub const MID: Rgb = Rgb::new(127, 127, 127);

/// Reference luminance, used only to *classify* inputs into the swap /
/// no-swap / zero-denominator buckets. It is never used as an oracle — the C
/// `.so` is always the oracle.
pub fn approx_luminance(c: Rgb) -> f64 {
    fn lin(n: u8) -> f64 {
        let v = (n as f32 / 255.0f32) as f64;
        let out = if v > 0.04045 {
            ((v + 0.055) / 1.055).powf(2.4)
        } else {
            v / 12.92
        };
        out as f32 as f64
    }
    0.2126f32 as f64 * lin(c.r) + 0.7152f32 as f64 * lin(c.g) + 0.0722f32 as f64 * lin(c.b)
}
