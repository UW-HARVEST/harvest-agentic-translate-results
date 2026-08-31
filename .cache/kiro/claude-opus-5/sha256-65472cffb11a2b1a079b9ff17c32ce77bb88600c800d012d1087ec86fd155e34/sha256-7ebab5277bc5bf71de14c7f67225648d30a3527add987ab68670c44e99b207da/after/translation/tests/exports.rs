//! ABI surface: every symbol the C `.so` exports must be exported by the Rust
//! `.so` under exactly the same name, and must be callable through `dlsym`.

mod common;

use common::{Libs, exported_symbols};

#[test]
fn rust_exports_superset_of_c_exports() {
    let libs = Libs::load();
    let c_syms = exported_symbols(&libs.c_path);
    let rust_syms = exported_symbols(&libs.rust_path);

    assert!(
        !c_syms.is_empty(),
        "nm reported no exports for {}",
        libs.c_path.display()
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !rust_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing exports present in the C .so: {missing:?}\n  C   ({}): {c_syms:?}\n  Rust({}): {rust_syms:?}",
        libs.c_path.display(),
        libs.rust_path.display()
    );
}

#[test]
fn every_c_export_is_resolvable_in_the_rust_so() {
    let libs = Libs::load();
    for name in exported_symbols(&libs.c_path) {
        let mut sym = name.clone().into_bytes();
        sym.push(0);
        unsafe { libs.rust.get::<*const ()>(&sym) }
            .unwrap_or_else(|e| panic!("dlsym(\"{name}\") failed on the Rust .so: {e}"));
    }
}

/// The documented API plus the incidentally-external helper.
#[test]
fn expected_symbols_are_present_in_both() {
    let libs = Libs::load();
    for lib in [&libs.c, &libs.rust] {
        unsafe { lib.get::<*const ()>(b"driver\0") }.expect("`driver` must be exported");
        unsafe { lib.get::<*const ()>(b"foo\0") }.expect("`foo` must be exported");
    }
}
