//! Phase D — symbol parity between the C and Rust shared objects.
//!
//! This is the mechanical check behind `SYMBOLS.md`: it shells out to `nm -D`
//! on both `.so`s and asserts the diff is empty. It fails the build if the Rust
//! library ever stops exporting something the C library exports.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

/// ELF/loader boilerplate that every shared object gets from the linker and
/// that is not part of the library's own API surface.
const LINKER_BOILERPLATE: &[&str] = &[
    "_init",
    "_fini",
    "_edata",
    "_end",
    "__bss_start",
    "__cxa_finalize",
    "__gmon_start__",
    "_ITM_deregisterTMCloneTable",
    "_ITM_registerTMCloneTable",
    "__tls_get_addr",
    "_Unwind_Resume",
    "__register_frame_info",
    "__deregister_frame_info",
];

/// Symbols provided by the platform libraries the `.so` actually links against.
///
/// Rather than hand-maintaining an allowlist (which is guesswork and goes
/// stale), resolve every dependency with `ldd` and union their exported symbol
/// sets. An undefined symbol in our `.so` is legitimate exactly when the loader
/// can satisfy it from one of those libraries.
fn platform_provided_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("ldd")
        .arg(so)
        .output()
        .expect("running `ldd`");
    let text = String::from_utf8_lossy(&out.stdout);
    let mut provided = BTreeSet::new();
    for line in text.lines() {
        // Formats: "libc.so.6 => /lib64/libc.so.6 (0x...)" or "/lib64/ld-linux... (0x...)"
        let path = if let Some((_, rhs)) = line.split_once("=>") {
            rhs.split_whitespace().next().unwrap_or("")
        } else {
            line.split_whitespace().next().unwrap_or("")
        };
        if path.starts_with('/') && std::path::Path::new(path).exists() {
            provided.extend(nm_symbols(std::path::Path::new(path), &['T', 'W', 'i', 'D', 'B']));
        }
    }
    provided
}

/// Undefined symbols that are the dynamic loader's own business rather than a
/// library dependency.
fn is_loader_boilerplate(name: &str) -> bool {
    LINKER_BOILERPLATE.contains(&name)
        || name.starts_with("_ITM_")
        || name.starts_with("__gmon")
        || name.starts_with("__pthread_key_create")
}

/// `nm -D` output filtered to symbols with the given type letters.
fn nm_symbols(so: &std::path::Path, wanted_types: &[char]) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg(so)
        .output()
        .expect("running `nm -D` (binutils must be installed)");
    assert!(
        out.status.success(),
        "nm -D {so:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        // Formats: "<addr> <T> <name>" or "                 U <name>"
        let mut parts = line.split_whitespace().collect::<Vec<_>>();
        if parts.len() < 2 {
            continue;
        }
        let name = parts.pop().unwrap();
        let ty = parts.pop().unwrap();
        let ty_char = ty.chars().next().unwrap_or('?');
        if ty.len() == 1 && wanted_types.contains(&ty_char) {
            // Strip any version suffix, e.g. `printf@GLIBC_2.2.5`.
            let bare = name.split('@').next().unwrap_or(name);
            set.insert(bare.to_string());
        }
    }
    set
}

fn defined_api_symbols(so: &std::path::Path) -> BTreeSet<String> {
    // 'T' = defined in .text (the functions), 'D'/'B' = defined data.
    nm_symbols(so, &['T', 'D', 'B'])
        .into_iter()
        .filter(|n| !LINKER_BOILERPLATE.contains(&n.as_str()))
        .filter(|n| !n.starts_with("_ZN") && !n.starts_with("anon."))
        .collect()
}

fn undefined_symbols(so: &std::path::Path) -> BTreeSet<String> {
    nm_symbols(so, &['U'])
        .into_iter()
        .filter(|n| !LINKER_BOILERPLATE.contains(&n.as_str()))
        .collect()
}

#[test]
fn d01_every_c_exported_symbol_is_exported_by_rust() {
    let c_so = c_so_path();
    let rust_so = rust_so_path();
    assert!(c_so.exists(), "missing {c_so:?}");
    assert!(rust_so.exists(), "missing {rust_so:?}");

    let c_syms = defined_api_symbols(&c_so);
    let rust_syms = defined_api_symbols(&rust_so);

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C   exports: {c_syms:?}\n\
         Rust exports: {rust_syms:?}",
        missing.len()
    );

    // The C library really does export both of these; guard against the diff
    // passing vacuously because `nm` returned nothing.
    for expected in ["driver", "print_foo"] {
        assert!(
            c_syms.contains(expected),
            "sanity check: C .so should export `{expected}`, got {c_syms:?}"
        );
        assert!(
            rust_syms.contains(expected),
            "Rust .so should export `{expected}`, got {rust_syms:?}"
        );
    }
}

#[test]
fn d02_rust_so_has_no_unresolved_non_libc_imports() {
    let rust_so = rust_so_path();
    let undef = undefined_symbols(&rust_so);
    let provided = platform_provided_symbols(&rust_so);
    assert!(
        provided.len() > 100,
        "ldd/nm probe of platform libraries returned only {} symbols; the check \
         would pass vacuously",
        provided.len()
    );

    let unresolved: Vec<&String> = undef
        .iter()
        .filter(|n| !is_loader_boilerplate(n))
        .filter(|n| !provided.contains(n.as_str()))
        .collect();
    assert!(
        unresolved.is_empty(),
        "Rust .so has {} undefined symbol(s) that no linked platform library \
         provides: {unresolved:?}",
        unresolved.len()
    );

    // `printf` is the C library's only real import; the Rust must import it too
    // (rather than reimplementing formatting, which would risk divergence).
    assert!(
        undef.contains("printf"),
        "Rust .so does not import `printf`; it must go through libc's formatter \
         to stay byte-identical. Undefined set: {undef:?}"
    );
    let c_undef = undefined_symbols(&c_so_path());
    assert!(
        c_undef.contains("printf"),
        "sanity check: C .so should import printf, got {c_undef:?}"
    );
}

#[test]
fn d03_rust_so_exports_nothing_the_c_so_does_not() {
    // Not strictly required for correctness, but an unexpected extra export is
    // a sign of a stub or a leaked helper, so it is worth surfacing.
    let c_syms = defined_api_symbols(&c_so_path());
    let rust_syms = defined_api_symbols(&rust_so_path());
    let extra: Vec<&String> = rust_syms.difference(&c_syms).collect();
    assert!(
        extra.is_empty(),
        "Rust .so exports symbols absent from the C .so: {extra:?}"
    );
}

#[test]
fn d04_no_stubs_in_the_rust_source() {
    // A symbol that exists only to satisfy `nm -D` is worse than a missing one,
    // so assert the translation contains no placeholder bodies.
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"),
    )
    .expect("read src/lib.rs");
    for marker in ["unimplemented!", "todo!", "unreachable!(\"stub", "panic!(\"stub"] {
        assert!(
            !src.contains(marker),
            "src/lib.rs contains a stub marker: {marker}"
        );
    }
}

#[test]
fn d05_both_symbols_are_callable_through_dlsym() {
    // Symbol presence in `nm` is not the same as being reachable via dlsym with
    // default visibility; prove both are actually callable.
    let l = Libs::load();
    let c_out = capture_stdout(|| unsafe { (l.c_driver())(1, 2, 1, 3) });
    let r_out = capture_stdout(|| unsafe { (l.rust_driver())(1, 2, 1, 3) });
    assert_eq!(c_out, b"1 2 1 3\n");
    assert_eq!(c_out, r_out);

    let raw = foo_bytes(pack_byte0(1, 2, 1), [0, 0, 0], 3);
    let c_out = capture_stdout(|| unsafe { (l.c_print_foo())(raw.as_ptr()) });
    let r_out = capture_stdout(|| unsafe { (l.rust_print_foo())(raw.as_ptr()) });
    assert_eq!(c_out, b"1 2 1 3\n");
    assert_eq!(c_out, r_out);
}
