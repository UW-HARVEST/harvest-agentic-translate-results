// Phase D -- symbol parity.
//
// `nm -D` on the C `.so` and on the Rust `.so` must agree: every symbol the C
// library exports has to be exported by the Rust library under the exact same
// name. The diff must be EMPTY.

mod common;
use common::*;

use std::process::Command;

fn defined_dynamic_symbols(so: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
            // exported code/data symbols only
            if kind == "T" || kind == "D" || kind == "B" || kind == "R" || kind == "W" {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

fn undefined_dynamic_symbols(so: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", so.to_str().unwrap()])
        .output()
        .expect("run nm");
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect();
    v.sort();
    v.dedup();
    v
}

/// Symbols the Rust `std` runtime imports from libc / libgcc in a `cdylib`.
fn is_libc_or_runtime(sym: &str) -> bool {
    let base = sym.split('@').next().unwrap_or(sym);
    const KNOWN: &[&str] = &[
        // weak CRT / TM hooks present in the C .so as well
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__cxa_finalize",
        "__cxa_thread_atexit_impl",
        "__gmon_start__",
        // libc
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat64", "fstatat64",
        "getcwd", "getenv", "gettid", "lseek64", "malloc", "memcmp", "memcpy", "memmove",
        "memset", "mmap64", "munmap", "open64", "openat64", "poll", "posix_memalign",
        "pthread_getattr_np", "pthread_attr_getstack", "pthread_attr_destroy", "pthread_self",
        "pthread_key_create", "pthread_key_delete", "pthread_setspecific", "pthread_getspecific",
        "pthread_mutex_lock", "pthread_mutex_unlock", "pthread_mutex_destroy", "pthread_rwlock_rdlock",
        "read", "readlink", "realloc", "realpath", "sigaction", "sigaltstack", "signal", "stat64",
        "statx", "strlen", "strncpy", "syscall", "sysconf", "write", "writev", "__errno_location",
        "__libc_start_main", "__tls_get_addr", "environ", "memrchr", "getrandom", "sysinfo",
    ];
    KNOWN.contains(&base) || base.starts_with("_Unwind_") || base.starts_with("__gxx_")
}

#[test]
fn d1_every_c_symbol_is_exported_by_rust() {
    let c_so = find_c_so();
    let rust_so = find_rust_so();
    let c_syms = defined_dynamic_symbols(&c_so);
    let r_syms = defined_dynamic_symbols(&rust_so);

    println!("C    ({}): {:?}", c_so.display(), c_syms);
    println!("Rust ({}): {:?}", rust_so.display(), r_syms);

    let missing: Vec<&String> = c_syms.iter().filter(|s| !r_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}",
        missing.len()
    );

    // sanity: the 7 functions of c_src/src/lib.c really are all there
    for want in [
        "add_node",
        "find_node_by_id",
        "get_children_count",
        "calculate_subtree_sum",
        "process_string",
        "safe_double_to_int",
        "maxnmin",
    ] {
        assert!(c_syms.iter().any(|s| s == want), "C .so lost {want}");
        assert!(r_syms.iter().any(|s| s == want), "Rust .so lost {want}");
    }
    assert_eq!(c_syms.len(), 7, "unexpected C export set: {c_syms:?}");
}

#[test]
fn d2_rust_has_no_unresolved_non_libc_imports() {
    let rust_so = find_rust_so();
    let undef = undefined_dynamic_symbols(&rust_so);
    let unexpected: Vec<&String> = undef.iter().filter(|s| !is_libc_or_runtime(s)).collect();
    assert!(
        unexpected.is_empty(),
        "Rust .so imports non-libc symbols (untranslated code?): {unexpected:?}"
    );
}

#[test]
fn d3_every_c_symbol_resolves_through_dlsym_in_both() {
    // `Pair::fresh()` resolves all 7 symbols in BOTH libraries via dlsym and
    // panics if any is missing, so simply constructing it is the assertion.
    let p = Pair::fresh();
    both_maxnmin(&p, "D3", 1, 2, 3, 4);
    both_d2i(&p, "D3", 1.5);
    both_process(&p, "D3", b"abc\0");
    both_add(&p, "D3", 1, -1, b"x", 1.0);
    both_query(&p, "D3", 1);
}
