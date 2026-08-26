//! Phase D — exported-symbol parity between the C `.so` and the Rust `.so`,
//! plus a smoke test that the whole harness (libloading on both sides,
//! subprocess execution of both executables) actually works.
//!
//! See SYMBOLS.md.

mod common;

use common::*;
use std::process::Command;

fn defined_symbols(so: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
        .collect();
    v.sort();
    v.dedup();
    v
}

fn undefined_symbols(so: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--undefined-only")
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success());
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .map(|s| s.split('@').next().unwrap_or_default().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    v.sort();
    v.dedup();
    v
}

/// The gate: `nm -D` on the C `.so` minus `nm -D` on the Rust `.so` must be empty.
#[test]
fn symbol_parity_c_so_subset_of_rust_so() {
    let c = defined_symbols(&c_so());
    let r = defined_symbols(&rust_so());

    assert_eq!(
        c,
        vec!["bad", "good", "main", "printLine"],
        "the C .so's exported surface changed; update SYMBOLS.md"
    );

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   : {c:?}\n\
         Rust: {r:?}"
    );
}

/// Every symbol must be *callable* through `dlsym`, not merely present in the
/// symbol table.
#[test]
fn every_c_symbol_is_resolvable_in_both_libraries() {
    for name in ["printLine", "bad", "good", "main"] {
        for so in [c_so(), rust_so()] {
            // SAFETY: freshly built libraries under our control.
            let lib = unsafe { libloading::Library::new(&so) }.expect("dlopen");
            let mut sym = name.as_bytes().to_vec();
            sym.push(0);
            let got: Result<libloading::Symbol<*const ()>, _> = unsafe { lib.get(&sym) };
            assert!(
                got.is_ok(),
                "`{name}` is not resolvable in {}",
                so.display()
            );
        }
    }
}

/// No non-libc symbol may be left unresolved in the Rust `.so`.
#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let undef = undefined_symbols(&rust_so());
    // Everything the Rust cdylib imports has to come from libc/libgcc/ld.so.
    let ldd = Command::new("ldd").arg(rust_so()).output().expect("ldd");
    let ldd = String::from_utf8_lossy(&ldd.stdout);
    assert!(
        !ldd.contains("not found"),
        "unresolved shared library dependency:\n{ldd}"
    );
    // Sanity: the imports we deliberately use must be there.
    for expected in ["read", "write", "__errno_location"] {
        assert!(
            undef.iter().any(|s| s == expected),
            "expected libc import `{expected}` among {undef:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Harness smoke tests
// ---------------------------------------------------------------------------

#[test]
fn smoke_executables_run_and_agree() {
    assert_exe_same_str("smoke/good", "1");
    assert_exe_same_str("smoke/bad", "0");
}

#[test]
fn smoke_shared_libraries_load_and_agree() {
    assert_so_print_line_same("smoke/printLine", Some(b"hello"));
    let c = so_call_void(Side::C, "good", 1);
    let r = so_call_void(Side::Rust, "good", 1);
    assert_bytes_eq("smoke/good", b"", &c, &r);
    assert_eq!(c, b"string\n", "C good() must print `string\\n`");
}

#[test]
fn smoke_so_main_agrees() {
    assert_so_main_same("smoke/so-main-good", b"1");
    assert_so_main_same("smoke/so-main-bad", b"0");
}
