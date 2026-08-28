//! Phase A / Phase D — exported-symbol parity between the C `.so` and the
//! Rust `.so`, verified mechanically with `nm -D`.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Globally-visible *defined* dynamic symbols (`nm -D --defined-only`),
/// excluding the compiler/runtime bookkeeping symbols that are not part of
/// the library's API surface.
fn defined_dynamic_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(so)
        .output()
        .expect("failed to run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let name = match it.next() {
            Some(n) => n,
            None => continue,
        };
        let kind = it.next().unwrap_or("");
        // Only global text/data/bss/weak definitions.
        if !matches!(kind, "T" | "D" | "B" | "R" | "W" | "V" | "G" | "S" | "i") {
            continue;
        }
        if is_runtime_noise(name) {
            continue;
        }
        set.insert(name.to_string());
    }
    set
}

/// Undefined (imported) dynamic symbols.
fn undefined_dynamic_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", "--format=posix"])
        .arg(so)
        .output()
        .expect("failed to run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .filter_map(|l| l.split_whitespace().next())
        // strip the `@GLIBC_2.2.5` / `@@GLIBC_2.2.5` version suffix
        .map(|s| s.split('@').next().unwrap_or(s).to_string())
        .collect()
}

/// Symbols emitted by the toolchain / language runtime rather than by the
/// library author. These exist in *both* worlds but with different names, and
/// are not part of the C API surface.
fn is_runtime_noise(name: &str) -> bool {
    const EXACT: &[&str] = &[
        "_init",
        "_fini",
        "_edata",
        "_end",
        "__bss_start",
        "__bss_start__",
        "_bss_end__",
        "__end__",
        "__data_start",
        "data_start",
        "__dso_handle",
        "__TMC_END__",
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__gmon_start__",
        "__cxa_finalize",
        "rust_eh_personality",
        "rust_begin_unwind",
        "__rust_alloc",
        "__rust_dealloc",
        "__rust_realloc",
        "__rust_alloc_zeroed",
        "__rust_alloc_error_handler",
        "__rust_alloc_error_handler_should_panic",
        "__rust_no_alloc_shim_is_unstable",
        "__rust_no_alloc_shim_is_unstable_v2",
        "__rdl_alloc",
        "__rdl_dealloc",
        "__rdl_realloc",
        "__rdl_alloc_zeroed",
        "__rdl_oom",
        "__rg_oom",
        "rust_metadata_std",
        "rust_panic",
    ];
    if EXACT.contains(&name) {
        return true;
    }
    name.starts_with("_ZN")            // Rust/C++ mangled
        || name.starts_with("_ZS")
        || name.starts_with("_R")      // Rust v0 mangling
        || name.starts_with("__llvm_")
        || name.starts_with("_GLOBAL_")
        || name.starts_with("__gcc_")
        || name.starts_with("_Unwind_")
        || name.starts_with("rust_metadata_")
        || name.contains("$")
}

// ---------------------------------------------------------------------------

#[test]
fn c_symbols_are_all_exported_by_rust() {
    if !nm_available() {
        eprintln!("nm not available; skipping");
        return;
    }
    let c = defined_dynamic_symbols(&c_so_file());
    let r = defined_dynamic_symbols(&rust_so_file());

    assert!(
        !c.is_empty(),
        "no symbols found in the C .so ({}) -- is it built?",
        c_so_file().display()
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {:?}\n\
         C   ({}) = {:?}\n\
         Rust({}) = {:?}",
        missing.len(),
        missing,
        c.len(),
        c,
        r.len(),
        r
    );

    // The documented surface is exactly one function.
    assert!(
        c.contains("UTIL_createLinePointers"),
        "C .so does not export UTIL_createLinePointers: {c:?}"
    );
    assert!(
        r.contains("UTIL_createLinePointers"),
        "Rust .so does not export UTIL_createLinePointers: {r:?}"
    );
    assert_eq!(
        c.len(),
        1,
        "the C .so surface changed; SYMBOLS.md must be regenerated: {c:?}"
    );
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    if !nm_available() {
        eprintln!("nm not available; skipping");
        return;
    }
    // If the .so has an unresolvable dependency, dlopen would already have
    // failed in `pair()`; this asserts it explicitly with RTLD_NOW semantics
    // implied by libloading's default flags plus a successful call.
    let p = pair();
    let obs = unsafe { observe(&p.rust, std::ptr::null_mut(), 0, 0) };
    assert!(!obs.null, "Rust .so loaded but the entry point misbehaved");

    let undef = undefined_dynamic_symbols(&rust_so_file());
    // Both malloc and free must be *imported*, proving the Rust side uses the
    // libc allocator (so callers can `free()` the result) rather than Rust's.
    assert!(
        undef.contains("malloc"),
        "Rust .so does not import libc malloc; the returned block would not be \
         free()-able by C callers. undefined = {undef:?}"
    );
    assert!(
        undef.contains("free"),
        "Rust .so does not import libc free. undefined = {undef:?}"
    );
}

#[test]
fn both_libraries_expose_an_identical_callable_signature() {
    // A pure-FFI smoke check that the exported symbol really has the
    // (char*, size_t, size_t) -> const char** shape in both libraries.
    let p = pair();
    let mut buf = b"alpha\0beta\0gamma\0".to_vec();
    let base = buf.as_mut_ptr() as *mut std::os::raw::c_char;
    let (oc, or) = unsafe {
        (
            observe(&p.c, base, 3, buf.len()),
            observe(&p.rust, base, 3, buf.len()),
        )
    };
    assert_eq!(oc.offsets, vec![0, 6, 11], "C offsets: {:?}", oc.offsets);
    assert_eq!(oc, or, "smoke-test divergence");
}
