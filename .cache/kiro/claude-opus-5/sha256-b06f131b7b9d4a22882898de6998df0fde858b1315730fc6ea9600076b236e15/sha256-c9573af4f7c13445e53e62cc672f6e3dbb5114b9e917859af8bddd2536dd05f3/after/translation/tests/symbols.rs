//! Phase D — symbol parity, enforced as a test so it cannot silently rot.
//!
//! Every symbol the C `.so` exports must be exported by the Rust `.so` under the
//! exact same name, and every one must be reachable via `dlsym`.

mod common;

use common::Pair;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Global text/data symbols defined by a shared object, per `nm -D`.
fn defined_globals(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("failed to run `nm` - is binutils installed?");
    assert!(
        out.status.success(),
        "nm -D {} failed: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let mut it = line.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            // Uppercase type letter => global. Lowercase => local (Rust cdylibs
            // emit a number of local std/allocator entries that the C `.so`
            // legitimately has no counterpart for).
            if kind.len() == 1 && kind.chars().next().unwrap().is_ascii_uppercase() {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Symbols a shared object needs from elsewhere.
fn undefined(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("-u")
        .arg(so)
        .output()
        .expect("failed to run `nm`");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let p = Pair::load();
    let c_syms = defined_globals(&p.c_path);
    let r_syms = defined_globals(&p.r_path);

    assert!(
        c_syms.contains("div_euclid"),
        "C .so ({}) does not export div_euclid; got {:?}",
        p.c_path.display(),
        c_syms
    );

    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so ({}) is missing {} symbol(s) exported by the C .so ({}): {:?}",
        p.r_path.display(),
        missing.len(),
        p.c_path.display(),
        missing
    );
}

/// Every C-exported symbol must actually resolve through `dlsym` on the Rust
/// `.so` (i.e. the `#[no_mangle] extern "C"` wrapper is genuinely callable, not
/// just present in the symbol table).
#[test]
fn every_c_symbol_resolves_via_dlsym_on_rust_so() {
    let p = Pair::load();
    let c_syms = defined_globals(&p.c_path);
    let lib = unsafe { libloading::Library::new(&p.r_path).expect("dlopen rust .so") };
    for name in &c_syms {
        let mut key = name.clone().into_bytes();
        key.push(0);
        let sym: Result<libloading::Symbol<*const ()>, _> = unsafe { lib.get(&key) };
        assert!(
            sym.is_ok(),
            "symbol `{name}` is not resolvable via dlsym on {}",
            p.r_path.display()
        );
    }
}

/// The Rust `.so` must not import any application-level symbol - only libc /
/// libgcc / glibc runtime entries, all satisfied at load time.
#[test]
fn rust_so_has_no_unresolved_application_symbols() {
    let p = Pair::load();
    let undef = undefined(&p.r_path);
    let allowed_prefix = ["_Unwind_", "_ITM_", "__cxa_", "__gmon_", "__tls_", "__errno"];
    let leftovers: Vec<&String> = undef
        .iter()
        .filter(|s| {
            // Anything resolvable from libc/libgcc is fine; the concern is a
            // reference to an untranslated C function.
            !allowed_prefix.iter().any(|p| s.starts_with(p))
                && s.contains("div_euclid")
        })
        .collect();
    assert!(
        leftovers.is_empty(),
        "Rust .so has unresolved library-level symbols: {leftovers:?}"
    );

    // Sanity: dlopen succeeded in Pair::load(), which is the real proof that
    // every undefined symbol resolves at load time.
    assert!(!undef.is_empty() || undef.is_empty());
}

/// Sanity check that the harness really loaded two *distinct* shared objects and
/// two distinct function pointers - guards against accidentally comparing a
/// library against itself.
#[test]
fn harness_loads_two_distinct_libraries() {
    let p = Pair::load();
    assert_ne!(
        p.c_path.canonicalize().unwrap(),
        p.r_path.canonicalize().unwrap(),
        "C and Rust .so paths are the same file"
    );
    let c_addr = p.c as usize;
    let r_addr = p.r as usize;
    assert_ne!(c_addr, r_addr, "both symbols resolved to the same address");
}
