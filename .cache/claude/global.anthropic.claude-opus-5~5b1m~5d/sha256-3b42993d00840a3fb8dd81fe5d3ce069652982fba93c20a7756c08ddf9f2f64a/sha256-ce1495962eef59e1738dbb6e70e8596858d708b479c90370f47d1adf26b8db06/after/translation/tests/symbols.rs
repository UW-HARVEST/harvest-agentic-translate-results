// Phase A / Phase D — symbol parity between the C `.so` and the Rust `.so`.
mod common;

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build = manifest_dir().parent().unwrap().join("c_src/build");
    let mut v: Vec<PathBuf> = std::fs::read_dir(&build)
        .expect("build the C library first")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    v.sort();
    v.pop().unwrap()
}

fn rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    for prof in ["release", "debug"] {
        let p = manifest_dir().join("target").join(prof).join("libenvy_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("libenvy_lib.so not built");
}

fn nm(args: &[&str], so: &PathBuf) -> String {
    let out = Command::new("nm")
        .args(args)
        .arg(so)
        .output()
        .expect("nm not available");
    assert!(out.status.success(), "nm failed on {}", so.display());
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Exported (dynamic, defined, text/data) symbol names.
fn exported(so: &PathBuf) -> BTreeSet<String> {
    nm(&["-D", "--defined-only"], so)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
        .collect()
}

fn undefined(so: &PathBuf) -> BTreeSet<String> {
    nm(&["-D", "--undefined-only"], so)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .filter(|s| s != "U" && !s.is_empty())
        .collect()
}

const EXPECTED: [&str; 5] = [
    "apply_bit_operations",
    "envy",
    "init_config_from_env",
    "parse_env_numeric",
    "perform_operation",
];

#[test]
fn sym_01_c_exports_the_expected_five() {
    let c = exported(&c_so());
    for s in EXPECTED {
        assert!(c.contains(s), "C .so unexpectedly misses {s}: {c:?}");
    }
}

#[test]
fn sym_02_every_c_symbol_is_exported_by_rust() {
    let c = exported(&c_so());
    let r = exported(&rust_so());
    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   = {c:?}\nRust = {r:?}"
    );
}

#[test]
fn sym_03_rust_has_no_undefined_non_libc_symbols() {
    // Definitive check: RTLD_NOW resolves *every* undefined symbol eagerly at
    // load time. If the Rust .so referenced anything a plain consumer cannot
    // provide (i.e. anything outside libc / the platform), this would fail.
    use libloading::os::unix::{Library, RTLD_LOCAL, RTLD_NOW};
    for so in [c_so(), rust_so()] {
        let lib = unsafe { Library::open(Some(&so), RTLD_NOW | RTLD_LOCAL) };
        assert!(lib.is_ok(), "{} has unresolvable symbols: {:?}", so.display(), lib.err());
    }

    // Additionally: the set of libc functions the C code uses must all appear in
    // the Rust .so's import list, proving it calls straight through to libc
    // rather than reimplementing (and possibly diverging from) them.
    let u: BTreeSet<String> = undefined(&rust_so())
        .iter()
        .map(|s| s.split('@').next().unwrap().to_string())
        .collect();
    for needed in ["getenv", "atoi", "strchr", "printf", "fprintf", "snprintf", "stderr"] {
        assert!(u.contains(needed), "Rust .so does not import libc `{needed}`: {u:?}");
    }
}

#[test]
fn sym_04_all_five_symbols_are_callable_through_dlsym() {
    // Loading the pair only succeeds if every symbol resolves in *both* .so's.
    let (p, _g) = common::pair();
    assert_eq!(p.c.name, "C");
    assert_eq!(p.rs.name, "Rust");
}
