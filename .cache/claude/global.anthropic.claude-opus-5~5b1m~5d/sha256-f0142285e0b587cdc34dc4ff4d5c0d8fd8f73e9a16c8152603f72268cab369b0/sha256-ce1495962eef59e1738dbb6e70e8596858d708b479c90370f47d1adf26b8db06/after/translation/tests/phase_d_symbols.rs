//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Runs `nm -D` on both shared objects and asserts every dynamic symbol the C
//! library defines is also defined by the Rust library under the *exact* same
//! name, then proves each one is actually resolvable via `dlsym`.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Symbols emitted by the toolchain rather than by the library's own source.
/// These are not part of the API surface and are excluded from the comparison.
fn is_toolchain_artifact(name: &str) -> bool {
    name.starts_with("_ITM_")
        || name.starts_with("__cxa_")
        || name.starts_with("_Unwind_")
        || name.starts_with("__gmon_start__")
        || name.starts_with("__tls_get_addr")
        || name.contains("@GLIBC")
        || matches!(
            name,
            "_init" | "_fini" | "__bss_start" | "_edata" | "_end" | "__odr_asan_gen_"
        )
}

/// `nm -D --defined-only <so>` -> set of defined dynamic symbol names.
fn defined_dynamic_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("failed to run `nm` on {}: {e}", so.display()));
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| line.split_whitespace().nth(2))
        .filter(|name| !is_toolchain_artifact(name))
        .map(str::to_owned)
        .collect()
}

/// `nm -D -u <so>` -> undefined dynamic symbols, excluding libc/toolchain.
fn undefined_non_libc_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("-u")
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("failed to run `nm -u` on {}: {e}", so.display()));
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| line.split_whitespace().nth(1))
        .filter(|name| !is_toolchain_artifact(name))
        .map(str::to_owned)
        .collect()
}

#[test]
fn d1_every_c_symbol_is_exported_by_rust() {
    let im = impls();
    let c_syms = defined_dynamic_symbols(&im.c_path);
    let rust_syms = defined_dynamic_symbols(&im.rust_path);

    eprintln!("C  .so {} defines {} symbol(s): {c_syms:?}", im.c_path.display(), c_syms.len());
    eprintln!(
        "Rust.so {} defines {} symbol(s): {rust_syms:?}",
        im.rust_path.display(),
        rust_syms.len()
    );

    assert!(
        !c_syms.is_empty(),
        "nm found no symbols in the C .so - the comparison would be vacuous"
    );

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "{} symbol(s) exported by the C .so are MISSING from the Rust .so: {missing:?}\n\
         Per Phase A: add the #[no_mangle] wrapper if the impl exists, or translate \
         the missing C source if a whole module was skipped.",
        missing.len()
    );

    // The C library's entire API is this one function; assert it explicitly so
    // the test cannot pass by comparing two empty sets.
    assert!(
        c_syms.contains("ldexp_q2"),
        "expected `ldexp_q2` among the C symbols, got {c_syms:?}"
    );
}

#[test]
fn d2_rust_so_has_no_undefined_non_libc_symbols() {
    let im = impls();
    let undef = undefined_non_libc_symbols(&im.rust_path);
    assert!(
        undef.is_empty(),
        "Rust .so has undefined non-libc symbols: {undef:?}"
    );
}

#[test]
fn d3_every_c_symbol_resolves_via_dlsym_in_the_rust_so() {
    let im = impls();
    let c_syms = defined_dynamic_symbols(&im.c_path);
    let lib = unsafe { libloading::Library::new(&im.rust_path) }.expect("dlopen Rust .so");
    for name in &c_syms {
        let mut key = name.clone().into_bytes();
        key.push(0);
        let sym: Result<libloading::Symbol<*const ()>, _> = unsafe { lib.get(&key) };
        assert!(
            sym.is_ok(),
            "symbol `{name}` is listed by nm but does not resolve via dlsym in {}",
            im.rust_path.display()
        );
    }
    eprintln!("D3: all {} C symbol(s) resolve via dlsym in the Rust .so", c_syms.len());
}

/// `g_expfrac` is a *function-local* `static const` in the C source, so it has
/// internal linkage and must NOT be exported by either library.
#[test]
fn d4_internal_linkage_data_is_not_exported() {
    let im = impls();
    for so in [&im.c_path, &im.rust_path] {
        let syms = defined_dynamic_symbols(so);
        for s in &syms {
            assert!(
                !s.to_lowercase().contains("expfrac"),
                "{} unexpectedly exports internal-linkage data `{s}`",
                so.display()
            );
        }
    }
}

/// The Rust `.so` must not export extra Rust-mangled or helper symbols that
/// would signal an accidentally-public surface.
#[test]
fn d5_rust_so_exports_no_mangled_extras() {
    let im = impls();
    let rust_syms = defined_dynamic_symbols(&im.rust_path);
    let c_syms = defined_dynamic_symbols(&im.c_path);
    let extra: Vec<&String> = rust_syms.difference(&c_syms).collect();
    let mangled: Vec<&&String> = extra
        .iter()
        .filter(|s| s.starts_with("_ZN") || s.contains("17h") || s.starts_with("_RN"))
        .collect();
    assert!(
        mangled.is_empty(),
        "Rust .so exports Rust-mangled symbols: {mangled:?}"
    );
    if !extra.is_empty() {
        eprintln!("D5: note - Rust .so has {} extra non-mangled symbol(s): {extra:?}", extra.len());
    }
}
