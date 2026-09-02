//! Phase D — symbol parity, enforced as a test rather than only in a document.
//!
//! Every dynamic symbol the C `.so` defines must also be resolvable in the Rust
//! `.so` under the exact same name. This runs `nm -D --defined-only` on both
//! libraries and additionally proves each name is `dlsym`-resolvable.

mod common;

use common::*;
use std::path::PathBuf;
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn defined_symbols(so: &PathBuf) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
        .collect();
    v.sort();
    v.dedup();
    v
}

fn rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().unwrap();
    for dir in exe.ancestors().skip(1) {
        let c = dir.join("libdriver.so");
        if c.exists() {
            return c;
        }
    }
    panic!("Rust cdylib not found; run `cargo build` first");
}

#[test]
fn phase_d_symbol_parity_is_exact() {
    let c_so = root().join("c_src/build/libdriver.so");
    assert!(c_so.exists(), "build the C library first: {}", c_so.display());
    let c_syms = defined_symbols(&c_so);
    let r_syms = defined_symbols(&rust_so());

    assert!(
        !c_syms.is_empty(),
        "nm reported no exported symbols for the C library"
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !r_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} C symbol(s): {missing:?}\n  C: {c_syms:?}\n  Rust: {r_syms:?}",
        missing.len()
    );
}

/// Every C-exported name must also be reachable via `dlsym` on the Rust library,
/// which is what an external consumer actually does.
#[test]
fn phase_d_every_c_symbol_is_dlsym_resolvable_in_rust() {
    // `libs()` resolves all five names through dlsym on both libraries and
    // panics if any lookup fails.
    let l = libs();
    assert_eq!(l.c.name, "C");
    assert_eq!(l.rust.name, "Rust");

    let c_so = root().join("c_src/build/libdriver.so");
    let expected = ["bad", "driver", "good", "printIntLine", "printLine"];
    let mut c_syms = defined_symbols(&c_so);
    c_syms.sort();
    assert_eq!(
        c_syms, expected,
        "the C export set changed; update SYMBOLS.md and the harness"
    );
}

/// Neither library may import a non-libc symbol that the other cannot satisfy.
#[test]
fn phase_d_no_unresolved_non_libc_imports() {
    let so = rust_so();
    let out = Command::new("ldd").arg(&so).output().expect("run ldd");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        !text.contains("not found"),
        "Rust .so has unresolved shared-library dependencies:\n{text}"
    );
}
