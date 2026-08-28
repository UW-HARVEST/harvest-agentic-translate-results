//! Shared harness: builds and loads the C shared library and the Rust cdylib,
//! then exposes both through identical `libloading` symbol lookups. Rust
//! functions are never called directly, only through the `#[no_mangle]` exports
//! of the .so, exactly as an external C caller would reach them.

// Each integration-test binary compiles this whole module but uses only part of
// it, so unused items here are expected.
#![allow(dead_code)]

use libloading::Library;
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// Set by child processes so they reuse the artifacts the parent just built.
pub const SKIP_BUILD_ENV: &str = "HARVEST_SKIP_BUILD";
/// Overrides which C .so is loaded (used to test alternative C compilations).
pub const C_SO_ENV: &str = "HARVEST_C_SO";
/// Extra arguments forwarded to `cargo build`, e.g. a feature selection.
pub const CARGO_ARGS_ENV: &str = "HARVEST_CARGO_ARGS";

pub struct Pair {
    pub c: Library,
    pub rust: Library,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workspace_root() -> PathBuf {
    manifest_dir()
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

fn skip_build() -> bool {
    std::env::var_os(SKIP_BUILD_ENV).is_some()
}

/// `cargo test` does not rebuild `cdylib` artifacts, so the test would happily
/// load a stale .so and report a false pass. Build it here.
fn build_rust() {
    if skip_build() {
        return;
    }
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = Command::new(cargo);
    cmd.arg("build").arg("--quiet").current_dir(manifest_dir());
    if !cfg!(debug_assertions) {
        cmd.arg("--release");
    }
    if let Ok(extra) = std::env::var(CARGO_ARGS_ENV) {
        for a in extra.split_whitespace() {
            cmd.arg(a);
        }
    }
    // Avoid inheriting the outer invocation's target-dir juggling variables.
    let status = cmd.status().expect("failed to run cargo build");
    assert!(status.success(), "cargo build of the cdylib failed");
}

fn build_c() -> PathBuf {
    if let Ok(p) = std::env::var(C_SO_ENV) {
        let p = PathBuf::from(p);
        assert!(p.exists(), "{} does not exist", p.display());
        return p;
    }
    let src = workspace_root().join("c_src");
    let build = src.join("build");
    if !skip_build() {
        std::fs::create_dir_all(&build).expect("create c build dir");
        if !build.join("CMakeCache.txt").exists() {
            let st = Command::new("cmake")
                .arg("..")
                .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
                .current_dir(&build)
                .output()
                .expect("run cmake configure");
            assert!(
                st.status.success(),
                "cmake configure failed: {}",
                String::from_utf8_lossy(&st.stderr)
            );
        }
        let st = Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build)
            .output()
            .expect("run cmake build");
        assert!(
            st.status.success(),
            "cmake build failed: {}",
            String::from_utf8_lossy(&st.stderr)
        );
    }
    find_so_in(&build).unwrap_or_else(|| {
        panic!(
            "no C .so found in {}; build it with cmake first",
            build.display()
        )
    })
}

fn find_so_in(dir: &Path) -> Option<PathBuf> {
    let mut found: Vec<PathBuf> = Vec::new();
    for e in std::fs::read_dir(dir).ok()?.flatten() {
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) == Some("so") {
            found.push(p);
        }
    }
    found.sort();
    found.pop()
}

fn rust_so() -> PathBuf {
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let p = manifest_dir()
        .join("target")
        .join(profile)
        .join("libarity_lib.so");
    assert!(
        p.exists(),
        "{} not found after cargo build; check [lib] name/crate-type",
        p.display()
    );
    p
}

/// Path of the C shared object that the harness loads.
pub fn c_so_path() -> PathBuf {
    build_c()
}

/// Path of the Rust cdylib that the harness loads.
pub fn rust_so_path() -> PathBuf {
    rust_so()
}

pub fn libs() -> &'static Pair {
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| {
        build_rust();
        let c_path = build_c();
        let rust_path = rust_so();
        unsafe {
            let c = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("failed to load {}: {e}", c_path.display()));
            let rust = Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("failed to load {}: {e}", rust_path.display()));
            Pair { c, rust }
        }
    })
}

// Signatures of the exported C API, mirroring `c_src/include/lib.h` and the
// internal helpers defined in `c_src/src/lib.c`.
pub type Fn2 = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type Fn3 = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
pub type Fn4 = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
pub type FnPtrCInt = unsafe extern "C" fn(*const std::ffi::c_char) -> c_int;
pub type FnShift = unsafe extern "C" fn(*mut c_int, c_int, c_int);
pub type FnMatrix = unsafe extern "C" fn(*mut [c_int; 4]);
/// The C definition takes `unsigned char len` even though `include/lib.h`
/// declares `int len`. Using `int` on both sides reproduces what a caller
/// compiled against the public header actually does.
pub type FnArity = unsafe extern "C" fn(c_int, *mut c_int) -> c_int;

#[macro_export]
macro_rules! sym {
    ($lib:expr, $name:literal, $ty:ty) => {{
        let s: libloading::Symbol<$ty> = unsafe {
            $lib.get(concat!($name, "\0").as_bytes())
                .unwrap_or_else(|e| panic!("missing symbol {}: {}", $name, e))
        };
        // The libraries live in a `OnceLock` for the whole process, so the
        // function pointer stays valid after the `Symbol` guard is dropped.
        *s
    }};
}
