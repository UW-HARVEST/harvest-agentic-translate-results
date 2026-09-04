//! Phase D -- symbol parity between the C and Rust shared objects.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

fn defined_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            // Exported code/data only; ignore nothing (both objects are tiny).
            Some(format!("{kind} {name}"))
        })
        .collect()
}

fn undefined_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "-u", so.to_str().unwrap()])
        .output()
        .expect("run nm");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

/// Every symbol the C `.so` exports must be exported by the Rust `.so` under the
/// exact same name. The diff must be empty.
#[test]
fn c_exports_are_all_present_in_rust() {
    let (c, rs) = common::libs();
    let cs = defined_symbols(&c.path);
    let rss = defined_symbols(&rs.path);

    let c_names: BTreeSet<&str> = cs.iter().map(|s| s.split(' ').nth(1).unwrap()).collect();
    let r_names: BTreeSet<&str> = rss.iter().map(|s| s.split(' ').nth(1).unwrap()).collect();

    let missing: Vec<&&str> = c_names.difference(&r_names).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C:    {c_names:?}\n\
         Rust: {r_names:?}"
    );

    // Documented in SYMBOLS.md: exactly these two.
    assert_eq!(
        c_names,
        BTreeSet::from(["match", "spectral_contrast"]),
        "the C .so's export set changed; SYMBOLS.md needs updating"
    );
    assert_eq!(
        r_names,
        BTreeSet::from(["match", "spectral_contrast"]),
        "the Rust .so exports surplus symbols"
    );
}

/// Both objects must be fully linkable: no undefined symbol outside libc /
/// libgcc.
#[test]
fn no_undefined_non_libc_symbols() {
    let (c, rs) = common::libs();
    for lib in [c, rs] {
        for sym in undefined_symbols(&lib.path) {
            let base = sym.split('@').next().unwrap();
            let ok = base.starts_with("__")
                || base.starts_with("_ITM_")
                || base.starts_with("_Unwind_")
                || base.starts_with("_dl")
                || matches!(
                    base,
                    "memcpy"
                        | "memmove"
                        | "memset"
                        | "bcmp"
                        | "strlen"
                        | "malloc"
                        | "calloc"
                        | "realloc"
                        | "free"
                        | "posix_memalign"
                        | "abort"
                        | "sqrt"
                        | "getenv"
                        | "getcwd"
                        | "readlink"
                        | "realpath"
                        | "open64"
                        | "close"
                        | "read"
                        | "write"
                        | "writev"
                        | "lseek64"
                        | "fstat64"
                        | "stat64"
                        | "statx"
                        | "mmap64"
                        | "munmap"
                        | "syscall"
                        | "gettid"
                        | "dl_iterate_phdr"
                        | "pthread_key_create"
                        | "pthread_key_delete"
                        | "pthread_setspecific"
                );
            assert!(
                ok,
                "{}: unexpected undefined non-libc symbol `{sym}`",
                lib.name
            );
        }
    }
}

/// The two entry points must be reachable through `dlsym` with the exact C
/// names (this is what `common::libs()` already does, asserted explicitly here).
#[test]
fn both_symbols_are_dlsym_reachable() {
    let (c, rs) = common::libs();
    // Trivial call that touches neither pointer: length <= 0.
    let vc = unsafe { (c.spectral_contrast)(std::ptr::null_mut(), std::ptr::null_mut(), 0) };
    let vr = unsafe { (rs.spectral_contrast)(std::ptr::null_mut(), std::ptr::null_mut(), 0) };
    assert_eq!(vc.to_bits(), vr.to_bits());

    // NB: `bins` must be >= 1. `match(_, _, 0, _)` faults in the C -- the
    // `v[length - 1] = 0` store in `differentiate` becomes `v[-1]`, which for a
    // zero-sized VLA is exactly the return address pushed by `call preprocess`.
    // See ERRORS.md rows 3-8.
    let mut a = [1.0f64, 2.0, 3.0, 4.0];
    let mut b = [1.0f64, 2.0, 3.0, 4.0];
    let vc = unsafe { (c.r#match)(a.as_mut_ptr(), b.as_mut_ptr(), 4, 0.5) };
    let vr = unsafe { (rs.r#match)(a.as_mut_ptr(), b.as_mut_ptr(), 4, 0.5) };
    assert_eq!(vc, vr);
}
