//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Both implementations are reached exclusively through their `.so` exports.
//! Every row uses many randomized inputs from a fixed-seed PRNG.

mod common;

use common::*;
use std::os::raw::c_int;

const N: usize = 20_000;

// ===========================================================================
// Rows 1-11 — the five arithmetic ops (lowest-level entry points)
// ===========================================================================

#[test]
fn cfg_01_add_op_random() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED);
    for _ in 0..N {
        let (a, b) = (rng.next_i32_mixed(), rng.next_i32_mixed());
        assert_eq!(
            p.c.add_op(a, b, 0, 0),
            p.r.add_op(a, b, 0, 0),
            "add_op({a}, {b})"
        );
    }
}

#[test]
fn cfg_02_add_op_boundary_grid() {
    let p = Pair::open();
    for &a in EDGE_I32.iter() {
        for &b in EDGE_I32.iter() {
            assert_eq!(
                p.c.add_op(a, b, 0, 0),
                p.r.add_op(a, b, 0, 0),
                "add_op({a}, {b})"
            );
        }
    }
}

#[test]
fn cfg_03_add_op_ignores_unused_args() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..N {
        let (a, b) = (rng.next_i32_mixed(), rng.next_i32_mixed());
        let (u1, u2) = (rng.next_i32_mixed(), rng.next_i32_mixed());
        let cv = p.c.add_op(a, b, u1, u2);
        let rv = p.r.add_op(a, b, u1, u2);
        assert_eq!(cv, rv, "add_op({a}, {b}, {u1}, {u2})");
        // and the unused args really are ignored, in both
        assert_eq!(cv, p.c.add_op(a, b, 0, 0), "C add_op used unused args");
        assert_eq!(rv, p.r.add_op(a, b, 0, 0), "Rust add_op used unused args");
    }
}

#[test]
fn cfg_04_multiply_op_random() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..N {
        let (a, b) = (rng.next_i32_mixed(), rng.next_i32_mixed());
        assert_eq!(
            p.c.multiply_op(a, b, 0, 0),
            p.r.multiply_op(a, b, 0, 0),
            "multiply_op({a}, {b})"
        );
    }
}

#[test]
fn cfg_05_multiply_op_boundary_grid() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 5);
    for &a in EDGE_I32.iter() {
        for &b in EDGE_I32.iter() {
            assert_eq!(
                p.c.multiply_op(a, b, 0, 0),
                p.r.multiply_op(a, b, 0, 0),
                "multiply_op({a}, {b})"
            );
        }
    }
    // large-magnitude pairs that certainly overflow
    for _ in 0..N {
        let a = rng.next_i32() | 0x4000_0000;
        let b = rng.next_i32() | 0x4000_0000;
        assert_eq!(
            p.c.multiply_op(a, b, 0, 0),
            p.r.multiply_op(a, b, 0, 0),
            "multiply_op({a}, {b}) overflow"
        );
    }
}

#[test]
fn cfg_06_subtract_op_random() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..N {
        let (a, b) = (rng.next_i32_mixed(), rng.next_i32_mixed());
        assert_eq!(
            p.c.subtract_op(a, b, 0, 0),
            p.r.subtract_op(a, b, 0, 0),
            "subtract_op({a}, {b})"
        );
    }
}

#[test]
fn cfg_07_subtract_op_boundary_grid() {
    let p = Pair::open();
    for &a in EDGE_I32.iter() {
        for &b in EDGE_I32.iter() {
            assert_eq!(
                p.c.subtract_op(a, b, 0, 0),
                p.r.subtract_op(a, b, 0, 0),
                "subtract_op({a}, {b})"
            );
        }
    }
    // 0 - INT_MIN and INT_MIN - 1 explicitly
    for (a, b) in [(0, i32::MIN), (i32::MIN, 1), (i32::MAX, -1)] {
        assert_eq!(
            p.c.subtract_op(a, b, 0, 0),
            p.r.subtract_op(a, b, 0, 0),
            "subtract_op({a}, {b})"
        );
    }
}

/// `(INT_MIN, -1)` is excluded here: it faults in BOTH libraries by design.
/// See `ERRORS.md` rows 3-4 and `phase_c_errors.rs`.
fn div_safe(a: c_int, b: c_int) -> bool {
    b != 0 && !(a == i32::MIN && b == -1)
}

#[test]
fn cfg_08_divide_op_random() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 8);
    let mut n = 0;
    while n < N {
        let (a, b) = (rng.next_i32_mixed(), rng.next_i32_mixed());
        if !div_safe(a, b) {
            continue;
        }
        assert_eq!(
            p.c.divide_op(a, b, 0, 0),
            p.r.divide_op(a, b, 0, 0),
            "divide_op({a}, {b})"
        );
        n += 1;
    }
}

#[test]
fn cfg_09_divide_op_boundary_grid() {
    let p = Pair::open();
    for &a in EDGE_I32.iter() {
        for &b in EDGE_I32.iter() {
            if !div_safe(a, b) {
                continue;
            }
            assert_eq!(
                p.c.divide_op(a, b, 0, 0),
                p.r.divide_op(a, b, 0, 0),
                "divide_op({a}, {b})"
            );
        }
    }
    // |a| < |b| (truncates to 0), and exact multiples, both signs
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..N {
        let b = loop {
            let v = rng.next_i32_mixed();
            if v != 0 && v != -1 && v != 1 {
                break v;
            }
        };
        let small = (rng.next_u32() % (b.unsigned_abs())) as i32;
        let a = if rng.next_u32() & 1 == 0 { small } else { -small };
        assert_eq!(
            p.c.divide_op(a, b, 0, 0),
            p.r.divide_op(a, b, 0, 0),
            "divide_op({a}, {b}) small-dividend"
        );
        let k = (rng.next_u32() % 1000) as i32;
        let m = b.wrapping_mul(k);
        if div_safe(m, b) {
            assert_eq!(
                p.c.divide_op(m, b, 0, 0),
                p.r.divide_op(m, b, 0, 0),
                "divide_op({m}, {b}) exact"
            );
        }
    }
}

#[test]
fn cfg_10_modulo_op_random() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 10);
    let mut n = 0;
    while n < N {
        let (a, b) = (rng.next_i32_mixed(), rng.next_i32_mixed());
        if !div_safe(a, b) {
            continue;
        }
        assert_eq!(
            p.c.modulo_op(a, b, 0, 0),
            p.r.modulo_op(a, b, 0, 0),
            "modulo_op({a}, {b})"
        );
        n += 1;
    }
}

#[test]
fn cfg_11_modulo_op_boundary_grid() {
    let p = Pair::open();
    for &a in EDGE_I32.iter() {
        for &b in EDGE_I32.iter() {
            if !div_safe(a, b) {
                continue;
            }
            assert_eq!(
                p.c.modulo_op(a, b, 0, 0),
                p.r.modulo_op(a, b, 0, 0),
                "modulo_op({a}, {b})"
            );
        }
    }
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..N {
        let b = loop {
            let v = rng.next_i32_mixed();
            if v != 0 && v != -1 {
                break v;
            }
        };
        let k = rng.next_i32_mixed();
        let m = b.wrapping_mul(k);
        if div_safe(m, b) {
            assert_eq!(
                p.c.modulo_op(m, b, 0, 0),
                p.r.modulo_op(m, b, 0, 0),
                "modulo_op({m}, {b}) exact multiple"
            );
        }
        // negative dividend keeps its sign in C
        let a = -((rng.next_u32() % 10_000) as i32);
        assert_eq!(
            p.c.modulo_op(a, b, 0, 0),
            p.r.modulo_op(a, b, 0, 0),
            "modulo_op({a}, {b}) negative dividend"
        );
    }
}

// ===========================================================================
// Rows 12-16 — find_node_by_id
// ===========================================================================

/// Fill both tables with the same randomized entries, directly through the
/// exported `node_table` object (state setup a real consumer can do).
fn seed_table(p: &Pair, rng: &mut Rng, count: usize, dup_ids: bool) -> Vec<TreeNode> {
    let mut nodes = Vec::new();
    for i in 0..MAX_NODES {
        let id = if i < count {
            if dup_ids {
                (rng.below(5) as c_int) + 1
            } else {
                i as c_int + 1
            }
        } else {
            rng.next_i32_mixed()
        };
        let mut label = [0i8; 32];
        let len = rng.below(31) as usize;
        for k in 0..len {
            label[k] = b"abcdefgl+-*/%"[rng.below(13) as usize] as i8;
        }
        nodes.push(TreeNode {
            id,
            value: rng.next_i32_mixed(),
            parent_id: rng.next_i32_mixed(),
            left_child_id: -1,
            right_child_id: -1,
            label,
        });
    }
    for (i, n) in nodes.iter().enumerate() {
        p.c.set_node(i, n);
        p.r.set_node(i, n);
    }
    p.c.set_node_count(count as c_int);
    p.r.set_node_count(count as c_int);
    p.assert_state_eq("seed_table");
    nodes
}

#[test]
fn cfg_12_find_node_empty_table() {
    let p = Pair::open();
    p.reset_both();
    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..N {
        let id = rng.next_i32_mixed();
        assert_eq!(
            p.c.find_node_by_id(id),
            p.r.find_node_by_id(id),
            "find_node_by_id({id}) on empty table"
        );
    }
}

#[test]
fn cfg_13_find_node_single_entry() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..2_000 {
        p.reset_both();
        seed_table(&p, &mut rng, 1, false);
        for id in [1, 0, -1, 2, rng.next_i32_mixed()] {
            assert_eq!(
                p.c.find_node_by_id(id),
                p.r.find_node_by_id(id),
                "find_node_by_id({id}) with node_count=1"
            );
        }
    }
}

#[test]
fn cfg_14_find_node_full_table_unique_ids() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 14);
    for _ in 0..500 {
        p.reset_both();
        seed_table(&p, &mut rng, MAX_NODES, false);
        // every id present (indices 0, mid, 49 included) plus misses
        for id in 1..=(MAX_NODES as c_int) {
            assert_eq!(
                p.c.find_node_by_id(id),
                p.r.find_node_by_id(id),
                "find_node_by_id({id}) full table"
            );
        }
        for id in [0, -1, 51, i32::MIN, i32::MAX, rng.next_i32_mixed()] {
            assert_eq!(
                p.c.find_node_by_id(id),
                p.r.find_node_by_id(id),
                "find_node_by_id({id}) full-table miss"
            );
        }
    }
}

#[test]
fn cfg_15_find_node_duplicate_ids_returns_first() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..2_000 {
        p.reset_both();
        let count = (rng.below(MAX_NODES as u32) + 1) as usize;
        let nodes = seed_table(&p, &mut rng, count, true);
        for id in 1..=6 {
            let cv = p.c.find_node_by_id(id);
            let rv = p.r.find_node_by_id(id);
            assert_eq!(cv, rv, "find_node_by_id({id}) with duplicate ids");
            // and it really is the first match
            let expect = nodes[..count].iter().position(|n| n.id == id).map(|i| i as isize);
            assert_eq!(cv, expect, "find_node_by_id({id}) did not return first match");
        }
    }
}

#[test]
fn cfg_16_find_node_count_truncated_below_contents() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 16);
    for _ in 0..2_000 {
        p.reset_both();
        seed_table(&p, &mut rng, MAX_NODES, false);
        let visible = rng.below(MAX_NODES as u32 + 1) as c_int;
        p.c.set_node_count(visible);
        p.r.set_node_count(visible);
        for id in 1..=(MAX_NODES as c_int) {
            assert_eq!(
                p.c.find_node_by_id(id),
                p.r.find_node_by_id(id),
                "find_node_by_id({id}) with node_count truncated to {visible}"
            );
        }
    }
}

// ===========================================================================
// Rows 17-25 — add_tree_node
// ===========================================================================

fn rand_label(rng: &mut Rng, len: usize) -> Vec<u8> {
    const ALPHA: &[u8] = b"abcdefghijkLMNopqrstuvwxyz-+*/%0123456789";
    let mut v: Vec<u8> = (0..len).map(|_| ALPHA[rng.below(ALPHA.len() as u32) as usize]).collect();
    v.push(0);
    v
}

/// Random label of a random length in `0..max`.
fn rand_label_upto(rng: &mut Rng, max: u32) -> Vec<u8> {
    let len = rng.below(max) as usize;
    rand_label(rng, len)
}

#[test]
fn cfg_17_add_root_nodes_sequential() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 17);
    for round in 0..400 {
        p.reset_both();
        for i in 0..MAX_NODES {
            let id = rng.next_i32_mixed();
            let value = rng.next_i32_mixed();
            let label = rand_label_upto(&mut rng, 40);
            let cv = p.c.add_tree_node(id, value, -1, &label);
            let rv = p.r.add_tree_node(id, value, -1, &label);
            assert_eq!(cv, rv, "round {round} add_tree_node #{i} return");
            p.assert_state_eq(&format!("round {round} after add #{i}"));
        }
    }
}

#[test]
fn cfg_18_add_child_fills_left_slot() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 18);
    for _ in 0..2_000 {
        p.reset_both();
        let root_id = (rng.below(100) as c_int) + 1;
        let l1 = rand_label(&mut rng, 6);
        let v0 = rng.next_i32_mixed();
        assert_eq!(
            p.c.add_tree_node(root_id, v0, -1, &l1),
            p.r.add_tree_node(root_id, v0, -1, &l1)
        );
        let child_id = root_id + 1000;
        let v1 = rng.next_i32_mixed();
        let l2 = rand_label(&mut rng, 8);
        let cv = p.c.add_tree_node(child_id, v1, root_id, &l2);
        let rv = p.r.add_tree_node(child_id, v1, root_id, &l2);
        assert_eq!(cv, rv, "add child return");
        p.assert_state_eq("left slot filled");
        assert_eq!(p.c.node(0).left_child_id, child_id, "C left slot");
        assert_eq!(p.r.node(0).left_child_id, child_id, "Rust left slot");
        assert_eq!(p.c.node(0).right_child_id, -1);
    }
}

#[test]
fn cfg_19_add_child_fills_right_slot() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 19);
    for _ in 0..2_000 {
        p.reset_both();
        let root_id = (rng.below(100) as c_int) + 1;
        let (v0, v1, v2) = (rng.next_i32_mixed(), rng.next_i32_mixed(), rng.next_i32_mixed());
        let l = rand_label(&mut rng, 5);
        for (id, v, par) in [
            (root_id, v0, -1),
            (root_id + 1, v1, root_id),
            (root_id + 2, v2, root_id),
        ] {
            let cv = p.c.add_tree_node(id, v, par, &l);
            let rv = p.r.add_tree_node(id, v, par, &l);
            assert_eq!(cv, rv, "add_tree_node({id}, {v}, {par})");
        }
        p.assert_state_eq("right slot filled");
        assert_eq!(p.c.node(0).left_child_id, root_id + 1);
        assert_eq!(p.c.node(0).right_child_id, root_id + 2);
        assert_eq!(p.r.node(0).left_child_id, root_id + 1);
        assert_eq!(p.r.node(0).right_child_id, root_id + 2);
    }
}

#[test]
fn cfg_20_add_third_child_succeeds_without_linking() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 20);
    for _ in 0..2_000 {
        p.reset_both();
        let l = rand_label(&mut rng, 4);
        let vs: Vec<c_int> = (0..5).map(|_| rng.next_i32_mixed()).collect();
        for (k, &v) in vs.iter().enumerate() {
            let (id, par) = if k == 0 { (1, -1) } else { (k as c_int + 1, 1) };
            let cv = p.c.add_tree_node(id, v, par, &l);
            let rv = p.r.add_tree_node(id, v, par, &l);
            assert_eq!(cv, rv, "add_tree_node({id}, {v}, {par})");
        }
        p.assert_state_eq("3rd/4th child not linked");
        // slots stay at the first two children in both
        assert_eq!((p.c.node(0).left_child_id, p.c.node(0).right_child_id), (2, 3));
        assert_eq!((p.r.node(0).left_child_id, p.r.node(0).right_child_id), (2, 3));
        assert_eq!(p.c.get_node_count(), 5);
    }
}

#[test]
fn cfg_21_add_child_of_duplicate_parent_links_first() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 21);
    for _ in 0..2_000 {
        p.reset_both();
        let l = rand_label(&mut rng, 7);
        let (v0, v1, v2) = (rng.next_i32_mixed(), rng.next_i32_mixed(), rng.next_i32_mixed());
        // two nodes with the SAME id 7
        for v in [v0, v1] {
            assert_eq!(
                p.c.add_tree_node(7, v, -1, &l),
                p.r.add_tree_node(7, v, -1, &l)
            );
        }
        let cv = p.c.add_tree_node(9, v2, 7, &l);
        let rv = p.r.add_tree_node(9, v2, 7, &l);
        assert_eq!(cv, rv);
        p.assert_state_eq("duplicate parent");
        assert_eq!(p.c.node(0).left_child_id, 9, "C linked under first dup");
        assert_eq!(p.c.node(1).left_child_id, -1, "C left second dup alone");
        assert_eq!(p.r.node(0).left_child_id, 9, "Rust linked under first dup");
        assert_eq!(p.r.node(1).left_child_id, -1, "Rust left second dup alone");
    }
}

#[test]
fn cfg_22_add_label_length_boundaries() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 22);
    for len in [0usize, 1, 2, 4, 29, 30, 31, 32, 33, 40, 64, 100] {
        for _ in 0..200 {
            p.reset_both();
            let label = rand_label(&mut rng, len);
            let v = rng.next_i32_mixed();
            let cv = p.c.add_tree_node(5, v, -1, &label);
            let rv = p.r.add_tree_node(5, v, -1, &label);
            assert_eq!(cv, rv, "add_tree_node label len {len}");
            p.assert_state_eq(&format!("label len {len}"));
            // and the truncation rule really is 31 bytes + forced NUL
            let cn = p.c.node(0);
            assert_eq!(cn.label[31], 0, "label[31] must be forced to NUL");
            let want = std::cmp::min(len, 31);
            for k in 0..want {
                assert_eq!(cn.label[k] as u8, label[k], "label byte {k} (len {len})");
            }
            for k in want..32 {
                assert_eq!(cn.label[k], 0, "label byte {k} must be zero-padded (len {len})");
            }
        }
    }
}

#[test]
fn cfg_23_shorter_label_scrubs_stale_tail() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 23);
    for _ in 0..2_000 {
        p.reset_both();
        // long label first, then reuse the same slot with a short one by
        // resetting node_count (the pattern inreftree itself relies on)
        let long = rand_label(&mut rng, 31);
        let short = rand_label_upto(&mut rng, 6);
        let v = rng.next_i32_mixed();
        assert_eq!(
            p.c.add_tree_node(1, v, -1, &long),
            p.r.add_tree_node(1, v, -1, &long)
        );
        p.c.set_node_count(0);
        p.r.set_node_count(0);
        assert_eq!(
            p.c.add_tree_node(2, v, -1, &short),
            p.r.add_tree_node(2, v, -1, &short)
        );
        p.assert_state_eq("stale tail scrubbed");
        let cn = p.c.node(0);
        for k in (short.len() - 1)..32 {
            assert_eq!(cn.label[k], 0, "stale byte {k} not scrubbed");
        }
    }
}

#[test]
fn cfg_24_fill_table_to_capacity_comparing_full_image() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 24);
    for round in 0..300 {
        p.reset_both();
        for i in 0..MAX_NODES {
            let id = i as c_int + 1;
            let value = rng.next_i32_mixed();
            let label = rand_label_upto(&mut rng, 35);
            // chain each node under a random already-present parent
            let parent = if i == 0 { -1 } else { (rng.below(i as u32) as c_int) + 1 };
            let cv = p.c.add_tree_node(id, value, parent, &label);
            let rv = p.r.add_tree_node(id, value, parent, &label);
            assert_eq!(cv, rv, "round {round} node {i} return");
            p.assert_state_eq(&format!("round {round} node {i}"));
        }
        assert_eq!(p.c.get_node_count(), MAX_NODES as c_int);
    }
}

#[test]
fn cfg_25_add_root_with_existing_duplicate_id() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 25);
    for _ in 0..2_000 {
        p.reset_both();
        let l = rand_label(&mut rng, 6);
        let id = rng.next_i32_mixed();
        for _ in 0..4 {
            let v = rng.next_i32_mixed();
            assert_eq!(
                p.c.add_tree_node(id, v, -1, &l),
                p.r.add_tree_node(id, v, -1, &l),
                "duplicate root id {id}"
            );
        }
        p.assert_state_eq("duplicate root ids");
    }
}

// ===========================================================================
// Rows 26-33 — calculate_tree_sum
// ===========================================================================

#[test]
fn cfg_26_sum_absent_id() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 26);
    for _ in 0..2_000 {
        p.reset_both();
        let sc = (rng.below(20) + 1) as usize;
        seed_table(&p, &mut rng, sc, false);
        for id in [0, -1, 999, i32::MIN, i32::MAX, rng.next_i32_mixed()] {
            assert_eq!(
                p.c.calculate_tree_sum(id),
                p.r.calculate_tree_sum(id),
                "calculate_tree_sum({id}) absent"
            );
        }
    }
}

#[test]
fn cfg_27_sum_single_leaf() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 27);
    for _ in 0..N / 4 {
        p.reset_both();
        let v = rng.next_i32_mixed();
        let l = rand_label(&mut rng, 4);
        assert_eq!(
            p.c.add_tree_node(1, v, -1, &l),
            p.r.add_tree_node(1, v, -1, &l)
        );
        assert_eq!(
            p.c.calculate_tree_sum(1),
            p.r.calculate_tree_sum(1),
            "leaf sum value={v}"
        );
    }
}

#[test]
fn cfg_28_sum_one_child_left_or_right() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 28);
    for _ in 0..4_000 {
        for right_only in [false, true] {
            p.reset_both();
            let l = rand_label(&mut rng, 4);
            let (v0, v1) = (rng.next_i32_mixed(), rng.next_i32_mixed());
            assert_eq!(
                p.c.add_tree_node(1, v0, -1, &l),
                p.r.add_tree_node(1, v0, -1, &l)
            );
            assert_eq!(
                p.c.add_tree_node(2, v1, 1, &l),
                p.r.add_tree_node(2, v1, 1, &l)
            );
            if right_only {
                // move the child to the right slot by hand, leaving left empty
                for lib in [&p.c, &p.r] {
                    let mut n = lib.node(0);
                    n.right_child_id = n.left_child_id;
                    n.left_child_id = -1;
                    lib.set_node(0, &n);
                }
            }
            p.assert_state_eq("one-child setup");
            assert_eq!(
                p.c.calculate_tree_sum(1),
                p.r.calculate_tree_sum(1),
                "one-child sum (right_only={right_only})"
            );
        }
    }
}

#[test]
fn cfg_29_sum_two_children_with_overflow() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 29);
    for _ in 0..4_000 {
        p.reset_both();
        let l = rand_label(&mut rng, 4);
        let vs: Vec<c_int> = (0..4)
            .map(|_| if rng.below(3) == 0 { i32::MAX - rng.next_i32().rem_euclid(4) } else { rng.next_i32_mixed() })
            .collect();
        for (k, &v) in vs.iter().enumerate() {
            let (id, par) = match k {
                0 => (1, -1),
                1 => (2, 1),
                2 => (3, 1),
                _ => (4, 2),
            };
            assert_eq!(
                p.c.add_tree_node(id, v, par, &l),
                p.r.add_tree_node(id, v, par, &l)
            );
        }
        for root in 1..=4 {
            assert_eq!(
                p.c.calculate_tree_sum(root),
                p.r.calculate_tree_sum(root),
                "two-level sum from {root}, values {vs:?}"
            );
        }
    }
}

#[test]
fn cfg_30_sum_deep_left_chain() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 30);
    for _ in 0..300 {
        p.reset_both();
        let l = rand_label(&mut rng, 3);
        for i in 0..MAX_NODES {
            let id = i as c_int + 1;
            let par = if i == 0 { -1 } else { i as c_int };
            let v = rng.next_i32_mixed();
            assert_eq!(
                p.c.add_tree_node(id, v, par, &l),
                p.r.add_tree_node(id, v, par, &l)
            );
        }
        for root in 1..=(MAX_NODES as c_int) {
            assert_eq!(
                p.c.calculate_tree_sum(root),
                p.r.calculate_tree_sum(root),
                "deep chain sum from {root}"
            );
        }
    }
}

#[test]
fn cfg_31_sum_child_id_pointing_nowhere() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 31);
    for _ in 0..4_000 {
        p.reset_both();
        let l = rand_label(&mut rng, 4);
        let v = rng.next_i32_mixed();
        assert_eq!(
            p.c.add_tree_node(1, v, -1, &l),
            p.r.add_tree_node(1, v, -1, &l)
        );
        // Dangling child ids: != -1 but unresolvable.
        //
        // Deliberately never `1`: a child id equal to the node's own id makes
        // `calculate_tree_sum` recurse forever. Both libraries do that (it is
        // the input that is cyclic, not the translation), so it cannot be
        // compared in-process — see the note in CONFIGS.md.
        let pick = |rng: &mut Rng| loop {
            let x = rng.next_i32_mixed();
            if x != -1 && x != 1 {
                break x;
            }
        };
        let (lc, rc) = (pick(&mut rng), pick(&mut rng));
        for lib in [&p.c, &p.r] {
            let mut n = lib.node(0);
            n.left_child_id = lc;
            n.right_child_id = rc;
            lib.set_node(0, &n);
        }
        p.assert_state_eq("dangling children setup");
        assert_eq!(
            p.c.calculate_tree_sum(1),
            p.r.calculate_tree_sum(1),
            "dangling child sum (l={lc}, r={rc})"
        );
    }
}

#[test]
fn cfg_32_sum_randomized_forests() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 32);
    for round in 0..1_500 {
        p.reset_both();
        let count = (rng.below(MAX_NODES as u32) + 1) as usize;
        let l = rand_label(&mut rng, 5);
        for i in 0..count {
            let id = i as c_int + 1;
            // mix of roots and children -> a forest, not a single tree
            let par = if i == 0 || rng.below(3) == 0 {
                -1
            } else {
                (rng.below(i as u32) as c_int) + 1
            };
            let v = rng.next_i32_mixed();
            assert_eq!(
                p.c.add_tree_node(id, v, par, &l),
                p.r.add_tree_node(id, v, par, &l),
                "round {round} node {i}"
            );
        }
        p.assert_state_eq(&format!("forest round {round}"));
        for root in 1..=(count as c_int) {
            assert_eq!(
                p.c.calculate_tree_sum(root),
                p.r.calculate_tree_sum(root),
                "forest round {round} sum from {root}"
            );
        }
    }
}

#[test]
fn cfg_33_sum_child_id_resolving_to_duplicate() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 33);
    for _ in 0..4_000 {
        p.reset_both();
        let l = rand_label(&mut rng, 4);
        let vs: Vec<c_int> = (0..3).map(|_| rng.next_i32_mixed()).collect();
        // ids: 1 (root), 5, 5  -> root's left child points at id 5
        assert_eq!(
            p.c.add_tree_node(1, vs[0], -1, &l),
            p.r.add_tree_node(1, vs[0], -1, &l)
        );
        assert_eq!(
            p.c.add_tree_node(5, vs[1], 1, &l),
            p.r.add_tree_node(5, vs[1], 1, &l)
        );
        assert_eq!(
            p.c.add_tree_node(5, vs[2], -1, &l),
            p.r.add_tree_node(5, vs[2], -1, &l)
        );
        p.assert_state_eq("duplicate child id setup");
        let cv = p.c.calculate_tree_sum(1);
        assert_eq!(cv, p.r.calculate_tree_sum(1), "sum with duplicate child id");
        assert_eq!(cv, vs[0].wrapping_add(vs[1]), "must resolve to the FIRST id 5");
    }
}

// ===========================================================================
// Row 34 — the exported data objects as raw state
// ===========================================================================

#[test]
fn cfg_34_node_table_and_count_as_raw_state() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 34);
    for _ in 0..2_000 {
        let mut img = vec![0u8; NODE_TABLE_BYTES];
        for b in img.iter_mut() {
            *b = rng.next_u32() as u8;
        }
        let count = rng.below(MAX_NODES as u32 + 1) as c_int; // defined range 0..=50
        p.c.set_node_table_image(&img);
        p.r.set_node_table_image(&img);
        p.c.set_node_count(count);
        p.r.set_node_count(count);
        p.assert_state_eq("raw state round-trip");
        assert_eq!(p.c.node_table_image(), img, "C image round-trip");
        assert_eq!(p.r.node_table_image(), img, "Rust image round-trip");
        // and the library reads that state the same way
        for id in [0, 1, -1, rng.next_i32_mixed()] {
            assert_eq!(
                p.c.find_node_by_id(id),
                p.r.find_node_by_id(id),
                "find over random image, id {id}, count {count}"
            );
        }
    }
}

// ===========================================================================
// Rows 35-39 — parse_operation
// ===========================================================================

#[test]
fn cfg_35_parse_single_operator_chars() {
    let p = Pair::open();
    for (ch, want) in [(b'+', 1), (b'*', 2), (b'-', 3), (b'/', 4), (b'%', 5)] {
        let s = cstr(&[ch]);
        let cv = p.c.parse_operation(&s);
        assert_eq!(cv, p.r.parse_operation(&s), "parse_operation({:?})", ch as char);
        assert_eq!(cv, want, "C parse_operation({:?})", ch as char);
    }
}

#[test]
fn cfg_36_parse_empty_and_non_operator_strings() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 36);
    let s = cstr(b"");
    assert_eq!(p.c.parse_operation(&s), p.r.parse_operation(&s), "empty string");
    const ALPHA: &[u8] = b"abcXYZ019 \t~!@#^&_=|:;,.?<>()[]{}";
    for _ in 0..N {
        let len = rng.below(24) as usize;
        let body: Vec<u8> = (0..len).map(|_| ALPHA[rng.below(ALPHA.len() as u32) as usize]).collect();
        let s = cstr(&body);
        assert_eq!(
            p.c.parse_operation(&s),
            p.r.parse_operation(&s),
            "parse_operation({:?})",
            String::from_utf8_lossy(&body)
        );
    }
}

#[test]
fn cfg_37_parse_operator_not_first() {
    let p = Pair::open();
    for s in [
        &b"ab+cd"[..],
        b"xx*",
        b"zzz-",
        b"q/r",
        b"nn%",
        b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa%",
        b"    +",
    ] {
        let z = cstr(s);
        assert_eq!(
            p.c.parse_operation(&z),
            p.r.parse_operation(&z),
            "parse_operation({:?})",
            String::from_utf8_lossy(s)
        );
    }
}

#[test]
fn cfg_38_parse_precedence_of_check_order() {
    let p = Pair::open();
    // the C checks +, *, -, /, % in that order regardless of position
    for (s, want) in [
        (&b"%/-*+"[..], 1),
        (b"%/-*", 2),
        (b"%/-", 3),
        (b"%/", 4),
        (b"%", 5),
        (b"-+", 1),
        (b"/*", 2),
        (b"%-", 3),
        (b"%/", 4),
    ] {
        let z = cstr(s);
        let cv = p.c.parse_operation(&z);
        assert_eq!(
            cv,
            p.r.parse_operation(&z),
            "parse_operation({:?})",
            String::from_utf8_lossy(s)
        );
        assert_eq!(cv, want, "C check-order for {:?}", String::from_utf8_lossy(s));
    }
}

#[test]
fn cfg_39_parse_randomized_operator_alphabet() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 39);
    const ALPHA: &[u8] = b"+*-/%a1 ";
    for _ in 0..N {
        let len = rng.below(17) as usize;
        let body: Vec<u8> = (0..len).map(|_| ALPHA[rng.below(ALPHA.len() as u32) as usize]).collect();
        let s = cstr(&body);
        assert_eq!(
            p.c.parse_operation(&s),
            p.r.parse_operation(&s),
            "parse_operation({:?})",
            String::from_utf8_lossy(&body)
        );
    }
}

// ===========================================================================
// Rows 40-42 — get_operation_func
// ===========================================================================

#[test]
fn cfg_40_get_operation_func_valid_range() {
    let p = Pair::open();
    // (10, 3) discriminates all five ops: 13 / 30 / 7 / 3 / 1
    let expect = [(1, 13), (2, 30), (3, 7), (4, 3), (5, 1)];
    for (op, want) in expect {
        let cv = p.c.get_operation_func_probe(op, 10, 3);
        let rv = p.r.get_operation_func_probe(op, 10, 3);
        assert_eq!(cv, rv, "get_operation_func({op}) probe");
        assert_eq!(cv, want, "C get_operation_func({op}) identity");
        assert_eq!(
            p.c.get_operation_func_identity(op),
            p.r.get_operation_func_identity(op),
            "get_operation_func({op}) returned a different exported symbol"
        );
    }
}

#[test]
fn cfg_41_get_operation_func_matches_direct_export() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 41);
    let names = ["add_op", "multiply_op", "subtract_op", "divide_op", "modulo_op"];
    for _ in 0..N {
        let op = (rng.below(5) + 1) as c_int;
        let (a, b) = (rng.next_i32_mixed(), rng.next_i32_mixed());
        if (op == 4 || op == 5) && !div_safe(a, b) && b != 0 {
            continue; // faults in both; covered in Phase C
        }
        let cv = p.c.get_operation_func_probe(op, a, b);
        let rv = p.r.get_operation_func_probe(op, a, b);
        assert_eq!(cv, rv, "get_operation_func({op})({a}, {b})");
        // cross-check against the directly exported symbol of the same .so
        assert_eq!(cv, p.c.op_by_name(names[(op - 1) as usize], a, b));
        assert_eq!(rv, p.r.op_by_name(names[(op - 1) as usize], a, b));
    }
}

#[test]
fn cfg_42_get_operation_func_random_op_values() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 42);
    for _ in 0..N {
        let op = rng.next_i32_mixed();
        let (a, b) = (rng.next_i32_mixed(), rng.next_i32_mixed());
        if (op == 4 || op == 5) && !div_safe(a, b) && b != 0 {
            continue;
        }
        assert_eq!(
            p.c.get_operation_func_probe(op, a, b),
            p.r.get_operation_func_probe(op, a, b),
            "get_operation_func({op})({a}, {b})"
        );
        assert_eq!(
            p.c.get_operation_func_identity(op),
            p.r.get_operation_func_identity(op),
            "get_operation_func({op}) symbol identity"
        );
    }
}

// ===========================================================================
// Rows 43-50 — inreftree
// ===========================================================================

#[test]
fn cfg_43_inreftree_random_params() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 43);
    p.reset_both();
    for _ in 0..N {
        let (a, b, c, d) = (
            rng.next_i32_mixed(),
            rng.next_i32_mixed(),
            rng.next_i32_mixed(),
            rng.next_i32_mixed(),
        );
        assert_eq!(
            p.c.inreftree(a, b, c, d),
            p.r.inreftree(a, b, c, d),
            "inreftree({a}, {b}, {c}, {d})"
        );
    }
}

/// Params whose sum is exactly `target`, so `tree_sum % 4` can be forced.
fn params_summing_to(rng: &mut Rng, target: c_int) -> (c_int, c_int, c_int, c_int) {
    let a = rng.next_i32() >> 3;
    let b = rng.next_i32() >> 3;
    let c = rng.next_i32() >> 3;
    let d = target.wrapping_sub(a).wrapping_sub(b).wrapping_sub(c);
    (a, b, c, d)
}

#[test]
fn cfg_44_inreftree_positive_modulo_classes() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 44);
    p.reset_both();
    for class in 0..4i32 {
        for _ in 0..N / 4 {
            let sum = (rng.next_u32() % 100_000) as i32 * 4 + class;
            let (a, b, c, d) = params_summing_to(&mut rng, sum);
            let cv = p.c.inreftree(a, b, c, d);
            let rv = p.r.inreftree(a, b, c, d);
            assert_eq!(cv, rv, "inreftree({a},{b},{c},{d}) sum={sum} class={class}");
            assert_eq!(p.c.calculate_tree_sum(1), sum, "sum setup wrong");
        }
    }
}

#[test]
fn cfg_45_inreftree_negative_modulo_classes() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 45);
    p.reset_both();
    for class in 1..4i32 {
        for _ in 0..N / 3 {
            let sum = -((rng.next_u32() % 100_000) as i32 * 4 + class);
            let (a, b, c, d) = params_summing_to(&mut rng, sum);
            let cv = p.c.inreftree(a, b, c, d);
            let rv = p.r.inreftree(a, b, c, d);
            assert_eq!(
                cv, rv,
                "inreftree({a},{b},{c},{d}) sum={sum} sum%4={}",
                sum % 4
            );
            assert_eq!(p.c.calculate_tree_sum(1), sum, "sum setup wrong");
        }
    }
}

#[test]
fn cfg_46_inreftree_zero_sum() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 46);
    p.reset_both();
    let fixed = [
        (0, 0, 0, 0),
        (1, -1, 0, 0),
        (0, 1, -1, 0),
        (5, 5, -5, -5),
        (i32::MIN, i32::MIN, 0, 0), // wraps to 0
        (i32::MAX, 1, i32::MIN, 0), // wraps to 0
    ];
    for (a, b, c, d) in fixed {
        assert_eq!(
            p.c.inreftree(a, b, c, d),
            p.r.inreftree(a, b, c, d),
            "inreftree({a},{b},{c},{d}) zero-sum"
        );
    }
    for _ in 0..N {
        let (a, b, c, d) = params_summing_to(&mut rng, 0);
        assert_eq!(
            p.c.inreftree(a, b, c, d),
            p.r.inreftree(a, b, c, d),
            "inreftree({a},{b},{c},{d}) zero-sum random"
        );
    }
}

#[test]
fn cfg_47_inreftree_param2_zero_crossed_with_modulo_class() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 47);
    p.reset_both();
    for class in [-3i32, -2, -1, 0, 1, 2, 3] {
        for _ in 0..N / 7 {
            let mag = (rng.next_u32() % 100_000) as i32 * 4;
            let sum = if class < 0 { -(mag - class) } else { mag + class };
            // param2 == 0 forces the retarget branch (target_id 2 -> 1)
            let a = rng.next_i32() >> 3;
            let c = rng.next_i32() >> 3;
            let d = sum.wrapping_sub(a).wrapping_sub(c);
            let cv = p.c.inreftree(a, 0, c, d);
            let rv = p.r.inreftree(a, 0, c, d);
            assert_eq!(cv, rv, "inreftree({a},0,{c},{d}) class={class}");
            // the same sum with param2 != 0 must take the other branch
            let (a2, b2, c2, d2) = params_summing_to(&mut rng, sum);
            if b2 != 0 {
                assert_eq!(
                    p.c.inreftree(a2, b2, c2, d2),
                    p.r.inreftree(a2, b2, c2, d2),
                    "inreftree({a2},{b2},{c2},{d2}) class={class}"
                );
            }
        }
    }
}

#[test]
fn cfg_48_inreftree_extreme_params() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 48);
    p.reset_both();
    let ext = EDGE_I32;
    for &a in ext.iter() {
        for &b in ext.iter() {
            for &c in ext.iter() {
                let d = rng.next_i32_mixed();
                assert_eq!(
                    p.c.inreftree(a, b, c, d),
                    p.r.inreftree(a, b, c, d),
                    "inreftree({a},{b},{c},{d}) extreme"
                );
                assert_eq!(
                    p.c.inreftree(a, 0, b, c),
                    p.r.inreftree(a, 0, b, c),
                    "inreftree({a},0,{b},{c}) extreme + retarget"
                );
            }
        }
    }
}

#[test]
fn cfg_49_inreftree_state_carry_over_and_dirty_table() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 49);
    p.reset_both();
    for round in 0..4_000 {
        // dirty the table first, so inreftree's `node_count = 0` reset (which
        // does NOT clear node_table) is exercised over real stale bytes
        if round % 3 == 0 {
            let n = rng.below(MAX_NODES as u32) as usize;
            for i in 0..n {
                let l = rand_label_upto(&mut rng, 35);
                let id = rng.next_i32_mixed();
                let v = rng.next_i32_mixed();
                assert_eq!(
                    p.c.add_tree_node(id, v, -1, &l),
                    p.r.add_tree_node(id, v, -1, &l),
                    "dirty add {i}"
                );
            }
        }
        let (a, b, c, d) = (
            rng.next_i32_mixed(),
            rng.next_i32_mixed(),
            rng.next_i32_mixed(),
            rng.next_i32_mixed(),
        );
        assert_eq!(
            p.c.inreftree(a, b, c, d),
            p.r.inreftree(a, b, c, d),
            "round {round}: inreftree({a},{b},{c},{d}) after dirtying"
        );
        p.assert_state_eq(&format!("round {round} post-inreftree state"));
    }
}

#[test]
fn cfg_50_inreftree_post_conditions() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 50);
    p.reset_both();
    for _ in 0..4_000 {
        let (a, b, c, d) = (
            rng.next_i32_mixed(),
            rng.next_i32_mixed(),
            rng.next_i32_mixed(),
            rng.next_i32_mixed(),
        );
        assert_eq!(p.c.inreftree(a, b, c, d), p.r.inreftree(a, b, c, d));
        p.assert_state_eq(&format!("inreftree({a},{b},{c},{d}) post-state"));
        assert_eq!(p.c.get_node_count(), 4, "C node_count after inreftree");
        assert_eq!(p.r.get_node_count(), 4, "Rust node_count after inreftree");
        // the observable tree inreftree builds
        assert_eq!(p.c.calculate_tree_sum(1), p.r.calculate_tree_sum(1));
        assert_eq!(p.c.find_node_by_id(2), p.r.find_node_by_id(2));
        assert_eq!(p.c.find_node_by_id(4), p.r.find_node_by_id(4));
    }
}

// ===========================================================================
// Rows 51-52 — composed pipelines driven from the low-level exports
// ===========================================================================

#[test]
fn cfg_51_hand_assembled_pipeline() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 51);
    for round in 0..2_000 {
        p.reset_both();
        let count = (rng.below(MAX_NODES as u32) + 1) as usize;
        for i in 0..count {
            let id = i as c_int + 1;
            let par = if i == 0 { -1 } else { (rng.below(i as u32) as c_int) + 1 };
            let v = rng.next_i32_mixed();
            let l = rand_label_upto(&mut rng, 35);
            assert_eq!(
                p.c.add_tree_node(id, v, par, &l),
                p.r.add_tree_node(id, v, par, &l),
                "round {round} build {i}"
            );
        }
        p.assert_state_eq(&format!("round {round} built"));

        // find -> sum -> parse -> dispatch -> call, entirely through exports
        let probe = (rng.below(count as u32) as c_int) + 1;
        assert_eq!(p.c.find_node_by_id(probe), p.r.find_node_by_id(probe));

        let csum = p.c.calculate_tree_sum(1);
        let rsum = p.r.calculate_tree_sum(1);
        assert_eq!(csum, rsum, "round {round} sum");

        // reproduce inreftree's op selection using only public entry points
        let op_bytes = b"+*-%";
        let idx = csum.rem_euclid(4) as usize;
        let s = cstr(&[op_bytes[idx]]);
        let cop = p.c.parse_operation(&s);
        let rop = p.r.parse_operation(&s);
        assert_eq!(cop, rop, "round {round} parse");

        let target = (rng.below(count as u32) as c_int) + 1;
        if div_safe(csum, target) || (cop != 4 && cop != 5) {
            assert_eq!(
                p.c.get_operation_func_probe(cop, csum, target),
                p.r.get_operation_func_probe(rop, rsum, target),
                "round {round} dispatch+call"
            );
        }
    }
}

#[test]
fn cfg_52_randomized_fuzz_driver_over_all_entry_points() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 52);
    p.reset_both();
    let names = ["add_op", "multiply_op", "subtract_op", "divide_op", "modulo_op"];
    for step in 0..5_000 {
        match rng.below(11) {
            0 => {
                let (a, b) = (rng.next_i32_mixed(), rng.next_i32_mixed());
                let k = rng.below(5) as usize;
                if (k >= 3) && !div_safe(a, b) && b != 0 {
                    continue;
                }
                assert_eq!(
                    p.c.op_by_name(names[k], a, b),
                    p.r.op_by_name(names[k], a, b),
                    "step {step}: {}({a}, {b})",
                    names[k]
                );
            }
            1 => {
                let id = rng.next_i32_mixed();
                assert_eq!(
                    p.c.find_node_by_id(id),
                    p.r.find_node_by_id(id),
                    "step {step}: find_node_by_id({id})"
                );
            }
            2 | 3 | 4 => {
                let id = if rng.below(2) == 0 {
                    (rng.below(8) as c_int) + 1
                } else {
                    rng.next_i32_mixed()
                };
                let par = match rng.below(3) {
                    0 => -1,
                    1 => (rng.below(8) as c_int) + 1,
                    _ => rng.next_i32_mixed(),
                };
                let v = rng.next_i32_mixed();
                let l = rand_label_upto(&mut rng, 40);
                assert_eq!(
                    p.c.add_tree_node(id, v, par, &l),
                    p.r.add_tree_node(id, v, par, &l),
                    "step {step}: add_tree_node({id}, {v}, {par})"
                );
            }
            5 => {
                let id = if rng.below(2) == 0 {
                    (rng.below(8) as c_int) + 1
                } else {
                    rng.next_i32_mixed()
                };
                // Duplicate ids plus chained parents can make a child id resolve
                // back to an ancestor; that recurses forever in BOTH libraries.
                if p.c.sum_terminates(id) && p.r.sum_terminates(id) {
                    assert_eq!(
                        p.c.calculate_tree_sum(id),
                        p.r.calculate_tree_sum(id),
                        "step {step}: calculate_tree_sum({id})"
                    );
                }
            }
            6 => {
                let len = rng.below(12) as usize;
                let body: Vec<u8> = (0..len)
                    .map(|_| b"+*-/%aZ 9"[rng.below(9) as usize])
                    .collect();
                let s = cstr(&body);
                assert_eq!(
                    p.c.parse_operation(&s),
                    p.r.parse_operation(&s),
                    "step {step}: parse_operation({:?})",
                    String::from_utf8_lossy(&body)
                );
            }
            7 => {
                let op = rng.next_i32_mixed();
                let (a, b) = (rng.next_i32_mixed(), rng.next_i32_mixed());
                if (op == 4 || op == 5) && !div_safe(a, b) && b != 0 {
                    continue;
                }
                assert_eq!(
                    p.c.get_operation_func_probe(op, a, b),
                    p.r.get_operation_func_probe(op, a, b),
                    "step {step}: get_operation_func({op})({a},{b})"
                );
            }
            8 => {
                let (a, b, c, d) = (
                    rng.next_i32_mixed(),
                    rng.next_i32_mixed(),
                    rng.next_i32_mixed(),
                    rng.next_i32_mixed(),
                );
                assert_eq!(
                    p.c.inreftree(a, b, c, d),
                    p.r.inreftree(a, b, c, d),
                    "step {step}: inreftree({a},{b},{c},{d})"
                );
            }
            9 => {
                let v = rng.below(MAX_NODES as u32 + 1) as c_int;
                p.c.set_node_count(v);
                p.r.set_node_count(v);
            }
            _ => {
                p.reset_both();
            }
        }
        p.assert_state_eq(&format!("step {step} state"));
    }
}

// ===========================================================================
// Rows 53-55 — writes over POISONED memory
//
// Every row above starts from a zeroed table, which cannot distinguish "wrote
// 0" from "wrote nothing". These rows pre-fill both tables with a non-zero
// pattern so any field the implementation forgets to store shows up in the
// image comparison. This is what catches e.g. a missing `label[31] = '\0'`.
// ===========================================================================

#[test]
fn cfg_53_add_tree_node_over_poisoned_slot() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 53);
    for len in [0usize, 1, 5, 29, 30, 31, 32, 33, 60] {
        for _ in 0..300 {
            p.poison_both(&mut rng, 0);
            let label = rand_label(&mut rng, len);
            let (id, v) = (rng.next_i32_mixed(), rng.next_i32_mixed());
            let cv = p.c.add_tree_node(id, v, -1, &label);
            let rv = p.r.add_tree_node(id, v, -1, &label);
            assert_eq!(cv, rv, "add_tree_node over poison, label len {len}");
            p.assert_state_eq(&format!("poisoned slot, label len {len}"));
            // every field of the slot must have been overwritten
            let cn = p.c.node(0);
            assert_eq!(cn.id, id);
            assert_eq!(cn.value, v);
            assert_eq!(cn.parent_id, -1);
            assert_eq!(cn.left_child_id, -1);
            assert_eq!(cn.right_child_id, -1);
            assert_eq!(cn.label[31], 0, "label[31] must be forced to NUL over poison");
            let want = std::cmp::min(len, 31);
            for k in 0..want {
                assert_eq!(cn.label[k] as u8, label[k], "label byte {k}, len {len}");
            }
            for k in want..32 {
                assert_eq!(cn.label[k], 0, "label byte {k} must be zeroed, len {len}");
            }
            // and the NEXT slot must still be untouched poison
            assert_ne!(p.c.node(1).id, 0, "poison in slot 1 was clobbered");
        }
    }
}

#[test]
fn cfg_54_add_tree_node_over_poison_at_every_index() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 54);
    for slot in 0..MAX_NODES {
        p.poison_both(&mut rng, slot as c_int);
        let label = rand_label_upto(&mut rng, 40);
        let (id, v) = (rng.next_i32_mixed(), rng.next_i32_mixed());
        // parent_id == -1 keeps the write independent of the poison bytes that
        // find_node_by_id would otherwise scan
        let cv = p.c.add_tree_node(id, v, -1, &label);
        let rv = p.r.add_tree_node(id, v, -1, &label);
        assert_eq!(cv, rv, "add_tree_node into poisoned slot {slot}");
        assert_eq!(cv, slot as c_int, "must return the slot index");
        p.assert_state_eq(&format!("poisoned slot {slot}"));
        assert_eq!(p.c.node(slot).label[31], 0, "slot {slot} label[31]");
        assert_eq!(p.r.node(slot).label[31], 0, "slot {slot} label[31]");
    }
}

#[test]
fn cfg_55_inreftree_over_poisoned_table() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 55);
    for _ in 0..4_000 {
        // inreftree resets only node_count, so it runs directly over poison
        let start_count = rng.below(MAX_NODES as u32 + 1) as c_int;
        p.poison_both(&mut rng, start_count);
        let (a, b, c, d) = (
            rng.next_i32_mixed(),
            rng.next_i32_mixed(),
            rng.next_i32_mixed(),
            rng.next_i32_mixed(),
        );
        assert_eq!(
            p.c.inreftree(a, b, c, d),
            p.r.inreftree(a, b, c, d),
            "inreftree({a},{b},{c},{d}) over poison"
        );
        p.assert_state_eq("inreftree over poison");
        // slots 0..4 rewritten, 4..50 still poison
        for i in 0..4 {
            assert_eq!(p.c.node(i).label[31], 0, "slot {i} label[31] after inreftree");
            assert_eq!(p.r.node(i).label[31], 0, "slot {i} label[31] after inreftree");
        }
    }
}
