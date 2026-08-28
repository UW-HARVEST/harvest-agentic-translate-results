//! Shared harness: loads the C shared library and the Rust `cdylib` side by
//! side with `libloading` and exposes their exported symbols behind an
//! identical interface, so every test drives both implementations purely
//! through the FFI boundary.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, MutexGuard, OnceLock};

/// Mirrors the C `DynamicArray` (and the Rust `#[repr(C)]` translation).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DynamicArray {
    pub data: *mut c_int,
    pub size: usize,
    pub capacity: usize,
}

pub type MatrixT = [[c_int; 4]; 3];

/// Every symbol the libraries export, resolved once per library.
pub struct Api {
    pub name: &'static str,
    pub init_array: unsafe extern "C" fn(usize) -> *mut DynamicArray,
    pub expand_array: unsafe extern "C" fn(*mut DynamicArray) -> c_int,
    pub add_element: unsafe extern "C" fn(*mut DynamicArray, c_int) -> c_int,
    pub free_array: unsafe extern "C" fn(*mut DynamicArray),
    pub process_flags: unsafe extern "C" fn(c_int) -> c_int,
    pub calculate_matrix_checksum: unsafe extern "C" fn() -> c_int,
    pub matrixsum: unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int,
    /// The exported `int matrix[3][4]` global.
    pub matrix: *mut MatrixT,
}

// The underlying `Library` handles are deliberately leaked and stay loaded for
// the whole process, so the resolved pointers remain valid.
unsafe impl Send for Api {}
unsafe impl Sync for Api {}

impl Api {
    fn load(name: &'static str, path: &Path) -> Api {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", path.display(), name));
        // Leak so the symbols below outlive this scope.
        let lib: &'static Library = Box::leak(Box::new(lib));

        unsafe fn sym<T: Copy>(lib: &'static Library, name: &[u8]) -> T {
            let s: Symbol<T> = unsafe { lib.get(name) }.unwrap_or_else(|e| {
                panic!("missing symbol {}: {e}", String::from_utf8_lossy(name))
            });
            *s
        }

        unsafe {
            Api {
                name,
                init_array: sym(lib, b"init_array\0"),
                expand_array: sym(lib, b"expand_array\0"),
                add_element: sym(lib, b"add_element\0"),
                free_array: sym(lib, b"free_array\0"),
                process_flags: sym(lib, b"process_flags\0"),
                calculate_matrix_checksum: sym(lib, b"calculate_matrix_checksum\0"),
                matrixsum: sym(lib, b"matrixsum\0"),
                matrix: sym::<*mut MatrixT>(lib, b"matrix\0"),
            }
        }
    }

    /// Snapshot of the `matrix` global.
    pub fn read_matrix(&self) -> MatrixT {
        unsafe { *self.matrix }
    }

    pub fn write_matrix(&self, m: &MatrixT) {
        unsafe { *self.matrix = *m };
    }

    /// Reads back the fields of a `DynamicArray` the library allocated.
    pub fn read_header(&self, p: *mut DynamicArray) -> DynamicArray {
        assert!(!p.is_null(), "{}: null DynamicArray", self.name);
        unsafe { *p }
    }

    /// Reads the first `n` elements of the array's backing buffer.
    pub fn read_data(&self, p: *mut DynamicArray, n: usize) -> Vec<c_int> {
        let h = self.read_header(p);
        assert!(!h.data.is_null(), "{}: null data pointer", self.name);
        (0..n).map(|i| unsafe { *h.data.add(i) }).collect()
    }
}

fn workspace_root() -> PathBuf {
    // .../<root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

/// The C library built by `c_src/CMakeLists.txt`; its file name follows the
/// enclosing directory, so it is discovered rather than hard-coded.
pub fn c_library_path() -> PathBuf {
    let build = workspace_root().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {} ({e}); build the C library first:\n  cd c_src && mkdir -p build \
                 && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build.display()
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "so"))
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one .so in {}, found {found:?}",
        build.display()
    );
    found.pop().unwrap()
}

/// The Rust `cdylib`, located next to the running test binary's target dir so
/// that debug and release profiles both work.
///
/// Cargo does not build a `cdylib`-only lib target as part of `cargo test`
/// (there is nothing for the test binaries to link against), so if the shared
/// object is absent it is built here on demand. The nested build gets its own
/// target directory, because the outer `cargo test` still holds the lock on the
/// primary one.
pub fn rust_library_path() -> PathBuf {
    const LIB_FILE: &str = "libmatrixsum_lib.so";

    let exe = std::env::current_exe().expect("current_exe");
    // target/<profile>/deps/<test binary>
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("test binary lives in target/<profile>/deps");

    let path = profile_dir.join(LIB_FILE);
    if path.exists() {
        return path;
    }

    let release = profile_dir.file_name().is_some_and(|n| n == "release");
    let nested_target = profile_dir.join("cdylib-for-tests");
    let built = nested_target
        .join(if release { "release" } else { "debug" })
        .join(LIB_FILE);

    let mut cmd = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    cmd.arg("build")
        .arg("--quiet")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("CARGO_TARGET_DIR", &nested_target);
    if release {
        cmd.arg("--release");
    }
    // Reproduce the feature selection this test binary was compiled with.
    cmd.arg("--no-default-features");
    let features = enabled_features();
    if !features.is_empty() {
        cmd.arg("--features").arg(features.join(","));
    }

    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn cargo to build the cdylib: {e}"));
    assert!(
        out.status.success(),
        "could not build the Rust cdylib:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        built.exists(),
        "cargo reported success but {} is missing",
        built.display()
    );
    built
}

/// The crate features active for this test build. Resolved at compile time from
/// the same feature set `cargo test` was invoked with, so the on-demand cdylib
/// build stays in sync. The crate declares no features today; any new one must
/// be listed here.
fn enabled_features() -> Vec<&'static str> {
    Vec::new()
}

static C_API: OnceLock<Api> = OnceLock::new();
static RUST_API: OnceLock<Api> = OnceLock::new();

/// The `matrix` global is process-wide shared state, but tests inside one binary
/// run concurrently. Any test that writes `matrix` (or reads a checksum derived
/// from it) must hold this guard.
static MATRIX_LOCK: Mutex<()> = Mutex::new(());

pub fn matrix_guard() -> MutexGuard<'static, ()> {
    // A panicking test poisons the lock; the data is plain memory that the next
    // test overwrites anyway, so recover rather than cascade failures.
    MATRIX_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn c() -> &'static Api {
    C_API.get_or_init(|| Api::load("C", &c_library_path()))
}

pub fn rust() -> &'static Api {
    RUST_API.get_or_init(|| Api::load("Rust", &rust_library_path()))
}

/// Convenience for tests that always want both.
pub fn both() -> (&'static Api, &'static Api) {
    (c(), rust())
}

/// The interesting `int` edge values, reused across tests.
pub const INT_PROBES: &[c_int] = &[
    0,
    1,
    -1,
    2,
    -2,
    3,
    7,
    8,
    15,
    16,
    -16,
    255,
    256,
    -255,
    0x0F,
    0x10,
    0xFF,
    0x100,
    1000,
    -1000,
    65535,
    -65536,
    0x7FFF_FFFE,
    c_int::MAX,
    c_int::MIN,
    c_int::MIN + 1,
    -0x4000_0000,
    0x4000_0000,
];
