//! Phase D — symbol parity between the two shared objects.
//!
//! Enforced as a test so the parity claim in `SYMBOLS.md` is checked mechanically
//! on every run rather than being a one-off manual observation.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

/// Defined dynamic symbols of a `.so`, from `nm -D --defined-only`.
fn defined_dynamic_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let (Some(_addr), Some(kind), Some(name)) =
                (parts.next(), parts.next(), parts.next())
            else {
                return None;
            };
            // Global/weak text and data symbols; skip the linker's own bookkeeping.
            if !matches!(kind, "T" | "t" | "D" | "B" | "W" | "V" | "R") {
                return None;
            }
            if name.starts_with("_init")
                || name.starts_with("_fini")
                || name.starts_with("__bss_start")
                || name.starts_with("_edata")
                || name.starts_with("_end")
            {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

/// Every symbol the C `.so` exports must be exported by the Rust `.so` under the
/// exact same name. The diff must be empty in that direction.
#[test]
fn d_symbol_parity_c_subset_of_rust() {
    let c_syms = defined_dynamic_symbols(&common::c_so_path());
    let rust_syms = defined_dynamic_symbols(&common::rust_so_path());

    assert!(
        c_syms.contains("driver"),
        "sanity check: the C .so should export `driver`, got {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}\n\
         C: {c_syms:?}\nRust: {rust_syms:?}"
    );
}

/// The Rust `.so` must not export anything the C `.so` does not — in particular
/// it must not leak the `static` (C-internal) `print_hex`.
#[test]
fn d_symbol_parity_no_extra_rust_exports() {
    let c_syms = defined_dynamic_symbols(&common::c_so_path());
    let rust_syms = defined_dynamic_symbols(&common::rust_so_path());

    let extra: Vec<&String> = rust_syms.difference(&c_syms).collect();
    assert!(
        extra.is_empty(),
        "symbols exported by the Rust .so that the C .so does not export: {extra:?}"
    );
}

/// The Rust `.so` must not have undefined references outside libc / the toolchain
/// runtime — an undefined non-libc symbol would mean part of the C source was
/// never translated and is being imported from somewhere else.
#[test]
fn d_no_undefined_non_libc_symbols() {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(common::rust_so_path())
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed");

    let known_prefixes = [
        "_Unwind_",
        "_ITM_",
        "__cxa_",
        "__gmon_start__",
        "__errno_location",
        "__tls_get_addr",
        "__libc_",
        "rust_eh_personality",
    ];
    let known_libc = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "fstat64",
        "getcwd", "getenv", "gettid", "lseek", "lseek64", "malloc", "memcmp", "memcpy",
        "memmove", "memset", "mmap", "mmap64", "munmap", "open", "open64", "posix_memalign",
        "printf", "pthread_key_create", "pthread_key_delete", "pthread_getspecific",
        "pthread_setspecific", "putchar", "puts", "read", "readlink", "realloc", "realpath",
        "stat", "stat64", "statx", "strlen", "syscall", "write", "writev", "fflush", "fwrite",
        "sysconf", "pthread_self", "pthread_mutex_lock", "pthread_mutex_unlock",
        "pthread_mutex_trylock", "pthread_rwlock_rdlock", "pthread_rwlock_unlock",
        "pthread_rwlock_wrlock", "pthread_condattr_init", "pthread_condattr_setclock",
        "pthread_cond_init", "pthread_cond_destroy", "pthread_condattr_destroy",
        "pthread_mutex_destroy", "pthread_mutexattr_init", "pthread_mutexattr_settype",
        "pthread_mutexattr_destroy", "pthread_cond_wait", "pthread_cond_timedwait",
        "pthread_cond_signal", "pthread_cond_broadcast", "pthread_detach", "pthread_join",
        "pthread_create", "pthread_attr_init", "pthread_attr_destroy",
        "pthread_attr_setstacksize", "sigaltstack", "sigaction", "sigemptyset", "sigaddset",
        "mprotect", "getpid", "abort", "exit", "poll", "signal", "raise", "environ",
    ];

    let mut unexpected = Vec::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let mut parts = line.split_whitespace();
        let (Some(kind), Some(name)) = (parts.next(), parts.next()) else {
            continue;
        };
        // Weak undefined symbols resolve to null harmlessly; only `U` matters.
        if kind != "U" {
            continue;
        }
        let bare = name.split('@').next().unwrap_or(name);
        if known_prefixes.iter().any(|p| bare.starts_with(p)) || known_libc.contains(&bare) {
            continue;
        }
        unexpected.push(bare.to_string());
    }

    assert!(
        unexpected.is_empty(),
        "Rust .so has undefined non-libc symbols, which suggests untranslated C: {unexpected:?}"
    );
}
