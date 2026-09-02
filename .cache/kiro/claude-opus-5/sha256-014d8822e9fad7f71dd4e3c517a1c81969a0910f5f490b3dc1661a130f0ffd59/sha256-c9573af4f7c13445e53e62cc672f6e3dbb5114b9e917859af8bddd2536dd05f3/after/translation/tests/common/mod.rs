//! Differential test harness: loads BOTH the C `.so` and the Rust `.so` with
//! `libloading` and calls `jumpnode` through the FFI boundary on each, so the
//! `#[no_mangle] extern "C"` export wrapper is under test too. No Rust function
//! is ever called directly.

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

pub type JumpnodeFn = unsafe extern "C" fn(
    std::os::raw::c_int,
    std::os::raw::c_int,
    std::os::raw::c_int,
    std::os::raw::c_int,
) -> std::os::raw::c_int;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate dir has a parent")
        .to_path_buf()
}

/// Build the C shared library exactly as the task describes, then locate it.
/// The library file name derives from the parent directory name in
/// `CMakeLists.txt`, so it is discovered rather than hard-coded.
fn c_library_path() -> PathBuf {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT.get_or_init(build_c_library).clone()
}

fn build_c_library() -> PathBuf {
    let root = workspace_root();
    let c_src = root.join("c_src");
    let build = c_src.join("build");

    if find_so(&build).is_none() {
        std::fs::create_dir_all(&build).expect("create c_src/build");
        let cfg = Command::new("cmake")
            .current_dir(&build)
            .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
            .output()
            .expect("run cmake configure");
        assert!(
            cfg.status.success(),
            "cmake configure failed:\n{}",
            String::from_utf8_lossy(&cfg.stderr)
        );
        let bld = Command::new("cmake")
            .current_dir(&build)
            .args(["--build", "."])
            .output()
            .expect("run cmake build");
        assert!(
            bld.status.success(),
            "cmake build failed:\n{}",
            String::from_utf8_lossy(&bld.stderr)
        );
    }

    find_so(&build).unwrap_or_else(|| panic!("no .so found in {}", build.display()))
}

fn find_so(dir: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut found: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    found.into_iter().next()
}

/// Locate the Rust cdylib.
///
/// `cargo test` does NOT relink the `cdylib` artifact (it builds the lib as a
/// test harness instead), so `target/<profile>/libjumpnode_lib.so` can be
/// arbitrarily stale — loading it would silently test an old binary. To
/// guarantee we always exercise the current `src/lib.rs`, build the cdylib
/// explicitly into a dedicated target directory. A separate `--target-dir`
/// keeps its own lock file, so this does not deadlock against the outer
/// `cargo test`. `--lib` means no test targets are compiled, so there is no
/// recursion.
fn rust_library_path() -> PathBuf {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT
        .get_or_init(|| {
            let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
            // Build the cdylib with overflow-checks on (debug) as well as off
            // (release) depending on FFI_TEST_PROFILE, so a latent Rust overflow
            // panic on a path where C silently wraps cannot hide.
            let profile = std::env::var("FFI_TEST_PROFILE").unwrap_or_else(|_| "release".into());
            assert!(
                profile == "release" || profile == "debug",
                "FFI_TEST_PROFILE must be `release` or `debug`, got {profile:?}"
            );
            let target_dir = manifest.join("target").join(format!("ffi-cdylib-{profile}"));
            let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

            let mut args: Vec<String> = vec!["build".into(), "--lib".into()];
            if profile == "release" {
                args.push("--release".into());
            }
            args.push("--target-dir".into());
            args.push(target_dir.to_str().unwrap().to_string());
            args.extend(cdylib_feature_args());

            let out = Command::new(&cargo)
                .current_dir(manifest)
                .args(&args)
                .output()
                .expect("run nested `cargo build --lib` for the cdylib");
            assert!(
                out.status.success(),
                "building the Rust cdylib failed:\n{}",
                String::from_utf8_lossy(&out.stderr)
            );

            let so = target_dir.join(&profile).join("libjumpnode_lib.so");
            assert!(
                so.exists(),
                "cdylib not produced at {} (stderr:\n{})",
                so.display(),
                String::from_utf8_lossy(&out.stderr)
            );
            so
        })
        .clone()
}

/// Propagate the feature selection of the running test binary to the nested
/// cdylib build, so `cargo test --no-default-features --features <combo>`
/// actually tests a `.so` built with that same combo.
///
/// Cargo exposes each enabled feature as `CARGO_FEATURE_<NAME>` in the build
/// environment. The crate declares no `[features]`, so this is normally empty;
/// the plumbing is here so adding features later cannot silently invalidate the
/// tests.
fn cdylib_feature_args() -> Vec<String> {
    let mut features: Vec<String> = std::env::vars()
        .filter_map(|(k, _)| k.strip_prefix("CARGO_FEATURE_").map(|f| f.to_lowercase()))
        .collect();
    features.sort();

    if let Ok(explicit) = std::env::var("FFI_TEST_FEATURES") {
        features = explicit
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
    }

    let mut args = Vec::new();
    if std::env::var("FFI_TEST_NO_DEFAULT_FEATURES").is_ok() {
        args.push("--no-default-features".to_string());
    }
    if !features.is_empty() {
        args.push("--features".to_string());
        args.push(features.join(","));
    }
    args
}

/// Both libraries, each loaded through `libloading`, plus the resolved
/// `jumpnode` symbol from each.
pub struct Pair {
    _c_lib: Library,
    _rust_lib: Library,
    c_fn: JumpnodeFn,
    rust_fn: JumpnodeFn,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

impl Pair {
    pub fn load() -> Pair {
        let c_path = c_library_path();
        let rust_path = rust_library_path();

        // SAFETY: both paths point at shared objects we just built ourselves.
        let c_lib = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", c_path.display()));
        let rust_lib = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", rust_path.display()));

        let c_fn = unsafe {
            let s: Symbol<JumpnodeFn> = c_lib
                .get(b"jumpnode\0")
                .expect("C .so does not export `jumpnode`");
            *s
        };
        let rust_fn = unsafe {
            let s: Symbol<JumpnodeFn> = rust_lib
                .get(b"jumpnode\0")
                .expect("Rust .so does not export `jumpnode` (missing #[no_mangle]?)");
            *s
        };

        Pair {
            _c_lib: c_lib,
            _rust_lib: rust_lib,
            c_fn,
            rust_fn,
            c_path,
            rust_path,
        }
    }

    pub fn c(&self, m: i32, n: i32, d: i32, f: i32) -> i32 {
        unsafe { (self.c_fn)(m, n, d, f) }
    }

    pub fn rust(&self, m: i32, n: i32, d: i32, f: i32) -> i32 {
        unsafe { (self.rust_fn)(m, n, d, f) }
    }

    /// Call both through their `.so` exports and assert byte-identical results.
    #[track_caller]
    pub fn assert_same(&self, m: i32, n: i32, d: i32, f: i32) -> i32 {
        let c = self.c(m, n, d, f);
        let r = self.rust(m, n, d, f);
        assert_eq!(
            c, r,
            "DIVERGENCE jumpnode(mode={m} (0o{m:o}), node_id={n}, depth={d}, flags={f}): \
             C returned {c} (0o{c:o}), Rust returned {r} (0o{r:o})"
        );
        c
    }
}

/// Deterministic xorshift64* PRNG. Fixed seeds keep every property-style run
/// reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform over the entire `i32` range, including `INT_MIN` / `INT_MAX`.
    pub fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }
    /// Uniform in `lo..=hi`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
}

/// Values that sit on interesting boundaries for every `int` argument.
pub const EXTREMES: &[i32] = &[
    i32::MIN,
    i32::MIN + 1,
    -2_147_483_647,
    -1_000_000_000,
    -65_537,
    -256,
    -128,
    -17,
    -16,
    -11,
    -10,
    -9,
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
    9,
    10,
    15,
    16,
    17,
    63,
    64,
    0o177,
    0o200,
    0o377,
    255,
    256,
    1000,
    65_535,
    65_536,
    1_000_000_000,
    2_147_483_646,
    i32::MAX,
];
