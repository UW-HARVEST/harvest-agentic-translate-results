// Phase D — symbol parity, enforced automatically.
//
// Every symbol the C `.so` exports must be exported by the Rust `.so` under
// the exact same name, and the Rust `.so` must have no undefined non-libc
// symbols.

mod common;

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn c_so() -> PathBuf {
    if let Ok(p) = std::env::var("BUFFAPP_C_SO") {
        return PathBuf::from(p);
    }
    let dir = root().join("c_src/build");
    let mut v: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("c_src/build missing — build the C library first")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    v.sort();
    assert_eq!(v.len(), 1, "expected one C .so, got {v:?}");
    v.pop().unwrap()
}

fn rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("BUFFAPP_RUST_SO") {
        return PathBuf::from(p);
    }
    for profile in ["release", "debug"] {
        let p = root().join("translation/target").join(profile).join("libbuffapp_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("libbuffapp_lib.so not built");
}

fn nm(path: &PathBuf, extra: &str) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg(extra)
        .arg(path)
        .output()
        .expect("nm not available");
    assert!(
        out.status.success(),
        "nm {extra} {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

fn is_libc_or_runtime(sym: &str) -> bool {
    // versioned glibc / unwinder / compiler-runtime imports
    let base = sym.split('@').next().unwrap_or(sym);
    sym.contains("@GLIBC")
        || sym.contains("@GCC")
        || base.starts_with("_ITM_")
        || base.starts_with("_Unwind_")
        || base.starts_with("__gmon_start__")
        || base.starts_with("__cxa_")
        || base.starts_with("__tls_get_addr")
        || base.starts_with("__errno_location")
        || matches!(
            base,
            "malloc" | "realloc" | "free" | "calloc" | "posix_memalign"
                | "memcpy" | "memmove" | "memset" | "bcmp" | "memcmp"
                | "strlen" | "strcpy" | "strcmp"
                | "sprintf" | "snprintf" | "printf" | "puts" | "fwrite" | "fflush"
                | "abort" | "getenv" | "getcwd" | "readlink" | "realpath"
                | "open" | "open64" | "close" | "read" | "write" | "writev"
                | "lseek" | "lseek64" | "stat" | "stat64" | "fstat" | "fstat64" | "statx"
                | "mmap" | "mmap64" | "munmap" | "dl_iterate_phdr" | "syscall"
                | "gettid" | "pthread_key_create" | "pthread_key_delete"
                | "pthread_getspecific" | "pthread_setspecific" | "pthread_self"
                | "pthread_mutex_lock" | "pthread_mutex_unlock" | "pthread_mutex_trylock"
                | "sysconf" | "sigaltstack" | "sigaction" | "mprotect" | "getrandom"
                | "__libc_start_main"
        )
}

#[test]
fn phase_d_every_c_symbol_is_exported_by_rust() {
    let cs: BTreeSet<String> = nm(&c_so(), "--defined-only").into_iter().collect();
    let rs: BTreeSet<String> = nm(&rust_so(), "--defined-only").into_iter().collect();

    assert!(
        !cs.is_empty(),
        "no defined symbols found in the C .so — check the build"
    );

    let missing: Vec<&String> = cs.difference(&rs).collect();
    assert!(
        missing.is_empty(),
        "Rust .so ({}) is MISSING {} symbol(s) exported by the C .so ({}): {missing:?}",
        rust_so().display(),
        missing.len(),
        c_so().display()
    );

    // Sanity: the six documented entry points really are in both.
    for want in [
        "append_to_buffer",
        "buffapp",
        "create_buffer",
        "destroy_buffer",
        "get_operation_name",
        "perform_operation",
    ] {
        assert!(cs.contains(want), "C .so missing {want}");
        assert!(rs.contains(want), "Rust .so missing {want}");
    }
    assert_eq!(cs.len(), 6, "C symbol surface changed: {cs:?}");
}

#[test]
fn phase_d_rust_so_has_no_undefined_non_libc_symbols() {
    let undef = nm(&rust_so(), "--undefined-only");
    let bad: Vec<&String> = undef.iter().filter(|s| !is_libc_or_runtime(s)).collect();
    assert!(
        bad.is_empty(),
        "Rust .so has undefined non-libc symbols: {bad:?}"
    );
}

/// Guard: the tests must be running against a real `.so` on disk that is
/// newer than or equal in surface to the source, and both libraries must be
/// loadable and distinct files.
#[test]
fn phase_d_harness_loads_two_distinct_shared_objects() {
    let c_path = c_so().canonicalize().unwrap();
    let r_path = rust_so().canonicalize().unwrap();
    assert_ne!(c_path, r_path, "harness loaded the same .so twice");
    eprintln!("C   .so: {}", c_path.display());
    eprintln!("Rust.so: {}", r_path.display());
    // Both must actually dlopen and expose the entry points.
    let _ = common::c();
    let _ = common::rust();
}
