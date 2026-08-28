//! Phase D - symbol parity.
//!
//! Re-runs `nm -D` on both shared objects at test time and requires the symbol
//! diff to be empty, so `SYMBOLS.md` cannot silently drift out of date.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Symbols emitted by the toolchain/runtime rather than by the library source.
/// These are excluded from the diff; see the table in `SYMBOLS.md`.
const TOOLCHAIN_GLUE: &[&str] = &[
    "_ITM_deregisterTMCloneTable",
    "_ITM_registerTMCloneTable",
    "__cxa_finalize",
    "__gmon_start__",
    "__rust_no_alloc_shim_is_unstable_v2",
];

fn nm_d(path: &Path) -> String {
    let out = Command::new("nm")
        .arg("-D")
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm -D {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Parse `nm -D` output, keeping only symbols with the requested type letters.
fn parse(text: &str, keep: &dyn Fn(char) -> bool) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let mut fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.is_empty() {
            continue;
        }
        // Layout is either "<addr> <type> <name>" or "<type> <name>".
        let name = fields.pop().unwrap();
        let Some(ty) = fields.pop() else { continue };
        let Some(ty) = ty.chars().next() else { continue };
        if !keep(ty) {
            continue;
        }
        // Strip any "@GLIBC_x.y" version suffix before comparing.
        let bare = name.split('@').next().unwrap_or(name);
        if TOOLCHAIN_GLUE.contains(&bare) {
            continue;
        }
        set.insert(bare.to_string());
    }
    set
}

fn defined(text: &str) -> BTreeSet<String> {
    // Defined and exported: T (text), D/B (data/bss), R (rodata), W/V (weak def).
    parse(text, &|t| matches!(t, 'T' | 'D' | 'B' | 'R' | 'W' | 'V' | 'i' | 'I'))
}

fn undefined(text: &str) -> BTreeSet<String> {
    parse(text, &|t| t == 'U')
}

#[test]
fn every_c_defined_symbol_is_exported_by_rust() {
    let c = nm_d(common::c_so_path());
    let rust = nm_d(common::rust_so_path());

    let c_defined = defined(&c);
    let rust_defined = defined(&rust);

    assert!(
        !c_defined.is_empty(),
        "parsed no defined symbols from the C .so - the parser is broken:\n{c}"
    );

    let missing: Vec<_> = c_defined.difference(&rust_defined).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {:?}\n\
         C defined:    {:?}\nRust defined: {:?}",
        missing.len(),
        missing,
        c_defined,
        rust_defined
    );

    // Every symbol must also actually be resolvable through dlsym.
    for name in &c_defined {
        let mut bytes = name.clone().into_bytes();
        bytes.push(0);
        unsafe {
            common::c_lib()
                .get::<*const ()>(&bytes)
                .unwrap_or_else(|e| panic!("dlsym({name}) failed on the C .so: {e}"));
            common::rust_lib()
                .get::<*const ()>(&bytes)
                .unwrap_or_else(|e| panic!("dlsym({name}) failed on the Rust .so: {e}"));
        }
    }
}

/// The Rust `.so` must have **no unresolved non-libc symbols**.
///
/// Rather than guess at an allowlist of libc names, this is proven the way the
/// dynamic loader proves it: open the library with `RTLD_NOW`, which forces
/// *eager* binding of every relocation, including function symbols that lazy
/// binding would otherwise defer. If any undefined symbol could not be satisfied
/// by libc / libgcc_s / the loader, `dlopen` fails and the test fails with the
/// offending name in the error string.
#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    use libloading::os::unix::{Library as UnixLibrary, RTLD_LOCAL, RTLD_NOW};

    for path in [common::rust_so_path(), common::c_so_path()] {
        // SAFETY: eagerly loading a leaf library we just built ourselves.
        let lib = unsafe { UnixLibrary::open(Some(path), RTLD_NOW | RTLD_LOCAL) };
        let lib = lib.unwrap_or_else(|e| {
            panic!(
                "RTLD_NOW dlopen of {} failed, so it has an unresolvable symbol: {e}",
                path.display()
            )
        });
        // `rev16` must be reachable in the eagerly-bound handle too.
        // SAFETY: signature taken from c_src/include/lib.h.
        let f = unsafe { lib.get::<common::Rev16Fn>(b"rev16\0") }
            .unwrap_or_else(|e| panic!("rev16 missing from {}: {e}", path.display()));
        assert_eq!(unsafe { f(0x0000_0001) }, 0x0000_8000);
        drop(lib);
    }

    // Document the residual undefined set: every entry must be a symbol the
    // process already has, i.e. resolvable through the global handle.
    let undef = undefined(&nm_d(common::rust_so_path()));
    let mut unresolvable = Vec::new();
    for name in &undef {
        let mut bytes = name.clone().into_bytes();
        bytes.push(0);
        // SAFETY: RTLD_DEFAULT lookup of an already-loaded symbol.
        let found = unsafe {
            UnixLibrary::this()
                .get::<*const ()>(&bytes)
                .is_ok()
        };
        if !found {
            unresolvable.push(name.clone());
        }
    }
    assert!(
        unresolvable.is_empty(),
        "Rust .so references symbols that resolve nowhere in the process: \
         {unresolvable:?}\nfull undefined set: {undef:?}"
    );
}

#[test]
fn rust_so_exports_no_symbols_the_c_so_lacks() {
    let c = defined(&nm_d(common::c_so_path()));
    let rust = defined(&nm_d(common::rust_so_path()));

    let extra: Vec<_> = rust.difference(&c).collect();
    assert!(
        extra.is_empty(),
        "the Rust .so exports symbols absent from the C .so: {extra:?}"
    );
}

#[test]
fn the_single_exported_symbol_is_rev16() {
    let c = defined(&nm_d(common::c_so_path()));
    assert_eq!(
        c.iter().map(String::as_str).collect::<Vec<_>>(),
        vec!["rev16"],
        "the C library's exported surface changed; update SYMBOLS.md"
    );
}
