//! Shared differential-test harness.
//!
//! Both libraries are loaded through `libloading` and every call crosses the
//! FFI boundary via `dlsym`, so the `#[no_mangle]`/`extern "C"` export wrappers
//! are exercised exactly as an external C consumer would exercise them.  No Rust
//! function is ever called directly.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// C struct btac1c_idxstate_s  (mirrored for test setup only)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct IdxState {
    pub idx: u16,
    pub lpred: i16,
    pub rpred: i16,
    pub tag: u8,
    pub bcfcn: u8,
    pub bsfcn: u8,
    pub usefx: u8,
    pub firfx: [[i16; 8]; 4],
}

impl IdxState {
    pub fn zeroed() -> Self {
        IdxState {
            idx: 0,
            lpred: 0,
            rpred: 0,
            tag: 0,
            bcfcn: 0,
            bsfcn: 0,
            usefx: 0,
            firfx: [[0i16; 8]; 4],
        }
    }
}

pub type GetPredictFunc = unsafe extern "C" fn(i32) -> i32;
pub type DiffPredict =
    unsafe extern "C" fn(i32, *mut i32, i32, i32, *mut IdxState) -> i32;
pub type DiffLayout = unsafe extern "C" fn(i32) -> i32;
pub type DiffSelector = unsafe extern "C" fn(i32) -> i32;
pub type DiffCallSelected =
    unsafe extern "C" fn(*mut i32, i32, i32, *mut IdxState) -> i32;

// ---------------------------------------------------------------------------
// Locating / building the two shared objects
// ---------------------------------------------------------------------------

/// Directory of the crate (`translation/`).
fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The working directory holding both `c_src/` and `translation/`.
fn root_dir() -> PathBuf {
    manifest_dir().parent().expect("crate has a parent dir").to_path_buf()
}

/// The cargo target profile directory that produced *this* test binary, so the
/// `.so` we dlopen is guaranteed to be from the same profile *and* the same
/// feature set as the test itself (`target/debug` or `target/release`).
fn profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test-bin>
    exe.parent()
        .and_then(|p| p.parent())
        .expect("test binary lives in target/<profile>/deps")
        .to_path_buf()
}

fn run(cmd: &mut Command) -> String {
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {:?}: {e}", cmd));
    if !out.status.success() {
        panic!(
            "command {:?} failed ({}):\n--- stdout ---\n{}\n--- stderr ---\n{}",
            cmd,
            out.status,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn c_compiler() -> String {
    std::env::var("CC").unwrap_or_else(|_| "cc".to_string())
}

/// Build (once) the CMake shared library from `c_src/` and return its path.
/// `c_src/` sources are never modified; only the generated `build/` tree is
/// touched, exactly as the task's build instructions prescribe.
fn build_c_cmake_lib() -> PathBuf {
    let c_src = root_dir().join("c_src");
    let build = c_src.join("build");

    let find_so = || -> Option<PathBuf> {
        let rd = std::fs::read_dir(&build).ok()?;
        rd.filter_map(|e| e.ok())
            .map(|e| e.path())
            .find(|p| p.extension().map(|x| x == "so").unwrap_or(false))
    };

    if let Some(p) = find_so() {
        return p;
    }

    std::fs::create_dir_all(&build).expect("mkdir c_src/build");
    run(Command::new("cmake")
        .current_dir(&build)
        .arg("..")
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON"));
    run(Command::new("cmake").current_dir(&build).arg("--build").arg("."));

    find_so().expect("cmake produced no .so in c_src/build")
}

/// Compile the test-only C shim (which textually `#include`s the untouched
/// `c_src/src/lib.c`) into a shared object that additionally exports
/// `__difftest_predict` / `__difftest_layout`.
fn build_c_shim_lib() -> PathBuf {
    let out_dir = profile_dir().join("difftest_c");
    std::fs::create_dir_all(&out_dir).expect("mkdir difftest_c");
    let so = out_dir.join("libcshim.so");
    let shim = manifest_dir().join("difftest_c").join("shim.c");
    let include = root_dir().join("c_src").join("include");

    let stale = match (std::fs::metadata(&so), std::fs::metadata(&shim)) {
        (Ok(a), Ok(b)) => match (a.modified(), b.modified()) {
            (Ok(a), Ok(b)) => a < b,
            _ => true,
        },
        _ => true,
    };
    if !so.exists() || stale {
        run(Command::new(c_compiler())
            .arg("-shared")
            .arg("-fPIC")
            // No -O flag: matches the CMake build, which sets no build type and
            // therefore compiles the ground-truth library unoptimised.
            .arg("-I")
            .arg(&include)
            .arg("-o")
            .arg(&so)
            .arg(&shim));
    }
    so
}

/// The feature set this test binary was compiled with, forwarded verbatim to the
/// fallback `cargo build` so the `.so` under test always matches the test's own
/// configuration.
fn active_features() -> Vec<&'static str> {
    let mut v = Vec::new();
    if cfg!(feature = "difftest") {
        v.push("difftest");
    }
    v
}

pub fn difftest_feature_enabled() -> bool {
    cfg!(feature = "difftest")
}

const RUST_SO_NAME: &str = "libget_predict_func_lib.so";

/// Build and locate the Rust cdylib for the current profile + feature set.
///
/// `cargo test` does **not** link integration tests against a `cdylib`-only
/// crate, so it does not necessarily (re)build the `.so` at all.  Any `.so`
/// sitting in `target/<profile>/` may therefore be *stale* — left over from an
/// earlier `cargo build` with different sources or different features.  Loading
/// it would silently verify the wrong artifact, so we never do that.
///
/// Instead we always run `cargo build --lib` ourselves, into a private target
/// directory (which also avoids lock contention with the `cargo test` invocation
/// that is currently running us), with the same profile and the same features as
/// this test binary.  Cargo's own freshness tracking then guarantees the `.so` we
/// dlopen was built from the current `src/lib.rs`.
fn rust_so_path() -> PathBuf {
    let profile = profile_dir();

    let is_release = profile
        .file_name()
        .map(|n| n == "release")
        .unwrap_or(false);
    let priv_target = profile.join("difftest_rs_target");
    std::fs::create_dir_all(&priv_target).expect("mkdir difftest_rs_target");

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = Command::new(cargo);
    cmd.current_dir(manifest_dir())
        .env("CARGO_TARGET_DIR", &priv_target)
        .arg("build")
        .arg("--offline")
        .arg("--lib")
        .arg("--no-default-features");
    if is_release {
        cmd.arg("--release");
    }
    let feats = active_features();
    if !feats.is_empty() {
        cmd.arg("--features").arg(feats.join(","));
    }
    run(&mut cmd);

    let built = priv_target
        .join(if is_release { "release" } else { "debug" })
        .join(RUST_SO_NAME);
    assert!(
        built.exists(),
        "Rust cdylib not found at {} after `cargo build --lib`",
        built.display()
    );

    // Freshness guard: the .so must be at least as new as every source file, so
    // a stale artifact can never be verified by mistake.
    let so_mtime = std::fs::metadata(&built)
        .and_then(|m| m.modified())
        .expect("cdylib mtime");
    for src in ["src/lib.rs", "Cargo.toml"] {
        let p = manifest_dir().join(src);
        if let Ok(m) = std::fs::metadata(&p).and_then(|m| m.modified()) {
            assert!(
                so_mtime >= m,
                "STALE Rust cdylib: {} is older than {} — the build did not pick \
                 up the current sources",
                built.display(),
                p.display()
            );
        }
    }
    built
}

fn load(path: &Path) -> Library {
    unsafe { Library::new(path) }
        .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()))
}

// ---------------------------------------------------------------------------
// The loaded pair
// ---------------------------------------------------------------------------

pub struct Libs {
    pub c: Library,
    pub c_shim: Library,
    pub rust: Library,
    pub c_so: PathBuf,
    pub rust_so: PathBuf,
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_so = build_c_cmake_lib();
        let shim_so = build_c_shim_lib();
        let rust_so = rust_so_path();
        Libs {
            c: load(&c_so),
            c_shim: load(&shim_so),
            rust: load(&rust_so),
            c_so,
            rust_so,
        }
    })
}

fn sym<T: Copy>(lib: &'static Library, name: &[u8]) -> T {
    let s: Symbol<T> = unsafe { lib.get(name) }.unwrap_or_else(|e| {
        panic!("dlsym {} failed: {e}", String::from_utf8_lossy(name))
    });
    *s
}

/// `get_predict_func` from the **C** `.so` (the CMake ground-truth artifact).
pub fn c_get_predict_func() -> GetPredictFunc {
    sym(&libs().c, b"get_predict_func\0")
}

/// `get_predict_func` from the **Rust** `.so`, via its `#[no_mangle]` export.
pub fn rust_get_predict_func() -> GetPredictFunc {
    sym(&libs().rust, b"get_predict_func\0")
}

/// `get_predict_func` from the C shim `.so` (same untouched `lib.c`, different
/// compile) — used to confirm the public result is compile-invariant.
pub fn c_shim_get_predict_func() -> GetPredictFunc {
    sym(&libs().c_shim, b"get_predict_func\0")
}

pub fn c_difftest_predict() -> DiffPredict {
    sym(&libs().c_shim, b"__difftest_predict\0")
}

pub fn rust_difftest_predict() -> DiffPredict {
    sym(&libs().rust, b"__difftest_predict\0")
}

pub fn c_difftest_selector() -> DiffSelector {
    sym(&libs().c_shim, b"__difftest_selector\0")
}

pub fn rust_difftest_selector() -> DiffSelector {
    sym(&libs().rust, b"__difftest_selector\0")
}

pub fn c_difftest_call_selected() -> DiffCallSelected {
    sym(&libs().c_shim, b"__difftest_call_selected\0")
}

pub fn rust_difftest_call_selected() -> DiffCallSelected {
    sym(&libs().rust, b"__difftest_call_selected\0")
}

/// Differentially compare `BTAC1C2_GetPredictFunc`'s *choice* of predictor.
pub fn assert_selector_eq(pfcn: i32, ctx: &str) -> i32 {
    let c = c_difftest_selector();
    let r = rust_difftest_selector();
    let cv = unsafe { c(pfcn) };
    let rv = unsafe { r(pfcn) };
    assert_eq!(
        cv, rv,
        "[{ctx}] BTAC1C2_GetPredictFunc({pfcn}) selected #{cv} in C but #{rv} in Rust"
    );
    cv
}

/// Differentially compare the composed selector-then-predict pipeline.
pub fn assert_call_selected_eq(
    psamp: &[i32; 8],
    idx: i32,
    pfcn: i32,
    st: &IdxState,
    ctx: &str,
) -> i32 {
    let c = c_difftest_call_selected();
    let r = rust_difftest_call_selected();
    let mut cs = *psamp;
    let mut cst = *st;
    let cv = unsafe { c(cs.as_mut_ptr(), idx, pfcn, &mut cst) };
    let mut rs = *psamp;
    let mut rst = *st;
    let rv = unsafe { r(rs.as_mut_ptr(), idx, pfcn, &mut rst) };
    assert_eq!(
        cv, rv,
        "[{ctx}] call_selected(idx={idx}, pfcn={pfcn}, psamp={psamp:?}): \
         C returned {cv}, Rust returned {rv}"
    );
    cv
}

pub fn c_difftest_layout() -> DiffLayout {
    sym(&libs().c_shim, b"__difftest_layout\0")
}

pub fn rust_difftest_layout() -> DiffLayout {
    sym(&libs().rust, b"__difftest_layout\0")
}

// ---------------------------------------------------------------------------
// Deterministic RNG (fixed seed, reproducible; no external crates)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        // Avoid the zero fixed-point of xorshift.
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }

    pub fn next_u64(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Full-range `i32`.
    pub fn i32_any(&mut self) -> i32 {
        self.next_u32() as i32
    }

    /// Inclusive range.
    pub fn i32_in(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }

    pub fn i16_any(&mut self) -> i16 {
        self.next_u32() as i16
    }

    /// A value drawn from a distribution that deliberately over-samples the
    /// interesting magnitudes: tiny, small, medium, and saturated extremes.
    pub fn i32_shaped(&mut self) -> i32 {
        match self.next_u64() % 8 {
            0 => 0,
            1 => self.i32_in(-4, 4),
            2 => self.i32_in(-1000, 1000),
            3 => self.i32_in(-100_000, 100_000),
            4 => i32::MAX,
            5 => i32::MIN,
            6 => self.i32_in(i32::MAX - 8, i32::MAX),
            _ => self.i32_any(),
        }
    }

    pub fn i16_shaped(&mut self) -> i16 {
        match self.next_u64() % 6 {
            0 => 0,
            1 => 256,
            2 => -256,
            3 => i16::MAX,
            4 => i16::MIN,
            _ => self.i16_any(),
        }
    }
}

// ---------------------------------------------------------------------------
// Differential comparison helpers
// ---------------------------------------------------------------------------

/// Compare `get_predict_func(pfcn)` across the C and Rust `.so`s.
pub fn assert_gpf_eq(pfcn: i32, ctx: &str) {
    let c = c_get_predict_func();
    let r = rust_get_predict_func();
    let cv = unsafe { c(pfcn) };
    let rv = unsafe { r(pfcn) };
    assert_eq!(
        cv, rv,
        "[{ctx}] get_predict_func({pfcn}): C returned {cv}, Rust returned {rv}"
    );
}

/// One differential call into a lowest-level predictor.
///
/// `which` selects the entry point: `0..=11` -> `BTAC1C2_PredictSample_PfnN`,
/// anything else -> the generic `BTAC1C2_PredictSample` dispatcher.
pub fn diff_predict(
    which: i32,
    psamp: &[i32; 8],
    idx: i32,
    pfcn: i32,
    st: &IdxState,
) -> (i32, i32) {
    let c = c_difftest_predict();
    let r = rust_difftest_predict();

    // Fresh copies per side so neither implementation can observe the other's
    // (nonexistent) mutations.
    let mut cs = *psamp;
    let mut cst = *st;
    let cv = unsafe { c(which, cs.as_mut_ptr(), idx, pfcn, &mut cst) };

    let mut rs = *psamp;
    let mut rst = *st;
    let rv = unsafe { r(which, rs.as_mut_ptr(), idx, pfcn, &mut rst) };

    // Neither side may mutate its inputs.
    assert_eq!(&cs, psamp, "C mutated psamp (which={which}, pfcn={pfcn})");
    assert_eq!(&rs, psamp, "Rust mutated psamp (which={which}, pfcn={pfcn})");

    (cv, rv)
}

pub fn assert_predict_eq(
    which: i32,
    psamp: &[i32; 8],
    idx: i32,
    pfcn: i32,
    st: &IdxState,
    ctx: &str,
) -> i32 {
    let (cv, rv) = diff_predict(which, psamp, idx, pfcn, st);
    assert_eq!(
        cv, rv,
        "[{ctx}] predict(which={which}, idx={idx}, pfcn={pfcn}, psamp={psamp:?}, \
         firfx={:?}): C returned {cv}, Rust returned {rv}",
        st.firfx
    );
    cv
}

/// `which` value that routes to the generic `BTAC1C2_PredictSample`.
pub const GENERIC: i32 = -1;
