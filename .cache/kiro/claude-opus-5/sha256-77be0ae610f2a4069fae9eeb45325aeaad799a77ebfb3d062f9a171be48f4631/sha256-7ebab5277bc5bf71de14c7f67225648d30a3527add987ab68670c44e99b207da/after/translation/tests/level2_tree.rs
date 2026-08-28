//! Level 2: the stateful tree table — `find_node_by_id`, `add_tree_node`,
//! `calculate_tree_sum` — plus the `node_table` / `node_count` globals.
//!
//! Each scenario zeroes both libraries' globals first, replays the identical
//! call sequence against each, and then compares return values, `node_count`,
//! and the raw bytes of the entire 50-entry `node_table`.

mod common;

use common::*;
use std::ffi::c_int;

/// Index of a returned node pointer within that library's own `node_table`,
/// or `None` for NULL. This makes pointers from the two libraries comparable.
fn slot(lib: &Lib, p: *mut TreeNode) -> Option<isize> {
    if p.is_null() {
        return None;
    }
    let base = lib.node_table_ptr();
    let diff = (p as isize - base as isize) / std::mem::size_of::<TreeNode>() as isize;
    Some(diff)
}

fn assert_state_matches(p: &Pair, ctx: &str) {
    assert_eq!(
        p.c.node_count(),
        p.rs.node_count(),
        "{ctx}: node_count diverged"
    );

    let cb = p.c.node_table_bytes();
    let rb = p.rs.node_table_bytes();
    if cb != rb {
        let ct = p.c.node_table();
        let rt = p.rs.node_table();
        for i in 0..MAX_NODES {
            assert_eq!(ct[i], rt[i], "{ctx}: node_table[{i}] diverged");
        }
        panic!("{ctx}: node_table bytes differ but no single entry did");
    }
}

/// Replay a sequence of `add_tree_node` calls against both libraries from a
/// zeroed state, checking return values and the table after every step.
fn replay(p: &Pair, ctx: &str, calls: &[(c_int, c_int, c_int, &[u8])]) {
    p.c.reset();
    p.rs.reset();
    assert_state_matches(p, &format!("{ctx}: after reset"));

    for (step, &(id, value, parent, label)) in calls.iter().enumerate() {
        let cr = p.c.add_tree_node(id, value, parent, label);
        let rr = p.rs.add_tree_node(id, value, parent, label);
        assert_eq!(
            cr, rr,
            "{ctx}: step {step} add_tree_node({id}, {value}, {parent}, {label:?}) return value"
        );
        assert_state_matches(p, &format!("{ctx}: after step {step}"));
    }
}

#[test]
fn find_node_by_id_on_empty_table() {
    let p = load();
    p.c.reset();
    p.rs.reset();
    for id in [-2, -1, 0, 1, 2, 50, c_int::MAX, c_int::MIN] {
        let c = slot(&p.c, p.c.find_node_by_id(id));
        let r = slot(&p.rs, p.rs.find_node_by_id(id));
        assert_eq!(c, r, "find_node_by_id({id}) on empty table");
        assert_eq!(c, None, "empty table must not match id {id}");
    }
}

#[test]
fn add_tree_node_basic_tree() {
    let p = load();
    replay(
        &p,
        "basic tree",
        &[
            (1, 10, -1, b"root\0"),
            (2, 20, 1, b"left\0"),
            (3, 30, 1, b"right\0"),
            (4, 40, 2, b"left-left\0"),
        ],
    );

    // Lookups over the populated table.
    for id in -3..=8 {
        let c = slot(&p.c, p.c.find_node_by_id(id));
        let r = slot(&p.rs, p.rs.find_node_by_id(id));
        assert_eq!(c, r, "find_node_by_id({id}) on populated table");
    }

    for id in -3..=8 {
        assert_eq!(
            p.c.calculate_tree_sum(id),
            p.rs.calculate_tree_sum(id),
            "calculate_tree_sum({id})"
        );
    }
}

#[test]
fn add_tree_node_rejects_missing_parent() {
    let p = load();
    // A parent id that is not in the table makes the C code return -1 *after*
    // it has already written the node into node_table[node_count] and without
    // incrementing node_count. That partial write must be reproduced.
    replay(
        &p,
        "missing parent",
        &[
            (1, 5, -1, b"root\0"),
            (2, 7, 99, b"orphan\0"),
            (3, 9, 1, b"real-child\0"),
            (4, 11, 42, b"orphan2\0"),
            (5, 13, 1, b"second-child\0"),
            (6, 15, 1, b"third-child\0"),
        ],
    );
}

#[test]
fn add_tree_node_third_child_is_dropped() {
    let p = load();
    // Once both child slots on the parent are taken, further children are
    // recorded in the table but never linked from the parent.
    replay(
        &p,
        "third child",
        &[
            (1, 1, -1, b"root\0"),
            (2, 2, 1, b"c1\0"),
            (3, 4, 1, b"c2\0"),
            (4, 8, 1, b"c3-unlinked\0"),
            (5, 16, 1, b"c4-unlinked\0"),
        ],
    );
    for id in 0..=6 {
        assert_eq!(
            p.c.calculate_tree_sum(id),
            p.rs.calculate_tree_sum(id),
            "calculate_tree_sum({id}) with unlinked children"
        );
    }
}

#[test]
fn add_tree_node_duplicate_ids() {
    let p = load();
    // find_node_by_id returns the *first* match, so duplicates shadow later
    // entries and parent linking targets the earlier node.
    replay(
        &p,
        "duplicate ids",
        &[
            (1, 100, -1, b"root\0"),
            (1, 200, -1, b"root-dup\0"),
            (2, 300, 1, b"child-of-first\0"),
            (3, 400, 1, b"child2-of-first\0"),
            (4, 500, 1, b"child3\0"),
        ],
    );
    for id in 0..=5 {
        assert_eq!(
            p.c.calculate_tree_sum(id),
            p.rs.calculate_tree_sum(id),
            "calculate_tree_sum({id}) with duplicate ids"
        );
    }
}

#[test]
fn add_tree_node_negative_and_extreme_ids() {
    let p = load();
    replay(
        &p,
        "extreme ids",
        &[
            (0, 1, -1, b"zero\0"),
            (-1, 2, -1, b"neg-one-id\0"),
            (c_int::MAX, 3, 0, b"max\0"),
            (c_int::MIN, 4, 0, b"min\0"),
            (-5, 5, c_int::MAX, b"child-of-max\0"),
            (-6, 6, c_int::MIN, b"child-of-min\0"),
        ],
    );
    for id in [0, -1, -5, -6, c_int::MAX, c_int::MIN, 7] {
        assert_eq!(
            p.c.calculate_tree_sum(id),
            p.rs.calculate_tree_sum(id),
            "calculate_tree_sum({id})"
        );
    }
}

#[test]
fn add_tree_node_label_truncation_and_padding() {
    let p = load();
    // strncpy(dst, src, 31) zero-pads short sources and does not terminate
    // long ones; the explicit label[31] = 0 then caps it.
    let long = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEF\0"; // 42 chars
    let exactly31 = b"0123456789012345678901234567890\0"; // 31 chars
    let exactly32 = b"01234567890123456789012345678901\0"; // 32 chars
    replay(
        &p,
        "labels",
        &[
            (1, 1, -1, long),
            (2, 2, 1, exactly31),
            (3, 3, 1, exactly32),
            (4, 4, 2, b"\0"),
            (5, 5, 2, b"a\0"),
        ],
    );

    // Pre-dirty the tables identically, then confirm zero-padding really wipes
    // the stale bytes in both implementations.
    p.c.reset();
    p.rs.reset();
    for lib in [&p.c, &p.rs] {
        let base = lib.node_table_ptr() as *mut u8;
        let len = MAX_NODES * std::mem::size_of::<TreeNode>();
        unsafe { std::ptr::write_bytes(base, 0xAB, len) };
        lib.set_node_count(0);
    }
    assert_state_matches(&p, "labels: after dirtying");
    for &(id, v, parent, label) in &[
        (1i32, 1i32, -1i32, b"root\0".as_slice()),
        (2, 2, 1, b"x\0".as_slice()),
    ] {
        assert_eq!(
            p.c.add_tree_node(id, v, parent, label),
            p.rs.add_tree_node(id, v, parent, label)
        );
        assert_state_matches(&p, "labels: dirty table write");
    }
}

#[test]
fn add_tree_node_fills_and_overflows_table() {
    let p = load();
    // 50 nodes exactly fill the table; the 51st and beyond must return -1
    // without touching anything.
    let mut labels: Vec<Vec<u8>> = Vec::new();
    for i in 0..55 {
        let mut l = format!("node-{i}").into_bytes();
        l.push(0);
        labels.push(l);
    }
    let mut calls: Vec<(c_int, c_int, c_int, &[u8])> = Vec::new();
    for i in 0..55i32 {
        // Binary-heap parent of id (i+1), so every parent already exists.
        let parent = if i == 0 { -1 } else { (i + 1) / 2 };
        calls.push((i + 1, i * 3 - 7, parent, labels[i as usize].as_slice()));
    }
    replay(&p, "overflow", &calls);

    assert_eq!(p.c.node_count(), MAX_NODES as c_int, "table should be full");
    for id in [1, 2, 3, 25, 50, 51, 55] {
        assert_eq!(
            p.c.calculate_tree_sum(id),
            p.rs.calculate_tree_sum(id),
            "calculate_tree_sum({id}) on full table"
        );
    }
}

#[test]
fn calculate_tree_sum_overflow_wraps_identically() {
    let p = load();
    replay(
        &p,
        "sum overflow",
        &[
            (1, c_int::MAX, -1, b"root\0"),
            (2, c_int::MAX, 1, b"a\0"),
            (3, c_int::MAX, 1, b"b\0"),
            (4, c_int::MIN, 2, b"c\0"),
            (5, c_int::MIN, 2, b"d\0"),
        ],
    );
    for id in 1..=5 {
        assert_eq!(
            p.c.calculate_tree_sum(id),
            p.rs.calculate_tree_sum(id),
            "calculate_tree_sum({id}) with overflowing values"
        );
    }
}

#[test]
fn calculate_tree_sum_hand_built_topologies() {
    let p = load();
    // Write the table directly (bypassing add_tree_node) to reach shapes the
    // builder cannot produce: deep chains, shared subtrees, dangling links.
    let scenarios: Vec<(&str, Vec<TreeNode>)> = vec![
        ("dangling children", {
            let mk = |id, value, l, r| TreeNode {
                id,
                value,
                parent_id: -1,
                left_child_id: l,
                right_child_id: r,
                label: [0; 32],
            };
            vec![mk(1, 5, 77, 88), mk(2, 6, -1, -1)]
        }),
        ("shared subtree counted twice", {
            let mk = |id, value, l, r| TreeNode {
                id,
                value,
                parent_id: -1,
                left_child_id: l,
                right_child_id: r,
                label: [0; 32],
            };
            vec![mk(1, 1, 2, 2), mk(2, 10, 3, 3), mk(3, 100, -1, -1)]
        }),
        ("deep left chain", {
            let mut v = Vec::new();
            for i in 0..40i32 {
                v.push(TreeNode {
                    id: i + 1,
                    value: i * 7 - 13,
                    parent_id: -1,
                    left_child_id: if i == 39 { -1 } else { i + 2 },
                    right_child_id: -1,
                    label: [0; 32],
                });
            }
            v
        }),
        ("child id zero is followed", {
            let mk = |id, value, l, r| TreeNode {
                id,
                value,
                parent_id: -1,
                left_child_id: l,
                right_child_id: r,
                label: [0; 32],
            };
            vec![mk(1, 3, 0, -1), mk(0, 9, -1, -1)]
        }),
    ];

    for (ctx, nodes) in scenarios {
        for lib in [&p.c, &p.rs] {
            lib.reset();
            let base = lib.node_table_ptr();
            for (i, n) in nodes.iter().enumerate() {
                unsafe { std::ptr::write(base.add(i), *n) };
            }
            lib.set_node_count(nodes.len() as c_int);
        }
        assert_state_matches(&p, ctx);
        for id in -1..=5 {
            assert_eq!(
                p.c.calculate_tree_sum(id),
                p.rs.calculate_tree_sum(id),
                "{ctx}: calculate_tree_sum({id})"
            );
        }
        assert_state_matches(&p, &format!("{ctx}: after sums"));
    }
}

#[test]
fn find_node_by_id_respects_node_count_not_contents() {
    let p = load();
    // Entries beyond node_count are invisible even when populated.
    for lib in [&p.c, &p.rs] {
        lib.reset();
        let base = lib.node_table_ptr();
        for i in 0..10usize {
            unsafe {
                std::ptr::write(
                    base.add(i),
                    TreeNode {
                        id: 100 + i as c_int,
                        value: i as c_int,
                        parent_id: -1,
                        left_child_id: -1,
                        right_child_id: -1,
                        label: [0; 32],
                    },
                )
            };
        }
        lib.set_node_count(3);
    }
    for id in 98..=112 {
        let c = slot(&p.c, p.c.find_node_by_id(id));
        let r = slot(&p.rs, p.rs.find_node_by_id(id));
        assert_eq!(c, r, "find_node_by_id({id}) with node_count = 3");
    }
    // A negative node_count must make the scan loop zero times, not wrap.
    for count in [-1, -100, c_int::MIN, 0] {
        p.c.set_node_count(count);
        p.rs.set_node_count(count);
        for id in [100, 101, 0, -1] {
            let c = slot(&p.c, p.c.find_node_by_id(id));
            let r = slot(&p.rs, p.rs.find_node_by_id(id));
            assert_eq!(c, r, "find_node_by_id({id}) with node_count = {count}");
            assert_eq!(c, None, "negative/zero count must find nothing");
        }
        assert_eq!(
            p.c.calculate_tree_sum(100),
            p.rs.calculate_tree_sum(100),
            "calculate_tree_sum with node_count = {count}"
        );
    }
}

#[test]
fn add_tree_node_with_full_count_returns_minus_one() {
    let p = load();
    // node_count at or above MAX_NODES short-circuits before any write.
    for count in [MAX_NODES as c_int, MAX_NODES as c_int + 1, 1000, c_int::MAX] {
        p.c.reset();
        p.rs.reset();
        p.c.set_node_count(count);
        p.rs.set_node_count(count);
        let cr = p.c.add_tree_node(9, 9, -1, b"nope\0");
        let rr = p.rs.add_tree_node(9, 9, -1, b"nope\0");
        assert_eq!(cr, rr, "add_tree_node with node_count = {count}");
        assert_eq!(cr, -1);
        assert_state_matches(&p, &format!("full count {count}"));
    }
}
