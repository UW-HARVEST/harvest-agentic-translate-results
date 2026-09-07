//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Asserts mechanically (via `nm -D`) that every symbol exported by the C
//! shared object is also exported by the Rust shared object with the exact same
//! name, and that the Rust `.so` has no undefined non-libc symbols.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

fn nm(path: &std::path::Path, args: &[&str]) -> String {
    let out = Command::new("nm")
        .args(args)
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm: {e}"));
    assert!(out.status.success(), "nm failed on {}: {:?}", path.display(), out.status);
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Global (uppercase) defined symbols in the dynamic symbol table.
fn defined_globals(path: &std::path::Path) -> BTreeSet<String> {
    nm(path, &["-D", "--defined-only"])
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            // Uppercase type letter == global. Skip Rust/compiler internals that
            // are not part of the C ABI surface.
            if kind.chars().all(|c| c.is_uppercase()) { Some(name.to_string()) } else { None }
        })
        .filter(|n| !n.starts_with("_ITM_") && !n.starts_with("__gmon"))
        .collect()
}

/// Undefined (imported) symbols, with any `@GLIBC_x.y` / `@GCC_x.y` version
/// suffix stripped so the allow-list below has to name the symbol itself.
fn undefined_symbols(path: &std::path::Path) -> BTreeSet<String> {
    nm(path, &["-D", "--undefined-only"])
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .map(|s| s.split('@').next().unwrap_or(s).to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Symbols that legitimately resolve out of the platform C runtime.
fn is_libc_or_runtime(name: &str) -> bool {
    const LIBC: &[&str] = &[
        "malloc", "calloc", "realloc", "free", "memcpy", "memmove", "memset", "memcmp", "strlen",
        "abort", "exit", "write", "writev", "dl_iterate_phdr", "getenv", "sysconf", "poll",
        "pthread_self", "pthread_getattr_np", "pthread_attr_getstack", "pthread_attr_destroy",
        "pthread_mutex_lock", "pthread_mutex_unlock", "pthread_mutex_trylock",
        "pthread_mutex_destroy", "pthread_rwlock_rdlock", "pthread_rwlock_unlock",
        "pthread_key_create", "pthread_key_delete", "pthread_getspecific", "pthread_setspecific",
        "pthread_condattr_init", "pthread_condattr_setclock", "pthread_cond_init",
        "pthread_cond_wait", "pthread_cond_timedwait", "pthread_cond_signal",
        "pthread_cond_broadcast", "pthread_cond_destroy", "pthread_attr_init",
        "pthread_attr_setstacksize", "pthread_create", "pthread_join", "pthread_detach",
        "pthread_sigmask", "sigaction", "sigaltstack", "sigemptyset", "sigaddset",
        "mmap", "mmap64", "munmap", "mprotect", "madvise", "open", "open64", "close", "read",
        "readlink", "stat", "stat64", "fstat", "fstat64", "lseek", "lseek64", "getcwd",
        "gettimeofday", "clock_gettime", "nanosleep", "sched_yield", "sched_getaffinity",
        "syscall", "prctl", "getpid", "gettid", "strerror_r", "__errno_location", "environ",
        "posix_memalign", "aligned_alloc", "realpath", "memrchr", "strnlen", "getauxval",
        "__libc_start_main", "__cxa_thread_atexit_impl", "__cxa_finalize", "__tls_get_addr",
        "__stack_chk_fail", "__assert_fail", "__memcpy_chk", "__snprintf_chk", "__vsnprintf_chk",
        "__register_atfork", "_Unwind_Resume", "_Unwind_Backtrace", "_Unwind_GetIP",
        "_Unwind_GetIPInfo", "_Unwind_RaiseException", "_Unwind_DeleteException",
        "_Unwind_GetLanguageSpecificData", "_Unwind_GetRegionStart", "_Unwind_GetTextRelBase",
        "_Unwind_GetDataRelBase", "_Unwind_SetGR", "_Unwind_SetIP", "_Unwind_GetCFA",
        "_Unwind_FindEnclosingFunction", "_Unwind_Find_FDE", "_Unwind_GetGR",
        "__libc_current_sigrtmin", "signal", "raise", "bcmp", "strchr", "strrchr", "strncmp",
        "strcmp", "getrandom", "statx", "copy_file_range", "sendfile", "sendfile64",
    ];
    LIBC.contains(&name)
        || name.starts_with("_ITM_")
        || name.starts_with("__gmon")
        || name.starts_with("__gxx")
        || name.starts_with("_ZSt")
        || name.starts_with("GLIBC_")
        || name.starts_with("GCC_")
        || name.starts_with("__libc_")
        || name.starts_with("__pthread_")
        || name.is_empty()
}

#[test]
fn d01_every_c_symbol_is_exported_by_rust() {
    let (c_path, rs_path) = common::so_paths();
    let c_syms = defined_globals(&c_path);
    let rs_syms = defined_globals(&rs_path);

    assert!(
        c_syms.contains("custom_strdup"),
        "sanity: C .so must export custom_strdup, got {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.difference(&rs_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {missing:?}\n\
         C   ({}): {c_syms:?}\n\
         Rust({}): {rs_syms:?}",
        missing.len(),
        c_path.display(),
        rs_path.display(),
    );
}

#[test]
fn d02_rust_so_has_no_undefined_non_libc_symbols() {
    let (_c, rs_path) = common::so_paths();
    let bad: Vec<String> =
        undefined_symbols(&rs_path).into_iter().filter(|n| !is_libc_or_runtime(n)).collect();
    assert!(
        bad.is_empty(),
        "Rust .so has undefined non-libc symbols (would fail to load for a C consumer): {bad:?}"
    );
}

#[test]
fn d03_no_binary_executable_in_either_project() {
    // Recorded mechanically: CMakeLists.txt declares only add_library(), and
    // Cargo.toml declares only a cdylib. If either ever grows an executable,
    // this test fails and the stdout-comparison requirement becomes live.
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cmake = std::fs::read_to_string(root.parent().unwrap().join("c_src/CMakeLists.txt"))
        .expect("read CMakeLists.txt");
    assert!(
        !cmake.contains("add_executable"),
        "c_src now builds an executable; C/Rust stdout must be compared byte-for-byte"
    );
    let cargo = std::fs::read_to_string(root.join("Cargo.toml")).expect("read Cargo.toml");
    assert!(!cargo.contains("[[bin]]"), "translation now builds a binary; compare stdout too");
    assert!(
        !root.join("src/main.rs").exists(),
        "translation now has src/main.rs; compare stdout too"
    );
}

#[test]
fn d04_no_cargo_features_beyond_default() {
    // Records that the feature axis really is a single point (see CONFIGS.md).
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cargo = std::fs::read_to_string(root.join("Cargo.toml")).expect("read Cargo.toml");
    assert!(
        !cargo.contains("[features]"),
        "Cargo.toml grew a [features] table; Phases B-C must be re-run per feature combination"
    );
}
