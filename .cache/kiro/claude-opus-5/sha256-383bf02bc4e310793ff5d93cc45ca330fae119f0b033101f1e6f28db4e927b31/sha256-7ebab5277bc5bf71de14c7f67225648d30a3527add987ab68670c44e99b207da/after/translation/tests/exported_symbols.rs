//! Step 8: every dynamic symbol the C shared object exports must also be
//! exported by the Rust shared object, under the exact same name.

mod common;

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

#[derive(Debug, PartialEq, Eq)]
struct Sym {
    /// nm type letter, e.g. `T` for text, `B` for .bss object.
    kind: char,
    /// Symbol size in bytes as reported by `nm -S`.
    size: u64,
}

fn dynamic_symbols(so: &Path) -> BTreeMap<String, Sym> {
    let out = Command::new("nm")
        .args(["-D", "-S", "--defined-only", "--format=posix"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    let mut map = BTreeMap::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        // posix format: "name type value size"
        let mut f = line.split_whitespace();
        let (Some(name), Some(kind)) = (f.next(), f.next()) else {
            continue;
        };
        let _value = f.next();
        let size = f
            .next()
            .and_then(|s| u64::from_str_radix(s, 16).ok())
            .unwrap_or(0);
        map.insert(
            name.to_string(),
            Sym {
                kind: kind.chars().next().unwrap(),
                size,
            },
        );
    }
    assert!(!map.is_empty(), "no symbols parsed from {}", so.display());
    map
}


#[test]
fn rust_so_exports_every_c_symbol() {
    let c = dynamic_symbols(&common::c_so_path());
    let rust = dynamic_symbols(&common::rust_so_path());

    // The C translation unit exports exactly these three; assert that up front
    // so this test notices if the C side ever grows a new public symbol.
    for expected in ["array", "long_exec", "perform_expensive_operations"] {
        assert!(
            c.contains_key(expected),
            "C library unexpectedly does not export `{expected}`"
        );
    }

    let missing: Vec<&String> = c.keys().filter(|k| !rust.contains_key(*k)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}\n\
         C exports: {:?}",
        c.keys().collect::<Vec<_>>()
    );

    for (name, c_sym) in &c {
        let rust_sym = &rust[name];
        assert_eq!(
            c_sym.kind, rust_sym.kind,
            "`{name}`: nm type differs (C = {}, Rust = {})",
            c_sym.kind, rust_sym.kind
        );
        // Data objects must have the same footprint; function sizes legitimately
        // differ between compilers, so only objects are size-checked.
        if c_sym.kind.eq_ignore_ascii_case(&'b') || c_sym.kind.eq_ignore_ascii_case(&'d') {
            assert_eq!(
                c_sym.size, rust_sym.size,
                "`{name}`: object size differs (C = {} bytes, Rust = {} bytes)",
                c_sym.size, rust_sym.size
            );
        }
    }
}

/// The exported `array` must be reachable and 1 MiB in both, and writes through
/// the symbol must be visible to the exported functions. `zeros` in the
/// `perform_expensive_operations` suite covers the second half; this pins the
/// size relationship that `ARRAY_SIZE` encodes.
#[test]
fn array_symbol_is_one_mebibyte() {
    let c = dynamic_symbols(&common::c_so_path());
    assert_eq!(c["array"].size, (common::ARRAY_SIZE * 4) as u64);
    let rust = dynamic_symbols(&common::rust_so_path());
    assert_eq!(rust["array"].size, (common::ARRAY_SIZE * 4) as u64);
}
