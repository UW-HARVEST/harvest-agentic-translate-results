//! Phase D — symbol parity, recomputed at test time (never trusting SYMBOLS.md).

mod common;

use std::collections::BTreeSet;
use std::process::Command;

fn nm_defined(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("`nm` must be available");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let a = it.next()?;
            let b = it.next()?;
            // "<addr> T <name>"  |  "         w <name>"
            let (kind, name) = match it.next() {
                Some(n) => (b, n),
                None => (a, b),
            };
            if kind == "T" || kind == "t" {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

fn nm_undefined(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(path)
        .output()
        .expect("`nm` must be available");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

/// The 11 functions defined in c_src/src/lib.c, as read from the source.
const EXPECTED: &[&str] = &[
    "add_operation",
    "arrayfunc",
    "compare_results_in_array",
    "compute_scaled_value",
    "compute_weighted_sum",
    "init_result_array",
    "modulo_operation",
    "multiply_operation",
    "process_with_foreach",
    "safe_double_to_int",
    "subtract_operation",
];

#[test]
fn c_so_exports_every_function_defined_in_lib_c() {
    let c = nm_defined(&common::c_so_path());
    for s in EXPECTED {
        assert!(c.contains(*s), "C .so is missing `{s}` — rebuild c_src");
    }
}

#[test]
fn rust_so_exports_every_c_symbol_exact_name() {
    let c = nm_defined(&common::c_so_path());
    let r = nm_defined(&common::rust_so_path());

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbol diff (C -> Rust) MUST be empty; missing from Rust .so: {missing:?}\n\
         C exports {} symbols, Rust exports {} symbols",
        c.len(),
        r.len()
    );
}

#[test]
fn rust_so_has_no_missing_non_libc_symbols() {
    let u = nm_undefined(&common::rust_so_path());
    let allowed_prefixes = [
        "_ITM_", "_Unwind_", "__cxa_", "__gmon_", "__tls_get_addr", "__errno_location", "__libc_",
        "__rust_", "_dl_",
    ];
    let libc_names: BTreeSet<&str> = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "fstat64",
        "getcwd", "getenv", "gettid", "lseek", "lseek64", "malloc", "memcmp", "memcpy", "memmove",
        "memset", "mmap", "mmap64", "munmap", "open", "open64", "posix_memalign",
        "pthread_key_create", "pthread_key_delete", "pthread_getspecific",
        "pthread_setspecific", "pthread_mutex_lock", "pthread_mutex_unlock", "pthread_self",
        "read", "readlink", "realloc", "realpath", "sigaction", "sigaltstack", "stat", "stat64",
        "statx", "strlen", "syscall", "sysconf", "write", "writev", "getrandom", "poll", "pipe2",
        "__sched_getaffinity_new", "sched_getaffinity", "dlsym", "dladdr",
    ]
    .into_iter()
    .collect();

    let bad: Vec<String> = u
        .iter()
        .map(|s| s.split('@').next().unwrap().to_string())
        .filter(|s| {
            !allowed_prefixes.iter().any(|p| s.starts_with(p)) && !libc_names.contains(s.as_str())
        })
        .collect();

    assert!(
        bad.is_empty(),
        "Rust .so has undefined non-libc symbols (untranslated code?): {bad:?}"
    );
}

/// Every symbol must be reachable through `dlsym` with the exact C name, in
/// both libraries. `common::c()` / `common::r()` panic if any lookup fails.
#[test]
fn every_symbol_is_dlsym_reachable_in_both() {
    let (c, r) = common::both();
    assert_eq!(c.name, "C");
    assert_eq!(r.name, "RUST");
    // Touch each pointer so the compiler cannot elide the loads.
    let ptrs: Vec<usize> = vec![
        c.add_operation as usize,
        c.multiply_operation as usize,
        c.subtract_operation as usize,
        c.modulo_operation as usize,
        c.safe_double_to_int as usize,
        c.compute_scaled_value as usize,
        c.compare_results_in_array as usize,
        c.init_result_array as usize,
        c.process_with_foreach as usize,
        c.compute_weighted_sum as usize,
        c.arrayfunc as usize,
        r.add_operation as usize,
        r.multiply_operation as usize,
        r.subtract_operation as usize,
        r.modulo_operation as usize,
        r.safe_double_to_int as usize,
        r.compute_scaled_value as usize,
        r.compare_results_in_array as usize,
        r.init_result_array as usize,
        r.process_with_foreach as usize,
        r.compute_weighted_sum as usize,
        r.arrayfunc as usize,
    ];
    assert_eq!(ptrs.len(), 22);
    assert!(ptrs.iter().all(|p| *p != 0));
    // The two libraries must be distinct objects (no accidental interposition).
    assert_ne!(
        c.arrayfunc as usize, r.arrayfunc as usize,
        "C and Rust `arrayfunc` resolved to the SAME address — one library is \
         interposing on the other and the differential test would be vacuous"
    );
}

/// Regression guard for the Cargo.toml profile settings: if `debug-assertions`
/// (and hence rustc's `ub_checks`) are ever re-enabled, the raw-pointer derefs
/// in the translation gain a null/alignment check that panics. A panic inside an
/// `extern "C"` fn is non-unwinding, so the process would abort (SIGABRT) where
/// the C segfaults (SIGSEGV) — see ERRORS.md rows E25..E27.
#[test]
fn rust_so_has_no_ub_check_instrumentation() {
    let bytes = std::fs::read(common::rust_so_path()).expect("read rust .so");
    for needle in [
        &b"null pointer dereference"[..],
        &b"misaligned pointer dereference"[..],
        &b"attempt to add with overflow"[..],
        &b"attempt to multiply with overflow"[..],
        &b"attempt to subtract with overflow"[..],
    ] {
        let found = bytes.windows(needle.len()).any(|w| w == needle);
        assert!(
            !found,
            "the Rust .so contains the ub-check/overflow-check panic message {:?}; \
             set `debug-assertions = false` and `overflow-checks = false` for this \
             profile in Cargo.toml — these checks have no C counterpart and turn a \
             SIGSEGV into a SIGABRT",
            String::from_utf8_lossy(needle)
        );
    }
}

/// The two libraries must be distinct files on disk, not the same artifact.
#[test]
fn c_and_rust_so_are_different_files() {
    let c = common::c_so_path();
    let r = common::rust_so_path();
    assert_ne!(c, r);
    assert!(c.exists(), "{} missing", c.display());
    assert!(r.exists(), "{} missing", r.display());
}

/// Proves the suite is testing the artifact of the profile it was built for
/// (a stale/wrong-profile `.so` would make every differential test vacuous).
#[test]
fn loaded_rust_so_belongs_to_this_profile() {
    let exe = std::env::current_exe().unwrap();
    let profile_dir = exe.parent().unwrap().parent().unwrap();
    let so = common::rust_so_path();
    assert_eq!(
        so.parent().unwrap(),
        profile_dir,
        "test binary lives in {} but loaded the .so from {}",
        profile_dir.display(),
        so.parent().unwrap().display()
    );
    eprintln!("profile artifact under test: {}", so.display());
}
