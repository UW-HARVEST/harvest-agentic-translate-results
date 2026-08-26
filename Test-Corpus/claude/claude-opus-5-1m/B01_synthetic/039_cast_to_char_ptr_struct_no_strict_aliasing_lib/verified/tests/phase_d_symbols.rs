//! Phase D — symbol parity, enforced as a test so it cannot silently rot.
//!
//! Every dynamic symbol *defined* by the C `.so` must also be defined by the
//! Rust `.so`, with the exact same name, and must be resolvable via `dlsym`.

mod common;

use common::*;
use std::process::Command;

/// Parse `nm -D --defined-only` output into the set of defined global symbol
/// names (stripping any `@GLIBC_x.y` version suffix).
fn defined_symbols(so: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("failed to run `nm`");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut syms = Vec::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let (kind, name) = match (it.next(), it.next(), it.next()) {
            // "<addr> <T> <name>"
            (Some(_addr), Some(k), Some(n)) => (k, n),
            // "         <w> <name>"  (no address)
            (Some(k), Some(n), None) => (k, n),
            _ => continue,
        };
        // Only globally-defined code/data symbols form the ABI contract.
        if !matches!(kind, "T" | "D" | "B" | "R" | "G" | "S" | "i") {
            continue;
        }
        let name = name.split('@').next().unwrap_or(name);
        syms.push(name.to_string());
    }
    syms.sort();
    syms.dedup();
    syms
}

/// Rust cdylibs export a handful of language-runtime symbols that are not part
/// of the translated API; they are allowed as *extras*, never as omissions.
fn is_rust_runtime_symbol(name: &str) -> bool {
    name.starts_with("_ZN")
        || name.starts_with("rust_")
        || name.starts_with("__rust")
        || name.starts_with("_R")
        || name.contains("$LT$")
}

#[test]
fn d01_every_c_symbol_is_exported_by_rust() {
    let l = libs();
    let c_syms = defined_symbols(&l.c_path);
    let rust_syms = defined_symbols(&l.rust_path);

    assert!(
        !c_syms.is_empty(),
        "nm found no defined symbols in the C .so — check the build"
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !rust_syms.contains(s)).collect();

    eprintln!("C defined symbols ({}): {:?}", c_syms.len(), c_syms);
    eprintln!(
        "Rust defined symbols, non-runtime ({}): {:?}",
        rust_syms.iter().filter(|s| !is_rust_runtime_symbol(s)).count(),
        rust_syms
            .iter()
            .filter(|s| !is_rust_runtime_symbol(s))
            .collect::<Vec<_>>()
    );

    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {:?}",
        missing.len(),
        missing
    );
}

#[test]
fn d02_every_c_symbol_is_dlsym_resolvable_in_both() {
    let l = libs();
    for name in defined_symbols(&l.c_path) {
        let mut key = name.clone().into_bytes();
        key.push(0);
        let in_c = unsafe { l.c.get::<*const ()>(&key) }.is_ok();
        let in_rust = unsafe { l.rust.get::<*const ()>(&key) }.is_ok();
        assert!(in_c, "`{name}` not dlsym-resolvable in the C .so");
        assert!(in_rust, "`{name}` not dlsym-resolvable in the Rust .so");
    }
}

#[test]
fn d03_rust_so_has_no_unresolved_non_libc_symbols() {
    let l = libs();
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", l.rust_path.to_str().unwrap()])
        .output()
        .expect("nm");
    let text = String::from_utf8_lossy(&out.stdout);

    // Anything the platform runtime provides: libc, libm, libgcc/unwind, ld.so,
    // pthread, and the crt glue that GCC/LLVM emit.
    let allowed_prefixes = [
        "_ITM_", "__cxa_", "__gmon_", "__libc_", "_Unwind_", "__tls_", "__gxx_",
        "__errno", "__pthread", "pthread_", "__gnu_", "__assert", "_dl_", "__dl",
        "__stack_chk", "__memcpy", "__snprintf", "__vsnprintf", "__sprintf",
        "__register_", "__deregister_", "__odr_", "_edata", "_end", "__bss_start",
        "__gcc_", "__divti3", "__udivti3", "__muloti4", "__clear_cache",
    ];

    let mut suspicious = Vec::new();
    for line in text.lines() {
        let name = match line.split_whitespace().last() {
            Some(n) => n.split('@').next().unwrap_or(n),
            None => continue,
        };
        if name.is_empty() || name == "U" || name == "w" {
            continue;
        }
        if allowed_prefixes.iter().any(|p| name.starts_with(p)) {
            continue;
        }
        // Plain lowercase libc/libm names (printf, memcpy, malloc, ...).
        if name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        {
            continue;
        }
        suspicious.push(name.to_string());
    }
    assert!(
        suspicious.is_empty(),
        "Rust .so has unresolved non-libc symbols (untranslated C module?): {suspicious:?}"
    );
}

#[test]
fn d04_c_so_defines_exactly_the_documented_surface() {
    // Pins SYMBOLS.md against drift in the C build.
    let l = libs();
    let c_syms = defined_symbols(&l.c_path);
    assert_eq!(
        c_syms,
        vec!["driver".to_string()],
        "the C ABI surface changed; update SYMBOLS.md"
    );
}
