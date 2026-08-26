//! Phase D — symbol parity, enforced from inside the test suite.
//!
//! `nm -D` is run on the C shared library that `build.rs` produced, and every
//! symbol it defines must be resolvable in the Rust `cdylib` (and vice versa).
//! This is the same check `run_all.sh` performs with `comm`, kept here so a
//! missing export fails `cargo test` too.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

use common::*;

fn defined_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let _addr = fields.next()?;
            let kind = fields.next()?;
            let name = fields.next()?;
            // Only global text/data symbols, and never the ELF bookkeeping ones.
            if kind == "T" || kind == "D" || kind == "B" {
                if name.starts_with("__") || name.starts_with("_ITM") {
                    None
                } else {
                    Some(name.to_string())
                }
            } else {
                None
            }
        })
        .collect()
}

/// The Rust `.so` must define every symbol the C `.so` defines, with the exact
/// same name — including the ones the `DEFINE_ARRAY` / `DEFINE_LIST` macros
/// generate and `main` itself.
#[test]
fn rust_so_exports_every_c_symbol() {
    let l = libs();
    let c_syms = defined_symbols(&l.c_path);
    let rs_syms = defined_symbols(&l.rs_path);

    assert!(
        c_syms.len() >= 63,
        "expected at least the 63 documented C symbols, found {}: {c_syms:?}",
        c_syms.len()
    );

    let missing: Vec<&String> = c_syms.difference(&rs_syms).collect();
    assert!(
        missing.is_empty(),
        "{} C symbol(s) are missing from the Rust .so: {missing:?}",
        missing.len()
    );

    let extra: Vec<&String> = rs_syms.difference(&c_syms).collect();
    assert!(
        extra.is_empty(),
        "the Rust .so exports {} symbol(s) the C .so does not: {extra:?}",
        extra.len()
    );

    // Every name must also actually resolve through `dlsym` in both libraries.
    for name in &c_syms {
        let mut bytes = name.as_bytes().to_vec();
        bytes.push(0);
        unsafe {
            l.c.get::<*const ()>(&bytes)
                .unwrap_or_else(|e| panic!("dlsym({name}) failed on the C library: {e}"));
            l.rs
                .get::<*const ()>(&bytes)
                .unwrap_or_else(|e| panic!("dlsym({name}) failed on the Rust library: {e}"));
        }
    }
}

/// The Rust `.so` must not leave anything undefined that is not provided by
/// libc / the Rust runtime's usual dependencies.
#[test]
fn rust_so_has_no_unexpected_undefined_symbols() {
    let l = libs();
    let out = Command::new("nm")
        .arg("-D")
        .arg("-u")
        .arg(&l.rs_path)
        .output()
        .expect("run nm -u");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    let c_defined = defined_symbols(&l.c_path);

    for line in text.lines() {
        let name = match line.split_whitespace().last() {
            Some(n) => n.split('@').next().unwrap_or(n).to_string(),
            None => continue,
        };
        assert!(
            !c_defined.contains(&name),
            "the Rust .so imports {name}, which is part of the translated C code \
             (it must be defined, not imported)"
        );
    }
}

/// Sanity: the exact set of names documented in `SYMBOLS.md`.
#[test]
fn documented_symbol_set_is_complete() {
    let mut expected: BTreeSet<String> = BTreeSet::new();
    for ty in ["int", "double", "item_t", "order_t"] {
        for op in ["create", "destroy", "push", "get", "size", "clear"] {
            expected.insert(format!("array_{ty}_{op}"));
        }
        for op in ["create", "destroy", "append", "prepend", "size", "clear"] {
            expected.insert(format!("list_{ty}_{op}"));
        }
    }
    for f in [
        "print_item",
        "print_order",
        "create_item",
        "create_order",
        "calculate_inventory_stats",
        "calculate_order_stats",
        "find_items_by_category",
        "find_expensive_items",
        "print_menu",
        "demo_integer_containers",
        "demo_double_containers",
        "demo_inventory_array",
        "demo_order_list",
        "demo_mixed_operations",
        "main",
    ] {
        expected.insert(f.to_string());
    }
    assert_eq!(expected.len(), 63, "the documented symbol count is 63");

    let l = libs();
    let c_syms = defined_symbols(&l.c_path);
    let rs_syms = defined_symbols(&l.rs_path);
    assert_eq!(
        c_syms, expected,
        "the C .so's symbol set differs from the documented one"
    );
    assert_eq!(
        rs_syms, expected,
        "the Rust .so's symbol set differs from the documented one"
    );
}
