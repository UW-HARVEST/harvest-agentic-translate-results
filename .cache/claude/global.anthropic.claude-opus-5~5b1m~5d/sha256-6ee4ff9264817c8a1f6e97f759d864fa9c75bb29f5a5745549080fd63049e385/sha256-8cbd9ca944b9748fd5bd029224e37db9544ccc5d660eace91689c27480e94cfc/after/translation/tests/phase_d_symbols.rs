//! Phase D — symbol parity between the C `.so` and the Rust `.so`.

mod harness;

use std::path::PathBuf;
use std::process::Command;

fn defined_symbols(so: &PathBuf) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("nm not available");
    assert!(out.status.success(), "nm failed on {}", so.display());
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
        .collect();
    v.sort();
    v.dedup();
    v
}

fn c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("c_src/build/libdriver.so")
}

fn rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().unwrap();
    let dir = exe.parent().unwrap().parent().unwrap().to_path_buf();
    let c = dir.join("libdriver.so");
    if c.exists() {
        return c;
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/libdriver.so")
}

#[test]
fn d1_every_c_symbol_is_exported_by_rust() {
    let _ = harness::pair(); // triggers the stale-artifact guard
    let cs = defined_symbols(&c_so());
    let rs = defined_symbols(&rust_so());
    assert!(!cs.is_empty(), "no symbols found in the C .so");
    let missing: Vec<&String> = cs.iter().filter(|s| !rs.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by C but MISSING from Rust: {missing:?}\nC={cs:?}\nRust={rs:?}"
    );
}

#[test]
fn d2_expected_symbol_set() {
    let cs = defined_symbols(&c_so());
    assert_eq!(
        cs,
        vec![
            "FIO_createFilename_fromOutDir".to_string(),
            "extractFilename".to_string()
        ],
        "the C symbol set changed; SYMBOLS.md must be regenerated"
    );
}

#[test]
fn d3_rust_so_has_no_unresolved_non_libc_symbols() {
    let so = rust_so();
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", so.to_str().unwrap()])
        .output()
        .expect("nm");
    let undef: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect();
    // Every undefined symbol must be satisfied by libc / libgcc / the loader,
    // i.e. it must carry a glibc/GCC version tag or be one of the well-known
    // weak loader hooks. In particular there must be NO unresolved Rust symbol
    // (`_ZN...` legacy or `_R...` v0 mangling), which is what a half-linked or
    // partially-translated cdylib would show.
    let rust_mangled: Vec<&String> = undef
        .iter()
        .filter(|s| s.starts_with("_ZN") || s.starts_with("_RN"))
        .collect();
    assert!(
        rust_mangled.is_empty(),
        "Rust .so has unresolved Rust-internal symbols: {rust_mangled:?}"
    );

    let known_weak = [
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__gmon_start__",
    ];
    let unversioned: Vec<&String> = undef
        .iter()
        .filter(|s| !s.contains('@') && !known_weak.contains(&s.as_str()))
        .collect();
    assert!(
        unversioned.is_empty(),
        "Rust .so has undefined symbols not provided by libc/libgcc: {unversioned:?}\n\
         all undefined: {undef:?}"
    );

    // And every symbol the *C* .so needs (minus strrchr, which the Rust
    // reimplements inline) must also be imported by the Rust .so, proving the
    // Rust really goes through libc `calloc`/`exit`/`fprintf`/`strerror`.
    let c_out = Command::new("nm")
        .args(["-D", "--undefined-only", c_so().to_str().unwrap()])
        .output()
        .expect("nm");
    let c_undef: Vec<String> = String::from_utf8_lossy(&c_out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect();
    for want in &c_undef {
        if want.starts_with("strrchr") {
            continue; // reimplemented in Rust, not imported
        }
        assert!(
            undef.contains(want),
            "Rust .so does not import {want}, which the C .so needs \
             (ABI/allocator mismatch risk).\nRust undefined: {undef:?}"
        );
    }
}
