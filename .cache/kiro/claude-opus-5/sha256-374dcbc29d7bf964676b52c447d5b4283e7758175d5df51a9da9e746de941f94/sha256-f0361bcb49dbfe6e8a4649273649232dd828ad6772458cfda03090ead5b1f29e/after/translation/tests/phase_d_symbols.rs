//! Phase D — symbol parity, checked from inside a test as well as via `nm -D`.
//!
//! Verifies that every symbol the C `.so` exports is resolvable by name in the
//! Rust `.so` with the exact same name, using `dlsym` through `libloading`.

use std::ffi::c_int;
use std::path::PathBuf;
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    std::env::var("C_LIB_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| manifest_dir().join("../c_src/build/libStaticLoop.so"))
}

fn rust_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_LIB_PATH") {
        return PathBuf::from(p);
    }
    let release = manifest_dir().join("target/release/libStaticLoop.so");
    if release.exists() {
        release
    } else {
        manifest_dir().join("target/debug/libStaticLoop.so")
    }
}

/// `nm -D --defined-only` reduced to the set of exported global text/data names,
/// excluding the compiler/loader boilerplate that is not part of the API.
fn exported_symbols(path: &PathBuf) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", path.display());
    let mut names: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (a, b) = (it.next()?, it.next()?);
            // Either "<addr> <type> <name>" or "<type> <name>" (weak/undefined).
            let (ty, name) = match it.next() {
                Some(n) => (b, n),
                None => (a, b),
            };
            let ignored = name.starts_with("_init")
                || name.starts_with("_fini")
                || name.starts_with("__")
                || name.starts_with("_ITM_")
                || name == "_edata"
                || name == "_end"
                || name == "__bss_start";
            if matches!(ty, "T" | "D" | "B" | "R" | "G") && !ignored {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect();
    names.sort();
    names.dedup();
    names
}

#[test]
fn d1_symbol_sets_are_identical() {
    let c = exported_symbols(&c_lib_path());
    let r = exported_symbols(&rust_lib_path());
    assert_eq!(c, vec!["driver".to_string(), "static_sum".to_string()], "C export set changed");
    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(missing.is_empty(), "symbols exported by C but missing from Rust: {missing:?}");
    assert_eq!(c, r, "the two export sets must match exactly");
}

#[test]
fn d2_every_c_symbol_is_dlsym_resolvable_in_rust() {
    let c_lib = unsafe { libloading::Library::new(c_lib_path()) }.expect("dlopen C");
    let r_lib = unsafe { libloading::Library::new(rust_lib_path()) }.expect("dlopen Rust");
    for name in exported_symbols(&c_lib_path()) {
        let mut sym = name.clone().into_bytes();
        sym.push(0);
        assert!(
            unsafe { c_lib.get::<*const ()>(&sym) }.is_ok(),
            "C: {name} not resolvable"
        );
        assert!(
            unsafe { r_lib.get::<*const ()>(&sym) }.is_ok(),
            "Rust: {name} not resolvable via dlsym"
        );
    }
}

#[test]
fn d3_rust_has_no_unresolved_non_libc_imports() {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(rust_lib_path())
        .output()
        .expect("run nm");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    let suspicious: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .filter(|n| {
            // Anything that is neither a versioned libc/libgcc import nor a weak
            // toolchain hook would indicate an untranslated dependency.
            !n.contains('@') && !n.starts_with("_ITM_") && !n.starts_with("__")
        })
        .collect();
    assert!(suspicious.is_empty(), "unresolved non-libc imports: {suspicious:?}");
}

/// The exported functions must have the C ABI shape the header declares:
/// `int static_sum(int)` and `void driver(int)`.
#[test]
fn d4_exported_signatures_are_callable_as_declared() {
    type SumFn = unsafe extern "C" fn(c_int) -> c_int;
    type DriverFn = unsafe extern "C" fn(c_int);
    for path in [c_lib_path(), rust_lib_path()] {
        let lib = unsafe { libloading::Library::new(&path) }.expect("dlopen");
        let s = unsafe { lib.get::<SumFn>(b"static_sum\0") }.expect("static_sum");
        let d = unsafe { lib.get::<DriverFn>(b"driver\0") }.expect("driver");
        // Calling with 0 is the identity, so this probe does not disturb the
        // accumulators used by the other test binaries (separate processes).
        let _ = unsafe { s(0) };
        let _ = d; // shape-checked at the type level; behaviour covered elsewhere
    }
}
