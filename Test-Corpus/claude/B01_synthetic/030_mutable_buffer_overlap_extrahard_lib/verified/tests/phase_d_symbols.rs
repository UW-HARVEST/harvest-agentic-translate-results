// Phase D — symbol parity between the C `.so` and the Rust `.so`.
//
// Every symbol the C shared library exports must be exported by the Rust
// shared library under the exact same name, and the Rust library must not leak
// extra public symbols that the C library keeps `static`.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Global *defined* symbols (`T`/`D`/`B`/`R`/`W`) from the dynamic symbol table.
fn exported_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("failed to run nm (binutils required)");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            // "<addr> <type> <name>" for defined symbols
            if f.len() == 3 && matches!(f[1], "T" | "D" | "B" | "R" | "W" | "V" | "G" | "S") {
                Some(f[2].to_string())
            } else {
                None
            }
        })
        // Ignore the linker/runtime bookkeeping symbols that are not part of
        // either library's API surface.
        .filter(|s| {
            !s.starts_with("_ITM_")
                && !s.starts_with("__")
                && s != "_init"
                && s != "_fini"
                && s != "_edata"
                && s != "_end"
        })
        .collect()
}

fn undefined_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(so)
        .output()
        .expect("failed to run nm");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            // undefined lines look like "                 U name@VER"
            if f.len() == 2 && (f[0] == "U" || f[0] == "w" || f[0] == "v") {
                Some(f[1].split('@').next().unwrap().to_string())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn d_01_every_c_symbol_is_exported_by_rust() {
    let c = exported_symbols(&c_so_path());
    let r = exported_symbols(&rust_so_path());
    assert!(
        !c.is_empty(),
        "no exported symbols found in the C library at {} — is it built?",
        c_so_path().display()
    );
    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   = {c:?}\nRust = {r:?}"
    );
    // The C library exports exactly these two.
    assert!(c.contains("driver"), "C .so must export `driver`: {c:?}");
    assert!(c.contains("fma_array"), "C .so must export `fma_array`: {c:?}");
}

#[test]
fn d_02_rust_does_not_export_the_static_inner() {
    // `inner` is `static` in driver.c, so neither library may export it.
    let c = exported_symbols(&c_so_path());
    let r = exported_symbols(&rust_so_path());
    assert!(!c.contains("inner"), "C .so unexpectedly exports `inner`");
    assert!(
        !r.contains("inner"),
        "the Rust .so exports `inner`, but the C source declares it `static` — \
         that is an ABI divergence"
    );
    // Nor any mangled Rust variant of it.
    for s in &r {
        assert!(
            !s.contains("inner"),
            "the Rust .so leaks an `inner`-like symbol: {s}"
        );
    }
}

#[test]
fn d_03_no_extra_public_api_symbols_in_rust() {
    let c = exported_symbols(&c_so_path());
    let r = exported_symbols(&rust_so_path());
    let extra: Vec<&String> = r.difference(&c).collect();
    assert!(
        extra.is_empty(),
        "the Rust .so exports API symbols the C .so does not: {extra:?}"
    );
}

#[test]
fn d_04_rust_undefined_symbols_all_resolve() {
    // Every undefined symbol must be satisfiable from the libraries the .so
    // actually links against (verified by successfully dlopen'ing it with
    // RTLD_NOW, which resolves *all* relocations eagerly).
    let path = rust_so_path();
    let lib = unsafe {
        libloading::os::unix::Library::open(
            Some(&path),
            libc::RTLD_NOW | libc::RTLD_LOCAL,
        )
    };
    assert!(
        lib.is_ok(),
        "dlopen(RTLD_NOW) of {} failed, so it has unresolved symbols: {:?}",
        path.display(),
        lib.err()
    );

    // And the C library's two libc imports must also be imported by Rust, so
    // that `printf` formatting and `memcpy` semantics come from the same code.
    let cu = undefined_symbols(&c_so_path());
    let ru = undefined_symbols(&rust_so_path());
    for want in ["printf", "memcpy"] {
        assert!(cu.contains(want), "C .so should import {want}: {cu:?}");
        assert!(
            ru.contains(want),
            "the Rust .so must import libc `{want}` (not reimplement it) so the \
             observable behaviour is identical; imports = {ru:?}"
        );
    }
}

#[test]
fn d_05_both_libraries_load_and_expose_callable_symbols() {
    // Sanity: the symbols are not just present in nm output, they are callable.
    let data: Vec<std::ffi::c_int> = vec![3, -4, 0];
    let c = capture_stdout(|| unsafe { (c_lib().driver)(data.as_ptr(), 3) });
    let r = capture_stdout(|| unsafe { (rust_lib().driver)(data.as_ptr(), 3) });
    assert_eq!(c, b"12\n12\n0\n", "C ground truth");
    assert_eq!(c, r, "Rust must match");

    let mut cb = vec![3, -4, 0];
    let mut rb = vec![3, -4, 0];
    let src: Vec<std::ffi::c_int> = vec![2, 5, 7];
    unsafe {
        (c_lib().fma_array)(cb.as_mut_ptr(), src.as_ptr(), src.as_ptr(), src.as_ptr(), 3);
        (rust_lib().fma_array)(rb.as_mut_ptr(), src.as_ptr(), src.as_ptr(), src.as_ptr(), 3);
    }
    assert_eq!(cb, vec![6, 30, 56], "C ground truth");
    assert_eq!(cb, rb, "Rust must match");
}
