//! Phase C: differential coverage of the branches `main.c` never reaches.
//!
//! `c_src/src/main.c` is a fixed, input-free driver, so the only way to reach the
//! error returns inside `tree.c` and `hashmap.c` is to call them from a second
//! program. This test compiles `tests/cprobe/probe.c` against the *unmodified*
//! `c_src/src/*.c` and runs it against the Rust `probe` binary, one scenario per
//! process, comparing stdout, stderr and the exit status.
//!
//! Both probes are still driven as subprocesses; the Rust crate is never loaded
//! as a library.

mod common;

use common::*;
use std::path::PathBuf;
use std::sync::OnceLock;

/// Kept in the same order as the tables in both probes.
const SCENARIOS: &[&str] = &[
    // tree_print: the `!has_root` early return
    "empty_print",
    // tree_add_node: the `data == NULL` branch
    "null_data",
    // strncpy(dst, src, MAX_DATA_LENGTH - 1) truncation, on both sides of 255
    "data_lengths",
    // tree_add_node: "Parent node %lu not found"
    "parent_missing",
    // tree_add_node: "Node with ID %lu already exists"
    "duplicate_ids",
    // tree_remove_node: "Node %lu not found", empty and populated
    "remove_missing",
    // tree_get_depth / get_height / count_descendants / find_path: absent node
    "queries_missing",
    // tree_find_path: max_length clamping, including 0 and negative
    "path_bounds",
    // root removal empties the tree; the next add becomes the new root
    "remove_root_then_add",
    // MAX_CHILDREN boundary and refill after freeing a slot
    "max_children",
    // the child-list shift, removing from the front, middle and back
    "remove_child_positions",
    // recursive subtree removal, then removing already-removed ids
    "subtree_cascade",
    // node id 0 as root, which collides with the "no root" sentinel
    "id_zero",
    // FNV-1a and %lu over extreme uint64 keys
    "big_ids",
    // chain longer than find_path's fixed 1000-entry scratch array
    "deep_chain",
    // hashmap_clear, which resets flags but leaves keys and values behind
    "clear_map",
    // tombstone reuse in hashmap_put, including the duplicate-key consequence
    "tombstones",
    // linear probing across a real hash collision, then the duplicate key that
    // tombstone reuse produces, and the rehash that has to survive it
    "collision_probing",
    // should_resize / hashmap_resize growth and tombstone compaction
    "resize_map",
    // long pseudo-random put/get/remove mix, dumping every slot
    "stress_map",
    // long pseudo-random add/remove/query mix over the tree
    "stress_tree",
];

fn rust_probe() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_probe"))
}

fn c_probe() -> PathBuf {
    // Compile once per test binary; the tests below run in parallel.
    static ONCE: OnceLock<PathBuf> = OnceLock::new();
    ONCE.get_or_init(|| {
        let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/cprobe/probe.c");
        compile_c_aux(&src, "cprobe")
    })
    .clone()
}

fn check(scenario: &str, c: &PathBuf, r: &PathBuf) {
    let cc = capture(c, &[scenario], None);
    let rr = capture(r, &[scenario], None);
    assert_same(scenario, &cc, &rr);
    assert_eq!(cc.code, Some(0), "scenario `{scenario}` should exit 0 in C");
    assert!(
        !cc.stdout.is_empty(),
        "scenario `{scenario}` produced no stdout in C, so it verifies nothing"
    );
}

#[test]
fn every_library_branch_matches() {
    let c = c_probe();
    let r = rust_probe();
    for scenario in SCENARIOS {
        check(scenario, &c, &r);
    }
}

#[test]
fn unknown_scenario_and_missing_argument_match() {
    let c = c_probe();
    let r = rust_probe();

    let cc = capture(&c, &[], None);
    let rr = capture(&r, &[], None);
    assert_same("no scenario argument", &cc, &rr);
    assert_eq!(cc.code, Some(2));

    let cc = capture(&c, &["nope"], None);
    let rr = capture(&r, &["nope"], None);
    assert_same("unknown scenario", &cc, &rr);
    assert_eq!(cc.code, Some(3));
}

#[test]
fn scenario_tables_agree_between_the_two_probes() {
    // A scenario that exists in only one probe would silently reduce coverage:
    // the C side would exit 3 and the Rust side would run something. Both probes
    // reject unknown names, so asking each of them for every name in the list is
    // enough to prove the tables line up.
    let c = c_probe();
    let r = rust_probe();
    for scenario in SCENARIOS {
        let cc = capture(&c, &[scenario], None);
        let rr = capture(&r, &[scenario], None);
        assert_eq!(
            cc.code,
            Some(0),
            "C probe does not know scenario `{scenario}`"
        );
        assert_eq!(
            rr.code,
            Some(0),
            "Rust probe does not know scenario `{scenario}`"
        );
    }
}
