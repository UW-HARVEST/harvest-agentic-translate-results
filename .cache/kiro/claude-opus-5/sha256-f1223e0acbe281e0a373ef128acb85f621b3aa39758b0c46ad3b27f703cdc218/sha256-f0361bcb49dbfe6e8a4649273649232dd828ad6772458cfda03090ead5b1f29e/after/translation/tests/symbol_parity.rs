// Phase D: symbol parity, re-derived at test time from `nm -D`.
// SYMBOLS.md records the result; this test makes it non-regressable.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

fn defined_dynamic_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("`nm` not available");
    assert!(
        out.status.success(),
        "nm -D --defined-only {} failed: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .map(|s| s.to_string())
        .collect()
}

fn undefined_dynamic_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("-u")
        .arg(so)
        .output()
        .expect("`nm` not available");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .map(|s| s.to_string())
        .collect()
}

/// Every symbol the C `.so` exports must be exported by the Rust `.so` under the
/// exact same name. The diff MUST be empty.
#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c = defined_dynamic_symbols(&common::c_so_path());
    let r = defined_dynamic_symbols(&common::rust_so_path());

    assert!(
        !c.is_empty(),
        "C .so exported no symbols -- did the C build succeed?"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {:?}\n\
         (C exports {} symbols, Rust exports {})",
        missing.len(),
        missing,
        c.len(),
        r.len()
    );
}

/// The seven documented entry points must all really be there (guards against a
/// vacuously-passing diff if `nm` output were ever empty/misparsed).
#[test]
fn documented_entry_points_present_in_both() {
    const EXPECTED: [&str; 7] = [
        "add_node",
        "find_node_by_id",
        "get_children_count",
        "calculate_subtree_sum",
        "process_string",
        "safe_double_to_int",
        "maxnmin",
    ];
    let c = defined_dynamic_symbols(&common::c_so_path());
    let r = defined_dynamic_symbols(&common::rust_so_path());
    for s in EXPECTED {
        assert!(c.contains(s), "C .so does not export {s}");
        assert!(r.contains(s), "Rust .so does not export {s}");
    }
    assert_eq!(
        c.len(),
        EXPECTED.len(),
        "C .so exports symbols not covered by SYMBOLS.md: {:?}",
        c.iter()
            .filter(|s| !EXPECTED.contains(&s.as_str()))
            .collect::<Vec<_>>()
    );
}

/// The Rust `.so` must not leave any non-libc symbol undefined -- that would mean
/// part of the C library was never translated and is being imported from nowhere.
#[test]
fn rust_so_has_no_undefined_non_libc_symbols() {
    let u = undefined_dynamic_symbols(&common::rust_so_path());
    let c_defined = defined_dynamic_symbols(&common::c_so_path());

    // Anything the C library itself defines must NOT appear as undefined in Rust.
    let leaked: Vec<&String> = u.iter().filter(|s| c_defined.contains(*s)).collect();
    assert!(
        leaked.is_empty(),
        "Rust .so imports symbols that belong to the C library (untranslated code): {leaked:?}"
    );

    // Everything else must be recognisable libc / unwinder / loader machinery.
    let allowed_prefix = [
        "_Unwind_", "__", "_ITM_", "pthread_", "gettid", "statx", "syscall",
    ];
    let allowed_exact = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat64", "getcwd",
        "getenv", "lseek64", "malloc", "memcpy", "memmove", "memset", "mmap64", "munmap",
        "open64", "posix_memalign", "read", "readlink", "realloc", "realpath", "stat64",
        "strlen", "write", "writev", "memrchr", "sysconf", "getauxval", "poll", "sigaltstack",
        "sigaction", "mprotect", "pipe2", "signal", "raise", "environ", "stat", "fstat",
        "lseek", "mmap", "open", "readdir64", "opendir", "closedir", "strerror_r", "dlsym",
    ];
    let unexpected: Vec<&String> = u
        .iter()
        .filter(|s| {
            // strip the `@GLIBC_2.17` / `@GCC_3.0` version suffix before matching
            let bare = s.split('@').next().unwrap_or(s.as_str());
            !allowed_prefix.iter().any(|p| bare.starts_with(p))
                && !allowed_exact.contains(&bare)
        })
        .collect();
    assert!(
        unexpected.is_empty(),
        "Rust .so has unexpected undefined symbols (possible untranslated code): {unexpected:?}"
    );
}
