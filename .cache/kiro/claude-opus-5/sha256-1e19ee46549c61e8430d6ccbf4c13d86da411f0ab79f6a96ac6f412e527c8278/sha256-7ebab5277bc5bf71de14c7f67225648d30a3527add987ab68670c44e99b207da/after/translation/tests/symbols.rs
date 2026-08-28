//! Export parity: every dynamic symbol the C `.so` defines must also be
//! defined by the Rust `cdylib`, under the identical name.

mod common;

use libloading::{Library, Symbol};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Global/weak *defined* dynamic symbols, as reported by `nm -D`.
fn exported_symbols(lib: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(lib)
        .output()
        .expect("failed to run `nm`; it must be on PATH");
    assert!(
        out.status.success(),
        "nm -D --defined-only {} failed: {}",
        lib.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    let text = String::from_utf8_lossy(&out.stdout);
    let mut symbols = BTreeSet::new();
    for line in text.lines() {
        let mut fields = line.split_whitespace();
        // Either "<addr> <type> <name>" or "<type> <name>" for absolute syms.
        let fields: Vec<&str> = fields.by_ref().collect();
        let (kind, name) = match fields.as_slice() {
            [_addr, kind, name] => (*kind, *name),
            [kind, name] => (*kind, *name),
            _ => continue,
        };
        // Uppercase codes are global; lowercase are local and not part of the
        // library's ABI surface.
        if kind.chars().all(|c| c.is_ascii_uppercase()) {
            symbols.insert(name.to_string());
        }
    }
    symbols
}

#[test]
fn rust_exports_superset_of_c_exports() {
    let c_lib = common::c_library_path();
    let rust_lib = common::rust_library_path();

    let c_syms = exported_symbols(&c_lib);
    let rust_syms = exported_symbols(&rust_lib);

    assert!(
        !c_syms.is_empty(),
        "no exported symbols found in {}",
        c_lib.display()
    );

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "the Rust cdylib is missing exports present in the C library: {missing:?}\n\
         C exports:    {c_syms:?}\n\
         Rust exports: {rust_syms:?}"
    );
}

/// The C header's public API, spelled out, must be resolvable by `dlsym` in
/// both libraries with a matching signature.
#[test]
fn public_api_symbols_are_loadable() {
    const PUBLIC_API: [&[u8]; 1] = [b"div_euclid\0"];

    let c_lib = common::c_library_path();
    let rust_lib = common::rust_library_path();

    unsafe {
        let c = Library::new(&c_lib).expect("load C library");
        let rust = Library::new(&rust_lib).expect("load Rust library");

        for name in PUBLIC_API {
            let readable = String::from_utf8_lossy(&name[..name.len() - 1]).to_string();
            let c_fn: Symbol<common::DivEuclidFn> = c
                .get(name)
                .unwrap_or_else(|e| panic!("C library lacks {readable}: {e}"));
            let rust_fn: Symbol<common::DivEuclidFn> = rust
                .get(name)
                .unwrap_or_else(|e| panic!("Rust library lacks {readable}: {e}"));
            assert_eq!((*c_fn)(17, 5), (*rust_fn)(17, 5));
        }
    }
}
