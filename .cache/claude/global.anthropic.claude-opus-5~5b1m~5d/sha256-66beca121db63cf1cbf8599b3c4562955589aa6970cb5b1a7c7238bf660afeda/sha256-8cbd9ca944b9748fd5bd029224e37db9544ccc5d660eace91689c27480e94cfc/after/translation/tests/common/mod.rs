//! Shared harness: loads BOTH the C `.so` and the Rust `.so` via `libloading`
//! and calls their exported symbols. The Rust implementation is NEVER called
//! directly as a Rust function — always through the dynamic library, exactly as
//! an external C consumer would, so the `#[no_mangle]` wrapper is under test.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

pub const RUST_SO_NAME: &str = "libcall_predict_lib.so";

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn repo_root() -> PathBuf {
    manifest_dir()
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

// ---------------------------------------------------------------- C library

fn find_c_so() -> Option<PathBuf> {
    let build = repo_root().join("c_src/build");
    let mut found = None;
    for e in std::fs::read_dir(&build).ok()?.flatten() {
        let p = e.path();
        let n = p.file_name()?.to_string_lossy().to_string();
        if n.starts_with("lib") && n.ends_with(".so") {
            found = Some(p);
        }
    }
    found
}

/// Build (if needed) and return the path to the C shared library.
pub fn c_so_path() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        if let Some(p) = find_c_so() {
            return p;
        }
        let build = repo_root().join("c_src/build");
        std::fs::create_dir_all(&build).expect("create c_src/build");
        let st = Command::new("cmake")
            .current_dir(&build)
            .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
            .status()
            .expect("run cmake configure");
        assert!(st.success(), "cmake configure failed");
        let st = Command::new("cmake")
            .current_dir(&build)
            .args(["--build", "."])
            .status()
            .expect("run cmake build");
        assert!(st.success(), "cmake build failed");
        find_c_so().expect("C .so produced in c_src/build")
    })
    .as_path()
}

// ------------------------------------------------------------- Rust library

/// Build the Rust cdylib for `profile` ("debug" | "release") with the given
/// feature list and return its path.
///
/// `cargo test` does NOT build a `crate-type = ["cdylib"]` library for an
/// integration test (the test cannot link it), so whatever `.so` happens to sit
/// in `target/<profile>/` may be stale or built with a different feature set.
/// Each variant is therefore built into its OWN `CARGO_TARGET_DIR`, always
/// freshly, so a test can never load someone else's artifact. Isolated target
/// dirs also mean no lock contention with the outer `cargo test`.
pub fn rust_so_variant(profile: &str, features: &[&str]) -> PathBuf {
    static BUILT: OnceLock<std::sync::Mutex<Vec<(String, PathBuf)>>> = OnceLock::new();
    let cache = BUILT.get_or_init(|| std::sync::Mutex::new(Vec::new()));

    let key = format!("{profile}|{}", features.join(","));
    if let Some((_, p)) = cache
        .lock()
        .unwrap()
        .iter()
        .find(|(k, _)| *k == key)
        .cloned()
    {
        return p;
    }

    let tag = if features.is_empty() {
        profile.to_string()
    } else {
        format!("{profile}-{}", features.join("-"))
    };
    let target_dir = manifest_dir().join("target/so").join(&tag);

    let mut args: Vec<String> = vec!["build".into(), "--offline".into(), "--lib".into()];
    if profile == "release" {
        args.push("--release".into());
    }
    if !features.is_empty() {
        args.push("--features".into());
        args.push(features.join(","));
    }

    let st = Command::new(env!("CARGO"))
        .current_dir(manifest_dir())
        .env("CARGO_TARGET_DIR", &target_dir)
        .args(&args)
        .status()
        .unwrap_or_else(|e| panic!("cargo build ({tag}): {e}"));
    assert!(st.success(), "cargo build ({tag}) failed");

    let out = target_dir.join(profile).join(RUST_SO_NAME);
    assert!(out.exists(), "rust cdylib not found at {out:?}");

    cache.lock().unwrap().push((key, out.clone()));
    out
}

/// Default-feature cdylib for `profile` — the shipped artifact shape.
pub fn rust_so_path(profile: &str) -> PathBuf {
    assert!(
        profile == "debug" || profile == "release",
        "unknown profile {profile}"
    );
    rust_so_variant(profile, &[])
}

// ------------------------------------------------------------------- harness

pub struct Libs {
    _c: libloading::Library,
    _rust: libloading::Library,
    pub rust_profile: &'static str,
    c_call_predict: unsafe extern "C" fn(i32) -> i32,
    rust_call_predict: unsafe extern "C" fn(i32) -> i32,
}

impl Libs {
    /// Canonical harness: compares against the release cdylib (the shipped
    /// artifact, built with `panic = "abort"`).
    pub fn load() -> Libs {
        Libs::load_profile("release")
    }

    pub fn load_profile(profile: &'static str) -> Libs {
        let cp = c_so_path().to_path_buf();
        let rp = rust_so_path(profile);
        unsafe {
            let c = libloading::Library::new(&cp)
                .unwrap_or_else(|e| panic!("dlopen C lib {cp:?}: {e}"));
            let rust = libloading::Library::new(&rp)
                .unwrap_or_else(|e| panic!("dlopen Rust lib {rp:?}: {e}"));

            let cf: libloading::Symbol<unsafe extern "C" fn(i32) -> i32> = c
                .get(b"call_predict\0")
                .expect("C .so exports call_predict");
            let rf: libloading::Symbol<unsafe extern "C" fn(i32) -> i32> = rust
                .get(b"call_predict\0")
                .expect("Rust .so exports call_predict");
            let c_call_predict = *cf;
            let rust_call_predict = *rf;

            Libs {
                _c: c,
                _rust: rust,
                rust_profile: profile,
                c_call_predict,
                rust_call_predict,
            }
        }
    }

    #[inline]
    pub fn c(&self, pfcn: i32) -> i32 {
        unsafe { (self.c_call_predict)(pfcn) }
    }

    #[inline]
    pub fn rust(&self, pfcn: i32) -> i32 {
        unsafe { (self.rust_call_predict)(pfcn) }
    }

    /// Assert byte-identical results and return the shared value.
    #[track_caller]
    pub fn assert_same(&self, pfcn: i32) -> i32 {
        let a = self.c(pfcn);
        let b = self.rust(pfcn);
        assert_eq!(
            a.to_le_bytes(),
            b.to_le_bytes(),
            "divergence for call_predict({pfcn}) [rust profile {}]: C returned {a}, Rust returned {b}",
            self.rust_profile
        );
        a
    }
}

/// Deterministic, fixed-seed PRNG (SplitMix64) so runs are reproducible.
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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Uniform in `lo..=hi`.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
}

pub const SEED: u64 = 0x5EED_1234_ABCD_9876;
