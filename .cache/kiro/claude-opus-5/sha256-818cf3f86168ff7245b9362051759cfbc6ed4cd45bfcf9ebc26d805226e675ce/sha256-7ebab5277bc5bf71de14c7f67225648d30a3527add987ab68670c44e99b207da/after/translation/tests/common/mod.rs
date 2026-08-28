//! Shared harness: loads the C reference `.so` and the Rust `.so` through
//! `libloading` and exposes them behind an identical FFI signature so every
//! comparison crosses a real dynamic-linking boundary.

#![allow(non_snake_case, dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

/// Byte-for-byte mirror of `cb_rgb_255` in `c_src/include/lib.h`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CbRgb255 {
    pub R: u8,
    pub G: u8,
    pub B: u8,
}

impl CbRgb255 {
    pub const fn new(R: u8, G: u8, B: u8) -> Self {
        Self { R, G, B }
    }
}

pub type ContrastRatioFn = unsafe extern "C" fn(CbRgb255, CbRgb255) -> f32;

/// Owns both libraries plus the resolved `contrast_ratio` entry points.
pub struct Harness {
    _c_lib: Library,
    _rust_lib: Library,
    c_contrast_ratio: ContrastRatioFn,
    rust_contrast_ratio: ContrastRatioFn,
}

impl Harness {
    pub fn load() -> Self {
        let c_path = find_c_library();
        let rust_path = find_rust_library();

        let (_c_lib, c_contrast_ratio) = load_symbol(&c_path);
        let (_rust_lib, rust_contrast_ratio) = load_symbol(&rust_path);

        Self {
            _c_lib,
            _rust_lib,
            c_contrast_ratio,
            rust_contrast_ratio,
        }
    }

    #[inline]
    pub fn c(&self, a: CbRgb255, b: CbRgb255) -> f32 {
        unsafe { (self.c_contrast_ratio)(a, b) }
    }

    #[inline]
    pub fn rust(&self, a: CbRgb255, b: CbRgb255) -> f32 {
        unsafe { (self.rust_contrast_ratio)(a, b) }
    }

    /// Compares the two implementations on one input pair.
    ///
    /// Equality is on the raw 32 bits, so `-0.0` never passes for `0.0` and a
    /// NaN must carry the identical sign and payload.
    #[inline]
    pub fn check(&self, a: CbRgb255, b: CbRgb255) {
        let got_c = self.c(a, b);
        let got_rust = self.rust(a, b);
        if got_c.to_bits() != got_rust.to_bits() {
            panic!(
                "mismatch for A={:?} B={:?}: C = {} (bits {:#010x}), Rust = {} (bits {:#010x})",
                a,
                b,
                got_c,
                got_c.to_bits(),
                got_rust,
                got_rust.to_bits(),
            );
        }
    }
}

fn load_symbol(path: &Path) -> (Library, ContrastRatioFn) {
    let lib = unsafe { Library::new(path) }
        .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
    let func = unsafe {
        let sym: Symbol<ContrastRatioFn> = lib
            .get(b"contrast_ratio\0")
            .unwrap_or_else(|e| panic!("no `contrast_ratio` in {}: {e}", path.display()));
        *sym
    };
    (lib, func)
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR points at `translation/`; the sibling is `c_src/`.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// The C library name is derived by CMake from the directory name, so glob for
/// whatever `lib*.so` landed in the build tree instead of hard-coding it.
fn find_c_library() -> PathBuf {
    let build_dir = workspace_root().join("c_src/build");
    let entries = std::fs::read_dir(&build_dir).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e}\nBuild the C reference first:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build_dir.display()
        )
    });

    let mut found: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            let name = match p.file_name().and_then(|n| n.to_str()) {
                Some(n) => n,
                None => return false,
            };
            name.starts_with("lib") && name.ends_with(".so")
        })
        .collect();
    found.sort();

    found.into_iter().next().unwrap_or_else(|| {
        panic!("no lib*.so found in {}", build_dir.display());
    })
}

/// Locates the Rust `cdylib` for the profile the tests were built with.
///
/// `cargo test` does **not** build a `cdylib`-only lib target, so the `.so`
/// must come from a preceding `cargo build`. That makes a stale artifact the
/// main hazard here: silently loading yesterday's `.so` would turn this entire
/// suite into a no-op. So the lookup is deliberately strict — it never falls
/// back to a different profile's directory, and it refuses to run against a
/// library older than the sources it is supposed to represent.
fn find_rust_library() -> PathBuf {
    if let Some(explicit) = std::env::var_os(SO_PATH_ENV) {
        let path = PathBuf::from(explicit);
        assert!(
            path.exists(),
            "{SO_PATH_ENV} points at {} which does not exist",
            path.display()
        );
        assert_fresh(&path);
        return path;
    }

    // The test binary lives in `target/<profile>/deps/`, so the cdylib for the
    // matching profile is exactly one directory up. Only that location is
    // accepted; a debug test run must not silently pick up a release build.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("test binary should live in target/<profile>/deps/");
    let path = profile_dir.join("libcontrast_ratio_lib.so");

    assert!(
        path.exists(),
        "{} is missing.\n`cargo test` does not build a cdylib-only lib target, \
         so build it first:\n  cd translation && cargo build{}\n\
         (or point {SO_PATH_ENV} at the .so explicitly)",
        path.display(),
        if profile_dir.ends_with("release") {
            " --release"
        } else {
            ""
        },
    );
    assert_fresh(&path);
    path
}

const SO_PATH_ENV: &str = "CONTRAST_RATIO_RUST_SO";

/// Rejects a `.so` that predates any crate source file.
fn assert_fresh(so: &Path) {
    let so_mtime = mtime(so).unwrap_or_else(|| panic!("cannot stat {}", so.display()));

    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
    let mut sources = vec![crate_dir.join("Cargo.toml")];
    collect_rs_files(&crate_dir.join("src"), &mut sources);

    for src in sources {
        if let Some(t) = mtime(&src) {
            if newest.as_ref().is_none_or(|(_, best)| t > *best) {
                newest = Some((src, t));
            }
        }
    }

    if let Some((newest_path, newest_mtime)) = newest {
        assert!(
            so_mtime >= newest_mtime,
            "{} is STALE: it is older than {}.\nRebuild before testing:\n  \
             cd translation && cargo build{}\nRunning against a stale library \
             would make these comparisons meaningless.",
            so.display(),
            newest_path.display(),
            if so.to_string_lossy().contains("/release/") {
                " --release"
            } else {
                ""
            },
        );
    }
}

fn mtime(path: &Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Deterministic 64-bit xorshift* so failures are always reproducible.
pub struct Rng(u64);

impl Rng {
    pub const fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    #[inline]
    pub fn color(&mut self) -> CbRgb255 {
        let v = self.next_u64();
        CbRgb255::new(v as u8, (v >> 8) as u8, (v >> 16) as u8)
    }
}

/// Values that bracket the `0.04045` branch threshold, the `0`/`255` extremes,
/// and the low end where the ratio denominator collapses toward zero.
///
/// `0.04045 * 255 == 10.31`, so the branch flips between 10 and 11.
pub const INTERESTING: &[u8] = &[
    0, 1, 2, 3, 4, 5, 9, 10, 11, 12, 13, 16, 31, 32, 63, 64, 100, 127, 128, 129, 191, 200, 251,
    252, 253, 254, 255,
];
