//! Phase D — symbol parity, enforced as a test so it cannot silently rot.

mod common;

use std::process::Command;

fn dynamic_symbols(so: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("nm must be available");
    assert!(
        out.status.success(),
        "nm failed on {:?}: {}",
        so,
        String::from_utf8_lossy(&out.stderr)
    );
    let mut names: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .filter(|n| !n.is_empty())
        .collect();
    names.sort();
    names.dedup();
    names
}

#[test]
fn symbol_parity_c_subset_of_rust() {
    let c = dynamic_symbols(&common::c_shared_object());
    let r = dynamic_symbols(&common::rust_shared_object());

    assert!(
        c.contains(&"get_predict_func".to_string()),
        "sanity: C .so must export get_predict_func; got {c:?}"
    );

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C:    {c:?}\n\
         Rust: {r:?}"
    );
}

#[test]
fn symbol_parity_no_extra_exports_in_rust() {
    let c = dynamic_symbols(&common::c_shared_object());
    let r = dynamic_symbols(&common::rust_shared_object());
    // The C's helpers all have internal linkage (`static`); the Rust must not
    // promote any of them into the dynamic symbol table.
    let leaked: Vec<&String> = r
        .iter()
        .filter(|s| !c.contains(s) && s.starts_with("BTAC1C2"))
        .collect();
    assert!(
        leaked.is_empty(),
        "Rust .so exports internal-linkage C helpers it should not: {leaked:?}"
    );
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let out = Command::new("nm")
        .arg("-D")
        .arg("-u")
        .arg(common::rust_shared_object())
        .output()
        .expect("nm");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    let bad: Vec<&str> = text
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .filter(|l| {
            !l.contains("@GLIBC")
                && !l.contains("@GCC")
                && !l.contains("_ITM_")
                && !l.contains("__gmon_start__")
        })
        .collect();
    assert!(bad.is_empty(), "unresolved non-libc symbols: {bad:?}");
}

/// The exported symbol must be reachable by exact name through `dlsym` in
/// both libraries — this is what an external consumer actually does.
#[test]
fn exported_symbol_is_dlsym_reachable_in_both() {
    let p = common::Pair::load();
    // Loading succeeded, which means dlsym("get_predict_func") worked on both.
    // Do one call each to prove the pointers are live.
    assert_eq!(unsafe { (p.c)(0) }, unsafe { (p.rust)(0) });
}
