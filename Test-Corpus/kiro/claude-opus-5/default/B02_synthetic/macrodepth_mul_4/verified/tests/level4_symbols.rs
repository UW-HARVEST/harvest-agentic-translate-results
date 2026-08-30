//! Level 4 — exported-symbol parity between the two shared objects.
//!
//! `mdcore.c` gives external linkage to `op_add`, `op_sub`, `op_mul`,
//! `helper_call`, `helper_ptr`, `use_generated`, `G_OP` and `G_OP_NAME`;
//! `accum_<OP>` is `static` and must stay unexported. Every dynamic symbol the
//! C object defines has to be defined by the Rust `cdylib` under the identical
//! name, and it must actually be resolvable with `dlsym`.

mod common;

use common::{Impl, c_so_path, rust_so_path};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// `nm -D --defined-only <so>` reduced to the set of symbol names.
fn defined_dynamic_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm -D {} failed: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .map(str::to_owned)
        .collect()
}

/// Rust's `cdylib` additionally exports its own runtime glue; only symbols the
/// C object could plausibly own are relevant for the "no extras" direction.
fn is_rust_runtime_symbol(name: &str) -> bool {
    name.starts_with("_ZN")
        || name.starts_with("rust_")
        || name.starts_with("__rust")
        || name.starts_with("_R")
        || name.contains("17h")
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c = defined_dynamic_symbols(&c_so_path());
    let r = defined_dynamic_symbols(&rust_so_path());

    assert!(
        !c.is_empty(),
        "nm found no defined symbols in the C reference object"
    );

    let missing: Vec<_> = c.difference(&r).cloned().collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}\n\
         C:    {c:?}\n\
         Rust: {r:?}"
    );
}

/// The exact public surface of `mdcore.c`, spelled out so a regression in the
/// `#[no_mangle]` wrappers is caught even if `nm` output changes shape.
#[test]
fn expected_symbols_are_present_in_both() {
    const EXPECTED: [&str; 8] = [
        "op_add",
        "op_sub",
        "op_mul",
        "helper_call",
        "helper_ptr",
        "use_generated",
        "G_OP",
        "G_OP_NAME",
    ];
    let c = defined_dynamic_symbols(&c_so_path());
    let r = defined_dynamic_symbols(&rust_so_path());
    for s in EXPECTED {
        assert!(c.contains(s), "C .so unexpectedly lacks {s}");
        assert!(r.contains(s), "Rust .so lacks {s}");
    }
}

/// `nm` says the symbol exists; `dlsym` proves it is usable.
#[test]
fn every_c_symbol_resolves_via_dlsym_in_both() {
    let (ci, ri) = Impl::pair();
    for s in defined_dynamic_symbols(&c_so_path()) {
        assert!(ci.has_symbol(&s), "C: dlsym({s}) failed");
        assert!(ri.has_symbol(&s), "Rust: dlsym({s}) failed");
    }
}

/// `accum_<OP>` is `static` in C, so neither object may leak it.
#[test]
fn static_accumulator_stays_private() {
    let c = defined_dynamic_symbols(&c_so_path());
    let r = defined_dynamic_symbols(&rust_so_path());
    for name in ["accum_add", "accum_sub", "accum_mul", "accum_op"] {
        assert!(!c.contains(name), "C .so exports {name}");
        assert!(!r.contains(name), "Rust .so exports {name}");
    }
}

/// The Rust object should not export extra non-runtime symbols that the C
/// object does not have; that would change the ABI an external caller sees.
#[test]
fn rust_so_exports_no_extra_c_like_symbols() {
    let c = defined_dynamic_symbols(&c_so_path());
    let r = defined_dynamic_symbols(&rust_so_path());
    let extra: Vec<_> = r
        .difference(&c)
        .filter(|n| !is_rust_runtime_symbol(n))
        .cloned()
        .collect();
    assert!(
        extra.is_empty(),
        "Rust .so exports unexpected C-style symbols: {extra:?}"
    );
}
