// Phase A / Phase D — symbol parity.
//
// Every dynamic symbol the C `.so` exports must also be exported by the Rust
// `.so` under the exact same name. The diff must be EMPTY.

mod common;

use common::*;
use std::process::Command;

/// Extract the names of the dynamic, *defined*, global text/data symbols.
fn nm_defined(path: &std::path::Path) -> Option<Vec<String>> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().unwrap()])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            // Global text / initialised data / bss / weak — anything a caller
            // could legitimately bind to.
            if matches!(kind, "T" | "D" | "B" | "R" | "W" | "V" | "G" | "S") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect();
    v.sort();
    v.dedup();
    Some(v)
}

#[test]
fn sym_01_nm_diff_is_empty() {
    let l = libs();
    let (c, r) = match (nm_defined(&l.c_path), nm_defined(&l.rust_path)) {
        (Some(c), Some(r)) => (c, r),
        _ => {
            eprintln!("`nm` unavailable — dlsym check (sym_02) still covers parity");
            return;
        }
    };
    assert!(
        !c.is_empty(),
        "nm found no exported symbols in the C .so ({:?})",
        l.c_path
    );

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C   ({}): {c:?}\n\
         RUST({}): {r:?}",
        missing.len(),
        c.len(),
        r.len()
    );

    // Sanity: the documented set is exactly what the C library exports.
    let mut expected: Vec<String> = EXPORTED_SYMBOLS.iter().map(|s| s.to_string()).collect();
    expected.sort();
    assert_eq!(
        c, expected,
        "SYMBOLS.md is stale: the C .so export set changed"
    );
}

#[test]
fn sym_02_every_c_symbol_dlsyms_in_rust() {
    let l = libs();
    for name in EXPORTED_SYMBOLS {
        let key = format!("{name}\0");
        unsafe {
            let c: Result<libloading::Symbol<*const ()>, _> = l.c.get(key.as_bytes());
            let r: Result<libloading::Symbol<*const ()>, _> = l.rust.get(key.as_bytes());
            assert!(c.is_ok(), "C .so does not export `{name}`");
            assert!(
                r.is_ok(),
                "Rust .so does not export `{name}` (add the #[no_mangle] extern \"C\" wrapper, \
                 or translate the missing C module)"
            );
            assert!(!c.unwrap().is_null(), "C `{name}` resolved to NULL");
            assert!(!r.unwrap().is_null(), "Rust `{name}` resolved to NULL");
        }
    }
}

#[test]
fn sym_03_no_undefined_non_libc_symbols_in_rust() {
    let l = libs();
    let out = match Command::new("nm")
        .args(["-D", "--undefined-only", l.rust_path.to_str().unwrap()])
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return,
    };
    // Anything the loader must resolve either comes from glibc / libgcc_s
    // (versioned or well-known) or is a weak stub. A leftover Rust-mangled or
    // project-local name would mean an untranslated dependency.
    let bad: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(1).map(str::to_string))
        .filter(|n| n.starts_with("_ZN") || EXPORTED_SYMBOLS.contains(&n.as_str()))
        .collect();
    assert!(
        bad.is_empty(),
        "Rust .so has unresolved non-libc symbols: {bad:?}"
    );
}
