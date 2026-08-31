//! Verifies that the Rust shared object exports every dynamic symbol the C
//! shared object does, with identical names.

mod common;

use std::path::PathBuf;
use std::process::Command;

fn exported_symbols(so: &PathBuf) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {so:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let mut syms: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let mut it = line.split_whitespace();
            let name = it.next()?;
            let kind = it.next()?;
            // Only global text/data symbols, matching what a caller can bind to.
            if matches!(kind, "T" | "D" | "B" | "R" | "W" | "V" | "i" | "G" | "S") {
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
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let c_so = root.join("c_src/build/libdriver.so");

    // Locate the Rust cdylib next to the test binary.
    let exe = std::env::current_exe().unwrap();
    let mut rust_so = exe.parent().unwrap().parent().unwrap().join("libdriver.so");
    if !rust_so.exists() {
        for profile in ["debug", "release"] {
            let alt = root
                .join("translation/target")
                .join(profile)
                .join("libdriver.so");
            if alt.exists() {
                rust_so = alt;
                break;
            }
        }
    }
    assert!(c_so.exists(), "missing {c_so:?}");
    assert!(rust_so.exists(), "missing {rust_so:?}");

    let c_syms = exported_symbols(&c_so);
    let rust_syms = exported_symbols(&rust_so);

    let missing: Vec<&String> = c_syms.iter().filter(|s| !rust_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}\nC: {c_syms:?}\nRust: {rust_syms:?}"
    );

    // The functions declared in driver.h plus the non-static `run` must be there.
    for required in ["driver", "run"] {
        assert!(
            rust_syms.iter().any(|s| s == required),
            "Rust .so does not export `{required}`; has {rust_syms:?}"
        );
    }
}
