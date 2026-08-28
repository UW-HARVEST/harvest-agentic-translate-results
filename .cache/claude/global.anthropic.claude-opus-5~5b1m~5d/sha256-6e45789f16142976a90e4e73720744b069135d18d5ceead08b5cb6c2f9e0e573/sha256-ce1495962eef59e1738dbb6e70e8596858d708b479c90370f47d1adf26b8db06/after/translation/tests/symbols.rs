//! Phase A / Phase D — symbol parity between the C `.so` and the Rust `.so`.

mod common;

use common::{c_so, rust_so};
use std::collections::BTreeSet;
use std::process::Command;

fn defined_dynamic_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("`nm` must be available");
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
            let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
            // Skip glibc/compiler bookkeeping that is not part of the API.
            if name.starts_with("_ITM_") || name.starts_with("__") || kind == "w" {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

fn undefined_dynamic_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", so.to_str().unwrap()])
        .output()
        .expect("`nm` must be available");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

/// The ten symbols the three C translation units export, spelled out by hand
/// from `c_src/include/*.h` + `c_src/src/driver.c` so the test cannot drift
/// silently along with a regenerated `nm` dump.
const EXPECTED: &[&str] = &[
    "add_task",
    "create_task_manager",
    "destroy_task_manager",
    "driver",
    "finalize_logger",
    "initialize_logger",
    "log_error",
    "log_info",
    "log_warning",
    "print_tasks",
];

#[test]
fn c_so_exports_exactly_the_documented_surface() {
    let c = defined_dynamic_symbols(&c_so());
    let expected: BTreeSet<String> = EXPECTED.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        c, expected,
        "the C .so's export surface changed; SYMBOLS.md must be regenerated"
    );
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c = defined_dynamic_symbols(&c_so());
    let r = defined_dynamic_symbols(&rust_so());
    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} C symbol(s): {missing:?}\n\
         C   exports: {c:?}\nRust exports: {r:?}",
        missing.len()
    );
}

/// Every symbol must be reachable through `dlsym`, not merely present in `nm`
/// (this is what an external C consumer actually does).
#[test]
fn every_symbol_is_dlsym_resolvable_in_both() {
    let pair = common::Pair::new("dlsym");
    for api in [&pair.c, &pair.rs] {
        // Constructing `Api` already resolved the nine header symbols; `driver`
        // is resolved lazily.
        let _ = api.driver();
        let _ = api.initialize_logger;
        let _ = api.add_task;
    }
}

#[test]
fn rust_so_has_no_unresolved_non_libc_imports() {
    let r = undefined_dynamic_symbols(&rust_so());
    // Everything the Rust cdylib imports must come from libc / libgcc_s, which
    // `ldd` proves are its only two dependencies.
    let out = Command::new("ldd")
        .arg(rust_so().to_str().unwrap())
        .output()
        .expect("`ldd` must be available");
    let ldd = String::from_utf8_lossy(&out.stdout);
    assert!(
        !ldd.contains("not found"),
        "Rust .so has unresolved shared-library dependencies:\n{ldd}"
    );
    // Sanity: the same libc entry points the C uses are imported by Rust too.
    for must in [
        "atoi", "fclose", "fopen", "free", "getenv", "malloc", "strchr", "strlen", "strncpy",
    ] {
        assert!(
            r.iter().any(|s| s.split('@').next() == Some(must)),
            "Rust .so does not import `{must}` — it is probably re-implementing \
             libc behaviour instead of calling it. Imports: {r:?}"
        );
    }
}
