//! Phase D — exported-symbol parity between the C and the Rust shared objects.
//!
//! See SYMBOLS.md.  The C `.so` defines exactly `driver` and `main`; the Rust
//! `.so` must define the same set, with the same names, and nothing else.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(path: &Path, extra: &str) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg(extra)
        .arg(path)
        .output()
        .expect("failed to run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

fn defined(path: &Path) -> BTreeSet<String> {
    nm(path, "--defined-only").into_iter().collect()
}

fn undefined(path: &Path) -> BTreeSet<String> {
    nm(path, "--undefined-only").into_iter().collect()
}

/// Symbols that come from libc / the language runtime and are satisfied by
/// DT_NEEDED entries (libc.so.6, libgcc_s.so.1, ld-linux).
fn is_runtime_symbol(s: &str) -> bool {
    let base = s.split('@').next().unwrap_or(s);
    base.starts_with("_ITM_")
        || base.starts_with("__cxa_")
        || base.starts_with("_Unwind_")
        || base.starts_with("__gmon_start__")
        || base.starts_with("pthread_")
        || base.starts_with("__tls_get_addr")
        || base.starts_with("__errno_location")
        || base.starts_with("__isoc99_")
        || base.starts_with("__libc_")
        || matches!(
            base,
            "abort"
                | "bcmp"
                | "calloc"
                | "close"
                | "dl_iterate_phdr"
                | "free"
                | "fstat"
                | "fstat64"
                | "getcwd"
                | "getenv"
                | "gettid"
                | "lseek"
                | "lseek64"
                | "malloc"
                | "memcmp"
                | "memcpy"
                | "memmove"
                | "memset"
                | "mmap"
                | "mmap64"
                | "munmap"
                | "open"
                | "open64"
                | "posix_memalign"
                | "printf"
                | "putchar"
                | "puts"
                | "read"
                | "readlink"
                | "realloc"
                | "realpath"
                | "signal"
                | "stat"
                | "stat64"
                | "statx"
                | "strlen"
                | "syscall"
                | "sysconf"
                | "write"
                | "writev"
                | "fflush"
                | "scanf"
        )
}

#[test]
fn c_and_rust_export_identical_symbol_sets() {
    let c = defined(common::c_so_path());
    let r = defined(common::rust_so_path());

    let missing: Vec<_> = c.difference(&r).cloned().collect();
    let extra: Vec<_> = r.difference(&c).cloned().collect();

    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );
    assert!(
        extra.is_empty(),
        "symbols exported by the Rust .so that the C .so does not export: {extra:?}"
    );
    // Sanity: the set really is the two documented entry points.
    assert_eq!(
        c,
        ["driver", "main"]
            .iter()
            .map(|s| s.to_string())
            .collect::<BTreeSet<_>>(),
        "the C .so's exported set changed; update SYMBOLS.md"
    );
}

#[test]
fn c_symbols_are_static_where_the_source_says_static() {
    // `print_hex` is `static` in the C source, so it must not be exported by
    // either object.
    for p in [common::c_so_path(), common::rust_so_path()] {
        let d = defined(p);
        assert!(
            !d.contains("print_hex"),
            "{} unexpectedly exports print_hex",
            p.display()
        );
    }
}

#[test]
fn rust_so_has_no_unresolved_non_runtime_symbols() {
    let u = undefined(common::rust_so_path());
    let bad: Vec<_> = u.iter().filter(|s| !is_runtime_symbol(s)).collect();
    assert!(
        bad.is_empty(),
        "Rust .so has undefined non-libc/non-runtime symbols: {bad:?}"
    );

    // And the same check for the C .so, so the classifier stays honest.
    let uc = undefined(common::c_so_path());
    let bad_c: Vec<_> = uc.iter().filter(|s| !is_runtime_symbol(s)).collect();
    assert!(
        bad_c.is_empty(),
        "C .so has undefined non-libc symbols: {bad_c:?}"
    );
}

#[test]
fn cargo_built_shared_objects_match_too() {
    // The differential tests drive a `rustc`-built cdylib; the artifact cargo
    // itself produces must export the same set.  If neither profile has been
    // built yet, build one here (into its own target directory, so it cannot
    // contend with the cargo invocation that is running this test) — the check
    // must never silently pass by finding nothing.
    let c = defined(common::c_so_path());
    let mut checked = 0;
    for profile in ["debug", "release"] {
        let p = common::manifest_dir().join(format!("target/{profile}/libdriver.so"));
        if !p.is_file() {
            continue;
        }
        checked += 1;
        assert_eq!(
            defined(&p),
            c,
            "cargo-built target/{profile}/libdriver.so has a different exported symbol set"
        );
    }
    if checked == 0 {
        let dir = common::tmp_dir().join("cargo-lib-build");
        let mut child = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
            .arg("build")
            .arg("--offline")
            .arg("--lib")
            .arg("--target-dir")
            .arg(&dir)
            .current_dir(common::manifest_dir())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("failed to spawn cargo to build the cdylib");
        // Hand-rolled timeout so a lock held by the outer cargo cannot hang the
        // test run.
        let start = std::time::Instant::now();
        let status = loop {
            match child.try_wait().unwrap() {
                Some(s) => break Some(s),
                None if start.elapsed() > std::time::Duration::from_secs(180) => {
                    let _ = child.kill();
                    break None;
                }
                None => std::thread::sleep(std::time::Duration::from_millis(100)),
            }
        };
        let built = dir.join("debug/libdriver.so");
        assert!(
            status.map(|s| s.success()).unwrap_or(false) && built.is_file(),
            "could not produce a cargo-built cdylib to compare ({})",
            built.display()
        );
        assert_eq!(
            defined(&built),
            c,
            "cargo-built libdriver.so has a different exported symbol set"
        );
        checked += 1;
    }
    assert!(checked > 0, "no cargo-built shared object was checked");
    eprintln!("checked {checked} cargo-built shared object(s)");
}
