//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Runs `nm -D` on both libraries and requires the defined-global symbol set
//! exported by C to be a subset of the Rust one, with an empty diff.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

/// Defined global/weak symbols (`nm -D --defined-only`) of a shared object.
fn defined_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let name = it.next()?;
            let kind = it.next()?;
            // T/t text, D/d data, B/b bss, W/w weak, R/r rodata.
            if matches!(kind, "T" | "D" | "B" | "R" | "W" | "V" | "G" | "S") {
                Some(name.to_string())
            } else {
                None
            }
        })
        // Symbols the toolchain injects into every ELF object; not part of the
        // library's own API surface.
        .filter(|n| {
            !matches!(
                n.as_str(),
                "_init"
                    | "_fini"
                    | "__bss_start"
                    | "_edata"
                    | "_end"
                    | "__bss_start__"
                    | "__bss_end__"
                    | "_bss_end__"
                    | "__end__"
                    | "__odr_asan_gen___rust_no_alloc_shim_is_unstable"
            )
        })
        .collect()
}

#[test]
fn phase_d_every_c_symbol_is_exported_by_rust() {
    let c = defined_symbols(&c_so_path());
    let r = defined_symbols(&rust_so_path());

    assert!(
        c.contains("driver") && c.contains("foo"),
        "sanity: C .so should export driver and foo, got {c:?}"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   = {c:?}\n\
         Rust= {r:?}"
    );
}

#[test]
fn phase_d_no_missing_non_libc_undefined_symbols_in_rust() {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", "--format=posix"])
        .arg(rust_so_path())
        .output()
        .expect("run nm");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);

    // Everything the Rust .so imports must be satisfiable by the C runtime /
    // unwinder. Anything else would be a dangling reference to untranslated
    // code.
    let allowed_prefixes = [
        "_Unwind_",
        "__",
        "_ITM_",
        "pthread_",
        "std",
        "gettid",
        "statx",
    ];
    let allowed_exact: BTreeSet<&str> = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "fstat64",
        "getcwd", "getenv", "lseek", "lseek64", "malloc", "memcmp", "memcpy", "memmove", "memset",
        "mmap", "mmap64", "munmap", "open", "open64", "posix_memalign", "printf", "read",
        "readlink", "realloc", "realpath", "stat", "stat64", "strlen", "syscall", "write",
        "writev", "sysconf", "getpagesize", "memrchr", "poll", "sigaction", "sigaltstack",
        "sigemptyset", "environ",
    ]
    .into_iter()
    .collect();

    let mut suspicious = Vec::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let Some(raw) = it.next() else { continue };
        let name = raw.split('@').next().unwrap_or(raw);
        if allowed_exact.contains(name) || allowed_prefixes.iter().any(|p| name.starts_with(p)) {
            continue;
        }
        suspicious.push(name.to_string());
    }
    assert!(
        suspicious.is_empty(),
        "Rust .so has undefined non-libc symbols (untranslated code?): {suspicious:?}"
    );
}

/// The Rust `.so` must load and both symbols must be callable through it.
#[test]
fn phase_d_symbols_are_loadable_and_callable() {
    let (c_foo, r_foo) = foo_pair();
    let (c_drv, r_drv) = driver_pair();
    let buf = CStrBuf::new(b"AxAxA");
    let p = buf.as_ptr();
    assert_eq!(unsafe { c_foo(p, b'A' as i8) }, unsafe {
        r_foo(p, b'A' as i8)
    });
    let oc = capture_stdout(|| unsafe { c_drv(p) });
    let or = capture_stdout(|| unsafe { r_drv(p) });
    assert_eq!(oc, or);
    assert_eq!(oc, b"A: 3\nx: 2\n");
}
