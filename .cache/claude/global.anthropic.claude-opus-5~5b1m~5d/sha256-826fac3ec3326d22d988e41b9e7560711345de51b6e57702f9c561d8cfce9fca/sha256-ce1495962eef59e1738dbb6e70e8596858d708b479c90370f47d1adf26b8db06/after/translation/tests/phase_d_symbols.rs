//! Phase D — symbol parity between the two shared objects.
//!
//! Enforced as a test so it cannot silently rot: every symbol the C `.so`
//! exports must be exported by the Rust `.so` under the exact same name, and
//! the Rust `.so` must not depend on any undefined symbol outside libc/libgcc.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(args: &[&str], so: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(args)
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", so.display()));
    assert!(
        out.status.success(),
        "nm {args:?} {} failed: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .map(|s| s.split('@').next().unwrap().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn defined_dynamic_symbols(so: &Path) -> BTreeSet<String> {
    nm(&["-D", "--defined-only"], so).into_iter().collect()
}

fn undefined_dynamic_symbols(so: &Path) -> BTreeSet<String> {
    nm(&["-D", "--undefined-only"], so).into_iter().collect()
}

/// Symbols any Rust `cdylib` legitimately imports: libc, libgcc's unwinder,
/// libpthread, and the weak ELF/ITM hooks the linker always emits.
fn is_runtime_provided(sym: &str) -> bool {
    const EXACT: &[&str] = &[
        // weak ELF / toolchain hooks
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__gmon_start__",
        "__cxa_finalize",
        "__cxa_thread_atexit_impl",
        "__errno_location",
        "__tls_get_addr",
        // libc: memory
        "malloc",
        "calloc",
        "realloc",
        "free",
        "posix_memalign",
        "memcpy",
        "memmove",
        "memset",
        "bcmp",
        "memcmp",
        "strlen",
        // libc: stdio / logging
        "puts",
        "printf",
        "fputs",
        "fwrite",
        "write",
        "writev",
        // libc: files & misc used by std's backtrace / path code
        "open",
        "open64",
        "close",
        "read",
        "lseek",
        "lseek64",
        "fstat",
        "fstat64",
        "stat",
        "stat64",
        "statx",
        "readlink",
        "realpath",
        "getcwd",
        "getenv",
        "mmap",
        "mmap64",
        "munmap",
        "abort",
        "syscall",
        "dl_iterate_phdr",
        "gettid",
        // libpthread (TLS destructors)
        "pthread_key_create",
        "pthread_key_delete",
        "pthread_setspecific",
        "pthread_getspecific",
    ];
    EXACT.contains(&sym) || sym.starts_with("_Unwind_") || sym.starts_with("__libc_")
}

#[test]
fn d1_every_c_symbol_is_exported_by_the_rust_so() {
    let c = c_so_path();
    let r = rust_so_path();
    let c_syms = defined_dynamic_symbols(&c);
    let r_syms = defined_dynamic_symbols(&r);

    eprintln!("C    ({}): {:?}", c.display(), c_syms);
    eprintln!("Rust ({}): {:?}", r.display(), r_syms);

    // The C library really does export something (guards against a broken build).
    assert!(
        !c_syms.is_empty(),
        "no dynamic symbols found in {} — is it built?",
        c.display()
    );

    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C   : {c_syms:?}\nRust: {r_syms:?}",
        missing.len()
    );
}

#[test]
fn d2_the_expected_four_symbols_are_present_on_both_sides() {
    // Derived from `SYMBOLS.md`; the `static` helpers and the three macros
    // deliberately produce no symbols on either side.
    const EXPECTED: [&str; 4] = ["gotomach", "process_value", "double_value", "triple_value"];
    let c_syms = defined_dynamic_symbols(&c_so_path());
    let r_syms = defined_dynamic_symbols(&rust_so_path());
    for s in EXPECTED {
        assert!(c_syms.contains(s), "C .so does not export `{s}`");
        assert!(r_syms.contains(s), "Rust .so does not export `{s}`");
    }
    // And nothing internal leaked out of either.
    for internal in [
        "is_valid_state",
        "check_char_flag",
        "init_processor",
        "cleanup_processor",
    ] {
        assert!(
            !c_syms.contains(internal),
            "`{internal}` is `static` in C and must not be exported"
        );
        assert!(
            !r_syms.contains(internal),
            "`{internal}` must stay private in Rust too"
        );
    }
}

#[test]
fn d3_rust_so_has_no_undefined_non_libc_symbols() {
    let r = rust_so_path();
    let undef = undefined_dynamic_symbols(&r);
    let unexpected: Vec<&String> = undef.iter().filter(|s| !is_runtime_provided(s)).collect();
    assert!(
        unexpected.is_empty(),
        "the Rust .so has {} undefined non-libc symbol(s): {unexpected:?}",
        unexpected.len()
    );
}

#[test]
fn d4_every_c_symbol_actually_resolves_via_dlsym_in_the_rust_so() {
    // `nm` parity is necessary but not sufficient — prove each export is
    // callable through `dlopen`/`dlsym`, which is what `Impl::load` does for
    // both libraries. A missing or mangled symbol panics there.
    let h = harness();
    assert_eq!(h.c.path, c_so_path());
    assert_eq!(h.r.path, rust_so_path());
}

#[test]
fn d5_phase_a_artifacts_exist() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for f in ["SYMBOLS.md", "ERRORS.md", "CONFIGS.md"] {
        let p = root.join(f);
        let meta = std::fs::metadata(&p)
            .unwrap_or_else(|e| panic!("Phase A artifact {} missing: {e}", p.display()));
        assert!(meta.len() > 512, "{} looks empty", p.display());
    }
}
