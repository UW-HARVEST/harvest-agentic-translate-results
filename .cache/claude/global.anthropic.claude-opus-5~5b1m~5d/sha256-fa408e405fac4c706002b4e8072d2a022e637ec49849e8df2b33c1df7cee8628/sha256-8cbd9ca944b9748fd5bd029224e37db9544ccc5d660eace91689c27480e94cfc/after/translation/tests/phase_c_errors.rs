//! Phase C — error-path differential tests, one test per ERRORS.md row.
//!
//! Every rejection asserts the *same* sentinel / error value from both
//! libraries, not merely "both failed". The four rows that kill the process
//! (rows 8, 18, 21, 23) are run in an identical **C host** subprocess so the
//! terminating signal is comparable (see `crash_host.rs`).

mod harness;
use harness::crash_host;
use harness::*;
use std::ffi::CString;

fn add(l: &Lib, id: i32, v: i32, parent: i32, label: &[u8]) -> i32 {
    let c = CString::new(label).unwrap();
    unsafe { (l.add_tree_node)(id, v, parent, c.as_ptr()) }
}

#[track_caller]
fn add_both(p: &Pair, id: i32, v: i32, parent: i32, label: &[u8], ctx: &str) -> i32 {
    let rc = add(&p.c, id, v, parent, label);
    let rr = add(&p.r, id, v, parent, label);
    assert_eq!(rc, rr, "{ctx}: add_tree_node ret C={rc} Rust={rr}");
    assert_state_eq(p, ctx);
    rc
}

#[track_caller]
fn sum_both(p: &Pair, id: i32, ctx: &str) -> i32 {
    let (c, r) = unsafe { ((p.c.calculate_tree_sum)(id), (p.r.calculate_tree_sum)(id)) };
    assert_eq!(c, r, "{ctx}: calculate_tree_sum({id}) C={c} Rust={r}");
    c
}

// ===========================================================================
// Row 1 — add_tree_node: table full
// ===========================================================================

#[test]
fn err01_add_tree_node_table_full() {
    let (p, _g) = fresh();
    let mut rng = Rng::new(1);
    add_both(p, 1, 5, -1, b"root", "err1/root");
    for i in 2..=MAX_NODES as i32 {
        add_both(p, i, rng.spicy_i32(), 1, b"n", "err1/fill");
    }
    assert_eq!(p.c.count(), 50);
    // Rejected, and the table must be left completely untouched.
    let before_c = p.c.table_bytes();
    let before_r = p.r.table_bytes();
    for _ in 0..50 {
        let ret = add_both(p, rng.spicy_i32(), rng.spicy_i32(), -1, b"nope", "err1/full");
        assert_eq!(ret, -1, "err1: full table must return -1");
    }
    assert_eq!(p.c.count(), 50, "err1: node_count must not change");
    assert_eq!(p.r.count(), 50);
    assert_eq!(p.c.table_bytes(), before_c, "err1: C table changed");
    assert_eq!(p.r.table_bytes(), before_r, "err1: Rust table changed");

    // node_count forced past MAX_NODES from the outside
    for bogus in [50i32, 51, 100, i32::MAX] {
        p.c.set_count(bogus);
        p.r.set_count(bogus);
        assert_eq!(add_both(p, 7, 7, -1, b"x", "err1/forced"), -1);
        assert_eq!(p.c.count(), bogus);
    }
}

// ===========================================================================
// Row 2 — add_tree_node: unknown parent (returns -1 but already scribbled)
// ===========================================================================

#[test]
fn err02_add_tree_node_unknown_parent() {
    let (p, _g) = fresh();
    let mut rng = Rng::new(2);
    for trial in 0..200 {
        p.c.zero_state();
        p.r.zero_state();
        add_both(p, 1, 11, -1, b"root", "err2/root");
        let missing = 100 + trial;
        let ret = add_both(p, 42, 99, missing, b"orphan", "err2");
        assert_eq!(ret, -1, "err2: unknown parent must return -1");
        assert_eq!(p.c.count(), 1, "err2: node_count must not advance");
        // ...but node_table[1] WAS overwritten before the failure was detected.
        // assert_state_eq inside add_both already proved C and Rust scribbled
        // the same bytes; pin the observable consequence explicitly.
        let n1 = p.c.table()[1];
        assert_eq!(n1.id, 42, "err2: C must have written the rejected node");
        assert_eq!(n1.value, 99);
        assert_eq!(n1.parent_id, missing);
        assert_eq!(n1.left_child_id, -1);
        assert_eq!(n1.right_child_id, -1);
        assert_eq!(&n1.label[..6], b"orphan");
        assert_eq!(p.r.table()[1], n1, "err2: Rust scribble differs");

        // random unknown parents from a fresh table (node_count == 0)
        p.c.zero_state();
        p.r.zero_state();
        let parent = rng.spicy_i32();
        if parent != -1 {
            assert_eq!(add_both(p, rng.spicy_i32(), rng.spicy_i32(), parent, b"o", "err2/r"), -1);
            assert_eq!(p.c.count(), 0);
        }
    }
}

// ===========================================================================
// Row 3 — add_tree_node: `parent->id != parent_id` is a dead branch
// ===========================================================================

#[test]
fn err03_add_tree_node_parent_id_branch_dead() {
    let (p, _g) = fresh();
    let mut rng = Rng::new(3);
    // find_node_by_id only ever returns a node whose id == the requested id, so
    // whenever the parent is found the second half of the `||` cannot fire:
    // the insert must succeed. Prove it over a random table with duplicate ids.
    for trial in 0..300 {
        p.c.zero_state();
        p.r.zero_state();
        let n = 1 + rng.below(8) as usize;
        let mut ids = Vec::new();
        for i in 0..n {
            let id = (rng.below(5) as i32) - 1; // pool incl. -1 -> duplicates
            ids.push(id);
            add_both(p, id, rng.spicy_i32(), -1, b"x", "err3/seed");
            let _ = i;
        }
        for &id in &ids {
            if id == -1 {
                continue; // -1 is the root sentinel, not a parent lookup
            }
            if p.c.count() >= MAX_NODES as i32 {
                break;
            }
            let ret = add_both(p, 900, rng.spicy_i32(), id, b"child", &format!("err3/{trial}"));
            assert_ne!(
                ret, -1,
                "err3: parent id {id} exists, so the dead branch must not reject"
            );
        }
    }
}

// ===========================================================================
// Row 4 — add_tree_node: parent_id == -1 skips the lookup entirely
// ===========================================================================

#[test]
fn err04_add_tree_node_root_sentinel() {
    let (p, _g) = fresh();
    let mut rng = Rng::new(4);
    // No node with id -1 exists, yet parent_id == -1 must still succeed,
    // and must NOT link the new node to anything.
    for i in 0..40 {
        let ret = add_both(p, rng.spicy_i32(), rng.spicy_i32(), -1, b"root", "err4");
        assert_eq!(ret, i, "err4: parent_id -1 must always succeed");
    }
    assert_eq!(p.c.count(), 40);
    // Even when a node with id == -1 really is in the table, -1 is still the
    // sentinel and is never used as a parent.
    p.c.zero_state();
    p.r.zero_state();
    add_both(p, -1, 5, -1, b"neg", "err4/negid");
    add_both(p, 2, 6, -1, b"second", "err4/second");
    assert_eq!(p.c.table()[0].left_child_id, -1, "err4: -1 must not be linked as parent");
    assert_eq!(p.c.table()[0].right_child_id, -1);
}

// ===========================================================================
// Row 5 — third child silently unlinked
// ===========================================================================

#[test]
fn err05_add_tree_node_third_child_dropped() {
    let (p, _g) = fresh();
    add_both(p, 1, 1, -1, b"root", "err5");
    for i in 2..=10i32 {
        let ret = add_both(p, i, i, 1, b"c", "err5");
        assert_eq!(ret, i - 1, "err5: node must still be added");
    }
    assert_eq!(p.c.table()[0].left_child_id, 2);
    assert_eq!(p.c.table()[0].right_child_id, 3);
    assert_eq!(p.c.count(), 10);
    // and the sum only walks the two linked children
    assert_eq!(sum_both(p, 1, "err5"), 1 + 2 + 3);
}

// ===========================================================================
// Rows 6-7 — label truncation and its boundaries
// ===========================================================================

#[test]
fn err06_add_tree_node_label_truncation() {
    let (p, _g) = fresh();
    for len in [32usize, 33, 40, 64, 200] {
        p.c.zero_state();
        p.r.zero_state();
        let label: Vec<u8> = (0..len).map(|i| b'a' + (i % 26) as u8).collect();
        add_both(p, 1, 1, -1, &label, &format!("err6/len{len}"));
        let l = p.c.table()[0].label;
        assert_eq!(&l[..31], &label[..31], "err6: first 31 bytes must be copied");
        assert_eq!(l[31], 0, "err6: label[31] must be the forced NUL");
        assert_eq!(p.r.table()[0].label, l, "err6: Rust label differs");
    }
}

#[test]
fn err07_add_tree_node_label_boundaries() {
    let (p, _g) = fresh();
    for len in [0usize, 1, 30, 31] {
        p.c.zero_state();
        p.r.zero_state();
        let label: Vec<u8> = vec![b'Z'; len];
        add_both(p, 1, 1, -1, &label, &format!("err7/len{len}"));
        let l = p.c.table()[0].label;
        assert_eq!(&l[..len], &label[..]);
        assert!(l[len..].iter().all(|&b| b == 0), "err7: must be NUL-padded to 32");
        assert_eq!(p.r.table()[0].label, l);
    }
}

// ===========================================================================
// Rows 9-13 — find_node_by_id rejections
// ===========================================================================

#[test]
fn err09_find_node_missing_id() {
    let (p, _g) = fresh();
    let mut rng = Rng::new(9);
    for i in 1..=10i32 {
        add_both(p, i, i, -1, b"n", "err9");
    }
    for id in [-5i32, -2, 0, 11, 12, 1000, i32::MIN, i32::MAX] {
        assert_eq!(p.c.find_index(id), None, "err9: C must return NULL for {id}");
        assert_eq!(p.r.find_index(id), None, "err9: Rust must return NULL for {id}");
    }
    for _ in 0..5000 {
        let id = rng.spicy_i32();
        let (c, r) = (p.c.find_index(id), p.r.find_index(id));
        assert_eq!(c, r, "err9: find_node_by_id({id}) C={c:?} Rust={r:?}");
        if !(1..=10).contains(&id) {
            assert_eq!(c, None, "err9: {id} is not in the table");
        }
    }
}

#[test]
fn err10_find_node_empty_table() {
    let (p, _g) = fresh();
    // node_count == 0: even ids physically present in node_table are invisible.
    let mut nodes = vec![TreeNode::default(); MAX_NODES];
    for (i, n) in nodes.iter_mut().enumerate() {
        n.id = i as i32 + 1;
    }
    p.c.set_table(&nodes);
    p.r.set_table(&nodes);
    p.c.set_count(0);
    p.r.set_count(0);
    for id in [-1i32, 0, 1, 2, 50, 51] {
        assert_eq!(p.c.find_index(id), None, "err10: C count=0 must return NULL for {id}");
        assert_eq!(p.r.find_index(id), None, "err10: Rust count=0 must return NULL for {id}");
        assert_eq!(sum_both(p, id, "err10"), 0, "err10: sum must be 0");
    }
}

#[test]
fn err11_find_node_negative_count() {
    let (p, _g) = fresh();
    let mut nodes = vec![TreeNode::default(); MAX_NODES];
    for (i, n) in nodes.iter_mut().enumerate() {
        n.id = i as i32 + 1;
        n.value = 1000 + i as i32;
        n.left_child_id = -1;
        n.right_child_id = -1;
    }
    for bogus in [-1i32, -7, -50, i32::MIN] {
        p.c.set_table(&nodes);
        p.r.set_table(&nodes);
        p.c.set_count(bogus);
        p.r.set_count(bogus);
        for id in [-1i32, 0, 1, 2, 49, 50] {
            let (c, r) = (p.c.find_index(id), p.r.find_index(id));
            assert_eq!(c, r, "err11: count={bogus} find({id}) C={c:?} Rust={r:?}");
            assert_eq!(c, None, "err11: negative count must find nothing");
            assert_eq!(sum_both(p, id, "err11"), 0);
        }
        // add_tree_node with a negative count writes at node_table[bogus] in C,
        // which is out of bounds; only the in-bounds -0 case is safe to poke.
        assert_eq!(p.c.count(), bogus);
        assert_eq!(p.r.count(), bogus);
    }
}

#[test]
fn err12_find_node_duplicate_ids() {
    let (p, _g) = fresh();
    let mut rng = Rng::new(12);
    for trial in 0..300 {
        p.c.zero_state();
        p.r.zero_state();
        let n = 2 + rng.below(10) as usize;
        let mut ids = Vec::new();
        for _ in 0..n {
            let id = (rng.below(3) as i32) + 1; // heavy duplication over {1,2,3}
            ids.push(id);
            add_both(p, id, rng.spicy_i32(), -1, b"d", "err12/seed");
        }
        for &id in &[1i32, 2, 3, 4] {
            let expect = ids.iter().position(|&x| x == id).map(|i| i as isize);
            assert_eq!(p.c.find_index(id), expect, "err12/{trial}: C must find first match");
            assert_eq!(p.r.find_index(id), expect, "err12/{trial}: Rust must find first match");
        }
    }
}

#[test]
fn err13_find_node_extreme_ids() {
    let (p, _g) = fresh();
    add_both(p, i32::MIN, 1, -1, b"min", "err13");
    add_both(p, i32::MAX, 2, -1, b"max", "err13");
    add_both(p, -1, 3, -1, b"neg1", "err13");
    add_both(p, 0, 4, -1, b"zero", "err13");
    for (id, want) in [
        (i32::MIN, Some(0isize)),
        (i32::MAX, Some(1)),
        (-1, Some(2)),
        (0, Some(3)),
        (i32::MIN + 1, None),
        (i32::MAX - 1, None),
        (1, None),
    ] {
        assert_eq!(p.c.find_index(id), want, "err13: C find({id})");
        assert_eq!(p.r.find_index(id), want, "err13: Rust find({id})");
    }
}

// ===========================================================================
// Rows 14-17, 19 — calculate_tree_sum rejections
// ===========================================================================

#[test]
fn err14_tree_sum_unknown_id() {
    let (p, _g) = fresh();
    let mut rng = Rng::new(14);
    for i in 1..=5i32 {
        add_both(p, i, i * 10, if i == 1 { -1 } else { 1 }, b"n", "err14");
    }
    for id in [-9i32, 0, 6, 7, 999, i32::MIN, i32::MAX] {
        assert_eq!(sum_both(p, id, "err14"), 0, "err14: unknown id must return 0");
    }
    for _ in 0..3000 {
        sum_both(p, rng.spicy_i32(), "err14/rand");
    }
    // A real node whose subtree sums to 0 is indistinguishable from "not found".
    p.c.zero_state();
    p.r.zero_state();
    add_both(p, 7, 0, -1, b"zero", "err14/amb");
    assert_eq!(sum_both(p, 7, "err14/amb"), 0);
    assert_eq!(sum_both(p, 8, "err14/amb"), 0);
}

#[test]
fn err15_tree_sum_id_branch_dead() {
    let (p, _g) = fresh();
    let mut rng = Rng::new(15);
    // `node->id != node_id` after a successful find is impossible; therefore any
    // id present in the table must yield a non-"not found" walk. Verify by
    // comparing against an independent model over random acyclic trees.
    for _ in 0..200 {
        p.c.zero_state();
        p.r.zero_state();
        let n = 1 + rng.below(12) as i32;
        let mut vals = vec![0i32; (n + 1) as usize];
        let mut kids: Vec<Vec<i32>> = vec![Vec::new(); (n + 1) as usize];
        for i in 1..=n {
            let parent = if i == 1 { -1 } else { 1 + rng.below(i as u64 - 1) as i32 };
            let v = (rng.below(1000) as i32) + 1; // strictly positive
            vals[i as usize] = v;
            add_both(p, i, v, parent, b"n", "err15");
            if parent != -1 && kids[parent as usize].len() < 2 {
                kids[parent as usize].push(i);
            }
        }
        for i in 1..=n {
            let got = sum_both(p, i, "err15");
            // model: values are all > 0, so a found node can never look "missing"
            fn model(i: i32, vals: &[i32], kids: &[Vec<i32>]) -> i32 {
                let mut s = vals[i as usize];
                for &k in &kids[i as usize] {
                    s = s.wrapping_add(model(k, vals, kids));
                }
                s
            }
            assert_eq!(got, model(i, &vals, &kids), "err15: subtree sum of {i}");
            assert!(got > 0, "err15: found node must not look like 'not found'");
        }
    }
}

#[test]
fn err16_tree_sum_sentinel_children() {
    let (p, _g) = fresh();
    // left/right == -1 means "no child" and is NOT recursed into, even though a
    // node with id -1 exists in the table.
    add_both(p, 1, 100, -1, b"root", "err16");
    add_both(p, -1, 7, -1, b"negid", "err16");
    assert_eq!(p.c.table()[0].left_child_id, -1);
    assert_eq!(sum_both(p, 1, "err16"), 100, "err16: -1 sentinel must not recurse");
    assert_eq!(sum_both(p, -1, "err16"), 7);

    // explicitly set both children to -1 through the exported table
    let mut nodes = vec![TreeNode::default(); MAX_NODES];
    nodes[0] = TreeNode { id: 5, value: 33, parent_id: -1, left_child_id: -1, right_child_id: -1, label: [0; 32] };
    p.c.set_table(&nodes);
    p.r.set_table(&nodes);
    p.c.set_count(1);
    p.r.set_count(1);
    assert_eq!(sum_both(p, 5, "err16/direct"), 33);
}

#[test]
fn err17_tree_sum_dangling_child() {
    let (p, _g) = fresh();
    let mut rng = Rng::new(17);
    for trial in 0..300 {
        let mut nodes = vec![TreeNode::default(); MAX_NODES];
        let l = rng.spicy_i32();
        let r = rng.spicy_i32();
        nodes[0] = TreeNode {
            id: 1,
            value: 25,
            parent_id: -1,
            // dangling unless the random value happens to be -1 or 1
            left_child_id: if l == 1 { 777 } else { l },
            right_child_id: if r == 1 { 888 } else { r },
            label: [0; 32],
        };
        p.c.set_table(&nodes);
        p.r.set_table(&nodes);
        p.c.set_count(1);
        p.r.set_count(1);
        let got = sum_both(p, 1, &format!("err17/{trial}"));
        assert_eq!(got, 25, "err17: dangling children must contribute 0, no error");
    }
}

#[test]
fn err19_tree_sum_overflow_wraps() {
    let (p, _g) = fresh();
    for (a, b, c) in [
        (i32::MAX, i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN, i32::MIN),
        (i32::MAX, 1, 1),
        (i32::MIN, -1, -1),
        (i32::MAX, i32::MIN, 1),
    ] {
        p.c.zero_state();
        p.r.zero_state();
        add_both(p, 1, a, -1, b"root", "err19");
        add_both(p, 2, b, 1, b"l", "err19");
        add_both(p, 3, c, 1, b"r", "err19");
        let got = sum_both(p, 1, "err19");
        assert_eq!(got, a.wrapping_add(b).wrapping_add(c), "err19: must wrap");
    }
}

// ===========================================================================
// Rows 20, 22, 24, 25 — arithmetic guards and semantics
// ===========================================================================

#[test]
fn err20_divide_by_zero() {
    let (p, _g) = fresh();
    let mut rng = Rng::new(20);
    for a in [0i32, 1, -1, 7, -7, i32::MIN, i32::MAX] {
        let (c, r) = unsafe { ((p.c.divide_op)(a, 0, 0, 0), (p.r.divide_op)(a, 0, 0, 0)) };
        assert_eq!(c, 0, "err20: C divide_op({a},0) must be 0, got {c}");
        assert_eq!(r, 0, "err20: Rust divide_op({a},0) must be 0, got {r}");
    }
    for _ in 0..5000 {
        let a = rng.spicy_i32();
        let (c, r) = unsafe { ((p.c.divide_op)(a, 0, 0, 0), (p.r.divide_op)(a, 0, 0, 0)) };
        assert_eq!((c, r), (0, 0), "err20: divide_op({a},0)");
    }
    // also through the function-pointer dispatch
    let cf = unsafe { (p.c.get_operation_func)(OP_DIVIDE) };
    let rf = unsafe { (p.r.get_operation_func)(OP_DIVIDE) };
    assert_eq!(unsafe { cf(5, 0, 0, 0) }, unsafe { rf(5, 0, 0, 0) });
    assert_eq!(unsafe { cf(5, 0, 0, 0) }, 0);
}

#[test]
fn err22_modulo_by_zero() {
    let (p, _g) = fresh();
    let mut rng = Rng::new(22);
    for a in [0i32, 1, -1, 7, -7, i32::MIN, i32::MAX] {
        let (c, r) = unsafe { ((p.c.modulo_op)(a, 0, 0, 0), (p.r.modulo_op)(a, 0, 0, 0)) };
        assert_eq!(c, 0, "err22: C modulo_op({a},0) must be 0");
        assert_eq!(r, 0, "err22: Rust modulo_op({a},0) must be 0");
    }
    for _ in 0..5000 {
        let a = rng.spicy_i32();
        let (c, r) = unsafe { ((p.c.modulo_op)(a, 0, 0, 0), (p.r.modulo_op)(a, 0, 0, 0)) };
        assert_eq!((c, r), (0, 0), "err22: modulo_op({a},0)");
    }
    let cf = unsafe { (p.c.get_operation_func)(OP_MODULO) };
    let rf = unsafe { (p.r.get_operation_func)(OP_MODULO) };
    assert_eq!(unsafe { cf(5, 0, 0, 0) }, unsafe { rf(5, 0, 0, 0) });
    assert_eq!(unsafe { cf(5, 0, 0, 0) }, 0);
}

#[test]
fn err24_modulo_sign_semantics() {
    let (p, _g) = fresh();
    // C truncates toward zero: the remainder takes the sign of the dividend.
    for (a, b, want) in [
        (-7i32, 2i32, -1i32),
        (7, -2, 1),
        (-7, -2, -1),
        (7, 2, 1),
        (-1, 2, -1),
        (i32::MIN, 2, 0),
        (i32::MIN, 3, -2),
        (i32::MAX, -3, 1),
    ] {
        let (c, r) = unsafe { ((p.c.modulo_op)(a, b, 0, 0), (p.r.modulo_op)(a, b, 0, 0)) };
        assert_eq!(c, want, "err24: C {a} % {b} should be {want}, got {c}");
        assert_eq!(r, want, "err24: Rust {a} % {b} should be {want}, got {r}");
    }
    // and division truncates toward zero as well
    for (a, b, want) in [(-7i32, 2i32, -3i32), (7, -2, -3), (-7, -2, 3), (1, 2, 0), (-1, 2, 0)] {
        let (c, r) = unsafe { ((p.c.divide_op)(a, b, 0, 0), (p.r.divide_op)(a, b, 0, 0)) };
        assert_eq!(c, want, "err24: C {a} / {b}");
        assert_eq!(r, want, "err24: Rust {a} / {b}");
    }
}

#[test]
fn err25_arith_overflow_wraps() {
    let (p, _g) = fresh();
    for (a, b) in [
        (i32::MAX, 1),
        (i32::MAX, i32::MAX),
        (i32::MIN, -1),
        (i32::MIN, i32::MIN),
        (65536, 65536),
        (-65536, 65537),
    ] {
        for (name, cf, rf) in [
            ("add", p.c.add_op, p.r.add_op),
            ("mul", p.c.multiply_op, p.r.multiply_op),
            ("sub", p.c.subtract_op, p.r.subtract_op),
        ] {
            let (c, r) = unsafe { (cf(a, b, 0, 0), rf(a, b, 0, 0)) };
            assert_eq!(c, r, "err25/{name}({a},{b}): C={c} Rust={r}");
            let want = match name {
                "add" => a.wrapping_add(b),
                "mul" => a.wrapping_mul(b),
                _ => a.wrapping_sub(b),
            };
            assert_eq!(c, want, "err25/{name}: C did not wrap two's-complement");
        }
    }
}

// ===========================================================================
// Rows 26-29 — parse_operation rejections
// ===========================================================================

#[test]
fn err26_parse_operation_null() {
    let (p, _g) = fresh();
    // NULL short-circuits before strchr; it is NOT an error, it is OP_ADD.
    let (c, r) = unsafe {
        (
            (p.c.parse_operation)(std::ptr::null()),
            (p.r.parse_operation)(std::ptr::null()),
        )
    };
    assert_eq!(c, OP_ADD, "err26: C parse_operation(NULL) must be OP_ADD, got {c}");
    assert_eq!(r, OP_ADD, "err26: Rust parse_operation(NULL) must be OP_ADD, got {r}");
    assert_eq!(c, r);
}

fn parse_both(p: &Pair, bytes: &[u8], ctx: &str) -> i32 {
    let s = CString::new(bytes).unwrap();
    let (c, r) = unsafe { ((p.c.parse_operation)(s.as_ptr()), (p.r.parse_operation)(s.as_ptr())) };
    assert_eq!(c, r, "{ctx}: parse_operation({bytes:?}) C={c} Rust={r}");
    c
}

#[test]
fn err27_parse_operation_no_operator() {
    let (p, _g) = fresh();
    let mut rng = Rng::new(27);
    for s in [
        &b"abc"[..],
        b"0123456789",
        b"ADD",
        b"plus",
        b" ",
        b"\t\n\r",
        b"~!@#^&()_=[]{}|;:'\",.<>?`",
        b"\xff\xfe\x80\x7f",
    ] {
        assert_eq!(parse_both(p, s, "err27"), OP_ADD, "err27: {s:?} must fall through to OP_ADD");
    }
    // random strings drawn from an operator-free alphabet
    let alphabet: Vec<u8> = (1u8..=255).filter(|c| !b"+*-/%".contains(c)).collect();
    for _ in 0..5000 {
        let len = rng.below(16) as usize;
        let s: Vec<u8> = (0..len).map(|_| alphabet[rng.below(alphabet.len() as u64) as usize]).collect();
        assert_eq!(parse_both(p, &s, "err27/rand"), OP_ADD, "err27: {s:?}");
    }
}

#[test]
fn err28_parse_operation_empty() {
    let (p, _g) = fresh();
    assert_eq!(parse_both(p, b"", "err28"), OP_ADD, "err28: \"\" must be OP_ADD");
}

#[test]
fn err29_parse_operation_priority() {
    let (p, _g) = fresh();
    // Fixed check order + * - / %, NOT left-to-right in the input.
    for (s, want) in [
        (&b"%/-*+"[..], OP_ADD),
        (b"%/-*", OP_MULTIPLY),
        (b"%/-", OP_SUBTRACT),
        (b"%/", OP_DIVIDE),
        (b"%", OP_MODULO),
        (b"/%", OP_DIVIDE),
        (b"-/%", OP_SUBTRACT),
        (b"*-/%", OP_MULTIPLY),
        (b"+*-/%", OP_ADD),
        (b"zzz%zzz+", OP_ADD),
    ] {
        assert_eq!(
            parse_both(p, s, "err29"),
            want,
            "err29: {:?} must select {want}",
            String::from_utf8_lossy(s)
        );
    }
}

// ===========================================================================
// Row 30 — out-of-range enum values across the FFI boundary
// ===========================================================================

#[test]
fn err30_get_operation_func_out_of_range_enum() {
    let (p, _g) = fresh();
    let mut rng = Rng::new(30);
    let mut cases: Vec<i32> = vec![
        i32::MIN,
        i32::MIN + 1,
        -1000,
        -6,
        -5,
        -1,
        0,
        6,
        7,
        8,
        100,
        0x1_0001, // low byte 1: must NOT be mistaken for OP_ADD by truncation
        0x1_0002,
        256,
        i32::MAX - 1,
        i32::MAX,
    ];
    for _ in 0..2000 {
        let v = rng.spicy_i32();
        if !(1..=5).contains(&v) {
            cases.push(v);
        }
    }
    for op in cases {
        let (c, r) = (p.c.op_func_name(op), p.r.op_func_name(op));
        assert_eq!(c, "add_op", "err30: C get_operation_func({op}) -> {c}, want add_op");
        assert_eq!(r, "add_op", "err30: Rust get_operation_func({op}) -> {r}, want add_op");
        // and the returned function must actually behave like add_op
        let cf = unsafe { (p.c.get_operation_func)(op) };
        let rf = unsafe { (p.r.get_operation_func)(op) };
        let (a, b) = (rng.spicy_i32(), rng.spicy_i32());
        let (cv, rv) = unsafe { (cf(a, b, 0, 0), rf(a, b, 0, 0)) };
        assert_eq!(cv, rv, "err30: op={op} f({a},{b})");
        assert_eq!(cv, a.wrapping_add(b), "err30: default must be add_op");
    }
}

#[test]
fn err31_get_operation_func_valid_enums() {
    let (p, _g) = fresh();
    for (op, want) in [
        (1, "add_op"),
        (2, "multiply_op"),
        (3, "subtract_op"),
        (4, "divide_op"),
        (5, "modulo_op"),
    ] {
        assert_eq!(p.c.op_func_name(op), want, "err31: C op {op}");
        assert_eq!(p.r.op_func_name(op), want, "err31: Rust op {op}");
    }
}

// ===========================================================================
// Rows 32-33 — inreftree target reset / dead NULL branch
// ===========================================================================

#[test]
fn err32_inreftree_target_reset() {
    let (p, _g) = fresh();
    let mut rng = Rng::new(32);
    // param2 == 0 -> node 2's value is 0 -> target_id reset from 2 to 1.
    // Compare C and Rust, and pin the model.
    for _ in 0..3000 {
        let (a, c, d) = (rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
        for b in [0i32, 1, -1] {
            let (rc, rr) = unsafe { ((p.c.inreftree)(a, b, c, d), (p.r.inreftree)(a, b, c, d)) };
            assert_eq!(rc, rr, "err32: inreftree({a},{b},{c},{d}) C={rc} Rust={rr}");
            let sum = a.wrapping_add(b).wrapping_add(c).wrapping_add(d);
            let target = if b == 0 { 1 } else { 2 };
            let want = match sum % 4 {
                0 => sum.wrapping_add(target),
                1 => sum.wrapping_mul(target),
                2 => sum.wrapping_sub(target),
                3 => sum.wrapping_rem(target),
                _ => sum.wrapping_add(target), // negative residue -> OOB -> OP_ADD
            };
            assert_eq!(rc, want, "err32: model mismatch ({a},{b},{c},{d})");
        }
    }
    // Row 33: the `target == NULL` half of the guard is dead — "left" always
    // contains 'l', so target_id is always 2 and find_node_by_id(2) succeeds.
    // Consequence: after inreftree the label of node index 1 is always "left".
    assert_eq!(&p.c.table()[1].label[..5], b"left\0");
    assert_eq!(&p.r.table()[1].label[..5], b"left\0");
    assert_eq!(p.c.count(), 4);
}

// ===========================================================================
// Row 34 — the out-of-bounds .rodata read on a negative residue
// ===========================================================================

#[test]
fn err34_inreftree_negative_modulo_oob_read() {
    let (p, _g) = fresh();
    // Hand-picked params giving each negative residue.
    for (a, b, c, d, res) in [
        (-1i32, 0i32, 0i32, 0i32, -1i32),
        (-2, 0, 0, 0, -2),
        (-3, 0, 0, 0, -3),
        (-5, 0, 0, 0, -1),
        (-6, 0, 0, 0, -2),
        (-7, 0, 0, 0, -3),
        (-1, 1, 0, 0, 0),
        (-9, 1, 1, 1, -2),
        (i32::MIN, 0, 0, 0, 0),
        (i32::MIN + 1, 0, 0, 0, -3),
        (i32::MIN + 2, 0, 0, 0, -2),
        (i32::MIN + 3, 0, 0, 0, -1),
    ] {
        let sum = a.wrapping_add(b).wrapping_add(c).wrapping_add(d);
        assert_eq!(sum % 4, res, "err34: residue setup for ({a},{b},{c},{d})");
        let (rc, rr) = unsafe { ((p.c.inreftree)(a, b, c, d), (p.r.inreftree)(a, b, c, d)) };
        assert_eq!(rc, rr, "err34: inreftree({a},{b},{c},{d}) C={rc} Rust={rr}");
        if res < 0 {
            // op_string[-1..-3] is '\0','t','f' -> none is an operator -> OP_ADD
            let target = if b == 0 { 1 } else { 2 };
            assert_eq!(rc, sum.wrapping_add(target), "err34: OOB byte must map to OP_ADD");
        }
    }
    // exhaustive sweep over a window of negative sums
    for a in -400i32..=400 {
        let (rc, rr) = unsafe { ((p.c.inreftree)(a, 0, 0, 0), (p.r.inreftree)(a, 0, 0, 0)) };
        assert_eq!(rc, rr, "err34/sweep: inreftree({a},0,0,0) C={rc} Rust={rr}");
    }
}

// ===========================================================================
// Row 35 — inreftree resets node_count first
// ===========================================================================

#[test]
fn err35_inreftree_ignores_stale_state() {
    let (p, _g) = fresh();
    let mut rng = Rng::new(35);
    for i in 0..200 {
        let mut nodes = vec![TreeNode::default(); MAX_NODES];
        for n in nodes.iter_mut() {
            n.id = rng.spicy_i32();
            n.value = rng.spicy_i32();
            n.parent_id = rng.spicy_i32();
            n.left_child_id = rng.spicy_i32();
            n.right_child_id = rng.spicy_i32();
            n.label = [b'l'; 32]; // stale labels full of 'l'
        }
        p.c.set_table(&nodes);
        p.r.set_table(&nodes);
        let bogus = [0i32, 1, 3, 17, 49, 50][(i % 6) as usize];
        p.c.set_count(bogus);
        p.r.set_count(bogus);
        let (a, b, c, d) = (rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
        let (rc, rr) = unsafe { ((p.c.inreftree)(a, b, c, d), (p.r.inreftree)(a, b, c, d)) };
        assert_eq!(rc, rr, "err35: inreftree({a},{b},{c},{d}) stale count={bogus}");
        assert_state_eq(p, "err35");
        assert_eq!(p.c.count(), 4, "err35: node_count must be reset to 4");
        // rows 4..49 keep their stale contents in both libraries
        assert_eq!(p.c.table()[10], p.r.table()[10]);
    }
}

// ===========================================================================
// Generic FFI boundary cases (null pointers, oversized lengths) beyond the table
// ===========================================================================

#[test]
fn err_generic_null_and_oversized_inputs() {
    let (p, _g) = fresh();
    // parse_operation(NULL) — row 26, re-checked here as the generic null case.
    assert_eq!(
        unsafe { (p.c.parse_operation)(std::ptr::null()) },
        unsafe { (p.r.parse_operation)(std::ptr::null()) }
    );
    // Very long label (8 KiB) — strncpy must still stop at 31.
    let huge = vec![b'Q'; 8192];
    p.c.zero_state();
    p.r.zero_state();
    let rc = add(&p.c, 1, 1, -1, &huge);
    let rr = add(&p.r, 1, 1, -1, &huge);
    assert_eq!(rc, rr);
    assert_state_eq(p, "err_generic/huge label");
    // Very long parse_operation input.
    let mut long = vec![b'z'; 100_000];
    long.push(b'%');
    assert_eq!(parse_both(p, &long, "err_generic/long parse"), OP_MODULO);
    // node_count exactly at each boundary of the MAX_NODES check.
    for count in [48i32, 49, 50] {
        p.c.zero_state();
        p.r.zero_state();
        p.c.set_count(count);
        p.r.set_count(count);
        let rc = add(&p.c, 1, 1, -1, b"b");
        let rr = add(&p.r, 1, 1, -1, b"b");
        assert_eq!(rc, rr, "err_generic: boundary count={count}");
        assert_eq!(rc, if count == 50 { -1 } else { count }, "err_generic: count={count}");
        assert_state_eq(p, "err_generic/boundary");
    }
}

// ===========================================================================
// Rows 8, 18, 21, 23 — process-killing rows, run in an identical C host
// ===========================================================================

#[test]
fn err08_null_label_segv_subprocess() {
    crash_host::assert_same_termination("null_label");
}

#[test]
fn err18_tree_sum_cycle_segv_subprocess() {
    crash_host::assert_same_termination("cycle");
}

#[test]
fn err21_intmin_div_sigfpe_subprocess() {
    crash_host::assert_same_termination("div_intmin");
}

#[test]
fn err23_intmin_rem_sigfpe_subprocess() {
    crash_host::assert_same_termination("rem_intmin");
}

/// Sanity: the C host harness itself agrees on a case that does NOT crash.
#[test]
fn err_crash_host_selftest() {
    crash_host::assert_same_termination("ok");
}
