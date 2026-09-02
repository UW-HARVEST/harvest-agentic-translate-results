//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Recomputes the `nm -D` diff recorded in `SYMBOLS.md` at test time so the
//! artifact cannot silently drift.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

fn nm_defined(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (a, b, c) = (it.next(), it.next(), it.next());
            match (a, b, c) {
                // "<addr> <type> <name>"
                (Some(_), Some(t), Some(name)) if t.len() == 1 => Some((t.to_string(), name.to_string())),
                // "<type> <name>" (undefined-address form)
                (Some(t), Some(name), None) if t.len() == 1 => Some((t.to_string(), name.to_string())),
                _ => None,
            }
        })
        // keep only real global definitions; drop linker/CRT synthetics
        .filter(|(t, name)| {
            matches!(t.as_str(), "T" | "D" | "B" | "R" | "W" | "V" | "G" | "S" | "i")
                && !name.starts_with("_init")
                && !name.starts_with("_fini")
                && !name.starts_with("__bss_start")
                && !name.starts_with("_edata")
                && !name.starts_with("_end")
        })
        .map(|(_, name)| name)
        .collect()
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c = nm_defined(c_so_path());
    let r = nm_defined(rust_so_path());

    assert!(
        c.contains("tfm"),
        "sanity: C .so must export tfm, got {c:?}"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {:?}\n\
         C  = {:?}\n\
         Rust (subset) = {:?}",
        missing.len(),
        missing,
        c,
        r.iter().take(20).collect::<Vec<_>>()
    );
}

#[test]
fn rust_so_has_no_missing_non_libc_undefined_symbols() {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(rust_so_path())
        .output()
        .expect("run nm");
    let text = String::from_utf8_lossy(&out.stdout);

    // Every undefined symbol must be satisfiable from the platform: libc,
    // libgcc's unwinder, or a linker-synthetic weak symbol.
    let allowed_prefixes = [
        "_Unwind_",
        "_ITM_",
        "__cxa_",
        "__gmon_start__",
        "__tls_get_addr",
        "__errno_location",
        "__libc_",
        "_dl_",
    ];
    let mut unexpected = Vec::new();
    for line in text.lines() {
        let name = match line.split_whitespace().last() {
            Some(n) => n.split('@').next().unwrap().to_string(),
            None => continue,
        };
        if name.is_empty() || name.len() == 1 {
            continue;
        }
        if allowed_prefixes.iter().any(|p| name.starts_with(p)) {
            continue;
        }
        // Confirm it resolves in the process (libc / libm / libpthread).
        let resolved = unsafe {
            let lib = libloading::os::unix::Library::this();
            let mut c = name.clone().into_bytes();
            c.push(0);
            lib.get::<*const ()>(&c).is_ok()
        };
        if !resolved {
            unexpected.push(name);
        }
    }
    assert!(
        unexpected.is_empty(),
        "Rust .so has unresolvable non-libc undefined symbols: {unexpected:?}"
    );
}

#[test]
fn both_libraries_load_and_resolve_tfm() {
    // Proves the whole harness really goes through dlopen/dlsym for both sides.
    let c = c_tfm();
    let r = rust_tfm();
    assert_ne!(c as usize, 0);
    assert_ne!(r as usize, 0);
    assert_ne!(
        c as usize, r as usize,
        "C and Rust tfm resolved to the same address — one .so shadowed the other"
    );

    let s = [1.0f32, 2.0, 3.0];
    let mut dc = poison(2);
    let mut dr = poison(2);
    unsafe {
        c(dc.as_mut_ptr(), s.as_ptr(), 1);
        r(dr.as_mut_ptr(), s.as_ptr(), 1);
    }
    assert_bits_eq("smoke", &s, 1, &dc, &dr);
}
