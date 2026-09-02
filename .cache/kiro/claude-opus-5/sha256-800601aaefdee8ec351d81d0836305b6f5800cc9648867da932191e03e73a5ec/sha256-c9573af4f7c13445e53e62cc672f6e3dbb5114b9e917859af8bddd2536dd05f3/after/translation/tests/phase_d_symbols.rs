//! Phase D — symbol parity enforced as an executing test, so the gate cannot
//! silently rot. Runs `nm -D` on both shared objects and asserts the diff is
//! empty.

mod common;

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn nm(path: &PathBuf, extra: &str) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg(extra)
        .arg(path)
        .output()
        .expect("nm must be available");
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

fn c_so() -> PathBuf {
    let dir = repo_root().join("c_src").join("build");
    let mut v: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{} not readable ({e}); build the C library first", dir.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let n = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            n.starts_with("lib") && n.ends_with(".so")
        })
        .collect();
    v.sort();
    v.pop().expect("no C .so in c_src/build")
}

fn rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_RUST_SO") {
        return PathBuf::from(p);
    }
    let t = repo_root().join("translation").join("target");
    for p in ["release", "debug"] {
        let c = t.join(p).join("librgb_to_hsv_lib.so");
        if c.exists() {
            return c;
        }
    }
    panic!("librgb_to_hsv_lib.so not built");
}

/// Every symbol the C `.so` exports must also be exported by the Rust `.so`,
/// with the exact same name. The diff must be empty.
#[test]
fn symbols_c_exports_are_all_present_in_rust() {
    let c: BTreeSet<String> = nm(&c_so(), "--defined-only").into_iter().collect();
    let r: BTreeSet<String> = nm(&rust_so(), "--defined-only").into_iter().collect();

    assert!(
        c.contains("rgb_to_hsv"),
        "sanity: the C .so must export rgb_to_hsv; got {c:?}"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         Per Phase A: add the #[no_mangle] export if the impl exists, or translate \
         the missing C source if a whole module was skipped.",
        missing.len()
    );
}

/// The Rust `.so` must not reference any undefined non-libc symbol (which would
/// mean a call into code that was never translated).
#[test]
fn symbols_rust_has_no_untranslated_undefined_references() {
    let undef = nm(&rust_so(), "--undefined-only");
    // Everything the Rust runtime legitimately imports from the platform.
    let allowed_exact: BTreeSet<&str> = [
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__gmon_start__",
        "__tls_get_addr",
        "__errno_location",
        "__cxa_finalize",
        "__cxa_thread_atexit_impl",
    ]
    .into_iter()
    .collect();
    let libc_names: BTreeSet<&str> = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "fstat64",
        "getcwd", "getenv", "gettid", "lseek", "lseek64", "malloc", "memcmp", "memcpy",
        "memmove", "memset", "mmap", "mmap64", "munmap", "open", "open64", "posix_memalign",
        "pthread_getspecific", "pthread_key_create", "pthread_key_delete",
        "pthread_setspecific", "read", "readlink", "realloc", "realpath", "stat", "stat64",
        "statx", "strlen", "syscall", "write", "writev", "sysconf", "pthread_self",
        "pthread_mutex_lock", "pthread_mutex_unlock", "pthread_mutex_trylock",
        "pthread_rwlock_rdlock", "pthread_rwlock_unlock", "pthread_rwlock_wrlock",
        "sigaltstack", "sigaction", "mprotect", "getpid", "poll", "signal", "raise",
        "__libc_start_main", "environ", "qsort",
    ]
    .into_iter()
    .collect();

    let mut bad = Vec::new();
    for sym in undef {
        // Strip any @GLIBC_x.y / @GCC_x.y version suffix.
        let base = sym.split('@').next().unwrap_or(&sym);
        let ok = allowed_exact.contains(base)
            || libc_names.contains(base)
            || base.starts_with("_Unwind_")
            || base.starts_with("__")   // compiler/libc internals
            || base.starts_with("_ZN")  // (shouldn't appear, but Rust-internal)
            || base.starts_with("_R");
        if !ok {
            bad.push(sym);
        }
    }
    assert!(
        bad.is_empty(),
        "Rust .so has undefined non-libc symbol(s), i.e. references to untranslated code: {bad:?}"
    );
}

/// The `rgb_to_hsv` export must be reachable through `dlopen`/`dlsym` on both
/// libraries (this is what every other test relies on).
#[test]
fn symbols_both_resolve_via_dlsym() {
    let c = common::c_fn();
    let r = common::rust_fn();
    let src = [0.2f32, 0.4, 0.6];
    let mut dc = [0.0f32; 3];
    let mut dr = [0.0f32; 3];
    unsafe {
        c(dc.as_mut_ptr(), src.as_ptr());
        r(dr.as_mut_ptr(), src.as_ptr());
    }
    assert_eq!(common::bits3(&dc), common::bits3(&dr));
    // Sanity: b is max => else branch => 60*(4 + (r-g)/delta) = 60*(4-0.5) = 210.
    assert_eq!(dc[0].to_bits(), 210.0f32.to_bits(), "got {}", dc[0]);
}
