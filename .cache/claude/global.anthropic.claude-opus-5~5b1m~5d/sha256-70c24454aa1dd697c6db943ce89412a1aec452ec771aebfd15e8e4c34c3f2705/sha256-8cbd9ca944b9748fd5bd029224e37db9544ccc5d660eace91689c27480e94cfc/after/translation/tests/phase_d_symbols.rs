//! Phase D — symbol parity between the C and Rust shared objects.
//!
//! Every symbol the C `.so` exports must be exported by the Rust `.so` under the
//! exact same name, and every one must be resolvable through `dlsym`.

mod common;

use common::{c_so_path, defined_dynamic_symbols, rust_so_path};

#[test]
fn every_c_exported_symbol_is_exported_by_rust() {
    let c = defined_dynamic_symbols(&c_so_path());
    let rust = defined_dynamic_symbols(&rust_so_path());

    assert!(
        !c.is_empty(),
        "nm reported no exported symbols for the C library -- is it built?"
    );

    let missing: Vec<&String> = c.iter().filter(|s| !rust.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}\n\
         C   : {c:?}\n\
         Rust: {rust:?}"
    );
}

#[test]
fn expected_symbols_are_present_and_dlsym_resolvable() {
    // Declared in c_src/include/staticalias.h; both must be loadable.
    let expected = ["driver", "static_alias"];

    for so in [c_so_path(), rust_so_path()] {
        let syms = defined_dynamic_symbols(&so);
        for name in expected {
            assert!(
                syms.iter().any(|s| s == name),
                "{so:?} does not export `{name}`; nm -D reports {syms:?}"
            );
        }
        // dlsym them for real (this is what exercises the #[no_mangle] wrappers).
        // SAFETY: plain C shared object.
        let lib = unsafe { libloading::Library::new(&so) }.expect("dlopen");
        for name in expected {
            let mut probe = name.as_bytes().to_vec();
            probe.push(0);
            // SAFETY: the symbol is looked up as an opaque function pointer.
            // `Option<fn>` makes a NULL resolution observable instead of UB.
            let sym: libloading::Symbol<Option<unsafe extern "C" fn()>> =
                unsafe { lib.get(&probe) }.unwrap_or_else(|e| panic!("{so:?} dlsym {name}: {e}"));
            assert!((*sym).is_some(), "{so:?}: {name} resolved to NULL");
        }
    }
}

/// Any *extra* symbol the Rust `.so` exports beyond the C surface is reported for
/// visibility. Rust `cdylib`s do not normally leak anything, and an accidental
/// export would be a translation smell rather than a hard failure of the gate.
#[test]
fn rust_exports_no_unexpected_public_symbols() {
    let rust = defined_dynamic_symbols(&rust_so_path());
    let allowed = ["driver", "static_alias"];
    let extra: Vec<&String> = rust
        .iter()
        .filter(|s| !allowed.contains(&s.as_str()))
        // Ignore the linker/CRT boilerplate every ELF DSO carries.
        .filter(|s| {
            !s.starts_with("_init")
                && !s.starts_with("_fini")
                && !s.starts_with("__")
                && !s.starts_with("_ITM_")
                && !s.contains("rust_eh_personality")
        })
        .collect();
    assert!(
        extra.is_empty(),
        "Rust .so exports unexpected symbols: {extra:?}"
    );
}
