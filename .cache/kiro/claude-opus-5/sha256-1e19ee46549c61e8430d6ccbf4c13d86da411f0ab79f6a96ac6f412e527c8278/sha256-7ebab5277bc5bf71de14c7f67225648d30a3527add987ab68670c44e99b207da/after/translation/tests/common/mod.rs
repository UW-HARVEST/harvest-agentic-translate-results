#![allow(dead_code)]

//! Shared harness: loads the C reference `.so` and the Rust `cdylib` and
//! exposes both `div_euclid` implementations through the FFI boundary only.

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

pub type DivEuclidFn = unsafe extern "C" fn(i32, i32) -> i32;

/// Root of the repository (parent of `translation/`).
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Locates the C shared library produced by the CMake build.
pub fn c_library_path() -> PathBuf {
    let build_dir = repo_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&build_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let is_so = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e == "so")
                .unwrap_or(false);
            if is_so {
                candidates.push(path);
            }
        }
    }

    candidates.sort();
    candidates.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no C .so found in {}; build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build_dir.display()
        )
    })
}

/// Locates the Rust `cdylib`, building it if cargo has not produced it yet.
///
/// `cargo test` does not emit the `cdylib` artifact for this crate (the test
/// harnesses link the lib as an rlib), so the release artifact is built on
/// demand. That release artifact is also the one an external caller would
/// actually load, `panic = "abort"` and all.
///
/// `DIV_EUCLID_RUST_SO` overrides the choice, which lets the same tests run
/// against an alternately-configured build of the same crate.
pub fn rust_library_path() -> PathBuf {
    if let Some(path) = std::env::var_os("DIV_EUCLID_RUST_SO") {
        let path = PathBuf::from(path);
        assert!(
            path.is_file(),
            "DIV_EUCLID_RUST_SO points at {}, which is not a file",
            path.display()
        );
        return path;
    }

    let manifest = repo_root().join("translation");
    let file = "libdiv_euclid_lib.so";
    let release = manifest.join("target").join("release").join(file);

    if !release.is_file() {
        let status = std::process::Command::new(env!("CARGO"))
            .args(["build", "--release"])
            .current_dir(&manifest)
            .status()
            .expect("failed to spawn cargo to build the Rust cdylib");
        assert!(status.success(), "`cargo build --release` failed");
    }

    assert!(
        release.is_file(),
        "expected the Rust cdylib at {}",
        release.display()
    );
    release
}

/// Both implementations, kept alive alongside the libraries that own them.
pub struct Harness {
    _c_lib: Library,
    _rust_lib: Library,
    c: DivEuclidFn,
    rust: DivEuclidFn,
}

impl Harness {
    pub fn load() -> Self {
        unsafe {
            let c_path = c_library_path();
            let rust_path = rust_library_path();

            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("failed to load {}: {e}", c_path.display()));
            let rust_lib = Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("failed to load {}: {e}", rust_path.display()));

            let c_sym: Symbol<DivEuclidFn> = c_lib
                .get(b"div_euclid\0")
                .expect("C .so does not export div_euclid");
            let rust_sym: Symbol<DivEuclidFn> = rust_lib
                .get(b"div_euclid\0")
                .expect("Rust .so does not export div_euclid");

            let c = *c_sym;
            let rust = *rust_sym;

            Harness {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rust,
            }
        }
    }

    pub fn c(&self, v1: i32, v2: i32) -> i32 {
        unsafe { (self.c)(v1, v2) }
    }

    pub fn rust(&self, v1: i32, v2: i32) -> i32 {
        unsafe { (self.rust)(v1, v2) }
    }

    /// Asserts byte-identical results for one input pair.
    #[track_caller]
    pub fn assert_match(&self, v1: i32, v2: i32) {
        let expected = self.c(v1, v2);
        let actual = self.rust(v1, v2);
        assert_eq!(
            expected.to_ne_bytes(),
            actual.to_ne_bytes(),
            "div_euclid({v1}, {v2}): C returned {expected}, Rust returned {actual}"
        );
    }
}

/// Values that sit on every boundary the C source branches on, plus the
/// neighbourhood of each so off-by-one differences surface.
pub fn edge_values() -> Vec<i32> {
    let mut v = vec![
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 2,
        i32::MIN / 2,
        i32::MIN / 2 - 1,
        i32::MIN / 2 + 1,
        -0x7fff_ffff,
        -1_000_000_007,
        -65_537,
        -65_536,
        -65_535,
        -256,
        -100,
        -7,
        -6,
        -5,
        -4,
        -3,
        -2,
        -1,
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        100,
        256,
        65_535,
        65_536,
        65_537,
        1_000_000_007,
        i32::MAX / 2 - 1,
        i32::MAX / 2,
        i32::MAX / 2 + 1,
        i32::MAX - 2,
        i32::MAX - 1,
        i32::MAX,
    ];

    // Powers of two and their neighbours, both signs.
    for shift in 0..31u32 {
        let p = 1i32 << shift;
        v.push(p);
        v.push(-p);
        if p != i32::MAX {
            v.push(p.wrapping_add(1));
            v.push(-p.wrapping_add(1));
        }
        v.push(p.wrapping_sub(1));
        v.push(-(p.wrapping_sub(1)));
    }

    v.sort_unstable();
    v.dedup();
    v
}

/// Deterministic xorshift64* so test runs are reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }

    /// Biased towards small magnitudes, where most divisor branches live.
    pub fn next_small_i32(&mut self) -> i32 {
        let r = self.next_u64();
        let magnitude = 1u64 << (r % 32);
        let value = (r >> 8) % magnitude.max(1);
        if r & (1 << 63) != 0 {
            -(value as i64) as i32
        } else {
            value as i32
        }
    }
}
