// Phase D — symbol parity between the C `.so` and the Rust `.so`.
//
// Re-derives both export lists with `nm -D --defined-only` at test time so
// SYMBOLS.md cannot silently go stale, and asserts the difference
// `C_exports \ Rust_exports` is empty.
//
// This suite does no stdout capturing, so the default libtest harness is fine.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Symbols `nm -D` reports for a shared object that are toolchain/libc
/// artifacts rather than part of the library's own API surface.
fn is_toolchain_artifact(name: &str) -> bool {
    name.starts_with("_ITM_")
        || name.starts_with("__cxa_")
        || name.starts_with("_Unwind_")
        || name == "__gmon_start__"
        || name == "_init"
        || name == "_fini"
        || name == "_edata"
        || name == "_end"
        || name == "__bss_start"
}

/// `nm -D --defined-only <so>` -> the set of exported (defined, dynamic) names.
fn exported_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("could not run `nm -D --defined-only {so:?}`: {e}"));
    assert!(
        out.status.success(),
        "`nm -D --defined-only {so:?}` failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        // Format: "<addr> <type> <name>" or "        <type> <name>".
        let mut it = line.split_whitespace().rev();
        let name = match it.next() {
            Some(n) => n,
            None => continue,
        };
        let kind = it.next().unwrap_or("");
        // Keep only real definitions: text, data, bss, rodata, weak, indirect.
        if !matches!(kind, "T" | "t" | "D" | "d" | "B" | "b" | "R" | "r" | "W" | "V" | "i" | "A") {
            continue;
        }
        if is_toolchain_artifact(name) {
            continue;
        }
        set.insert(name.to_string());
    }
    set
}

fn c_exports() -> BTreeSet<String> {
    exported_symbols(&c_lib().path)
}

fn rust_exports() -> BTreeSet<String> {
    exported_symbols(&rust_lib().path)
}

/// Everything `driver.c` gives external linkage to. Derived by hand from the C
/// source as a cross-check on the `nm` parsing: if the parser silently returned
/// an empty set, the parity test below would pass vacuously.
const C_API: &[&str] = &["printLine", "bad", "good", "driver"];

#[test]
fn nm_parsing_produces_the_expected_c_api() {
    let c = c_exports();
    for want in C_API {
        assert!(
            c.contains(*want),
            "`nm` parsing lost the C symbol `{want}`; parsed set = {c:?}"
        );
    }
    assert_eq!(
        c.len(),
        C_API.len(),
        "the C .so exports symbols not accounted for in SYMBOLS.md: {:?}",
        c.iter().filter(|s| !C_API.contains(&s.as_str())).collect::<Vec<_>>()
    );
}

/// THE Phase D gate: the symbol diff must reach empty.
#[test]
fn phase_d_symbol_parity_c_minus_rust_is_empty() {
    let c = c_exports();
    let r = rust_exports();
    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so does not export {} of the C .so's {} symbols: {:?}\n\
         C    = {:?}\nRust = {:?}",
        missing.len(),
        c.len(),
        missing,
        c,
        r
    );
}

/// The Rust `.so` must not invent extra C-API exports either. Rust cdylibs emit
/// no mangled Rust symbols into `.dynsym` by default, so for this crate the two
/// sets are exactly equal.
#[test]
fn rust_so_exports_no_extra_api_symbols() {
    let c = c_exports();
    let r = rust_exports();
    let extra: Vec<&String> = r.difference(&c).collect();
    assert!(
        extra.is_empty(),
        "the Rust .so exports {} symbol(s) the C .so does not: {:?}",
        extra.len(),
        extra
    );
}

/// Every C symbol must be reachable through `dlsym` on the Rust object, not just
/// visible to `nm`. This is what an external C caller actually does.
#[test]
fn every_c_symbol_is_resolvable_by_dlsym_in_the_rust_so() {
    // `Lib::load` already resolved all four symbols in both objects via
    // `dlsym`; constructing them is the assertion.
    let c = c_lib();
    let r = rust_lib();
    assert_eq!(c.name, "C");
    assert_eq!(r.name, "Rust");

    // Resolve them a second time by raw name through a fresh handle, so a typo
    // in the harness cannot mask a missing export.
    let lib = unsafe { libloading::Library::new(&r.path).expect("dlopen Rust .so") };
    for name in C_API {
        let mut raw = name.as_bytes().to_vec();
        raw.push(0);
        let sym: Result<libloading::Symbol<*const ()>, _> = unsafe { lib.get(&raw) };
        assert!(sym.is_ok(), "dlsym(\"{name}\") failed on the Rust .so: {:?}", sym.err());
    }
}

/// No undefined symbol in the Rust `.so` may be anything other than libc /
/// libgcc-unwind: an undefined symbol from elsewhere would mean the translation
/// depends on something that is not present at load time.
#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(&rust_lib().path)
        .output()
        .expect("run nm --undefined-only");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);

    // Every libc / libgcc entry point the translation is allowed to import.
    const ALLOWED: &[&str] = &[
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "fstat64",
        "getcwd", "getenv", "gettid", "lseek", "lseek64", "malloc", "memcmp", "memcpy", "memmove",
        "memset", "mmap", "mmap64", "munmap", "open", "open64", "posix_memalign", "printf", "puts",
        "fputs", "fwrite", "putchar", "pthread_key_create", "pthread_key_delete",
        "pthread_getspecific", "pthread_setspecific", "read", "readlink", "realloc", "realpath",
        "stat", "stat64", "statx", "strlen", "syscall", "write", "writev", "__errno_location",
        "__tls_get_addr", "__libc_start_main", "sysconf", "getrandom", "pthread_self",
        "pthread_mutex_lock", "pthread_mutex_unlock", "pthread_mutex_trylock", "pthread_mutex_destroy",
        "pthread_rwlock_rdlock", "pthread_rwlock_unlock", "pthread_rwlock_wrlock", "sigaction",
        "sigaltstack", "signal", "raise", "mprotect", "poll", "nanosleep", "clock_gettime",
        "sched_yield", "sched_getaffinity", "strerror_r", "fflush", "fdopen", "fclose",
    ];

    let mut offenders = Vec::new();
    for line in text.lines() {
        let mut it = line.split_whitespace().rev();
        let name = match it.next() {
            Some(n) => n,
            None => continue,
        };
        // Strip the "@GLIBC_2.x" / "@GCC_3.0" version suffix.
        let bare = name.split('@').next().unwrap_or(name);
        if is_toolchain_artifact(bare) || bare.starts_with("__") && bare.ends_with("_impl") {
            continue;
        }
        if ALLOWED.contains(&bare) {
            continue;
        }
        offenders.push(bare.to_string());
    }
    assert!(
        offenders.is_empty(),
        "Rust .so imports symbols that are neither libc nor libgcc-unwind: {offenders:?}"
    );
}
