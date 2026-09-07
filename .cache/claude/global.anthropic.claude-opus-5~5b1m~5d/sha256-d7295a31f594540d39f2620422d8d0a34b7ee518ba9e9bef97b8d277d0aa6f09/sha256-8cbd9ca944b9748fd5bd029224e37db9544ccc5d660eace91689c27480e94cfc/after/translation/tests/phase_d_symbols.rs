// Phase D — symbol parity and feature-combination gate.
//
// Runs `nm -D` on both shared objects and requires the set of DEFINED symbols to
// be identical, then re-verifies that every exported symbol is actually callable
// through `dlopen`/`dlsym` (a stub-free existence check).

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Weak, toolchain-synthesised symbols that are not part of the library surface.
const TOOLCHAIN_WEAK: &[&str] = &[
    "_ITM_deregisterTMCloneTable",
    "_ITM_registerTMCloneTable",
    "__cxa_finalize",
    "__cxa_thread_atexit_impl",
    "__gmon_start__",
    "gettid",
    "statx",
];

fn nm(path: &Path, extra: &[&str]) -> String {
    let mut cmd = Command::new("nm");
    cmd.arg("-D");
    cmd.args(extra);
    cmd.arg(path);
    let out = cmd.output().expect("run nm (binutils must be installed)");
    assert!(
        out.status.success(),
        "nm failed on {path:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Symbol names from an `nm` listing, stripped of `@GLIBC_x.y` version suffixes.
fn names(listing: &str) -> BTreeSet<String> {
    listing
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .map(|s| s.split('@').next().unwrap_or(s).to_string())
        .filter(|s| !TOOLCHAIN_WEAK.contains(&s.as_str()))
        .collect()
}

#[test]
fn defined_symbol_sets_are_identical() {
    let c = names(&nm(&c_so_path(), &["--defined-only"]));
    let r = names(&nm(&rust_so_path(), &["--defined-only"]));

    let missing: Vec<_> = c.difference(&r).cloned().collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         Per Phase A: add the #[no_mangle] wrapper, or translate the skipped C module."
    );

    // The C surface is exactly `driver`; make sure that is still true, so this
    // test cannot pass vacuously against an empty C symbol set.
    assert_eq!(
        c,
        BTreeSet::from(["driver".to_string()]),
        "the C .so's exported surface changed; regenerate SYMBOLS.md"
    );
    assert!(r.contains("driver"), "Rust .so must export `driver`");

    // Extra Rust exports are reported for visibility but only `driver` is
    // expected; a `static`-linkage C function leaking out would be a bug.
    let extra: Vec<_> = r.difference(&c).cloned().collect();
    assert!(
        extra.is_empty(),
        "Rust .so exports symbols the C .so does not: {extra:?}"
    );
}

#[test]
fn c_internal_static_symbols_are_not_exported_by_either_so() {
    // `multi_stage` and `y` are `static` in C (internal linkage) and must not
    // appear in the dynamic symbol table of either object.
    for path in [c_so_path(), rust_so_path()] {
        let defined = names(&nm(&path, &["--defined-only"]));
        for hidden in ["multi_stage", "y", "Y"] {
            assert!(
                !defined.contains(hidden),
                "{path:?} unexpectedly exports the internal symbol `{hidden}`"
            );
        }
    }
}

#[test]
fn rust_so_has_no_unresolved_non_libc_imports() {
    // Every undefined symbol in the Rust object must come from libc/libgcc/libm
    // /libpthread/libdl. If it did not, `dlopen` (already done in `libs()`)
    // would have failed — this test makes the requirement explicit and names any
    // offender.
    let undefined = names(&nm(&rust_so_path(), &["--undefined-only"]));

    let known_runtime_prefixes = ["_Unwind_", "__", "pthread_", "_ITM_"];
    let known_libc: &[&str] = &[
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "fstat64",
        "getcwd", "getenv", "gettid", "lseek", "lseek64", "malloc", "memcmp", "memcpy",
        "memmove", "memset", "mmap", "mmap64", "munmap", "open", "open64", "posix_memalign",
        "printf", "puts", "read", "readlink", "realloc", "realpath", "sigaction", "sigaltstack",
        "stat", "stat64", "statx", "strlen", "syscall", "write", "writev", "memrchr", "fflush",
        "dlsym", "dladdr", "getrandom", "sysconf", "pipe2", "poll", "abs", "exit",
    ];

    let unknown: Vec<_> = undefined
        .iter()
        .filter(|s| {
            !known_libc.contains(&s.as_str())
                && !known_runtime_prefixes.iter().any(|p| s.starts_with(p))
        })
        .cloned()
        .collect();

    assert!(
        unknown.is_empty(),
        "Rust .so imports symbols that are not libc/libgcc runtime entries: {unknown:?}"
    );

    // Cross-check with the loader: `dlopen` resolving successfully proves there
    // are 0 genuinely missing symbols.
    let _ = libs();
}

#[test]
fn ldd_reports_no_undefined_symbols_for_either_so() {
    for path in [c_so_path(), rust_so_path()] {
        let out = Command::new("ldd").arg("-r").arg(&path).output();
        let Ok(out) = out else {
            eprintln!("ldd unavailable; relying on dlopen instead");
            continue;
        };
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            !text.contains("undefined symbol"),
            "ldd -r reports undefined symbols in {path:?}:\n{text}"
        );
    }
}

#[test]
fn only_one_build_configuration_exists() {
    // The Phase D "repeat for every feature combination" requirement: assert
    // there are no cargo features, so the default build IS the full matrix. If a
    // feature is ever added, this test fails and the matrix must be extended.
    let cargo = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
    )
    .expect("read Cargo.toml");
    assert!(
        !cargo.contains("[features]"),
        "Cargo.toml now declares features — Phases B and C must be re-run for \
         every feature combination"
    );
    // No #ifdef / conditional compilation in the C source either.
    let src = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/src/driver.c"),
    )
    .expect("read driver.c");
    assert!(
        !src.contains("#ifdef") && !src.contains("#if "),
        "the C source now has conditional compilation — enumerate those configs"
    );
}

#[test]
fn exported_driver_is_callable_and_not_a_stub() {
    // Guard against the "stub that lies about behaviour" failure mode: the
    // exported symbol must produce real, input-dependent output in both objects.
    let a = assert_same(1, 2, 3);
    let b = assert_same(0, 0, 0);
    assert_ne!(a, b, "`driver` output does not depend on its input — stub?");
    assert!(!a.is_empty() && !b.is_empty(), "`driver` produced no output");
}
