//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Asserts mechanically (via `nm -D --defined-only`) that every symbol the C
//! shared library exports is also exported by the Rust shared library with the
//! exact same name. The diff must be empty.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn rust_so() -> PathBuf {
    let m = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    for c in [
        m.join("target/release/libdriver.so"),
        m.join("target/debug/libdriver.so"),
    ] {
        if c.exists() {
            return c;
        }
    }
    panic!("Rust .so not built");
}

/// Global text/data symbols, excluding Rust-runtime and linker-generated noise.
fn exported(path: &PathBuf) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", path.display());
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
            // Only global code/data symbols; skip weak/local and Rust runtime bits.
            if !matches!(kind, "T" | "D" | "B" | "R") {
                return None;
            }
            if name.starts_with('_') || name.contains("rust_") || name.starts_with("__") {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

#[test]
fn symbol_parity_c_subset_of_rust() {
    let c = root().join("c_src/build/libdriver.so");
    assert!(c.exists(), "build the C .so first: {}", c.display());
    let r = rust_so();

    let c_syms = exported(&c);
    let r_syms = exported(&r);

    assert!(
        c_syms.contains("decode_base64"),
        "C .so unexpectedly missing decode_base64; got {c_syms:?}"
    );

    let missing: Vec<_> = c_syms.difference(&r_syms).cloned().collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {:?}\n\
         C exports:    {:?}\n\
         Rust exports: {:?}",
        missing.len(),
        missing,
        c_syms,
        r_syms
    );
}

/// No non-libc symbol may be left undefined in the Rust .so.
#[test]
fn rust_so_has_no_unresolved_nonlibc_symbols() {
    let r = rust_so();
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", r.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    // Anything carrying a version tag (`@GLIBC_*`, `@GCC_*`) comes from the
    // platform C runtime / unwinder and is expected. Untagged names are only
    // acceptable if they are known linker/runtime specials.
    let runtime_specials = [
        "_ITM_",
        "__gmon_start__",
        "__cxa_",
        "_Unwind_",
        "__tls_get_addr",
        "__gnu_",
        "_GLOBAL_OFFSET_TABLE_",
    ];
    let bad: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .filter(|n| {
            if n.contains("@GLIBC_") || n.contains("@GCC_") || n.contains("@CXXABI_") {
                return false; // platform C runtime
            }
            !runtime_specials.iter().any(|k| n.starts_with(k))
        })
        .collect();
    assert!(
        bad.is_empty(),
        "Rust .so has unresolved non-libc symbols: {bad:?}\nfull nm output:\n{text}"
    );
}
