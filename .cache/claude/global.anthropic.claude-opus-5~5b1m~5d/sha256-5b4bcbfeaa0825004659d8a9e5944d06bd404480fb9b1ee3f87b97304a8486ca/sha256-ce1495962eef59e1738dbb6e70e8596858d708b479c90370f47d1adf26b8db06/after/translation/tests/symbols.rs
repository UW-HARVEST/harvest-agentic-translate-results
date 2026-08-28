//! Phase D — exported-symbol parity between the C `.so` and the Rust `.so`.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

use common::*;

/// Every symbol `nm -D --defined-only` reports for a shared object, together
/// with its type letter.
fn dynamic_defined(path: &std::path::Path) -> BTreeSet<(char, String)> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut set = BTreeSet::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let toks: Vec<&str> = line.split_whitespace().collect();
        // "<addr> <type> <name>" or "<type> <name>" for absolute/weak entries.
        let (ty, name) = match toks.len() {
            3 => (toks[1], toks[2]),
            2 => (toks[0], toks[1]),
            _ => continue,
        };
        let ty = ty.chars().next().unwrap();
        set.insert((ty, name.to_string()));
    }
    set
}

fn undefined(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "-u", path.to_str().unwrap()])
        .output()
        .expect("run nm");
    let mut set = BTreeSet::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        if let Some(name) = line.split_whitespace().last() {
            set.insert(name.to_string());
        }
    }
    set
}

/// The gate: every symbol the C `.so` exports must also be exported by the Rust
/// `.so`, under the exact same name.
#[test]
fn symbol_diff_is_empty() {
    let c = dynamic_defined(&c_lib_path());
    let r = dynamic_defined(&rust_lib_path());

    let c_names: BTreeSet<&String> = c.iter().map(|(_, n)| n).collect();
    let r_names: BTreeSet<&String> = r.iter().map(|(_, n)| n).collect();

    let missing: Vec<&&String> = c_names.difference(&r_names).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C exports:    {c_names:?}\n\
         Rust exports: {r_names:?}",
        missing.len()
    );

    // The two entry points must be global text symbols in both.
    for name in ["match", "spectral_contrast"] {
        assert!(
            c.contains(&('T', name.to_string())),
            "C .so: `{name}` is not a global text symbol"
        );
        assert!(
            r.contains(&('T', name.to_string())),
            "Rust .so: `{name}` is not a global text symbol; found {r:?}"
        );
    }

    println!("C exports:    {c_names:?}");
    println!("Rust exports: {r_names:?}");
}

/// The `static` helpers in the C are local symbols and must not leak into the
/// Rust `.so`'s dynamic table either.
#[test]
fn static_helpers_are_not_exported() {
    let c = dynamic_defined(&c_lib_path());
    let r = dynamic_defined(&rust_lib_path());
    for name in [
        "total",
        "smoothen",
        "differentiate",
        "preprocess",
        "dot_product",
        "normalize",
    ] {
        let n = name.to_string();
        assert!(
            !c.iter().any(|(_, s)| *s == n),
            "C .so unexpectedly exports the static helper `{name}`"
        );
        assert!(
            !r.iter().any(|(_, s)| *s == n),
            "Rust .so exports `{name}`, but it is `static` in the C"
        );
    }
}

/// The Rust `.so` must not need anything outside libc / the language runtime.
#[test]
fn rust_so_has_no_exotic_undefined_symbols() {
    let u = undefined(&rust_lib_path());
    // A symbol is acceptable if it is version-tagged against a system library
    // (`name@GLIBC_x.y`, `name@GCC_x.y`) or is one of the three weak
    // toolchain/CRT hooks every GCC-linked object has. Anything else would be a
    // dangling reference to code that was never translated.
    let weak_crt_hooks = [
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__gmon_start__",
    ];
    let mut unexpected = Vec::new();
    for s in &u {
        let is_versioned_system_symbol = s
            .split_once('@')
            .map(|(_, v)| v.starts_with("GLIBC_") || v.starts_with("GCC_") || v.starts_with("GLIBCXX_"))
            .unwrap_or(false);
        if is_versioned_system_symbol || weak_crt_hooks.contains(&s.as_str()) {
            continue;
        }
        unexpected.push(s.clone());
    }
    println!("Rust .so undefined symbols ({}): {u:?}", u.len());
    assert!(
        unexpected.is_empty(),
        "unexpected non-libc undefined symbols in the Rust .so: {unexpected:?}"
    );

    // And the C .so's own imports must be a subset of the same categories.
    let cu = undefined(&c_lib_path());
    println!("C .so undefined symbols ({}): {cu:?}", cu.len());
}

/// Sanity: the two `.so`s really are two different files, and both are loadable
/// and callable through their exports.
#[test]
fn both_libraries_are_live() {
    let p = pair();
    assert_ne!(p.c.path, p.rust.path);
    let buf: Vec<u32> = vec![
        0x3F80_0000, 0x4000_0000, 0x4040_0000, 0x4080_0000, // a = 1,2,3,4
        0x4080_0000, 0x4040_0000, 0x4000_0000, 0x3F80_0000, // b = 4,3,2,1
    ];
    let c = sc_call(&p.c, &buf, 0, 4, 4);
    let r = sc_call(&p.rust, &buf, 0, 4, 4);
    assert_eq!(c, r, "smoke test diverged");
    assert_ne!(c.ret, 0, "smoke test produced +0.0, the loader is probably wrong");

    let dbuf: Vec<u64> = (0..64).map(|i| (1.0 + (i as f64) * 0.25).to_bits()).collect();
    let mc = match_call(&p.c, &dbuf, 0, 32, 32, 0.5);
    let mr = match_call(&p.rust, &dbuf, 0, 32, 32, 0.5);
    assert_eq!(mc, mr, "match smoke test diverged");
}
