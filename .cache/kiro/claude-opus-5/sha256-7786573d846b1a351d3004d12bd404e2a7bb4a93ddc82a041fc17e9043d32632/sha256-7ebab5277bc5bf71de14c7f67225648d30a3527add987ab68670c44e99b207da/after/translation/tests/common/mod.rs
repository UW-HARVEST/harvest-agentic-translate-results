//! Shared harness: locate and dynamically load both the C and the Rust shared
//! libraries, and call `tritanopia` through their exported symbols only.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

/// Byte-for-byte mirror of `cb_rgb_255` from `c_src/include/lib.h`.
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct CbRgb255 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

type TritanopiaFn = unsafe extern "C" fn(CbRgb255) -> CbRgb255;

/// Repository root (the directory holding `c_src/` and `translation/`).
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// The C shared library is named after the repository directory, so glob for
/// whatever `.so` CMake produced in `c_src/build`.
fn c_library_path() -> PathBuf {
    let build_dir = repo_root().join("c_src/build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build_dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C library first.", build_dir.display()))
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().map(|e| e == "so").unwrap_or(false)
                && path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    candidates.sort();
    candidates
        .pop()
        .unwrap_or_else(|| panic!("no lib*.so found in {}", build_dir.display()))
}

/// The Rust `cdylib`. The integration-test binary lives in
/// `target/<profile>/deps/`, so the `.so` sits two directories up.
fn rust_library_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let deps_dir = exe.parent().expect("deps dir");
    let candidates = [
        deps_dir.join("libtritanopia_lib.so"),
        deps_dir
            .parent()
            .expect("profile dir")
            .join("libtritanopia_lib.so"),
    ];
    for candidate in &candidates {
        if candidate.exists() {
            return candidate.clone();
        }
    }
    panic!(
        "libtritanopia_lib.so not found; looked in {:?}",
        candidates
    );
}

/// A loaded implementation, kept alive alongside the symbol it vends.
pub struct Impl {
    _lib: Library,
    tritanopia: TritanopiaFn,
    pub label: &'static str,
}

impl Impl {
    fn load(path: &Path, label: &'static str) -> Impl {
        // SAFETY: both libraries are plain leaf libraries with no initialisers
        // that run arbitrary code beyond the usual CRT setup.
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to load {}: {e}", path.display()));
        let tritanopia = {
            let sym: Symbol<TritanopiaFn> = unsafe { lib.get(b"tritanopia\0") }
                .unwrap_or_else(|e| panic!("`tritanopia` missing from {}: {e}", path.display()));
            *sym
        };
        Impl {
            _lib: lib,
            tritanopia,
            label,
        }
    }

    #[inline]
    pub fn tritanopia(&self, rgb: CbRgb255) -> CbRgb255 {
        unsafe { (self.tritanopia)(rgb) }
    }
}

/// Load the C library (ground truth) and the Rust library (under test).
pub fn load_pair() -> (Impl, Impl) {
    (
        Impl::load(&c_library_path(), "c"),
        Impl::load(&rust_library_path(), "rust"),
    )
}
