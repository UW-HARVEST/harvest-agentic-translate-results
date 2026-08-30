#![allow(non_snake_case)]
//! Phase A — symbol parity + ABI layout, checked through the .so boundary.

mod common;
use common::*;

/// Every symbol the C .so exports must also be exported by the Rust .so.
#[test]
fn symbol_parity() {
    let l = libs();
    let c_syms = nm_defined(&l.c_path);
    let r_syms = nm_defined(&l.r_path);
    assert!(
        c_syms.len() >= 38,
        "expected >= 38 exported C symbols, got {}: {:?}",
        c_syms.len(),
        c_syms
    );

    // Every C symbol must be loadable from BOTH libraries.
    let mut missing = Vec::new();
    for s in &c_syms {
        let ok_c = unsafe { l.c.get::<*const ()>(s.as_bytes()) }.is_ok();
        let ok_r = unsafe { l.r.get::<*const ()>(s.as_bytes()) }.is_ok();
        assert!(ok_c, "sanity: C .so cannot dlsym its own symbol {s}");
        if !ok_r || !r_syms.contains(s) {
            missing.push(s.clone());
        }
    }
    assert!(
        missing.is_empty(),
        "symbols exported by C .so but MISSING from Rust .so: {missing:?}"
    );
    eprintln!("symbol parity OK: {} symbols", c_syms.len());
}

/// The Rust .so must not need any non-libc symbol resolved externally.
#[test]
fn no_undefined_non_libc_symbols() {
    let l = libs();
    let out = std::process::Command::new("nm")
        .args(["-D", "-u", "--format=posix"])
        .arg(&l.r_path)
        .output()
        .expect("run nm");
    let text = String::from_utf8_lossy(&out.stdout);
    let bad: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .filter(|s| s.starts_with("c2") || *s == "reverse_collide")
        .collect();
    assert!(bad.is_empty(), "unresolved library symbols: {bad:?}");
}

fn nm_defined(p: &std::path::Path) -> Vec<String> {
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(p)
        .output()
        .expect("run nm");
    let text = String::from_utf8_lossy(&out.stdout);
    let mut v: Vec<String> = text
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let name = it.next()?;
            let kind = it.next()?;
            if kind == "T" || kind == "t" {
                Some(name.to_string())
            } else {
                None
            }
        })
        .filter(|s| s.starts_with("c2") || s == "reverse_collide")
        .collect();
    v.sort();
    v.dedup();
    v
}

/// ABI layout of every struct crossing the boundary.
#[test]
fn layout_sizes() {
    use std::mem::{align_of, size_of};
    assert_eq!((size_of::<c2v>(), align_of::<c2v>()), (8, 4));
    assert_eq!((size_of::<c2r>(), align_of::<c2r>()), (8, 4));
    assert_eq!((size_of::<c2x>(), align_of::<c2x>()), (16, 4));
    assert_eq!((size_of::<c2Circle>(), align_of::<c2Circle>()), (12, 4));
    assert_eq!((size_of::<c2AABB>(), align_of::<c2AABB>()), (16, 4));
    assert_eq!((size_of::<c2Capsule>(), align_of::<c2Capsule>()), (20, 4));
    assert_eq!((size_of::<c2GJKCache>(), align_of::<c2GJKCache>()), (36, 4));
    assert_eq!((size_of::<c2Proxy>(), align_of::<c2Proxy>()), (72, 4));
    assert_eq!((size_of::<c2sv>(), align_of::<c2sv>()), (36, 4));
    // 4 * 36 + 4 + 4
    assert_eq!((size_of::<c2Simplex>(), align_of::<c2Simplex>()), (152, 4));
}

/// Struct-by-value return ABI (`c2v` / `c2r` / `c2x` in xmm registers).
#[test]
fn struct_return_abi() {
    let (cv, rv) = pair::<FnV_ff>("c2V");
    same("c2V(1.5,-2.5)", cv(1.5, -2.5), rv(1.5, -2.5));
    let (cri, rri) = pair::<FnR_void>("c2RotIdentity");
    same("c2RotIdentity", cri(), rri());
    let (cxi, rxi) = pair::<FnX_void>("c2xIdentity");
    same("c2xIdentity", cxi(), rxi());
}
