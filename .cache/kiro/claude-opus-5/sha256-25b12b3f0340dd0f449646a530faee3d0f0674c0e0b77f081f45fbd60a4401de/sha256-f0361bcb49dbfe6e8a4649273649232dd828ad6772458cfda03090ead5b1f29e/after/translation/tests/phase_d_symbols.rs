//! Phase D — symbol parity, verified from inside the test suite as well as
//! from the shell.

mod common;
use common::pair;
use std::process::Command;

fn defined_dynamic_symbols(so: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm -D");
    assert!(out.status.success(), "nm failed on {}", so.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let a = it.next()?;
            let b = it.next()?;
            let (ty, name) = match it.next() {
                Some(n) => (b, n),
                None => (a, b),
            };
            // Only global/weak text & data definitions.
            if matches!(ty, "T" | "t" | "D" | "B" | "R" | "W" | "V") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn phase_d_every_c_symbol_is_exported_by_rust() {
    let p = pair();
    let c_syms = defined_dynamic_symbols(&p.c_path);
    let rs_syms = defined_dynamic_symbols(&p.rs_path);

    assert!(
        c_syms.iter().any(|s| s == "ldexp_q2"),
        "C .so must export ldexp_q2; got {c_syms:?}"
    );

    // Ignore linker/CRT/toolchain-generated names when diffing.
    let ignored = |s: &str| {
        matches!(
            s,
            "_init" | "_fini" | "__bss_start" | "_edata" | "_end" | "_IO_stdin_used"
        ) || s.starts_with("__")
    };

    let missing: Vec<&String> = c_syms
        .iter()
        .filter(|s| !ignored(s) && !rs_syms.contains(s))
        .collect();

    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C: {c_syms:?}\nRS(subset): {:?}",
        rs_syms.iter().take(40).collect::<Vec<_>>()
    );
}

#[test]
fn phase_d_rust_symbol_is_callable_through_dlsym() {
    // Already implied by every other test, but assert it explicitly so the
    // no_mangle wrapper is covered even if all differential tests were skipped.
    let p = pair();
    let c = unsafe { (p.c)(1.0, 0) };
    let r = unsafe { (p.rs)(1.0, 0) };
    assert_eq!(c.to_bits(), r.to_bits());
    assert_eq!(c.to_bits(), 1.0f32.to_bits());
}
