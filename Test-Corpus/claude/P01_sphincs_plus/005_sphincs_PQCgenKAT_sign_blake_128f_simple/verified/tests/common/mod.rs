// Common helpers for the FFI integration tests.
//
// Each test loads BOTH the C-built `libsphincs_core.so` and the Rust-built
// `libsphincs_plus.so` via libloading, then drives them through their
// public C-ABI exports and asserts byte-identical results.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;

pub struct Libs {
    /// The hash backend (blake/haraka/sha2/shake) library, kept alive so its
    /// symbols remain resolvable for the core .so via lazy resolution.
    pub _hash: Library,
    pub c: Library,
    pub r: Library,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path to the C `libsphincs_core.so` (random source = /dev/urandom variant).
pub fn c_so_path() -> PathBuf {
    manifest_dir()
        .join("c_src")
        .join("build")
        .join("app")
        .join("libsphincs_core.so")
}

/// Path to the hash backend library. We pick whichever one was built
/// (only one will exist at a time).
pub fn hash_so_path() -> PathBuf {
    let backends = ["blake", "haraka", "sha2", "shake"];
    let lib_dir = manifest_dir().join("c_src").join("build").join("lib");
    for b in backends {
        let p = lib_dir.join(b).join(format!("lib{}.so", b));
        if p.exists() {
            return p;
        }
    }
    panic!("no hash backend .so found in {:?}", lib_dir);
}

/// Path to the Rust `libsphincs_plus.so`.
pub fn rust_so_path() -> PathBuf {
    // Cargo will set OUT_DIR; we just look at target/release.
    let mut p = manifest_dir();
    p.push("target");
    p.push("release");
    p.push("libsphincs_plus.so");
    p
}

pub fn open_libs() -> Libs {
    unsafe {
        // The C build splits sphincs_core and the hash backend across two
        // shared libraries that have a circular dependency on each other
        // (core calls hash, hash calls a few SPX_* helpers from core).
        // Neither lib lists the other in DT_NEEDED, so we have to dlopen
        // both with RTLD_GLOBAL with deferred symbol resolution (RTLD_LAZY)
        // so all the cross-references resolve once both are mapped in.
        let c = open_global_lazy(&c_so_path());
        let hash = open_global_lazy(&hash_so_path());
        let r = open_global_lazy(&rust_so_path());
        Libs { _hash: hash, c, r }
    }
}

#[cfg(unix)]
unsafe fn open_global_lazy(path: &std::path::Path) -> Library {
    use libloading::os::unix::Library as UnixLib;
    let lib = UnixLib::open(
        Some(path),
        libloading::os::unix::RTLD_LAZY | libloading::os::unix::RTLD_GLOBAL,
    )
    .unwrap_or_else(|e| panic!("failed to open .so {:?}: {}", path, e));
    Library::from(lib)
}

#[cfg(not(unix))]
unsafe fn open_global_lazy(path: &std::path::Path) -> Library {
    Library::new(path).unwrap()
}

/// Look up a symbol by name in both libs.
pub unsafe fn sym<'a, T: Copy>(lib: &'a Library, name: &[u8]) -> Symbol<'a, T> {
    lib.get(name)
        .unwrap_or_else(|e| panic!("missing symbol {}: {}", String::from_utf8_lossy(name), e))
}
