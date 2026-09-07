//! Phase D -- symbol parity enforced as a test.

mod common;

use std::path::PathBuf;
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn defined_symbols(so: &PathBuf) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .filter(|s| !s.is_empty())
        .collect();
    v.sort();
    v.dedup();
    v
}

fn c_so() -> PathBuf {
    let dir = root().join("c_src").join("build");
    let mut v: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("c_src/build exists (build the C lib first)")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    v.sort();
    v.pop().expect("a .so in c_src/build")
}

fn rust_so() -> PathBuf {
    for p in ["release", "debug"] {
        let c = root().join("translation").join("target").join(p).join("libnormalize_lib.so");
        if c.is_file() {
            return c;
        }
    }
    panic!("libnormalize_lib.so not built")
}

#[test]
fn d1_every_c_symbol_is_exported_by_rust() {
    let c = defined_symbols(&c_so());
    let r = defined_symbols(&rust_so());
    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C exports:    {c:?}\n\
         Rust exports: {r:?}",
        missing.len()
    );
    assert!(c.contains(&"normalize".to_string()), "C .so must export `normalize`, got {c:?}");
}

#[test]
fn d2_rust_has_no_undefined_non_libc_symbols() {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", rust_so().to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);

    // Everything std pulls in: allocator, syscalls, TLS, unwinder, CRT markers.
    let allowed_exact = [
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__cxa_finalize",
        "__cxa_thread_atexit_impl",
        "__errno_location",
        "__gmon_start__",
        "__tls_get_addr",
        "abort",
        "bcmp",
        "calloc",
        "close",
        "dl_iterate_phdr",
        "free",
        "fstat64",
        "getcwd",
        "getenv",
        "gettid",
        "lseek64",
        "malloc",
        "memcmp",
        "memcpy",
        "memmove",
        "memset",
        "mmap64",
        "munmap",
        "open64",
        "posix_memalign",
        "read",
        "readlink",
        "realloc",
        "realpath",
        "sqrtf",
        "stat64",
        "statx",
        "strlen",
        "syscall",
        "write",
        "writev",
    ];

    let mut unexpected = Vec::new();
    for line in text.lines() {
        let Some(sym) = line.split_whitespace().last() else { continue };
        let base = sym.split('@').next().unwrap_or(sym);
        let ok = allowed_exact.contains(&base)
            || base.starts_with("_Unwind_")
            || base.starts_with("pthread_")
            || base.starts_with("__libc_")
            || base.starts_with("_dl_");
        if !ok {
            unexpected.push(base.to_string());
        }
    }
    assert!(
        unexpected.is_empty(),
        "Rust .so has undefined non-libc symbols (unresolved translation deps?): {unexpected:?}"
    );
}

/// The project builds no binary/driver -- assert that so the "compare stdout"
/// gate is provably not applicable.
#[test]
fn d3_no_driver_binary_in_c_project() {
    let cml = std::fs::read_to_string(root().join("c_src").join("CMakeLists.txt")).unwrap();
    assert!(
        !cml.contains("add_executable"),
        "c_src/CMakeLists.txt now builds an executable -- stdout comparison must be added"
    );
    assert!(cml.contains("add_library"), "expected a library target");
}
