//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Asserts that every symbol *defined and exported* by the C shared library is
//! also exported by the Rust shared library under the exact same name, and that
//! the Rust `.so` has no unresolved non-libc undefined symbols.

mod common;

use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn rust_so() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    let deps = exe.parent().unwrap();
    for cand in [
        deps.join("libdriver.so"),
        deps.parent().unwrap().join("libdriver.so"),
    ] {
        if cand.exists() {
            return cand;
        }
    }
    panic!("Rust cdylib not found");
}

/// Returns (defined_symbols, undefined_symbols) from `nm -D`.
fn nm(path: &PathBuf) -> (Vec<String>, Vec<String>) {
    let out = Command::new("nm")
        .arg("-D")
        .arg(path)
        .output()
        .expect("failed to run nm");
    assert!(out.status.success(), "nm failed on {path:?}");
    let text = String::from_utf8_lossy(&out.stdout);
    let mut defined = Vec::new();
    let mut undefined = Vec::new();
    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 2 {
            continue;
        }
        let (kind, name) = (cols[cols.len() - 2], cols[cols.len() - 1]);
        if kind.len() != 1 {
            continue;
        }
        match kind {
            "U" => undefined.push(name.to_string()),
            _ => defined.push(name.to_string()),
        }
    }
    defined.sort();
    defined.dedup();
    undefined.sort();
    undefined.dedup();
    (defined, undefined)
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c = workspace_root().join("c_src/build/libdriver.so");
    let (c_defined, _) = nm(&c);
    let (rust_defined, _) = nm(&rust_so());

    let missing: Vec<&String> = c_defined
        .iter()
        .filter(|s| !rust_defined.contains(s))
        .collect();

    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C defined:    {c_defined:?}\n\
         Rust defined: {rust_defined:?}"
    );

    // Sanity: the actual API symbol must really be there.
    assert!(
        rust_defined.iter().any(|s| s == "UTIL_createLinePointers"),
        "Rust .so does not export UTIL_createLinePointers: {rust_defined:?}"
    );
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let (_, undefined) = nm(&rust_so());
    let allowed_prefixes = [
        "__", "_ITM", "_Unwind", "malloc", "free", "calloc", "realloc", "memcpy", "memmove",
        "memset", "memcmp", "strlen", "abort", "exit", "write", "getenv", "dl", "pthread_",
        "sysconf", "posix_memalign", "gettid", "statx", "bcmp", "syscall", "open", "close",
        "read", "poll", "sigaltstack", "sigaction", "mmap", "munmap", "mprotect", "signal",
        "raise", "getcwd", "readlink", "environ", "qsort", "strerror", "clock_gettime", "nanosleep",
    ];
    let suspicious: Vec<&String> = undefined
        .iter()
        .filter(|s| {
            // Anything version-tagged `@GLIBC_*` / `@GCC_*` is provided by the
            // platform C library or unwinder, i.e. resolved at load time.
            if s.contains("@GLIBC_") || s.contains("@GCC_") {
                return false;
            }
            let base = s.split('@').next().unwrap();
            !allowed_prefixes.iter().any(|p| base.starts_with(p))
        })
        .collect();
    assert!(
        suspicious.is_empty(),
        "Rust .so has unresolved non-libc undefined symbols: {suspicious:?}"
    );
}
