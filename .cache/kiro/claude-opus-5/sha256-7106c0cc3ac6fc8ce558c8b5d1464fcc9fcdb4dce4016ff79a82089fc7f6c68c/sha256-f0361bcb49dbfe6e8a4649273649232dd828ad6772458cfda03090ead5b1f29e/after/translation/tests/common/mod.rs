//! Shared differential-test harness.
//!
//! Loads BOTH the C `.so` and the Rust `.so` through `libloading` and exposes
//! paired function pointers. No Rust function is ever called directly — every
//! call goes through the `cdylib`'s exported symbol, exactly as an external
//! consumer would, so the `#[no_mangle]` / `extern "C"` wrappers are under test
//! too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub struct Libs {
    pub c: Library,
    pub r: Library,
    pub c_path: PathBuf,
    pub r_path: PathBuf,
}

fn first_so(dir: &Path) -> Option<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    found.into_iter().next()
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    let root = manifest_dir().parent().unwrap().to_path_buf();
    let build = root.join("c_src").join("build");
    first_so(&build).unwrap_or_else(|| {
        panic!(
            "no C .so found in {}; build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn rust_so_path() -> PathBuf {
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir().join("target"));
    for profile in ["release", "debug"] {
        let p = target.join(profile).join("libfallcalc_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libfallcalc_lib.so not found under {}; build it with `cargo build --release`",
        target.display()
    );
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let r_path = rust_so_path();
        // RTLD_NOW is what libloading uses by default on Linux, so a missing
        // undefined symbol in either library shows up right here.
        let c = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", c_path.display()));
        let r = unsafe { Library::new(&r_path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", r_path.display()));
        Libs { c, r, c_path, r_path }
    })
}

/// Fetch the same symbol from both libraries as a raw `extern "C"` fn pointer.
macro_rules! pair {
    ($name:literal, $ty:ty) => {{
        let l = $crate::common::libs();
        unsafe {
            let cs: Symbol<$ty> = l
                .c
                .get(concat!($name, "\0").as_bytes())
                .unwrap_or_else(|e| panic!("C .so missing symbol {}: {e}", $name));
            let rs: Symbol<$ty> = l
                .r
                .get(concat!($name, "\0").as_bytes())
                .unwrap_or_else(|e| panic!("Rust .so missing symbol {}: {e}", $name));
            (*cs, *rs)
        }
    }};
}

pub type FnSafeDoubleToInt = unsafe extern "C" fn(f64) -> i32;
pub type FnProcessArrayReverse = unsafe extern "C" fn(*mut i32, i32) -> i32;
pub type FnSwitchFallthrough = unsafe extern "C" fn(i32, i32) -> i32;
pub type FnAllocateAndCompute = unsafe extern "C" fn(i32, f64) -> i32;
pub type FnForeachSum = unsafe extern "C" fn(*mut i32, i32) -> i32;
pub type FnFallcalc = unsafe extern "C" fn(i32, i32, i32, i32) -> i32;

pub fn safe_double_to_int() -> (FnSafeDoubleToInt, FnSafeDoubleToInt) {
    pair!("safe_double_to_int", FnSafeDoubleToInt)
}
pub fn process_array_reverse() -> (FnProcessArrayReverse, FnProcessArrayReverse) {
    pair!("process_array_reverse", FnProcessArrayReverse)
}
pub fn switch_fallthrough_calculator() -> (FnSwitchFallthrough, FnSwitchFallthrough) {
    pair!("switch_fallthrough_calculator", FnSwitchFallthrough)
}
pub fn allocate_and_compute() -> (FnAllocateAndCompute, FnAllocateAndCompute) {
    pair!("allocate_and_compute", FnAllocateAndCompute)
}
pub fn foreach_sum() -> (FnForeachSum, FnForeachSum) {
    pair!("foreach_sum", FnForeachSum)
}
pub fn fallcalc() -> (FnFallcalc, FnFallcalc) {
    pair!("fallcalc", FnFallcalc)
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seeds keep every run reproducible.
// ---------------------------------------------------------------------------
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
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
    /// Uniform in `[lo, hi]` (inclusive), `lo <= hi`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    pub fn next_f64_bits(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
    /// A "reasonable" finite double spread over many magnitudes, both signs.
    pub fn next_finite_f64(&mut self) -> f64 {
        let m = (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64; // [0,1)
        let exp = self.range_i32(-40, 40);
        let sign = if self.next_u64() & 1 == 0 { 1.0 } else { -1.0 };
        sign * m * 2f64.powi(exp)
    }
}

/// Collects divergences so a whole configuration row is reported at once.
#[derive(Default)]
pub struct Diff {
    pub row: &'static str,
    pub cases: usize,
    pub failures: Vec<String>,
}

impl Diff {
    pub fn new(row: &'static str) -> Self {
        Diff { row, cases: 0, failures: Vec::new() }
    }
    pub fn check(&mut self, ctx: impl std::fmt::Display, c: i32, r: i32) {
        self.cases += 1;
        if c != r {
            if self.failures.len() < 25 {
                self.failures.push(format!("  {ctx}: C={c} Rust={r}"));
            }
        }
    }
    pub fn finish(self) {
        assert!(self.cases > 0, "row {} ran zero cases", self.row);
        if !self.failures.is_empty() {
            panic!(
                "row {}: {} divergence(s) out of {} cases:\n{}",
                self.row,
                self.failures.len(),
                self.cases,
                self.failures.join("\n")
            );
        }
    }
}
