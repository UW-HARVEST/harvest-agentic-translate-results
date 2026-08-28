//! Shared differential-test harness.
//!
//! Loads BOTH shared objects via `libloading` and calls `rgb_to_hsv` through
//! the dynamic-symbol boundary. The Rust implementation is *never* called
//! directly — always through `librgb_to_hsv_lib.so`'s exported symbol, so the
//! `#[no_mangle] extern "C"` wrapper is under test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_float;
use std::path::PathBuf;

/// ABI of the single exported entry point (`c_src/include/lib.h`):
/// `void rgb_to_hsv(float *dest, const float *src);`
pub type RgbToHsvFn = unsafe extern "C" fn(*mut c_float, *const c_float);

pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    // `Library` must outlive the fn pointer; keep it alive in the struct.
    _lib: Library,
    pub f: RgbToHsvFn,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Locate the C shared object built by `c_src/CMakeLists.txt`.
///
/// The CMake project name is derived from the *parent directory name*, so the
/// file name is not fixed; discover it instead of hard-coding.
fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_C_SO") {
        return PathBuf::from(p);
    }
    let build_dir = manifest_dir().join("../c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build_dir) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name.starts_with("lib") && name.ends_with(".so") {
                found.push(p);
            }
        }
    }
    found.sort();
    match found.len() {
        0 => panic!(
            "no C .so found in {}\nBuild it first:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build_dir.display()
        ),
        _ => found.remove(0),
    }
}

/// Locate the Rust cdylib.
///
/// `target/debug` is preferred because that is what `cargo test --features …`
/// rebuilds, so the loaded `.so` matches the feature set under test. The
/// release artifact is the fallback.
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_RUST_SO") {
        return PathBuf::from(p);
    }
    let base = manifest_dir().join("target");
    for profile in ["debug", "release"] {
        let p = base.join(profile).join("librgb_to_hsv_lib.so");
        if p.is_file() {
            return p;
        }
    }
    panic!(
        "librgb_to_hsv_lib.so not found under {}\nBuild it first: cargo build",
        base.display()
    );
}

/// Locate a Rust cdylib built WITHOUT `debug_assertions`, i.e. without
/// rustc's UB-checking instrumentation.
///
/// This matters only for the null-pointer rows: a `debug_assertions` build
/// inserts a "null pointer dereference occurred" check that converts the
/// undefined behaviour into a *defined* abort. The C is compiled with no
/// equivalent instrumentation (`-fsanitize=null` is not enabled), so a
/// like-for-like UB comparison must use the uninstrumented artifact.
pub fn rust_so_nochecks() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_RUST_SO_NOCHECKS") {
        return PathBuf::from(p);
    }
    let p = manifest_dir()
        .join("target/release")
        .join("librgb_to_hsv_lib.so");
    assert!(
        p.is_file(),
        "uninstrumented Rust cdylib not found at {}\nBuild it first: cargo build --release",
        p.display()
    );
    p
}

/// Path of the C `.so` (public so child processes can be pointed at it).
pub fn c_so() -> PathBuf {
    c_so_path()
}

/// Path of the Rust `.so` resolved by the default rules.
pub fn rust_so() -> PathBuf {
    rust_so_path()
}

fn load(name: &'static str, path: PathBuf) -> Impl {
    unsafe {
        let lib = Library::new(&path)
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
        let sym: Symbol<RgbToHsvFn> = lib.get(b"rgb_to_hsv\0").unwrap_or_else(|e| {
            panic!("dlsym(rgb_to_hsv) missing from {}: {e}", path.display())
        });
        let f = *sym;
        Impl {
            name,
            path,
            _lib: lib,
            f,
        }
    }
}

/// Load the C `.so` and the Rust `.so`. Returns `(c, rust)`.
pub fn both() -> (Impl, Impl) {
    (load("C", c_so_path()), load("Rust", rust_so_path()))
}

/// Load one specific shared object by path (used by the null-pointer child).
pub fn load_one(name: &'static str, path: PathBuf) -> Impl {
    load(name, path)
}

// ---------------------------------------------------------------------------
// Invocation helpers
// ---------------------------------------------------------------------------

/// Canary written into `dest` before every call. If the implementation fails
/// to store a lane, the canary survives and the comparison notices.
pub const CANARY: u32 = 0xDEAD_BEEF;

/// Call `f(dest, src)` with a fresh canary-filled 3-float `dest`.
/// Returns the raw bits of `dest[0..3]`.
pub fn call_bits(f: RgbToHsvFn, src: &[f32; 3]) -> [u32; 3] {
    let mut dest = [f32::from_bits(CANARY); 3];
    unsafe { f(dest.as_mut_ptr(), src.as_ptr()) };
    [dest[0].to_bits(), dest[1].to_bits(), dest[2].to_bits()]
}

/// Same as [`call_bits`] but the input is given as raw bit patterns, so
/// arbitrary NaN payloads / signalling NaNs survive intact.
pub fn call_bits_raw(f: RgbToHsvFn, src_bits: &[u32; 3]) -> [u32; 3] {
    let src = [
        f32::from_bits(src_bits[0]),
        f32::from_bits(src_bits[1]),
        f32::from_bits(src_bits[2]),
    ];
    call_bits(f, &src)
}

// ---------------------------------------------------------------------------
// Differential assertions
// ---------------------------------------------------------------------------

fn fmt(bits: &[u32; 3]) -> String {
    format!(
        "[{:#010x} ({:e}), {:#010x} ({:e}), {:#010x} ({:e})]",
        bits[0],
        f32::from_bits(bits[0]),
        bits[1],
        f32::from_bits(bits[1]),
        bits[2],
        f32::from_bits(bits[2])
    )
}

fn fmt_in(bits: &[u32; 3]) -> String {
    fmt(bits)
}

/// Accumulates divergences so a whole randomized row reports every failing
/// vector at once instead of aborting on the first.
pub struct Diff {
    pub row: &'static str,
    pub checked: usize,
    failures: Vec<String>,
}

impl Diff {
    pub fn new(row: &'static str) -> Self {
        Diff {
            row,
            checked: 0,
            failures: Vec::new(),
        }
    }

    /// Run one differential comparison on raw input bits.
    pub fn check_raw(&mut self, c: &Impl, rust: &Impl, src_bits: [u32; 3]) {
        self.checked += 1;
        let got_c = call_bits_raw(c.f, &src_bits);
        let got_r = call_bits_raw(rust.f, &src_bits);
        if got_c != got_r {
            if self.failures.len() < 20 {
                self.failures.push(format!(
                    "  src   = {}\n    C    = {}\n    Rust = {}",
                    fmt_in(&src_bits),
                    fmt(&got_c),
                    fmt(&got_r)
                ));
            }
        }
    }

    pub fn check(&mut self, c: &Impl, rust: &Impl, src: [f32; 3]) {
        self.check_raw(c, rust, [src[0].to_bits(), src[1].to_bits(), src[2].to_bits()]);
    }

    /// Record an already-computed pair of outputs (for aliasing rows that need
    /// bespoke buffer setup).
    pub fn check_outputs(&mut self, label: String, got_c: &[u32], got_r: &[u32]) {
        self.checked += 1;
        if got_c != got_r {
            if self.failures.len() < 20 {
                self.failures.push(format!(
                    "  {label}\n    C    = {got_c:#010x?}\n    Rust = {got_r:#010x?}"
                ));
            }
        }
    }

    pub fn n_failures(&self) -> usize {
        self.failures.len()
    }

    /// Assert the row passed. Panics with every recorded divergence.
    pub fn finish(self) {
        assert!(
            self.checked > 0,
            "row {} ran 0 comparisons — the test is vacuous",
            self.row
        );
        if !self.failures.is_empty() {
            panic!(
                "\nCONFIGS/ERRORS row {}: {} of {} vectors DIVERGED between C and Rust\n{}\n",
                self.row,
                self.failures.len(),
                self.checked,
                self.failures.join("\n")
            );
        }
        eprintln!(
            "row {:<44} OK  ({} vectors, bit-exact)",
            self.row, self.checked
        );
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (PCG32) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_C0DE;

pub struct Pcg32 {
    state: u64,
    inc: u64,
}

impl Pcg32 {
    pub fn new(seed: u64) -> Self {
        let mut r = Pcg32 {
            state: 0,
            inc: (seed << 1) | 1,
        };
        r.next_u32();
        r.state = r.state.wrapping_add(seed);
        r.next_u32();
        r
    }

    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// Uniform in `[0, 1)` with 24 bits of entropy.
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Uniform in `[lo, hi)`.
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.unit() * (hi - lo)
    }

    /// Uniform integer in `[0, n)`.
    pub fn below(&mut self, n: u32) -> u32 {
        // Modulo bias is irrelevant for test-vector generation.
        self.next_u32() % n
    }

    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u32) as usize]
    }

    /// Three ascending distinct values in `[0, 1]`, returned as `(lo, mid, hi)`.
    pub fn three_sorted_unit(&mut self) -> (f32, f32, f32) {
        loop {
            let mut v = [self.unit(), self.unit(), self.unit()];
            v.sort_by(|a, b| a.partial_cmp(b).unwrap());
            if v[0] < v[1] && v[1] < v[2] {
                return (v[0], v[1], v[2]);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Curated special values (every IEEE-754 binary32 class the C can meet)
// ---------------------------------------------------------------------------

/// 24 values spanning: signed zeros, subnormals, normals in and out of the
/// documented `[0, 1]` range, negatives, extremes, infinities and NaNs
/// (quiet, signalling, custom payload, negative-sign).
pub const SPECIALS: [u32; 24] = [
    0x0000_0000, // +0.0
    0x8000_0000, // -0.0
    0x0000_0001, // +FLT_TRUE_MIN (smallest subnormal)
    0x8000_0001, // -FLT_TRUE_MIN
    0x007F_FFFF, // largest subnormal
    0x0080_0000, // +FLT_MIN (smallest normal)
    0x3400_0000, // 2^-23
    0x3727_C5AC, // 1e-5
    0x3DCC_CCCD, // 0.1
    0x3F00_0000, // 0.5
    0x3F7F_FFFF, // largest float < 1.0
    0x3F80_0000, // 1.0
    0x3F80_0001, // 1.0 + 1ulp  (one step past documented range)
    0xBF80_0000, // -1.0
    0x4000_0000, // 2.0
    0x437F_0000, // 255.0
    0x7F7F_FFFF, // +FLT_MAX
    0xFF7F_FFFF, // -FLT_MAX
    0x7F80_0000, // +Inf
    0xFF80_0000, // -Inf
    0x7FC0_0000, // quiet NaN (default)
    0xFFC0_0000, // negative quiet NaN
    0x7F80_0001, // signalling NaN
    0x7FD5_5555, // quiet NaN, custom payload
];
