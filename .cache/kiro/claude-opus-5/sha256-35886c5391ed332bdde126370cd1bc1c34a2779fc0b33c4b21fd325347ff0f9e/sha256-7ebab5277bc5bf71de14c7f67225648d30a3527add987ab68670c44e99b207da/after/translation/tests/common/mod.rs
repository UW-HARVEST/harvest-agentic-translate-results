//! Shared helpers: locate and load the C and Rust shared libraries.
//!
//! Included by several test binaries, not all of which use every helper.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// `char *bin2hex(char *hex, size_t hex_maxlen, const uint8_t *bin, size_t bin_len)`
pub type Bin2HexFn = unsafe extern "C" fn(*mut i8, usize, *const u8, usize) -> *mut i8;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn find_so(dir: &Path) -> Option<PathBuf> {
    let mut found: Vec<PathBuf> = Vec::new();
    let entries = std::fs::read_dir(dir).ok()?;
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) == Some("so") {
            found.push(p);
        }
    }
    found.sort();
    found.into_iter().next()
}

/// Path to the C shared library (built via cmake into `c_src/build`).
pub fn c_so_path() -> PathBuf {
    let build = workspace_root().join("c_src").join("build");
    find_so(&build).unwrap_or_else(|| {
        panic!(
            "no .so found in {}. Build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

/// Path to the Rust cdylib, building it on demand.
///
/// `cargo test` does not emit the `cdylib` artifact (it only needs the rlib for
/// the test harness), so the first caller shells out to `cargo build --lib`
/// using a dedicated `--target-dir`. The separate directory keeps us clear of
/// the lock held by the outer `cargo test`.
pub fn rust_so_path() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(build_rust_so).clone()
}

fn build_rust_so() -> PathBuf {
    const SO_NAME: &str = "libbin2hex_lib.so";

    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let target_dir = manifest.join("target").join("cdylib-under-test");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    // Build the cdylib with the same optimisation profile as the test binary, so
    // that `cargo test --release` really exercises the release artifact
    // (which, per Cargo.toml, is built with `panic = "abort"`).
    let release = !cfg!(debug_assertions);

    let mut cmd = Command::new(&cargo);
    cmd.current_dir(manifest).args(["build", "--lib"]);
    if release {
        cmd.arg("--release");
    }
    cmd.arg("--target-dir").arg(&target_dir);
    let status = cmd.status();

    if let Ok(st) = status {
        if st.success() {
            let profile_subdir = if release { "release" } else { "debug" };
            let expected = target_dir.join(profile_subdir).join(SO_NAME);
            if expected.exists() {
                return expected;
            }
            // Custom profile name: fall back to scanning the target dir.
            if let Ok(entries) = std::fs::read_dir(&target_dir) {
                let mut hits: Vec<PathBuf> = entries
                    .flatten()
                    .map(|e| e.path().join(SO_NAME))
                    .filter(|p| p.exists())
                    .collect();
                hits.sort();
                if let Some(p) = hits.into_iter().next() {
                    return p;
                }
            }
        }
    }

    // Fall back to an artifact left behind by a plain `cargo build`.
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile_dir = deps.parent().expect("profile dir");
    for c in [profile_dir.join(SO_NAME), deps.join(SO_NAME)] {
        if c.exists() {
            return c;
        }
    }
    if let Some(p) = find_so(profile_dir) {
        return p;
    }
    panic!(
        "could not build or locate {SO_NAME} (looked under {} and {})",
        target_dir.display(),
        profile_dir.display()
    )
}

pub struct Impls {
    _c_lib: Library,
    _rust_lib: Library,
    pub c_bin2hex: Bin2HexFn,
    pub rust_bin2hex: Bin2HexFn,
}

impl Impls {
    pub fn load() -> Impls {
        unsafe {
            let c_lib = Library::new(c_so_path()).expect("load C .so");
            let rust_lib = Library::new(rust_so_path()).expect("load Rust .so");
            let c_sym: Symbol<Bin2HexFn> = c_lib.get(b"bin2hex\0").expect("C bin2hex");
            let r_sym: Symbol<Bin2HexFn> = rust_lib.get(b"bin2hex\0").expect("Rust bin2hex");
            let c_bin2hex = *c_sym;
            let rust_bin2hex = *r_sym;
            Impls {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c_bin2hex,
                rust_bin2hex,
            }
        }
    }
}

/// Output of one `bin2hex` invocation: the full buffer contents plus whether the
/// returned pointer equalled the buffer that was passed in.
#[derive(Debug, PartialEq, Eq)]
pub struct Outcome {
    pub buf: Vec<u8>,
    pub returned_input_ptr: bool,
}

/// Call `f` with a freshly poisoned buffer of `hex_maxlen` bytes.
pub fn run(f: Bin2HexFn, hex_maxlen: usize, bin: &[u8], fill: u8) -> Outcome {
    let mut buf: Vec<u8> = vec![fill; hex_maxlen];
    let ptr = buf.as_mut_ptr() as *mut i8;
    let ret = unsafe { f(ptr, hex_maxlen, bin.as_ptr(), bin.len()) };
    Outcome {
        buf,
        returned_input_ptr: ret == ptr,
    }
}
