//! Shared harness: locates and loads both the C and the Rust shared libraries
//! and calls `tool_basename` strictly through the FFI boundary.

// Each integration-test binary includes this module but uses only part of it.
#![allow(dead_code)]

use std::ffi::c_char;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use libloading::{Library, Symbol};

/// Signature of the exported function under test.
pub type ToolBasenameFn = unsafe extern "C" fn(*mut c_char) -> *mut c_char;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workspace_root() -> PathBuf {
    manifest_dir()
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn dylib_name(stem: &str) -> String {
    if cfg!(target_os = "macos") {
        format!("lib{stem}.dylib")
    } else if cfg!(target_os = "windows") {
        format!("{stem}.dll")
    } else {
        format!("lib{stem}.so")
    }
}

/// Path to the C shared library, building it with CMake on first use.
pub fn c_library_path() -> PathBuf {
    let build_dir = workspace_root().join("c_src").join("build");
    let candidate = build_dir.join(dylib_name("driver"));
    if candidate.is_file() {
        return candidate;
    }

    std::fs::create_dir_all(&build_dir).expect("create c_src/build");
    let configure = Command::new("cmake")
        .arg("..")
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

    assert!(candidate.is_file(), "C library missing at {candidate:?}");
    candidate
}

/// Path to the Rust `cdylib` produced for the current test profile.
pub fn rust_library_path() -> PathBuf {
    let name = dylib_name("driver");

    // The test binary lives in `<target>/<profile>/deps/`, so the cdylib sits
    // one directory up. Walk up from the current executable to stay correct
    // for any `--target-dir`, profile or `--target` selection.
    if let Ok(exe) = std::env::current_exe() {
        let mut dir: Option<&Path> = exe.parent();
        while let Some(d) = dir {
            let candidate = d.join(&name);
            if candidate.is_file() {
                return candidate;
            }
            dir = d.parent();
        }
    }

    for profile in ["debug", "release"] {
        let candidate = manifest_dir().join("target").join(profile).join(&name);
        if candidate.is_file() {
            return candidate;
        }
    }

    panic!(
        "could not locate the Rust cdylib ({name}); build it with `cargo build` \
         before running the tests"
    );
}

/// Both libraries kept alive for the whole test-process lifetime so that the
/// function pointers taken out of them remain valid.
pub struct Libs {
    _c: Library,
    _rust: Library,
    pub c_tool_basename: ToolBasenameFn,
    pub rust_tool_basename: ToolBasenameFn,
}

// SAFETY: the loaded libraries are immutable code images and the entry point
// under test is re-entrant, so sharing the handles across threads is sound.
unsafe impl Sync for Libs {}
unsafe impl Send for Libs {}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = c_library_path();
        let rust_path = rust_library_path();

        // SAFETY: both paths refer to shared libraries we just built; loading
        // them runs their (empty) initialisers only.
        let c = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("failed to load C library {c_path:?}: {e}"));
        let rust = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|e| panic!("failed to load Rust library {rust_path:?}: {e}"));

        // SAFETY: the symbol has the C signature declared by `ToolBasenameFn`.
        let c_sym: Symbol<ToolBasenameFn> = unsafe { c.get(b"tool_basename\0") }
            .expect("C library must export `tool_basename`");
        let rust_sym: Symbol<ToolBasenameFn> = unsafe { rust.get(b"tool_basename\0") }
            .expect("Rust library must export `tool_basename`");

        let c_tool_basename = *c_sym;
        let rust_tool_basename = *rust_sym;

        Libs {
            _c: c,
            _rust: rust,
            c_tool_basename,
            rust_tool_basename,
        }
    })
}

/// Result of one call, expressed in a way that is comparable between the two
/// libraries: the byte offset of the returned pointer into the input buffer
/// plus the bytes of the returned NUL-terminated string.
#[derive(Debug, PartialEq, Eq)]
pub struct CallResult {
    pub offset: isize,
    pub tail: Vec<u8>,
    /// Buffer contents after the call, to prove the input is left untouched.
    pub buffer_after: Vec<u8>,
}

/// Calls `f` with a fresh, private copy of `input` (NUL terminator appended).
///
/// `input` must not contain an interior NUL byte, because the C function's
/// contract is a NUL-terminated string.
pub fn call(f: ToolBasenameFn, input: &[u8]) -> CallResult {
    let mut buf: Vec<u8> = Vec::with_capacity(input.len() + 1);
    buf.extend_from_slice(input);
    buf.push(0);

    let base = buf.as_mut_ptr() as *mut c_char;
    // SAFETY: `base` points at a NUL-terminated buffer that outlives the call.
    let ret = unsafe { f(base) };
    assert!(!ret.is_null(), "tool_basename must never return null");

    // SAFETY: the C code only ever returns `path` or a pointer one past a
    // separator inside the same buffer, so the offset is in bounds.
    let offset = unsafe { ret.offset_from(base) };
    assert!(
        offset >= 0 && (offset as usize) <= input.len(),
        "returned pointer {offset} is outside the input buffer of len {}",
        input.len()
    );

    let tail = buf[offset as usize..buf.len() - 1].to_vec();

    CallResult {
        offset,
        tail,
        buffer_after: buf,
    }
}

/// Asserts the C and Rust implementations agree byte-for-byte on `input`.
pub fn assert_same(input: &[u8]) {
    let l = libs();
    let c = call(l.c_tool_basename, input);
    let r = call(l.rust_tool_basename, input);
    assert_eq!(
        c, r,
        "mismatch for input {:?} (lossy: {:?})",
        input,
        String::from_utf8_lossy(input)
    );
}
