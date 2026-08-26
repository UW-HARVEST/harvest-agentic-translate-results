//! Phase D — symbol parity.
//!
//! Asserts mechanically (via `nm -D`) that every symbol the C shared object
//! exports is also exported by the Rust shared object under the exact same name,
//! and that the Rust object has no undefined symbol outside libc / the platform
//! runtime. This is the machine-checked version of `SYMBOLS.md`.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use common::{c_impl, c_impl_o2, rust_impl, rust_so_path};

/// `(defined, undefined)` global symbol names reported by `nm -D`.
fn dynamic_symbols(path: &Path) -> (BTreeSet<String>, BTreeSet<String>) {
    let out = Command::new("nm")
        .arg("-D")
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    let text = String::from_utf8_lossy(&out.stdout);
    let mut defined = BTreeSet::new();
    let mut undefined = BTreeSet::new();

    for line in text.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        // Either "<addr> <type> <name>" or "<type> <name>".
        let (kind, name) = match fields.len() {
            3 => (fields[1], fields[2]),
            2 => (fields[0], fields[1]),
            _ => continue,
        };
        // Strip the "@GLIBC_2.x" version suffix so names compare cleanly.
        let bare = name.split('@').next().unwrap().to_string();
        match kind {
            "U" => {
                undefined.insert(bare);
            }
            // Weak-undefined symbols (`w`) are toolchain hooks, not API.
            "w" | "v" => {}
            _ => {
                defined.insert(bare);
            }
        }
    }

    (defined, undefined)
}

/// Every symbol the C `.so` exports must be exported by the Rust `.so` too.
#[test]
fn rust_so_exports_every_c_symbol() {
    let c_path = c_impl().path;
    let rust_path = rust_so_path();

    let (c_defined, _) = dynamic_symbols(&c_path);
    let (rust_defined, _) = dynamic_symbols(&rust_path);

    let missing: Vec<&String> = c_defined.difference(&rust_defined).collect();
    assert!(
        missing.is_empty(),
        "the Rust shared object is missing {} exported symbol(s): {missing:?}\n\
         C   ({}) exports: {c_defined:?}\n\
         Rust({}) exports: {rust_defined:?}",
        missing.len(),
        c_path.display(),
        rust_path.display()
    );

    // Sanity: the three documented entry points really are there.
    for expected in ["find_container_of_a", "find_container_of_b", "main"] {
        assert!(
            c_defined.contains(expected),
            "the C reference unexpectedly lacks {expected}"
        );
        assert!(
            rust_defined.contains(expected),
            "the Rust translation lacks {expected}"
        );
    }
}

/// The `-O2` build of the reference must not change the exported surface either.
#[test]
fn rust_so_exports_every_c_symbol_o2() {
    let (c_defined, _) = dynamic_symbols(&c_impl_o2().path);
    let (rust_defined, _) = dynamic_symbols(&rust_so_path());
    let missing: Vec<&String> = c_defined.difference(&rust_defined).collect();
    assert!(missing.is_empty(), "missing from the Rust .so: {missing:?}");
}

/// Nothing the Rust object imports may be outside libc / the platform runtime,
/// i.e. there must be no unresolved symbol belonging to the translated library
/// itself.
#[test]
fn rust_so_has_no_foreign_undefined_symbols() {
    let (_, undefined) = dynamic_symbols(&rust_so_path());

    let allowed_prefixes = [
        "_Unwind_", "__", "pthread_", "std", "core", "rust_", "gnu_",
    ];
    let allowed_exact = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "fstat64",
        "getcwd", "getenv", "gettid", "lseek", "lseek64", "malloc", "memcmp", "memcpy", "memmove",
        "memset", "mmap", "mmap64", "munmap", "open", "open64", "posix_memalign", "read",
        "readlink", "realloc", "realpath", "stat", "stat64", "statx", "strlen", "syscall", "write",
        "writev", "sysconf", "getrandom", "poll", "sigaction", "sigaltstack", "signal", "raise",
        "exit", "_exit", "environ", "dlsym", "dladdr", "pipe2", "fcntl", "getpid", "abs",
    ];

    let foreign: Vec<&String> = undefined
        .iter()
        .filter(|s| {
            !allowed_exact.contains(&s.as_str())
                && !allowed_prefixes.iter().any(|p| s.starts_with(p))
        })
        .collect();

    assert!(
        foreign.is_empty(),
        "the Rust shared object has undefined non-libc symbols: {foreign:?}"
    );
}

/// Both objects must resolve the three entry points through `dlsym`, which is
/// what an external consumer actually does.
#[test]
fn both_objects_resolve_entry_points_via_dlsym() {
    let c = c_impl();
    let rust = rust_impl();
    for name in [
        &b"find_container_of_a\0"[..],
        b"find_container_of_b\0",
        b"main\0",
    ] {
        assert!(c.exports(name), "C reference does not export {name:?}");
        assert!(rust.exports(name), "Rust translation does not export {name:?}");
    }
}
