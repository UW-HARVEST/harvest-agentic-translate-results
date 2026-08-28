//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Runs `nm -D` on both artefacts and asserts that **every** dynamic symbol the
//! C library defines is also defined by the Rust library under the exact same
//! name, and that the Rust library has no undefined symbol outside libc /
//! libgcc / the linker's own weak hooks.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(path: &Path, extra: &str) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg(extra)
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("failed to run `nm -D {extra} {}`: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm -D {extra} {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            // "<addr> <type> <name>" or "         <type> <name>"
            let mut it = line.split_whitespace().rev();
            let name = it.next()?;
            let kind = it.next()?;
            // Only real code/data definitions, not rustc's read-only blobs.
            let keep = matches!(kind, "T" | "t" | "D" | "d" | "B" | "b" | "W" | "w" | "U" | "V");
            if keep {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Symbols emitted by every ELF shared object regardless of source, plus the
/// Rust/GCC runtime imports. These are not part of the library's API surface.
fn is_toolchain_symbol(s: &str) -> bool {
    const WEAK_HOOKS: &[&str] = &[
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__cxa_finalize",
        "__cxa_thread_atexit_impl",
        "__gmon_start__",
        "_init",
        "_fini",
        "__bss_start",
        "_edata",
        "_end",
    ];
    WEAK_HOOKS.contains(&s)
        || s.starts_with("_Unwind_")
        || s.starts_with("__")
        || s.contains("@GLIBC")
        || s.contains("@GCC")
}

#[test]
fn d01_every_c_symbol_is_exported_by_rust() {
    let (c_path, r_path) = common::so_paths();

    let c_defined: BTreeSet<String> = nm(&c_path, "--defined-only")
        .into_iter()
        .filter(|s| !is_toolchain_symbol(s))
        .collect();
    let r_defined: BTreeSet<String> = nm(&r_path, "--defined-only")
        .into_iter()
        .filter(|s| !is_toolchain_symbol(s))
        .collect();

    assert!(
        !c_defined.is_empty(),
        "sanity: the C .so must define at least one API symbol"
    );
    assert!(
        c_defined.contains("gaussian_kernel"),
        "sanity: expected `gaussian_kernel` among {c_defined:?}"
    );

    let missing: Vec<&String> = c_defined.difference(&r_defined).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is MISSING {} symbol(s) exported by the C .so: {:?}\n\
         C: {:?}\nRust: {:?}",
        missing.len(),
        missing,
        c_defined,
        r_defined
    );
}

#[test]
fn d02_rust_has_no_unresolved_non_libc_symbols() {
    let (_, r_path) = common::so_paths();
    let undefined: Vec<String> = nm(&r_path, "--undefined-only")
        .into_iter()
        .filter(|s| !is_toolchain_symbol(s))
        .collect();
    // Everything left must be a plain libc/libm name (they appear unversioned
    // only when the platform does not use symbol versioning).
    const ALLOWED_BARE_LIBC: &[&str] = &[
        "expf", "malloc", "free", "calloc", "realloc", "memcpy", "memmove", "memset", "bcmp",
        "strlen", "abort", "getenv", "getcwd", "readlink", "realpath", "syscall", "gettid", "write",
        "writev", "read", "close", "open64", "lseek64", "fstat64", "stat64", "statx", "mmap64",
        "munmap", "dl_iterate_phdr", "posix_memalign", "pthread_key_create", "pthread_key_delete",
        "pthread_setspecific", "pthread_getspecific", "sysconf", "environ",
    ];
    let unexpected: Vec<&String> = undefined
        .iter()
        .filter(|s| !ALLOWED_BARE_LIBC.contains(&s.as_str()))
        .collect();
    assert!(
        unexpected.is_empty(),
        "Rust .so has unresolved non-libc symbols: {unexpected:?}"
    );
}

#[test]
fn d03_both_libraries_resolve_the_same_expf() {
    // The C links `m`; the Rust declares `extern "C" fn expf`. If they ever
    // resolved to different implementations the whole differential suite would
    // be comparing apples to oranges, so assert both import `expf`.
    let (c_path, r_path) = common::so_paths();
    for p in [&c_path, &r_path] {
        let undef = nm(p, "--undefined-only");
        assert!(
            undef.iter().any(|s| s == "expf" || s.starts_with("expf@")),
            "{} must import expf from libm, got {:?}",
            p.display(),
            undef
        );
    }
}
