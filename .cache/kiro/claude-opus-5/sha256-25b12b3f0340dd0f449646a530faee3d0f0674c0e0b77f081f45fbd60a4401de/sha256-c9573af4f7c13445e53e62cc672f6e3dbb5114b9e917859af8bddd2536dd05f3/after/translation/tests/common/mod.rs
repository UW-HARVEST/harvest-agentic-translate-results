//! Shared differential-test harness.
//!
//! Loads BOTH shared objects with `libloading` and calls `ldexp_q2` through the
//! `.so` exports only. The Rust function is never called directly, so the
//! `#[no_mangle]` / `extern "C"` wrapper is part of what is under test.

#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::OnceLock;

use libloading::{Library, Symbol};

pub type LdexpQ2 = unsafe extern "C" fn(f32, i32) -> f32;

/// Repository root (parent of the `translation/` crate directory).
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate dir has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_PATH") {
        return PathBuf::from(p);
    }
    let build_dir = repo_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&build_dir) {
        for e in entries.flatten() {
            let p = e.path();
            let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            if name.starts_with("lib") && name.ends_with(".so") {
                candidates.push(p);
            }
        }
    }
    candidates.sort();
    candidates.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no C shared object found in {}. Build it with:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build_dir.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        return PathBuf::from(p);
    }
    // The integration-test binary lives in target/<profile>/deps/, so the
    // cdylib sits one directory up. Fall back to the well-known locations.
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(deps) = exe.parent() {
            if let Some(profile_dir) = deps.parent() {
                roots.push(profile_dir.to_path_buf());
            }
        }
    }
    let target = repo_root().join("translation").join("target");
    roots.push(target.join("release"));
    roots.push(target.join("debug"));

    for r in &roots {
        let p = r.join("libldexp_q2_lib.so");
        if p.is_file() {
            return p;
        }
    }
    panic!(
        "no Rust cdylib libldexp_q2_lib.so found; looked in {:?}. \
         Build it with: cd translation && cargo build --release",
        roots
    );
}

struct Libs {
    c: Library,
    rust: Library,
}

// Safety: the libraries stay loaded for the whole process lifetime.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c_path = find_c_so();
        let rust_path = find_rust_so();
        let c = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("failed to dlopen C .so {}: {e}", c_path.display()));
        let rust = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|e| panic!("failed to dlopen Rust .so {}: {e}", rust_path.display()));
        Libs { c, rust }
    })
}

/// The two `ldexp_q2` implementations, both obtained via `dlsym`.
pub struct Pair {
    pub c: LdexpQ2,
    pub rust: LdexpQ2,
}

pub fn pair() -> &'static Pair {
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| {
        let l = libs();
        let c: Symbol<LdexpQ2> = unsafe { l.c.get(b"ldexp_q2\0") }
            .expect("symbol `ldexp_q2` missing from the C .so");
        let rust: Symbol<LdexpQ2> = unsafe { l.rust.get(b"ldexp_q2\0") }
            .expect("symbol `ldexp_q2` missing from the Rust .so (check #[no_mangle])");
        Pair { c: *c, rust: *rust }
    })
}

/// Bit-exact comparison of one input pair. Returns `Err(message)` on divergence.
pub fn check(y: f32, exp_q2: i32) -> Result<f32, String> {
    let p = pair();
    let cv = unsafe { (p.c)(y, exp_q2) };
    let rv = unsafe { (p.rust)(y, exp_q2) };
    if cv.to_bits() == rv.to_bits() {
        Ok(cv)
    } else {
        Err(format!(
            "DIVERGENCE for ldexp_q2(y = {y:?} [bits 0x{ybits:08X}], exp_q2 = {exp_q2} [0x{ebits:08X}])\n  \
             C    = {cv:?} (bits 0x{cb:08X})\n  \
             Rust = {rv:?} (bits 0x{rb:08X})",
            ybits = y.to_bits(),
            ebits = exp_q2 as u32,
            cb = cv.to_bits(),
            rb = rv.to_bits(),
        ))
    }
}

/// Assert one input pair matches.
#[track_caller]
pub fn assert_same(y: f32, exp_q2: i32) {
    if let Err(msg) = check(y, exp_q2) {
        panic!("{msg}");
    }
}

/// Assert a whole batch matches, reporting up to 10 divergences at once.
#[track_caller]
pub fn assert_batch(label: &str, cases: impl IntoIterator<Item = (f32, i32)>) {
    let mut fails: Vec<String> = Vec::new();
    let mut n = 0usize;
    for (y, e) in cases {
        n += 1;
        if let Err(msg) = check(y, e) {
            if fails.len() < 10 {
                fails.push(msg);
            }
        }
    }
    assert!(n > 0, "[{label}] generated zero cases — the test is vacuous");
    if !fails.is_empty() {
        panic!(
            "[{label}] {} of {n} cases diverged; first {}:\n{}",
            fails.len(),
            fails.len(),
            fails.join("\n")
        );
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed, no external dependency.
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

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    /// Uniform in `lo..=hi` (inclusive), works across the whole i32 range.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64) as u64 + 1;
        let v = self.next_u64() % span;
        (lo as i64 + v as i64) as i32
    }

    /// A random `f32` from arbitrary bits (can be NaN / inf / subnormal).
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }

    /// A random *finite normal* `f32` with a wide exponent range.
    pub fn normal_f32(&mut self) -> f32 {
        loop {
            let bits = self.next_u32();
            let v = f32::from_bits(bits);
            if v.is_normal() {
                return v;
            }
        }
    }

    /// A random `f32` in a modest magnitude band, so that scaling rarely
    /// saturates and the mantissa rounding is what is actually compared.
    pub fn moderate_f32(&mut self) -> f32 {
        // exponent field in [96, 158] -> roughly 2^-31 .. 2^31
        let mant = self.next_u32() & 0x007F_FFFF;
        let exp = 96 + (self.next_u32() % 63);
        let sign = (self.next_u32() & 1) << 31;
        f32::from_bits(sign | (exp << 23) | mant)
    }
}

/// Interesting `f32` values that every row should also be probed with.
pub const SPECIAL_F32_BITS: &[u32] = &[
    0x0000_0000, // +0.0
    0x8000_0000, // -0.0
    0x0000_0001, // smallest positive subnormal
    0x8000_0001, // smallest negative subnormal
    0x007F_FFFF, // largest positive subnormal
    0x807F_FFFF, // largest negative subnormal
    0x0080_0000, // FLT_MIN (smallest positive normal)
    0x8080_0000, // -FLT_MIN
    0x3F80_0000, // 1.0
    0xBF80_0000, // -1.0
    0x4000_0000, // 2.0
    0x3F00_0000, // 0.5
    0x7F7F_FFFF, // FLT_MAX
    0xFF7F_FFFF, // -FLT_MAX
    0x7F80_0000, // +inf
    0xFF80_0000, // -inf
    0x7FC0_0000, // default quiet NaN
    0xFFC0_0000, // negative quiet NaN
    0x7FC0_1234, // quiet NaN with payload
    0x7F80_0001, // signalling NaN
    0xFF80_0001, // negative signalling NaN
    0x7FBF_FFFF, // largest signalling NaN
    0x4B00_0000, // 2^23
    0xCB00_0000, // -2^23
    0x0000_0002,
    0x1234_5678,
    0x89AB_CDEF,
];

pub fn specials() -> impl Iterator<Item = f32> {
    SPECIAL_F32_BITS.iter().copied().map(f32::from_bits)
}

/// Interesting `exp_q2` values, mechanically derived from the C branches.
pub const SPECIAL_EXPS: &[i32] = &[
    i32::MIN,
    i32::MIN + 1,
    i32::MIN + 2,
    i32::MIN + 3,
    i32::MIN + 4,
    -2_000_000_000,
    -1_000_000,
    -100_001,
    -1024,
    -512,
    -384,
    -256,
    -129,
    -128,
    -127,
    -126,
    -125,
    -124,
    -121,
    -120,
    -119,
    -64,
    -33,
    -32,
    -31,
    -8,
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
    7,
    8,
    31,
    32,
    33,
    63,
    64,
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
    480,
    1_000,
    10_000,
    100_000,
    1_000_000,
];

/// Exponents whose loop trip count is in the millions (`⌈exp_q2 / 120⌉`
/// iterations). Kept out of `SPECIAL_EXPS` so the broad cross-product tests stay
/// fast; exercised explicitly by the `INT_MAX` row with a small `y` set.
pub const HUGE_EXPS: &[i32] = &[2_000_000_000, i32::MAX - 1, i32::MAX];
