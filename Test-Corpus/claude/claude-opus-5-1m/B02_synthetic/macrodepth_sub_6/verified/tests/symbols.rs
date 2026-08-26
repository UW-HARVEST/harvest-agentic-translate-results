//! Phase A / Phase D — exported-symbol parity between the C `.so` and the Rust
//! `cdylib`, re-checked for whichever configuration this binary was built with.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// `nm -D --defined-only` → set of dynamic symbol names.
fn defined_syms(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {so:?}: {}",
        show(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
        .collect()
}

/// `nm -D --defined-only` → (name, type-letter) pairs.
fn defined_syms_typed(so: &Path) -> BTreeSet<(String, String)> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let _addr = it.next()?;
            let ty = it.next()?.to_string();
            let name = it.next()?.to_string();
            Some((name, ty))
        })
        .collect()
}

/// Symbols the Rust runtime adds; they are additions, never omissions.
fn is_rust_internal(s: &str) -> bool {
    s.starts_with("_ZN")
        || s.starts_with("_ZS")
        || s.starts_with("__rust")
        || s.starts_with("rust_")
        || s.starts_with("DW.ref.")
        || s.starts_with("_R")
        || s == "rust_eh_personality"
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c = defined_syms(c_lib_path());
    let r = defined_syms(rust_lib_path());

    assert!(
        !c.is_empty(),
        "no symbols found in the C .so — harness problem"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so [OP={OP} REPEAT={REPEAT}]:\n{:#?}\n\
         C symbols:    {:?}\nRust symbols: {:?}",
        missing.len(),
        missing,
        c,
        r.iter().filter(|s| !is_rust_internal(s)).collect::<Vec<_>>()
    );
}

#[test]
fn the_eight_documented_symbols_are_present_with_matching_types() {
    // Exactly the surface recorded in SYMBOLS.md.
    let expected: [(&str, &str); 8] = [
        ("op_add", "T"),
        ("op_sub", "T"),
        ("op_mul", "T"),
        ("helper_call", "T"),
        ("helper_ptr", "T"),
        ("use_generated", "T"),
        ("G_OP", "D"),
        ("G_OP_NAME", "D"),
    ];

    let c = defined_syms_typed(c_lib_path());
    let r = defined_syms_typed(rust_lib_path());

    // The C .so exports precisely these eight and nothing else.
    let c_names: BTreeSet<&str> = c.iter().map(|(n, _)| n.as_str()).collect();
    let want: BTreeSet<&str> = expected.iter().map(|(n, _)| *n).collect();
    assert_eq!(
        c_names, want,
        "the C library surface changed — SYMBOLS.md needs updating"
    );

    for (name, ty) in expected {
        let cty = c
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, t)| t.clone())
            .unwrap_or_else(|| panic!("C .so lacks {name}"));
        let rty = r
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, t)| t.clone())
            .unwrap_or_else(|| panic!("Rust .so lacks {name}"));
        assert_eq!(cty, ty, "unexpected nm type for {name} in the C .so");
        assert_eq!(
            rty, cty,
            "nm type mismatch for {name}: C={cty} Rust={rty} \
             (T=function/text, D=initialised writable data)"
        );
    }
}

#[test]
fn static_accum_has_internal_linkage_in_both() {
    // DEFINE_ACCUM expands to `static int accum_<op>(int n)`, so it must NOT be
    // a dynamic symbol in either library.
    let c = defined_syms(c_lib_path());
    let r = defined_syms(rust_lib_path());
    for op in ["add", "sub", "mul"] {
        let n = format!("accum_{op}");
        assert!(!c.contains(&n), "C .so unexpectedly exports {n}");
        assert!(!r.contains(&n), "Rust .so unexpectedly exports {n}");
    }
}

#[test]
fn main_is_not_part_of_the_library_surface() {
    // `main` lives in mdmain.c, which is linked into the executable only.
    let c = defined_syms(c_lib_path());
    let r = defined_syms(rust_lib_path());
    assert!(!c.contains("main"), "C library .so exports main");
    assert!(!r.contains("main"), "Rust cdylib exports main");
}

#[test]
fn rust_so_has_no_unresolved_symbols() {
    // RTLD_NOW forces every undefined symbol to be bound at load time; success
    // therefore proves there are no missing (non-libc or otherwise) symbols.
    use libloading::os::unix::{Library, RTLD_LOCAL, RTLD_NOW};
    for (label, p) in [("C", c_lib_path()), ("Rust", rust_lib_path())] {
        let lib = unsafe { Library::open(Some(p), RTLD_NOW | RTLD_LOCAL) };
        assert!(
            lib.is_ok(),
            "{label} .so has unresolved symbols (RTLD_NOW failed): {:?}",
            lib.err()
        );
    }
}

#[test]
fn exported_globals_live_in_a_writable_section_in_both() {
    // mdmacros.h declares `extern int (*G_OP)(int,int);` and
    // `extern const char *G_OP_NAME;` -- neither pointer is const, so both must
    // sit in a writable section (C puts them in .data). If Rust placed them in
    // .data.rel.ro, RELRO would make a consumer's store segfault.
    for (label, p) in [("C", c_lib_path()), ("Rust", rust_lib_path())] {
        for sym in ["G_OP", "G_OP_NAME"] {
            let sec = symbol_section(p, sym);
            let flags = section_flags(p, &sec);
            assert!(
                flags.contains('W'),
                "{label} .so: {sym} is in non-writable section {sec} (flags {flags})"
            );
            assert_eq!(
                sec, ".data",
                "{label} .so: {sym} should be in .data (as the C compiler places it), found {sec}"
            );
        }
    }
}

/// Section name containing `sym`, via `readelf`.
fn symbol_section(so: &Path, sym: &str) -> String {
    let syms = Command::new("readelf")
        .args(["-sW"])
        .arg(so)
        .output()
        .expect("readelf -s");
    let idx = String::from_utf8_lossy(&syms.stdout)
        .lines()
        .find(|l| l.split_whitespace().last() == Some(sym))
        .and_then(|l| {
            l.split_whitespace()
                .nth(6)
                .map(|s| s.trim().parse::<usize>().ok())
        })
        .flatten()
        .unwrap_or_else(|| panic!("symbol {sym} not found in {so:?}"));

    let secs = Command::new("readelf")
        .args(["-SW"])
        .arg(so)
        .output()
        .expect("readelf -S");
    let text = String::from_utf8_lossy(&secs.stdout);
    for l in text.lines() {
        // "  [27] .data   PROGBITS  ..."
        if let Some(open) = l.find('[') {
            if let Some(close) = l.find(']') {
                if let Ok(n) = l[open + 1..close].trim().parse::<usize>() {
                    if n == idx {
                        if let Some(name) = l[close + 1..].split_whitespace().next() {
                            return name.to_string();
                        }
                    }
                }
            }
        }
    }
    panic!("section index {idx} not found in {so:?}");
}

/// Flag letters of a section, via `readelf -SW`.
fn section_flags(so: &Path, section: &str) -> String {
    let secs = Command::new("readelf")
        .args(["-SW"])
        .arg(so)
        .output()
        .expect("readelf -S");
    let text = String::from_utf8_lossy(&secs.stdout);
    for l in text.lines() {
        let mut it = l.split_whitespace();
        // [ n] name type addr off size es flg lk inf al
        if let Some(close) = l.find(']') {
            let rest = &l[close + 1..];
            let mut r = rest.split_whitespace();
            if r.next() == Some(section) {
                // flags are the token after size+es, i.e. field 5 of `rest`
                let fields: Vec<&str> = rest.split_whitespace().collect();
                if fields.len() > 6 {
                    return fields[6].to_string();
                }
            }
        }
        let _ = it.next();
    }
    String::new()
}
