#![allow(dead_code)]

//! Shared harness: builds/locates both shared libraries and loads
//! `to_barycentric` out of each one through `libloading`.
//!
//! The Rust side is deliberately exercised *only* through the `.so` export so
//! that the `#[no_mangle] extern "C"` wrapper and the C ABI struct
//! passing/returning convention are part of what is under test.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Mirrors `typedef struct lm_vec2 { float x, y; } lm_vec2;`
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct LmVec2 {
    pub x: f32,
    pub y: f32,
}

impl LmVec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Raw representation, used for byte-exact comparison (NaN included).
    pub fn bits(&self) -> (u32, u32) {
        (self.x.to_bits(), self.y.to_bits())
    }
}

pub type ToBarycentricFn =
    unsafe extern "C" fn(LmVec2, LmVec2, LmVec2, LmVec2) -> LmVec2;

/// Workspace root (the directory holding `c_src/` and `translation/`).
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

fn first_shared_object(dir: &Path) -> Option<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension().and_then(|s| s.to_str()) == Some("so")
                && p.file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|n| n.starts_with("lib"))
        })
        .collect();
    found.sort();
    found.pop()
}

/// Build (if necessary) and return the path to the C shared library.
///
/// A build directory under `target/` is used so that nothing inside `c_src/`
/// is ever written to; an already-present `c_src/build` output is reused when
/// it exists.
pub fn c_library_path() -> PathBuf {
    let root = workspace_root();

    if let Some(so) = first_shared_object(&root.join("c_src/build")) {
        return so;
    }

    let build_dir = root.join("translation/target/c-build");
    std::fs::create_dir_all(&build_dir).expect("create C build dir");

    let configure = Command::new("cmake")
        .arg(root.join("c_src"))
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .current_dir(&build_dir)
        .output()
        .expect("run cmake configure");
    assert!(
        configure.status.success(),
        "cmake configure failed:\n{}",
        String::from_utf8_lossy(&configure.stderr)
    );

    let build = Command::new("cmake")
        .args(["--build", "."])
        .current_dir(&build_dir)
        .output()
        .expect("run cmake build");
    assert!(
        build.status.success(),
        "cmake build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    first_shared_object(&build_dir).expect("C shared library was produced")
}

/// Build and return the path to the Rust `cdylib`.
///
/// A dedicated target directory keeps this nested `cargo build` from
/// contending on the lock held by the running `cargo test`.
pub fn rust_library_path() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out_dir = manifest.join("target/ffi-cdylib");

    let mut cmd = Command::new(env!("CARGO"));
    cmd.args(["build", "--release", "--lib"])
        .arg("--target-dir")
        .arg(&out_dir)
        .current_dir(manifest);

    // Propagate the feature selection this test binary was compiled with so
    // the loaded `.so` matches the configuration under test.
    cmd.args(active_feature_args());

    // Avoid inheriting the outer cargo invocation's environment, which would
    // otherwise redirect the nested build back at the locked target dir.
    for var in ["CARGO_TARGET_DIR", "RUSTC_WORKSPACE_WRAPPER", "RUSTC_WRAPPER"] {
        cmd.env_remove(var);
    }

    let out = cmd.output().expect("run nested cargo build");
    assert!(
        out.status.success(),
        "building the Rust cdylib failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    first_shared_object(&out_dir.join("release"))
        .expect("Rust shared library was produced")
}

/// The crate currently declares no `[features]`, so there is exactly one
/// configuration. This hook keeps the harness correct if features are added.
fn active_feature_args() -> Vec<String> {
    let enabled: Vec<String> = std::env::vars()
        .filter_map(|(k, _)| k.strip_prefix("CARGO_FEATURE_").map(str::to_owned))
        .map(|f| f.to_lowercase().replace('_', "-"))
        .collect();

    if enabled.is_empty() {
        return Vec::new();
    }
    vec![
        "--no-default-features".to_string(),
        "--features".to_string(),
        enabled.join(","),
    ]
}

/// Both implementations, loaded from their respective shared objects.
pub struct Pair {
    _c_lib: libloading::Library,
    _rust_lib: libloading::Library,
    c_fn: ToBarycentricFn,
    rust_fn: ToBarycentricFn,
}

impl Pair {
    pub fn load() -> Self {
        let c_path = c_library_path();
        let rust_path = rust_library_path();

        unsafe {
            let c_lib = libloading::Library::new(&c_path)
                .unwrap_or_else(|e| panic!("load {}: {e}", c_path.display()));
            let rust_lib = libloading::Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("load {}: {e}", rust_path.display()));

            let c_fn: libloading::Symbol<ToBarycentricFn> = c_lib
                .get(b"to_barycentric\0")
                .expect("C .so exports to_barycentric");
            let rust_fn: libloading::Symbol<ToBarycentricFn> = rust_lib
                .get(b"to_barycentric\0")
                .expect("Rust .so exports to_barycentric");

            let c_fn = *c_fn;
            let rust_fn = *rust_fn;

            Self { _c_lib: c_lib, _rust_lib: rust_lib, c_fn, rust_fn }
        }
    }

    pub fn c(&self, p1: LmVec2, p2: LmVec2, p3: LmVec2, p: LmVec2) -> LmVec2 {
        unsafe { (self.c_fn)(p1, p2, p3, p) }
    }

    pub fn rust(&self, p1: LmVec2, p2: LmVec2, p3: LmVec2, p: LmVec2) -> LmVec2 {
        unsafe { (self.rust_fn)(p1, p2, p3, p) }
    }

    /// Call both and assert the returned struct is bit-identical.
    #[track_caller]
    pub fn assert_same(
        &self,
        label: &str,
        p1: LmVec2,
        p2: LmVec2,
        p3: LmVec2,
        p: LmVec2,
    ) {
        let expected = self.c(p1, p2, p3, p);
        let actual = self.rust(p1, p2, p3, p);
        assert_eq!(
            expected.bits(),
            actual.bits(),
            "mismatch [{label}]\n  inputs: p1={p1:?} p2={p2:?} p3={p3:?} p={p:?}\n  \
             C    = ({:?}, {:?}) bits {:#010x?}\n  Rust = ({:?}, {:?}) bits {:#010x?}",
            expected.x,
            expected.y,
            expected.bits(),
            actual.x,
            actual.y,
            actual.bits(),
        );
    }
}

/// Deterministic xorshift64* generator so failures are reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
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

    /// Uniform in `[-range, range]`.
    pub fn coord(&mut self, range: f32) -> f32 {
        let unit = (self.next_u32() as f64) / (u32::MAX as f64);
        ((unit * 2.0 - 1.0) as f32) * range
    }

    /// Arbitrary bit pattern reinterpreted as `f32` (may be NaN/inf/subnormal).
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
}

/// Interesting float values for edge-case sweeps.
pub const EDGE_FLOATS: &[f32] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    2.0,
    -2.0,
    3.0,
    1e-30,
    -1e-30,
    1e30,
    -1e30,
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    f32::MAX,
    f32::MIN,
    f32::EPSILON,
    1.0e-45, // smallest positive subnormal
    16777216.0,
    16777217.0,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
];
