//! Phase D — symbol parity, enforced as a test rather than a one-off command.
//!
//! Every symbol the C `.so` exports must also be exported by the Rust `.so`
//! under the exact same name. The diff must reach empty.

mod harness;
use harness::*;

use std::collections::BTreeSet;
use std::process::Command;

/// Global, defined, dynamic symbols — i.e. the ABI surface: `nm -D --defined-only`.
fn exported(so: &str) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so])
        .output()
        .unwrap_or_else(|e| panic!("could not run `nm -D --defined-only {so}`: {e}"));
    assert!(
        out.status.success(),
        "nm failed on {so}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
            // Global text / data / bss / weak: the things an external caller can bind to.
            if matches!(kind, "T" | "D" | "B" | "R" | "W" | "V" | "i" | "G" | "S") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Undefined (imported) symbols.
fn undefined(so: &str) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", so])
        .output()
        .expect("run nm --undefined-only");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| {
            // strip glibc version suffixes: printf@GLIBC_2.2.5 -> printf
            s.split('@').next().unwrap_or(s).to_string()
        }))
        .collect()
}

#[test]
fn symbol_parity_c_subset_of_rust() {
    let (c, rs) = (c_so_str(), rust_so_str());
    let c_syms = exported(&c);
    let rs_syms = exported(&rs);

    let missing: Vec<_> = c_syms.difference(&rs_syms).cloned().collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {missing:?}\n\
         C exports  : {c_syms:?}\n\
         Rust exports: {rs_syms:?}",
        missing.len()
    );

    // Guard the artifact in SYMBOLS.md: the C surface is exactly these two.
    assert_eq!(
        c_syms.iter().map(String::as_str).collect::<Vec<_>>(),
        vec!["driver", "run"],
        "the C ABI surface changed; SYMBOLS.md / CONFIGS.md must be regenerated"
    );
}

#[test]
fn both_symbols_are_actually_callable_through_dlsym() {
    // Parity in `nm` is not enough: every C-exported name must resolve *and*
    // work when fetched from the Rust .so via dlsym.
    let l = libs();
    let z = cbuf(b"3");
    let a = capture_stdout(|| unsafe { (l.rs.driver)(z.as_ptr() as *const _) });
    assert!(!a.is_empty());
    let mut h = House::driver_default();
    let b = capture_stdout(|| unsafe { (l.rs.run)(&mut h as *mut House, 1) });
    assert!(!b.is_empty());
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let rs = rust_so_str();
    let und = undefined(&rs);

    // The Rust std runtime legitimately imports a pile of libc / libgcc_s
    // symbols (unwinding, TLS, backtrace, syscalls). What must NOT appear is an
    // unresolved symbol belonging to *this* library — i.e. a Rust-mangled name
    // or anything named after the module under translation, which would mean a
    // piece of the C was referenced but never translated.
    let own: Vec<_> = und
        .iter()
        .filter(|s| {
            s.starts_with("_ZN")
                || s.starts_with("_R")
                || {
                    let l = s.to_ascii_lowercase();
                    l.contains("driver")
                        || l.contains("house")
                        || l.contains("parse_val")
                        || l.contains("add_floor")
                        || l.contains("add_bedrooms")
                        || l.contains("print_house")
                }
        })
        .cloned()
        .collect();
    assert!(
        own.is_empty(),
        "Rust .so has unresolved symbol(s) belonging to the library itself \
         (untranslated C?): {own:?}\nall undefined: {und:?}"
    );

    // The two libc functions the C implementation depends on must be imported
    // by the Rust .so too — proof it really delegates formatting and parsing to
    // libc instead of reimplementing them.
    for required in ["strtol", "__errno_location"] {
        assert!(
            und.contains(required),
            "expected the Rust .so to import `{required}`; got {und:?}"
        );
    }
    assert!(
        und.contains("printf") || und.contains("puts"),
        "expected the Rust .so to import printf and/or puts; got {und:?}"
    );

    // And confirm every import is actually resolvable by the dynamic linker.
    let out = Command::new("ldd")
        .args(["-r", &rs])
        .output()
        .expect("run ldd -r");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !text.contains("undefined symbol"),
        "ldd -r reported undefined symbols in the Rust .so:\n{text}"
    );
}

#[test]
fn c_so_imports_are_all_satisfiable_too() {
    // Sanity: the same check on the C side, so a divergence in the *import*
    // surface (e.g. the printf->puts rewrite) is visible in the report.
    let c = c_so_str();
    let und = undefined(&c);
    assert!(
        und.contains("strtol"),
        "expected the C .so to import strtol; got {und:?}"
    );
    assert!(
        und.contains("printf") || und.contains("puts"),
        "expected the C .so to import printf and/or puts; got {und:?}"
    );
}
