//! Step 8 as an automated check: every dynamic symbol the C shared object
//! defines must also be defined by the Rust cdylib under the exact same name.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Names emitted by the toolchain/CRT rather than by the translation unit.
fn is_toolchain_symbol(name: &str) -> bool {
    matches!(
        name,
        "_init"
            | "_fini"
            | "__bss_start"
            | "_edata"
            | "_end"
            | "__cxa_finalize"
            | "_ITM_registerTMCloneTable"
            | "_ITM_deregisterTMCloneTable"
            | "__gmon_start__"
    ) || name.starts_with("__odr_asan")
}

fn defined_dynamic_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("failed to run `nm` (binutils required)");
    assert!(
        out.status.success(),
        "nm -D --defined-only {} failed: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2))
        .filter(|n| !is_toolchain_symbol(n))
        .map(str::to_owned)
        .collect()
}

#[test]
fn rust_exports_every_c_symbol() {
    let c_syms = defined_dynamic_symbols(&c_lib_path());
    let rust_syms = defined_dynamic_symbols(&rust_lib_path());

    assert!(
        !c_syms.is_empty(),
        "no symbols read from the C library — is it built?"
    );

    // The documented API must be present in both.
    for expected in [
        "hatch",
        "increment_counter",
        "update_accumulator",
        "apply_operation",
        "add_three",
        "multiply_add",
        "complex_calc",
        "shift_array_data",
        "process_pointer_data",
        "compute_with_dynamic_memory",
        "get_time_based_value",
        "manipulate_records",
    ] {
        assert!(c_syms.contains(expected), "C library lost `{expected}`");
    }

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "the Rust cdylib is missing {} symbol(s) exported by the C .so: {missing:?}",
        missing.len()
    );
}

/// Every exported C symbol must also be resolvable through `dlsym` on both
/// libraries, not merely present in the symbol table.
#[test]
fn every_c_symbol_is_loadable_from_both() {
    let libs = load();
    for name in defined_dynamic_symbols(&c_lib_path()) {
        libs.pair::<unsafe extern "C" fn()>(&name);
    }
}
