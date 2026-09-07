//! Phase D — symbol parity, enforced as a test.
//!
//! Asserts that every symbol the C `.so` exports is also exported by the Rust
//! `.so` under the exact same name, and that the Rust `.so` has no unresolved
//! non-libc dependencies.

mod common;

use std::path::PathBuf;
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn so(which: &str) -> PathBuf {
    match which {
        "c" => std::env::var("DIFF_C_SO")
            .map(PathBuf::from)
            .unwrap_or_else(|_| manifest_dir().join("../c_src/build/libdriver.so")),
        _ => std::env::var("DIFF_RUST_SO").map(PathBuf::from).unwrap_or_else(|_| {
            let d = manifest_dir().join("target/debug/libdriver.so");
            if d.exists() { d } else { manifest_dir().join("target/release/libdriver.so") }
        }),
    }
}

fn nm(path: &PathBuf, extra: &str) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg(extra)
        .arg(path)
        .output()
        .expect("failed to run `nm` (binutils required)");
    assert!(out.status.success(), "nm failed on {}", path.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

/// Every exported C symbol must be exported by the Rust `.so` too.
#[test]
fn symbol_parity_c_exports_subset_of_rust_exports() {
    let c_path = so("c");
    let r_path = so("rust");
    assert!(c_path.exists(), "C .so not built at {}", c_path.display());
    assert!(r_path.exists(), "Rust .so not built at {}", r_path.display());

    let c_defined = nm(&c_path, "--defined-only");
    let r_defined = nm(&r_path, "--defined-only");
    assert!(!c_defined.is_empty(), "C .so exports nothing — build problem?");

    let missing: Vec<&String> = c_defined.iter().filter(|s| !r_defined.contains(s)).collect();
    assert!(missing.is_empty(), "symbols exported by C but MISSING from Rust: {missing:?}");

    // Sanity: the one documented entry point really is there.
    assert!(c_defined.iter().any(|s| s == "custom_strdup"));
    assert!(r_defined.iter().any(|s| s == "custom_strdup"));
}

/// The Rust `.so` must have no unresolved symbols beyond libc / the unwinder.
#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let r_path = so("rust");
    let undefined = nm(&r_path, "--undefined-only");
    // Everything legitimately imported carries a glibc/libgcc version tag or is
    // a weak toolchain marker.
    let allowed_markers = [
        "GLIBC", "GCC_", "_ITM_", "__gmon_start__", "statx", "gettid", "CXXABI",
    ];
    let bad: Vec<&String> = undefined
        .iter()
        .filter(|s| !allowed_markers.iter().any(|m| s.contains(m)))
        .collect();
    assert!(bad.is_empty(), "Rust .so has unresolved non-libc symbols: {bad:?}");

    // And the loader must be able to resolve everything for real.
    let out = Command::new("ldd").arg("-r").arg(&r_path).output().expect("ldd failed to run");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !text.contains("undefined symbol"),
        "ldd -r reports undefined symbols in the Rust .so:\n{text}"
    );
}
