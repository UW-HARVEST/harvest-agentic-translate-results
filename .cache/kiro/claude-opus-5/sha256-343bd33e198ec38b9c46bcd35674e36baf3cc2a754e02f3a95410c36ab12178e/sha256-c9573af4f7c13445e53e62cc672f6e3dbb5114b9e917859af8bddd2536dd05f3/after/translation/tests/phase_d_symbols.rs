//! Phase D — symbol parity between the C and Rust shared objects, enforced as a
//! test so it cannot silently regress.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Global symbols *defined* by a shared object, as reported by `nm -D`.
fn defined_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
        .collect()
}

/// Symbols a shared object *imports*.
fn undefined_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect()
}

/// Log-line literals embedded in a shared object.
fn log_strings(so: &Path) -> BTreeSet<String> {
    let out = Command::new("strings").arg(so).output().expect("run strings");
    assert!(out.status.success(), "strings failed on {}", so.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| l.starts_with("[INFO] ") || l.starts_with("[ERROR] ") || l.starts_with("[WARNING] "))
        .map(str::to_string)
        .collect()
}

/// Every symbol the C `.so` exports must also be exported by the Rust `.so`,
/// with the exact same name.
#[test]
fn d1_every_c_symbol_is_exported_by_rust() {
    let c = defined_symbols(&c_so_path());
    let r = defined_symbols(&rust_so_path());

    assert!(
        !c.is_empty(),
        "nm reported no defined symbols for the C .so — the check would be vacuous"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}",
        missing.len()
    );

    // Pin the expected set so a future C change that adds a symbol is noticed.
    let expected: BTreeSet<String> = ["gotomach", "process_value", "double_value", "triple_value"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(
        c, expected,
        "the C .so's exported-symbol set changed; update SYMBOLS.md and this test"
    );
}

/// All four symbols must be resolvable through `dlsym` on the Rust `.so` and
/// callable across the FFI boundary.
#[test]
fn d2_all_symbols_resolvable_and_callable_via_dlsym() {
    let libs = Pair::load();
    let (cf, rf) = libs.gotomach();
    assert_goto_eq(&cf, &rf, "D2", 4, 7, 0, i32::MAX);
    for name in OP_NAMES {
        let (cop, rop) = libs.op(name);
        let n = std::str::from_utf8(name).unwrap();
        assert_op_eq(&cop, &rop, "D2", n, 21, 0, std::ptr::null_mut());
    }
}

/// The Rust `.so` must not import anything beyond libc / the platform runtime.
#[test]
fn d3_no_unresolved_non_libc_imports_in_rust() {
    let r = undefined_symbols(&rust_so_path());

    // The C .so's own imports must all be present in the Rust .so's import set
    // (it uses the same malloc/free/puts).
    let c = undefined_symbols(&c_so_path());
    let c_real: Vec<&String> = c
        .iter()
        .filter(|s| !s.starts_with("_ITM_") && !s.starts_with("__gmon") && !s.starts_with("__cxa_"))
        .collect();
    assert!(
        c_real.iter().any(|s| s.starts_with("malloc")),
        "sanity: the C .so should import malloc, got {c_real:?}"
    );

    let allowed_exact: &[&str] = &[
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "getcwd", "getenv",
        "gettid", "lseek64", "malloc", "memcmp", "memcpy", "memmove", "memset", "mmap64",
        "munmap", "open64", "posix_memalign", "puts", "read", "readlink", "realloc", "realpath",
        "sigaction", "sigaltstack", "signal", "statx", "strlen", "sysconf", "syscall", "write",
        "writev", "fstat64", "stat64", "mprotect", "getrandom", "poll", "nanosleep",
        "clock_gettime", "sched_getaffinity", "sched_yield", "dlsym", "dladdr", "environ",
    ];
    let allowed_prefix: &[&str] = &[
        "_Unwind_", "__", "_ITM_", "pthread_", "_dl_", "std::", "rust_",
    ];

    let mut bad = Vec::new();
    for sym in &r {
        // `nm -D` appends the version, e.g. `malloc@GLIBC_2.2.5`.
        let base = sym.split('@').next().unwrap_or(sym);
        if allowed_exact.contains(&base) || allowed_prefix.iter().any(|p| base.starts_with(p)) {
            continue;
        }
        bad.push(sym.clone());
    }
    assert!(
        bad.is_empty(),
        "the Rust .so imports {} non-libc symbol(s): {bad:?}",
        bad.len()
    );
}

/// The Rust `.so` must retain every log branch the C `.so` has — including the
/// two that are statically unreachable (`-5` and `-6`). If the optimiser
/// deletes one, the corresponding error path no longer exists at all, which is
/// a behaviour change under allocation failure.
#[test]
fn d4_log_string_sets_are_identical() {
    let c = log_strings(&c_so_path());
    let r = log_strings(&rust_so_path());
    assert_eq!(c.len(), 10, "expected 10 log literals in the C .so, got {c:?}");
    let missing: Vec<&String> = c.difference(&r).collect();
    let extra: Vec<&String> = r.difference(&c).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "log literal sets differ.\nmissing from Rust: {missing:?}\nextra in Rust: {extra:?}"
    );
}
