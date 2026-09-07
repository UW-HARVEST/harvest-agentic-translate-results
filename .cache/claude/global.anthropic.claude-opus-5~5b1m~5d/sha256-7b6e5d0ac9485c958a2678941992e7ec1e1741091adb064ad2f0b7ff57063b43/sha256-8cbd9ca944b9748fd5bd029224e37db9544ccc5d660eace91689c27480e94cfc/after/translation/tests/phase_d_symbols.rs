// Phase D -- symbol parity between the C `.so` and the Rust `.so`.

mod common;
use common::*;
use std::collections::BTreeSet;
use std::process::Command;

/// Global *defined* symbols (`nm -D --defined-only`, binding `T`/`t`-free: we
/// keep only `T`/`W`/`D`/`B`/`R` and then filter to the ones a caller can use).
fn defined_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(so)
        .output()
        .expect("run nm (binutils required)");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let name = it.next()?;
            let kind = it.next()?;
            // Public code/data a C caller could bind to.
            if matches!(kind, "T" | "W" | "D" | "B" | "R") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

fn undefined_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", "--format=posix"])
        .arg(so)
        .output()
        .expect("run nm");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next().map(|s| s.to_string()))
        .collect()
}

/// Symbols the toolchain injects into any ELF shared object; they are not part
/// of the library's own API surface.
fn is_toolchain_noise(name: &str) -> bool {
    name.starts_with("_ITM_")
        || name.starts_with("__gmon")
        || name.starts_with("__cxa_finalize")
        || name.starts_with("_fini")
        || name.starts_with("_init")
        || name.starts_with("__gnu_lto")
        || name == "_edata"
        || name == "_end"
        || name == "__bss_start"
        || name.starts_with("__odr_asan")
        || name.starts_with("_Jv_")
        || name.starts_with("__register_frame")
        || name.starts_with("__deregister_frame")
        || name.starts_with("_Unwind_")
        || name.starts_with("__rust_")
        || name.starts_with("rust_")
        || name.starts_with("_R") // Rust v0 mangled internals
        || name.starts_with("_ZN") // legacy mangled internals
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c = defined_symbols(&c_so_path());
    let r = defined_symbols(&rust_so_path());

    let c_api: BTreeSet<_> = c.iter().filter(|s| !is_toolchain_noise(s)).cloned().collect();
    let missing: Vec<_> = c_api.difference(&r).cloned().collect();

    assert!(
        !c_api.is_empty(),
        "nm found no symbols in the C .so -- harness bug"
    );
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {missing:?}\n\
         C API   : {c_api:?}\n\
         Rust all: {r:?}",
        missing.len()
    );
}

#[test]
fn the_three_documented_symbols_are_present_in_both() {
    for so in [c_so_path(), rust_so_path()] {
        let syms = defined_symbols(&so);
        for want in ["fma_array", "call_fma", "driver"] {
            assert!(
                syms.contains(want),
                "{} does not export `{want}`",
                so.display()
            );
        }
    }
}

/// Authoritative check: `dlopen(RTLD_NOW)` resolves EVERY undefined symbol
/// eagerly, so it fails if the `.so` imports anything the system cannot supply.
/// No allowlist to keep in sync.
#[test]
fn both_so_files_resolve_every_symbol_eagerly() {
    use libloading::os::unix::{Library as UnixLibrary, RTLD_LOCAL, RTLD_NOW};
    for so in [c_so_path(), rust_so_path()] {
        let lib = unsafe { UnixLibrary::open(Some(&so), RTLD_NOW | RTLD_LOCAL) };
        assert!(
            lib.is_ok(),
            "dlopen(RTLD_NOW) failed for {} -- it has unresolved symbols: {:?}",
            so.display(),
            lib.err()
        );
    }
}

/// Secondary, human-readable check: every symbol the Rust `.so` imports comes
/// from glibc, the unwinder, or the ELF toolchain -- never from another Rust
/// crate's runtime that a plain C consumer would not have.
#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    // libc / libgcc / ld.so symbol name prefixes (version suffixes like
    // `@GLIBC_2.2.5` are stripped first).
    let allowed_prefixes = [
        "_ITM", "_Unwind", "__", "abort", "bcmp", "calloc", "close", "dl_", "dlopen", "dlsym",
        "environ", "exit", "fcntl", "fflush", "free", "fstat", "getcwd", "getenv", "gettid",
        "lseek", "malloc", "memcmp", "memcpy", "memmove", "memset", "mmap", "mprotect", "munmap",
        "open", "poll", "posix_", "printf", "pthread_", "read", "real", "sig", "sscanf", "stat",
        "strlen", "syscall", "sysconf", "write",
    ];
    let undef = undefined_symbols(&rust_so_path());
    let bad: Vec<_> = undef
        .iter()
        .map(|s| s.split('@').next().unwrap_or(s).to_string())
        .filter(|s| {
            !s.is_empty()
                && !allowed_prefixes.iter().any(|p| s.starts_with(p))
                && !is_toolchain_noise(s)
        })
        .collect();
    assert!(
        bad.is_empty(),
        "Rust .so imports non-libc symbols: {bad:?}\nall undefined: {undef:?}"
    );
}

/// The Rust `.so` must be loadable and callable with no host-crate linkage --
/// i.e. it behaves like a plain C shared library.
#[test]
fn rust_so_is_usable_as_a_plain_c_library() {
    let l = libs();
    let f = l.rust.call_fma();
    assert_eq!(unsafe { f([7, 8, 9].as_ptr(), 3) }, 9);
    let f = l.c.call_fma();
    assert_eq!(unsafe { f([7, 8, 9].as_ptr(), 3) }, 9);
}
