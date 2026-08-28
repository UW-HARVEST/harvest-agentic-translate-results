//! Phase A / Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Every symbol the C `.so` exports must also be exported by the Rust `.so`
//! under the exact same name, and the Rust `.so` must not leave any non-libc
//! symbol undefined.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(args: &[&str], so: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(args)
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm: {e}"));
    assert!(
        out.status.success(),
        "nm {:?} {} failed: {}",
        args,
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .filter(|s| !s.is_empty())
        .collect()
}

/// Strip the `@GLIBC_x.y` / `@@VER` version suffix.
fn base(sym: &str) -> String {
    sym.split('@').next().unwrap_or(sym).to_string()
}

#[test]
fn exported_symbols_match_exactly() {
    let (c, r) = common::load_pair();

    let c_exp: BTreeSet<String> = nm(&["-D", "--defined-only"], &c.path)
        .into_iter()
        .map(|s| base(&s))
        .collect();
    let r_exp: BTreeSet<String> = nm(&["-D", "--defined-only"], &r.path)
        .into_iter()
        .map(|s| base(&s))
        .collect();

    println!("C exports  ({}): {:?}", c_exp.len(), c_exp);
    println!("Rust exports ({}): {:?}", r_exp.len(), r_exp);

    // The C library's entire exported ABI.
    assert_eq!(
        c_exp,
        BTreeSet::from(["contrast_ratio".to_string()]),
        "unexpected C export set — the surface map in SYMBOLS.md is stale"
    );

    let missing: Vec<&String> = c_exp.difference(&r_exp).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by C but MISSING from Rust: {missing:?}"
    );

    // Both must be dynamically resolvable, which load_pair() already proved by
    // dlsym'ing `contrast_ratio` out of each library.
    assert_eq!(c.call(common::Rgb::WHITE, common::Rgb::BLACK).is_finite(), false);
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let (_c, r) = common::load_pair();

    // Undefined symbols permitted in the Rust cdylib: libm/libc/libgcc runtime.
    // Anything else would mean a translated module referenced something that was
    // never provided.
    let allowed_exact: BTreeSet<&str> = [
        "pow", "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free",
        "fstat64", "getcwd", "getenv", "gettid", "lseek64", "malloc", "memcpy",
        "memmove", "memset", "mmap64", "munmap", "open64", "posix_memalign",
        "read", "readlink", "realloc", "realpath", "stat64", "statx", "strlen",
        "syscall", "write", "writev", "__errno_location", "__tls_get_addr",
        "__cxa_finalize", "__cxa_thread_atexit_impl", "__gmon_start__",
        "pthread_key_create", "pthread_key_delete", "pthread_setspecific",
        "pthread_getspecific", "pthread_mutex_lock", "pthread_mutex_unlock",
        "pthread_mutex_trylock", "pthread_rwlock_rdlock", "pthread_rwlock_unlock",
        "pthread_self", "sysconf", "sigaction", "sigaltstack", "mprotect",
        "pthread_attr_getstack", "pthread_getattr_np", "pthread_attr_destroy",
        "poll", "environ", "__libc_start_main",
    ]
    .into_iter()
    .collect();

    let allowed_prefix = ["_Unwind_", "_ITM_", "__rust", "rust_"];

    let undef: Vec<String> = nm(&["-D", "-u"], &r.path)
        .into_iter()
        .map(|s| base(&s))
        .collect();

    let unexpected: Vec<&String> = undef
        .iter()
        .filter(|s| {
            !allowed_exact.contains(s.as_str())
                && !allowed_prefix.iter().any(|p| s.starts_with(p))
        })
        .collect();

    println!("Rust undefined ({}): {:?}", undef.len(), undef);
    assert!(
        unexpected.is_empty(),
        "Rust .so has unresolved NON-libc symbols (missing translation?): {unexpected:?}"
    );

    // The translation must reuse the platform `pow`, exactly like the C library,
    // otherwise results cannot be guaranteed bit-identical.
    assert!(
        undef.iter().any(|s| s == "pow"),
        "Rust .so does not import libm `pow`; the C library does, so the \
         translation would be using a different implementation: {undef:?}"
    );
}
