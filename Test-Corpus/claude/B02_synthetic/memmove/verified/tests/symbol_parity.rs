//! Phase D — symbol parity between the C and the Rust shared object.
//!
//! Every symbol *defined* (exported) by `libcdriver.so` must also be defined by
//! `libdriver.so` under the exact same name, and both objects must be loadable
//! (which proves that no undefined non-libc symbol is left dangling).

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

/// Linker / CRT bookkeeping entries that every ELF shared object carries and
/// that are not part of any library's API.
const NOISE: &[&str] = &[
    "_init",
    "_fini",
    "_edata",
    "_end",
    "__bss_start",
    "_ITM_deregisterTMCloneTable",
    "_ITM_registerTMCloneTable",
    "__gmon_start__",
    "__cxa_finalize",
    "__cxa_thread_atexit_impl",
    "_IO_stdin_used",
    "__libc_start_main",
];

/// Symbols that are part of the *Rust language runtime* rather than of the
/// translated library's API.  They only ever appear on the Rust side, so they
/// can never hide a missing C symbol.
fn is_rust_runtime(name: &str) -> bool {
    name.starts_with("_ZN")
        || name.starts_with("_R")
        || name.starts_with("__rust")
        || name.starts_with("rust_")
        || name.starts_with("__rdl_")
        || name.starts_with("__rg_")
        || name == "rust_eh_personality"
}

fn defined_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("nm on {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let (kind, name) = match (it.next(), it.next(), it.next()) {
            // "<addr> <kind> <name>"
            (Some(_addr), Some(k), Some(n)) => (k, n),
            // "         <kind> <name>"  (weak / absolute without address)
            (Some(k), Some(n), None) => (k, n),
            _ => continue,
        };
        // Only global/weak *data or text* definitions matter.
        if !matches!(kind, "T" | "t" | "D" | "d" | "B" | "b" | "R" | "r" | "W" | "w" | "V" | "v" | "A" | "i")
        {
            continue;
        }
        let name = name.split('@').next().unwrap_or(name);
        if NOISE.contains(&name) {
            continue;
        }
        set.insert(name.to_string());
    }
    set
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c = defined_symbols(&c_so_path());
    let rust = defined_symbols(&rust_so_path());

    // Sanity: the C object must at least export the documented entry point.
    assert!(
        c.contains("process_buffer"),
        "the C .so does not export process_buffer: {c:?}"
    );

    let missing: Vec<&String> = c.difference(&rust).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}\n\
         C   ({} symbols): {c:?}\n\
         Rust({} symbols, Rust-runtime names elided): {:?}",
        c.len(),
        rust.len(),
        rust.iter().filter(|n| !is_rust_runtime(n)).collect::<Vec<_>>()
    );

    println!("C .so exports {} symbol(s): {c:?}", c.len());
    println!(
        "Rust .so exports the same plus the Rust runtime; non-runtime names: {:?}",
        rust.iter().filter(|n| !is_rust_runtime(n)).collect::<Vec<_>>()
    );
}

#[test]
fn both_objects_dlopen_and_resolve_process_buffer() {
    // Loading exercises the dynamic linker: a dangling undefined symbol would
    // make `dlopen` fail here.
    let c = c_process_buffer();
    let r = rust_process_buffer();
    let mut buf = [1u8, 1, 1, 2, 2, 3];
    let a = unsafe { c(buf.as_mut_ptr(), 6, 0x02, 2, 0) };
    let mut buf2 = [1u8, 1, 1, 2, 2, 3];
    let b = unsafe { r(buf2.as_mut_ptr(), 6, 0x02, 2, 0) };
    assert_eq!(a, b);
    assert_eq!(buf, buf2);
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    // `nm -D -u` lists imports; each one must come from a versioned system
    // library (they all carry an `@GLIBC_*` / `@GCC_*` version tag) or be a weak
    // optional symbol.  Anything else would mean a missing translation unit.
    let out = Command::new("nm")
        .args(["-D", "-u"])
        .arg(rust_so_path())
        .output()
        .expect("nm -D -u");
    let text = String::from_utf8_lossy(&out.stdout);
    let mut suspicious = Vec::new();
    for line in text.lines() {
        let name = match line.split_whitespace().last() {
            Some(n) => n,
            None => continue,
        };
        let bare = name.split('@').next().unwrap_or(name);
        if name.contains('@') || NOISE.contains(&bare) {
            continue;
        }
        // Weak, unversioned, optional glibc/pthread hooks are fine.
        if bare.starts_with("__") || bare.starts_with("_ITM") {
            continue;
        }
        suspicious.push(name.to_string());
    }
    assert!(
        suspicious.is_empty(),
        "Rust .so has unresolved non-libc imports: {suspicious:?}"
    );
}
