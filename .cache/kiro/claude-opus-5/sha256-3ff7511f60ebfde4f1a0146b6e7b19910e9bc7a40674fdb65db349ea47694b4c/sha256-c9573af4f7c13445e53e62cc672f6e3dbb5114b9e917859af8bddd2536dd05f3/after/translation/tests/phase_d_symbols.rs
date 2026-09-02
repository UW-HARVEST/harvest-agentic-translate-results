//! Phase D — symbol parity, enforced as a test.
//!
//! Runs `nm -D` on both shared objects and requires that every defined (`T`)
//! symbol exported by the C `.so` is also exported by the Rust `.so` under the
//! exact same name, and that the Rust `.so` has no undefined non-libc symbols.

mod harness;
use harness::*;

use std::collections::BTreeSet;
use std::process::Command;

fn nm(path: &std::path::Path) -> Vec<(String, String)> {
    let out = Command::new("nm")
        .arg("-D")
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm -D {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            match f.len() {
                // "<addr> T name"
                3 => Some((f[1].to_string(), f[2].to_string())),
                // "w name" / "U name"
                2 => Some((f[0].to_string(), f[1].to_string())),
                _ => None,
            }
        })
        .collect()
}

fn defined(syms: &[(String, String)]) -> BTreeSet<String> {
    syms.iter()
        .filter(|(t, _)| matches!(t.as_str(), "T" | "t" | "D" | "B" | "R" | "W" | "V" | "i"))
        .map(|(_, n)| n.split('@').next().unwrap().to_string())
        .collect()
}

/// Known libc / language-runtime imports that a Rust `cdylib` legitimately
/// leaves undefined. Anything outside this set would mean a missing
/// translation.
fn is_runtime_import(name: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "_Unwind_",
        "__",
        "pthread_",
        "_ITM_",
        "dl_iterate_phdr",
    ];
    const EXACT: &[&str] = &[
        "abort", "bcmp", "calloc", "close", "free", "fstat", "fstat64", "getcwd", "getenv",
        "gettid", "lseek", "lseek64", "malloc", "memcmp", "memcpy", "memmove", "memset", "mmap",
        "mmap64", "munmap", "open", "open64", "posix_memalign", "read", "readlink", "realloc",
        "realpath", "stat", "stat64", "statx", "strlen", "syscall", "write", "writev", "sysconf",
        "getauxval", "pipe2", "poll", "sigaction", "sigaltstack", "mprotect", "raise", "signal",
        "dlsym", "dladdr", "qsort", "exp", "log", "pow", "sqrt", "fmaf", "fma",
    ];
    PREFIXES.iter().any(|p| name.starts_with(p)) || EXACT.contains(&name)
}

#[test]
fn phase_d_every_c_symbol_is_exported_by_rust() {
    let p = Pair::load();
    let c_syms = nm(&p.c_path);
    let rs_syms = nm(&p.rs_path);

    let c_defined = defined(&c_syms);
    let rs_defined = defined(&rs_syms);

    assert!(
        c_defined.contains("pow43"),
        "the C .so must export pow43; got {c_defined:?}"
    );

    let missing: Vec<&String> = c_defined.difference(&rs_defined).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   ({}): {c_defined:?}\n\
         Rust({}): {rs_defined:?}",
        p.c_path.display(),
        p.rs_path.display()
    );
}

#[test]
fn phase_d_rust_has_no_unexpected_undefined_symbols() {
    let p = Pair::load();
    let rs_syms = nm(&p.rs_path);
    let unexpected: Vec<&String> = rs_syms
        .iter()
        .filter(|(t, _)| t == "U")
        .map(|(_, n)| n)
        .filter(|n| !is_runtime_import(n.split('@').next().unwrap()))
        .collect();
    assert!(
        unexpected.is_empty(),
        "Rust .so has undefined non-libc symbols (a module was probably left untranslated): {unexpected:?}"
    );
}

/// The C source declares `g_pow43` `static`, so it must NOT appear as a dynamic
/// symbol in either object. Guards against accidentally widening the ABI.
#[test]
fn phase_d_static_table_is_not_exported() {
    let p = Pair::load();
    for path in [&p.c_path, &p.rs_path] {
        let syms = nm(path);
        assert!(
            !syms.iter().any(|(_, n)| n == "g_pow43" || n == "G_POW43"),
            "{} unexpectedly exports the static table",
            path.display()
        );
    }
}

/// Guards against a stubbed export: a `unimplemented!()`/constant stub would
/// return the same value for every input. Requires the Rust export to produce
/// many distinct values across the domain, matching the C's distinct-value
/// count exactly.
#[test]
fn phase_d_export_is_not_a_stub() {
    let p = Pair::load();
    let c_vals: BTreeSet<u32> = (DOMAIN_LO..=DOMAIN_HI).map(|x| p.c(x).to_bits()).collect();
    let rs_vals: BTreeSet<u32> = (DOMAIN_LO..=DOMAIN_HI).map(|x| p.rs(x).to_bits()).collect();
    assert!(
        c_vals.len() > 1000,
        "sanity: the C should produce many distinct values, got {}",
        c_vals.len()
    );
    assert_eq!(
        c_vals.len(),
        rs_vals.len(),
        "distinct-value counts differ: C {} vs Rust {} (stubbed or lossy export?)",
        c_vals.len(),
        rs_vals.len()
    );
    assert_eq!(c_vals, rs_vals, "the sets of produced values differ");
}
