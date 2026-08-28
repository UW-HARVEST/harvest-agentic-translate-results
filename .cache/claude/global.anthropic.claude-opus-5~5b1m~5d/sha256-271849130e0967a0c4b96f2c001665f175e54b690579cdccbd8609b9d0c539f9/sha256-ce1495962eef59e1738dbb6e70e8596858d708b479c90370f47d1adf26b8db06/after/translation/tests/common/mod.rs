// Shared differential-test harness.
//
// Loads BOTH the C `.so` and the Rust `.so` with `libloading` and exposes the
// six exported symbols as raw `extern "C"` function pointers. Nothing in the
// test suite ever calls the Rust crate directly — every call crosses the FFI
// boundary through `dlsym`, exactly as an external C consumer would, so the
// `#[no_mangle]` export wrappers are themselves under test.

#![allow(dead_code)]

use std::ffi::{c_double, c_int};
use std::path::{Path, PathBuf};

pub type FnD2I = unsafe extern "C" fn(c_double) -> c_int;
pub type FnPtrCount = unsafe extern "C" fn(*mut c_int, c_int) -> c_int;
pub type FnII = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type FnID = unsafe extern "C" fn(c_int, c_double) -> c_int;
pub type FnIIII = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// The full exported surface of one implementation (C or Rust).
pub struct Api {
    pub which: &'static str,
    pub path: PathBuf,
    pub safe_double_to_int: FnD2I,
    pub process_array_reverse: FnPtrCount,
    pub switch_fallthrough_calculator: FnII,
    pub allocate_and_compute: FnID,
    pub foreach_sum: FnPtrCount,
    pub fallcalc: FnIIII,
}

impl Api {
    fn load(which: &'static str, path: &Path) -> Api {
        // Leak the Library so the resolved function pointers stay valid for the
        // whole process; the .so must never be unloaded while we hold fn ptrs.
        let lib: &'static libloading::Library = Box::leak(Box::new(unsafe {
            libloading::Library::new(path)
                .unwrap_or_else(|e| panic!("failed to dlopen {} .so at {:?}: {e}", which, path))
        }));

        macro_rules! sym {
            ($name:literal, $ty:ty) => {{
                let s: libloading::Symbol<$ty> = unsafe {
                    lib.get(concat!($name, "\0").as_bytes()).unwrap_or_else(|e| {
                        panic!("{} .so ({:?}) does not export `{}`: {e}", which, path, $name)
                    })
                };
                *s
            }};
        }

        Api {
            which,
            path: path.to_path_buf(),
            safe_double_to_int: sym!("safe_double_to_int", FnD2I),
            process_array_reverse: sym!("process_array_reverse", FnPtrCount),
            switch_fallthrough_calculator: sym!("switch_fallthrough_calculator", FnII),
            allocate_and_compute: sym!("allocate_and_compute", FnID),
            foreach_sum: sym!("foreach_sum", FnPtrCount),
            fallcalc: sym!("fallcalc", FnIIII),
        }
    }
}

/// Repository root (the directory holding `c_src/` and `translation/`).
pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Path to the C shared library. Its name is derived from the repo directory
/// name by CMake, so it is discovered by globbing rather than hardcoded.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_PATH") {
        return PathBuf::from(p);
    }
    let build_dir = repo_root().join("c_src").join("build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build_dir) {
        for e in rd.flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with("lib") && name.ends_with(".so") && p.is_file() {
                found.push(p);
            }
        }
    }
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one lib*.so in {:?}, found {:?}.\n\
         Build it with: cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build_dir,
        found
    );
    found.pop().unwrap()
}

/// Path to the Rust `cdylib`. Resolved relative to the running test binary so
/// that it automatically picks `target/debug/` vs `target/release/` (and any
/// custom `CARGO_TARGET_DIR`) to match the profile under test.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("test binary has a parent dir");
    let profile_dir = deps.parent().unwrap_or(deps);
    for dir in [profile_dir, deps] {
        let cand = dir.join("libfallcalc_lib.so");
        if cand.is_file() {
            return cand;
        }
    }
    panic!(
        "libfallcalc_lib.so not found in {:?} or {:?}. Run `cargo build` \
         (matching the test profile) first.",
        profile_dir, deps
    );
}

/// Load both implementations. Order is C first so a missing C build fails loudly.
pub fn both() -> (Api, Api) {
    let c = Api::load("C", &c_so_path());
    let r = Api::load("Rust", &rust_so_path());
    (c, r)
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seeds keep every property test
// reproducible across runs and across the two profiles.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
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

    /// Full-range random `i32` (covers `INT_MIN`..`INT_MAX` uniformly).
    pub fn i32_any(&mut self) -> i32 {
        self.next_u32() as i32
    }

    /// Uniform in `[lo, hi]` inclusive.
    pub fn i32_in(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }

    pub fn usize_in(&mut self, lo: usize, hi: usize) -> usize {
        debug_assert!(lo <= hi);
        lo + (self.next_u64() % (hi - lo + 1) as u64) as usize
    }

    /// Uniform random bit pattern reinterpreted as `f64`: yields NaNs, infinities,
    /// subnormals and huge magnitudes, i.e. the whole of axis A.
    pub fn f64_bits(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }

    /// A finite `f64` spread over many exponents (never NaN/Inf).
    pub fn f64_finite(&mut self) -> f64 {
        loop {
            let v = f64::from_bits(self.next_u64());
            if v.is_finite() {
                return v;
            }
        }
    }

    /// A "reasonable" finite double in roughly `[-1e6, 1e6]` with a fraction.
    pub fn f64_moderate(&mut self) -> f64 {
        let m = (self.next_u64() % 2_000_000_001) as f64 - 1_000_000_000.0;
        m / 1000.0
    }
}

// ---------------------------------------------------------------------------
// Differential assertions
// ---------------------------------------------------------------------------

/// Compare one `int` result from C and Rust, reporting the exact inputs.
#[track_caller]
pub fn diff_eq(row: &str, ctx: impl std::fmt::Display, c_val: c_int, r_val: c_int) {
    assert_eq!(
        c_val, r_val,
        "[{row}] DIVERGENCE with {ctx}: C returned {c_val} (0x{c_val:08x}), \
         Rust returned {r_val} (0x{r_val:08x})"
    );
}

/// `f64` formatter that also prints the raw bits, so NaN payload / -0.0
/// differences are visible in failure output.
pub struct Bits(pub f64);

impl std::fmt::Display for Bits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} (bits 0x{:016x})", self.0, self.0.to_bits())
    }
}

/// Call `process_array_reverse` on both libraries with `end` pointing at the
/// last element of `buf`, which is how `fallcalc` uses it (in-bounds walk back).
pub fn reverse_both(c: &Api, r: &Api, buf: &mut [c_int], count: c_int) -> (c_int, c_int) {
    assert!(!buf.is_empty());
    let end = unsafe { buf.as_mut_ptr().add(buf.len() - 1) };
    let cv = unsafe { (c.process_array_reverse)(end, count) };
    let rv = unsafe { (r.process_array_reverse)(end, count) };
    (cv, rv)
}

pub fn foreach_both(c: &Api, r: &Api, buf: &mut [c_int], count: c_int) -> (c_int, c_int) {
    let p = buf.as_mut_ptr();
    let cv = unsafe { (c.foreach_sum)(p, count) };
    let rv = unsafe { (r.foreach_sum)(p, count) };
    (cv, rv)
}
