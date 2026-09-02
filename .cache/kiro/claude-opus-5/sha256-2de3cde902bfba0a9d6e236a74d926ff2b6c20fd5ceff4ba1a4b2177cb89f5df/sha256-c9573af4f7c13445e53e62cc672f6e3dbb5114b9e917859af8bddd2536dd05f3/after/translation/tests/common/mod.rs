//! Shared differential-test harness.
//!
//! Loads BOTH shared libraries with `libloading` and calls their exported
//! `get_predict_func` symbols through the FFI boundary. The Rust
//! implementation is NEVER called directly — only via its `.so` export, so
//! the `#[no_mangle] extern "C"` wrapper is under test too.

// Not every test file uses every helper.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

/// Path of the Rust `.so` actually loaded — exposed for the Phase D tests.
pub fn rust_shared_object() -> PathBuf {
    rust_so_path()
}

/// Path of the C `.so` actually loaded — exposed for the Phase D tests.
pub fn c_shared_object() -> PathBuf {
    c_so_path()
}

pub type GetPredictFunc = unsafe extern "C" fn(std::ffi::c_int) -> std::ffi::c_int;

/// Workspace root = parent of the `translation/` crate directory.
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate dir must have a parent")
        .to_path_buf()
}

/// Locate (building if necessary) the C shared library.
fn c_so_path() -> PathBuf {
    let root = workspace_root();
    let build_dir = root.join("c_src").join("build");

    let find = || -> Option<PathBuf> {
        let entries = std::fs::read_dir(&build_dir).ok()?;
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                return Some(p);
            }
        }
        None
    };

    if let Some(p) = find() {
        return p;
    }

    // Not built yet -> build it. Never modifies c_src sources.
    std::fs::create_dir_all(&build_dir).expect("create c_src/build");
    let ok = Command::new("cmake")
        .arg("..")
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .current_dir(&build_dir)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    assert!(ok, "cmake configure of c_src failed");
    let ok = Command::new("cmake")
        .arg("--build")
        .arg(".")
        .current_dir(&build_dir)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    assert!(ok, "cmake build of c_src failed");

    find().expect("no .so produced in c_src/build")
}

/// Locate the Rust cdylib.
///
/// `cargo test` does NOT emit the `cdylib` artifact (integration tests do not
/// link the library, so Cargo stops at `rmeta`). We therefore build the
/// cdylib explicitly, into a dedicated target directory so we never contend
/// on the build lock held by the outer `cargo test`. Building here also
/// guarantees the `.so` under test always matches the current sources.
///
/// Environment overrides, used by `run_all.sh` to sweep build
/// configurations (pointer identity — which `get_predict_func` depends on —
/// is exactly the property that optimisation level, LTO and
/// identical-code-folding can perturb, so every configuration is tested):
///
/// * `FFI_SO_EXTRA_CARGO_ARGS` — extra whitespace-separated `cargo build`
///   args (feature selection, `--config` profile overrides, …).
/// * `FFI_SO_PROFILE` — cargo profile name (default `release`).
/// * `FFI_SO_TAG` — suffix for the dedicated target dir, so different
///   configurations do not clobber each other.
fn rust_so_path() -> PathBuf {
    static ONCE: OnceLock<PathBuf> = OnceLock::new();
    ONCE.get_or_init(|| {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let profile = std::env::var("FFI_SO_PROFILE").unwrap_or_else(|_| "release".to_string());
        let tag = std::env::var("FFI_SO_TAG").unwrap_or_else(|_| "default".to_string());
        let target_dir = manifest.join("target").join(format!("ffi-so-{tag}"));

        let mut cmd = Command::new(env!("CARGO"));
        cmd.arg("build")
            .arg("--quiet")
            .arg("--profile")
            .arg(&profile)
            .arg("--manifest-path")
            .arg(manifest.join("Cargo.toml"))
            .arg("--target-dir")
            .arg(&target_dir)
            .current_dir(&manifest)
            .env_remove("CARGO_TARGET_DIR")
            .env_remove("RUSTFLAGS");
        if let Ok(args) = std::env::var("FFI_SO_EXTRA_CARGO_ARGS") {
            for a in args.split_whitespace() {
                cmd.arg(a);
            }
        }
        let status = cmd.status().expect("failed to spawn cargo to build cdylib");
        assert!(
            status.success(),
            "cargo build of the cdylib failed (profile = {profile})"
        );

        // Cargo puts the `dev` profile's output in `debug/`.
        let out_dir = if profile == "dev" { "debug" } else { &profile };
        let so = target_dir.join(out_dir).join("libget_predict_func_lib.so");
        assert!(so.exists(), "cargo build succeeded but {:?} is missing", so);
        so
    })
    .clone()
}

/// Both libraries, held open for the lifetime of a test.
pub struct Pair {
    _c_lib: Library,
    _rust_lib: Library,
    pub c: GetPredictFunc,
    pub rust: GetPredictFunc,
}

impl Pair {
    pub fn load() -> Pair {
        let c_path = c_so_path();
        let r_path = rust_so_path();

        unsafe {
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {:?}: {e}", c_path));
            let rust_lib = Library::new(&r_path)
                .unwrap_or_else(|e| panic!("dlopen {:?}: {e}", r_path));

            let c_sym: Symbol<GetPredictFunc> = c_lib
                .get(b"get_predict_func\0")
                .expect("C .so must export get_predict_func");
            let r_sym: Symbol<GetPredictFunc> = rust_lib
                .get(b"get_predict_func\0")
                .expect("Rust .so must export get_predict_func");

            let c = *c_sym;
            let rust = *r_sym;

            Pair {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rust,
            }
        }
    }

    /// Call both and assert byte-identical `int` results.
    #[track_caller]
    pub fn assert_same(&self, pfcn: i32) -> i32 {
        let cv = unsafe { (self.c)(pfcn) };
        let rv = unsafe { (self.rust)(pfcn) };
        assert_eq!(
            cv.to_ne_bytes(),
            rv.to_ne_bytes(),
            "divergence at pfcn = {pfcn} (0x{pfcn:08x}): C = {cv}, Rust = {rv}"
        );
        cv
    }

    /// Call both, assert they match AND that the value equals `expected`.
    #[track_caller]
    pub fn assert_same_and_eq(&self, pfcn: i32, expected: i32) {
        let v = self.assert_same(pfcn);
        assert_eq!(
            v, expected,
            "pfcn = {pfcn}: both returned {v} but the C source dictates {expected}"
        );
    }
}

/// Deterministic PRNG (SplitMix64) so every randomized run is reproducible.
pub struct Rng(u64);

impl Rng {
    pub const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

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

    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }

    /// Uniform in `lo..=hi` (inclusive), works across the whole i32 range.
    pub fn in_range(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
}

/// The reference oracle taken straight from the C source: `get_predict_func`
/// returns 1 for every `pfcn` that has a specialised `_PfnNN` predictor
/// (0..=11) and 0 otherwise (the `default:` arm leaves `result` at 0).
pub fn c_source_oracle(pfcn: i32) -> i32 {
    if (0..=11).contains(&pfcn) { 1 } else { 0 }
}
