//! Phase D — symbol parity between the C `.so` and the Rust `.so`.

mod harness;

use harness::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn defined_dynamic_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("failed to run `nm` (binutils required)");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_owned))
        .filter(|s| !s.is_empty())
        .collect()
}

/// The core Phase D gate: the symbol diff must be EMPTY.
#[test]
fn rust_so_exports_every_c_symbol() {
    let l = libs();
    let c = defined_dynamic_symbols(&l.c_path);
    let r = defined_dynamic_symbols(&l.r_path);

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "Rust .so ({}) is MISSING these symbols exported by the C .so ({}): {:?}",
        l.r_path.display(),
        l.c_path.display(),
        missing
    );

    // Sanity: the five documented exports really are present in both.
    for s in ["driver", "good", "bad", "printHexCharLine", "printLine"] {
        assert!(c.contains(s), "C .so unexpectedly lacks `{s}`");
        assert!(r.contains(s), "Rust .so unexpectedly lacks `{s}`");
    }
    assert_eq!(c.len(), 5, "C export set changed: {c:?}");
}

/// The two `static` C helpers must NOT be exported by either library.
#[test]
fn static_helpers_are_not_exported() {
    let l = libs();
    let c = defined_dynamic_symbols(&l.c_path);
    let r = defined_dynamic_symbols(&l.r_path);
    for s in ["goodG2B", "goodB2G"] {
        assert!(!c.contains(s), "C .so unexpectedly exports static `{s}`");
        assert!(
            !r.contains(s),
            "Rust .so exports `{s}` but it is `static` in C (linkage mismatch)"
        );
    }
}

/// Every symbol the Rust `.so` leaves undefined must be a libc / libgcc runtime
/// symbol — i.e. no untranslated module left a dangling reference.
#[test]
fn rust_so_has_no_missing_non_libc_symbols() {
    let l = libs();
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", l.r_path.to_str().unwrap()])
        .output()
        .expect("failed to run `nm`");
    let text = String::from_utf8_lossy(&out.stdout);

    let suspicious: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .filter(|s| {
            // Allowed: versioned glibc/libgcc symbols and the standard weak
            // runtime hooks.
            let base = s.split('@').next().unwrap_or(s);
            let versioned = s.contains("@GLIBC") || s.contains("@GCC") || s.contains("@GLIBCXX");
            let known_weak = matches!(
                base,
                "_ITM_deregisterTMCloneTable"
                    | "_ITM_registerTMCloneTable"
                    | "__gmon_start__"
                    | "__cxa_finalize"
                    | "__cxa_thread_atexit_impl"
                    | "_edata"
                    | "_end"
                    | "__bss_start"
            );
            !(versioned || known_weak)
        })
        .collect();

    assert!(
        suspicious.is_empty(),
        "Rust .so has non-libc undefined symbols (untranslated code?): {suspicious:?}"
    );
}

/// All five exports must be resolvable through `dlsym` with the exact C name
/// and callable across the FFI boundary (this is what the whole suite relies on).
#[test]
fn all_exports_are_dlsym_resolvable_and_callable() {
    let l = libs();
    for lib in [&l.c, &l.r] {
        let _ = sym_driver(lib);
        let _ = sym_good(lib);
        let _ = sym_bad(lib);
        let _ = sym_hex(lib);
        let _ = sym_line(lib);
    }
    // And they actually run without aborting.
    diff("phase-d smoke", |lib| unsafe {
        sym_driver(lib)(0);
        sym_driver(lib)(1);
        sym_good(lib)();
        sym_bad(lib)();
        sym_hex(lib)(0x41);
        sym_line(lib)(std::ptr::null());
    });
}

/// The project builds no binary/driver executable, so the "compare stdout of
/// the two binaries" gate is not applicable. Assert that stays true, so the
/// gate is re-evaluated if a binary is ever added.
#[test]
fn project_builds_no_binary_target() {
    let cargo = std::fs::read_to_string("Cargo.toml").unwrap();
    assert!(
        !cargo.contains("[[bin]]"),
        "Cargo.toml now declares a [[bin]]; add a binary stdout-comparison test"
    );
    assert!(
        !Path::new("src/main.rs").exists(),
        "src/main.rs appeared; add a binary stdout-comparison test"
    );
    let cmake = std::fs::read_to_string("../c_src/CMakeLists.txt").unwrap();
    assert!(
        !cmake.contains("add_executable"),
        "CMakeLists.txt now declares an executable; add a binary comparison test"
    );
}
