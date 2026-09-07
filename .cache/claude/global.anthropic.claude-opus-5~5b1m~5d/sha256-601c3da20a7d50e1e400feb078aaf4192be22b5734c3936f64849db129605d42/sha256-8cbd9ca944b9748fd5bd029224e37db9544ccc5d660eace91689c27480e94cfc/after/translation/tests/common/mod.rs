//! Shared differential-test harness.
//!
//! Both the C shared object and the Rust `cdylib` are loaded with `libloading`
//! and every call goes through the exported `div_euclid` symbol. The Rust code
//! is NEVER called directly, so the `#[no_mangle] extern "C"` wrapper and the
//! C ABI are exercised exactly as an external consumer would exercise them.

#![allow(dead_code)]

use std::os::raw::c_int;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub const INT_MIN: i32 = i32::MIN;
pub const INT_MAX: i32 = i32::MAX;

/// `int div_euclid(int, int)`
pub type DivFn = unsafe extern "C" fn(c_int, c_int) -> c_int;

pub struct Libs {
    // Keep the libraries alive for the whole process; the raw fn pointers below
    // point into their text segments.
    _c_lib: &'static libloading::Library,
    _rust_lib: &'static libloading::Library,
    c_div: DivFn,
    rust_div: DivFn,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Locate the C `.so` produced by `c_src/build`.
fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build_dir = workspace_root().join("c_src").join("build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build_dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                found.push(p);
            }
        }
    }
    found.sort();
    found.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no C .so found in {}; build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build_dir.display()
        )
    })
}

/// Locate the Rust `cdylib`. Prefers `release` (the profile the task builds),
/// falls back to `debug` (which `cargo test` always produces).
fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let root = workspace_root().join("translation").join("target");
    let candidates = [
        root.join("release").join("libdiv_euclid_lib.so"),
        root.join("debug").join("libdiv_euclid_lib.so"),
    ];
    for c in candidates.iter() {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "no Rust cdylib found; run `cargo build --release` in translation/. Looked at: {:?}",
        candidates
    );
}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c_path = find_c_so();
        let rust_path = find_rust_so();

        // Leak the Library handles: they must outlive every raw fn pointer, and
        // the process keeps them for its whole lifetime anyway.
        let c_lib: &'static libloading::Library = Box::leak(Box::new(unsafe {
            libloading::Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", c_path.display()))
        }));
        let rust_lib: &'static libloading::Library = Box::leak(Box::new(unsafe {
            libloading::Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", rust_path.display()))
        }));

        let c_div: DivFn = unsafe {
            *c_lib
                .get::<DivFn>(b"div_euclid\0")
                .expect("C .so does not export `div_euclid`")
        };
        let rust_div: DivFn = unsafe {
            *rust_lib
                .get::<DivFn>(b"div_euclid\0")
                .expect("Rust .so does not export `div_euclid`")
        };

        Libs {
            _c_lib: c_lib,
            _rust_lib: rust_lib,
            c_div,
            rust_div,
            c_path,
            rust_path,
        }
    })
}

impl Libs {
    #[inline]
    pub fn c(&self, v1: i32, v2: i32) -> i32 {
        unsafe { (self.c_div)(v1 as c_int, v2 as c_int) as i32 }
    }
    #[inline]
    pub fn rust(&self, v1: i32, v2: i32) -> i32 {
        unsafe { (self.rust_div)(v1 as c_int, v2 as c_int) as i32 }
    }
}

pub fn lib() -> &'static Libs {
    libs()
}

/// Compare C and Rust for a single input pair. Returns `Err(message)` on
/// divergence so callers can accumulate failures instead of aborting on the
/// first one.
#[inline]
pub fn diff(v1: i32, v2: i32) -> Result<i32, String> {
    let l = lib();
    let c = l.c(v1, v2);
    let r = l.rust(v1, v2);
    if c == r {
        Ok(c)
    } else {
        Err(format!(
            "DIVERGENCE div_euclid({v1}, {v2}): C = {c} (0x{c:08x}) != Rust = {r} (0x{r:08x})"
        ))
    }
}

/// Accumulates divergences across many inputs and reports them all at once.
pub struct Checker {
    label: &'static str,
    cases: u64,
    failures: Vec<String>,
}

impl Checker {
    pub fn new(label: &'static str) -> Self {
        Checker {
            label,
            cases: 0,
            failures: Vec::new(),
        }
    }

    #[inline]
    pub fn check(&mut self, v1: i32, v2: i32) {
        self.cases += 1;
        if let Err(m) = diff(v1, v2) {
            if self.failures.len() < 25 {
                self.failures.push(m);
            }
        }
    }

    /// Like `check`, but also asserts the shared result equals `expected`
    /// (used by the error-path rows that pin down an exact C return value).
    #[inline]
    pub fn check_eq(&mut self, v1: i32, v2: i32, expected: i32) {
        self.cases += 1;
        match diff(v1, v2) {
            Ok(v) => {
                if v != expected {
                    if self.failures.len() < 25 {
                        self.failures.push(format!(
                            "AGREED-BUT-UNEXPECTED div_euclid({v1}, {v2}) = {v}, expected {expected}"
                        ));
                    }
                }
            }
            Err(m) => {
                if self.failures.len() < 25 {
                    self.failures.push(m);
                }
            }
        }
    }

    pub fn cases(&self) -> u64 {
        self.cases
    }

    pub fn finish(self) {
        assert!(self.cases > 0, "[{}] ran zero cases", self.label);
        if !self.failures.is_empty() {
            panic!(
                "[{}] {} divergence(s) out of {} cases:\n{}",
                self.label,
                self.failures.len(),
                self.cases,
                self.failures.join("\n")
            );
        }
        println!("[{}] OK - {} cases matched", self.label, self.cases);
    }
}

/// SplitMix64 — deterministic, no external crates.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Uniform in `[lo, hi]` inclusive; `lo <= hi`, computed in i64 to keep the
    /// full `i32` range representable.
    #[inline]
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        debug_assert!(lo <= hi);
        let span = (hi - lo) as u64 + 1;
        if span == 0 {
            return self.next_u64() as i64;
        }
        lo + (self.next_u64() % span) as i64
    }
    #[inline]
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        self.range(lo as i64, hi as i64) as i32
    }
    #[inline]
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    /// A boundary-biased `i32`: extremes, powers of two, and neighbourhoods of
    /// `0` / `INT_MIN` / `INT_MAX`, mixed with uniform values.
    pub fn nasty_i32(&mut self) -> i32 {
        match self.next_u64() % 8 {
            0 => INT_MIN,
            1 => INT_MAX,
            2 => {
                let sh = (self.next_u64() % 32) as u32;
                let v = 1i32.wrapping_shl(sh);
                if self.bool() {
                    v.wrapping_neg()
                } else {
                    v
                }
            }
            3 => {
                let base = match self.next_u64() % 3 {
                    0 => 0i64,
                    1 => INT_MIN as i64,
                    _ => INT_MAX as i64,
                };
                let off = self.range(-4, 4);
                (base + off) as i32
            }
            4 => self.range_i32(-8, 8),
            _ => self.next_i32(),
        }
    }
}

/// The 9-value boundary grid used by several rows.
pub const BOUNDARY_GRID: [i32; 9] = [
    INT_MIN,
    INT_MIN + 1,
    -2,
    -1,
    0,
    1,
    2,
    INT_MAX - 1,
    INT_MAX,
];

/// Number of randomized samples per Phase-B row. Override with `DIFF_SAMPLES`.
pub fn samples() -> u64 {
    std::env::var("DIFF_SAMPLES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(20_000)
}

pub const SEED: u64 = 0x5EED_1234;
