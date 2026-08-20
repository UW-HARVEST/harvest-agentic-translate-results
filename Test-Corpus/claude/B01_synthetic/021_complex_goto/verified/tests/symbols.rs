//! Phase D — symbol parity between the two artifacts.
//!
//! The C target is an executable, so its exported surface is *empty*: it defines
//! zero dynamic symbols and its only worker function is `static void foo(int,
//! int)` (internal linkage).  These tests assert that mechanically — the Rust
//! artifact must export everything the C artifact exports (a set which must be
//! computed, not assumed), must not leak any additional surface, and must have no
//! unresolved non-libc symbols.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(args: &[&str], bin: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(args)
        .arg(bin)
        .output()
        .unwrap_or_else(|e| panic!("run nm on {}: {e}", bin.display()));
    assert!(
        out.status.success(),
        "nm {args:?} {} failed: {}",
        bin.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.to_string())
        .collect()
}

/// Symbol names from an `nm` listing, dropping the address and type columns.
fn names(lines: &[String]) -> BTreeSet<String> {
    lines
        .iter()
        .filter_map(|l| l.split_whitespace().last())
        .filter(|n| !n.is_empty())
        .map(|n| n.to_string())
        .collect()
}

/// Every symbol the C artifact exports dynamically must also be exported by the
/// Rust artifact, with the exact same name.  The diff must be empty.
#[test]
fn dynamic_export_diff_is_empty() {
    let c = names(&nm(&["-D", "--defined-only"], &common::c_bin()));
    let r = names(&nm(&["-D", "--defined-only"], &common::rust_bin()));

    let missing: Vec<_> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C artifact but missing from the Rust artifact: {missing:?}"
    );

    // Sanity-check the premise of `SYMBOLS.md`: this really is an executable with
    // no exported surface, so the empty diff is a fact about the C target rather
    // than an artefact of a broken `nm` invocation.
    assert!(
        c.is_empty(),
        "the C artifact unexpectedly exports dynamic symbols {c:?}; \
         SYMBOLS.md must be regenerated and each one wrapped in Rust"
    );
    assert!(
        r.is_empty(),
        "the Rust artifact exports dynamic symbols the C artifact does not: {r:?}"
    );
}

/// The Rust artifact must not have unresolved symbols beyond libc and the
/// language runtime's unwinder.
#[test]
fn rust_has_no_unresolved_non_libc_symbols() {
    let undefined = names(&nm(&["-D", "--undefined-only"], &common::rust_bin()));

    // Anything provided by glibc/ld.so or the unwinder is expected; the point of
    // the check is that no *translated* symbol is left dangling.
    let allowed_exact: BTreeSet<&str> = [
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__gmon_start__",
        "__libc_start_main",
        "__errno_location",
        "__cxa_finalize",
        "__cxa_thread_atexit_impl",
        "__tls_get_addr",
        "__xpg_strerror_r",
        "abort",
        "bcmp",
        "calloc",
        "close",
        "dl_iterate_phdr",
        "dup",
        "fcntl",
        "free",
        "fstat64",
        "getauxval",
        "getcwd",
        "getenv",
        "gettid",
        "lseek64",
        "lseek",
        "malloc",
        "memcmp",
        "memcpy",
        "memmove",
        "memset",
        "mmap64",
        "mprotect",
        "munmap",
        "open64",
        "pause",
        "poll",
        "posix_memalign",
        "puts",
        "printf",
        "read",
        "readlink",
        "realloc",
        "realpath",
        "scanf",
        "__isoc99_scanf",
        "sigaction",
        "sigaltstack",
        "signal",
        "stat64",
        "statx",
        "strlen",
        "syscall",
        "sysconf",
        "write",
        "writev",
    ]
    .into_iter()
    .collect();

    let mut unexpected = Vec::new();
    for sym in &undefined {
        // Strip the version suffix: "memcpy@GLIBC_2.14" -> "memcpy".
        let base = sym.split('@').next().unwrap_or(sym);
        let libc_like = allowed_exact.contains(base)
            || base.starts_with("_Unwind_")
            || base.starts_with("pthread_")
            || base.starts_with("__pthread_");
        if !libc_like {
            unexpected.push(sym.clone());
        }
    }
    assert!(
        unexpected.is_empty(),
        "the Rust artifact has unresolved non-libc symbols (a translated symbol \
         is missing an implementation): {unexpected:?}"
    );
}

/// The *global* function surface must match: `main` is global in both artifacts,
/// and `foo` is local (`static`) in the C artifact, so it must not be global in
/// the Rust artifact either.
#[test]
fn global_function_surface_matches() {
    for bin in [common::c_bin(), common::rust_bin()] {
        let lines = nm(&["--defined-only"], &bin);
        let has_global_main = lines
            .iter()
            .any(|l| l.split_whitespace().collect::<Vec<_>>().as_slice() == ["T", "main"] || l.ends_with(" T main"));
        assert!(
            has_global_main,
            "{} does not define a global `main`",
            bin.display()
        );

        // No global symbol named `foo`: the C declares it `static`.
        let global_foo = lines.iter().any(|l| l.ends_with(" T foo"));
        assert!(
            !global_foo,
            "{} exports `foo` globally, but the C declares it `static`",
            bin.display()
        );
    }
}

/// Neither artifact offers a `dlopen`-able symbol surface, so a consumer that
/// tries to load it and resolve the translated function must fail on both.
///
/// This is the `libloading` view of `SYMBOLS.md`: not "the symbols match" by
/// assertion, but "no symbol is reachable through the dynamic loader", checked
/// against both artifacts the same way.
#[test]
fn neither_artifact_exposes_loadable_symbols() {
    for bin in [common::c_bin(), common::rust_bin()] {
        // Safety: loading an ELF object; any constructor it runs belongs to the
        // artifact under test.  Executables are normally not loadable at all,
        // which is exactly what is being asserted.
        let lib = unsafe { libloading::Library::new(&bin) };
        match lib {
            Err(_) => { /* not loadable at all: no exported surface. */ }
            Ok(lib) => {
                for sym in [&b"foo\0"[..], b"scan_int\0", b"driver_foo\0"] {
                    let found = unsafe { lib.get::<*const ()>(sym) };
                    assert!(
                        found.is_err(),
                        "{} unexpectedly exports {:?} through the dynamic loader",
                        bin.display(),
                        String::from_utf8_lossy(sym)
                    );
                }
            }
        }
    }
}
