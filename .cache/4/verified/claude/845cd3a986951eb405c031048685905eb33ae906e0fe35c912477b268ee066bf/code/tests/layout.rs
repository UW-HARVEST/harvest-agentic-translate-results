//! Phase D: exported-symbol parity between the C and the Rust shared library,
//! plus the ABI assumptions the differential tests rely on.

mod common;

use common::*;
use std::process::Command;

fn defined_symbols(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut syms: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
        .collect();
    syms.sort();
    syms.dedup();
    syms
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let _g = lock();
    ensure_c_artifacts();
    let c = defined_symbols(&c_lib_path());
    let r = defined_symbols(&rust_lib_path());
    assert!(c.len() >= 16, "expected the C .so to export >= 16 symbols, got {:?}", c);

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {:?}",
        missing
    );

    // Informational: the Rust .so may export more (see SYMBOLS.md).
    let extra: Vec<&String> = r.iter().filter(|s| !c.contains(s)).collect();
    println!("C symbols: {}, Rust-only extras: {:?}", c.len(), extra);
}

#[test]
fn rust_so_has_no_unresolved_non_libc_imports() {
    let _g = lock();
    let out = Command::new("nm")
        .arg("-D")
        .arg("--undefined-only")
        .arg(rust_lib_path())
        .output()
        .expect("run nm");
    assert!(out.status.success());
    let bad: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(1).map(|s| s.to_string()))
        .filter(|s| {
            !s.contains("GLIBC")
                && !s.starts_with("_ITM_")
                && !s.starts_with("_Unwind_")
                && s != "__gmon_start__"
        })
        .collect();
    assert!(bad.is_empty(), "unresolved non-libc symbols: {:?}", bad);
}

#[test]
fn c_struct_layouts_match() {
    // Values printed by a C program compiled against c_src/include (see
    // SYMBOLS.md); the differential tests pass these structs by value.
    assert_eq!(std::mem::size_of::<CToken>(), 280);
    assert_eq!(std::mem::align_of::<CToken>(), 8);
    assert_eq!(std::mem::offset_of!(CToken, ttype), 0);
    assert_eq!(std::mem::offset_of!(CToken, value), 4);
    assert_eq!(std::mem::offset_of!(CToken, length), 264);
    assert_eq!(std::mem::offset_of!(CToken, line), 272);
    assert_eq!(std::mem::offset_of!(CToken, column), 276);

    assert_eq!(std::mem::size_of::<COps>(), 40);
    assert_eq!(std::mem::align_of::<COps>(), 8);
    assert_eq!(std::mem::offset_of!(COps, next_token), 0);
    assert_eq!(std::mem::offset_of!(COps, peek_token), 8);
    assert_eq!(std::mem::offset_of!(COps, reset), 16);
    assert_eq!(std::mem::offset_of!(COps, load_text), 24);
    assert_eq!(std::mem::offset_of!(COps, get_stats), 32);

    assert_eq!(std::mem::size_of::<CResult>(), 64);
    assert_eq!(std::mem::offset_of!(CResult, char_count), 56);
}

#[test]
fn both_libraries_expose_all_ops_pointers() {
    let _g = lock();
    let p = libs();
    for api in [&p.c, &p.rust] {
        let ops = (api.get_tokenizer_ops)();
        assert!(ops.next_token.is_some(), "{}: next_token", api.name);
        assert!(ops.peek_token.is_some(), "{}: peek_token", api.name);
        assert!(ops.reset.is_some(), "{}: reset", api.name);
        assert!(ops.load_text.is_some(), "{}: load_text", api.name);
        assert!(ops.get_stats.is_some(), "{}: get_stats", api.name);
    }
}
