//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Every symbol the C shared object exports must also be exported by the Rust
//! shared object under the exact same name. The diff must be empty.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

/// Run `nm -D --defined-only` and return the exported symbol names.
fn exported_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("failed to run `nm` (binutils required)");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            // "<addr> <type> <name>" — weak/undefined entries have no address.
            let mut it = line.split_whitespace();
            let (_addr, ty, name) = (it.next()?, it.next()?, it.next()?);
            // Only globally visible definitions (T/D/B/R/W and friends).
            if ty.chars().all(|c| c.is_ascii_uppercase()) {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Symbols that belong to the platform runtime rather than the library's API.
fn is_runtime_symbol(name: &str) -> bool {
    name.starts_with("_ITM_")
        || name.starts_with("__cxa_")
        || name.starts_with("__gmon_")
        || name.starts_with("_Unwind_")
        || name.starts_with("_init")
        || name.starts_with("_fini")
        || name.starts_with("__rust_")
        || name.starts_with("rust_")
        || name.starts_with("_ZN")
        || name.starts_with("_R")
}

#[test]
fn phase_d_rust_so_exports_every_c_symbol() {
    let c_so = c_so_path();
    let rust_so = rust_so_path();

    let c_syms: BTreeSet<String> = exported_symbols(&c_so)
        .into_iter()
        .filter(|s| !is_runtime_symbol(s))
        .collect();
    let rust_syms = exported_symbols(&rust_so);

    eprintln!("C    .so {} exports {} API symbol(s):", c_so.display(), c_syms.len());
    for s in &c_syms {
        eprintln!("    {s}");
    }

    assert!(
        !c_syms.is_empty(),
        "no API symbols found in the C .so — the harness is misreading `nm` output"
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !rust_syms.contains(*s)).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is MISSING {} symbol(s) exported by the C .so: {:?}\n\
         Either add the #[no_mangle] extern \"C\" wrapper, or translate the C \
         source that was skipped.",
        missing.len(),
        missing
    );

    eprintln!("symbol diff (C -> Rust) is EMPTY: all {} present", c_syms.len());
}

/// The `static` C helpers must NOT be exported by either library.
#[test]
fn phase_d_static_helpers_are_not_exported() {
    let c_syms = exported_symbols(&c_so_path());
    let rust_syms = exported_symbols(&rust_so_path());

    for helper in ["Protanopia", "Deuteranopia", "Tritanopia"] {
        assert!(
            !c_syms.contains(helper),
            "`{helper}` is `static` in C and must not be an exported symbol"
        );
        assert!(
            !rust_syms.contains(helper),
            "`{helper}` must stay private in Rust to match the C .so's symbol table"
        );
    }
    eprintln!("the three `static` kernels are private in both libraries");
}

/// The public entry point really is resolvable by name from both `.so`s.
#[test]
fn phase_d_colourblind_is_resolvable_in_both() {
    assert!(exported_symbols(&c_so_path()).contains("colourblind"));
    assert!(exported_symbols(&rust_so_path()).contains("colourblind"));
    // And it actually works through both handles.
    let input = [0.2f32, 0.4f32, 0.6f32];
    for &imp in &VALID_IMPAIRMENTS {
        let c = c_lib().call(imp, input);
        let r = rust_lib().call(imp, input);
        assert!(bits_eq(c, r), "Impairment={imp}: {} vs {}", fmt3(c), fmt3(r));
    }
}
