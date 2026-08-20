// Phase D — symbol parity, enforced as a test so it cannot silently rot.
//
// Every symbol exported by the C shared object must be exported by the Rust
// shared object under the exact same name, and the Rust object must not have
// any undefined symbol that cannot be satisfied by the libraries it links.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

fn nm(args: &[&str], path: &std::path::Path) -> String {
    let out = Command::new("nm")
        .args(args)
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm: {e}"));
    assert!(
        out.status.success(),
        "nm {args:?} {path:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Names of globally-visible *defined* symbols, ignoring toolchain-internal
/// ones that neither object "owns".
fn defined_symbols(path: &std::path::Path) -> BTreeSet<String> {
    nm(&["-D", "--defined-only"], path)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_owned))
        .filter(|n| {
            !n.starts_with("_ITM_")
                && !n.starts_with("__cxa_")
                && !n.starts_with("__gmon_")
                && n != "_edata"
                && n != "_end"
                && n != "__bss_start"
                && n != "_init"
                && n != "_fini"
        })
        .collect()
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let (c_path, rust_path) = common::so_paths();
    let c_syms = defined_symbols(&c_path);
    let rust_syms = defined_symbols(&rust_path);

    assert!(
        c_syms.contains("driver"),
        "sanity: the C .so must export `driver`, got {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C   ({}): {c_syms:?}\n\
         RS  ({}): {rust_syms:?}",
        missing.len(),
        c_syms.len(),
        rust_syms.len()
    );
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let (_c_path, rust_path) = common::so_paths();
    // ldd resolves everything the loader needs; an unresolvable symbol shows up
    // as "undefined symbol" here.
    let out = Command::new("ldd")
        .arg("-r")
        .arg(&rust_path)
        .output()
        .expect("run ldd");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !text.contains("undefined symbol"),
        "Rust .so has unresolved symbols:\n{text}"
    );
    assert!(
        !text.contains("not found"),
        "Rust .so has unresolved libraries:\n{text}"
    );
}

#[test]
fn rust_so_imports_the_same_libc_printf_as_c() {
    // The C library's only import is `printf`; the translation must go through
    // the very same C runtime entry point, otherwise stdout buffering and the
    // exact byte stream could differ from the original library.
    let (c_path, rust_path) = common::so_paths();
    let c_undef = nm(&["-D", "--undefined-only"], &c_path);
    let rust_undef = nm(&["-D", "--undefined-only"], &rust_path);
    assert!(c_undef.contains("printf@"), "unexpected C imports:\n{c_undef}");
    assert!(
        rust_undef.contains("printf@"),
        "the Rust .so does not import libc printf:\n{rust_undef}"
    );
}
