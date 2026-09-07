//! Phase D — symbol parity checked programmatically against the two `.so`
//! files, plus a dlsym probe that the Rust export is reachable by name.

mod harness;

use std::process::Command;

fn defined_dynamic_symbols(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(path)
        .output()
        .expect("failed to run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut syms: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next().map(|s| s.to_string()))
        .filter(|s| !s.is_empty())
        .collect();
    syms.sort();
    syms.dedup();
    syms
}

/// Every symbol exported by the C `.so` must also be exported by the Rust
/// `.so`, with the exact same name. The diff must be empty.
#[test]
fn symbol_parity_c_subset_of_rust() {
    let l = harness::libs();
    let c_syms = defined_dynamic_symbols(&l.c_path);
    let rs_syms = defined_dynamic_symbols(&l.rs_path);

    assert!(
        !c_syms.is_empty(),
        "nm reported no exported symbols for the C .so ({})",
        l.c_path.display()
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !rs_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by C but missing from Rust: {missing:?}\n  C:    {c_syms:?}\n  Rust: {rs_syms:?}"
    );
}

/// The single documented public symbol must be resolvable via `dlsym` in both
/// objects (this is what every differential test relies on).
#[test]
fn dataentry_resolvable_in_both() {
    let _c = harness::c_fn();
    let _r = harness::rs_fn();
}

/// No undefined non-libc/libgcc symbols in the Rust `.so`.
#[test]
fn rust_so_has_no_foreign_undefined_symbols() {
    let l = harness::libs();
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", "--format=posix"])
        .arg(&l.rs_path)
        .output()
        .expect("failed to run nm");
    assert!(out.status.success(), "nm failed");
    let text = String::from_utf8_lossy(&out.stdout);
    let suspicious: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .filter(|s| !s.is_empty())
        .filter(|s| {
            // Everything the Rust std runtime legitimately imports from the
            // platform C library / unwinder.
            !(s.starts_with("__")
                || s.starts_with("_ITM_")
                || s.starts_with("_Unwind_")
                || s.contains("@GLIBC")
                || s.contains("@GCC")
                || matches!(
                    *s,
                    "abort"
                        | "calloc"
                        | "close"
                        | "dl_iterate_phdr"
                        | "dlsym"
                        | "free"
                        | "getenv"
                        | "malloc"
                        | "memcmp"
                        | "memcpy"
                        | "memmove"
                        | "memset"
                        | "posix_memalign"
                        | "realloc"
                        | "strlen"
                        | "write"
                        | "writev"
                ))
        })
        .collect();
    assert!(
        suspicious.is_empty(),
        "unexpected undefined symbols in Rust .so: {suspicious:?}"
    );
}
