//! Differential test: load the C shared library and the Rust cdylib via
//! `libloading` and compare `half2float` outputs bit-for-bit.
//!
//! Both libraries are called only through their exported C ABI symbols, so the
//! `#[no_mangle]` wrappers are exercised as an external caller would use them.

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

type Half2Float = unsafe extern "C" fn(u16) -> f32;

/// Workspace root: `<repo>/translation/..`
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Locate the C shared library produced by the CMake build.
fn c_lib_path() -> PathBuf {
    let build_dir = repo_root().join("c_src/build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&build_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let is_so = path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("lib") && n.ends_with(".so"))
                .unwrap_or(false);
            if is_so {
                candidates.push(path);
            }
        }
    }
    candidates.sort();
    candidates.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no C shared library found in {}; build it with:\n  cd c_src && mkdir -p build && cd build \
             && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build_dir.display()
        )
    })
}

/// Locate the Rust cdylib. The test binary lives in `target/<profile>/deps/`,
/// so the cdylib sits two levels up from the executable.
///
/// `cargo test` does not necessarily build the `cdylib` artifact (only the
/// rlib/test harness), so if it is absent we build it on demand into a separate
/// target directory to avoid contending for the primary target-dir lock that
/// the running `cargo test` invocation already holds.
fn rust_lib_path() -> PathBuf {
    let name = format!("lib{}.so", "half2float_lib");

    // Allows pointing the same differential suite at e.g. the release artifact.
    if let Ok(p) = std::env::var("TRANSLATION_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "TRANSLATION_RUST_SO={} is not a file", p.display());
        return p;
    }

    let exe = std::env::current_exe().expect("current_exe");
    let mut dir = exe.parent().expect("deps dir").to_path_buf();
    // `<profile>/deps/<test-bin>` -> `<profile>`
    if dir.file_name().and_then(|n| n.to_str()) == Some("deps") {
        dir.pop();
    }
    let direct = dir.join(&name);
    if direct.is_file() {
        return direct;
    }

    // Fall back to building the cdylib ourselves.
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let aux_target = manifest_dir.join("target/ffi-cdylib");
    let feature_args: Vec<String> = std::env::var("TRANSLATION_TEST_FEATURE_ARGS")
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_string)
        .collect();

    let mut cmd = std::process::Command::new(env!("CARGO"));
    cmd.arg("build")
        .arg("--lib")
        .arg("--manifest-path")
        .arg(manifest_dir.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(&aux_target)
        .args(&feature_args)
        // Prevent inheriting the parent cargo's jobserver / config quirks.
        .env_remove("CARGO_MAKEFLAGS")
        .env_remove("RUSTC_WORKSPACE_WRAPPER");
    let out = cmd.output().expect("failed to spawn cargo build for cdylib");
    assert!(
        out.status.success(),
        "cargo build --lib failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let built = aux_target.join("debug").join(&name);
    if built.is_file() {
        return built;
    }
    panic!(
        "Rust cdylib {} not found in {} nor {}",
        name,
        dir.display(),
        aux_target.join("debug").display()
    );
}

struct Libs {
    _c: Library,
    _rust: Library,
    c_half2float: Half2Float,
    rust_half2float: Half2Float,
}

impl Libs {
    fn load() -> Self {
        let c_path = c_lib_path();
        let rust_path = rust_lib_path();
        unsafe {
            let c = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("failed to load {}: {e}", c_path.display()));
            let rust = Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("failed to load {}: {e}", rust_path.display()));
            let c_sym: Symbol<Half2Float> = c
                .get(b"half2float\0")
                .expect("C .so does not export half2float");
            let rust_sym: Symbol<Half2Float> = rust
                .get(b"half2float\0")
                .expect("Rust .so does not export half2float");
            let c_half2float = *c_sym;
            let rust_half2float = *rust_sym;
            Libs {
                _c: c,
                _rust: rust,
                c_half2float,
                rust_half2float,
            }
        }
    }
}

/// Exhaustive check over all 65536 possible `uint16_t` inputs, comparing the
/// raw bit patterns of the returned floats (NaN-safe, sign-of-zero-safe).
#[test]
fn half2float_matches_c_for_all_u16_inputs() {
    let libs = Libs::load();
    let mut mismatches: Vec<(u16, u32, u32)> = Vec::new();

    for h in 0u16..=u16::MAX {
        let c_out = unsafe { (libs.c_half2float)(h) };
        let rust_out = unsafe { (libs.rust_half2float)(h) };
        let c_bits = c_out.to_bits();
        let rust_bits = rust_out.to_bits();
        if c_bits != rust_bits {
            mismatches.push((h, c_bits, rust_bits));
            if mismatches.len() >= 20 {
                break;
            }
        }
    }

    assert!(
        mismatches.is_empty(),
        "half2float mismatches (input, c_bits, rust_bits): {:#x?}",
        mismatches
    );
}

/// Spot-check the interesting classes of half-precision values explicitly so a
/// failure report points at a meaningful case rather than just an index.
#[test]
fn half2float_matches_c_for_special_values() {
    let libs = Libs::load();

    let cases: [u16; 24] = [
        0x0000, // +0
        0x8000, // -0
        0x0001, // smallest positive subnormal
        0x8001, // smallest negative subnormal
        0x03ff, // largest positive subnormal
        0x83ff, // largest negative subnormal
        0x0400, // smallest positive normal
        0x8400, // smallest negative normal
        0x3c00, // 1.0
        0xbc00, // -1.0
        0x3555, // ~1/3
        0x7bff, // largest finite
        0xfbff, // most negative finite
        0x7c00, // +inf
        0xfc00, // -inf
        0x7c01, // signalling NaN
        0xfc01, // negative signalling NaN
        0x7e00, // quiet NaN
        0xfe00, // negative quiet NaN
        0x7fff, // NaN, all payload bits set
        0xffff, // negative NaN, all payload bits set
        0x4900, // 10.0
        0xc900, // -10.0
        0x6400, // 1024.0
    ];

    for h in cases {
        let c_bits = unsafe { (libs.c_half2float)(h) }.to_bits();
        let rust_bits = unsafe { (libs.rust_half2float)(h) }.to_bits();
        assert_eq!(
            c_bits, rust_bits,
            "half2float({h:#06x}): C returned {c_bits:#010x}, Rust returned {rust_bits:#010x}"
        );
    }
}

/// The Rust .so must export every dynamic symbol the C .so exports.
#[test]
fn rust_so_exports_every_c_symbol() {
    let c_path = c_lib_path();
    let rust_path = rust_lib_path();

    let dynamic_symbols = |path: &Path| -> Vec<String> {
        let out = std::process::Command::new("nm")
            .arg("-D")
            .arg("--defined-only")
            .arg(path)
            .output()
            .expect("failed to run nm");
        assert!(
            out.status.success(),
            "nm -D {} failed: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|line| {
                let mut fields = line.split_whitespace();
                let (_addr, kind, name) = (fields.next()?, fields.next()?, fields.next()?);
                // Only compare code/data definitions, not linker-generated
                // section boundaries or debug entries.
                if matches!(kind, "T" | "t" | "D" | "d" | "B" | "b" | "R" | "r" | "W" | "w" | "i")
                {
                    Some(name.to_string())
                } else {
                    None
                }
            })
            .collect()
    };

    let c_symbols = dynamic_symbols(&c_path);
    let rust_symbols = dynamic_symbols(&rust_path);

    // Symbols emitted by every toolchain-produced shared object rather than by
    // the translated source itself.
    let is_toolchain_symbol = |name: &str| {
        matches!(
            name,
            "_init"
                | "_fini"
                | "__bss_start"
                | "_edata"
                | "_end"
                | "__libc_csu_init"
                | "__libc_csu_fini"
        ) || name.starts_with("_ITM_")
            || name.starts_with("__gmon_")
            || name.starts_with("__cxa_")
    };

    let missing: Vec<&String> = c_symbols
        .iter()
        .filter(|s| !is_toolchain_symbol(s) && !rust_symbols.contains(s))
        .collect();

    assert!(
        missing.is_empty(),
        "Rust .so ({}) is missing symbols exported by the C .so ({}): {:?}",
        rust_path.display(),
        c_path.display(),
        missing
    );
}
