//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Enforced as a test so it re-runs under every feature combination.

mod harness;

use std::path::PathBuf;
use std::process::Command;

fn nm_defined(so: &PathBuf) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("`nm` must be available on PATH");
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
            let (_addr, ty, name) = (it.next()?, it.next()?, it.next()?);
            // Only global text/data symbols form the ABI surface.
            matches!(ty, "T" | "D" | "B" | "R" | "W" | "V" | "G" | "S").then(|| name.to_string())
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

fn nm_undefined(so: &PathBuf) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", so.to_str().unwrap()])
        .output()
        .expect("`nm` must be available on PATH");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so() -> PathBuf {
    manifest_dir().parent().unwrap().join("c_src/build/libdriver.so")
}

fn rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    let base = manifest_dir().join("target");
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for profile in ["debug", "release"] {
        let cand = base.join(profile).join("libdriver.so");
        if let Ok(md) = std::fs::metadata(&cand) {
            let t = md.modified().unwrap_or(std::time::UNIX_EPOCH);
            if best.as_ref().map(|(bt, _)| t > *bt).unwrap_or(true) {
                best = Some((t, cand));
            }
        }
    }
    best.expect("Rust cdylib not built").1
}

#[test]
fn phase_d_symbol_parity_no_missing_symbols() {
    let c = nm_defined(&c_so());
    let r = nm_defined(&rust_so());
    assert!(
        c.contains(&"FIO_createFilename_fromOutDir".to_string()),
        "sanity: C .so should export FIO_createFilename_fromOutDir, got {c:?}"
    );
    assert!(
        c.contains(&"extractFilename".to_string()),
        "sanity: C .so should export extractFilename, got {c:?}"
    );
    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   = {c:?}\n Rust = {r:?}"
    );
}

#[test]
fn phase_d_no_undefined_non_libc_symbols() {
    // Everything the Rust .so imports must come from libc / libgcc-unwind /
    // the platform, i.e. nothing that looks like an untranslated C function.
    let allowed_prefixes = [
        "_ITM_", "_Unwind_", "__cxa_", "__errno_location", "__gmon_start__", "__tls_get_addr",
        "__libc_", "_rust_", "__rust_",
    ];
    let allowed_exact = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "exit", "fputs", "free", "fstat",
        "fstat64", "fwrite", "getcwd", "getenv", "gettid", "lseek", "lseek64", "malloc", "memcmp",
        "memcpy", "memmove", "memset", "mmap", "mmap64", "munmap", "open", "open64",
        "posix_memalign", "pthread_key_create", "pthread_key_delete", "pthread_getspecific",
        "pthread_setspecific", "read", "readlink", "realloc", "realpath", "stat", "stat64",
        "statx", "stderr", "stdout", "strerror", "strlen", "strrchr", "syscall", "write",
        "writev", "sigaltstack", "sysconf", "mprotect", "pipe2", "poll", "signal", "sigaction",
        "raise", "getrandom", "clock_gettime", "nanosleep", "sched_yield", "pthread_self",
        "pthread_mutex_lock", "pthread_mutex_unlock", "pthread_mutex_trylock", "pthread_rwlock_rdlock",
        "pthread_rwlock_unlock", "pthread_rwlock_wrlock", "environ", "__environ",
    ];
    let undef = nm_undefined(&rust_so());
    let suspicious: Vec<&String> = undef
        .iter()
        .map(|s| s)
        .filter(|s| {
            let bare = s.split('@').next().unwrap_or(s);
            !allowed_prefixes.iter().any(|p| bare.starts_with(p))
                && !allowed_exact.contains(&bare)
        })
        .collect();
    assert!(
        suspicious.is_empty(),
        "Rust .so has undefined symbols that are not libc/platform imports \
         (a sign of untranslated C): {suspicious:?}"
    );
}
