//! Phase D — symbol parity gate.
//!
//! Runs `nm -D` on both `.so` files and asserts that every symbol the C library
//! exports is also exported by the Rust library, under the exact same name.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn c_so() -> PathBuf {
    let build = root().join("c_src").join("build");
    let mut v: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("read_dir {}: {e}", build.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let n = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            n.starts_with("lib") && n.ends_with(".so")
        })
        .collect();
    v.sort();
    v.into_iter().next().expect("C .so not built")
}

fn rust_so() -> PathBuf {
    let t = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    for p in ["release", "debug"] {
        let c = t.join(p).join("libarrayfunc_lib.so");
        if c.is_file() {
            return c;
        }
    }
    panic!("Rust .so not built")
}

/// Symbols the toolchain/CRT injects; not part of the translated surface.
fn is_toolchain_symbol(n: &str) -> bool {
    matches!(
        n,
        "_init"
            | "_fini"
            | "__bss_start"
            | "_edata"
            | "_end"
            | "__libc_csu_init"
            | "__libc_csu_fini"
            | "_ITM_registerTMCloneTable"
            | "_ITM_deregisterTMCloneTable"
            | "__gmon_start__"
            | "__cxa_finalize"
            | "rust_eh_personality"
            | "__rust_no_alloc_shim_is_unstable_v2"
    ) || n.starts_with("_ZN")
        || n.starts_with("__rust")
        || n.starts_with("rust_")
        || n.starts_with("_R")
        || n.starts_with("__gnu")
        || n.starts_with("_Unwind")
        || n.starts_with("__Unwind")
}

fn defined_symbols(so: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("nm not available");
    assert!(out.status.success(), "nm failed on {}", so.display());
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            // exported code/data
            if !matches!(kind, "T" | "W" | "D" | "B" | "R" | "G") {
                return None;
            }
            if is_toolchain_symbol(name) {
                return None;
            }
            Some(name.to_string())
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn phase_d_every_c_symbol_is_exported_by_rust() {
    let c = defined_symbols(&c_so());
    let r = defined_symbols(&rust_so());
    assert!(!c.is_empty(), "no symbols read from the C .so");

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} C symbol(s): {:?}\nC:    {:?}\nRust: {:?}",
        missing.len(),
        missing,
        c,
        r
    );

    // Also report anything extra, to keep SYMBOLS.md honest.
    let extra: Vec<&String> = r.iter().filter(|s| !c.contains(s)).collect();
    assert!(
        extra.is_empty(),
        "Rust .so exports {} symbol(s) the C .so does not: {:?}",
        extra.len(),
        extra
    );

    assert_eq!(c.len(), 11, "expected the 11 external functions of lib.c, got {c:?}");
}

#[test]
fn phase_d_all_symbols_resolve_at_dlopen() {
    // `libs()` performs `dlopen` + `dlsym` for all 11 symbols on BOTH libraries;
    // if any were missing or unresolvable this panics.
    let p = common::libs();
    assert_eq!(p.c.name, "C");
    assert_eq!(p.rs.name, "Rust");
    // Undefined symbols in the Rust .so must be libc/toolchain only.
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", rust_so().to_str().unwrap()])
        .output()
        .expect("nm");
    let project = [
        "add_operation",
        "multiply_operation",
        "subtract_operation",
        "modulo_operation",
        "safe_double_to_int",
        "compute_scaled_value",
        "compare_results_in_array",
        "init_result_array",
        "process_with_foreach",
        "compute_weighted_sum",
        "arrayfunc",
    ];
    let txt = String::from_utf8_lossy(&out.stdout);
    for name in project {
        assert!(
            !txt.split_whitespace().any(|t| t == name),
            "Rust .so has UNDEFINED project symbol {name}"
        );
    }
}
