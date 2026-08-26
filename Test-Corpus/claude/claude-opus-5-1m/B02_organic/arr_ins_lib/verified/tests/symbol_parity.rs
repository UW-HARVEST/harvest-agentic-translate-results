//! Phase D — exported-symbol parity between the C `.so` and the Rust `.so`.
mod common;

use common::*;
use std::process::Command;

fn nm_defined(path: &str) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path])
        .output()
        .expect("nm must be available");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path,
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
        .collect();
    v.sort();
    v.dedup();
    v
}

fn nm_undefined(path: &str) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", path])
        .output()
        .expect("nm must be available");
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn c_and_rust_export_the_same_symbols() {
    let c = c_so_path().display().to_string();
    let r = rust_so_path().display().to_string();
    eprintln!("C   .so: {}", c);
    eprintln!("RUST.so: {}", r);

    let cs = nm_defined(&c);
    let rs = nm_defined(&r);

    let missing: Vec<&String> = cs.iter().filter(|s| !rs.contains(s)).collect();
    let extra: Vec<&String> = rs.iter().filter(|s| !cs.contains(s)).collect();

    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {:?}",
        missing
    );
    assert!(
        extra.is_empty(),
        "symbols exported by the Rust .so but not by the C .so: {:?}",
        extra
    );
    assert_eq!(cs.len(), 16, "unexpected C symbol count: {:?}", cs);
    assert_eq!(cs, rs);
}

#[test]
fn every_documented_symbol_resolves_in_both_libraries() {
    // dlsym both libraries for every name in SYMBOLS.md.
    let s = session();
    for name in ALL_SYMBOLS {
        for lib_path in [c_so_path(), rust_so_path()] {
            let lib = unsafe { libloading::Library::new(&lib_path) }.unwrap();
            let mut nul = name.to_string();
            nul.push('\0');
            let got: Result<libloading::Symbol<'_, unsafe extern "C" fn()>, _> =
                unsafe { lib.get(nul.as_bytes()) };
            assert!(
                got.is_ok(),
                "symbol {} not resolvable in {}",
                name,
                lib_path.display()
            );
        }
    }
    // silence "unused" for the session guard
    let _ = s.c.tag;
}

#[test]
fn rust_undefined_symbols_are_all_libc_or_unwind() {
    let r = rust_so_path().display().to_string();
    let und = nm_undefined(&r);
    // Everything must be a versioned glibc / libgcc import or a weak ITM/gmon
    // hook — i.e. nothing from the translated library itself is missing.
    for s in &und {
        let ok = s.contains("@GLIBC")
            || s.contains("@GCC")
            || s.starts_with("_ITM_")
            || s.starts_with("__gmon_start__")
            || s.starts_with("_Unwind_")
            || s == "gettid"
            || s == "statx";
        assert!(ok, "unexpected undefined symbol in Rust .so: {}", s);
        assert!(
            !s.starts_with("stbds_") && s != "strkey" && s != "arr_ins",
            "library symbol left undefined in Rust .so: {}",
            s
        );
    }
}
