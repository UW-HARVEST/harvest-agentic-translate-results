//! The Rust `.so` must export every dynamic symbol the C `.so` exports, with
//! the same name and the same symbol kind (function vs. object).
mod common;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;

fn c_so() -> PathBuf {
    if let Ok(p) = std::env::var("LZ4_C_SO") {
        return PathBuf::from(p);
    }
    let cwd = std::env::current_dir().unwrap();
    for c in [
        cwd.join("../c_src/build/liblz4.so"),
        cwd.join("c_src/build/liblz4.so"),
    ] {
        if c.exists() {
            return c;
        }
    }
    panic!("C liblz4.so not found");
}

fn rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("LZ4_RUST_SO") {
        return PathBuf::from(p);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(d) = exe.parent().and_then(|p| p.parent()) {
            let c = d.join("liblz4.so");
            if c.exists() {
                return c;
            }
        }
    }
    let cwd = std::env::current_dir().unwrap();
    for c in [
        cwd.join("target/release/liblz4.so"),
        cwd.join("target/debug/liblz4.so"),
    ] {
        if c.exists() {
            return c;
        }
    }
    panic!("Rust liblz4.so not found; run `cargo build --release`");
}

/// name -> symbol type letter, for defined dynamic symbols only.
fn dynamic_symbols(path: &PathBuf) -> BTreeMap<String, char> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("failed to run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut map = BTreeMap::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let (_addr, ty, name) = match (it.next(), it.next(), it.next()) {
            (Some(a), Some(t), Some(n)) => (a, t, n),
            _ => continue,
        };
        let ty = ty.chars().next().unwrap();
        // T/t: text (function). D/d, B/b, R/r: data objects. W: weak.
        if !matches!(ty, 'T' | 'D' | 'B' | 'R' | 'W') {
            continue;
        }
        // Ignore symbols the toolchain injects rather than the source declaring.
        if name.starts_with("_ZN")
            || name.starts_with("__rust")
            || name.starts_with("rust_")
            || name.starts_with("_init")
            || name.starts_with("_fini")
        {
            continue;
        }
        map.insert(name.to_string(), ty);
    }
    map
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c = dynamic_symbols(&c_so());
    let r = dynamic_symbols(&rust_so());
    assert!(!c.is_empty(), "no symbols found in the C .so");

    let missing: Vec<&String> = c.keys().filter(|k| !r.contains_key(*k)).collect();
    assert!(
        missing.is_empty(),
        "{} symbol(s) exported by the C .so but missing from the Rust .so: {:?}",
        missing.len(),
        missing
    );

    // Same kind: a function in C must be a function in Rust, not a data object.
    let mut kind_mismatch = Vec::new();
    for (name, cty) in &c {
        let rty = r[name];
        let cls = |t: char| if matches!(t, 'T' | 't' | 'W') { 'F' } else { 'O' };
        if cls(*cty) != cls(rty) {
            kind_mismatch.push(format!("{}: C={} Rust={}", name, cty, rty));
        }
    }
    assert!(
        kind_mismatch.is_empty(),
        "symbol kind mismatches: {:?}",
        kind_mismatch
    );

    eprintln!(
        "{} C symbols all present in the Rust .so ({} exported there in total)",
        c.len(),
        r.len()
    );
}

/// Every C symbol must also be resolvable via `dlsym` on the Rust library, which
/// is what an external caller actually does.
#[test]
fn every_c_symbol_is_dlsym_resolvable_in_rust() {
    let c = dynamic_symbols(&c_so());
    let libs = common::libs();
    let mut failures = Vec::new();
    for name in c.keys() {
        let res: Result<libloading::Symbol<'_, *const ()>, _> =
            unsafe { libs.r.get(name.as_bytes()) };
        if res.is_err() {
            failures.push(name.clone());
        }
    }
    assert!(
        failures.is_empty(),
        "not resolvable via dlsym in the Rust .so: {:?}",
        failures
    );
}
