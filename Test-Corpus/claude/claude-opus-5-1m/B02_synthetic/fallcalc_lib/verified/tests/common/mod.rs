// Shared differential-test harness.
//
// Both the C reference library and the Rust translation are loaded as shared
// objects through `libloading`; NO Rust function is ever called directly, so
// the `#[no_mangle] extern "C"` export wrappers are exercised too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Function-pointer signatures of the six exported C symbols
// ---------------------------------------------------------------------------

pub type FnSafeDoubleToInt = unsafe extern "C" fn(f64) -> i32;
pub type FnProcessArrayReverse = unsafe extern "C" fn(*mut i32, i32) -> i32;
pub type FnSwitchFallthrough = unsafe extern "C" fn(i32, i32) -> i32;
pub type FnAllocateAndCompute = unsafe extern "C" fn(i32, f64) -> i32;
pub type FnForeachSum = unsafe extern "C" fn(*mut i32, i32) -> i32;
pub type FnFallcalc = unsafe extern "C" fn(i32, i32, i32, i32) -> i32;

/// One loaded implementation (either the C `.so` or the Rust `.so`).
pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: Library,
    pub safe_double_to_int: FnSafeDoubleToInt,
    pub process_array_reverse: FnProcessArrayReverse,
    pub switch_fallthrough_calculator: FnSwitchFallthrough,
    pub allocate_and_compute: FnAllocateAndCompute,
    pub foreach_sum: FnForeachSum,
    pub fallcalc: FnFallcalc,
}

impl Impl {
    fn load(name: &'static str, path: PathBuf) -> Impl {
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", name, path.display()));

        macro_rules! sym {
            ($t:ty, $s:literal) => {{
                let s: Symbol<$t> = unsafe { lib.get($s.as_bytes()) }.unwrap_or_else(|e| {
                    panic!("symbol `{}` missing from {} ({}): {e}", $s, name, path.display())
                });
                *s
            }};
        }

        let safe_double_to_int = sym!(FnSafeDoubleToInt, "safe_double_to_int");
        let process_array_reverse = sym!(FnProcessArrayReverse, "process_array_reverse");
        let switch_fallthrough_calculator = sym!(FnSwitchFallthrough, "switch_fallthrough_calculator");
        let allocate_and_compute = sym!(FnAllocateAndCompute, "allocate_and_compute");
        let foreach_sum = sym!(FnForeachSum, "foreach_sum");
        let fallcalc = sym!(FnFallcalc, "fallcalc");

        Impl {
            name,
            path,
            _lib: lib,
            safe_double_to_int,
            process_array_reverse,
            switch_fallthrough_calculator,
            allocate_and_compute,
            foreach_sum,
            fallcalc,
        }
    }
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/<profile>/` — derived from the running test executable
/// (`target/<profile>/deps/<test>-<hash>`).
fn target_profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<name>-<hash>
    let deps = exe.parent().expect("deps dir");
    if deps.file_name().map(|n| n == "deps").unwrap_or(false) {
        deps.parent().expect("profile dir").to_path_buf()
    } else {
        deps.to_path_buf()
    }
}

pub fn c_so_path() -> PathBuf {
    // `FALLCALC_C_SO` lets the harness be pointed at an alternatively-built
    // reference `.so` (e.g. an -O2 build) without touching `c_src/`.
    if let Ok(p) = std::env::var("FALLCALC_C_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "FALLCALC_C_SO points at a missing file: {}", p.display());
        return p;
    }
    let p = manifest_dir().join("c_src/build/libtranslated_rust.so");
    assert!(
        p.exists(),
        "C shared library not found at {}.\nBuild it with:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// `"debug"` or `"release"`, derived from the running test executable's path.
fn current_profile() -> String {
    target_profile_dir()
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "debug".to_string())
}

/// `cargo test` does not itself emit the `cdylib` artifact (no test target depends
/// on it), so if it has not been pre-built we build it into a *separate*
/// target-dir. Using a separate dir avoids contending for the outer cargo's
/// build lock.
fn build_rust_so_fallback(profile: &str) -> PathBuf {
    let scratch = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| manifest_dir().join("target"))
        .join("cdylib-for-tests");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = std::process::Command::new(cargo);
    cmd.current_dir(manifest_dir())
        .arg("build")
        .arg("--offline")
        .arg("--no-default-features")
        .arg("--lib")
        .arg("--target-dir")
        .arg(&scratch);
    if profile == "release" {
        cmd.arg("--release");
    }
    // Do not inherit the outer cargo's env, which would redirect the target dir.
    cmd.env_remove("CARGO_TARGET_DIR");
    let out = cmd.output().expect("failed to spawn `cargo build` for the cdylib");
    let p = scratch.join(profile).join("libfallcalc_lib.so");
    assert!(
        out.status.success() && p.exists(),
        "could not build the Rust cdylib.\nstdout:\n{}\nstderr:\n{}\nexpected artifact: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
        p.display()
    );
    p
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("FALLCALC_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "FALLCALC_RUST_SO points at a missing file: {}", p.display());
        return p;
    }
    let profile = current_profile();
    let p = target_profile_dir().join("libfallcalc_lib.so");
    if p.exists() {
        return p;
    }
    build_rust_so_fallback(&profile)
}

static C_IMPL: OnceLock<Impl> = OnceLock::new();
static RUST_IMPL: OnceLock<Impl> = OnceLock::new();

pub fn c_impl() -> &'static Impl {
    C_IMPL.get_or_init(|| Impl::load("C", c_so_path()))
}

pub fn rust_impl() -> &'static Impl {
    RUST_IMPL.get_or_init(|| Impl::load("Rust", rust_so_path()))
}

/// Convenience: `(c, rust)`.
pub fn both() -> (&'static Impl, &'static Impl) {
    (c_impl(), rust_impl())
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x2545_F491_4F6C_DD1D;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { SEED } else { seed })
    }
    /// Default fixed seed.
    pub fn fixed() -> Rng {
        Rng(SEED)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// Inclusive range.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + self.below(span) as i64) as i32
    }
    /// Raw 64-bit pattern reinterpreted as `f64` (may be NaN/inf/subnormal).
    pub fn next_f64_bits(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
    /// Uniform finite double in `[lo, hi]`.
    pub fn f64_in(&mut self, lo: f64, hi: f64) -> f64 {
        let u = (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64; // [0,1)
        lo + u * (hi - lo)
    }
    /// A "spicy" i32: boundary constants most of the time, random otherwise.
    pub fn spicy_i32(&mut self) -> i32 {
        const POOL: [i32; 31] = [
            0, 1, -1, 2, -2, 3, -3, 4, -4, 5, -5, 9, 10, -10, 11, -11, 126, 127, 128, 129, 130,
            0o100, -0o100, 0o200, -0o200, 0o777, -0o777, i32::MAX, i32::MIN, i32::MAX - 1,
            i32::MIN + 1,
        ];
        if self.below(4) == 0 {
            self.next_i32()
        } else {
            POOL[self.below(POOL.len() as u64) as usize]
        }
    }
}

// ---------------------------------------------------------------------------
// Assertion helpers with rich diagnostics
// ---------------------------------------------------------------------------

#[track_caller]
pub fn eq_i32(row: &str, ctx: impl std::fmt::Debug, c: i32, r: i32) {
    assert_eq!(
        c, r,
        "[{row}] C/Rust divergence for input {ctx:?}: C returned {c} (0x{c:08x}), \
         Rust returned {r} (0x{r:08x})"
    );
}

// ---------------------------------------------------------------------------
// Caller-owned buffers (allocated by the test, shared with both libraries)
// ---------------------------------------------------------------------------

/// Allocate a `Vec<i32>` and hand out its raw pointer. The buffer belongs to the
/// test, so both libraries read exactly the same bytes.
pub struct Buf(pub Vec<i32>);

impl Buf {
    pub fn new(v: Vec<i32>) -> Buf {
        Buf(v)
    }
    pub fn random(rng: &mut Rng, n: usize) -> Buf {
        Buf((0..n).map(|_| rng.next_i32()).collect())
    }
    pub fn ptr(&mut self) -> *mut i32 {
        self.0.as_mut_ptr()
    }
    /// Pointer to element `i` (used for the reverse walk).
    pub fn ptr_at(&mut self, i: usize) -> *mut i32 {
        unsafe { self.0.as_mut_ptr().add(i) }
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
}
