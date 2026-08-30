// Phase D: symbol parity, checked mechanically rather than by eye.
//
// Every symbol the C `.so` exports must also be exported by the Rust `.so`
// under the exact same name, and both must be reachable through `dlsym`.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

/// `nm -D --defined-only <so>` -> set of exported symbol names.
fn exported_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {so:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect()
}

/// Rust cdylibs also export a few toolchain-internal symbols that have no C
/// counterpart; the parity requirement runs C -> Rust, so these are irrelevant,
/// but filter them when reporting the reverse direction.
fn is_rust_runtime_symbol(s: &str) -> bool {
    s.starts_with("_ZN")
        || s.starts_with("__rust")
        || s.starts_with("rust_")
        || s.starts_with("_R")
        || matches!(
            s,
            "_init"
                | "_fini"
                | "__bss_start"
                | "_edata"
                | "_end"
                | "__libc_csu_init"
                | "__libc_csu_fini"
        )
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c_syms = exported_symbols(&c_so_path());
    let rust_syms = exported_symbols(&rust_so_path());

    assert!(
        c_syms.contains("driver"),
        "sanity: C .so should export `driver`, got {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C   exports: {c_syms:?}\n\
         Rust exports (non-runtime): {:?}",
        missing.len(),
        rust_syms
            .iter()
            .filter(|s| !is_rust_runtime_symbol(s))
            .collect::<Vec<_>>()
    );
}

/// `print_hex` is `static` in the C, so it must NOT appear in either ABI.
/// Exporting it from Rust would itself be a divergence.
#[test]
fn static_helper_is_not_exported() {
    for so in [c_so_path(), rust_so_path()] {
        let syms = exported_symbols(&so);
        assert!(
            !syms.contains("print_hex"),
            "{so:?} unexpectedly exports the `static` helper `print_hex`"
        );
    }
}

/// Both `driver` symbols must be resolvable through `dlsym` and callable.
#[test]
fn both_driver_symbols_are_callable() {
    let c_out = capture_stdout(|| unsafe { c_driver()(0x0102_0304) });
    let rust_out = capture_stdout(|| unsafe { rust_driver()(0x0102_0304) });
    assert!(!c_out.is_empty() && !rust_out.is_empty());
    assert_eq!(c_out, rust_out);
}
