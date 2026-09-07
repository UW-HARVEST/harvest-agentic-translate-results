//! Shared harness for the differential tests.
//!
//! Locates and `dlopen`s the shared objects, mirrors the C ABI types, and
//! provides a fixed-seed PRNG so every "randomized" run is reproducible.
//!
//! There are four shared objects in play:
//!   * `c_src/build/lib<dir>.so`  — the C library exactly as CMake ships it
//!                                  (exports only `call_predict`).
//!   * `target/<profile>/libcall_predict_lib.so` — the Rust cdylib.
//!   * `harness/libcwrap.so`      — `harness/wrap.c`, which `#include`s the
//!                                  unmodified `c_src/src/lib.c` and re-exports
//!                                  its `static` helpers as `wrap_*`.
//!   * the same Rust cdylib built with `--features test_internals`, which
//!                                  re-exports its private helpers as `rsw_*`.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Types mirroring the C ABI
// ---------------------------------------------------------------------------

/// `struct btac1c_idxstate_s` from `c_src/src/lib.c`.
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

pub type FnInt = unsafe extern "C" fn(c_int) -> c_int;
pub type FnPredict = unsafe extern "C" fn(*mut c_int, c_int, c_int, *mut IdxState) -> c_int;
pub type FnCallThrough = unsafe extern "C" fn(c_int, *mut c_int, c_int, *mut IdxState) -> c_int;
pub type FnProbe = unsafe extern "C" fn() -> c_int;

// ---------------------------------------------------------------------------
// Locating the shared objects
// ---------------------------------------------------------------------------

pub fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// The C library built by `c_src/CMakeLists.txt`. Its name is derived from the
/// (randomly named) working directory, so glob for it instead of hardcoding.
pub fn c_lib_path() -> PathBuf {
    let dir = workspace_root().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {}: {e}. Build the C library first:\n  cd c_src && mkdir -p build && \
                 cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                dir.display()
            )
        })
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one .so in {}, found {:?}",
        dir.display(),
        found
    );
    found.pop().unwrap()
}

/// The static-helper harness built from `harness/wrap.c`.
pub fn c_wrap_path() -> PathBuf {
    let p = workspace_root().join("harness/libcwrap.so");
    assert!(
        p.exists(),
        "{} missing. Build it with:\n  gcc -fPIC -shared -I c_src/include -o \
         harness/libcwrap.so harness/wrap.c",
        p.display()
    );
    p
}

/// The Rust `cdylib`s to test, one per cargo profile.
///
/// `cargo test` deliberately does NOT emit the `cdylib` artifact (integration
/// tests only need an rlib to link against), so relying on whatever
/// `target/<profile>/libcall_predict_lib.so` happens to be lying around silently
/// tests a stale library. Instead, build the cdylib on demand into a dedicated
/// `--target-dir` (a separate dir, so it does not contend for the target lock
/// held by the running `cargo test`) with the same feature set this test binary
/// was compiled with, for BOTH profiles. Every assertion then runs against both,
/// which also covers the optimised codegen of the shipped `--release` artifact.
pub fn rust_lib_paths() -> &'static [PathBuf] {
    static PATHS: OnceLock<Vec<PathBuf>> = OnceLock::new();
    PATHS.get_or_init(|| {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let target_dir = manifest.join("target/xverify");
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

        // Mirror this test binary's own feature selection.
        let mut features: Vec<&str> = Vec::new();
        if cfg!(feature = "test_internals") {
            features.push("test_internals");
        }

        let mut out = Vec::new();
        for (profile_flag, profile_dir) in [(None, "debug"), (Some("--release"), "release")] {
            let mut cmd = std::process::Command::new(&cargo);
            cmd.current_dir(manifest)
                .arg("build")
                .arg("--lib")
                .arg("--target-dir")
                .arg(&target_dir)
                .arg("--no-default-features");
            if !features.is_empty() {
                cmd.arg("--features").arg(features.join(","));
            }
            if let Some(f) = profile_flag {
                cmd.arg(f);
            }
            let status = cmd
                .status()
                .unwrap_or_else(|e| panic!("failed to spawn `{cargo} build`: {e}"));
            assert!(
                status.success(),
                "`cargo build {} --target-dir target/xverify` failed",
                profile_flag.unwrap_or("")
            );
            let so = target_dir.join(profile_dir).join("libcall_predict_lib.so");
            assert!(so.exists(), "expected {} to exist after build", so.display());
            out.push(so);
        }
        out
    })
}

/// Loads a symbol out of a library, panicking with a useful message.
pub fn sym<'a, T: Copy>(lib: &'a Library, name: &str) -> Symbol<'a, T> {
    unsafe {
        lib.get(format!("{name}\0").as_bytes())
            .unwrap_or_else(|e| panic!("missing symbol `{name}`: {e}"))
    }
}

pub fn dlopen(path: &Path) -> Library {
    unsafe { Library::new(path) }.unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()))
}

// ---------------------------------------------------------------------------
// The public pair: `call_predict` from the real C .so and the Rust .so
// ---------------------------------------------------------------------------

pub struct PublicPair {
    pub rust_so: PathBuf,
    _c: Library,
    _r: Library,
    c_call_predict: FnInt,
    r_call_predict: FnInt,
}

impl PublicPair {
    fn load(rust_so: &Path) -> Self {
        let c = dlopen(&c_lib_path());
        let r = dlopen(rust_so);
        let c_call_predict = *sym::<FnInt>(&c, "call_predict");
        let r_call_predict = *sym::<FnInt>(&r, "call_predict");
        PublicPair {
            rust_so: rust_so.to_path_buf(),
            _c: c,
            _r: r,
            c_call_predict,
            r_call_predict,
        }
    }

    /// One pair per Rust profile artifact; every test runs against all of them.
    pub fn all() -> Vec<Self> {
        rust_lib_paths().iter().map(|p| Self::load(p)).collect()
    }

    pub fn call_predict(&self, pfcn: c_int) -> (c_int, c_int) {
        unsafe { ((self.c_call_predict)(pfcn), (self.r_call_predict)(pfcn)) }
    }

    pub fn assert_same(&self, pfcn: c_int) {
        let (c, r) = self.call_predict(pfcn);
        assert_eq!(
            c, r,
            "call_predict({pfcn}): C returned {c}, Rust ({}) returned {r}",
            self.rust_so.display()
        );
    }
}

// ---------------------------------------------------------------------------
// rng: SplitMix64, fixed seed -> reproducible "randomized" inputs
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform over the whole 32-bit signed range.
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Small mixed-sign value in `-2048..=2047`, the realistic audio-sample shape.
    pub fn small_i32(&mut self) -> i32 {
        (self.next_u64() as u32 % 4096) as i32 - 2048
    }
    pub fn next_i16(&mut self) -> i16 {
        self.next_u64() as u16 as i16
    }
    pub fn below(&mut self, n: u32) -> u32 {
        (self.next_u64() as u32) % n
    }
}
