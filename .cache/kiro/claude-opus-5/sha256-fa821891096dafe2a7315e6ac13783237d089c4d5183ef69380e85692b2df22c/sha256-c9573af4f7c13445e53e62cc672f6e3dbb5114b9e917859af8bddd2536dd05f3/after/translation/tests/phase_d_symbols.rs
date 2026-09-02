//! Phase D — symbol parity enforced as a test.
//!
//! Compares `nm -D` output of the C `.so` and the Rust `.so` and fails if the
//! Rust library is missing any symbol the C library exports.

mod common;

use std::path::PathBuf;
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn so_in(dir: PathBuf) -> PathBuf {
    let mut v: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    v.sort();
    v.pop()
        .unwrap_or_else(|| panic!("no .so in {}", dir.display()))
}

/// (name, type-letter) pairs for defined dynamic symbols.
fn defined_symbols(so: &PathBuf) -> Vec<(String, char)> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("failed to run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    let text = String::from_utf8_lossy(&out.stdout);
    let mut v: Vec<(String, char)> = text
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let a = it.next()?;
            let b = it.next()?;
            let c = it.next();
            match c {
                // "<addr> <T> <name>"
                Some(name) => Some((name.to_string(), b.chars().next()?)),
                // "<T> <name>" (no address)
                None => Some((b.to_string(), a.chars().next()?)),
            }
        })
        .collect();
    v.sort();
    v
}

fn undefined_symbols(so: &PathBuf) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", so.to_str().unwrap()])
        .output()
        .expect("failed to run nm");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    let mut v: Vec<String> = text
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect();
    v.sort();
    v.dedup();
    v
}

/// Symbols the Rust `std` runtime legitimately imports from libc / libgcc.
fn is_runtime_import(sym: &str) -> bool {
    let base = sym.split('@').next().unwrap_or(sym);
    const KNOWN: &[&str] = &[
        "malloc", "realloc", "free", "calloc", "posix_memalign", "memcpy", "memmove", "memset",
        "bcmp", "strlen", "abort", "__errno_location", "__tls_get_addr", "pthread_key_create",
        "pthread_key_delete", "pthread_setspecific", "pthread_getspecific",
        "__cxa_thread_atexit_impl", "__cxa_finalize", "__gmon_start__",
        "_ITM_registerTMCloneTable", "_ITM_deregisterTMCloneTable", "dl_iterate_phdr", "open",
        "open64", "close", "read", "write", "writev", "lseek", "lseek64", "fstat", "fstat64",
        "stat", "stat64", "statx", "mmap", "mmap64", "munmap", "readlink", "realpath", "getcwd",
        "getenv", "syscall", "gettid", "sysconf", "getpid", "sigaltstack", "sigaction",
        "pthread_self", "pthread_getattr_np", "pthread_attr_getstack", "pthread_attr_destroy",
        "pthread_mutex_lock", "pthread_mutex_unlock", "pthread_mutex_trylock",
        "pthread_mutex_destroy", "pthread_mutexattr_init", "pthread_mutexattr_settype",
        "pthread_mutexattr_destroy", "pthread_cond_wait", "pthread_cond_signal",
        "pthread_cond_broadcast", "pthread_cond_destroy", "pthread_rwlock_rdlock",
        "pthread_rwlock_unlock", "pthread_rwlock_wrlock", "memrchr", "memchr", "strerror_r",
        "poll", "nanosleep", "clock_gettime", "sched_yield", "environ", "__libc_start_main",
    ];
    if KNOWN.contains(&base) {
        return true;
    }
    // libgcc unwinder
    base.starts_with("_Unwind_") || base.starts_with("__gcc_") || base.starts_with("_ZN")
}

#[test]
fn d1_rust_so_exports_every_c_symbol() {
    let c_so = so_in(root().join("c_src").join("build"));
    let rs_so = {
        let exe = std::env::current_exe().unwrap();
        let profile = exe.parent().and_then(|p| p.parent()).unwrap().to_path_buf();
        let p = profile.join("libmatrixsum_lib.so");
        if p.exists() {
            p
        } else {
            so_in(root().join("translation").join("target").join("release"))
        }
    };

    let c_syms = defined_symbols(&c_so);
    let rs_syms = defined_symbols(&rs_so);
    assert!(!c_syms.is_empty(), "nm found no symbols in the C .so");

    let rs_names: Vec<&str> = rs_syms.iter().map(|(n, _)| n.as_str()).collect();
    let missing: Vec<&(String, char)> = c_syms
        .iter()
        .filter(|(n, _)| !rs_names.contains(&n.as_str()))
        .collect();

    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {:?}\nC: {:?}\nRUST: {:?}",
        missing.len(),
        missing,
        c_syms,
        rs_syms
    );

    // Symbol *kind* must match too: `matrix` must stay a data object, the rest
    // must stay text.
    for (name, kind) in &c_syms {
        let rs_kind = rs_syms
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, k)| *k)
            .unwrap();
        assert_eq!(
            kind.to_ascii_uppercase(),
            rs_kind.to_ascii_uppercase(),
            "symbol `{name}` kind differs: C={kind} RUST={rs_kind}"
        );
    }

    // Expected, mechanically-derived set from c_src/src/lib.c.
    let expected = [
        "add_element",
        "calculate_matrix_checksum",
        "expand_array",
        "free_array",
        "init_array",
        "matrix",
        "matrixsum",
        "process_flags",
    ];
    for e in expected {
        assert!(
            c_syms.iter().any(|(n, _)| n == e),
            "C .so unexpectedly lacks `{e}` — is the build stale?"
        );
        assert!(
            rs_names.contains(&e),
            "Rust .so lacks `{e}`"
        );
    }
}

#[test]
fn d2_rust_so_has_no_non_libc_undefined_symbols() {
    let rs_so = {
        let exe = std::env::current_exe().unwrap();
        let profile = exe.parent().and_then(|p| p.parent()).unwrap().to_path_buf();
        let p = profile.join("libmatrixsum_lib.so");
        if p.exists() {
            p
        } else {
            so_in(root().join("translation").join("target").join("release"))
        }
    };
    let undef = undefined_symbols(&rs_so);
    let unexpected: Vec<&String> = undef.iter().filter(|s| !is_runtime_import(s)).collect();
    assert!(
        unexpected.is_empty(),
        "Rust .so has non-libc undefined symbols: {unexpected:?}"
    );
}

#[test]
fn d3_matrix_data_symbol_same_size() {
    let c_so = so_in(root().join("c_src").join("build"));
    let rs_so = {
        let exe = std::env::current_exe().unwrap();
        let profile = exe.parent().and_then(|p| p.parent()).unwrap().to_path_buf();
        let p = profile.join("libmatrixsum_lib.so");
        if p.exists() {
            p
        } else {
            so_in(root().join("translation").join("target").join("release"))
        }
    };
    let size_of = |so: &PathBuf| -> u64 {
        let out = Command::new("nm")
            .args(["-D", "-S", "--defined-only", so.to_str().unwrap()])
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            let f: Vec<&str> = line.split_whitespace().collect();
            if f.len() == 4 && f[3] == "matrix" {
                return u64::from_str_radix(f[1], 16).unwrap();
            }
        }
        panic!("no sized `matrix` symbol in {}", so.display());
    };
    assert_eq!(
        size_of(&c_so),
        size_of(&rs_so),
        "`matrix` data symbol size differs"
    );
    assert_eq!(size_of(&c_so), 48, "3*4*sizeof(int)");
}
