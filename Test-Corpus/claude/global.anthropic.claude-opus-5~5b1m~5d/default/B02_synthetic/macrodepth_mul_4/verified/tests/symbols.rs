//! Phase D — exported-symbol parity between the C `.so` and the Rust `.so`.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

/// Dynamic *defined* symbols of an object, as reported by `nm -D --defined-only`.
fn dyn_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("nm {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .map(|s| s.to_string())
        .collect()
}

/// The C library's full exported surface (from `nm -D` on the C `.so`).
const EXPECTED_C_SYMBOLS: [&str; 8] = [
    "G_OP",
    "G_OP_NAME",
    "helper_call",
    "helper_ptr",
    "op_add",
    "op_mul",
    "op_sub",
    "use_generated",
];

#[test]
fn symbol_parity_c_vs_rust() {
    let c = dyn_symbols(&c_so_path());
    let r = dyn_symbols(&rust_so_path());

    // Sanity: the C surface is what SYMBOLS.md documents.
    let expected: BTreeSet<String> = EXPECTED_C_SYMBOLS.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        c, expected,
        "the C .so's exported surface changed; update SYMBOLS.md"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing C-exported symbols {missing:?}\n\
         C  ({}): {c:?}\nRust has {} dynamic symbols",
        c.len(),
        r.len()
    );
}

/// The `static` (file-local) macro-generated accumulators must NOT be exported by
/// either object — a stub/extra export would be a fidelity bug in the other
/// direction.
#[test]
fn static_accum_not_exported() {
    let c = dyn_symbols(&c_so_path());
    let r = dyn_symbols(&rust_so_path());
    for name in ["accum_add", "accum_sub", "accum_mul", "accum", "main"] {
        assert!(!c.contains(name), "C unexpectedly exports {name}");
        assert!(!r.contains(name), "Rust unexpectedly exports {name}");
    }
}

/// Every documented symbol must actually be resolvable through `dlsym` in both
/// objects (catches e.g. a symbol present in `nm` but not dynamically bindable).
#[test]
fn all_symbols_resolvable_via_dlsym() {
    let (c, r) = pair();
    for name in ["op_add", "op_sub", "op_mul", "helper_call", "helper_ptr"] {
        let _ = c.bin(name);
        let _ = r.bin(name);
    }
    let _ = c.un("use_generated");
    let _ = r.un("use_generated");
    let _ = c.g_op();
    let _ = r.g_op();
    let _ = c.g_op_name();
    let _ = r.g_op_name();
}
