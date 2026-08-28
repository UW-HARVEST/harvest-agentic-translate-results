//! Differential test: loads BOTH the C shared library and the Rust `cdylib`
//! through `libloading` and compares `max_size_frame` results byte-for-byte.
//!
//! Neither implementation is called directly; both go through the dynamic
//! symbol table exactly as an external C caller would, which also exercises the
//! `#[no_mangle]` export wrapper on the Rust side.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use libloading::{Library, Symbol};

/// `tflac_u32 max_size_frame(tflac_u32, tflac_u32, tflac_u32)`
type MaxSizeFrameFn = unsafe extern "C" fn(u32, u32, u32) -> u32;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Locates the C shared library produced by `c_src/CMakeLists.txt`. The CMake
/// project name is derived from the parent directory name, so the file name is
/// not fixed; glob the build directory instead.
fn c_library_path() -> PathBuf {
    let build_dir = workspace_root().join("c_src").join("build");
    let entries = std::fs::read_dir(&build_dir).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e}. Build the C library first:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build_dir.display()
        )
    });

    let mut candidates: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            let name = match p.file_name().and_then(|n| n.to_str()) {
                Some(n) => n,
                None => return false,
            };
            name.starts_with("lib") && name.ends_with(".so") && p.is_file()
        })
        .collect();
    candidates.sort();

    candidates.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no lib*.so found in {}; build the C library first",
            build_dir.display()
        )
    })
}

/// Builds and locates the Rust `cdylib` under test.
///
/// `cargo test` does **not** rebuild a `cdylib`-only lib target for integration
/// tests, so simply globbing `target/<profile>/` would happily dlopen a stale
/// artifact and let a broken translation pass. This function therefore invokes
/// `cargo build --lib` itself, into a dedicated `--target-dir` so it cannot
/// deadlock against the build-directory lock the outer `cargo test` still holds.
fn rust_library_path() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(build_rust_library).clone()
}

fn build_rust_library() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let target_dir = manifest_dir.join("target").join("differential-cdylib");

    // Mirror the profile the test binary itself was compiled with.
    let release = cfg!(not(debug_assertions));

    let mut cmd = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    cmd.arg("build")
        .arg("--lib")
        .arg("--manifest-path")
        .arg(manifest_dir.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(&target_dir)
        .arg("--offline");
    if release {
        cmd.arg("--release");
    }

    // Reproduce the feature selection this test binary was built with, so every
    // `cargo test --no-default-features --features <combo>` invocation loads a
    // matching .so. `translation` currently declares no [features]; the list is
    // built via cfg! so it stays correct if features are added later.
    cmd.arg("--no-default-features");
    let features: Vec<&str> = ENABLED_FEATURES.iter().copied().flatten().collect();
    if !features.is_empty() {
        cmd.arg("--features").arg(features.join(","));
    }

    // `cargo test` exports variables that would otherwise leak into the nested
    // build and confuse it about which package or target directory to use.
    for var in [
        "CARGO_MANIFEST_DIR",
        "CARGO_PKG_NAME",
        "CARGO_CRATE_NAME",
        "CARGO_PRIMARY_PACKAGE",
        "CARGO_TARGET_DIR",
        "RUSTC_WORKSPACE_WRAPPER",
    ] {
        cmd.env_remove(var);
    }

    let output = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn cargo to build the cdylib: {e}"));
    assert!(
        output.status.success(),
        "nested `cargo build --lib` failed ({}):\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let file_name = format!(
        "{}max_size_frame_lib{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    );
    let path = target_dir
        .join(if release { "release" } else { "debug" })
        .join(&file_name);
    assert!(
        path.is_file(),
        "nested build did not produce {}",
        path.display()
    );
    path
}

/// Features enabled for this test binary. `translation` declares none, so this
/// is empty; add `cfg!(feature = "x").then_some("x")` entries alongside any new
/// feature so the nested `cdylib` build stays in sync.
const ENABLED_FEATURES: &[Option<&str>] = &[];

struct Impls {
    _c_lib: Library,
    _rust_lib: Library,
    c: MaxSizeFrameFn,
    rust: MaxSizeFrameFn,
}

impl Impls {
    fn load() -> Self {
        let c_path = c_library_path();
        let rust_path = rust_library_path();

        // SAFETY: both paths point at shared objects we just built; loading them
        // runs their (empty) initialisers.
        let c_lib = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
        let rust_lib = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_path.display()));

        // SAFETY: the symbol has the C signature declared in include/lib.h.
        let c: Symbol<MaxSizeFrameFn> = unsafe { c_lib.get(b"max_size_frame\0") }
            .expect("C .so must export max_size_frame");
        let rust: Symbol<MaxSizeFrameFn> = unsafe { rust_lib.get(b"max_size_frame\0") }
            .expect("Rust .so must export max_size_frame");

        let c = *c;
        let rust = *rust;

        Impls {
            _c_lib: c_lib,
            _rust_lib: rust_lib,
            c,
            rust,
        }
    }

    #[track_caller]
    fn assert_same(&self, blocksize: u32, channels: u32, bitdepth: u32) {
        // SAFETY: plain integer-in/integer-out FFI calls, no pointers involved.
        let expected = unsafe { (self.c)(blocksize, channels, bitdepth) };
        let actual = unsafe { (self.rust)(blocksize, channels, bitdepth) };
        assert_eq!(
            expected, actual,
            "max_size_frame(blocksize={blocksize}, channels={channels}, bitdepth={bitdepth}): \
             C returned {expected}, Rust returned {actual}"
        );
    }
}

/// Values that matter for the three predicates in the expression
/// (`channels != 2`, `channels == 2`, `bitdepth != 32`) plus 32-bit wrap-around
/// boundaries.
const INTERESTING: &[u32] = &[
    0,
    1,
    2,
    3,
    4,
    5,
    6,
    7,
    8,
    12,
    16,
    20,
    24,
    31,
    32,
    33,
    64,
    255,
    256,
    576,
    1152,
    4096,
    4607,
    65535,
    65536,
    0x00FF_FFFF,
    0x0100_0000,
    0x1FFF_FFFF,
    0x2000_0000,
    0x7FFF_FFFF,
    0x8000_0000,
    0x8000_0001,
    0xFFFF_FFFE,
    0xFFFF_FFFF,
];

#[test]
fn both_libraries_export_the_symbol() {
    // Loading succeeds only if both .so files export `max_size_frame`.
    let impls = Impls::load();
    impls.assert_same(4096, 2, 16);
}

/// Realistic FLAC parameter space: every channel count and bit depth an encoder
/// can produce, across the standard block sizes.
#[test]
fn matches_c_on_realistic_flac_parameters() {
    let impls = Impls::load();
    for blocksize in [0u32, 1, 16, 192, 576, 1152, 2304, 4096, 4608, 8192, 16384, 65535] {
        for channels in 0u32..=8 {
            for bitdepth in 0u32..=32 {
                impls.assert_same(blocksize, channels, bitdepth);
            }
        }
    }
}

/// Exhaustive cross product of the boundary values, which covers unsigned
/// overflow in each of the three product terms and in the final sum.
#[test]
fn matches_c_on_boundary_cross_product() {
    let impls = Impls::load();
    for &blocksize in INTERESTING {
        for &channels in INTERESTING {
            for &bitdepth in INTERESTING {
                impls.assert_same(blocksize, channels, bitdepth);
            }
        }
    }
}

/// Dense sweep of small values, where the `+7` rounding and the `/ 8`
/// truncation are most easily observed.
#[test]
fn matches_c_on_dense_small_values() {
    let impls = Impls::load();
    for blocksize in 0u32..=40 {
        for channels in 0u32..=40 {
            for bitdepth in 0u32..=40 {
                impls.assert_same(blocksize, channels, bitdepth);
            }
        }
    }
}

/// Deterministic pseudo-random fuzzing over the full 32-bit domain (SplitMix64,
/// so the test needs no external RNG crate and always replays identically).
#[test]
fn matches_c_on_pseudorandom_inputs() {
    let impls = Impls::load();

    let mut state: u64 = 0x243F_6A88_85A3_08D3;
    let mut next = move || -> u32 {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) >> 32) as u32
    };

    for _ in 0..200_000 {
        let blocksize = next();
        let channels = next();
        let bitdepth = next();
        impls.assert_same(blocksize, channels, bitdepth);

        // Also probe with the predicate-relevant values forced in, so the
        // `channels == 2` / `bitdepth == 32` branches get random partners.
        impls.assert_same(blocksize, 2, bitdepth);
        impls.assert_same(blocksize, channels, 32);
        impls.assert_same(blocksize, 2, 32);
    }
}
