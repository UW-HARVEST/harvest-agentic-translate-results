//! Phase D — symbol parity, enforced as a test.
//!
//! Uses `nm -D` on both shared objects and requires the set of *defined*
//! symbols in the Rust `.so` to be a superset of the C `.so`'s, and requires
//! every *undefined* symbol of the Rust `.so` to be resolvable libc/libgcc.

mod common;

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so() -> PathBuf {
    // Force the harness to (build and) resolve both libraries first.
    let _ = common::pair();
    manifest_dir().join("../c_src/build/libdriver.so")
}

fn rust_so() -> PathBuf {
    let _ = common::pair();
    common::rust_so_path()
}

fn nm(so: &PathBuf, flag: &str) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg(flag)
        .arg(so)
        .output()
        .expect("nm not available");
    assert!(
        out.status.success(),
        "nm -D {flag} {} failed: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .filter(|s| !s.is_empty())
        .collect()
}

#[test]
fn d_exported_symbol_parity() {
    let c = nm(&c_so(), "--defined-only");
    let r = nm(&rust_so(), "--defined-only");

    assert!(
        c.contains("driver") && c.contains("run"),
        "sanity: C .so should export driver and run, got {c:?}"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING these C exports: {missing:?}\n  C   = {c:?}\n  Rust= {r:?}"
    );
}

#[test]
fn d_no_unresolved_non_libc_imports() {
    // Everything the Rust .so imports must be provided by the libraries it
    // links (libc / libgcc_s / ld.so) — i.e. nothing from a missing module of
    // our own translation.
    let undef = nm(&rust_so(), "--undefined-only");
    let allowed_prefixes = [
        "_ITM_", "__cxa_", "__gmon_", "__errno_location", "__tls_get_addr", "_Unwind_", "statx",
        "gettid",
    ];
    let libc_names: BTreeSet<&str> = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat64", "getcwd",
        "getenv", "lseek64", "malloc", "memcpy", "memmove", "memset", "mmap64", "munmap",
        "open64", "posix_memalign", "printf", "pthread_key_create", "pthread_key_delete",
        "pthread_setspecific", "puts", "read", "readlink", "realloc", "realpath", "stat64",
        "strlen", "strtol", "syscall", "write", "writev", "sysconf", "getauxval", "qsort",
        "strerror_r", "pthread_getspecific", "pthread_mutex_lock", "pthread_mutex_unlock",
        "pthread_self", "sigaltstack", "sigaction", "mprotect", "poll", "nanosleep",
        "clock_gettime", "memrchr", "strchr", "abs",
    ]
    .into_iter()
    .collect();

    let mut suspicious = Vec::new();
    for sym in &undef {
        let base = sym.split('@').next().unwrap_or(sym);
        if allowed_prefixes.iter().any(|p| base.starts_with(p)) || libc_names.contains(base) {
            continue;
        }
        suspicious.push(sym.clone());
    }
    assert!(
        suspicious.is_empty(),
        "Rust .so has non-libc undefined symbols (untranslated module?): {suspicious:?}"
    );

    // And the loader can actually satisfy everything: `ldd -r` reports no
    // "undefined symbol" lines.
    let out = Command::new("ldd").arg("-r").arg(rust_so()).output().expect("ldd");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !text.contains("undefined symbol"),
        "ldd -r reported unresolved symbols:\n{text}"
    );
}
