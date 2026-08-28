//! Phase D — symbol parity, enforced as a test rather than a one-off command.
//!
//! Every dynamic symbol the C `.so` defines must also be defined by the Rust
//! `.so` under the exact same name, and must be `dlsym`-able with the right
//! calling convention.

mod common;

use common::*;
use std::path::PathBuf;
use std::process::Command;

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so() -> PathBuf {
    std::env::var("C_DRIVER_SO").map(PathBuf::from).unwrap_or_else(|_| {
        crate_root()
            .parent()
            .unwrap()
            .join("c_src/build/libdriver.so")
    })
}

fn rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().unwrap();
    let dir = exe.parent().unwrap().parent().unwrap().to_path_buf();
    let c = dir.join("libdriver.so");
    if c.exists() {
        return c;
    }
    for p in ["release", "debug"] {
        let c = crate_root().join("target").join(p).join("libdriver.so");
        if c.exists() {
            return c;
        }
    }
    panic!("Rust libdriver.so not found");
}

/// Names of the dynamic symbols DEFINED (not imported) by `so`.
fn defined_dynamic_symbols(so: &PathBuf) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm -D failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (a, b) = (it.next()?, it.next()?);
            // "<addr> <type> <name>" or "<type> <name>" for undefined/weak
            let (ty, name) = match it.next() {
                Some(n) => (b, n),
                None => (a, b),
            };
            // Skip the toolchain/loader boilerplate that is not part of the API.
            if name.starts_with("_ITM_")
                || name.starts_with("__cxa")
                || name.starts_with("__gmon")
                || name.starts_with("_init")
                || name.starts_with("_fini")
                || name.starts_with("__bss")
                || name.starts_with("_edata")
                || name.starts_with("_end")
            {
                return None;
            }
            // Only global/weak text & data definitions.
            if matches!(ty, "T" | "t" | "D" | "d" | "B" | "b" | "R" | "r" | "W" | "V") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn every_c_symbol_is_exported_by_the_rust_so() {
    let (c, r) = (c_so(), rust_so());
    let cs = defined_dynamic_symbols(&c);
    let rs = defined_dynamic_symbols(&r);

    assert!(
        !cs.is_empty(),
        "nm found no defined symbols in {} — the C library did not build",
        c.display()
    );

    let missing: Vec<&String> = cs.iter().filter(|s| !rs.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C   ({}): {cs:?}\n\
         Rust({}): {rs:?}",
        missing.len(),
        c.display(),
        r.display(),
    );

    // The three functions in c_src/src/lib.c, spelled out so that a future
    // regression in the nm parsing cannot silently make this test vacuous.
    for expected in ["get_os_arch", "w_regexec", "parse_uname_string"] {
        assert!(
            cs.iter().any(|s| s == expected),
            "C .so unexpectedly does not export {expected}: {cs:?}"
        );
        assert!(
            rs.iter().any(|s| s == expected),
            "Rust .so does not export {expected}: {rs:?}"
        );
    }
    assert_eq!(
        cs.len(),
        3,
        "the C .so's public surface changed; update SYMBOLS.md: {cs:?}"
    );
}

#[test]
fn every_symbol_is_dlsym_able_from_both_so_files() {
    // `both()` panics if any of the three symbols cannot be resolved with the
    // expected signature in either library.
    let b = both();
    assert_eq!(b.c.name, "C");
    assert_eq!(b.rs.name, "Rust");
    // Smoke-call each resolved pointer so a bogus but resolvable symbol is caught.
    diff_arch(b"x86_64", "D/dlsym");
    diff_regexec(Some(b"^([0-9]+)$"), Some(b"42"), 2, 4, "D/dlsym");
    diff_parse(b"h [D|p: 1.2 (c)] x86_64", "D/dlsym");
}

#[test]
fn rust_so_has_no_undefined_non_libc_symbols() {
    let r = rust_so();
    let out = Command::new("nm")
        .args(["-D", "-u", r.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    // Anything not satisfied by libc / libgcc_s / the loader would make the
    // `.so` unloadable; `both()` already dlopens it, which proves resolvability.
    // Here we additionally assert nothing from the C library itself is imported
    // (i.e. the Rust implementation is genuinely its own, not a thunk).
    for line in text.lines() {
        let name = line.split_whitespace().last().unwrap_or("");
        let base = name.split('@').next().unwrap_or(name);
        assert!(
            !matches!(base, "get_os_arch" | "w_regexec" | "parse_uname_string"),
            "the Rust .so IMPORTS {base} instead of defining it — it is a thunk \
             over the C library, not a translation"
        );
    }
}
