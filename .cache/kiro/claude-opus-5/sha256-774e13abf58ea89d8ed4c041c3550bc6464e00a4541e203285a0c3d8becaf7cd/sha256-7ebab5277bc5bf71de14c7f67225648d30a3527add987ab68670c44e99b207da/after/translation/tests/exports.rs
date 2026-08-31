//! Step 8: every symbol the C `.so` exports must also be exported by the Rust
//! `.so`, under the exact same name, and must be reachable via `dlsym`.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use common::{IMPLS, Impl, Libs, c_so, rust_so};

/// Defined (exported) dynamic symbols, as `nm -D --defined-only` reports them.
fn exported_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", so.display()));
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            // "<addr> <type> <name>" or "         <type> <name>" for weak/undef.
            let mut parts = line.split_whitespace().rev();
            let name = parts.next()?;
            let kind = parts.next()?;
            // Only code/data actually defined by this object.
            if kind.chars().all(|c| c.is_ascii_uppercase()) && kind.len() == 1 {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Symbols the Rust cdylib emits for its own runtime rather than as API.
fn is_rust_internal(name: &str) -> bool {
    name.starts_with("_ZN")
        || name.starts_with("rust_")
        || name.starts_with("__rust")
        || name.starts_with("_R")
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c = exported_symbols(c_so());
    let rust = exported_symbols(rust_so());

    assert!(
        !c.is_empty(),
        "no exported symbols found in {} -- is it built?",
        c_so().display()
    );

    let missing: Vec<&String> = c.difference(&rust).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}\n  C   : {c:?}\n  Rust: {:?}",
        rust.iter().filter(|s| !is_rust_internal(s)).collect::<Vec<_>>()
    );

    // The C API surface is exactly these two functions; `inner` is `static` and
    // must not appear in either object.
    assert_eq!(
        c,
        ["driver", "fma_array"]
            .iter()
            .map(|s| s.to_string())
            .collect::<BTreeSet<_>>(),
        "unexpected C export set -- extend the test's function coverage"
    );
    assert!(
        !rust.contains("inner"),
        "Rust .so exports `inner`, which is `static` in C"
    );
}

#[test]
fn every_c_symbol_is_dlsym_reachable_in_both() {
    let libs = Libs::get();
    for name in exported_symbols(c_so()) {
        for which in IMPLS {
            // Type is irrelevant for a reachability check.
            let _: libloading::Symbol<'_, unsafe extern "C" fn()> = libs.sym(which, &name);
        }
    }
    // Sanity: a symbol that exists in neither must not resolve.
    let libs = Libs::get();
    for which in IMPLS {
        let lib = match which {
            Impl::C => &libs.c,
            Impl::Rust => &libs.rust,
        };
        let missing: Result<libloading::Symbol<'_, unsafe extern "C" fn()>, _> =
            unsafe { lib.get(b"definitely_not_a_real_symbol") };
        assert!(missing.is_err(), "{which:?} resolved a bogus symbol");
    }
}
