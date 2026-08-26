//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Mirrors `SYMBOLS.md`. Everything here is derived mechanically from `nm -D`,
//! never from a hand-written list of "important" symbols.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(args: &[&str], so: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(args)
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("running nm on {so:?}: {e}"));
    assert!(
        out.status.success(),
        "nm {args:?} {so:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .filter(|s| !s.is_empty())
        .collect()
}

/// Dynamic, *defined* symbols — i.e. the exported ABI surface.
fn exported(so: &Path) -> BTreeSet<String> {
    nm(&["-D", "--defined-only"], so)
        .into_iter()
        // strip glibc version suffixes, e.g. `foo@GLIBC_2.2.5`
        .map(|s| s.split('@').next().unwrap().to_string())
        .collect()
}

fn undefined(so: &Path) -> BTreeSet<String> {
    nm(&["-D", "-u"], so)
        .into_iter()
        .map(|s| s.split('@').next().unwrap().to_string())
        .collect()
}

/// THE gate: every symbol the C `.so` exports must be exported by the Rust
/// `.so` under the exact same name.
#[test]
fn c_symbols_are_all_exported_by_rust() {
    let c = exported(&c_so());
    let r = exported(&rust_so());

    assert!(
        !c.is_empty(),
        "no exported symbols found in the C .so — did it build?"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C exports:    {c:?}\n\
         Rust exports: {r:?}",
        missing.len()
    );

    // Pin the surface documented in SYMBOLS.md so that a future C change that
    // adds a symbol cannot silently pass.
    let expected: BTreeSet<String> = ["driver", "forward_goto_example", "open_with_cleanup"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(
        c, expected,
        "the C .so's exported surface changed; update SYMBOLS.md"
    );
}

/// Each exported C symbol must also be reachable through `dlsym` on the Rust
/// `.so` (i.e. exported with default visibility, not just present in the file).
#[test]
fn every_c_symbol_is_dlsym_able_in_rust() {
    let c = exported(&c_so());
    let lib = unsafe { libloading::Library::new(rust_so()) }.expect("dlopen rust .so");
    for name in &c {
        let mut bytes = name.clone().into_bytes();
        bytes.push(0);
        let sym: Result<libloading::Symbol<*const ()>, _> = unsafe { lib.get(&bytes) };
        assert!(sym.is_ok(), "dlsym({name}) failed on the Rust .so");
    }
}

/// The Rust `.so` must not reference anything that cannot be resolved:
/// `RTLD_NOW` forces every relocation to be bound at load time.
#[test]
fn rust_so_loads_with_rtld_now() {
    use libloading::os::unix as unix_dl;
    let path = rust_so();
    let lib = unsafe {
        unix_dl::Library::open(Some(&path), libc::RTLD_NOW | libc::RTLD_LOCAL)
    };
    let lib = lib.unwrap_or_else(|e| {
        panic!("dlopen({path:?}, RTLD_NOW) failed — unresolved symbols: {e}")
    });
    // Keep it loaded long enough to be meaningful, then drop.
    drop(lib);
}

/// Every undefined symbol of the Rust `.so` must resolve against the C library
/// / runtime already present in this process (nothing dangling, nothing from a
/// module that was never translated).
#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    // Load both objects with global scope so RTLD_DEFAULT can see them.
    use libloading::os::unix as unix_dl;
    let rl = unsafe {
        unix_dl::Library::open(
            Some(rust_so()),
            libc::RTLD_NOW | libc::RTLD_GLOBAL,
        )
    }
    .expect("dlopen rust .so RTLD_GLOBAL");

    let mut unresolved = Vec::new();
    for name in undefined(&rust_so()) {
        // Weak toolchain hooks are allowed to be absent by design.
        if matches!(
            name.as_str(),
            "_ITM_deregisterTMCloneTable"
                | "_ITM_registerTMCloneTable"
                | "__gmon_start__"
                | "__pthread_key_create"
        ) {
            continue;
        }
        let cname = std::ffi::CString::new(name.clone()).unwrap();
        let p = unsafe { libc::dlsym(libc::RTLD_DEFAULT, cname.as_ptr()) };
        if p.is_null() {
            unresolved.push(name);
        }
    }
    drop(rl);
    assert!(
        unresolved.is_empty(),
        "Rust .so has unresolved symbol(s): {unresolved:?}"
    );
}

/// Both objects must import the same `stdio` entry points — that is what makes
/// the buffering (and therefore the observable byte stream) identical.
#[test]
fn both_objects_import_the_same_stdio_surface() {
    let c = undefined(&c_so());
    let r = undefined(&rust_so());
    // `fwrite` is only in the C object because GCC lowers
    // `fprintf(stderr, "literal")` to `fwrite`; the emitted bytes are the same.
    for sym in ["fopen", "fgets", "ferror", "fclose", "printf", "stderr"] {
        assert!(c.contains(sym), "C .so should import {sym}");
        assert!(r.contains(sym), "Rust .so should import {sym}");
    }
    assert!(
        r.contains("fprintf"),
        "Rust .so should import fprintf: {r:?}"
    );
}
