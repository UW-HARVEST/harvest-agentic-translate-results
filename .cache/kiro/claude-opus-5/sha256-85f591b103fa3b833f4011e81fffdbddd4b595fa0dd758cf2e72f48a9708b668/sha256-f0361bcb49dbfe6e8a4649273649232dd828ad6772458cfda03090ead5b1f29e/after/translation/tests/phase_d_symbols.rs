//! Phase D — symbol parity and build-shape invariants, checked mechanically.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm_defined(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {so:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .map(|s| s.to_string())
        .collect()
}

fn nm_undefined(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--undefined-only")
        .arg(so)
        .output()
        .expect("run nm");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .map(|s| s.to_string())
        .collect()
}

/// Every symbol the C `.so` exports must also be exported by the Rust `.so`,
/// with the exact same name. The diff must be empty.
#[test]
fn sym_01_every_c_symbol_is_exported_by_rust() {
    let c = nm_defined(&c_so_path());
    let r = nm_defined(&rust_so_path());

    // Sanity: the C library really does export the two known symbols.
    assert!(c.contains("driver"), "C .so must export `driver`; got {c:?}");
    assert!(c.contains("run"), "C .so must export `run`; got {c:?}");

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   = {c:?}\nRust = {r:?}"
    );
}

/// Both exported symbols must actually be resolvable via `dlsym`, i.e. the
/// `#[no_mangle] extern "C"` wrappers are real entry points.
#[test]
fn sym_02_exports_are_dlsym_resolvable_in_both() {
    // Loading each library and taking both symbols is exactly what the
    // differential harness does; do it explicitly so a dlsym regression shows
    // up as its own failure.
    let _ = c_impl().driver_bytes(b"1");
    let _ = rust_impl().driver_bytes(b"1");
    let _ = c_impl().run_once(DEFAULT_HOUSE, 0);
    let _ = rust_impl().run_once(DEFAULT_HOUSE, 0);
}

/// The Rust `.so` must have no unresolved non-libc/runtime imports.
#[test]
fn sym_03_rust_so_has_no_unresolved_imports() {
    let out = Command::new("ldd")
        .arg(rust_so_path())
        .output()
        .expect("run ldd");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        !text.contains("not found"),
        "Rust .so has unresolved dependencies:\n{text}"
    );

    // Every libc symbol the C .so imports must also be imported (or provided)
    // by the Rust .so — otherwise the Rust build is not doing the same work.
    let cu = nm_undefined(&c_so_path());
    let ru = nm_undefined(&rust_so_path());
    let rd = nm_defined(&rust_so_path());
    for sym in ["printf@GLIBC_2.2.5", "strtol@GLIBC_2.2.5"] {
        assert!(
            cu.contains(sym),
            "expected the C .so to import {sym}; imports = {cu:?}"
        );
        assert!(
            ru.contains(sym) || rd.contains(sym),
            "Rust .so neither imports nor defines {sym}; imports = {ru:?}"
        );
    }
}

/// The project builds no executable in either language, so there is no binary
/// stdout comparison to perform. Assert that mechanically so the claim in
/// CONFIGS.md cannot silently go stale.
#[test]
fn sym_04_project_builds_no_binary_target() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();

    let cmake = std::fs::read_to_string(root.join("c_src/CMakeLists.txt")).expect("CMakeLists.txt");
    assert!(
        !cmake.contains("add_executable"),
        "c_src now builds an executable; the binary stdout comparison must be implemented"
    );

    let mut c_sources: Vec<std::path::PathBuf> = Vec::new();
    for entry in std::fs::read_dir(root.join("c_src/src")).expect("c_src/src") {
        let p = entry.expect("dir entry").path();
        if p.extension().and_then(|e| e.to_str()) == Some("c") {
            c_sources.push(p);
        }
    }
    assert!(!c_sources.is_empty(), "no C sources found");
    for p in &c_sources {
        let text = std::fs::read_to_string(p).expect("read C source");
        assert!(
            !text.contains("int main("),
            "{p:?} defines main(); the binary stdout comparison must be implemented"
        );
    }

    let cargo = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
        .expect("Cargo.toml");
    assert!(
        !cargo.contains("[[bin]]"),
        "translation now declares a [[bin]]; the binary comparison must be implemented"
    );
    assert!(
        !Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/main.rs")
            .exists(),
        "src/main.rs exists; the binary comparison must be implemented"
    );
}

/// Every C source file under `c_src/src` must have a corresponding translation.
/// Guards against the "a whole module was never translated" failure mode.
#[test]
fn sym_05_every_c_translation_unit_is_translated() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let mut units: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(root.join("c_src/src")).expect("c_src/src") {
        let p = entry.expect("entry").path();
        if p.extension().and_then(|e| e.to_str()) == Some("c") {
            units.push(p.file_stem().unwrap().to_string_lossy().to_string());
        }
    }
    units.sort();
    assert_eq!(
        units,
        vec!["driver".to_string()],
        "the set of C translation units changed; the Rust crate must cover all of them"
    );

    // driver.c defines exactly these function names; all must be represented in
    // the Rust source (exported or private).
    let rust_src =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"))
            .expect("src/lib.rs");
    for f in [
        "add_floor",
        "add_bedrooms",
        "print_house",
        "run",
        "parse_val",
        "driver",
    ] {
        assert!(
            rust_src.contains(&format!("fn {f}(")),
            "Rust translation is missing `fn {f}`"
        );
    }
}
