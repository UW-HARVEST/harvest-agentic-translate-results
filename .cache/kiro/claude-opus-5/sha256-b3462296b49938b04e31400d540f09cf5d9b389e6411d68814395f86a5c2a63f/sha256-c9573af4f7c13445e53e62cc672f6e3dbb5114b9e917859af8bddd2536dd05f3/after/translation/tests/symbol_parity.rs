//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Runs `nm -D --defined-only` on both shared objects and requires the exported
//! symbol sets to be identical, including any macro-generated names. Also
//! requires that every exported symbol is actually resolvable through
//! `libloading` (an entry in `.dynsym` is not by itself proof that `dlsym`
//! works).

mod harness;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn c_so() -> PathBuf {
    std::env::var("C_DRIVER_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|_| repo_root().join("c_src/build/libdriver.so"))
}

fn rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let release = manifest.join("target/release/libdriver.so");
    if release.exists() {
        release
    } else {
        manifest.join("target/debug/libdriver.so")
    }
}

/// Exported (dynamic, defined) symbol names, excluding the libc/toolchain
/// boilerplate that every ELF shared object carries.
fn exported_symbols(so: &Path) -> BTreeSet<String> {
    assert!(so.exists(), "{} does not exist", so.display());
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("failed to run `nm`; is binutils installed?");
    assert!(
        out.status.success(),
        "nm -D failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| line.split_whitespace().last())
        .map(str::to_string)
        .filter(|name| {
            // Toolchain / runtime boilerplate, not part of the library's API.
            !(name.starts_with("__")
                || name.starts_with("_ITM_")
                || name.starts_with("_Z")
                || name == "_init"
                || name == "_fini"
                || name == "_edata"
                || name == "_end"
                || name == "_DYNAMIC"
                || name == "_GLOBAL_OFFSET_TABLE_"
                || name == "rust_eh_personality")
        })
        .collect()
}

#[test]
fn symbols_01_rust_exports_every_c_symbol() {
    let c = exported_symbols(&c_so());
    let r = exported_symbols(&rust_so());

    assert!(
        !c.is_empty(),
        "no symbols extracted from the C .so; the nm parsing is broken"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n  C:    {c:?}\n  Rust: {r:?}",
        missing.len()
    );
}

/// The C `.so` exports exactly `driver` and `run`; pin that so a future change
/// to the C source cannot silently shrink the verified surface.
#[test]
fn symbols_02_expected_c_surface() {
    let c = exported_symbols(&c_so());
    let expected: BTreeSet<String> = ["driver", "run"].iter().map(|s| s.to_string()).collect();
    assert_eq!(
        c, expected,
        "the C .so's exported surface changed; SYMBOLS.md / CONFIGS.md / ERRORS.md must be regenerated"
    );
}

/// `static` C functions and the `the_house` global must stay internal in the
/// Rust build too, so the dynamic surfaces match in both directions.
#[test]
fn symbols_03_rust_does_not_export_c_internals() {
    let r = exported_symbols(&rust_so());
    for internal in [
        "add_floor",
        "add_bedrooms",
        "add_floor_to_the_house",
        "print_the_house",
        "the_house",
        "THE_HOUSE",
    ] {
        assert!(
            !r.contains(internal),
            "the Rust .so exports `{internal}`, which is `static` (internal linkage) in the C"
        );
    }
}

/// `.dynsym` presence is not the same as `dlsym` success — check both symbols
/// actually resolve out of both libraries.
#[test]
fn symbols_04_every_c_symbol_resolves_via_dlsym() {
    type IntFn = unsafe extern "C" fn(std::ffi::c_int);

    for so in [c_so(), rust_so()] {
        let lib = unsafe { libloading::Library::new(&so) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", so.display()));
        for name in exported_symbols(&c_so()) {
            let sym: Result<libloading::Symbol<IntFn>, _> =
                unsafe { lib.get(format!("{name}\0").as_bytes()) };
            assert!(
                sym.is_ok(),
                "dlsym(`{name}`) failed in {}: {:?}",
                so.display(),
                sym.err()
            );
        }
    }
}
