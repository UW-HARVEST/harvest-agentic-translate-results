//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Enforced as a test so that a partially-translated library cannot pass
//! verification: if the C `.so` exports a symbol the Rust `.so` does not, this
//! fails, no matter how well the present subset behaves.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// `nm -D --defined-only <so>` -> the set of exported symbol names.
fn exported(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .filter(|s| !s.is_empty())
        .collect()
}

/// `nm -D --undefined-only <so>` -> the set of imported symbol names.
fn imported(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .map(|s| s.split('@').next().unwrap_or("").to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// The 7 functions defined in `c_src/src/lib.c`, all with external linkage.
const EXPECTED: &[&str] = &[
    "check_permissions",
    "compare_operations",
    "complexmode",
    "copy_and_sum",
    "create_result_string",
    "multiply_with_log",
    "safe_add",
];

#[test]
fn d1_every_c_symbol_is_exported_by_rust() {
    let c = exported(&c_so_path());
    let r = exported(&rust_so_path());

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {:?}\n\
         C exports {} symbols, Rust exports {}.\n\
         Each missing symbol means either a missing #[no_mangle] wrapper or a C \
         module that was never translated.",
        missing.len(),
        missing,
        c.len(),
        r.len()
    );
}

#[test]
fn d2_symbol_sets_are_identical() {
    let c = exported(&c_so_path());
    let r = exported(&rust_so_path());
    let extra: Vec<&String> = r.difference(&c).collect();
    assert!(
        extra.is_empty(),
        "the Rust .so exports symbols the C .so does not: {extra:?}"
    );
    assert_eq!(c, r, "symbol sets differ");
}

#[test]
fn d3_c_exports_exactly_the_functions_in_lib_c() {
    // Guards against the C build silently losing a function (which would make
    // d1/d2 pass vacuously).
    let c = exported(&c_so_path());
    let expected: BTreeSet<String> = EXPECTED.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        c, expected,
        "the C .so does not export exactly the 7 functions of lib.c"
    );
}

#[test]
fn d4_all_symbols_resolve_via_dlsym_in_both() {
    // `libs()` resolves all 7 exports with RTLD_NOW in both objects and panics
    // if any dlsym fails, so simply constructing it is the check. Calling each
    // one proves the export wrappers are real code, not aliases of each other.
    let (c, r) = libs();
    let addrs_c = [
        c.create_result_string as usize,
        c.check_permissions as usize,
        c.safe_add as usize,
        c.multiply_with_log as usize,
        c.copy_and_sum as usize,
        c.compare_operations as usize,
        c.complexmode as usize,
    ];
    let addrs_r = [
        r.create_result_string as usize,
        r.check_permissions as usize,
        r.safe_add as usize,
        r.multiply_with_log as usize,
        r.copy_and_sum as usize,
        r.compare_operations as usize,
        r.complexmode as usize,
    ];
    assert_eq!(addrs_c.iter().collect::<BTreeSet<_>>().len(), 7);
    assert_eq!(
        addrs_r.iter().collect::<BTreeSet<_>>().len(),
        7,
        "two Rust exports resolved to the same address (aliased/stubbed?)"
    );
    // Cross-object: the two libraries must be distinct mappings.
    for a in addrs_c {
        assert!(!addrs_r.contains(&a), "C and Rust resolved to the same code");
    }
}

#[test]
fn d5_rust_imports_only_libc_and_compiler_runtime() {
    let r = imported(&rust_so_path());
    let allowed_prefixes = [
        "_Unwind_",
        "_ITM_",
        "__cxa_",
        "__gmon_",
        "__tls_get_addr",
        "__errno_location",
        "__libc_",
        "__rust",
        "pthread_",
        "dl_iterate_phdr",
    ];
    // Plain libc entry points the translation or std may call.
    let allowed_exact: BTreeSet<&str> = [
        "abort", "bcmp", "calloc", "close", "free", "fstat", "fstat64", "getcwd", "getenv",
        "gettid", "lseek", "lseek64", "malloc", "memcmp", "memcpy", "memmove", "memset", "mmap",
        "mmap64", "munmap", "open", "open64", "posix_memalign", "printf", "puts", "read",
        "readlink", "realloc", "realpath", "snprintf", "stat", "stat64", "statx", "strcmp",
        "strlen", "syscall", "write", "writev", "sysconf", "sigaltstack", "sigaction",
        "sigaddset", "sigemptyset", "mprotect", "poll", "nanosleep", "clock_gettime", "getpid",
        "exit", "_exit", "raise", "signal", "madvise", "environ",
    ]
    .into_iter()
    .collect();

    let unexpected: Vec<&String> = r
        .iter()
        .filter(|s| {
            !allowed_exact.contains(s.as_str())
                && !allowed_prefixes.iter().any(|p| s.starts_with(p))
        })
        .collect();
    assert!(
        unexpected.is_empty(),
        "the Rust .so imports non-libc / non-runtime symbols (unresolvable for a \
         plain C consumer): {unexpected:?}"
    );
}

#[test]
fn d6_c_and_rust_import_the_same_libc_primitives() {
    // The translation must forward to the SAME libc allocator/formatter as the C
    // build; otherwise heap ownership would not be interchangeable across the
    // FFI boundary and printf formatting could differ.
    let ci = imported(&c_so_path());
    let ri = imported(&rust_so_path());
    for need in ["malloc", "free", "memcpy", "snprintf", "strcmp"] {
        assert!(ci.contains(need), "C .so should import {need}");
        assert!(
            ri.contains(need),
            "Rust .so must import libc {need} (found: {:?})",
            ri
        );
    }
    // The C reaches stdout via printf and (after GCC's rewrite) puts; the Rust
    // uses printf. At least one printf-family import must be present in each.
    for (label, set) in [("C", &ci), ("Rust", &ri)] {
        assert!(
            set.contains("printf") || set.contains("puts"),
            "{label} .so has no printf-family import"
        );
    }
}

#[test]
fn d7_test_binary_loads_the_matching_profile_cdylib() {
    // Guards the harness itself: if the tests silently loaded the release
    // cdylib while running under `cargo test` (debug), debug-only codegen
    // differences (e.g. rustc's null-check instrumentation) would go unnoticed.
    let exe = std::env::current_exe().unwrap();
    let so = rust_so_path();
    let exe_profile = exe.parent().unwrap().parent().unwrap();
    assert_eq!(
        so.parent().unwrap(),
        exe_profile,
        "loaded cdylib {} does not come from this test binary's profile dir {}",
        so.display(),
        exe_profile.display()
    );
}
