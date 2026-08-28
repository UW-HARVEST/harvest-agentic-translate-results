//! Checks that the Rust `.so` exports every dynamic symbol the C `.so` does.

mod common;

use common::*;

fn defined_dynamic_symbols(path: &std::path::Path) -> Vec<String> {
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", path.display());
    let mut syms: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut parts = l.split_whitespace();
            let (a, b) = (parts.next()?, parts.next()?);
            let (kind, name) = match parts.next() {
                Some(n) => (b, n),
                None => (a, b),
            };
            // Only globally visible code/data symbols.
            if matches!(kind, "T" | "D" | "B" | "R" | "W" | "G" | "S") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect();
    syms.sort();
    syms.dedup();
    syms
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let c_so = std::fs::read_dir(root.join("c_src/build"))
        .expect("c_src/build exists")
        .flatten()
        .map(|e| e.path())
        .find(|p| p.extension().and_then(|s| s.to_str()) == Some("so"))
        .expect("C .so built");

    let c_syms = defined_dynamic_symbols(&c_so);
    assert!(!c_syms.is_empty(), "nm found no symbols in the C .so");

    // Resolve each through libloading on the Rust library: the authoritative
    // check that an external caller can actually reach the symbol.
    let l = libs();
    let mut missing = Vec::new();
    for s in &c_syms {
        let ok = unsafe {
            l.rust
                .get::<*const ()>(format!("{s}\0").as_bytes())
                .is_ok()
        };
        if !ok {
            missing.push(s.clone());
        }
    }
    assert!(
        missing.is_empty(),
        "Rust .so is missing exports present in the C .so: {missing:?}\nC symbols: {c_syms:?}"
    );
}

#[test]
fn expected_symbols_resolve_in_both() {
    let l = libs();
    for s in EXPORTED_SYMBOLS {
        unsafe {
            l.c.get::<*const ()>(format!("{s}\0").as_bytes())
                .unwrap_or_else(|e| panic!("C .so missing {s}: {e}"));
            l.rust
                .get::<*const ()>(format!("{s}\0").as_bytes())
                .unwrap_or_else(|e| panic!("Rust .so missing {s}: {e}"));
        }
    }
}
