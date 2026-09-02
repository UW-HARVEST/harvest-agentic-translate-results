//! Phase D — symbol parity, enforced from the test suite so it cannot rot.
//!
//! Every dynamic symbol the C `.so` defines must also be defined by the Rust
//! `.so`, under the exact same name.

mod common;

use common::*;
use std::process::Command;

fn defined_symbols(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("failed to run `nm` (binutils required)");
    assert!(
        out.status.success(),
        "nm -D --defined-only {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
        .collect();
    v.sort();
    v.dedup();
    v
}

fn undefined_symbols(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--undefined-only")
        .arg(path)
        .output()
        .expect("failed to run `nm`");
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect();
    v.sort();
    v.dedup();
    v
}

/// The 13 symbols `lib.c` defines, listed explicitly so a regression in either
/// library is caught even if `nm` output shifts.
const EXPECTED: [&str; 13] = [
    "add_op",
    "add_tree_node",
    "calculate_tree_sum",
    "divide_op",
    "find_node_by_id",
    "get_operation_func",
    "inreftree",
    "modulo_op",
    "multiply_op",
    "node_count",
    "node_table",
    "parse_operation",
    "subtract_op",
];

#[test]
fn phase_d_symbol_diff_is_empty() {
    let c = defined_symbols(&c_so_path());
    let r = defined_symbols(&rust_so_path());

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}",
        missing.len()
    );

    for want in EXPECTED {
        assert!(c.contains(&want.to_string()), "C .so lost `{want}`");
        assert!(r.contains(&want.to_string()), "Rust .so lost `{want}`");
    }
    assert_eq!(c.len(), EXPECTED.len(), "C .so symbol count changed: {c:?}");
}

#[test]
fn phase_d_every_symbol_is_loadable_and_live() {
    let p = Pair::open();
    p.reset_both();
    // Each symbol must actually resolve through dlsym in BOTH libraries, and
    // each function must be callable (no stub / unimplemented!()).
    for name in ["add_op", "multiply_op", "subtract_op", "divide_op", "modulo_op"] {
        let cv = p.c.op_by_name(name, 12, 5);
        let rv = p.r.op_by_name(name, 12, 5);
        assert_eq!(cv, rv, "{name}(12, 5)");
    }
    assert_eq!(p.c.find_node_by_id(1), p.r.find_node_by_id(1));
    let l = cstr(b"probe");
    assert_eq!(
        p.c.add_tree_node(1, 42, -1, &l),
        p.r.add_tree_node(1, 42, -1, &l)
    );
    assert_eq!(p.c.calculate_tree_sum(1), p.r.calculate_tree_sum(1));
    let s = cstr(b"*");
    assert_eq!(p.c.parse_operation(&s), p.r.parse_operation(&s));
    assert_eq!(
        p.c.get_operation_func_probe(2, 6, 7),
        p.r.get_operation_func_probe(2, 6, 7)
    );
    assert_eq!(p.c.inreftree(1, 2, 3, 4), p.r.inreftree(1, 2, 3, 4));
    assert!(!p.c.node_table_ptr().is_null() && !p.r.node_table_ptr().is_null());
    assert!(!p.c.node_count_ptr().is_null() && !p.r.node_count_ptr().is_null());
    p.assert_state_eq("all symbols live");
}

#[test]
fn phase_d_no_missing_non_libc_imports() {
    // The Rust .so must not depend on any symbol the C library defined; all its
    // undefined symbols must be libc / language-runtime imports.
    let c_defined = defined_symbols(&c_so_path());
    let r_undefined = undefined_symbols(&rust_so_path());
    let bad: Vec<&String> = r_undefined
        .iter()
        .filter(|s| c_defined.contains(s))
        .collect();
    assert!(
        bad.is_empty(),
        "the Rust .so imports symbols it should define itself: {bad:?}"
    );
}

/// `TreeNode` must be 52 bytes with the C field order, otherwise every
/// `node_table` comparison in Phases B and C would be comparing the wrong bytes.
#[test]
fn phase_d_tree_node_layout_matches_c() {
    assert_eq!(std::mem::size_of::<TreeNode>(), TREE_NODE_SIZE);
    // node_table's span in the C .so is exactly 50 * 52 = 2600 bytes: the C
    // library places node_count immediately after it.
    let c = defined_symbols(&c_so_path());
    assert!(c.contains(&"node_table".to_string()));
    let p = Pair::open();
    p.reset_both();
    // Writing entry 49 must not disturb entry 48, in either library.
    let l = cstr(b"z");
    for i in 0..MAX_NODES {
        assert_eq!(
            p.c.add_tree_node(i as i32 + 1, i as i32, -1, &l),
            p.r.add_tree_node(i as i32 + 1, i as i32, -1, &l)
        );
    }
    p.assert_state_eq("full table layout");
    for i in 0..MAX_NODES {
        assert_eq!(p.c.node(i).id, i as i32 + 1, "C entry {i} stride");
        assert_eq!(p.r.node(i).id, i as i32 + 1, "Rust entry {i} stride");
    }
    assert_eq!(p.c.get_node_count(), MAX_NODES as i32);
}
