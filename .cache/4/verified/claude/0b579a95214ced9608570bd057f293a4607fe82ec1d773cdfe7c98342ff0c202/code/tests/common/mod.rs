//! Shared differential-test harness.
//!
//! Both implementations are loaded as **shared objects** through `libloading`
//! and called through their exported `ldexp_q2` symbol. The Rust crate is never
//! linked directly — this deliberately exercises the `#[no_mangle]`/`extern "C"`
//! export wrapper exactly as an external C consumer would.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// ABI of the single public entry point (`float ldexp_q2(float, int)`).
pub type LdexpQ2 = unsafe extern "C" fn(f32, i32) -> f32;

pub struct Impl {
    pub name: String,
    pub path: PathBuf,
    func: LdexpQ2,
}

impl Impl {
    #[inline]
    pub fn ldexp_q2(&self, y: f32, exp_q2: i32) -> f32 {
        unsafe { (self.func)(y, exp_q2) }
    }
}

pub struct Harness {
    pub c: Impl,
    /// Every Rust `cdylib` we could find (release and/or debug profile).
    pub rust: Vec<Impl>,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load(path: &Path, name: &str) -> Impl {
    // Leak the Library so the resolved function pointer is valid for 'static.
    let lib: &'static Library = Box::leak(Box::new(unsafe {
        Library::new(path).unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()))
    }));
    let sym: Symbol<'static, LdexpQ2> = unsafe {
        lib.get(b"ldexp_q2\0")
            .unwrap_or_else(|e| panic!("dlsym(ldexp_q2) in {} failed: {e}", path.display()))
    };
    Impl {
        name: name.to_string(),
        path: path.to_path_buf(),
        func: *sym,
    }
}

/// Build `c_src` with the documented default configuration if it is not present.
fn ensure_c_so() -> PathBuf {
    let root = manifest_dir();
    let build = root.join("c_src/build");
    let so = build.join("libtranslated_rust.so");
    if so.is_file() {
        return so;
    }
    std::fs::create_dir_all(&build).expect("mkdir c_src/build");
    let status = Command::new("cmake")
        .current_dir(&build)
        .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
        .status()
        .expect("failed to run cmake (configure)");
    assert!(status.success(), "cmake configure failed");
    let status = Command::new("cmake")
        .current_dir(&build)
        .args(["--build", "."])
        .status()
        .expect("failed to run cmake (build)");
    assert!(status.success(), "cmake build failed");
    assert!(so.is_file(), "C .so not produced at {}", so.display());
    so
}

/// Locate every available Rust `cdylib`.
///
/// `cargo test` does not build the `cdylib` target, so as a last resort we build
/// it into a private target directory (avoiding any lock contention with the
/// `cargo test` invocation that is running us).
fn rust_so_paths() -> Vec<PathBuf> {
    let root = manifest_dir();
    let mut found = Vec::new();

    if let Ok(list) = std::env::var("LDEXP_RUST_SO") {
        for p in list.split(':').filter(|s| !s.is_empty()) {
            let p = PathBuf::from(p);
            assert!(p.is_file(), "LDEXP_RUST_SO entry not found: {}", p.display());
            found.push(p);
        }
        return found;
    }

    let target = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| root.join("target"));
    for profile in ["release", "debug"] {
        let p = target.join(profile).join("libldexp_q2_lib.so");
        if p.is_file() {
            found.push(p);
        }
    }
    if !found.is_empty() {
        return found;
    }

    // Fallback: build the cdylib ourselves.
    let private = target.join("difftest-cdylib");
    let status = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .current_dir(&root)
        .args([
            "build",
            "--release",
            "--lib",
            "--target-dir",
            private.to_str().unwrap(),
        ])
        .status()
        .expect("failed to run cargo build for the cdylib");
    assert!(status.success(), "cargo build --lib failed");
    let p = private.join("release/libldexp_q2_lib.so");
    assert!(p.is_file(), "cdylib not produced at {}", p.display());
    vec![p]
}

pub fn harness() -> &'static Harness {
    static H: OnceLock<Harness> = OnceLock::new();
    H.get_or_init(|| {
        let c_path = ensure_c_so();
        let c = load(&c_path, "C");
        let rust: Vec<Impl> = rust_so_paths()
            .iter()
            .map(|p| {
                let name = format!(
                    "rust[{}]",
                    p.parent()
                        .and_then(|d| d.file_name())
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default()
                );
                load(p, &name)
            })
            .collect();
        assert!(!rust.is_empty(), "no Rust cdylib found to compare against");
        eprintln!(
            "[harness] C = {}\n[harness] Rust = {}",
            c.path.display(),
            rust
                .iter()
                .map(|r| r.path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
        Harness { c, rust }
    })
}

// ---------------------------------------------------------------------------
// Comparison
// ---------------------------------------------------------------------------

/// Compare one `(y, exp_q2)` pair across C and every Rust `.so`, bit-for-bit.
///
/// Returns the C result so callers can additionally assert on it.
#[inline]
pub fn check(row: &str, y: f32, exp_q2: i32) -> f32 {
    let h = harness();
    let expect = h.c.ldexp_q2(y, exp_q2);
    for r in &h.rust {
        let got = r.ldexp_q2(y, exp_q2);
        assert_eq!(
            expect.to_bits(),
            got.to_bits(),
            "[{row}] {} diverged: ldexp_q2(y=0x{:08x} ({:e}), exp_q2={}) \
             C=0x{:08x} ({:e})  Rust=0x{:08x} ({:e})",
            r.name,
            y.to_bits(),
            y,
            exp_q2,
            expect.to_bits(),
            expect,
            got.to_bits(),
            got,
        );
    }
    expect
}

/// Like [`check`] but also asserts the C result equals a known-good bit pattern
/// (used by the error-path table, where the expected value is documented).
pub fn check_exact(row: &str, y: f32, exp_q2: i32, expected_bits: u32) {
    let got = check(row, y, exp_q2);
    assert_eq!(
        got.to_bits(),
        expected_bits,
        "[{row}] reference C result changed: ldexp_q2(0x{:08x}, {}) = 0x{:08x}, \
         ERRORS.md documents 0x{:08x}",
        y.to_bits(),
        exp_q2,
        got.to_bits(),
        expected_bits
    );
}

// ---------------------------------------------------------------------------
// Deterministic RNG (xorshift64*) — no external dev-dependency needed.
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_ABCD_0001;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }
    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in `[lo, hi]` (inclusive), works across the whole i64 range.
    #[inline]
    pub fn range_i64(&mut self, lo: i64, hi: i64) -> i64 {
        debug_assert!(lo <= hi);
        let span = (hi - lo) as u64;
        if span == u64::MAX {
            return self.next_u64() as i64;
        }
        lo + (self.next_u64() % (span + 1)) as i64
    }
    #[inline]
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        self.range_i64(lo as i64, hi as i64) as i32
    }
    /// Any `f32` bit pattern (includes NaNs, infinities, subnormals, ±0).
    #[inline]
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    /// A finite normal `f32` with a random sign and a broad exponent range.
    pub fn normal_f32(&mut self) -> f32 {
        let bits = self.next_u32();
        let sign = bits & 0x8000_0000;
        // exponent in [1, 254] -> never zero/subnormal, never inf/NaN
        let exp = (((bits >> 23) & 0xFF) % 254 + 1) << 23;
        f32::from_bits(sign | exp | (bits & 0x007F_FFFF))
    }
    /// A subnormal `f32` (exponent field 0, non-zero mantissa), random sign.
    pub fn subnormal_f32(&mut self) -> f32 {
        let bits = self.next_u32();
        let sign = bits & 0x8000_0000;
        let mant = (bits & 0x007F_FFFF).max(1);
        f32::from_bits(sign | mant)
    }
    /// A NaN with a random payload and sign; `quiet == false` yields a signalling NaN.
    pub fn nan_f32(&mut self, quiet: bool) -> f32 {
        let bits = self.next_u32();
        let sign = bits & 0x8000_0000;
        let payload = (bits & 0x003F_FFFF).max(1);
        let quiet_bit = if quiet { 0x0040_0000 } else { 0 };
        f32::from_bits(sign | 0x7F80_0000 | quiet_bit | payload)
    }
}

// ---------------------------------------------------------------------------
// Fixed input panels
// ---------------------------------------------------------------------------

/// Every distinct `f32` class, including signed zeros, subnormals, extremes,
/// infinities and NaNs (quiet + signalling, both signs, odd payloads).
pub fn y_panel() -> Vec<f32> {
    [
        0x0000_0000u32, // +0
        0x8000_0000,    // -0
        0x0000_0001,    // + min subnormal
        0x8000_0001,    // - min subnormal
        0x007F_FFFF,    // + max subnormal
        0x807F_FFFF,    // - max subnormal
        0x0080_0000,    // + min normal (FLT_MIN)
        0x8080_0000,    // - min normal
        0x3F80_0000,    // +1.0
        0xBF80_0000,    // -1.0
        0x3F00_0000,    // +0.5
        0x4049_0FDB,    // +pi
        0xC049_0FDB,    // -pi
        0x7F7F_FFFF,    // +FLT_MAX
        0xFF7F_FFFF,    // -FLT_MAX
        0x7F80_0000,    // +inf
        0xFF80_0000,    // -inf
        0x7FC0_0000,    // +qNaN (default)
        0xFFC0_0000,    // -qNaN (x86 "real indefinite")
        0x7FC0_0001,    // +qNaN, payload 1
        0xFFC0_DEAD,    // -qNaN, odd payload
        0x7F80_0001,    // +sNaN, payload 1
        0xFFBF_FFFF,    // -sNaN, max payload
        0x0000_0002,    // + subnormal 2
    ]
    .iter()
    .map(|&b| f32::from_bits(b))
    .collect()
}

/// Every `exp_q2` value at a documented branch boundary of the C code.
pub fn exp_boundary_panel() -> Vec<i32> {
    vec![
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 2,
        i32::MIN + 3,
        i32::MIN + 124,
        i32::MIN + 125,
        i32::MIN + 128,
        -2_000_000,
        -1024,
        -132,
        -131,
        -130,
        -129,
        -128,
        -127,
        -126,
        -125,
        -124,
        -123,
        -9,
        -8,
        -7,
        -6,
        -5,
        -4,
        -3,
        -2,
        -1,
        0,
        1,
        2,
        3,
        4,
        5,
        115,
        116,
        117,
        118,
        119,
        120,
        121,
        122,
        123,
        124,
        239,
        240,
        241,
        242,
        360,
        361,
        20_000,
    ]
}
