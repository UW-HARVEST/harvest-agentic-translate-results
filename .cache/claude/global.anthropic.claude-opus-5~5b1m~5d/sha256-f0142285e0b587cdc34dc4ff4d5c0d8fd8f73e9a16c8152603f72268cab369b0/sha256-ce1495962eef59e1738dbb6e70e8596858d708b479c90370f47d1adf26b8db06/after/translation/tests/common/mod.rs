//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls `ldexp_q2` only via
//! its exported C symbol. The Rust implementation is never called directly, so
//! the `#[unsafe(no_mangle)] extern "C"` export wrapper and the C ABI are part
//! of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The C signature: `float ldexp_q2(float y, int exp_q2);`
pub type LdexpQ2 = unsafe extern "C" fn(f32, std::ffi::c_int) -> f32;

pub struct Impls {
    _c_lib: Library,
    _rust_lib: Library,
    c: LdexpQ2,
    rust: LdexpQ2,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

impl Impls {
    /// Call the **C** `.so`'s exported `ldexp_q2`.
    #[inline]
    pub fn c(&self, y: f32, exp_q2: i32) -> f32 {
        unsafe { (self.c)(y, exp_q2) }
    }

    /// Call the **Rust** `.so`'s exported `ldexp_q2`.
    #[inline]
    pub fn rust(&self, y: f32, exp_q2: i32) -> f32 {
        unsafe { (self.rust)(y, exp_q2) }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Locate the C shared object. The CMake target name is derived from the parent
/// directory name, so glob for `c_src/build/lib*.so` instead of hardcoding.
fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_PATH") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "C_SO_PATH does not exist: {}", p.display());
        return p;
    }
    let build_dir = manifest_dir().join("../c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build_dir) {
        for entry in rd.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with("lib") && name.ends_with(".so") && path.is_file() {
                found.push(path);
            }
        }
    }
    found.sort();
    assert!(
        !found.is_empty(),
        "no C .so found in {}. Build it first:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build_dir.display()
    );
    found.remove(0)
}

/// Locate the Rust cdylib. Prefer the profile the test binary itself was built
/// under (derived from the test executable's own path), then fall back.
fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "RUST_SO_PATH does not exist: {}", p.display());
        return p;
    }
    const SO: &str = "libldexp_q2_lib.so";

    // The test binary lives in <target>/<profile>/deps/<name>-<hash>, so the
    // cdylib for the same profile is two levels up.
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe()
        && let Some(deps) = exe.parent()
    {
        candidates.push(deps.join(SO));
        if let Some(profile_dir) = deps.parent() {
            candidates.push(profile_dir.join(SO));
        }
    }
    let md = manifest_dir();
    candidates.push(md.join("target/release").join(SO));
    candidates.push(md.join("target/debug").join(SO));

    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "Rust cdylib {SO} not found. Tried:\n{}\nBuild it with `cargo build` / `cargo build --release`.",
        candidates
            .iter()
            .map(|p| format!("  {}", p.display()))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// Guard against the silent-stale-artifact failure mode.
///
/// Integration tests do not link the `cdylib`, so `cargo test` will happily run
/// against a `.so` left over from an earlier build unless the lib target is
/// also built. (`crate-type` includes `rlib` precisely so that `cargo test`
/// rebuilds it.) If the `.so` is nonetheless older than any Rust source file,
/// every differential assertion would be meaningless, so fail loudly instead.
fn assert_not_stale(rust_so: &Path) {
    let so_mtime = match std::fs::metadata(rust_so).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return,
    };
    let src_dir = manifest_dir().join("src");
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    let mut stack = vec![src_dir];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|e| e == "rs")
                && let Ok(t) = entry.metadata().and_then(|m| m.modified())
                && newest.as_ref().is_none_or(|(nt, _)| t > *nt)
            {
                newest = Some((t, p));
            }
        }
    }
    if let Some((src_mtime, src_path)) = newest {
        assert!(
            so_mtime >= src_mtime,
            "STALE ARTIFACT: {} is older than {}.\n\
             The differential tests would be comparing the C library against an \
             out-of-date Rust build, so every result would be meaningless.\n\
             Run `cargo build --release && cargo build` (or ./scripts/verify_all.sh) first.",
            rust_so.display(),
            src_path.display()
        );
    }
}

fn load(path: &Path) -> (Library, LdexpQ2) {
    let lib = unsafe { Library::new(path) }
        .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
    let f: LdexpQ2 = unsafe {
        let sym: Symbol<LdexpQ2> = lib
            .get(b"ldexp_q2\0")
            .unwrap_or_else(|e| panic!("symbol `ldexp_q2` missing from {}: {e}", path.display()));
        *sym
    };
    (lib, f)
}

/// Process-wide singleton so the libraries are dlopen'd once.
pub fn impls() -> &'static Impls {
    static IMPLS: OnceLock<Impls> = OnceLock::new();
    IMPLS.get_or_init(|| {
        let c_path = find_c_so();
        let rust_path = find_rust_so();
        assert_not_stale(&rust_path);
        let (c_lib, c) = load(&c_path);
        let (rust_lib, rust) = load(&rust_path);
        Impls {
            _c_lib: c_lib,
            _rust_lib: rust_lib,
            c,
            rust,
            c_path,
            rust_path,
        }
    })
}

// ---------------------------------------------------------------------------
// Bit-exact comparison
// ---------------------------------------------------------------------------

/// Compare a single (y, exp_q2) pair between C and Rust **by raw bits**, so
/// `+0.0` vs `-0.0` and NaN sign/payload differences are not silently accepted.
///
/// Returns `Err(description)` on divergence.
pub fn check(y: f32, exp_q2: i32) -> Result<(), String> {
    let im = impls();
    let cv = im.c(y, exp_q2);
    let rv = im.rust(y, exp_q2);
    if cv.to_bits() == rv.to_bits() {
        Ok(())
    } else {
        Err(format!(
            "DIVERGENCE ldexp_q2(y=0x{:08x} [{:e}], exp_q2={} [0x{:08x}]):\n  \
             C    = 0x{:08x} [{:e}]\n  Rust = 0x{:08x} [{:e}]",
            y.to_bits(),
            y,
            exp_q2,
            exp_q2 as u32,
            cv.to_bits(),
            cv,
            rv.to_bits(),
            rv,
        ))
    }
}

/// Run `check` over an iterator of cases, collecting up to 10 failures so a
/// broken row reports a useful sample instead of only the first mismatch.
pub fn check_all<I: IntoIterator<Item = (f32, i32)>>(label: &str, cases: I) {
    let mut samples: Vec<String> = Vec::new();
    let mut failed: u64 = 0;
    let mut count: u64 = 0;
    for (y, exp_q2) in cases {
        count += 1;
        if let Err(e) = check(y, exp_q2) {
            failed += 1;
            if samples.len() < 10 {
                samples.push(e);
            }
        }
    }
    assert!(count > 0, "{label}: generated 0 cases (test is vacuous!)");
    assert!(
        failed == 0,
        "{label}: {failed} of {count} cases diverged. First {}:\n\n{}",
        samples.len(),
        samples.join("\n\n")
    );
    eprintln!("{label}: {count} cases OK");
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    #[inline]
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `[lo, hi]` inclusive.
    #[inline]
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    /// An arbitrary `f32` from raw random bits (may be any class: normal,
    /// subnormal, zero, inf, qNaN, sNaN).
    #[inline]
    pub fn next_f32_bits(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    /// A random **finite normal** `f32` with a full 24-bit random mantissa and
    /// a random sign, exponent kept in a range where the scaling stays
    /// meaningful (biased exponent 1..254 avoids inf/NaN encodings).
    #[inline]
    pub fn next_normal_f32(&mut self) -> f32 {
        let r = self.next_u64();
        let sign = ((r >> 63) as u32) << 31;
        let exp = (((r >> 32) as u32) % 254 + 1) << 23; // biased 1..=254
        let mant = (r as u32) & 0x007F_FFFF;
        f32::from_bits(sign | exp | mant)
    }
    /// A random `f32` whose magnitude is moderate (biased exponent 100..154),
    /// so products neither overflow nor underflow — isolates pure rounding.
    #[inline]
    pub fn next_midrange_f32(&mut self) -> f32 {
        let r = self.next_u64();
        let sign = ((r >> 63) as u32) << 31;
        let exp = (((r >> 32) as u32) % 55 + 100) << 23;
        let mant = (r as u32) & 0x007F_FFFF;
        f32::from_bits(sign | exp | mant)
    }
    /// A random subnormal (biased exponent 0, non-zero mantissa).
    #[inline]
    pub fn next_subnormal_f32(&mut self) -> f32 {
        let r = self.next_u32();
        let sign = (r >> 31) << 31;
        let mant = (r & 0x007F_FFFF).max(1);
        f32::from_bits(sign | mant)
    }
}

// ---------------------------------------------------------------------------
// Special-value catalogues
// ---------------------------------------------------------------------------

/// Every interesting IEEE-754 `f32` class, as raw bits so NaN payloads and
/// signed zeros survive exactly.
pub const SPECIAL_Y_BITS: &[u32] = &[
    0x0000_0000, // +0.0
    0x8000_0000, // -0.0
    0x0000_0001, // smallest positive subnormal
    0x8000_0001, // smallest negative subnormal
    0x007F_FFFF, // largest positive subnormal
    0x807F_FFFF, // largest negative subnormal
    0x0080_0000, // FLT_MIN (smallest positive normal)
    0x8080_0000, // -FLT_MIN
    0x3F80_0000, // +1.0
    0xBF80_0000, // -1.0
    0x4000_0000, // +2.0
    0x3F00_0000, // +0.5
    0x7F7F_FFFF, // FLT_MAX
    0xFF7F_FFFF, // -FLT_MAX
    0x7F80_0000, // +inf
    0xFF80_0000, // -inf
    0x7FC0_0000, // +qNaN (default)
    0xFFC0_0000, // -qNaN
    0x7FC0_1234, // +qNaN with payload
    0x7FA0_0000, // +sNaN
    0xFFA0_0000, // -sNaN
    0x7F80_0001, // +sNaN, minimal payload
    0x3FC0_0000, // +1.5
    0x4B7F_FFFF, // 16777215.0 (2^24-1, largest exactly-representable odd int)
];

pub fn special_ys() -> Vec<f32> {
    SPECIAL_Y_BITS.iter().map(|&b| f32::from_bits(b)).collect()
}

/// The NaN subset (quiet and signalling, both signs, with payloads).
pub const NAN_Y_BITS: &[u32] = &[
    0x7FC0_0000,
    0xFFC0_0000,
    0x7FC0_1234,
    0xFFC0_5678,
    0x7FFF_FFFF,
    0xFFFF_FFFF,
    0x7FC0_0001,
    0xFFDE_ADBE,
];

pub const SNAN_Y_BITS: &[u32] = &[0x7FA0_0000, 0xFFA0_0000, 0x7F80_0001, 0xFF80_0001];

/// `exp_q2` values that produce each distinct scale regime, derived from
/// `k = (e >> 2) & 31`:
///   k == 0  -> scale 2^30 (identity, since frac[0] * 2^30 == 1.0f)
///   k == 30 -> scale 1
///   k == 31 -> scale 0 (annihilates y)
pub const EXP_IDENTITY: i32 = 0;
pub const EXP_SCALE_ZERO: &[i32] = &[-1, -2, -3, -4]; // k == 31, all 4 residues
pub const EXP_SCALE_ONE_NEG: &[i32] = &[-5, -6, -7, -8]; // k == 30, all 4 residues
pub const EXP_SCALE_ONE_POS: i32 = 120; // k == 30
pub const EXP_NEG_IDENTITY: &[i32] = &[-128, -256, -384, -512, -1024, -1152]; // k == 0
pub const EXP_MULTITRIP: &[i32] = &[121, 200, 240, 241, 500, 1200, 12000];
