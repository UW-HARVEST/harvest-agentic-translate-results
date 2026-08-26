// Phase D — symbol parity between the C and Rust shared objects.
//
// Recomputed with `nm -D` at test time so SYMBOLS.md cannot silently rot.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(path: &Path, extra: &str) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", extra])
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("cannot run nm on {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm {extra} {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .filter(|s| !s.is_empty())
        .collect()
}

/// Every symbol the C `.so` exports must also be exported by the Rust `.so`.
#[test]
fn d1_every_c_symbol_is_exported_by_rust() {
    let c_defined = nm(&c_so_path(), "--defined-only");
    let r_defined = nm(&rust_so_path(), "--defined-only");

    let missing: Vec<&String> = c_defined.difference(&r_defined).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C:    {c_defined:?}\nRust: {r_defined:?}"
    );

    // The 12 functions from SYMBOLS.md, spelled out so a wholesale loss of the
    // C library's exports could not make the diff vacuously empty.
    for name in [
        "is_valid_operation",
        "get_operation_priority",
        "add_operation",
        "multiply_operation",
        "subtract_operation",
        "divide_operation",
        "modulo_operation",
        "select_operation",
        "get_computation_timestamp",
        "allocate_results",
        "perform_computation_with_history",
        "mathop",
    ] {
        assert!(c_defined.contains(name), "C .so lost {name}");
        assert!(r_defined.contains(name), "Rust .so does not export {name}");
    }
    assert_eq!(c_defined.len(), 12, "unexpected C export set: {c_defined:?}");
}

/// Every symbol the Rust `.so` needs must be satisfiable: only libc / libgcc
/// imports are allowed (no dangling non-libc undefined symbol).
#[test]
fn d2_rust_has_no_unresolved_non_libc_symbols() {
    let undef = nm(&rust_so_path(), "--undefined-only");
    let allowed_prefixes = [
        "_ITM_", "__cxa_", "__gmon_", "_Unwind_", "__tls_get_addr", "__errno_location",
    ];
    let libc_names: BTreeSet<&str> = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "fstat64",
        "getcwd", "getenv", "gettid", "lseek", "lseek64", "malloc", "memcmp", "memcpy",
        "memmove", "memset", "mmap", "mmap64", "munmap", "open", "open64", "posix_memalign",
        "printf", "pthread_key_create", "pthread_key_delete", "pthread_getspecific",
        "pthread_setspecific", "read", "readlink", "realloc", "realpath", "stat", "stat64",
        "statx", "strlen", "syscall", "time", "write", "writev", "sigaltstack", "sigaction",
        "mprotect", "pthread_self", "pthread_getattr_np", "pthread_attr_getstack",
        "pthread_attr_destroy", "poll", "sysconf", "getpid", "__libc_start_main",
    ]
    .into_iter()
    .collect();

    let mut bad = Vec::new();
    for sym in &undef {
        let base = sym.split('@').next().unwrap_or(sym);
        if allowed_prefixes.iter().any(|p| base.starts_with(p)) {
            continue;
        }
        if libc_names.contains(base) {
            continue;
        }
        bad.push(sym.clone());
    }
    assert!(
        bad.is_empty(),
        "unresolved non-libc symbols in the Rust .so: {bad:?}"
    );

    // The three libc entry points the C translation unit itself imports must be
    // imported by the Rust object too (real libc, not a re-implementation).
    for want in ["calloc", "printf", "time"] {
        assert!(
            undef
                .iter()
                .any(|s| s.split('@').next() == Some(want)),
            "Rust .so must import libc {want} like the C .so does; undefined = {undef:?}"
        );
    }
}

/// Both libraries must resolve completely at load time (this is implicitly true
/// because `common::both()` dlopen()s them, but assert it explicitly).
#[test]
fn d3_both_libraries_load_and_expose_the_api() {
    let (c, r) = both();
    assert_eq!(c.op_addrs.len(), 5);
    assert_eq!(r.op_addrs.len(), 5);
    // Distinct addresses within each library (no accidental symbol aliasing).
    for i in 0..5 {
        for j in (i + 1)..5 {
            assert_ne!(c.op_addrs[i], c.op_addrs[j], "C aliased op symbols {i}/{j}");
            assert_ne!(
                r.op_addrs[i], r.op_addrs[j],
                "Rust aliased op symbols {i}/{j}"
            );
        }
    }
    assert!(c_so_path().exists());
    assert!(rust_so_path().exists());
}
