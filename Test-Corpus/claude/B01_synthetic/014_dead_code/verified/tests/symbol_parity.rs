//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Enforces mechanically what `SYMBOLS.md` documents: every symbol *defined* by
//! the C shared object must also be exported, under the exact same name, by the
//! Rust `cdylib` — and must actually be callable through `dlsym`.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Names of all `nm -D --defined-only` symbols of a shared object.
fn defined_dynamic_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", so.display()));
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .filter(|s| !s.is_empty())
        .collect()
}

/// Linker-generated names that are not part of any API surface.
fn is_toolchain_symbol(name: &str) -> bool {
    matches!(
        name,
        "_init" | "_fini" | "_edata" | "_end" | "__bss_start" | "__gmon_start__"
    ) || name.starts_with("_ITM_")
        || name.starts_with("__cxa_")
}

#[test]
fn phase_d_rust_so_exports_every_c_symbol() {
    let c_so = common::c_so_path();
    let rust_so = common::rust_so_path();

    let c_syms = defined_dynamic_symbols(&c_so);
    let rust_syms = defined_dynamic_symbols(&rust_so);

    let missing: Vec<&String> = c_syms
        .iter()
        .filter(|s| !is_toolchain_symbol(s))
        .filter(|s| !rust_syms.contains(*s))
        .collect();

    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n  C   : {}\n  Rust: {}",
        missing.len(),
        c_so.display(),
        rust_so.display()
    );

    // The four documented API symbols must really be there.
    for expected in ["printLine", "bad", "good", "main"] {
        assert!(
            c_syms.contains(expected),
            "C .so unexpectedly lacks {expected}"
        );
        assert!(
            rust_syms.contains(expected),
            "Rust .so lacks {expected} (present in the C .so)"
        );
    }
}

#[test]
fn phase_d_every_c_symbol_is_dlsym_resolvable_in_rust() {
    let c_so = common::c_so_path();
    let rust_so = common::rust_so_path();

    let lib_c = unsafe { libloading::Library::new(&c_so) }.expect("dlopen C .so");
    let lib_r = unsafe { libloading::Library::new(&rust_so) }.expect("dlopen Rust .so");

    for name in defined_dynamic_symbols(&c_so) {
        if is_toolchain_symbol(&name) {
            continue;
        }
        let mut sym = name.clone().into_bytes();
        sym.push(0);
        let in_c = unsafe { lib_c.get::<*const ()>(&sym) }.is_ok();
        let in_r = unsafe { lib_r.get::<*const ()>(&sym) }.is_ok();
        assert!(in_c, "C .so: {name} not resolvable via dlsym");
        assert!(
            in_r,
            "Rust .so: {name} not resolvable via dlsym (the C .so exports it)"
        );
    }
}

#[test]
fn phase_d_rust_so_has_no_unresolved_non_libc_symbols() {
    let rust_so = common::rust_so_path();
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(&rust_so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed");

    // Everything the Rust .so imports must come from libc/libgcc (glibc or
    // GCC unwinder versioned symbols, or plain unversioned libc names).
    let allowed_unversioned: BTreeSet<&str> = [
        "bcmp",
        "memcpy",
        "memmove",
        "memset",
        "strlen",
        "malloc",
        "calloc",
        "realloc",
        "free",
        "abort",
        "write",
        "writev",
        "read",
        "close",
        "syscall",
    ]
    .into_iter()
    .collect();

    let mut offenders = Vec::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let Some(name) = line.split_whitespace().last() else {
            continue;
        };
        if name.contains("@GLIBC")
            || name.contains("@GCC")
            || name.starts_with("_ITM_")
            || name.starts_with("__cxa_")
            || name.starts_with("_Unwind_")
            || name == "__gmon_start__"
            || allowed_unversioned.contains(name)
        {
            continue;
        }
        offenders.push(name.to_string());
    }
    assert!(
        offenders.is_empty(),
        "Rust .so has unresolved non-libc symbols: {offenders:?}"
    );
}

/// The `static` C helpers must not be exported by either side.
#[test]
fn phase_d_static_helpers_are_not_exported() {
    let c_syms = defined_dynamic_symbols(&common::c_so_path());
    let rust_syms = defined_dynamic_symbols(&common::rust_so_path());
    for hidden in ["helperBad", "helperGood"] {
        assert!(!c_syms.contains(hidden), "C .so exports {hidden}?");
        assert!(
            !rust_syms.contains(hidden),
            "Rust .so exports {hidden}, but the C `static` function is hidden"
        );
    }
}
