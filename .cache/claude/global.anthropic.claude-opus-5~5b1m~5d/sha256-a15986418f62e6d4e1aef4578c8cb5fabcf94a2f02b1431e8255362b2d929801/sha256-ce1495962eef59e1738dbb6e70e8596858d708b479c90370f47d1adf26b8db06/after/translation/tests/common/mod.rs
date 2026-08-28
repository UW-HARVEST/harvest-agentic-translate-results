//! Shared differential-test harness.
//!
//! Loads BOTH shared objects with `libloading`:
//!   * the C reference  `c_src/build/lib<project>.so`
//!   * the Rust cdylib  `translation/target/<dir>/libto_barycentric_lib.so`
//!
//! and calls `to_barycentric` in each through an identical
//! `unsafe extern "C" fn(Vec2, Vec2, Vec2, Vec2) -> Vec2` pointer, so the
//! `#[no_mangle]` export wrapper and the SysV struct-passing convention are
//! part of what is under test. The Rust implementation is *never* called
//! directly — the crate is `crate-type = ["cdylib"]` only, so it cannot be.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// ABI types
// ---------------------------------------------------------------------------

/// Mirrors `typedef struct lm_vec2 { float x, y; } lm_vec2;`
#[repr(C)]
#[derive(Copy, Clone, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const fn new(x: f32, y: f32) -> Self {
        Vec2 { x, y }
    }
    pub const fn from_bits(x: u32, y: u32) -> Self {
        Vec2 {
            x: f32::from_bits(x),
            y: f32::from_bits(y),
        }
    }
    pub fn bits(self) -> (u32, u32) {
        (self.x.to_bits(), self.y.to_bits())
    }
}

impl std::fmt::Debug for Vec2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({:e}|{:#010x}, {:e}|{:#010x})",
            self.x,
            self.x.to_bits(),
            self.y,
            self.y.to_bits()
        )
    }
}

pub type ToBarycentricFn = unsafe extern "C" fn(Vec2, Vec2, Vec2, Vec2) -> Vec2;

// ---------------------------------------------------------------------------
// Library provisioning
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn repo_root() -> PathBuf {
    manifest_dir().parent().expect("crate has a parent").to_path_buf()
}

fn find_so(dir: &Path) -> Option<PathBuf> {
    let rd = std::fs::read_dir(dir).ok()?;
    let mut hits: Vec<PathBuf> = rd
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib") && n.ends_with(".so"))
                    .unwrap_or(false)
        })
        .collect();
    hits.sort();
    hits.pop()
}

/// Build (once) and return the path to the C reference `.so`.
fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_PATH") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "C_SO_PATH does not exist: {}", p.display());
        return p;
    }

    let c_src = repo_root().join("c_src");
    let build = c_src.join("build");

    if let Some(p) = find_so(&build) {
        return p;
    }

    std::fs::create_dir_all(&build).expect("mkdir c_src/build");
    let cfg = Command::new("cmake")
        .arg("..")
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .current_dir(&build)
        .output()
        .expect("run cmake configure");
    assert!(
        cfg.status.success(),
        "cmake configure failed:\n{}\n{}",
        String::from_utf8_lossy(&cfg.stdout),
        String::from_utf8_lossy(&cfg.stderr)
    );
    let bld = Command::new("cmake")
        .args(["--build", "."])
        .current_dir(&build)
        .output()
        .expect("run cmake build");
    assert!(
        bld.status.success(),
        "cmake build failed:\n{}\n{}",
        String::from_utf8_lossy(&bld.stdout),
        String::from_utf8_lossy(&bld.stderr)
    );

    find_so(&build).expect("C .so produced by cmake --build")
}

/// Build (once) and return the path to the Rust cdylib.
///
/// `cargo test` does *not* emit the cdylib for a `crate-type = ["cdylib"]`
/// crate (it builds the lib as a test harness instead), so the harness builds
/// it explicitly into a side target dir. The feature flags of the current test
/// run are forwarded through `FFI_FEATURE_ARGS` (set by
/// `check_all_features.sh`), and the target dir is keyed on them so that
/// different feature combinations never see each other's artifacts.
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "RUST_SO_PATH does not exist: {}", p.display());
        return p;
    }

    let feature_args: Vec<String> = std::env::var("FFI_FEATURE_ARGS")
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_owned)
        .collect();

    // Stable, filesystem-safe key for this feature combination.
    let key = if feature_args.is_empty() {
        "default".to_string()
    } else {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in feature_args.join(" ").bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        format!("feat-{h:016x}")
    };

    let target_dir = manifest_dir().join("target").join("ffi").join(&key);

    let mut cmd = Command::new(env!("CARGO"));
    cmd.arg("build")
        .arg("--offline")
        .arg("--release")
        .arg("--target-dir")
        .arg(&target_dir)
        .args(&feature_args)
        .current_dir(manifest_dir())
        // Keep the child cargo from inheriting the parent's rustflags/profile env.
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env_remove("RUSTC_WORKSPACE_WRAPPER");
    let out = cmd.output().expect("run cargo build for cdylib");
    assert!(
        out.status.success(),
        "cargo build (cdylib) failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let rel = target_dir.join("release");
    let p = rel.join("libto_barycentric_lib.so");
    if p.is_file() {
        return p;
    }
    find_so(&rel).unwrap_or_else(|| panic!("no .so under {}", rel.display()))
}

pub struct Libs {
    pub c: ToBarycentricFn,
    pub rust: ToBarycentricFn,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

// SAFETY: both are plain `extern "C"` fn pointers into leaked, never-unloaded
// libraries; the underlying C function is pure (no globals, per ERRORS.md).
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = rust_so_path();

        // Leak both libraries so the symbols stay valid for the whole process.
        let c_lib: &'static libloading::Library = Box::leak(Box::new(unsafe {
            libloading::Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()))
        }));
        let rust_lib: &'static libloading::Library = Box::leak(Box::new(unsafe {
            libloading::Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_path.display()))
        }));

        let c: libloading::Symbol<'static, ToBarycentricFn> = unsafe {
            c_lib
                .get(b"to_barycentric\0")
                .expect("C .so exports to_barycentric")
        };
        let rust: libloading::Symbol<'static, ToBarycentricFn> = unsafe {
            rust_lib
                .get(b"to_barycentric\0")
                .expect("Rust .so exports to_barycentric")
        };

        Libs {
            c: *c,
            rust: *rust,
            c_path,
            rust_path,
        }
    })
}

pub fn c_so() -> PathBuf {
    libs().c_path.clone()
}
pub fn rust_so() -> PathBuf {
    libs().rust_path.clone()
}

// ---------------------------------------------------------------------------
// Differential comparison
// ---------------------------------------------------------------------------

/// Call both `.so`s and assert the returned `lm_vec2`s are byte-identical.
///
/// Compares raw bit patterns, so `+0.0` vs `-0.0` and differing NaN payloads
/// are both failures.
#[track_caller]
pub fn diff(row: &str, p1: Vec2, p2: Vec2, p3: Vec2, p: Vec2) {
    let l = libs();
    let rc = unsafe { (l.c)(p1, p2, p3, p) };
    let rr = unsafe { (l.rust)(p1, p2, p3, p) };
    if rc.bits() != rr.bits() {
        panic!(
            "[{row}] DIVERGENCE\n  p1 = {p1:?}\n  p2 = {p2:?}\n  p3 = {p3:?}\n  p  = {p:?}\n\
             \n  C    = {rc:?}\n  Rust = {rr:?}\n\
             \n  repro: to_barycentric(\n\
             \x20   Vec2::from_bits({:#010x}, {:#010x}),\n\
             \x20   Vec2::from_bits({:#010x}, {:#010x}),\n\
             \x20   Vec2::from_bits({:#010x}, {:#010x}),\n\
             \x20   Vec2::from_bits({:#010x}, {:#010x}))",
            p1.x.to_bits(),
            p1.y.to_bits(),
            p2.x.to_bits(),
            p2.y.to_bits(),
            p3.x.to_bits(),
            p3.y.to_bits(),
            p.x.to_bits(),
            p.y.to_bits(),
        );
    }
}

/// Like [`diff`] but also returns the (agreed) result for extra assertions.
#[track_caller]
pub fn diff_get(row: &str, p1: Vec2, p2: Vec2, p3: Vec2, p: Vec2) -> Vec2 {
    diff(row, p1, p2, p3, p);
    let l = libs();
    unsafe { (l.c)(p1, p2, p3, p) }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64) — fixed seeds, no external crates
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    /// Every test row uses its own fixed seed so failures are reproducible.
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
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform in `0..n` (n > 0).
    #[inline]
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }

    #[inline]
    pub fn chance(&mut self, percent: u32) -> bool {
        self.below(100) < percent
    }

    /// Uniform in `[0, 1)`.
    #[inline]
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    // -- float generators, one per value class in CONFIGS.md axis 3 --------

    /// Any of the 2^32 bit patterns (≈2 % NaN, ≈0 % inf, all classes).
    #[inline]
    pub fn any_bits(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }

    /// Small integer in `-8..=8`.
    #[inline]
    pub fn small_int(&mut self) -> f32 {
        (self.below(17) as i32 - 8) as f32
    }

    /// Integer in `-64..=64`.
    #[inline]
    pub fn lattice_int(&mut self) -> f32 {
        (self.below(129) as i32 - 64) as f32
    }

    /// Quarter-step value in `[-4, 4]` (exactly representable).
    #[inline]
    pub fn quarter(&mut self) -> f32 {
        (self.below(33) as i32 - 16) as f32 / 4.0
    }

    /// Dyadic: 8-bit mantissa, exponent 2^-8..2^8 — all products exact.
    pub fn dyadic(&mut self) -> f32 {
        let m = (self.below(256) as i32) - 128; // -128..127
        let e = (self.below(17) as i32) - 8; // -8..8
        (m as f32) * 2f32.powi(e)
    }

    /// Full-mantissa normal float with exponent in `2^lo .. 2^hi`.
    pub fn normal_in(&mut self, lo: i32, hi: i32) -> f32 {
        let span = (hi - lo + 1) as u32;
        let e = lo + self.below(span) as i32;
        let mant = self.next_u32() & 0x007F_FFFF;
        // Build a normal directly: sign | biased-exponent | mantissa.
        let biased = (e + 127) as u32;
        debug_assert!(biased >= 1 && biased <= 254);
        let sign = (self.next_u32() & 1) << 31;
        f32::from_bits(sign | (biased << 23) | mant)
    }

    /// Power of two `±2^k`, `k ∈ [lo, hi]`.
    pub fn pow2(&mut self, lo: i32, hi: i32) -> f32 {
        let span = (hi - lo + 1) as u32;
        let k = lo + self.below(span) as i32;
        let v = 2f32.powi(k);
        if self.next_u32() & 1 == 0 {
            v
        } else {
            -v
        }
    }

    /// Random-payload quiet NaN, random sign.
    pub fn qnan(&mut self) -> f32 {
        let sign = (self.next_u32() & 1) << 31;
        // quiet bit set, payload never all-zero-below-quiet-bit is fine either way
        let payload = self.next_u32() & 0x003F_FFFF;
        f32::from_bits(sign | 0x7F80_0000 | 0x0040_0000 | payload)
    }

    /// Random-payload *signalling* NaN (quiet bit clear, payload non-zero).
    pub fn snan(&mut self) -> f32 {
        let sign = (self.next_u32() & 1) << 31;
        let mut payload = self.next_u32() & 0x003F_FFFF;
        if payload == 0 {
            payload = 1; // payload 0 with quiet bit clear would be infinity
        }
        f32::from_bits(sign | 0x7F80_0000 | payload)
    }

    /// Either kind of NaN.
    pub fn any_nan(&mut self) -> f32 {
        if self.next_u32() & 1 == 0 {
            self.qnan()
        } else {
            self.snan()
        }
    }

    /// `+inf` or `-inf`.
    pub fn inf(&mut self) -> f32 {
        if self.next_u32() & 1 == 0 {
            f32::INFINITY
        } else {
            f32::NEG_INFINITY
        }
    }

    /// Subnormal, random sign.
    pub fn subnormal(&mut self) -> f32 {
        let sign = (self.next_u32() & 1) << 31;
        let mut m = self.next_u32() & 0x007F_FFFF;
        if m == 0 {
            m = 1;
        }
        f32::from_bits(sign | m)
    }

    /// One entry from [`SPECIALS`].
    pub fn special(&mut self) -> f32 {
        let i = self.below(SPECIALS.len() as u32) as usize;
        f32::from_bits(SPECIALS[i])
    }

    /// Draw a `Vec2` with `f` applied to each component.
    pub fn vec2(&mut self, mut f: impl FnMut(&mut Self) -> f32) -> Vec2 {
        let x = f(self);
        let y = f(self);
        Vec2::new(x, y)
    }
}

/// The 24-entry special-value table used by config row B14.
pub const SPECIALS: [u32; 24] = [
    0x0000_0000, // +0.0
    0x8000_0000, // -0.0
    0x0000_0001, // +FLT_TRUE_MIN (smallest subnormal)
    0x8000_0001, // -FLT_TRUE_MIN
    0x007F_FFFF, // largest positive subnormal
    0x807F_FFFF, // largest negative subnormal
    0x0080_0000, // +FLT_MIN
    0x8080_0000, // -FLT_MIN
    0x3F80_0000, // +1.0
    0xBF80_0000, // -1.0
    0x3F80_0001, // +1.0 + 1ulp
    0x4000_0000, // +2.0
    0x3F00_0000, // +0.5
    0x7F7F_FFFF, // +FLT_MAX
    0xFF7F_FFFF, // -FLT_MAX
    0x7F00_0000, // large power of two
    0x7F80_0000, // +inf
    0xFF80_0000, // -inf
    0x7FC0_0000, // +QNaN (default)
    0xFFC0_0000, // -QNaN (x86 "real indefinite")
    0x7FC0_1234, // +QNaN, payload
    0xFFDE_ADBE, // -QNaN, payload
    0x7F80_0001, // +SNaN
    0xFF80_0BAD, // -SNaN, payload
];

// ---------------------------------------------------------------------------
// Handy constants
// ---------------------------------------------------------------------------

pub const IND: u32 = 0xFFC0_0000; // x86 default QNaN from an invalid operation
pub const P_ZERO: Vec2 = Vec2::new(0.0, 0.0);

pub fn is_nan_bits(b: u32) -> bool {
    (b & 0x7F80_0000) == 0x7F80_0000 && (b & 0x007F_FFFF) != 0
}
