//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! The Rust `.so` must export every symbol the C `.so` exports, under the exact
//! same name. The diff must be empty; a partially exported library means the
//! translation is incomplete regardless of how well the exported subset behaves.
//!
//! Also checks that the Rust `.so` has no unresolved references to project code
//! (every import must be libc, the unwinder, or a weak CRT hook) by loading it
//! with `RTLD_NOW`, which resolves every relocation eagerly.

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn c_so() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../c_src/build/libdriver.so")
        .canonicalize()
        .expect("C library missing — build it with cmake first")
}

fn rust_so() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for p in [
        root.join("target/release/libdriver.so"),
        root.join("target/debug/libdriver.so"),
    ] {
        if p.exists() {
            return p.canonicalize().unwrap();
        }
    }
    panic!("Rust cdylib missing — run `cargo build --release`");
}

/// Names of dynamic symbols *defined* by an object, via `nm -D --defined-only`.
fn defined_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {so:?}");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect()
}

/// Names of dynamic symbols *imported* (undefined) by an object.
fn undefined_symbols(so: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "-u"])
        .arg(so)
        .output()
        .expect("run nm");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect()
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c = defined_symbols(&c_so());
    let r = defined_symbols(&rust_so());

    // Sanity: the C library must actually export the four known entry points,
    // otherwise an empty diff would be vacuous.
    for want in ["driver", "printLine", "bad", "good"] {
        assert!(c.contains(want), "C .so is missing {want}; nm parse is wrong");
    }

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "Rust .so does not export {} of the C .so's {} symbols: {missing:?}\n\
         Add the #[no_mangle] wrapper if the impl exists, or translate the \
         missing C source if a whole module was skipped.",
        missing.len(),
        c.len()
    );
}

#[test]
fn rust_has_no_unresolved_project_symbols() {
    // Every import must be libc, the libgcc unwinder, or a weak CRT hook.
    let allowed_exact = [
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__gmon_start__",
    ];
    let unexpected: Vec<String> = undefined_symbols(&rust_so())
        .into_iter()
        .filter(|s| {
            !(s.contains("@GLIBC_")
                || s.contains("@GCC_")
                || s.starts_with("_Unwind_")
                || allowed_exact.contains(&s.as_str()))
        })
        .collect();
    assert!(
        unexpected.is_empty(),
        "Rust .so has non-libc unresolved symbols: {unexpected:?}"
    );

    // RTLD_NOW resolves every relocation at load time, so a successful open is
    // positive proof that nothing is left dangling.
    let lib = unsafe {
        libloading::os::unix::Library::open(Some(rust_so()), 2 /* RTLD_NOW */)
    };
    assert!(lib.is_ok(), "dlopen(RTLD_NOW) failed: {:?}", lib.err());
}

#[test]
fn both_libraries_resolve_all_four_entry_points_by_name() {
    // Looking each symbol up by name through dlsym is what an external consumer
    // does; it also proves the Rust exports are callable, not just present in
    // the symbol table.
    for so in [c_so(), rust_so()] {
        let lib = unsafe { libloading::os::unix::Library::open(Some(&so), 2) }
            .unwrap_or_else(|e| panic!("dlopen {so:?}: {e}"));
        unsafe {
            lib.get::<unsafe extern "C" fn(std::ffi::c_int)>(b"driver\0")
                .unwrap_or_else(|e| panic!("driver in {so:?}: {e}"));
            lib.get::<unsafe extern "C" fn()>(b"bad\0").unwrap();
            lib.get::<unsafe extern "C" fn()>(b"good\0").unwrap();
            lib.get::<unsafe extern "C" fn(*const std::ffi::c_char)>(b"printLine\0")
                .unwrap();
        }
    }
}
