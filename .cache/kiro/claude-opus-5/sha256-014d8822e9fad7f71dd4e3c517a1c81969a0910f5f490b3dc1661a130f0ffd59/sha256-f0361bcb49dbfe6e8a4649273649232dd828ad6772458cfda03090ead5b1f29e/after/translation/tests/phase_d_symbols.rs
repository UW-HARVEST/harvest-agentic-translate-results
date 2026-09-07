//! Phase D — symbol parity between the C `.so` and the Rust `cdylib`.
//!
//! Shells out to `nm -D` on both objects and asserts the exported symbol sets
//! match exactly: every C symbol present in Rust, and no internal-linkage C
//! `static` leaking out of the Rust side either.

mod common;

use common::Pair;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

/// Defined, dynamically-exported symbols, minus the toolchain/libc runtime
/// bookkeeping that both objects carry but which is not part of the API.
fn exported_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("failed to run `nm` (binutils required)");
    assert!(
        out.status.success(),
        "nm failed on {so:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let _addr = it.next();
        let kind = match it.next() {
            Some(k) => k,
            None => continue,
        };
        let name = match it.next() {
            Some(n) => n,
            None => continue,
        };
        // Only global text/data/bss; skip weak/local/absolute runtime entries.
        if !matches!(kind, "T" | "D" | "B" | "R") {
            continue;
        }
        // Toolchain/runtime plumbing, not API.
        if name.starts_with("__")
            || name.starts_with("_ZN")
            || name.starts_with("_R")
            || name == "_init"
            || name == "_fini"
            || name == "_edata"
            || name == "_end"
        {
            continue;
        }
        set.insert(name.to_string());
    }
    set
}

fn c_so() -> PathBuf {
    let build = workspace_root().join("c_src").join("build");
    let mut found = None;
    for e in std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {build:?}: {e}. Build the C library first."))
        .flatten()
    {
        let p = e.path();
        let n = p.file_name().unwrap_or_default().to_string_lossy().to_string();
        if n.starts_with("lib") && n.ends_with(".so") {
            found = Some(p);
        }
    }
    found.expect("no lib*.so in c_src/build")
}

fn rust_so() -> PathBuf {
    // Go through the shared harness so the cdylib is rebuilt and the staleness
    // check runs before `nm` inspects it.
    common::rust_so_path()
}

#[test]
fn symbol_sets_are_identical() {
    let c = exported_symbols(&c_so());
    let r = exported_symbols(&rust_so());

    assert!(
        c.contains("jumpnode"),
        "sanity: C .so must export `jumpnode`, got {c:?}"
    );

    let missing_in_rust: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing_in_rust.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing_in_rust:?}\n\
         C   = {c:?}\nRust = {r:?}"
    );

    let extra_in_rust: Vec<&String> = r.difference(&c).collect();
    assert!(
        extra_in_rust.is_empty(),
        "the Rust .so exports symbols the C .so does not (internal-linkage leak?): \
         {extra_in_rust:?}\nC   = {c:?}\nRust = {r:?}"
    );

    assert_eq!(c, r, "exported symbol sets diverge");
}

/// The C `static` helpers have internal linkage and must not be exported by
/// either object. In particular the `#[cfg(test)]` probe module in src/lib.rs
/// must never reach a real build.
#[test]
fn internal_statics_are_not_exported() {
    let c = exported_symbols(&c_so());
    let r = exported_symbols(&rust_so());
    for name in [
        "find_node_by_id",
        "add_node",
        "process_backward",
        "compute_size_metric",
        "safe_double_to_int",
        "initialize_test_data",
        "node_storage",
        "node_count",
        "c_sprintf_node_depth",
        "c_strlen",
    ] {
        assert!(!c.contains(name), "C .so unexpectedly exports `{name}`");
        assert!(
            !r.contains(name),
            "Rust .so leaks internal-linkage symbol `{name}`"
        );
    }
    for name in r.iter() {
        assert!(
            !name.starts_with("probe_") && !name.contains("probe"),
            "Rust .so leaked a test-only probe symbol: {name}"
        );
    }
}

/// Both objects must resolve `jumpnode` through `dlsym` with the same ABI.
#[test]
fn both_objects_dlsym_jumpnode() {
    let p = Pair::load();
    let (a, b) = p.call(0o3, 0, 0, 0);
    assert_eq!(a, b);
    assert_eq!(a, 36);
}

/// No undefined non-libc symbols in the Rust object (nothing left unimplemented
/// that would only fail at call time).
#[test]
fn rust_so_has_no_unexpected_undefined_symbols() {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--undefined-only")
        .arg(rust_so())
        .output()
        .expect("nm failed");
    let text = String::from_utf8_lossy(&out.stdout);
    let allowed_prefixes = [
        "__", "_ITM_", "_Unwind", "memcpy", "memset", "memmove", "memcmp", "malloc", "free",
        "realloc", "calloc", "abort", "sqrt", "strlen", "write", "getenv", "dl_iterate_phdr",
        "pthread_", "syscall", "sigaltstack", "mmap", "munmap", "mprotect", "poll", "readlink",
        "gnu_get_libc_version", "posix_memalign", "bcmp", "close", "open", "open64", "read",
        "statx", "sysconf", "getauxval",
    ];
    let mut unexpected = Vec::new();
    for line in text.lines() {
        let name = line.split_whitespace().last().unwrap_or("");
        if name.is_empty() {
            continue;
        }
        // A version tag (`foo@GLIBC_2.x`) means the symbol is satisfied by the
        // C runtime, which is by definition not a missing translation.
        if name.contains("@GLIBC") || name.contains("@GCC") || name.contains("@CXXABI") {
            continue;
        }
        if allowed_prefixes.iter().any(|p| name.starts_with(p)) {
            continue;
        }
        unexpected.push(name.to_string());
    }
    assert!(
        unexpected.is_empty(),
        "Rust .so has undefined non-libc symbols (unimplemented/missing impl?): {unexpected:?}"
    );
}
