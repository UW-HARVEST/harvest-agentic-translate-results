//! Shared harness: loads the C shared library and the Rust cdylib side by side
//! and exposes both through `libloading` so that every call in every test goes
//! across the real FFI boundary.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// FFI signatures of the public C API (see c_src/src/lib.c)
// ---------------------------------------------------------------------------
pub type OperationFunc = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
pub type ModifierFunc = unsafe extern "C" fn(c_int, c_int);

pub type FnApplyOperation = unsafe extern "C" fn(OperationFunc, c_int, c_int, c_int) -> c_int;
pub type FnTernary = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
pub type FnShiftArray = unsafe extern "C" fn(*mut c_int, c_int, c_int);
pub type FnProcessPtr = unsafe extern "C" fn(*const c_int, c_int) -> c_int;
pub type FnBinary = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type FnUnary = unsafe extern "C" fn(c_int) -> c_int;
pub type FnManipulateRecords = unsafe extern "C" fn(*mut DataRecord, c_int, c_int) -> c_int;
pub type FnHatch = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// Mirrors the (private) `DataRecord` typedef in the C translation unit.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DataRecord {
    pub id: c_int,
    pub value: c_int,
    pub timestamp: i64,
    pub name: [c_char; 32],
}

impl DataRecord {
    pub fn new(id: c_int, value: c_int) -> Self {
        DataRecord {
            id,
            value,
            timestamp: 0,
            name: [0; 32],
        }
    }
}

// ---------------------------------------------------------------------------
// Library discovery
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // .../<root>/translation  ->  .../<root>
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

fn find_so(dir: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut found: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("so"))
        .collect();
    found.sort();
    found.into_iter().next()
}

/// Path to the C shared library produced by `c_src/build`.
pub fn c_lib_path() -> PathBuf {
    let build = workspace_root().join("c_src").join("build");
    find_so(&build).unwrap_or_else(|| {
        panic!(
            "no .so found in {}. Build the C library first:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

/// Path to the Rust cdylib. `HATCH_RUST_SO` overrides; otherwise prefers
/// `release` and falls back to `debug`.
pub fn rust_lib_path() -> PathBuf {
    if let Some(p) = std::env::var_os("HATCH_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "HATCH_RUST_SO={} does not exist", p.display());
        return p;
    }
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    for profile in ["release", "debug"] {
        let candidate = target.join(profile).join("libhatch_lib.so");
        if candidate.exists() {
            return candidate;
        }
    }
    panic!(
        "libhatch_lib.so not found under {}. Build it first: cargo build --release",
        target.display()
    );
}

/// Both implementations, loaded as independent shared objects.
pub struct Libs {
    pub c: Library,
    pub rust: Library,
}

pub fn load() -> Libs {
    // RTLD_LOCAL (libloading's default) keeps the two symbol namespaces apart,
    // so `c.get("hatch")` and `rust.get("hatch")` really are distinct code.
    let c = unsafe { Library::new(c_lib_path()) }.expect("failed to dlopen the C library");
    let rust = unsafe { Library::new(rust_lib_path()) }.expect("failed to dlopen the Rust cdylib");
    Libs { c, rust }
}

impl Libs {
    /// Fetch the same symbol from both libraries.
    pub fn pair<'a, T>(&'a self, name: &str) -> (Symbol<'a, T>, Symbol<'a, T>) {
        let mut sym = name.as_bytes().to_vec();
        sym.push(0);
        let c = unsafe { self.c.get::<T>(&sym) }
            .unwrap_or_else(|e| panic!("C library is missing symbol `{name}`: {e}"));
        let r = unsafe { self.rust.get::<T>(&sym) }
            .unwrap_or_else(|e| panic!("Rust library is missing symbol `{name}`: {e}"));
        (c, r)
    }
}

/// Interesting `int` values: identities, small magnitudes, and the boundaries
/// where 32-bit wrapping behaviour shows up.
pub const INTS: &[c_int] = &[
    0,
    1,
    -1,
    2,
    -2,
    3,
    7,
    -7,
    10,
    -10,
    17,
    100,
    -100,
    255,
    256,
    1000,
    -1000,
    65535,
    65536,
    -65536,
    1_000_000,
    -1_000_000,
    46341,
    -46341,
    i32::MAX,
    i32::MIN,
    i32::MAX - 1,
    i32::MIN + 1,
    0x5555_5555,
    -0x5555_5555,
];
