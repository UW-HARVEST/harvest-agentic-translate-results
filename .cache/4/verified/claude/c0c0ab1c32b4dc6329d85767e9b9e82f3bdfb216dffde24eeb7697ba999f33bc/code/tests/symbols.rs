// Phase A / Phase D: exported-symbol parity between the C and the Rust
// shared object (see SYMBOLS.md).

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(args: &[&str], so: &Path) -> String {
    let out = Command::new("nm")
        .args(args)
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Names of the dynamic symbols *defined* by `so`.
fn defined_dynamic_symbols(so: &Path) -> BTreeSet<String> {
    nm(&["-D", "--defined-only"], so)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_owned))
        .map(|s| s.split('@').next().unwrap().to_owned())
        .collect()
}

/// Names of the dynamic symbols `so` needs someone else to provide.
fn undefined_dynamic_symbols(so: &Path) -> Vec<String> {
    nm(&["-D", "-u"], so)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_owned))
        .collect()
}

#[test]
fn c_so_symbols_are_all_exported_by_rust_so() {
    let c = defined_dynamic_symbols(&common::c_so());
    let r = defined_dynamic_symbols(&common::rust_so());

    assert!(
        c.contains("fma_array") && c.contains("driver") && c.contains("main"),
        "unexpected C symbol table: {c:?}"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}\n\
         C   : {c:?}\nRust: {r:?}"
    );
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let undef = undefined_dynamic_symbols(&common::rust_so());
    let non_libc: Vec<&String> = undef
        .iter()
        .filter(|s| {
            !s.contains("@GLIBC")
                && !s.contains("@GCC")
                && !s.starts_with("_ITM_")
                && *s != "__gmon_start__"
        })
        .collect();
    assert!(
        non_libc.is_empty(),
        "Rust .so has unresolved non-libc symbols: {non_libc:?}"
    );
}

#[test]
fn both_so_expose_the_same_callable_entry_points() {
    // Not just the names -- prove each one is actually loadable via dlsym from
    // both objects (this is what a real consumer does).
    let l = common::libs();
    let _ = l.c_fma();
    let _ = l.rust_fma();
    let _ = l.c_driver();
    let _ = l.rust_driver();

    for so in [common::c_so(), common::rust_so()] {
        let lib = unsafe { libloading::Library::new(&so) }.expect("dlopen");
        let sym = unsafe { lib.get::<common::MainFn>(b"main\0") };
        assert!(
            sym.is_ok(),
            "`main` is not dlsym-able from {}",
            so.display()
        );
    }
}
