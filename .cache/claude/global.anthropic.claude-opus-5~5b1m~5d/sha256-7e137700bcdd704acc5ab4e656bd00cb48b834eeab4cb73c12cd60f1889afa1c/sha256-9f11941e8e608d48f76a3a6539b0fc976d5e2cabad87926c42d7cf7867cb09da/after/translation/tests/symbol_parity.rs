//! Phase D — `nm -D` symbol parity between the C `.so` and the Rust `.so`.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

/// Weak / toolchain-injected symbols that belong to the C runtime or to Rust
/// std, not to the translated source. Excluded from the parity comparison.
const TOOLCHAIN: &[&str] = &[
    "_ITM_deregisterTMCloneTable",
    "_ITM_registerTMCloneTable",
    "__cxa_finalize",
    "__cxa_thread_atexit_impl",
    "__gmon_start__",
    "gettid",
    "statx",
    "_init",
    "_fini",
    "__bss_start",
    "_edata",
    "_end",
];

/// Defined, globally-visible dynamic symbols (`nm -D`, type letter upper-case,
/// i.e. global; skipping undefined `U` and weak `w`/`v`/`V`).
fn defined_dynamic_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg(path)
        .output()
        .expect("nm not available");
    assert!(out.status.success(), "nm -D {} failed", path.display());
    let text = String::from_utf8_lossy(&out.stdout);

    let mut set = BTreeSet::new();
    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 2 {
            continue;
        }
        // "<addr> <type> <name>" for defined, "         U <name>" / "w <name>"
        // for undefined/weak.
        let (ty, name) = if cols.len() >= 3 {
            (cols[1], cols[2])
        } else {
            (cols[0], cols[1])
        };
        if ty == "U" || ty.chars().next().is_some_and(|c| c.is_lowercase()) {
            continue; // undefined or weak/local
        }
        let base = name.split('@').next().unwrap_or(name);
        if TOOLCHAIN.contains(&base) {
            continue;
        }
        set.insert(base.to_string());
    }
    set
}

#[test]
fn phase_d_symbol_diff_is_empty() {
    let c_so = std::path::PathBuf::from(std::env::var("DIFF_C_SO").unwrap());
    let r_so = std::path::PathBuf::from(std::env::var("DIFF_RUST_SO").unwrap());

    let cs = defined_dynamic_symbols(&c_so);
    let rs = defined_dynamic_symbols(&r_so);

    let missing: Vec<&String> = cs.difference(&rs).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so \
         [OP={OP} REPEAT={REPEAT}]: {missing:?}\nC: {cs:?}\nRust: {rs:?}"
    );

    // Sanity: the full documented API really is present.
    for want in [
        "op_add",
        "op_sub",
        "op_mul",
        "helper_call",
        "helper_ptr",
        "use_generated",
        "G_OP",
        "G_OP_NAME",
    ] {
        assert!(cs.contains(want), "C .so lost {want}");
        assert!(rs.contains(want), "Rust .so lost {want}");
    }

    // `accum_<OP>` is `static` in C — it must not be exported by either side.
    for op in ["add", "sub", "mul"] {
        let n = format!("accum_{op}");
        assert!(!cs.contains(&n), "C unexpectedly exports {n}");
        assert!(!rs.contains(&n), "Rust must not export the static {n}");
    }
}

#[test]
fn phase_d_no_unresolved_non_libc_symbols_in_rust_so() {
    let r_so = std::path::PathBuf::from(std::env::var("DIFF_RUST_SO").unwrap());
    let out = Command::new("ldd")
        .arg("-r")
        .arg(&r_so)
        .output()
        .expect("ldd not available");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !combined.contains("undefined symbol"),
        "Rust .so has unresolved symbols:\n{combined}"
    );
}
