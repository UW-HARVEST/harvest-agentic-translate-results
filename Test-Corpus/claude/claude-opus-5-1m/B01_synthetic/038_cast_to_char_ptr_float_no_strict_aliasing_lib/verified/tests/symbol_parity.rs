//! Phase D: automated `nm -D` symbol-parity check between the C `.so` and the
//! Rust `.so`. Every symbol the C library exports must be exported by the Rust
//! library under the exact same name.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    std::env::var("HARVEST_C_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|_| manifest_dir().join("c_src/build/libdriver.so"))
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_RUST_SO") {
        return PathBuf::from(p);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent().and_then(|d| d.parent()) {
            let c = dir.join("libdriver.so");
            if c.exists() {
                return c;
            }
        }
    }
    manifest_dir().join("target/debug/libdriver.so")
}

/// Names of the dynamic symbols that the object *defines* (`nm -D --defined-only`).
fn defined_symbols(so: &PathBuf) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("failed to run `nm` (binutils required)");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let mut it = line.split_whitespace();
            // "<addr> <type> <name>" or "<type> <name>" for weak/undefined
            let (ty, name) = match (it.next(), it.next(), it.next()) {
                (Some(_addr), Some(ty), Some(name)) => (ty, name),
                (Some(ty), Some(name), None) => (ty, name),
                _ => return None,
            };
            // Keep global/weak text+data definitions.
            if matches!(ty, "T" | "t" | "D" | "d" | "B" | "b" | "R" | "r" | "W" | "V" | "G" | "S") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Toolchain/runtime symbols that are not part of the library's API surface and
/// that the C and Rust toolchains legitimately emit differently.
fn is_toolchain_symbol(name: &str) -> bool {
    name.starts_with("_ITM_")
        || name.starts_with("__cxa_")
        || name.starts_with("_Unwind_")
        || name.starts_with("__gnu_")
        || name.starts_with("rust_")
        || name.starts_with("_R")            // Rust v0 mangled internals
        || name.starts_with("__rust_")
        || name == "__gmon_start__"
        || name == "_init"
        || name == "_fini"
        || name == "__bss_start"
        || name == "_edata"
        || name == "_end"
}

#[test]
fn phase_d_every_c_symbol_is_exported_by_rust() {
    let c_so = c_so_path();
    let r_so = rust_so_path();
    assert!(c_so.exists(), "missing C .so at {}", c_so.display());
    assert!(r_so.exists(), "missing Rust .so at {}", r_so.display());

    let c_syms: BTreeSet<String> = defined_symbols(&c_so)
        .into_iter()
        .filter(|s| !is_toolchain_symbol(s))
        .collect();
    let r_syms: BTreeSet<String> = defined_symbols(&r_so)
        .into_iter()
        .filter(|s| !is_toolchain_symbol(s))
        .collect();

    // The C library must expose at least the one documented entry point.
    assert!(
        c_syms.contains("driver"),
        "sanity: C .so does not define `driver`; found {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C symbols:    {c_syms:?}\n\
         Rust symbols: {r_syms:?}",
        missing.len()
    );
}

/// The Rust `.so` must not leak extra public API either -- in particular the
/// `static` C helper `print_hex` must stay internal.
#[test]
fn phase_d_rust_exports_no_extra_public_api() {
    let c_syms: BTreeSet<String> = defined_symbols(&c_so_path())
        .into_iter()
        .filter(|s| !is_toolchain_symbol(s))
        .collect();
    let r_syms: BTreeSet<String> = defined_symbols(&rust_so_path())
        .into_iter()
        .filter(|s| !is_toolchain_symbol(s))
        .collect();

    let extra: Vec<&String> = r_syms.difference(&c_syms).collect();
    assert!(
        extra.is_empty(),
        "Rust .so exports symbols the C .so does not: {extra:?}"
    );
    assert!(
        !r_syms.contains("print_hex"),
        "Rust .so exports the internal helper `print_hex`"
    );
}
