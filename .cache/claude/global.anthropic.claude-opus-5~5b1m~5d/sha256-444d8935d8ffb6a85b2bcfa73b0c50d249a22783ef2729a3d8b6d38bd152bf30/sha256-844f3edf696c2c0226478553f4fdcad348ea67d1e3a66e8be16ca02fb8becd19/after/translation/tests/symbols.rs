//! Phase D — symbol parity, checked from inside the test suite.
//!
//! Every dynamic symbol the C `.so` defines must also be defined by the Rust
//! `.so` with the exact same name, and every symbol must additionally be
//! resolvable with `dlsym` (which is what an external caller actually does).

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn defined_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
        .filter(|s| !s.starts_with("__") && !s.starts_with("_ITM") && s != "_init" && s != "_fini")
        .collect()
}

fn undefined_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", so.to_str().unwrap()])
        .output()
        .expect("run nm");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

#[test]
fn d01_every_c_symbol_is_exported_by_rust() {
    let p = Pair::load();
    let c = defined_symbols(&p.c.path);
    let r = defined_symbols(&p.rs.path);
    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by C but missing from Rust: {missing:?}\nC: {c:?}\nRust: {r:?}"
    );
    // the nine documented exports must all be there
    for want in [
        "op_add",
        "op_sub",
        "op_mul",
        "G_OP",
        "G_OP_NAME",
        "helper_call",
        "helper_ptr",
        "use_generated",
        "main",
    ] {
        assert!(c.contains(want), "C .so is missing {want}");
        assert!(r.contains(want), "Rust .so is missing {want}");
    }
    // the file-static C accumulator must NOT be exported by either
    for so_syms in [&c, &r] {
        assert!(
            !so_syms.iter().any(|s| s.starts_with("accum")),
            "the `static` accum_<OP> must not be exported"
        );
    }
}

#[test]
fn d02_no_unexpected_undefined_symbols() {
    let p = Pair::load();
    // every import must be a libc/unwinder symbol, i.e. resolvable in the
    // already-loaded process image
    let allowed_prefixes = [
        "_Unwind_", "__", "_ITM", "pthread_", "std", "rust_", "core::",
    ];
    let libc_names: BTreeSet<&str> = [
        "printf", "fprintf", "puts", "putchar", "atoi", "strtol", "stderr", "stdout", "memcpy",
        "memmove", "memset", "memcmp", "bcmp", "strlen", "malloc", "calloc", "realloc", "free",
        "posix_memalign", "abort", "write", "writev", "read", "close", "open", "open64", "lseek64",
        "fstat64", "stat64", "statx", "mmap64", "munmap", "getcwd", "getenv", "readlink",
        "realpath", "syscall", "gettid", "dl_iterate_phdr", "sysconf", "strerror_r", "signal",
        "sigaction", "sigaltstack", "raise", "getpid", "environ",
    ]
    .into_iter()
    .collect();
    for so in [&p.c.path, &p.rs.path] {
        for sym in undefined_symbols(so) {
            let base = sym.split('@').next().unwrap().to_string();
            let ok = allowed_prefixes.iter().any(|pfx| base.starts_with(pfx))
                || libc_names.contains(base.as_str());
            assert!(
                ok,
                "{}: unexpected undefined (non-libc) symbol {sym}",
                so.display()
            );
        }
    }
}

#[test]
fn d03_all_symbols_resolvable_via_dlsym() {
    let p = Pair::load();
    // touching every accessor forces a dlsym of every documented export
    for d in [&p.c, &p.rs] {
        let _ = d.op_add();
        let _ = d.op_sub();
        let _ = d.op_mul();
        let _ = d.helper_call();
        let _ = d.helper_ptr();
        let _ = d.use_generated();
        let _ = d.main_fn();
        assert!(!d.g_op_slot().is_null());
        assert!(!d.g_op_name_slot().is_null());
    }
}
