//! Phase A / Phase D — symbol-parity gate.
//!
//! Runs `nm -D` on both shared objects and asserts the Rust `.so` exports every
//! symbol the C `.so` exports, with the exact same name.

mod common;

use common::{c_so_path, rust_so_path};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// `(kind, name)` for every dynamic symbol reported by `nm -D`.
fn nm(path: &Path, extra: &[&str]) -> Vec<(char, String)> {
    let mut cmd = Command::new("nm");
    cmd.arg("-D");
    cmd.args(extra);
    cmd.arg(path);
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm {:?} {} failed: {}",
        extra,
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            // "<addr> <T> <name>" or "         <U> <name>"
            let mut it = line.split_whitespace().rev();
            let name = it.next()?.to_string();
            let kind = it.next()?.chars().next()?;
            Some((kind, name))
        })
        .collect()
}

/// Global *defined* symbols — the actual exported ABI. Weak glue symbols
/// (`_ITM_*`, `__gmon_start__`, `__cxa_*`) are toolchain artefacts, not library
/// API, and `nm --defined-only` already excludes the undefined ones.
fn exported(path: &Path) -> BTreeSet<String> {
    nm(path, &["--defined-only"])
        .into_iter()
        // Uppercase kind == global binding. T/text, D/data, B/bss, R/rodata,
        // W/V weak. Lowercase == local, not part of the dynamic ABI.
        .filter(|(k, _)| k.is_ascii_uppercase())
        .map(|(_, n)| n)
        .filter(|n| !n.starts_with("_ITM_") && n != "__gmon_start__")
        .collect()
}

fn undefined(path: &Path) -> BTreeSet<String> {
    nm(path, &["--undefined-only"])
        .into_iter()
        .map(|(_, n)| n)
        // strip the @GLIBC_x.y / @GCC_x.y version suffix
        .map(|n| n.split('@').next().unwrap_or(&n).to_string())
        .collect()
}

#[test]
fn both_shared_objects_exist() {
    let c = c_so_path();
    let r = rust_so_path();
    assert!(c.is_file(), "C .so missing: {}", c.display());
    assert!(r.is_file(), "Rust .so missing: {}", r.display());
    eprintln!("C    .so: {}", c.display());
    eprintln!("Rust .so: {}", r.display());
}

#[test]
fn c_exports_exactly_tfm() {
    // Locks in the artifact in SYMBOLS.md: the C library's whole ABI is `tfm`.
    // If a future c_src ever grows a symbol, this fails loudly instead of the
    // parity check silently comparing two equally-incomplete sets.
    let c = exported(&c_so_path());
    assert_eq!(
        c.iter().cloned().collect::<Vec<_>>(),
        vec!["tfm".to_string()],
        "C .so exported set changed; update SYMBOLS.md and translate any new module"
    );
}

#[test]
fn rust_exports_every_c_symbol() {
    let c = exported(&c_so_path());
    let r = exported(&rust_so_path());

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) that the C .so exports: {:?}\n\
         C   exports: {:?}\nRust exports: {:?}",
        missing.len(),
        missing,
        c,
        r
    );
    eprintln!("symbol diff (C \\ Rust) is EMPTY; C exports {:?}", c);
}

#[test]
fn rust_has_no_unresolved_non_libc_symbols() {
    // Every undefined symbol in the Rust .so must be a libc / libgcc-unwind
    // import, i.e. resolvable at load time. Anything else would mean a dangling
    // reference to an untranslated C function.
    let allowed_prefixes = [
        "_ITM_", "_Unwind_", "__cxa_", "__gmon_start__", "__errno_location",
        "__tls_get_addr", "__libc_", "__stack_chk", "pthread_", "gettid",
    ];
    let allowed_exact: BTreeSet<&str> = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat",
        "fstat64", "getcwd", "getenv", "lseek", "lseek64", "malloc", "memcmp",
        "memcpy", "memmove", "memset", "mmap", "mmap64", "munmap", "open",
        "open64", "posix_memalign", "read", "readlink", "realloc", "realpath",
        "stat", "stat64", "statx", "strlen", "syscall", "write", "writev",
        "sqrtf", "sqrt", "memrchr", "poll", "sysconf", "sigaltstack",
        "sigaction", "mprotect", "getpid", "raise", "signal", "dlsym",
        "environ", "__environ", "_exit", "exit", "nanosleep", "sched_yield",
    ]
    .into_iter()
    .collect();

    let r = undefined(&rust_so_path());
    let bad: Vec<&String> = r
        .iter()
        .filter(|n| {
            !allowed_exact.contains(n.as_str())
                && !allowed_prefixes.iter().any(|p| n.starts_with(p))
        })
        .collect();
    assert!(
        bad.is_empty(),
        "Rust .so has unresolved NON-libc undefined symbols (untranslated C?): {:?}",
        bad
    );
}

#[test]
fn tfm_is_loadable_from_both() {
    // Proves the dynamic symbol is actually callable via dlsym in both objects
    // (this is what every other test relies on).
    let p = common::pair();
    let src = [1.0f32, 2.0, 3.0];
    let mut dc = [0.0f32; 2];
    let mut dr = [0.0f32; 2];
    unsafe {
        (p.c.tfm)(dc.as_mut_ptr(), src.as_ptr(), 1);
        (p.rs.tfm)(dr.as_mut_ptr(), src.as_ptr(), 1);
    }
    assert_eq!(
        common::bits(&dc),
        common::bits(&dr),
        "smoke test diverged: C={:?} Rust={:?}",
        dc,
        dr
    );
}

#[test]
fn shared_objects_are_not_stale() {
    // `cargo test` builds only the TEST targets; because the tests `dlopen` the
    // cdylib rather than link it, cargo has no reason to rebuild
    // `libtfm_lib.so`. Testing a stale object silently "passes" against code
    // that no longer exists — exactly the failure mode that hid an E6/E7
    // divergence during development. Fail loudly instead.
    fn mtime(p: &Path) -> std::time::SystemTime {
        std::fs::metadata(p)
            .unwrap_or_else(|e| panic!("stat {}: {e}", p.display()))
            .modified()
            .expect("mtime")
    }

    let rs_so = rust_so_path();
    // Only `src/lib.rs` is checked. Cargo's own fingerprint correctly handles
    // manifest changes (a `[profile.dev]`-only edit legitimately leaves the
    // release artifact untouched, so comparing against Cargo.toml would be a
    // false positive); what cargo does NOT do is rebuild a dlopen-only cdylib
    // during `cargo test`, and that is exactly what this guards.
    let src = common::crate_root().join("src").join("lib.rs");
    assert!(
        mtime(&rs_so) >= mtime(&src),
        "STALE Rust .so: {} is older than {}.\n\
         Run `cargo build` (and `cargo build --release`) before `cargo test`, \
         or just use ./run_all.sh",
        rs_so.display(),
        src.display()
    );

    let c_so = c_so_path();
    let c_src = common::work_root().join("c_src").join("src").join("lib.c");
    let c_hdr = common::work_root().join("c_src").join("include").join("lib.h");
    for input in [&c_src, &c_hdr] {
        assert!(
            mtime(&c_so) >= mtime(input),
            "STALE C .so: {} is older than {}. Rebuild it with cmake.",
            c_so.display(),
            input.display()
        );
    }
}
