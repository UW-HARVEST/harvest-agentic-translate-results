//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Every dynamic symbol the C library exports must be exported by the Rust
//! library under the exact same name, and the Rust library must not leave any
//! non-libc symbol undefined.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

/// Boilerplate the linker/CRT adds to every ELF shared object; not part of the
/// library's own surface.
const CRT_NOISE: &[&str] = &[
    "_init",
    "_fini",
    "__bss_start",
    "_edata",
    "_end",
    "_ITM_deregisterTMCloneTable",
    "_ITM_registerTMCloneTable",
    "__cxa_finalize",
    "__gmon_start__",
    "__register_frame_info",
    "__deregister_frame_info",
    "_Jv_RegisterClasses",
];

fn nm(args: &[&str], path: &str) -> Vec<String> {
    let out = Command::new("nm")
        .args(args)
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {path}: {e}"));
    assert!(
        out.status.success(),
        "nm {args:?} {path} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .map(|s| s.split('@').next().unwrap().to_string())
        .collect()
}

fn defined(path: &str) -> BTreeSet<String> {
    nm(&["-D", "--defined-only"], path)
        .into_iter()
        .filter(|s| !CRT_NOISE.contains(&s.as_str()))
        .collect()
}

fn undefined(path: &str) -> BTreeSet<String> {
    nm(&["-D", "-u"], path)
        .into_iter()
        .filter(|s| !CRT_NOISE.contains(&s.as_str()))
        .collect()
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c = c_so_path().to_string_lossy().to_string();
    let r = rust_so_path().to_string_lossy().to_string();
    let c_syms = defined(&c);
    let r_syms = defined(&r);

    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {missing:?}\n\
         C exports:    {c_syms:?}\n\
         Rust exports: {r_syms:?}",
        missing.len()
    );

    // Positive check: the surface is non-empty and contains `merge_sort`.
    assert!(!c_syms.is_empty(), "no symbols found in the C .so — bad nm parse?");
    assert!(c_syms.contains("merge_sort"), "C .so lost `merge_sort`: {c_syms:?}");
    assert!(r_syms.contains("merge_sort"), "Rust .so lost `merge_sort`: {r_syms:?}");
}

#[test]
fn c_static_helpers_are_not_exported_by_either() {
    let c = c_so_path().to_string_lossy().to_string();
    let r = rust_so_path().to_string_lossy().to_string();
    let c_syms = defined(&c);
    let r_syms = defined(&r);
    for name in [
        "spritebatch_internal_sprite_less_than_or_equal",
        "spritebatch_internal_merge_sort_iteration",
        "spritebatch_internal_merge_sort_recurse",
    ] {
        assert!(!c_syms.contains(name), "C unexpectedly exports `{name}`");
        assert!(
            !r_syms.contains(name),
            "Rust exports `{name}` but it is `static` in the C — surface mismatch"
        );
    }
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let r = rust_so_path().to_string_lossy().to_string();
    let undef = undefined(&r);
    // Everything the Rust cdylib imports must come from libc / the dynamic
    // loader / the unwinder, i.e. must be resolvable in this process.
    let unresolved: Vec<&String> = undef
        .iter()
        .filter(|s| {
            let s = s.as_str();
            !(s.starts_with("__libc")
                || s.starts_with("_Unwind")
                || s.starts_with("__tls")
                || s.starts_with("__rust")
                || s.starts_with("__cxa")
                || s.starts_with("_ZN")
                || is_resolvable(s))
        })
        .collect();
    assert!(
        unresolved.is_empty(),
        "Rust .so has unresolved non-libc symbols: {unresolved:?} (all undefined: {undef:?})"
    );
}

/// Can this symbol be found in the already-loaded process image (libc, libm,
/// libgcc, ld.so)? If yes, it is not a translation gap.
fn is_resolvable(name: &str) -> bool {
    let mut c = name.as_bytes().to_vec();
    c.push(0);
    unsafe {
        let h = libc::dlsym(libc::RTLD_DEFAULT, c.as_ptr() as *const libc::c_char);
        !h.is_null()
    }
}

#[test]
fn rust_so_exports_no_extra_public_api() {
    // The Rust .so may carry allocator/panic-runtime glue, but it must not
    // publish an extra *library* entry point that the C does not have; the two
    // `merge_sort`-shaped surfaces must be the same size.
    let c = c_so_path().to_string_lossy().to_string();
    let r = rust_so_path().to_string_lossy().to_string();
    let c_syms = defined(&c);
    let r_syms = defined(&r);
    let extra: Vec<&String> = r_syms
        .difference(&c_syms)
        .filter(|s| {
            let s = s.as_str();
            // Rust runtime glue that is not part of the translated API.
            !(s.starts_with("__rust")
                || s.starts_with("rust_")
                || s.starts_with("_ZN")
                || s.starts_with("_R")
                || s.starts_with("__rdl")
                || s.starts_with("_Unwind"))
        })
        .collect();
    assert!(
        extra.is_empty(),
        "Rust .so exports extra public symbols not present in the C .so: {extra:?}"
    );
}
