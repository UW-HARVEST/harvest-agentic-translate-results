// Phase D — symbol parity between the C `.so` and the Rust `.so`.
//
// Every dynamic symbol the C library exports must be exported by the Rust
// library under the exact same name, and the Rust library must have zero
// unresolved (non-libc) imports.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(path: &Path, extra: &str) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg(extra)
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm -D {extra} {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .map(|s| s.split('@').next().unwrap_or(s).to_string())
        .collect()
}

fn defined(path: &Path) -> BTreeSet<String> {
    nm(path, "--defined-only")
}

/// Every C export must also be a Rust export, with the identical name.
#[test]
fn c_exports_are_subset_of_rust_exports() {
    let c_path = common::c_so_path();
    let rust_path = common::rust_so_path();
    assert!(c_path.exists(), "missing C .so at {}", c_path.display());
    assert!(
        rust_path.exists(),
        "missing Rust .so at {}",
        rust_path.display()
    );

    let c_syms = defined(&c_path);
    let rust_syms = defined(&rust_path);

    // Sanity: we really did parse the C library.
    for expected in [
        "convert_double_to_int",
        "find_value_in_buffer",
        "process_negation",
        "create_numeric_buffer",
        "calculate_with_doubles",
        "doubleneg",
    ] {
        assert!(
            c_syms.contains(expected),
            "expected C .so to export {expected}; parsed C symbols: {c_syms:?}"
        );
    }

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         Either add the #[no_mangle] extern \"C\" wrapper, or translate the \
         missing C source.",
        missing.len()
    );
}

/// The Rust `.so` must load with `RTLD_NOW`, which forces the dynamic linker to
/// bind *every* undefined symbol immediately.  If any non-libc symbol were
/// missing, this `dlopen` would fail.
#[test]
fn rust_so_has_no_unresolved_symbols() {
    use libloading::os::unix::{Library, RTLD_LOCAL, RTLD_NOW};

    let rust_path = common::rust_so_path();
    let lib = unsafe { Library::open(Some(&rust_path), RTLD_NOW | RTLD_LOCAL) }.unwrap_or_else(
        |e| {
            panic!(
                "RTLD_NOW dlopen of {} failed, i.e. it has unresolved symbols: {e}",
                rust_path.display()
            )
        },
    );
    drop(lib);

    let c_path = common::c_so_path();
    let lib = unsafe { Library::open(Some(&c_path), RTLD_NOW | RTLD_LOCAL) }
        .unwrap_or_else(|e| panic!("RTLD_NOW dlopen of the C .so failed: {e}"));
    drop(lib);
}

/// Undefined symbols in the Rust `.so` must all come from the platform runtime
/// (libc / libm / libgcc unwinder) — never from an untranslated module of the
/// library itself.
#[test]
fn rust_undefined_symbols_are_platform_only() {
    let rust_path = common::rust_so_path();
    let undef = nm(&rust_path, "--undefined-only");
    let c_defined = defined(&common::c_so_path());

    let self_referential: Vec<&String> = undef.intersection(&c_defined).collect();
    assert!(
        self_referential.is_empty(),
        "Rust .so imports symbols that the library itself should define \
         (untranslated module?): {self_referential:?}"
    );

    let rust_mangled: Vec<&String> = undef.iter().filter(|s| s.starts_with("_ZN")).collect();
    assert!(
        rust_mangled.is_empty(),
        "Rust .so has unresolved Rust-mangled imports: {rust_mangled:?}"
    );
}

/// The two libraries can be loaded side by side (`RTLD_LOCAL`) without their
/// identically-named exports colliding — the precondition for every other test.
#[test]
fn both_libraries_resolve_all_six_symbols() {
    let (c, rs) = common::apis();
    assert_eq!(c.name, "C");
    assert_eq!(rs.name, "Rust");
    // Distinct code addresses prove we are not accidentally calling one library
    // twice through a global-scope symbol clash.
    assert_ne!(
        c.doubleneg as usize, rs.doubleneg as usize,
        "C and Rust `doubleneg` resolved to the same address"
    );
    assert_ne!(
        c.convert_double_to_int as usize, rs.convert_double_to_int as usize,
        "C and Rust `convert_double_to_int` resolved to the same address"
    );
    assert_ne!(c.find_value_in_buffer as usize, rs.find_value_in_buffer as usize);
    assert_ne!(c.process_negation as usize, rs.process_negation as usize);
    assert_ne!(c.create_numeric_buffer as usize, rs.create_numeric_buffer as usize);
    assert_ne!(c.calculate_with_doubles as usize, rs.calculate_with_doubles as usize);
}
