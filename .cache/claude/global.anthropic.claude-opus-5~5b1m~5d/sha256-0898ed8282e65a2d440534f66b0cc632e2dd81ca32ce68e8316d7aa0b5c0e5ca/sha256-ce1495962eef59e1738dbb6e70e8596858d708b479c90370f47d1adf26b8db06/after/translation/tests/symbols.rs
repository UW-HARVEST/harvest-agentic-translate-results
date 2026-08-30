//! Phase D — symbol parity between the C `.so` and the Rust `.so`.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(args: &[&str], so: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(args)
        .arg(so)
        .output()
        .expect("nm must be available");
    assert!(out.status.success(), "nm failed on {so:?}");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_owned))
        .collect()
}

fn defined(so: &Path) -> BTreeSet<String> {
    nm(&["-D", "--defined-only"], so).into_iter().collect()
}

/// Symbols the Rust toolchain itself may add to any `cdylib`; they are not part
/// of the translated API surface.
fn is_rust_runtime(sym: &str) -> bool {
    sym.starts_with("_ZN")
        || sym.starts_with("__rust")
        || sym.starts_with("rust_")
        || sym.starts_with("_R")
        || sym == "rust_eh_personality"
        || sym.starts_with("__rdl_")
        || sym.starts_with("_ITM_")
        || sym.starts_with("__cxa")
}

/// Every symbol the C `.so` exports must be exported by the Rust `.so` under
/// the exact same name.
fn c_exports_are_all_present_in_rust() {
    let c = defined(&common::c_lib_path());
    let r = defined(&common::rust_lib_path());

    assert!(
        c.contains("driver") && c.contains("print_foo"),
        "unexpected C symbol set: {c:?}"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C: {c:?}\nRust: {r:?}"
    );
    println!("\n    C exports {} symbols, all present in Rust: {:?}", c.len(), c);
}

/// The Rust `.so` must not export extra *API* symbols beyond the C ones
/// (Rust-runtime symbols excluded).
fn rust_exports_no_extra_api_symbols() {
    let c = defined(&common::c_lib_path());
    let r = defined(&common::rust_lib_path());
    let extra: Vec<&String> = r
        .difference(&c)
        .filter(|s| !is_rust_runtime(s))
        .collect();
    assert!(extra.is_empty(), "Rust .so exports unexpected extra symbols: {extra:?}");
}

/// No unresolved non-libc / non-runtime imports in the Rust `.so`.
fn rust_has_no_missing_undefined_symbols() {
    let undef = nm(&["-D", "-u"], &common::rust_lib_path());
    let allowed = |s: &str| {
        let base = s.split('@').next().unwrap_or(s);
        base.starts_with('_')
            || is_rust_runtime(base)
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
                    | "pthread_key_create"
                    | "pthread_key_delete"
                    | "pthread_setspecific"
                    | "read"
                    | "readlink"
                    | "realloc"
                    | "realpath"
                    | "stat"
                    | "stat64"
                    | "statx"
                    | "strlen"
                    | "syscall"
                    | "write"
                    | "writev"
            )
    };
    let bad: Vec<&String> = undef.iter().filter(|s| !allowed(s)).collect();
    assert!(bad.is_empty(), "Rust .so has unexpected undefined symbols: {bad:?}");
}

/// `driver` and `print_foo` must be loadable through `dlsym` from both objects.
fn both_symbols_resolve_via_dlsym() {
    let i = common::impls();
    for w in [common::Which::C, common::Which::Rust] {
        let _ = i.driver(w);
        let _ = i.print_foo(w);
    }
}

fn main() {
    common::run_tests(&[
        ("c_exports_are_all_present_in_rust", c_exports_are_all_present_in_rust),
        ("rust_exports_no_extra_api_symbols", rust_exports_no_extra_api_symbols),
        (
            "rust_has_no_missing_undefined_symbols",
            rust_has_no_missing_undefined_symbols,
        ),
        ("both_symbols_resolve_via_dlsym", both_symbols_resolve_via_dlsym),
    ]);
}
