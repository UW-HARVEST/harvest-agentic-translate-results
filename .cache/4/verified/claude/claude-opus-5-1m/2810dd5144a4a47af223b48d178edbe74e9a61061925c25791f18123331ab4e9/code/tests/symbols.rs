//! Phase D — symbol parity, enforced as a test so it cannot silently rot.
//!
//! Every symbol the C `.so` exports must also be exported by the Rust `.so`
//! under the exact same name, and the Rust `.so` must have no unresolved
//! non-libc dependencies.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

fn nm(args: &[&str], path: &std::path::Path) -> String {
    let out = Command::new("nm")
        .args(args)
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Dynamic, *defined* symbol names. Rust cdylibs additionally export a few
/// runtime/personality symbols that no C library has; those are extras, and
/// extras are allowed. Only C symbols missing from Rust are a failure.
fn defined_dynamic(path: &std::path::Path) -> BTreeSet<String> {
    nm(&["-D", "--defined-only"], path)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(str::to_owned))
        .collect()
}

fn paths() -> (std::path::PathBuf, std::path::PathBuf) {
    let l = common::libs(); // ensures both .so files are built
    let _ = l;
    let root = common::manifest_dir();
    let c = root.join("c_src/build/libtranslated_rust.so");
    let release = std::env::current_exe()
        .ok()
        .and_then(|p| {
            p.parent()
                .and_then(|d| d.parent())
                .and_then(|d| d.file_name())
                .map(|n| n.to_string_lossy().into_owned())
        })
        .map(|d| d == "release")
        .unwrap_or(false);
    let r = root
        .join("target")
        .join(if release { "dylib-release" } else { "dylib-debug" })
        .join(if release { "release" } else { "debug" })
        .join("libfloat2half_lib.so");
    (c, r)
}

#[test]
fn every_c_exported_symbol_is_exported_by_rust() {
    let (c_path, r_path) = paths();
    let c_syms = defined_dynamic(&c_path);
    let r_syms = defined_dynamic(&r_path);

    assert!(
        !c_syms.is_empty(),
        "no symbols found in C .so at {}",
        c_path.display()
    );

    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {:?}\n\
         C exports:    {:?}\n\
         Rust exports: {:?}",
        missing.len(),
        missing,
        c_syms,
        r_syms
    );

    // The C library's entire public surface, per c_src/include/lib.h.
    assert!(c_syms.contains("float2half"), "C must export float2half");
    assert!(r_syms.contains("float2half"), "Rust must export float2half");
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let (_, r_path) = paths();
    // Undefined symbols that the dynamic loader must satisfy.
    let undef: Vec<String> = nm(&["-D", "--undefined-only"], &r_path)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_owned))
        .filter(|s| !s.is_empty())
        .collect();

    // Everything a Rust cdylib legitimately imports from the platform.
    let allowed_prefixes = [
        "__", "_ITM_", "_Unwind_", "abort", "calloc", "free", "malloc", "realloc", "memcpy",
        "memmove", "memset", "memcmp", "bcmp", "strlen", "getenv", "write", "writev", "close",
        "open", "read", "poll", "pthread_", "sysconf", "dl_iterate_phdr", "dlsym", "gettid",
        "syscall", "sigaction", "sigaltstack", "mmap", "munmap", "mprotect", "getcwd", "readlink",
        "statx", "stat", "lstat", "fstat", "signal", "raise", "posix_memalign", "qsort", "strerror",
        "clock_gettime", "nanosleep", "sched_yield", "getrandom", "environ", "exit", "lseek",
        "lseek64", "realpath", "sigaddset", "sigemptyset", "pipe", "pipe2", "dup", "dup2", "fcntl",
        "getpid", "gettimeofday", "mremap", "madvise", "brk", "sbrk",
    ];
    // `nm -D --undefined-only` prints versioned names like `realpath@GLIBC_2.3`;
    // compare on the bare symbol name.
    let undef: Vec<String> = undef
        .into_iter()
        .map(|s| s.split('@').next().unwrap_or(&s).to_owned())
        .collect();
    let unexpected: Vec<&String> = undef
        .iter()
        .filter(|s| !allowed_prefixes.iter().any(|p| s.starts_with(p)))
        .collect();
    assert!(
        unexpected.is_empty(),
        "Rust .so has unresolved non-libc symbols: {unexpected:?}"
    );
}

/// The two `static` C tables have internal linkage, so they must NOT appear in
/// either `.so`'s dynamic symbol table. Confirms the Rust translation did not
/// accidentally widen the ABI.
#[test]
fn static_tables_are_not_exported_by_either_library() {
    let (c_path, r_path) = paths();
    for (label, p) in [("C", &c_path), ("Rust", &r_path)] {
        let syms = defined_dynamic(p);
        for internal in ["m__base", "m__shift", "M_BASE", "M_SHIFT"] {
            assert!(
                !syms.contains(internal),
                "{label} .so unexpectedly exports internal table `{internal}`"
            );
        }
    }
}
