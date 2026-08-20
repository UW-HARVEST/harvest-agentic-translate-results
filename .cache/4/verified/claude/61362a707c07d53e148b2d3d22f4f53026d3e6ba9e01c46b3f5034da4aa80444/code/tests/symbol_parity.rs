// Phase D -- automated symbol-parity check (the `SYMBOLS.md` gate).
//
// Every dynamic text symbol exported by the C `.so` must also be exported by
// the Rust `.so` under the exact same name, and every one must be callable
// through `dlsym`.

mod common;

use common::*;
use std::process::Command;

fn exported_text_symbols(so: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    let text = String::from_utf8_lossy(&out.stdout);
    let mut syms: Vec<String> = text
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            // `T` = global text (function) symbol
            if kind == "T" {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect();
    syms.sort();
    syms.dedup();
    syms
}

#[test]
fn c_symbols_are_all_exported_by_rust() {
    let c_so = c_lib_path();
    let r_so = rust_lib_path();
    assert!(c_so.exists(), "missing {}", c_so.display());
    assert!(r_so.exists(), "missing {}", r_so.display());

    let c_syms = exported_text_symbols(&c_so);
    let r_syms = exported_text_symbols(&r_so);

    assert_eq!(
        c_syms.len(),
        9,
        "expected the 9 functions of c_src/src/lib.c, got {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !r_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {missing:?}\n\
         C symbols:    {c_syms:?}\n\
         Rust symbols: {r_syms:?}",
        missing.len()
    );
    println!("symbol parity OK: {} symbols\n{c_syms:#?}", c_syms.len());
}

#[test]
fn every_c_symbol_is_dlsym_resolvable_in_both() {
    // load_both() resolves all nine symbols in both libraries and panics if
    // any is absent, so simply constructing it is the assertion.
    let (c, r) = load_both();
    assert_eq!(c.name, "C");
    assert_eq!(r.name, "Rust");
}

#[test]
fn rust_so_has_no_undefined_application_symbols() {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(rust_lib_path())
        .output()
        .expect("run nm");
    let text = String::from_utf8_lossy(&out.stdout);
    // Anything that is neither a libc/libm/pthread/dl import, nor a
    // compiler-runtime (unwind / ITM / gmon / cxa) import, would be an
    // untranslated application symbol.
    let allowed_prefixes = [
        "_ITM_", "_Unwind_", "__cxa_", "__gmon_", "__errno", "__tls_", "__libc", "_dl_",
    ];
    let libc_names = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat64", "getcwd",
        "getenv", "gettid", "lseek64", "malloc", "memcpy", "memmove", "memset", "mmap64",
        "munmap", "open64", "posix_memalign", "pthread_key_create", "pthread_key_delete",
        "pthread_setspecific", "pthread_getspecific", "pthread_mutex_lock",
        "pthread_mutex_unlock", "pthread_mutex_trylock", "pthread_self", "readlink", "realloc",
        "sigaction", "sigaltstack", "strlen", "syscall", "sysconf", "write", "writev", "memrchr",
        "poll", "pthread_rwlock_rdlock", "pthread_rwlock_unlock", "stat64", "pthread_mutex_destroy",
        "pthread_condattr_init", "pthread_cond_destroy", "pthread_condattr_destroy",
        "pthread_condattr_setclock", "pthread_cond_init", "pthread_cond_signal",
        "pthread_cond_timedwait", "pthread_cond_wait", "pthread_mutexattr_init",
        "pthread_mutexattr_destroy", "pthread_mutexattr_settype", "pthread_mutex_init",
        "pthread_atfork", "pthread_detach", "pthread_create", "pthread_join", "sched_yield",
        "nanosleep", "signal", "getpid", "environ", "strerror_r", "malloc_usable_size",
        "pthread_setname_np", "pthread_getattr_np", "pthread_attr_getstack",
        "pthread_attr_destroy", "pthread_attr_init", "pthread_attr_setstacksize", "mprotect",
        "sigemptyset", "sigaddset", "pthread_sigmask", "raise", "dlsym", "__register_atfork",
    ];
    let mut suspicious: Vec<String> = Vec::new();
    for line in text.lines() {
        let name = match line.split_whitespace().last() {
            Some(n) => n,
            None => continue,
        };
        let base = name.split('@').next().unwrap_or(name);
        if base.is_empty() {
            continue;
        }
        if allowed_prefixes.iter().any(|p| base.starts_with(p)) || libc_names.contains(&base) {
            continue;
        }
        suspicious.push(base.to_string());
    }
    // Report rather than hard-fail on unknown-but-plausible libc imports; the
    // real gate is that none of the nine application functions is undefined.
    let c_syms = exported_text_symbols(&c_lib_path());
    let untranslated: Vec<&String> = suspicious
        .iter()
        .filter(|s| c_syms.contains(s))
        .collect();
    assert!(
        untranslated.is_empty(),
        "Rust .so IMPORTS application symbols instead of defining them \
         (untranslated C code): {untranslated:?}"
    );
    if !suspicious.is_empty() {
        println!("note: other undefined (assumed platform) symbols: {suspicious:?}");
    }
}
