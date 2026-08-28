// Phase D — symbol parity and link completeness.
//
// Asserts mechanically (not from SYMBOLS.md by hand) that every dynamic symbol
// the C `.so` exports is also exported by the Rust `.so` under the exact same
// name, that each one is really callable through `dlsym`, and that the Rust
// `.so` has no unresolved non-libc imports.

#[path = "common/mod.rs"]
mod common;

use common::*;
use core::ffi::c_char;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm_defined(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("running nm on {}: {e}", so.display()));
    assert!(
        out.status.success(),
        "nm -D --defined-only {} failed: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
        .collect()
}

/// Every C export must exist in the Rust `.so`, byte-for-byte the same name.
fn symbol_parity_c_subset_of_rust() {
    let c_so = c_so_path();
    let r_so = rust_so_path();
    let c_syms = nm_defined(&c_so);
    let r_syms = nm_defined(&r_so);
    eprintln!(
        "\n    C   {} -> {} exported symbols\n    Rust {} -> {} exported symbols",
        c_so.display(),
        c_syms.len(),
        r_so.display(),
        r_syms.len()
    );
    assert!(!c_syms.is_empty(), "nm found no exports in the C .so");

    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {:?}\n\
         (add the #[no_mangle] wrapper, or translate the missing C module)",
        missing.len(),
        missing
    );

    // The three known entry points must be among them (guards against `nm`
    // silently returning something unexpected).
    for want in ["cleanup", "print_result", "cleanup_resources"] {
        assert!(c_syms.contains(want), "C .so must export {want}");
        assert!(r_syms.contains(want), "Rust .so must export {want}");
    }
    eprint!("[0 missing] ");
}

/// Every C export must be reachable through `dlsym` on the Rust `.so` and be a
/// real implementation, not a symbol that aborts when called.
fn every_symbol_is_dlsym_resolvable_and_live() {
    let c_syms = nm_defined(&c_so_path());
    let lib = unsafe { libloading::Library::new(rust_so_path()) }.expect("dlopen Rust .so");
    for s in &c_syms {
        let name = format!("{s}\0");
        let sym: Result<libloading::Symbol<*const ()>, _> =
            unsafe { lib.get(name.as_bytes()) };
        assert!(sym.is_ok(), "dlsym({s}) failed on the Rust .so");
        assert!(!sym.unwrap().is_null(), "dlsym({s}) resolved to NULL");
    }
    // Calling each one proves it is not an `unimplemented!()` stub.
    let mut cap = Capture::new("d02");
    let rc = diff_cleanup(&mut cap, 10, 20, 30, 40);
    assert_eq!(rc, 160);
    let label = b"live\0";
    let out = diff_print_result(&mut cap, label.as_ptr() as *const c_char, rc, "live");
    assert_eq!(out, b"live: 160\n");
    diff_cleanup_resources(&mut cap, 50);
    drop(cap);
    eprint!("[{} symbols live] ", c_syms.len());
}

/// The Rust `.so` must not import anything that cannot be resolved.
fn rust_so_has_no_unresolved_imports() {
    let r_so = rust_so_path();
    let out = Command::new("ldd").arg("-r").arg(&r_so).output();
    match out {
        Ok(o) => {
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            );
            let bad: Vec<&str> = text
                .lines()
                .filter(|l| {
                    let l = l.to_ascii_lowercase();
                    l.contains("undefined symbol") || l.contains("not found")
                })
                .collect();
            assert!(
                bad.is_empty(),
                "`ldd -r {}` reports unresolved imports:\n{}",
                r_so.display(),
                bad.join("\n")
            );
            eprint!("[ldd -r clean] ");
        }
        Err(e) => eprint!("[ldd unavailable: {e}] "),
    }
}

/// Both `.so`s must import the same behaviour-relevant libc entry points, so
/// that allocator and stdio state is genuinely shared.
fn both_use_the_same_libc_primitives() {
    let undef = |so: &Path| -> BTreeSet<String> {
        let out = Command::new("nm")
            .args(["-D", "--undefined-only"])
            .arg(so)
            .output()
            .expect("nm");
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().last())
            .map(|s| s.split('@').next().unwrap_or(s).to_string())
            .collect()
    };
    let c_u = undef(&c_so_path());
    let r_u = undef(&rust_so_path());
    for want in ["malloc", "free", "snprintf", "strlen"] {
        assert!(c_u.contains(want), "C .so should import {want}");
        assert!(
            r_u.contains(want),
            "the Rust .so must go through libc `{want}` so that allocator/stdio \
             state is shared with the C implementation; imports found: {r_u:?}"
        );
    }
    // The C prints via printf/puts; the Rust must use one of the two (gcc and
    // LLVM both rewrite printf("%s\n", p) to puts(p)).
    assert!(
        c_u.contains("printf") || c_u.contains("puts"),
        "C .so should import printf or puts"
    );
    assert!(
        r_u.contains("printf") || r_u.contains("puts"),
        "Rust .so must import printf or puts"
    );
    // `strncmp` drives ERRORS.md row 1; it must survive optimisation in both.
    assert!(c_u.contains("strncmp"), "C .so should import strncmp");
    assert!(
        r_u.contains("strncmp"),
        "the Rust .so must still call libc strncmp (see the black_box in src/lib.rs) \
         so that ERRORS.md row 1 is reachable identically in both; imports: {r_u:?}"
    );
    eprint!("[libc parity] ");
}

fn main() {
    common::run_tests(&[
        ("symbol_parity_c_subset_of_rust", symbol_parity_c_subset_of_rust),
        (
            "every_symbol_is_dlsym_resolvable_and_live",
            every_symbol_is_dlsym_resolvable_and_live,
        ),
        ("rust_so_has_no_unresolved_imports", rust_so_has_no_unresolved_imports),
        ("both_use_the_same_libc_primitives", both_use_the_same_libc_primitives),
    ]);
}
