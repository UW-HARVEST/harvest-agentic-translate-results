//! Phase D — symbol parity between the two shared objects, checked with `nm -D`
//! from inside the test suite (not just by hand at the shell).
mod common;

use std::collections::BTreeMap;
use std::process::Command;

fn nm_defined(path: &std::path::Path) -> BTreeMap<String, String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("nm not available");
    assert!(out.status.success(), "nm failed on {}", path.display());
    let mut m = BTreeMap::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() == 3 {
            m.insert(f[2].to_string(), f[1].to_string());
        }
    }
    m
}

fn nm_undefined(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(path)
        .output()
        .expect("nm");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let root = common::workspace_root();
    let c = nm_defined(&root.join("c_src/build/libjansson.so"));
    let r = nm_defined(&root.join("translation/target/release/libjansson.so"));

    let missing: Vec<&String> = c.keys().filter(|k| !r.contains_key(*k)).collect();
    assert!(
        missing.is_empty(),
        "{} C symbols missing from the Rust .so: {:?}",
        missing.len(),
        missing
    );

    // types must match too (T vs B vs D)
    let mut type_mismatch = Vec::new();
    for (k, t) in &c {
        let rt = &r[k];
        if t != rt {
            type_mismatch.push(format!("{k}: C={t} Rust={rt}"));
        }
    }
    assert!(type_mismatch.is_empty(), "nm type mismatches: {type_mismatch:?}");

    assert_eq!(c.len(), 130, "unexpected C symbol count");
    assert_eq!(r.len(), c.len(), "Rust exports a different number of symbols");
}

#[test]
fn rust_so_has_no_non_libc_undefined_symbols() {
    let root = common::workspace_root();
    let und = nm_undefined(&root.join("translation/target/release/libjansson.so"));
    // Anything supplied by libc / libgcc / the dynamic loader is fine.
    let allowed_prefix = [
        "_ITM_", "_Unwind_", "__cxa_", "__errno", "__gmon", "__tls_", "__libc",
    ];
    let libc_syms: &[&str] = &[
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "fclose", "fgetc", "fopen",
        "free", "fstat", "fstat64", "fwrite", "getcwd", "getenv", "getpid", "gettid",
        "gettimeofday", "lseek64", "malloc", "memcpy", "memmove", "memset", "mmap64",
        "munmap", "open", "open64", "posix_memalign", "pthread_key_create",
        "pthread_key_delete", "pthread_setspecific", "read", "readlink", "realloc",
        "realpath", "sched_yield", "snprintf", "sprintf", "stat", "stat64", "statx",
        "stdin", "strerror", "strlen", "strtod", "strtoll", "syscall", "vsnprintf",
        "write", "writev", "memrchr", "sysconf", "pthread_self", "qsort", "fflush",
        "lseek", "poll", "pipe2", "sigaction", "sigaltstack", "signal", "raise",
        "pthread_getattr_np", "pthread_attr_getstack", "pthread_attr_destroy",
        "__libc_start_main", "environ", "getrandom",
    ];
    let bad: Vec<&String> = und
        .iter()
        .filter(|s| {
            let base = s.split('@').next().unwrap();
            !allowed_prefix.iter().any(|p| base.starts_with(p)) && !libc_syms.contains(&base)
        })
        .collect();
    assert!(bad.is_empty(), "unexpected undefined symbols in Rust .so: {bad:?}");
}

#[test]
fn both_libraries_load_and_agree_on_version() {
    let p = common::pair();
    unsafe {
        let cs = std::ffi::CStr::from_ptr((p.c.jansson_version_str)());
        let rs = std::ffi::CStr::from_ptr((p.r.jansson_version_str)());
        assert_eq!(cs, rs);
        assert_eq!(cs.to_str().unwrap(), "2.15.0");
    }
}
