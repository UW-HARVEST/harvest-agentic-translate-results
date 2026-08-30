//! Phase D — exported-symbol parity between the two shared objects.
//!
//! Enforces `SYMBOLS.md` mechanically: every symbol the C `.so` exports must
//! also be exported by the Rust `.so` under the exact same name, and the Rust
//! `.so` must not import anything outside libc / the Rust runtime.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use common::*;

/// Runs `nm` with the given flags and returns the symbol names it reports.
fn nm(flags: &[&str], so: &Path) -> Option<BTreeSet<String>> {
    let out = Command::new("nm").args(flags).arg(so).output().ok()?;
    if !out.status.success() {
        eprintln!(
            "nm {flags:?} {} failed: {}",
            so.display(),
            String::from_utf8_lossy(&out.stderr)
        );
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    Some(
        text.lines()
            .filter_map(|l| l.split_whitespace().last())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect(),
    )
}

/// Symbols that are part of the platform/toolchain rather than the library's
/// own API surface, so they are not expected to match one-for-one.
fn is_runtime_symbol(sym: &str) -> bool {
    const PREFIXES: [&str; 6] = ["_ITM_", "__cxa_", "__gmon_", "_Unwind_", "_GLOBAL_", "__tls_"];
    PREFIXES.iter().any(|p| sym.starts_with(p))
}

#[test]
fn symbol_parity() {
    silence_panic_hook();
    let mut s = Suite::new("symbol_parity");

    let c_so = c_so_path();
    let rust_so = rust_so_path();

    // -- every C export is also a Rust export -----------------------------
    s.row("D1 defined_symbol_diff_is_empty", || {
        let (Some(c), Some(r)) = (
            nm(&["-D", "--defined-only"], &c_so),
            nm(&["-D", "--defined-only"], &rust_so),
        ) else {
            eprintln!("  (nm unavailable — skipping)");
            return;
        };

        let c_api: BTreeSet<_> = c.iter().filter(|s| !is_runtime_symbol(s)).cloned().collect();
        let missing: Vec<_> = c_api.difference(&r).cloned().collect();

        eprintln!("    C exports:    {c_api:?}");
        eprintln!("    Rust exports: {:?}", r.iter().filter(|s| !is_runtime_symbol(s)).collect::<Vec<_>>());
        assert!(
            missing.is_empty(),
            "Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}",
            missing.len()
        );
        // The C library's entire public API is `slice`; assert it explicitly so
        // this test fails loudly if the C surface ever grows.
        assert!(c_api.contains("slice"), "C .so should export `slice`: {c_api:?}");
    });

    // -- the exported symbol is actually callable through both handles -----
    s.row("D2 exported_slice_is_callable", || {
        for which in [Impl::C, Impl::Rust] {
            let out = call(which, b"abc\0", None, None);
            assert_eq!(out.ret, 0, "{}: slice() not callable via dlsym", which.name());
            assert_eq!(out.stdout, b"abc\n", "{}", which.name());
        }
    });

    // -- the Rust .so imports nothing exotic ------------------------------
    s.row("D3 rust_imports_are_libc_or_runtime_only", || {
        let Some(undef) = nm(&["-D", "-u"], &rust_so) else {
            eprintln!("  (nm unavailable — skipping)");
            return;
        };
        // Everything the Rust cdylib needs must come from libc/libgcc; a
        // reference to another *library* symbol would mean an untranslated
        // dependency was linked in instead of being ported.
        let known_libc: BTreeSet<&str> = [
            "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat64", "getcwd",
            "getenv", "gettid", "lseek64", "malloc", "memcpy", "memmove", "memset", "mmap64",
            "munmap", "open64", "posix_memalign", "printf", "pthread_key_create",
            "pthread_key_delete", "pthread_setspecific", "puts", "read", "readlink", "realloc",
            "realpath", "stat64", "statx", "strlen", "syscall", "write", "writev",
            "__errno_location", "__libc_start_main", "sysconf", "pthread_getspecific",
            "pthread_mutex_lock", "pthread_mutex_unlock", "pthread_self", "memrchr", "strerror_r",
            "poll", "sigaction", "sigaltstack", "signal", "raise", "getpid", "openat64", "pread64",
            "fwrite", "fputs", "fflush", "exit", "qsort", "strcmp", "strncmp", "memchr",
        ]
        .into_iter()
        .collect();

        let unexpected: Vec<String> = undef
            .iter()
            .filter(|s| !is_runtime_symbol(s))
            .filter(|s| {
                // Strip the `@GLIBC_x.y` version suffix `nm` appends.
                let base = s.split('@').next().unwrap_or(s);
                !known_libc.contains(base)
            })
            .cloned()
            .collect();

        assert!(
            unexpected.is_empty(),
            "Rust .so imports unexpected non-libc symbols: {unexpected:?}"
        );
    });

    // -- the two libraries agree on the soname ----------------------------
    s.row("D4 soname_matches", || {
        let read_soname = |p: &Path| -> Option<String> {
            let out = Command::new("readelf").arg("-d").arg(p).output().ok()?;
            if !out.status.success() {
                return None;
            }
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .find(|l| l.contains("SONAME"))
                .and_then(|l| l.split('[').nth(1))
                .and_then(|l| l.split(']').next())
                .map(|s| s.to_string())
        };
        match (read_soname(&c_so), read_soname(&rust_so)) {
            (Some(c), Some(r)) => {
                eprintln!("    C soname={c:?} Rust soname={r:?}");
                assert_eq!(c, r, "soname mismatch: the Rust .so is not a drop-in replacement");
            }
            _ => eprintln!("  (readelf unavailable or no SONAME — skipping)"),
        }
    });

    s.finish();
}
