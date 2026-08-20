//! Phase D — symbol parity between the C `.so` and the Rust `.so`.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

fn nm(args: &[&str], path: &std::path::Path) -> String {
    let out = Command::new("nm")
        .args(args)
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm: {e}"));
    assert!(
        out.status.success(),
        "nm {args:?} {path:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Parse `nm -D` output into (name, type) pairs.
fn parse(text: &str) -> Vec<(String, char)> {
    text.lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace().rev();
            let name = it.next()?;
            let ty = it.next()?;
            let ty = ty.chars().next()?;
            if ty.is_ascii_alphabetic() && ty.len_utf8() == 1 {
                Some((name.to_string(), ty))
            } else {
                None
            }
        })
        .collect()
}

fn defined(path: &std::path::Path) -> BTreeSet<String> {
    parse(&nm(&["-D", "--defined-only"], path))
        .into_iter()
        .map(|(n, _)| n)
        .collect()
}

/// Symbols that the Rust standard library legitimately imports from the C
/// runtime. Anything outside this set would mean the Rust `.so` expects a
/// symbol from the *library under test* that it does not provide itself.
fn is_platform_symbol(name: &str) -> bool {
    const KNOWN: &[&str] = &[
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__gmon_start__",
        "__cxa_finalize",
        "__cxa_thread_atexit_impl",
        "__errno_location",
        "__tls_get_addr",
        "__libc_start_main",
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
        "pthread_key_create",
        "pthread_key_delete",
        "pthread_getspecific",
        "pthread_setspecific",
        "pthread_mutex_lock",
        "pthread_mutex_unlock",
        "pthread_self",
        "read",
        "readlink",
        "realloc",
        "realpath",
        "sigaction",
        "sigaltstack",
        "stat64",
        "statx",
        "strlen",
        "syscall",
        "sysconf",
        "write",
        "writev",
    ];
    let base = name.split('@').next().unwrap_or(name);
    KNOWN.contains(&base) || base.starts_with("_Unwind_")
}

#[test]
fn c_defined_symbols_all_exported_by_rust() {
    let c = c_so_path();
    let r = rust_so_path();
    let c_syms = defined(&c);
    let r_syms = defined(&r);

    // The C library exports exactly one symbol; make sure the test is not
    // vacuous.
    assert!(
        c_syms.contains("read_side_info"),
        "C .so must export read_side_info, got {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C:    {c_syms:?}\n\
         Rust: {r_syms:?}"
    );
}

#[test]
fn static_c_symbols_are_not_exported_by_either() {
    // `get_bits` and the three scalefactor-band tables are `static` in the C, so
    // neither object may export them.
    for path in [c_so_path(), rust_so_path()] {
        let syms = defined(&path);
        for hidden in ["get_bits", "g_scf_long", "g_scf_short", "g_scf_mixed", "G_SCF"] {
            assert!(
                !syms.iter().any(|s| s == hidden),
                "{path:?} must not export the file-local symbol {hidden}"
            );
        }
    }
}

#[test]
fn rust_has_no_unexpected_undefined_symbols() {
    let r = rust_so_path();
    let text = nm(&["-D", "-u"], &r);
    let unresolved: Vec<String> = parse(&text)
        .into_iter()
        .map(|(n, _)| n)
        .filter(|n| !is_platform_symbol(n))
        .collect();
    assert!(
        unresolved.is_empty(),
        "Rust .so has non-libc undefined symbols: {unresolved:?}"
    );
}

#[test]
fn both_libraries_bind_eagerly() {
    // RTLD_NOW forces every undefined symbol to be resolved at load time, which
    // is a stronger statement than the name-based allowlist above.
    for path in [c_so_path(), rust_so_path()] {
        unsafe {
            let lib = libloading::os::unix::Library::open(
                Some(&path),
                libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_LOCAL,
            )
            .unwrap_or_else(|e| panic!("RTLD_NOW dlopen of {path:?} failed: {e}"));
            let sym: libloading::os::unix::Symbol<ReadSideInfoFn> =
                lib.get(b"read_side_info\0").expect("read_side_info");
            let _ = sym;
        }
    }
}

#[test]
fn exported_symbol_is_callable_from_both() {
    // Smoke test that both handles really point at working code.
    let mut rng = Rng::new(0xd001);
    let hdr = make_hdr(&mut rng, true, true, 0, false);
    let (case, _si) = build(&mut rng, hdr, |_r, si| set_window(si, 0));
    diff(&case, "phase_d smoke");
}
