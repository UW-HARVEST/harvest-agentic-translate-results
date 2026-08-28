//! Phase D — symbol parity between the C `.so` and the Rust `.so`, enforced by
//! an actual `nm -D` diff rather than by eyeballing `SYMBOLS.md`.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

/// Defined, exported dynamic symbols of a shared object.
fn exported(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let name = it.next()?;
            let kind = it.next()?;
            // Skip the linker-synthesised / CRT bookkeeping symbols that are
            // not part of either library's API.
            const IGNORED: &[&str] = &[
                "_init",
                "_fini",
                "__bss_start",
                "_edata",
                "_end",
                "__odr_asan_gen_",
            ];
            if IGNORED.contains(&name) {
                return None;
            }
            // Only code / data definitions.
            if matches!(kind, "T" | "t" | "D" | "d" | "B" | "b" | "R" | "r" | "W" | "V" | "G") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Undefined dynamic symbols of a shared object.
fn undefined(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "-u", "--format=posix"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm -u failed on {}", path.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next().map(|s| s.to_string()))
        .collect()
}

#[test]
fn sym_01_every_c_symbol_is_exported_by_rust() {
    let c = exported(&c_so_path());
    let r = exported(&rust_so_path());
    println!("C   ({}) exports: {:?}", c_so_path().display(), c);
    println!("Rust({}) exports: {:?}", rust_so_path().display(), r);

    assert!(
        c.contains("normalize"),
        "sanity: the C .so must export `normalize`, got {c:?}"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is MISSING {} symbol(s) exported by the C .so: {missing:?}",
        missing.len()
    );
}

#[test]
fn sym_02_no_unresolved_non_libc_symbols_in_rust() {
    // Everything the Rust cdylib imports must come from libc / libgcc_s /
    // libm, i.e. be resolvable in the already-loaded process image. dlopen
    // succeeding (with immediate resolution below) proves that.
    let u = undefined(&rust_so_path());
    println!("Rust .so imports {} symbols: {:?}", u.len(), u);

    // Loading the library resolves every non-lazy import; force lazy ones to
    // resolve by actually calling through the symbol.
    let f = rust_normalize();
    let src = [3.0f32, 4.0f32];
    let mut dst = [0.0f32; 2];
    unsafe { f(dst.as_mut_ptr(), src.as_ptr(), 2) };
    assert_eq!(dst, [0.6f32, 0.8f32]);

    // No mangled Rust symbols may leak out of the cdylib as *undefined*.
    let leaked: Vec<&String> = u.iter().filter(|s| s.starts_with("_ZN") || s.starts_with("_R")).collect();
    assert!(leaked.is_empty(), "unresolved Rust-mangled imports: {leaked:?}");
}

#[test]
fn sym_03_both_libraries_agree_on_the_smoke_case() {
    let c = c_normalize();
    let r = rust_normalize();
    let src = [3.0f32, 4.0f32];
    let mut a = [0.0f32; 2];
    let mut b = [0.0f32; 2];
    unsafe {
        c(a.as_mut_ptr(), src.as_ptr(), 2);
        r(b.as_mut_ptr(), src.as_ptr(), 2);
    }
    assert_eq!(a.map(f32::to_bits), b.map(f32::to_bits));
}

#[test]
fn sym_04_report_paths() {
    // Fails loudly if either artifact is missing, so the rest of the suite
    // never silently tests one library against itself.
    let cp = c_so_path();
    let rp = rust_so_path();
    println!("C    .so: {}", cp.display());
    println!("Rust .so: {}", rp.display());
    assert!(cp.is_file());
    assert!(rp.is_file());
    assert_ne!(cp, rp);
}
