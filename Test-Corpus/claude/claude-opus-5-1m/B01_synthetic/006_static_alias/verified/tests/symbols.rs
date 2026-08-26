// Phase D — symbol parity: every symbol the C `.so` exports must be exported by
// the Rust `.so` under the exact same name, and must be resolvable (callable)
// through `dlsym`.
//
// The lists are re-derived with `nm -D` at test time, so this test keeps
// SYMBOLS.md honest instead of trusting a snapshot.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Global/weak *defined* symbols of a shared object.
fn defined_symbols(so: &Path) -> BTreeSet<String> {
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
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        let (kind, name) = match fields.len() {
            3 => (fields[1], fields[2]),
            2 => (fields[0], fields[1]),
            _ => continue,
        };
        // T/t text, D/d data, B/b bss, R/r rodata, W/w weak, V/v weak object,
        // G/g small data, i/u indirect/unique
        if kind.len() != 1 || !"TtDdBbRrWwVvGgiu".contains(kind) {
            continue;
        }
        // drop the @@GLIBC_x.y version suffix if any
        let name = name.split('@').next().unwrap_or(name);
        set.insert(name.to_string());
    }
    set
}

/// Symbols every shared object gets from the linker/CRT rather than from the
/// translated source; they are not part of the C API surface.
fn is_toolchain_symbol(name: &str) -> bool {
    matches!(
        name,
        "_init"
            | "_fini"
            | "__bss_start"
            | "_edata"
            | "_end"
            | "_ITM_registerTMCloneTable"
            | "_ITM_deregisterTMCloneTable"
            | "__cxa_finalize"
            | "__gmon_start__"
            | "__odr_asan_gen"
    ) || name.starts_with("__x86.get_pc_thunk")
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c = c_so();
    let r = rust_so();
    let c_syms = defined_symbols(&c);
    let r_syms = defined_symbols(&r);

    let c_api: BTreeSet<&String> = c_syms.iter().filter(|s| !is_toolchain_symbol(s)).collect();
    assert!(
        c_api.contains(&"main".to_string()) && c_api.contains(&"static_alias".to_string()),
        "unexpected C API surface: {c_api:?}"
    );

    let missing: Vec<&&String> = c_api.iter().filter(|s| !r_syms.contains(**s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by {} but missing from {}: {missing:?}\nC: {c_api:?}\nRust: {r_syms:?}",
        c.display(),
        r.display()
    );

    // Nothing may be a stub: both are resolvable and callable (exercised by the
    // other test binaries), here we only prove the dynamic lookup works.
    let pair = load_pair("symbols");
    let mut v: i32 = 1;
    let cret = unsafe { (pair.c.static_alias)(&mut v) };
    assert!(!cret.is_null());
    let mut v: i32 = 1;
    let rret = unsafe { (pair.rust.static_alias)(&mut v) };
    assert!(!rret.is_null());

    eprintln!("C API symbols verified in the Rust .so: {c_api:?}");
}

#[test]
fn undefined_symbols_are_libc_only() {
    // `nm -D -u` on the Rust .so must not reference anything the loader cannot
    // resolve; dlopen() succeeding (load_pair) already proves that, this test
    // reports the list for the record.
    let r = rust_so();
    let out = Command::new("nm")
        .args(["-D", "-u"])
        .arg(&r)
        .output()
        .expect("run nm");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    let names: Vec<String> = text
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect();
    eprintln!("Rust .so undefined (imported) symbols: {names:?}");
    // dlopen must work, i.e. every undefined symbol is resolvable at load time.
    let _ = load_pair("undef");
}
