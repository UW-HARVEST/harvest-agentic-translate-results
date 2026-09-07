//! Phase D — symbol parity enforced as a test, so it cannot silently regress.

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn defined_dynamic_symbols(so: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(so)
        .output()
        .expect("nm must be available");
    assert!(out.status.success(), "nm failed on {}", so.display());
    let text = String::from_utf8_lossy(&out.stdout);
    let mut v: Vec<String> = text
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .map(|s| s.to_string())
        // Toolchain/runtime scaffolding that is not part of the C API surface.
        .filter(|s| {
            !s.starts_with("_ITM_")
                && !s.starts_with("__cxa")
                && !s.starts_with("__gmon")
                && !s.starts_with("_init")
                && !s.starts_with("_fini")
                && !s.starts_with("__bss_start")
                && !s.starts_with("_edata")
                && !s.starts_with("_end")
                && !s.starts_with("rust_")
                && !s.starts_with("__rust")
                && !s.starts_with("_R")
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

fn c_so() -> PathBuf {
    let build = root().join("c_src").join("build");
    let mut c: Vec<PathBuf> = std::fs::read_dir(&build)
        .expect("c_src/build must exist — build the C library first")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("lib") && n.ends_with(".so"))
                .unwrap_or(false)
        })
        .collect();
    c.sort();
    c.pop().expect("no lib*.so in c_src/build")
}

fn rust_sos() -> Vec<PathBuf> {
    let t = root().join("translation").join("target");
    ["release", "debug"]
        .iter()
        .map(|p| t.join(p).join("libhsv_to_rgb_lib.so"))
        .filter(|p| p.is_file())
        .collect()
}

/// Every symbol the C `.so` exports must also be exported by the Rust `.so`,
/// under the exact same name. The diff must be empty.
#[test]
fn symbol_parity_c_subset_of_rust() {
    let c = defined_dynamic_symbols(&c_so());
    assert!(!c.is_empty(), "C .so exported no symbols — bad build?");
    let sos = rust_sos();
    assert!(!sos.is_empty(), "no Rust cdylib built");
    for so in &sos {
        let r = defined_dynamic_symbols(so);
        let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
        assert!(
            missing.is_empty(),
            "{} is missing C symbols {:?}\n  C   = {:?}\n  Rust= {:?}",
            so.display(),
            missing,
            c,
            r
        );
    }
}

/// The C `.so` exports exactly one symbol; assert that so a newly added C
/// function cannot slip past the parity check unnoticed.
#[test]
fn c_surface_is_exactly_hsv_to_rgb() {
    let c = defined_dynamic_symbols(&c_so());
    assert_eq!(
        c,
        vec!["hsv_to_rgb".to_string()],
        "the C export surface changed; re-derive SYMBOLS.md"
    );
}

/// The Rust `.so` must have no unresolved non-libc dependencies.
#[test]
fn rust_has_no_unexpected_undefined_symbols() {
    for so in rust_sos() {
        let out = Command::new("nm")
            .args(["-D", "--undefined-only", "--format=posix"])
            .arg(&so)
            .output()
            .expect("nm");
        let text = String::from_utf8_lossy(&out.stdout);
        let bad: Vec<&str> = text
            .lines()
            .filter_map(|l| l.split_whitespace().next())
            .filter(|s| {
                // libc / libm / libgcc / CRT scaffolding is expected.
                !s.contains("@GLIBC")
                    && !s.contains("@GCC")
                    && !s.starts_with("_ITM_")
                    && !s.starts_with("__gmon")
                    && !s.starts_with("__cxa")
                    && !s.starts_with("_Unwind")
            })
            .collect();
        assert!(bad.is_empty(), "{} has unresolved symbols {:?}", so.display(), bad);
    }
}

/// The project builds no binary executable, so the "compare stdout" gate is
/// structurally N/A. Verified mechanically rather than asserted in prose.
#[test]
fn no_binary_target_exists() {
    let cm = std::fs::read_to_string(root().join("c_src/CMakeLists.txt")).unwrap();
    assert!(
        !cm.contains("add_executable"),
        "c_src now builds an executable; the stdout-comparison gate must be implemented"
    );
    let cargo = std::fs::read_to_string(root().join("translation/Cargo.toml")).unwrap();
    assert!(!cargo.contains("[[bin]]"), "translation now declares a binary target");
    assert!(!root().join("translation/src/main.rs").exists(), "src/main.rs appeared");
    assert!(!root().join("translation/src/bin").exists(), "src/bin appeared");
}

/// `Cargo.toml` declares no cargo features, so the default build is the only
/// configuration. If a feature is ever added, this test fails and the feature
/// matrix must be re-run.
#[test]
fn no_cargo_features_declared() {
    let cargo = std::fs::read_to_string(root().join("translation/Cargo.toml")).unwrap();
    assert!(
        !cargo.contains("[features]"),
        "features were added; re-run phases B and C for every feature combination"
    );
}
