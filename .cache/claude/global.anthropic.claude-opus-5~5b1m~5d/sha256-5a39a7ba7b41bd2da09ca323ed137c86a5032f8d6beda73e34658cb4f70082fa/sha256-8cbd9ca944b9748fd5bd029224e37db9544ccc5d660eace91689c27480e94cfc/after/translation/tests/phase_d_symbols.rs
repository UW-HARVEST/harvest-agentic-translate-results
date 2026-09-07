//! Phase D — symbol parity enforced as a test, so it cannot silently regress.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

fn nm_defined(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("failed to run `nm` — binutils required");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .map(|s| s.to_string())
        .collect()
}

fn nm_undefined(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "-u"])
        .arg(path)
        .output()
        .expect("failed to run `nm`");
    assert!(out.status.success(), "nm -u failed on {}", path.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .map(|s| s.split('@').next().unwrap_or(s).to_string())
        .collect()
}

/// Every symbol the C `.so` exports must also be exported by the Rust `.so`,
/// with the exact same name. The diff must be empty.
#[test]
fn symbol_parity_c_subset_of_rust() {
    let p = common::pair();
    let c = nm_defined(&p.c_path);
    let rust = nm_defined(&p.rust_path);

    assert!(
        c.contains("float2half"),
        "C .so must export float2half; exports were {c:?}"
    );

    let missing: Vec<&String> = c.difference(&rust).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C exports:    {c:?}\n\
         Rust exports: {rust:?}"
    );

    // The C library exports exactly one symbol; assert that so a future C
    // module addition forces this test (and SYMBOLS.md) to be revisited.
    assert_eq!(
        c.len(),
        1,
        "C .so export count changed to {}; SYMBOLS.md must be regenerated: {c:?}",
        c.len()
    );
}

/// The Rust `.so` must have no unresolved non-libc symbols: everything
/// undefined has to be a C-runtime / unwinder import, not project code.
#[test]
fn no_unresolved_non_libc_symbols_in_rust() {
    let p = common::pair();
    let undef = nm_undefined(&p.rust_path);

    // Anything from libc, libm, libpthread, libgcc's unwinder, or the
    // toolchain's weak hooks is expected.
    let allowed_exact: &[&str] = &[
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__gmon_start__",
        "__cxa_finalize",
        "__cxa_thread_atexit_impl",
        "__errno_location",
        "__tls_get_addr",
        "abort",
        "bcmp",
        "calloc",
        "close",
        "dl_iterate_phdr",
        "free",
        "fstat",
        "fstat64",
        "getcwd",
        "getenv",
        "gettid",
        "lseek",
        "lseek64",
        "malloc",
        "memcmp",
        "memcpy",
        "memmove",
        "memset",
        "mmap",
        "mmap64",
        "munmap",
        "open",
        "open64",
        "posix_memalign",
        "pthread_key_create",
        "pthread_key_delete",
        "pthread_getspecific",
        "pthread_setspecific",
        "read",
        "readlink",
        "realloc",
        "realpath",
        "stat",
        "stat64",
        "statx",
        "strlen",
        "syscall",
        "write",
        "writev",
    ];

    let unexpected: Vec<&String> = undef
        .iter()
        .filter(|s| {
            !allowed_exact.contains(&s.as_str())
                && !s.starts_with("_Unwind_")
                && !s.starts_with("__libc_")
                && !s.starts_with("pthread_")
        })
        .collect();

    assert!(
        unexpected.is_empty(),
        "Rust .so has unresolved NON-libc symbols (these would be untranslated \
         project code): {unexpected:?}"
    );
}

/// The Rust `.so` must not stub anything out. There is no legitimate reason for
/// this crate's source to contain a panicking placeholder.
#[test]
fn no_stubs_in_rust_source() {
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"),
    )
    .expect("read src/lib.rs");

    for bad in ["unimplemented!", "todo!", "unreachable!("] {
        assert!(
            !src.contains(bad),
            "src/lib.rs contains a stub (`{bad}`); a symbol that lies about its \
             behaviour is worse than a missing symbol"
        );
    }
}

/// Feature-combination guard: `CONFIGS.md` and `SYMBOLS.md` both state that the
/// crate declares no features, which is why a single test run covers every
/// combination. Fail loudly if a feature is ever added without revisiting them.
#[test]
fn crate_declares_no_features() {
    let toml = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
    )
    .expect("read Cargo.toml");

    let has_features_section = toml
        .lines()
        .any(|l| l.trim() == "[features]" || l.trim().starts_with("[features."));

    assert!(
        !has_features_section,
        "Cargo.toml now declares [features]; SYMBOLS.md/CONFIGS.md claim there is \
         only the default configuration, and Phases B-C must be re-run for every \
         new combination via `cargo test --no-default-features --features <combo>`"
    );
}
