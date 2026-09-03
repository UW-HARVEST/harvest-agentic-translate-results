//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Runs `nm -D --defined-only` on both objects for the configuration this test
//! was built with and requires the "missing from Rust" set to be empty. Also
//! verifies each symbol is actually *usable* through `dlsym` (i.e. not merely
//! present) and has the right kind (text vs data).

mod common;

use common::{c_lib_path, pair, rust_lib_path, OP, REPEAT};
use std::collections::BTreeMap;
use std::process::Command;

/// symbol name -> nm type letter
fn dynamic_symbols(path: &std::path::Path) -> BTreeMap<String, char> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut map = BTreeMap::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() >= 3 {
            map.insert(cols[2].to_string(), cols[1].chars().next().unwrap());
        }
    }
    map
}

#[test]
fn c_so_exports_the_expected_eight_symbols() {
    let c = dynamic_symbols(&c_lib_path());
    let names: Vec<&str> = c.keys().map(|s| s.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "G_OP",
            "G_OP_NAME",
            "helper_call",
            "helper_ptr",
            "op_add",
            "op_mul",
            "op_sub",
            "use_generated"
        ],
        "the C .so surface changed [OP={} REPEAT={}]",
        OP,
        REPEAT
    );
    // `accum_<OP>` is `static` in mdcore.c and must NOT be dynamic.
    assert!(
        !c.contains_key(&format!("accum_{}", OP)),
        "accum_{} unexpectedly exported by the C .so",
        OP
    );
}

#[test]
fn every_c_symbol_is_exported_by_the_rust_so() {
    let c = dynamic_symbols(&c_lib_path());
    let r = dynamic_symbols(&rust_lib_path());

    let missing: Vec<&String> = c.keys().filter(|k| !r.contains_key(*k)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so \
         [OP={} REPEAT={}]: {:?}",
        OP,
        REPEAT,
        missing
    );

    // Same kind: `T` (code) for functions, `D` (initialised data) for the two
    // globals, so a consumer's `dlsym` + dereference sees the same shape.
    for (name, ckind) in &c {
        let rkind = r[name];
        assert_eq!(
            *ckind, rkind,
            "symbol `{}` has nm type {} in C but {} in Rust",
            name, ckind, rkind
        );
    }
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--undefined-only")
        .arg(rust_lib_path())
        .output()
        .expect("run nm");
    let undef: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .filter(|s| !s.is_empty())
        // drop the `@GLIBC_2.x` version tag
        .map(|s| s.split('@').next().unwrap().to_string())
        .collect();

    // Everything left must come from libc / the dynamic loader / libgcc.
    let allowed_prefix = [
        "_", "abort", "bcmp", "calloc", "close", "dl", "environ", "exit", "fcntl", "free",
        "getcwd", "getenv", "gettid", "malloc", "mem", "mmap", "mprotect", "munmap", "open",
        "poll", "posix_", "pthread_", "raise", "read", "realloc", "sig", "stat", "str", "sys",
        "syscall", "write", "unlink", "lseek", "readlink", "sched_", "gnu_get_libc_version",
        "getrandom", "openat", "statx", "fstat", "pipe", "dup", "sysconf", "nanosleep", "clock_",
        "cfree", "qsort", "bsearch", "atexit", "getpid", "realpath", "abs", "ftruncate", "isatty",
        "rmdir", "chdir", "fdopen", "fwrite", "fputs", "fflush", "printf", "puts", "putchar",
        "madvise", "mremap", "prctl", "gettimeofday",
    ];
    let bad: Vec<&String> = undef
        .iter()
        .filter(|s| !allowed_prefix.iter().any(|p| s.starts_with(p)))
        .collect();
    assert!(
        bad.is_empty(),
        "Rust .so has unresolved non-libc symbols [OP={} REPEAT={}]: {:?}",
        OP,
        REPEAT,
        bad
    );
}

#[test]
fn all_symbols_resolve_via_dlsym_in_both_objects() {
    let p = pair();
    for sym in [
        "op_add",
        "op_sub",
        "op_mul",
        "helper_call",
        "helper_ptr",
        "use_generated",
    ] {
        // `bin2`/`un1` panic with a clear message when dlsym fails.
        let _ = p.c.addr(sym);
        let _ = p.r.addr(sym);
    }
    for sym in ["G_OP", "G_OP_NAME"] {
        assert_ne!(p.c.addr(sym), 0);
        assert_ne!(p.r.addr(sym), 0);
    }
    // The two data slots must hold non-null payloads on both sides.
    assert_ne!(p.c.g_op() as usize, 0);
    assert_ne!(p.r.g_op() as usize, 0);
    assert_eq!(p.c.g_op_name(), p.r.g_op_name());
}
