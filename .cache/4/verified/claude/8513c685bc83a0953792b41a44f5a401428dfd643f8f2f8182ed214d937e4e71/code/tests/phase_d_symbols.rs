// PHASE D — symbol parity, enforced as a test rather than a one-off inspection.
//
// Asserts that every symbol the C `.so` exports is also exported by the Rust
// `.so` under the exact same name, and that the Rust `.so` has no unresolvable
// (non-libc) undefined symbols.

mod common;
use common::*;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Exported (defined, global) symbol names, via `nm -D --defined-only --extern-only`.
fn exported_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--extern-only", so.to_str().unwrap()])
        .output()
        .expect("failed to run `nm` (binutils required for the symbol-parity test)");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            // "<addr> <type> <name>"; keep code/data definitions only.
            let mut it = l.split_whitespace();
            let _addr = it.next()?;
            let ty = it.next()?;
            let name = it.next()?;
            if matches!(ty, "T" | "t" | "D" | "B" | "R" | "W" | "i") {
                // Strip any @VERSION suffix.
                Some(name.split('@').next().unwrap().to_string())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn rust_so_exports_every_c_so_symbol() {
    let c = exported_symbols(&c_so_path());
    let r = exported_symbols(&rust_so_path());

    println!("C exports   ({}): {:?}", c.len(), c);
    println!("Rust exports ({}): {:?}", r.len(), r);

    // The C library must actually have been parsed; a silently empty set would
    // make this test vacuous.
    assert!(
        c.contains("driver") && c.contains("run"),
        "nm did not report the expected C exports; parsed set = {:?}",
        c
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "The Rust .so is MISSING {} symbol(s) exported by the C .so: {:?}\n\
         Each one means either a missing #[no_mangle] export wrapper or an \
         untranslated C module.",
        missing.len(),
        missing
    );
}

#[test]
fn both_libraries_fully_resolve_with_rtld_now() {
    // RTLD_NOW forces every undefined symbol to be bound at load time, so a
    // successful open proves there are 0 unresolved/missing non-libc symbols.
    // This is stronger and less brittle than allowlisting libc symbol names.
    const RTLD_NOW: i32 = 2;
    const RTLD_LOCAL: i32 = 0;
    for path in [c_so_path(), rust_so_path()] {
        unsafe {
            libloading::os::unix::Library::open(Some(&path), RTLD_NOW | RTLD_LOCAL)
                .unwrap_or_else(|e| {
                    panic!(
                        "{} has undefined symbols that cannot be resolved: {}",
                        path.display(),
                        e
                    )
                });
        }
    }
}

#[test]
fn c_static_functions_are_not_exported_by_either_library() {
    // These are `static` in driver.c, so neither library may export them.
    let c = exported_symbols(&c_so_path());
    let r = exported_symbols(&rust_so_path());
    for name in [
        "add_floor",
        "add_bedrooms",
        "add_floor_to_the_house",
        "print_the_house",
        "parse_val",
        "the_house",
    ] {
        assert!(!c.contains(name), "C unexpectedly exports `{}`", name);
        assert!(
            !r.contains(name),
            "Rust exports `{}`, but it has internal linkage in C",
            name
        );
    }
}
