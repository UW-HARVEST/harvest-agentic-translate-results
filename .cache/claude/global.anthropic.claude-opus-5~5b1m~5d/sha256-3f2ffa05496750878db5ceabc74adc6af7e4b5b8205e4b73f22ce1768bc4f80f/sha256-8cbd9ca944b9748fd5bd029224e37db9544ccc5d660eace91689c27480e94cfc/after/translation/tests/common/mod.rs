//! Shared plumbing for the differential tests.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and driven
//! purely through their exported symbols — the Rust functions are never called
//! directly, so the `#[no_mangle]` / `extern "C"` wrappers are under test too.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// The C struct btac1c_idxstate_s, as an FFI-compatible Rust mirror.
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

impl Default for IdxState {
    fn default() -> Self {
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

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed, reproducible property testing.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
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
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// Uniform in `-bound..=bound` (bound must be < i32::MAX).
    pub fn small(&mut self, bound: i32) -> i32 {
        let span = (bound as i64) * 2 + 1;
        ((self.next_u64() % (span as u64)) as i64 - bound as i64) as i32
    }
}

// ---------------------------------------------------------------------------
// Value shapes — the psamp[] input-shape axis from CONFIGS.md.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// |v| < 1000 — no intermediate overflow anywhere.
    Small,
    /// |v| ~ 2^20 — overflows nothing for the 2-tap arms, stresses the 8-tap ones.
    Medium,
    /// Full i32 domain incl. i32::MIN / i32::MAX — signed overflow (wrapping).
    Extreme,
    /// Strictly positive (truncating `/` rounds toward zero here).
    Positive,
    /// Strictly negative (`>>` rounds toward -inf, `/` toward zero — they differ).
    Negative,
    /// All zeroes.
    Zero,
}

impl Shape {
    pub const ALL: [Shape; 6] = [
        Shape::Small,
        Shape::Medium,
        Shape::Extreme,
        Shape::Positive,
        Shape::Negative,
        Shape::Zero,
    ];

    pub fn gen(self, rng: &mut Rng) -> [i32; 8] {
        let mut out = [0i32; 8];
        for slot in out.iter_mut() {
            *slot = match self {
                Shape::Small => rng.small(999),
                Shape::Medium => rng.small(1 << 20),
                Shape::Extreme => match rng.below(8) {
                    0 => i32::MIN,
                    1 => i32::MAX,
                    2 => i32::MIN + 1,
                    3 => i32::MAX - 1,
                    _ => rng.next_i32(),
                },
                Shape::Positive => 1 + (rng.next_u64() % 100_000) as i32,
                Shape::Negative => -1 - (rng.next_u64() % 100_000) as i32,
                Shape::Zero => 0,
            };
        }
        out
    }
}

/// The `idx` shape axis: values that hit every wrap point of `(idx - k) & 7`.
pub const IDX_BOUNDARIES: &[i32] = &[
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 15, 16, -1, -2, -3, -4, -5, -6, -7, -8, -9, -16,
    i32::MIN, i32::MIN + 1, i32::MIN + 7, i32::MIN + 8, i32::MAX, i32::MAX - 1,
    i32::MAX - 7, i32::MAX - 8, 1 << 30, -(1 << 30),
];

/// Every `pfcn` value that any `switch` in lib.c mentions, plus the values that
/// fall through to a `default:` arm.
pub const PFCN_ALL: &[i32] = &[
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 99, -1, -2, -12, -100,
    i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1,
];

// ---------------------------------------------------------------------------
// Locating and building the two shared objects.
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn project_root() -> PathBuf {
    manifest_dir().parent().unwrap().to_path_buf()
}

/// `target/<profile>/` — the directory holding the cdylib cargo just built for
/// the feature set this test run was compiled with.
fn artifact_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/difftest-<hash>
    exe.parent().unwrap().parent().unwrap().to_path_buf()
}

fn find_so(dir: &Path, contains: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for e in entries.flatten() {
        let p = e.path();
        let name = p.file_name()?.to_string_lossy().to_string();
        if name.starts_with("lib") && name.ends_with(".so") && name.contains(contains) {
            return Some(p);
        }
    }
    None
}

/// Build (once) the C shared library via its own CMake build system and return
/// the path to the resulting `.so`.
pub fn c_so_path() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let root = project_root();
        let c_src = root.join("c_src");
        let build = c_src.join("build");
        if find_so(&build, "").is_none() {
            std::fs::create_dir_all(&build).expect("mkdir c_src/build");
            let st = Command::new("cmake")
                .current_dir(&build)
                .arg("..")
                .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
                .status()
                .expect("run cmake");
            assert!(st.success(), "cmake configure failed");
            let st = Command::new("cmake")
                .current_dir(&build)
                .args(["--build", "."])
                .status()
                .expect("run cmake --build");
            assert!(st.success(), "cmake build failed");
        }
        find_so(&build, "").expect("C .so not found in c_src/build")
    })
    .as_path()
}

/// Build (once) the C *harness* shared object: the original translation unit
/// `#include`d verbatim from a file outside `c_src/`, plus the
/// `__difftest_*` re-exports of its internal-linkage functions.
pub fn c_harness_so_path() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let root = project_root();
        let out_dir = artifact_dir().join("difftest-c");
        std::fs::create_dir_all(&out_dir).expect("mkdir difftest-c");
        let out = out_dir.join("libdifftest_c_harness.so");
        let src = manifest_dir().join("tests").join("difftest_c_harness.c");
        let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
        let st = Command::new(&cc)
            .arg("-std=c11")
            .arg("-O2")
            .arg("-fPIC")
            .arg("-shared")
            .arg("-I")
            .arg(root.join("c_src").join("include"))
            .arg("-I")
            .arg(root.join("c_src").join("src"))
            .arg(&src)
            .arg("-o")
            .arg(&out)
            .status()
            .unwrap_or_else(|e| panic!("failed to run {cc}: {e}"));
        assert!(st.success(), "compiling the C harness failed");
        out
    })
    .as_path()
}

/// The Rust cdylib, built for *this* test binary's feature set.
///
/// `[lib] crate-type = ["cdylib"]` means `cargo test` does not itself produce
/// the shared object, so the test drives `cargo build` into a dedicated
/// `--target-dir` (avoiding any lock contention with the running `cargo test`)
/// and then loads the resulting `.so` through `libloading`, exactly as an
/// external C consumer would.  The Rust functions are never called directly.
pub fn rust_so_path() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        // Honour an explicit override (used by the feature-matrix driver script).
        if let Ok(p) = std::env::var("DIFFTEST_RUST_SO") {
            let p = PathBuf::from(p);
            assert!(p.exists(), "DIFFTEST_RUST_SO={} does not exist", p.display());
            return p;
        }

        let features_tag = if cfg!(feature = "difftest") {
            "difftest"
        } else {
            "nofeat"
        };
        let target_dir = manifest_dir()
            .join("target")
            .join(format!("difftest-so-{features_tag}"));

        let mut cmd = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
        cmd.current_dir(manifest_dir())
            .arg("build")
            .arg("--offline")
            .arg("--lib")
            .arg("--no-default-features")
            .arg("--target-dir")
            .arg(&target_dir);
        if cfg!(feature = "difftest") {
            cmd.args(["--features", "difftest"]);
        }
        if cfg!(not(debug_assertions)) {
            cmd.arg("--release");
        }
        // Do not inherit the parent cargo's env, which would redirect the build.
        cmd.env_remove("CARGO_TARGET_DIR")
            .env_remove("RUSTC_WORKSPACE_WRAPPER")
            .env_remove("CARGO_BUILD_TARGET_DIR");
        let st = cmd.status().expect("run cargo build for the cdylib");
        assert!(st.success(), "cargo build of the cdylib failed");

        let profile_dir = if cfg!(debug_assertions) { "debug" } else { "release" };
        let dir = target_dir.join(profile_dir);
        find_so(&dir, "get_predict_func_lib")
            .unwrap_or_else(|| panic!("Rust cdylib not found in {}", dir.display()))
    })
    .as_path()
}

// ---------------------------------------------------------------------------
// The loaded pair.
// ---------------------------------------------------------------------------

pub type GetPredictFunc = unsafe extern "C" fn(i32) -> i32;
pub type DifftestPredict =
    unsafe extern "C" fn(i32, *mut i32, i32, i32, *mut IdxState) -> i32;
pub type DifftestLayout = unsafe extern "C" fn(i32) -> i32;
pub type DifftestDispatchTag = unsafe extern "C" fn(i32) -> i32;

pub struct Side {
    pub name: &'static str,
    pub lib: libloading::Library,
}

impl Side {
    pub fn sym<T>(&self, name: &str) -> libloading::Symbol<'_, T> {
        unsafe {
            self.lib
                .get(name.as_bytes())
                .unwrap_or_else(|e| panic!("{}: missing symbol `{}`: {}", self.name, name, e))
        }
    }
    pub fn has(&self, name: &str) -> bool {
        unsafe {
            self.lib
                .get::<unsafe extern "C" fn()>(name.as_bytes())
                .is_ok()
        }
    }
}

/// Load the C `.so` (public ABI) and the Rust `.so`.
pub fn pair_public() -> (Side, Side) {
    let c = unsafe { libloading::Library::new(c_so_path()) }.expect("load C .so");
    let r = unsafe { libloading::Library::new(rust_so_path()) }.expect("load Rust .so");
    (
        Side { name: "C", lib: c },
        Side { name: "Rust", lib: r },
    )
}

/// Load the C harness `.so` (internal predictors re-exported) and the Rust `.so`.
pub fn pair_internal() -> (Side, Side) {
    let c = unsafe { libloading::Library::new(c_harness_so_path()) }.expect("load C harness .so");
    let r = unsafe { libloading::Library::new(rust_so_path()) }.expect("load Rust .so");
    (
        Side { name: "C-harness", lib: c },
        Side { name: "Rust", lib: r },
    )
}

/// True when this test binary was compiled with the `difftest` feature, i.e.
/// when the Rust `.so` exposes the internal-predictor hooks.
pub fn difftest_enabled() -> bool {
    cfg!(feature = "difftest")
}
