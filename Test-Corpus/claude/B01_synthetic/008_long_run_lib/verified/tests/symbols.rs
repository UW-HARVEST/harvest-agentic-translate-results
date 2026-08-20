// Phase D — symbol parity between the C `.so` and the Rust `.so`.
//
// Re-derives the table in SYMBOLS.md at test time with `nm -D`.

mod common;

use common::*;
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

/// name -> (nm type letter, size)
fn defined_symbols(so: &Path) -> BTreeMap<String, (char, u64)> {
    let out = Command::new("nm")
        .args(["-D", "-S", "--defined-only"])
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("cannot run nm: {e}"));
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let mut map = BTreeMap::new();
    for line in text.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        match f.len() {
            // <addr> <size> <type> <name>
            4 => {
                let size = u64::from_str_radix(f[1], 16).unwrap_or(0);
                map.insert(f[3].to_string(), (f[2].chars().next().unwrap(), size));
            }
            // <addr> <type> <name>  (no size)
            3 => {
                map.insert(f[2].to_string(), (f[1].chars().next().unwrap(), 0));
            }
            _ => {}
        }
    }
    map
}

fn undefined_symbols(so: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "-u"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect()
}

#[test]
fn c_and_rust_export_the_same_symbols() {
    let c_so = c_so_path();
    let rust_so = rust_so_path();
    let c = defined_symbols(&c_so);
    let r = defined_symbols(&rust_so);
    println!("C   ({}): {c:?}", c_so.display());
    println!("Rust({}): {r:?}", rust_so.display());

    assert!(
        !c.is_empty(),
        "no symbols parsed from {} — is nm working?",
        c_so.display()
    );

    // Every C symbol must exist in the Rust `.so` with the same name and the
    // same nm kind (T = text, B = .bss data).
    let mut missing = Vec::new();
    for (name, (kind, size)) in &c {
        match r.get(name) {
            None => missing.push(name.clone()),
            Some((rkind, rsize)) => {
                assert_eq!(
                    kind, rkind,
                    "symbol {name}: C kind {kind} vs Rust kind {rkind}"
                );
                if *kind == 'B' || *kind == 'D' {
                    assert_eq!(
                        size, rsize,
                        "data symbol {name}: C size 0x{size:x} vs Rust size 0x{rsize:x}"
                    );
                }
            }
        }
    }
    assert!(missing.is_empty(), "symbols missing from the Rust .so: {missing:?}");

    // The Rust `.so` must not leak anything beyond the C surface (apart from
    // possible compiler runtime symbols, which are all `_`-prefixed).
    let extra: Vec<&String> = r
        .keys()
        .filter(|k| !c.contains_key(*k) && !k.starts_with('_'))
        .collect();
    assert!(extra.is_empty(), "Rust .so exports non-C symbols: {extra:?}");

    // Exactly the three documented symbols.
    for expected in ["array", "long_exec", "perform_expensive_operations"] {
        assert!(c.contains_key(expected), "C .so lost {expected}");
        assert!(r.contains_key(expected), "Rust .so lost {expected}");
    }
}

#[test]
fn both_rust_profiles_match_the_c_surface() {
    let c = defined_symbols(&c_so_path());
    let mut checked = 0;
    for profile in ["debug", "release"] {
        let p = rust_so_path_for(profile);
        if !p.exists() {
            eprintln!("{} not built — skipping", p.display());
            continue;
        }
        let r = defined_symbols(&p);
        for (name, (kind, size)) in &c {
            let (rkind, rsize) = r
                .get(name)
                .unwrap_or_else(|| panic!("{profile}: missing symbol {name}"));
            assert_eq!(kind, rkind, "{profile}: {name} kind");
            if *kind == 'B' {
                assert_eq!(size, rsize, "{profile}: {name} size");
            }
        }
        checked += 1;
        println!("{profile}: symbol surface matches C ({} symbols)", c.len());
    }
    assert!(checked > 0, "no Rust .so found at all");
}

/// `readelf -sW` address of a defined symbol.
fn symbol_address(so: &Path, symbol: &str) -> u64 {
    let out = Command::new("readelf")
        .args(["-sW"])
        .arg(so)
        .output()
        .expect("run readelf");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() >= 8 && f.last() == Some(&symbol) && f[3] == "OBJECT" {
            return u64::from_str_radix(f[1], 16).expect("hex address");
        }
    }
    panic!("object symbol {symbol} not found in {}", so.display());
}

#[test]
fn array_object_alignment_matches() {
    // gcc aligns the 1 MiB `.bss` array to 32 bytes; a consumer of the exported
    // symbol can observe that (e.g. aligned vector loads), so the Rust object
    // must be at least as aligned.
    let c_addr = symbol_address(&c_so_path(), "array");
    assert_eq!(c_addr % 32, 0, "C array is not 32-byte aligned: {c_addr:#x}");
    for profile in ["debug", "release"] {
        let p = rust_so_path_for(profile);
        if !p.exists() {
            eprintln!("{} not built — skipping", p.display());
            continue;
        }
        let a = symbol_address(&p, "array");
        assert_eq!(
            a % 32,
            0,
            "{profile}: Rust array at {a:#x} is less aligned than the C object"
        );
        println!("{profile}: array at {a:#x} (32-byte aligned, like C at {c_addr:#x})");
    }
}

#[test]
fn rust_so_imports_libc_prng_and_printf() {
    // The C uses the *platform* PRNG and printf; the translation must import
    // the very same symbols, otherwise `long_exec`'s output could not be
    // byte-identical.
    let c = undefined_symbols(&c_so_path());
    let r = undefined_symbols(&rust_so_path());
    for sym in ["srand", "rand", "printf"] {
        let versioned = |v: &Vec<String>| v.iter().any(|s| s == sym || s.starts_with(&format!("{sym}@")));
        assert!(versioned(&c), "C .so should import {sym}");
        assert!(
            versioned(&r),
            "Rust .so must import libc {sym} (found: {r:?})"
        );
    }
}

#[test]
fn rust_so_has_no_unresolved_symbols() {
    // `ldd -r` performs data *and* function relocation checks and prints
    // "undefined symbol: X" for anything that cannot be resolved.
    for so in [c_so_path(), rust_so_path()] {
        let out = Command::new("ldd").arg("-r").arg(&so).output().expect("run ldd");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        println!("ldd -r {}:\n{text}", so.display());
        assert!(
            !text.contains("undefined symbol"),
            "{} has unresolved symbols:\n{text}",
            so.display()
        );
        assert!(
            !text.contains("not found"),
            "{} has missing dependencies:\n{text}",
            so.display()
        );
    }
}
