//! Verifies that the Rust `.so` exports every dynamic symbol the C `.so` does.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Defined dynamic symbols reported by `nm -D --defined-only`.
fn exported_symbols(so: &PathBuf) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("nm not available");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let a = it.next()?;
            let b = it.next()?;
            // "<addr> <type> <name>" for defined symbols.
            let (ty, name) = match it.next() {
                Some(n) => (b, n),
                None => (a, b),
            };
            // Code and data symbols only; ignore weak/indirect runtime glue.
            if matches!(ty, "T" | "D" | "B" | "R") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn rust_exports_superset_of_c() {
    let c_so = root().join("c_src/build/libdriver.so");
    assert!(c_so.exists(), "C library not built at {}", c_so.display());
    let c_syms = exported_symbols(&c_so);

    // The C library exports exactly the public API from the two headers plus
    // the internal-but-non-static allocate_matrix.
    let expected: BTreeSet<String> = [
        "allocate_matrix",
        "free_matrix",
        "initialize_matrix_from_string",
        "multiply_matrices",
        "matrix_to_string",
        "write_to_file",
        "driver",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    assert_eq!(
        c_syms, expected,
        "the C library's export set changed; update this test"
    );

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut checked = 0;
    for profile in ["debug", "release"] {
        let rust_so = manifest.join("target").join(profile).join("libdriver.so");
        if !rust_so.exists() {
            continue;
        }
        checked += 1;
        let r_syms = exported_symbols(&rust_so);
        let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
        assert!(
            missing.is_empty(),
            "{profile}: Rust .so is missing symbols exported by the C .so: {missing:?}"
        );
    }
    assert!(checked > 0, "no Rust cdylib found to inspect");
}

#[test]
fn rust_symbols_are_unmangled_functions() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rust_so = ["debug", "release"]
        .iter()
        .map(|p| manifest.join("target").join(p).join("libdriver.so"))
        .find(|p| p.exists())
        .expect("no Rust cdylib found");

    let syms = exported_symbols(&rust_so);
    for name in [
        "allocate_matrix",
        "free_matrix",
        "initialize_matrix_from_string",
        "multiply_matrices",
        "matrix_to_string",
        "write_to_file",
        "driver",
    ] {
        assert!(
            syms.contains(name),
            "Rust .so does not export {name}; exports: {syms:?}"
        );
        // No Rust name mangling leaked through.
        assert!(
            !syms.iter().any(|s| s.starts_with("_ZN") && s.contains(name)),
            "found a mangled variant of {name}"
        );
    }
}
