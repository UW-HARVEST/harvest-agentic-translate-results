//! Differential test: load BOTH the C shared library and the Rust cdylib via
//! `libloading` and compare `rev16` outputs byte-for-byte.
//!
//! The Rust side is deliberately loaded through its `.so` export table rather
//! than being linked directly, so the `#[no_mangle] extern "C"` wrapper is
//! exercised exactly as an external C caller would exercise it.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use libloading::{Library, Symbol};

type Rev16 = unsafe extern "C" fn(u32) -> u32;

/// Workspace root (the directory that holds both `c_src/` and `translation/`).
fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Locate the C shared library produced by the CMake build.
///
/// The CMake project name is derived from the parent directory's file name, so
/// the library file name is not fixed; scan the build tree for any `.so`.
fn c_library_path() -> PathBuf {
    let build_dir = workspace_root().join("c_src").join("build");
    assert!(
        build_dir.is_dir(),
        "C build directory {} not found. Build it with:\n  cd c_src && mkdir -p build && cd build \
         && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build_dir.display()
    );

    let mut candidates: Vec<PathBuf> = Vec::new();
    collect_shared_objects(&build_dir, &mut candidates);
    candidates.sort();

    candidates
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("no .so found under {}", build_dir.display()))
}

fn collect_shared_objects(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // CMakeFiles holds object files only; skip it to keep the scan cheap.
            if path.file_name().is_some_and(|n| n == "CMakeFiles") {
                continue;
            }
            collect_shared_objects(&path, out);
        } else if path.extension().is_some_and(|e| e == "so") {
            out.push(path);
        }
    }
}

/// Locate this crate's own cdylib, building it if `cargo test` did not.
///
/// Integration test binaries live in `target/<profile>/deps/`, so the cdylib
/// normally sits in the parent of the test executable's directory. However,
/// nothing in the test binary *links* the cdylib, so Cargo may only emit an
/// `.rmeta` for the lib target. In that case build the cdylib explicitly into a
/// side target directory (a separate directory avoids contending for the build
/// lock that the outer `cargo test` invocation uses).
fn rust_library_path() -> PathBuf {
    static ONCE: OnceLock<PathBuf> = OnceLock::new();
    ONCE.get_or_init(build_rust_library).clone()
}

fn dylib_file_name() -> String {
    format!("{}rev16_lib{}", std::env::consts::DLL_PREFIX, std::env::consts::DLL_SUFFIX)
}

fn build_rust_library() -> PathBuf {
    let test_exe = std::env::current_exe().expect("current_exe");
    let deps_dir = test_exe.parent().expect("deps dir").to_path_buf();
    let profile_dir = deps_dir.parent().expect("profile dir").to_path_buf();

    // "debug" or "release" (or a custom profile's directory name).
    let profile_dir_name = profile_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("debug")
        .to_string();

    let name = dylib_file_name();

    for dir in [&profile_dir, &deps_dir] {
        let candidate = dir.join(&name);
        if candidate.is_file() {
            return candidate;
        }
    }

    // Not built by the outer invocation: build it ourselves.
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let side_target = manifest_dir.join("target").join("ffi-cdylib");
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());

    let mut cmd = std::process::Command::new(cargo);
    cmd.current_dir(manifest_dir)
        .arg("build")
        .arg("--lib")
        .arg("--no-default-features")
        .arg("--target-dir")
        .arg(&side_target);

    // Mirror the feature set the test binary itself was compiled with, so the
    // cdylib under test exercises the same code paths.
    let features = enabled_features();
    if !features.is_empty() {
        cmd.arg("--features").arg(features.join(","));
    }
    if profile_dir_name == "release" {
        cmd.arg("--release");
    }

    let status = cmd
        .status()
        .unwrap_or_else(|e| panic!("failed to spawn cargo to build the cdylib: {e}"));
    assert!(status.success(), "`cargo build --lib` for the cdylib failed: {status}");

    let built = side_target.join(&profile_dir_name).join(&name);
    assert!(
        built.is_file(),
        "cdylib still missing after building: {}",
        built.display()
    );
    built
}

/// Features this test binary was compiled with.
///
/// The crate currently declares no `[features]`, so this is always empty; it is
/// kept as the single place to extend if features are added later.
fn enabled_features() -> Vec<&'static str> {
    Vec::new()
}

struct Libs {
    // Kept alive for the lifetime of the loaded symbols.
    _c: Library,
    _rust: Library,
    c_rev16: Rev16,
    rust_rev16: Rev16,
}

impl Libs {
    fn load() -> Self {
        let c_path = c_library_path();
        let rust_path = rust_library_path();

        // SAFETY: both paths point at libraries we just built from this repo.
        // Loading them runs their (empty) initializers only.
        unsafe {
            let c = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("failed to load C lib {}: {e}", c_path.display()));
            let rust = Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("failed to load Rust lib {}: {e}", rust_path.display()));

            let c_sym: Symbol<Rev16> = c
                .get(b"rev16\0")
                .unwrap_or_else(|e| panic!("`rev16` missing from C lib: {e}"));
            let rust_sym: Symbol<Rev16> = rust
                .get(b"rev16\0")
                .unwrap_or_else(|e| panic!("`rev16` missing from Rust lib: {e}"));

            let c_rev16 = *c_sym;
            let rust_rev16 = *rust_sym;

            Libs { _c: c, _rust: rust, c_rev16, rust_rev16 }
        }
    }

    #[track_caller]
    fn assert_same(&self, input: u32) {
        // SAFETY: both symbols have the C signature `uint32_t(uint32_t)`.
        let (c, rust) = unsafe { ((self.c_rev16)(input), (self.rust_rev16)(input)) };
        assert_eq!(
            c, rust,
            "rev16(0x{input:08X}) mismatch: C = 0x{c:08X}, Rust = 0x{rust:08X}"
        );
    }
}

#[test]
fn rev16_matches_on_edge_cases() {
    let libs = Libs::load();

    let mut inputs = vec![
        0u32,
        1,
        2,
        3,
        0x8000,
        0xFFFF,
        0x0001_0000,
        0xFFFF_0000,
        0xFFFF_FFFF,
        0xAAAA_AAAA,
        0x5555_5555,
        0xCCCC_CCCC,
        0x3333_3333,
        0xF0F0_F0F0,
        0x0F0F_0F0F,
        0xFF00_FF00,
        0x00FF_00FF,
        0x1234_5678,
        0xDEAD_BEEF,
        0xCAFE_BABE,
        u32::MAX - 1,
        0x8000_0000,
        0x7FFF_FFFF,
    ];

    // Every single-bit input, including bits 16..=31 that the C code discards.
    inputs.extend((0..32).map(|bit| 1u32 << bit));
    // Every single cleared bit.
    inputs.extend((0..32).map(|bit| !(1u32 << bit)));

    for input in inputs {
        libs.assert_same(input);
    }
}

#[test]
fn rev16_matches_over_full_low_16_bits() {
    let libs = Libs::load();

    // Exhaustive over the 16 bits the algorithm actually reads.
    for low in 0..=0xFFFFu32 {
        libs.assert_same(low);
    }
}

#[test]
fn rev16_matches_with_arbitrary_high_bits() {
    let libs = Libs::load();

    // The C code drops bits 16..=31 on the first statement. Sweep the low half
    // exhaustively against several high-half patterns to confirm the Rust
    // translation drops them identically instead of "fixing" the truncation.
    for high in [0x0000u32, 0xFFFF, 0xAAAA, 0x5555, 0x1234, 0x8000] {
        for low in 0..=0xFFFFu32 {
            libs.assert_same((high << 16) | low);
        }
    }
}

#[test]
fn rev16_matches_on_pseudorandom_inputs() {
    let libs = Libs::load();

    // Deterministic xorshift32 sweep over the whole 32-bit domain.
    let mut state: u32 = 0x1234_5678;
    for _ in 0..200_000 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        libs.assert_same(state);
    }
}
