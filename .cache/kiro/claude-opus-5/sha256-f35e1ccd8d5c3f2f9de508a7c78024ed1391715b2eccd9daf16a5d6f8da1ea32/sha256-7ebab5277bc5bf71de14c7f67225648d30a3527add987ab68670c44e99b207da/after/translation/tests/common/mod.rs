//! Shared helpers: locate and load the C and Rust shared libraries.
//!
//! Each integration-test binary pulls in this module, so some items are unused
//! in some of them.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

/// Mirror of the C `cn_rnd_t`.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CnRnd {
    pub state: [u64; 2],
}

pub type NextDouble = unsafe extern "C" fn(*mut CnRnd) -> f64;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation dir has a parent")
        .to_path_buf()
}

/// Find the C shared library produced by CMake in `c_src/build`.
pub fn c_lib_path() -> PathBuf {
    let build = workspace_root().join("c_src").join("build");
    let mut found: Vec<PathBuf> = Vec::new();
    collect_so(&build, &mut found);
    assert!(
        !found.is_empty(),
        "no .so found under {}; build the C library first:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build.display()
    );
    // Prefer a top-level artifact (shortest path).
    found.sort_by_key(|p| p.components().count());
    found.remove(0)
}

fn collect_so(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_so(&p, out);
        } else if p.extension().and_then(|s| s.to_str()) == Some("so") {
            out.push(p);
        }
    }
}

/// Find the Rust cdylib. Located next to the test executable (target/<profile>/).
pub fn rust_lib_path() -> PathBuf {
    const NAME: &str = "libnext_double_lib.so";

    // The test binary lives in target/<profile>/deps/<name>-<hash>.
    if let Ok(exe) = std::env::current_exe() {
        for dir in exe.ancestors().skip(1).take(3) {
            let cand = dir.join(NAME);
            if cand.is_file() {
                return cand;
            }
        }
    }

    let target = workspace_root().join("translation").join("target");
    for profile in ["debug", "release"] {
        let cand = target.join(profile).join(NAME);
        if cand.is_file() {
            return cand;
        }
    }
    panic!(
        "could not locate {NAME}; run `cargo build` in translation/ first (searched {})",
        target.display()
    );
}

pub struct Libs {
    // Keep the libraries alive for the lifetime of the symbols.
    _c: libloading::Library,
    _rust: libloading::Library,
    pub c_next_double: NextDouble,
    pub rust_next_double: NextDouble,
}

impl Libs {
    pub fn load() -> Libs {
        unsafe {
            let c = libloading::Library::new(c_lib_path()).expect("load C .so");
            let rust = libloading::Library::new(rust_lib_path()).expect("load Rust .so");

            let c_sym: libloading::Symbol<NextDouble> = c
                .get(b"next_double\0")
                .expect("C .so exports next_double");
            let r_sym: libloading::Symbol<NextDouble> = rust
                .get(b"next_double\0")
                .expect("Rust .so exports next_double");

            let c_next_double = *c_sym;
            let rust_next_double = *r_sym;

            Libs {
                _c: c,
                _rust: rust,
                c_next_double,
                rust_next_double,
            }
        }
    }
}
