//! Shared differential-test harness.
//!
//! BOTH libraries are loaded as shared objects through `libloading`; the Rust
//! implementation is *never* called directly, so the `#[no_mangle]` /
//! `extern "C"` export wrappers are exercised exactly as an external C caller
//! would exercise them.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_double, c_int};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Function signatures exported by both `.so`s
// ---------------------------------------------------------------------------
type FnSafeDoubleToInt = unsafe extern "C" fn(c_double) -> c_int;
type FnProcessArrayReverse = unsafe extern "C" fn(*mut c_int, c_int) -> c_int;
type FnSwitchFallthrough = unsafe extern "C" fn(c_int, c_int) -> c_int;
type FnAllocateAndCompute = unsafe extern "C" fn(c_int, c_double) -> c_int;
type FnForeachSum = unsafe extern "C" fn(*mut c_int, c_int) -> c_int;
type FnFallcalc = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// One loaded implementation (either the C `.so` or the Rust `.so`).
pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: Library,
    safe_double_to_int: FnSafeDoubleToInt,
    process_array_reverse: FnProcessArrayReverse,
    switch_fallthrough_calculator: FnSwitchFallthrough,
    allocate_and_compute: FnAllocateAndCompute,
    foreach_sum: FnForeachSum,
    fallcalc: FnFallcalc,
}

impl Impl {
    fn load(name: &'static str, path: &Path) -> Impl {
        // `Library::new` uses RTLD_NOW|RTLD_LOCAL on ELF platforms, so any
        // unresolved (non-lazy) symbol in the object makes this fail. That is
        // itself part of the SYMBOLS.md check.
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", name, path.display()));

        macro_rules! sym {
            ($t:ty, $n:literal) => {{
                let s: Symbol<$t> = unsafe { lib.get($n) }.unwrap_or_else(|e| {
                    panic!(
                        "{} is missing symbol `{}`: {e}",
                        name,
                        String::from_utf8_lossy($n)
                    )
                });
                // Safe: `lib` is stored alongside these pointers in the same
                // struct and never unloaded for the lifetime of the process
                // (the `Impl`s live in a `OnceLock` static).
                *s
            }};
        }

        let safe_double_to_int = sym!(FnSafeDoubleToInt, b"safe_double_to_int");
        let process_array_reverse = sym!(FnProcessArrayReverse, b"process_array_reverse");
        let switch_fallthrough_calculator =
            sym!(FnSwitchFallthrough, b"switch_fallthrough_calculator");
        let allocate_and_compute = sym!(FnAllocateAndCompute, b"allocate_and_compute");
        let foreach_sum = sym!(FnForeachSum, b"foreach_sum");
        let fallcalc = sym!(FnFallcalc, b"fallcalc");

        Impl {
            name,
            path: path.to_path_buf(),
            _lib: lib,
            safe_double_to_int,
            process_array_reverse,
            switch_fallthrough_calculator,
            allocate_and_compute,
            foreach_sum,
            fallcalc,
        }
    }

    pub fn safe_double_to_int(&self, d: f64) -> i32 {
        unsafe { (self.safe_double_to_int)(d) }
    }
    pub fn process_array_reverse(&self, end: *mut i32, count: i32) -> i32 {
        unsafe { (self.process_array_reverse)(end, count) }
    }
    pub fn switch_fallthrough_calculator(&self, value: i32, operation: i32) -> i32 {
        unsafe { (self.switch_fallthrough_calculator)(value, operation) }
    }
    pub fn allocate_and_compute(&self, size: i32, multiplier: f64) -> i32 {
        unsafe { (self.allocate_and_compute)(size, multiplier) }
    }
    pub fn foreach_sum(&self, array: *mut i32, count: i32) -> i32 {
        unsafe { (self.foreach_sum)(array, count) }
    }
    pub fn fallcalc(&self, p1: i32, p2: i32, p3: i32, p4: i32) -> i32 {
        unsafe { (self.fallcalc)(p1, p2, p3, p4) }
    }
}

pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

static PAIR: OnceLock<Pair> = OnceLock::new();

/// Crate root (`translation/`).
pub fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Working directory that holds both `c_src/` and `translation/`.
pub fn work_root() -> PathBuf {
    crate_root().parent().expect("crate has a parent dir").to_path_buf()
}

fn mtime(p: &Path) -> std::time::SystemTime {
    std::fs::metadata(p)
        .and_then(|m| m.modified())
        .unwrap_or_else(|e| panic!("cannot stat {}: {e}", p.display()))
}

/// Refuse to test an artifact that is older than the source it was built from.
/// Without this guard a stale `.so` silently makes every differential test pass.
fn assert_fresh(label: &str, artifact: &Path, source: &Path) {
    let a = mtime(artifact);
    let s = mtime(source);
    assert!(
        a >= s,
        "STALE {label} ARTIFACT: {} is older than {}.\n\
         The differential tests would be comparing against an out-of-date build.\n\
         Rebuild it and re-run.",
        artifact.display(),
        source.display()
    );
}

/// Locate the C shared object. The CMake project name is derived from the
/// parent directory name, so the file name is not fixed -- glob for it.
fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("FALLCALC_C_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "FALLCALC_C_SO={} is not a file", p.display());
        return p;
    }
    let build = work_root().join("c_src/build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {} ({e}). Build the C library first:\n  \
                 cd c_src && mkdir -p build && cd build && \
                 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build.display()
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("so"))
        .collect();
    candidates.sort();
    assert_eq!(
        candidates.len(),
        1,
        "expected exactly one .so in {}, found {:?}",
        build.display(),
        candidates
    );
    candidates.pop().unwrap()
}

/// Locate the Rust cdylib.
///
/// `cargo test` does **not** build a `cdylib`-only lib target (integration tests
/// do not link against it), so there is no artifact to find unless we make one.
/// Rather than fall back to whatever `.so` happens to be lying around in
/// `target/` -- which silently tests a stale build and makes every differential
/// assertion vacuous -- this builds the cdylib on demand into a *separate*
/// target directory (so it cannot deadlock on the outer cargo's build lock) and
/// then checks the result is newer than `src/lib.rs`.
///
/// Override with `FALLCALC_RUST_SO=/path/to/libfallcalc_lib.so` to test a
/// specific artifact (used by `run_all.sh` to also cover the release build).
fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("FALLCALC_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(
            p.is_file(),
            "FALLCALC_RUST_SO={} is not a file",
            p.display()
        );
        return p;
    }

    let root = crate_root();
    let target_dir = root.join("target/ffi-so");
    let out = std::process::Command::new(std::env::var("CARGO").unwrap_or("cargo".into()))
        .arg("build")
        .arg("--lib")
        .arg("--manifest-path")
        .arg(root.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(&target_dir)
        .output()
        .expect("failed to spawn cargo to build the cdylib under test");
    assert!(
        out.status.success(),
        "cargo build --lib failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let so = target_dir.join("debug/libfallcalc_lib.so");
    assert!(
        so.is_file(),
        "cargo build --lib did not produce {}",
        so.display()
    );
    so
}

/// Both implementations, loaded once per test process.
pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| {
        let c_path = find_c_so();
        let rs_path = find_rust_so();

        assert_fresh("C", &c_path, &work_root().join("c_src/src/lib.c"));
        assert_fresh("Rust", &rs_path, &crate_root().join("src/lib.rs"));

        if std::env::var_os("FALLCALC_VERBOSE").is_some() {
            eprintln!("C   .so: {}", c_path.display());
            eprintln!("Rust.so: {}", rs_path.display());
        }

        Pair {
            c: Impl::load("C", &c_path),
            rs: Impl::load("Rust", &rs_path),
        }
    })
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) -- fixed seed per test for reproducibility.
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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `[lo, hi]` inclusive.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    /// Uniform `f64` in `[0, 1)`.
    pub fn unit_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }
    /// Uniform `f64` in `[lo, hi)`.
    pub fn range_f64(&mut self, lo: f64, hi: f64) -> f64 {
        lo + self.unit_f64() * (hi - lo)
    }
    /// Arbitrary 64-bit pattern reinterpreted as `f64` (may be NaN/inf).
    pub fn raw_f64(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
    /// A value biased toward interesting integers.
    pub fn interesting_i32(&mut self) -> i32 {
        const POOL: [i32; 22] = [
            0,
            1,
            -1,
            2,
            -2,
            5,
            -5,
            8,
            10,
            -10,
            63,
            64,
            127,
            128,
            129,
            -128,
            511,
            512,
            i32::MAX,
            i32::MIN,
            i32::MAX - 1,
            i32::MIN + 1,
        ];
        match self.next_u64() % 3 {
            0 => POOL[(self.next_u64() % POOL.len() as u64) as usize],
            1 => self.range_i32(-1000, 1000),
            _ => self.next_i32(),
        }
    }
}

// ---------------------------------------------------------------------------
// Assertion helpers -- report the exact differing inputs.
// ---------------------------------------------------------------------------

#[track_caller]
pub fn cmp(label: &str, args: impl std::fmt::Debug, c: i32, rs: i32) {
    assert_eq!(
        c, rs,
        "\nDIVERGENCE in {label}\n  args : {args:?}\n  C    : {c} (0x{c:08x})\n  Rust : {rs} (0x{rs:08x})\n"
    );
}

/// Compare `safe_double_to_int` for one input, printing the exact bit pattern
/// on failure (decimal formatting of doubles is lossy).
#[track_caller]
pub fn cmp_sdti(p: &Pair, d: f64) {
    let c = p.c.safe_double_to_int(d);
    let rs = p.rs.safe_double_to_int(d);
    assert_eq!(
        c, rs,
        "\nDIVERGENCE in safe_double_to_int\n  d    : {d:?} (bits 0x{:016x})\n  C    : {c}\n  Rust : {rs}\n",
        d.to_bits()
    );
}

#[track_caller]
pub fn cmp_switch(p: &Pair, value: i32, operation: i32) {
    let c = p.c.switch_fallthrough_calculator(value, operation);
    let rs = p.rs.switch_fallthrough_calculator(value, operation);
    cmp(
        "switch_fallthrough_calculator",
        (value, operation),
        c,
        rs,
    );
}

#[track_caller]
pub fn cmp_alloc(p: &Pair, size: i32, multiplier: f64) {
    let c = p.c.allocate_and_compute(size, multiplier);
    let rs = p.rs.allocate_and_compute(size, multiplier);
    assert_eq!(
        c, rs,
        "\nDIVERGENCE in allocate_and_compute\n  size : {size}\n  mult : {multiplier:?} (bits 0x{:016x})\n  C    : {c}\n  Rust : {rs}\n",
        multiplier.to_bits()
    );
}

#[track_caller]
pub fn cmp_fallcalc(p: &Pair, a: i32, b: i32, c_: i32, d: i32) {
    let c = p.c.fallcalc(a, b, c_, d);
    let rs = p.rs.fallcalc(a, b, c_, d);
    cmp("fallcalc", (a, b, c_, d), c, rs);
}

/// `foreach_sum` over an identical copy of `buf` for each implementation.
#[track_caller]
pub fn cmp_foreach(p: &Pair, buf: &[i32], count: i32) {
    let mut a = buf.to_vec();
    let mut b = buf.to_vec();
    let c = p.c.foreach_sum(a.as_mut_ptr(), count);
    let rs = p.rs.foreach_sum(b.as_mut_ptr(), count);
    assert_eq!(a, b, "foreach_sum mutated the buffer differently");
    cmp("foreach_sum", (buf.len(), count), c, rs);
}

/// `process_array_reverse` starting at `buf[end_idx]`, walking backwards
/// `count` elements.
#[track_caller]
pub fn cmp_reverse(p: &Pair, buf: &[i32], end_idx: usize, count: i32) {
    if count > 0 {
        assert!(
            end_idx < buf.len() && (count as usize) <= end_idx + 1,
            "test bug: backward walk of {count} from index {end_idx} leaves the buffer"
        );
    }
    let mut a = buf.to_vec();
    let mut b = buf.to_vec();
    let c = unsafe { p.c.process_array_reverse(a.as_mut_ptr().add(end_idx), count) };
    let rs = unsafe { p.rs.process_array_reverse(b.as_mut_ptr().add(end_idx), count) };
    assert_eq!(a, b, "process_array_reverse mutated the buffer differently");
    cmp("process_array_reverse", (buf.len(), end_idx, count), c, rs);
}
