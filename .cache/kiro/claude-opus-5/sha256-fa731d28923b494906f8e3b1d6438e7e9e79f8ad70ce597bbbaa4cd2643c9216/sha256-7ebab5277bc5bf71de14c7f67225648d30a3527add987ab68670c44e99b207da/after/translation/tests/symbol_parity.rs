//! Export parity: every symbol the C shared object exposes in its dynamic
//! symbol table must also be exposed, under the exact same name, by the Rust
//! shared object.
//!
//! This is the automated form of comparing `nm -D` on the two `.so` files.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Symbols every ELF shared object gets from the toolchain / libc rather than
/// from the translated source. They are not part of the API surface being
/// verified, and the C and Rust toolchains legitimately differ here.
const TOOLCHAIN_SYMBOLS: &[&str] = &[
    "_init",
    "_fini",
    "__bss_start",
    "_edata",
    "_end",
    "__gmon_start__",
    "_ITM_deregisterTMCloneTable",
    "_ITM_registerTMCloneTable",
    "__cxa_finalize",
    "__gxx_personality_v0",
    "_Unwind_Resume",
];

/// Run `nm -D --defined-only` and return the set of defined dynamic symbol names.
fn defined_dynamic_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("failed to run nm - is binutils installed?");
    assert!(
        out.status.success(),
        "nm -D {} failed: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            // "<addr> <type> <name>" or "         <type> <name>"
            let mut it = line.split_whitespace().rev();
            let name = it.next()?;
            let kind = it.next()?;
            // Skip anything that is not a real definition of code or data.
            if kind.len() != 1 {
                return None;
            }
            Some(name.to_string())
        })
        .filter(|n| !TOOLCHAIN_SYMBOLS.contains(&n.as_str()))
        .collect()
}

#[test]
fn rust_so_exports_every_symbol_the_c_so_exports() {
    let c_so = common::c_so_path();
    let rust_so = common::rust_so_path();

    let c_syms = defined_dynamic_symbols(&c_so);
    let rust_syms = defined_dynamic_symbols(&rust_so);

    println!("C   ({}): {:?}", c_so.display(), c_syms);
    println!("Rust({}): {} symbols", rust_so.display(), rust_syms.len());

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}",
        missing.len()
    );

    // Sanity: the API function really is in both sets, so the comparison above
    // is not vacuously true because of a parsing bug.
    assert!(
        c_syms.contains("hdr_bitrate"),
        "C .so does not export hdr_bitrate - nm parsing is wrong: {c_syms:?}"
    );
    assert!(rust_syms.contains("hdr_bitrate"));
}

/// The public header declares exactly one function; make sure the C library
/// does not export additional API symbols that the header hides (which would
/// widen the surface the translation has to cover).
#[test]
fn c_api_surface_is_only_hdr_bitrate() {
    let c_syms = defined_dynamic_symbols(&common::c_so_path());
    let api: Vec<&String> = c_syms.iter().filter(|s| !s.starts_with('_')).collect();
    assert_eq!(
        api,
        vec![&"hdr_bitrate".to_string()],
        "unexpected C API symbols: {api:?}"
    );
}
