//! Shared harness: loads the C reference `.so` and the Rust `.so` and exposes
//! their exported symbols through identical FFI signatures.
//!
//! Nothing here calls the Rust crate directly -- every call goes through
//! `libloading`, so the `#[no_mangle]` wrappers are part of what is tested.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

/// `double spectral_contrast(float_t *a, float_t *b, int length)`
///
/// `spectral_contrast.c` never includes `match.h`, so its `float_t` is
/// `<math.h>`'s, i.e. `float` on x86-64 Linux. The signature below is the one
/// the object code actually implements.
pub type SpectralContrastFn =
    unsafe extern "C" fn(*mut core::ffi::c_float, *mut core::ffi::c_float, core::ffi::c_int) -> f64;

/// `int match(float_t *test, float_t *reference, int bins, double threshold)`
///
/// `match.c` includes `match.h`, where `float_t` is `double`.
pub type MatchFn = unsafe extern "C" fn(
    *mut core::ffi::c_double,
    *mut core::ffi::c_double,
    core::ffi::c_int,
    f64,
) -> core::ffi::c_int;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_so(dir: &Path, prefer: Option<&str>) -> Option<PathBuf> {
    let mut hits: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension().and_then(|s| s.to_str()) == Some("so")
                && p.file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|n| n.starts_with("lib"))
        })
        .collect();
    hits.sort();
    if let Some(stem) = prefer
        && let Some(exact) = hits.iter().find(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|n| n.contains(stem))
        })
    {
        return Some(exact.clone());
    }
    hits.pop()
}

/// Locates `c_src/build/lib<project>.so`, produced by the CMake build.
pub fn c_library_path() -> PathBuf {
    let build = manifest_dir().join("../c_src/build");
    find_so(&build, None).unwrap_or_else(|| {
        panic!(
            "no C shared library under {}; build it with\n  cd c_src && mkdir -p build && cd build \
             && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

/// Locates every Rust `cdylib` that has been built (debug and/or release), so
/// the comparison runs against each profile that exists.
pub fn rust_library_paths() -> Vec<PathBuf> {
    if let Ok(explicit) = std::env::var("RUST_SO_PATH") {
        return vec![PathBuf::from(explicit)];
    }
    let root = manifest_dir().join("target");
    let mut found = Vec::new();
    for profile in ["debug", "release"] {
        if let Some(p) = find_so(&root.join(profile), Some("underhanded")) {
            found.push(p);
        }
    }
    assert!(
        !found.is_empty(),
        "no Rust cdylib under {}; run `cargo build`",
        root.display()
    );
    found
}

/// The C reference library plus every built Rust library, each paired with a
/// human-readable label used in assertion messages.
pub struct Pair {
    pub c: Library,
    pub rust: Vec<(String, Library)>,
}

impl Pair {
    pub fn load() -> Self {
        let c_path = c_library_path();
        // SAFETY: both libraries are plain C-ABI shared objects built from this
        // repository; loading them runs no initializers with side effects.
        let c = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("loading {}: {e}", c_path.display()));
        let rust = rust_library_paths()
            .into_iter()
            .map(|p| {
                let label = p
                    .parent()
                    .and_then(|d| d.file_name())
                    .and_then(|s| s.to_str())
                    .unwrap_or("rust")
                    .to_string();
                let lib = unsafe { Library::new(&p) }
                    .unwrap_or_else(|e| panic!("loading {}: {e}", p.display()));
                (label, lib)
            })
            .collect();
        Self { c, rust }
    }

    fn sym<'a, T>(lib: &'a Library, name: &[u8]) -> Symbol<'a, T> {
        unsafe { lib.get(name) }.unwrap_or_else(|e| {
            panic!(
                "missing export `{}`: {e}",
                String::from_utf8_lossy(&name[..name.len() - 1])
            )
        })
    }

    pub fn c_spectral_contrast(&self) -> Symbol<'_, SpectralContrastFn> {
        Self::sym(&self.c, b"spectral_contrast\0")
    }

    pub fn c_match(&self) -> Symbol<'_, MatchFn> {
        Self::sym(&self.c, b"match\0")
    }

    pub fn rust_spectral_contrast(&self) -> Vec<(&str, Symbol<'_, SpectralContrastFn>)> {
        self.rust
            .iter()
            .map(|(l, lib)| (l.as_str(), Self::sym(lib, b"spectral_contrast\0")))
            .collect()
    }

    pub fn rust_match(&self) -> Vec<(&str, Symbol<'_, MatchFn>)> {
        self.rust
            .iter()
            .map(|(l, lib)| (l.as_str(), Self::sym(lib, b"match\0")))
            .collect()
    }
}

/// Bit-exact comparison of a `double`, so that `NaN`s and signed zeros are
/// distinguished (this is a byte-for-byte check, not a numeric one).
#[track_caller]
pub fn assert_f64_bits_eq(c: f64, rust: f64, what: &str) {
    assert_eq!(
        c.to_bits(),
        rust.to_bits(),
        "{what}: C returned {c:?} (bits {:#018x}), Rust returned {rust:?} (bits {:#018x})",
        c.to_bits(),
        rust.to_bits()
    );
}

#[track_caller]
pub fn assert_f32_slice_bits_eq(c: &[f32], rust: &[f32], what: &str) {
    assert_eq!(c.len(), rust.len(), "{what}: length mismatch");
    for (i, (a, b)) in c.iter().zip(rust).enumerate() {
        assert_eq!(
            a.to_bits(),
            b.to_bits(),
            "{what}: element {i} differs -- C {a:?} ({:#010x}) vs Rust {b:?} ({:#010x})",
            a.to_bits(),
            b.to_bits()
        );
    }
}

#[track_caller]
pub fn assert_f64_slice_bits_eq(c: &[f64], rust: &[f64], what: &str) {
    assert_eq!(c.len(), rust.len(), "{what}: length mismatch");
    for (i, (a, b)) in c.iter().zip(rust).enumerate() {
        assert_eq!(
            a.to_bits(),
            b.to_bits(),
            "{what}: element {i} differs -- C {a:?} ({:#018x}) vs Rust {b:?} ({:#018x})",
            a.to_bits(),
            b.to_bits()
        );
    }
}

/// Small deterministic xorshift PRNG, so the corpora are reproducible without a
/// dependency.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform in `[lo, hi)`.
    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + self.unit() * (hi - lo)
    }
}
