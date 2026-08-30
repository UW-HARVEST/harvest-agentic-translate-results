//! Phase D -- symbol parity between the C `.so` and the Rust `.so`.
//!
//! Every dynamic symbol the C library exports must also be exported by the Rust
//! library under the exact same name, and the Rust library must have no
//! unresolved (non-libc) symbols. This is checked from inside the test suite so
//! it holds for whichever profile/feature combination is being run.

mod common;

use common::*;
use std::ffi::c_int;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn nm_defined(so: &PathBuf) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
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
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect()
}

/// Both `.so` paths, resolved the same way the harness resolves them.
fn so_paths() -> (PathBuf, PathBuf) {
    // `libs()` asserts both files exist and are loadable.
    let _ = libs();
    let c = std::env::var("STATICALIAS_C_SO").map(PathBuf::from).unwrap_or_else(|_| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/build/libStaticAlias.so")
    });
    let exe = std::env::current_exe().unwrap();
    let rust = std::env::var("STATICALIAS_RUST_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|_| exe.parent().unwrap().parent().unwrap().join("libStaticAlias.so"));
    (c, rust)
}

#[test]
fn sym_01_rust_exports_every_c_symbol() {
    let _g = lock();
    let (c_so, rust_so) = so_paths();
    let c_syms = nm_defined(&c_so);
    let rust_syms = nm_defined(&rust_so);

    println!("C   .so: {} ({} exported symbols)", c_so.display(), c_syms.len());
    println!("Rust.so: {} ({} exported symbols)", rust_so.display(), rust_syms.len());

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );

    // The two documented public entry points must really be there.
    for want in ["static_alias", "driver"] {
        assert!(c_syms.contains(want), "C .so should export `{want}`");
        assert!(rust_syms.contains(want), "Rust .so should export `{want}`");
    }
}

#[test]
fn sym_02_rust_so_has_no_unresolved_symbols() {
    let _g = lock();
    let (_, rust_so) = so_paths();
    let out = Command::new("ldd").arg("-r").arg(&rust_so).output();
    let Ok(out) = out else {
        eprintln!("`ldd` unavailable; skipping");
        return;
    };
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let bad: Vec<&str> = text
        .lines()
        .filter(|l| l.contains("undefined symbol") || l.contains("not found"))
        .collect();
    assert!(
        bad.is_empty(),
        "the Rust .so has unresolved symbols: {bad:?}"
    );
}

#[test]
fn sym_03_symbols_are_callable_through_dlsym() {
    let _g = lock();
    // Not just present in `nm` -- actually resolvable and callable through the
    // export wrappers, which is what `libs()` does for both libraries.
    let l = libs();
    for lib in [&l.c, &l.rust] {
        set_inner(lib, 5);
        let mut v: c_int = 5;
        let r = unsafe { (lib.static_alias)(&mut v) };
        assert_eq!(r, lib.inner_addr, "{}: static_alias via dlsym", lib.name);
        assert_eq!(get_inner(lib), 10);
        set_inner(lib, 1);
        let out = capture_stdout(lib.name, || unsafe { (lib.driver)(1, 3) });
        assert_eq!(out, b"2\n4\n8\n", "{}: driver via dlsym", lib.name);
    }
}

