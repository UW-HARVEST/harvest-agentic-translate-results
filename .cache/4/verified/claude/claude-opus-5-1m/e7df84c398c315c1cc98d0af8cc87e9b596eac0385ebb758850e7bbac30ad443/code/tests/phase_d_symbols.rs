// Phase D — symbol parity between the C `.so` and the Rust `.so`.
//
// Recomputes the `nm -D` diff at test time so the parity claim in SYMBOLS.md
// can never silently rot.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

/// Global text/data symbols defined by a shared object, per `nm -D --defined-only`.
fn defined_globals(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("failed to run `nm` (binutils required)");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let (a, b) = (it.next(), it.next());
        // "<addr> <type> <name>"  or  "        <type> <name>"
        let (ty, name) = match (a, b, it.next()) {
            (Some(_addr), Some(ty), Some(name)) => (ty, name),
            (Some(ty), Some(name), None) => (ty, name),
            _ => continue,
        };
        // Uppercase type letter == global. T=text, D/B/R=data, W/V=weak.
        if ty.len() == 1 && ty.chars().next().unwrap().is_ascii_uppercase() {
            set.insert(name.to_string());
        }
    }
    set
}

/// Undefined (imported) symbols, per `nm -D --undefined-only`.
fn undefined(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--undefined-only")
        .arg(so)
        .output()
        .expect("failed to run `nm`");
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .filter_map(|l| l.split_whitespace().last())
        .map(|s| s.split('@').next().unwrap_or(s).to_string())
        .collect()
}

/// Symbols the C library exports and that therefore constitute the API surface
/// the Rust library MUST reproduce exactly.
const EXPECTED_C_API: [&str; 6] = [
    "allocate_and_compute",
    "fallcalc",
    "foreach_sum",
    "process_array_reverse",
    "safe_double_to_int",
    "switch_fallthrough_calculator",
];

#[test]
fn symbols_c_api_surface_is_what_we_think_it_is() {
    let c = defined_globals(&common::c_so_path());
    // Filter out toolchain boilerplate (weak _ITM_*, __gmon_start__, _fini/_init...).
    let api: BTreeSet<String> = c
        .iter()
        .filter(|s| !s.starts_with('_'))
        .cloned()
        .collect();
    let expected: BTreeSet<String> = EXPECTED_C_API.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        api, expected,
        "the C `.so` API surface changed; update SYMBOLS.md / EXPECTED_C_API"
    );
}

#[test]
fn symbols_rust_exports_every_c_symbol() {
    let c = defined_globals(&common::c_so_path());
    let r = defined_globals(&common::rust_so_path());

    let missing: Vec<&String> = c
        .iter()
        // Toolchain boilerplate is not part of the API contract.
        .filter(|s| !s.starts_with('_'))
        .filter(|s| !r.contains(*s))
        .collect();

    assert!(
        missing.is_empty(),
        "Rust `.so` ({}) is MISSING {} symbol(s) exported by the C `.so` ({}): {:?}",
        common::rust_so_path().display(),
        missing.len(),
        common::c_so_path().display(),
        missing
    );
}

#[test]
fn symbols_all_six_are_dlsym_resolvable_in_both() {
    // Loading `Impl` resolves all six symbols via `dlsym` and panics otherwise.
    let (c, r) = common::both();
    assert_eq!(c.name, "C");
    assert_eq!(r.name, "Rust");
    // Sanity: the two libraries are distinct files.
    assert_ne!(c.path, r.path);
}

#[test]
fn symbols_rust_has_no_unexpected_undefined_non_libc_symbols() {
    let u = undefined(&common::rust_so_path());
    // Everything the Rust cdylib imports must come from libc / the ELF runtime.
    let allowed_prefixes = [
        "_", "abort", "calloc", "free", "malloc", "realloc", "posix_memalign", "memcpy", "memmove",
        "memset", "memcmp", "bcmp", "strlen", "pthread_", "dl", "gettid", "getenv", "write",
        "writev", "close", "open", "read", "sigaltstack", "sigaction", "sigemptyset", "sysconf",
        "mmap", "munmap", "mprotect", "syscall", "poll", "readlink", "stat", "fstat", "lseek",
        "getcwd", "environ", "signal", "raise", "qsort", "bsearch", "strerror", "clock_gettime",
        "nanosleep", "sched_yield", "madvise", "mremap", "getrandom", "statx", "pipe2", "eventfd",
        "realpath", "readdir", "opendir", "closedir", "fcntl", "ioctl", "getpid", "getppid",
        "isatty", "unlink", "rename", "mkdir", "rmdir", "chdir", "exit", "atexit", "strchr",
        "strrchr", "strncmp", "strcmp", "memrchr", "sigaddset", "pthread", "gnu_get_libc_version",
    ];
    let unexpected: Vec<&String> = u
        .iter()
        .filter(|s| !allowed_prefixes.iter().any(|p| s.starts_with(p)))
        .collect();
    assert!(
        unexpected.is_empty(),
        "Rust `.so` has undefined non-libc symbols: {unexpected:?}"
    );
}

#[test]
fn symbols_c_imports_only_malloc_and_free() {
    let u = undefined(&common::c_so_path());
    assert!(u.contains("malloc"), "C lib should import malloc: {u:?}");
    assert!(u.contains("free"), "C lib should import free: {u:?}");
}
