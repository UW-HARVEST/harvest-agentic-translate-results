//! Phase B — valid-path differential tests.
//!
//! One test per row of CONFIGS.md. Every call is dispatched through `dlsym`
//! into the C `.so` and into the Rust `.so`; return values *and* the complete
//! 2604-byte global state (`node_count` + `node_table`) are compared.

mod common;

use common::{harness, Rng, MAX_NODES};

const SEED: u64 = 0x5EED_1234_ABCD_0001;

fn boundary_grid() -> Vec<i32> {
    vec![i32::MIN, i32::MIN + 1, -3, -2, -1, 0, 1, 2, 3, i32::MAX - 1, i32::MAX]
}

// ===========================================================================
// Rows 1..11 — the five arithmetic primitives
// ===========================================================================

// row 1
#[test]
fn row01_add_op_randomized() {
    let (_g, p) = harness();
    let mut rng = Rng::new(SEED ^ 1);
    for _ in 0..2000 {
        let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
        let (u1, u2) = (rng.next_i32(), rng.next_i32());
        let cv = unsafe { (p.c.add_op)(a, b, u1, u2) };
        let rv = unsafe { (p.r.add_op)(a, b, u1, u2) };
        assert_eq!(cv, rv, "add_op({a},{b},{u1},{u2})");
    }
    p.assert_state("row01");
}

// row 2
#[test]
fn row02_add_op_boundary_grid() {
    let (_g, p) = harness();
    for &a in &boundary_grid() {
        for &b in &boundary_grid() {
            let cv = unsafe { (p.c.add_op)(a, b, 0, 0) };
            let rv = unsafe { (p.r.add_op)(a, b, 0, 0) };
            assert_eq!(cv, rv, "add_op({a},{b})");
        }
    }
}

// row 3
#[test]
fn row03_multiply_op_randomized() {
    let (_g, p) = harness();
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..2000 {
        let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
        let cv = unsafe { (p.c.multiply_op)(a, b, 7, -9) };
        let rv = unsafe { (p.r.multiply_op)(a, b, 7, -9) };
        assert_eq!(cv, rv, "multiply_op({a},{b})");
    }
}

// row 4
#[test]
fn row04_multiply_op_boundary_grid() {
    let (_g, p) = harness();
    for &a in &boundary_grid() {
        for &b in &boundary_grid() {
            let cv = unsafe { (p.c.multiply_op)(a, b, 0, 0) };
            let rv = unsafe { (p.r.multiply_op)(a, b, 0, 0) };
            assert_eq!(cv, rv, "multiply_op({a},{b})");
        }
    }
}

// row 5
#[test]
fn row05_subtract_op_randomized() {
    let (_g, p) = harness();
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..2000 {
        let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
        let cv = unsafe { (p.c.subtract_op)(a, b, 0, 0) };
        let rv = unsafe { (p.r.subtract_op)(a, b, 0, 0) };
        assert_eq!(cv, rv, "subtract_op({a},{b})");
    }
}

// row 6
#[test]
fn row06_subtract_op_boundary_grid() {
    let (_g, p) = harness();
    for &a in &boundary_grid() {
        for &b in &boundary_grid() {
            let cv = unsafe { (p.c.subtract_op)(a, b, 0, 0) };
            let rv = unsafe { (p.r.subtract_op)(a, b, 0, 0) };
            assert_eq!(cv, rv, "subtract_op({a},{b})");
        }
    }
}

/// `INT_MIN / -1` and `INT_MIN % -1` trap on x86-64 (see ERRORS.md rows 32/33,
/// exercised out-of-process in `phase_c_crash.rs`), so the in-process valid-path
/// tests must skip exactly that pair.
fn traps(a: i32, b: i32) -> bool {
    b == 0 || (a == i32::MIN && b == -1)
}

// row 7
#[test]
fn row07_divide_op_randomized() {
    let (_g, p) = harness();
    let mut rng = Rng::new(SEED ^ 7);
    let mut n = 0;
    while n < 2000 {
        let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
        if traps(a, b) {
            continue;
        }
        n += 1;
        let cv = unsafe { (p.c.divide_op)(a, b, 0, 0) };
        let rv = unsafe { (p.r.divide_op)(a, b, 0, 0) };
        assert_eq!(cv, rv, "divide_op({a},{b})");
    }
}

// row 8
#[test]
fn row08_divide_op_boundary_grid() {
    let (_g, p) = harness();
    for &a in &boundary_grid() {
        for &b in &boundary_grid() {
            if traps(a, b) {
                continue;
            }
            let cv = unsafe { (p.c.divide_op)(a, b, 0, 0) };
            let rv = unsafe { (p.r.divide_op)(a, b, 0, 0) };
            assert_eq!(cv, rv, "divide_op({a},{b})");
        }
    }
}

// row 9
#[test]
fn row09_modulo_op_randomized() {
    let (_g, p) = harness();
    let mut rng = Rng::new(SEED ^ 9);
    let mut n = 0;
    while n < 2000 {
        let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
        if traps(a, b) {
            continue;
        }
        n += 1;
        let cv = unsafe { (p.c.modulo_op)(a, b, 0, 0) };
        let rv = unsafe { (p.r.modulo_op)(a, b, 0, 0) };
        assert_eq!(cv, rv, "modulo_op({a},{b})");
    }
}

// row 10
#[test]
fn row10_modulo_op_boundary_grid() {
    let (_g, p) = harness();
    for &a in &boundary_grid() {
        for &b in &boundary_grid() {
            if traps(a, b) {
                continue;
            }
            let cv = unsafe { (p.c.modulo_op)(a, b, 0, 0) };
            let rv = unsafe { (p.r.modulo_op)(a, b, 0, 0) };
            assert_eq!(cv, rv, "modulo_op({a},{b})");
        }
    }
}

// row 11
#[test]
fn row11_unused_params_are_ignored_identically() {
    let (_g, p) = harness();
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..500 {
        let (a, mut b) = (rng.interesting_i32(), rng.interesting_i32());
        if traps(a, b) {
            b = 3;
        }
        let (u1, u2) = (rng.next_i32(), rng.next_i32());
        let cs = unsafe {
            [
                (p.c.add_op)(a, b, u1, u2),
                (p.c.multiply_op)(a, b, u1, u2),
                (p.c.subtract_op)(a, b, u1, u2),
                (p.c.divide_op)(a, b, u1, u2),
                (p.c.modulo_op)(a, b, u1, u2),
            ]
        };
        let rs = unsafe {
            [
                (p.r.add_op)(a, b, u1, u2),
                (p.r.multiply_op)(a, b, u1, u2),
                (p.r.subtract_op)(a, b, u1, u2),
                (p.r.divide_op)(a, b, u1, u2),
                (p.r.modulo_op)(a, b, u1, u2),
            ]
        };
        assert_eq!(cs, rs, "ops({a},{b},{u1},{u2})");
        // and the unused params really are unused: same result with 0,0
        let zc = unsafe { (p.c.add_op)(a, b, 0, 0) };
        assert_eq!(cs[0], zc, "add_op unused-param sensitivity");
    }
}

// ===========================================================================
// Row 12 — get_operation_func over the valid enum range
// ===========================================================================

#[test]
fn row12_get_operation_func_valid_range() {
    let (_g, p) = harness();
    let mut rng = Rng::new(SEED ^ 12);
    for op in 1..=5i32 {
        let ci = p.c.op_index(op);
        let ri = p.r.op_index(op);
        assert_eq!(ci, ri, "get_operation_func({op}) resolved to a different symbol");
        assert_eq!(ci, (op - 1) as usize, "get_operation_func({op}) index");
        // and calling the returned pointer must agree
        for _ in 0..200 {
            let a = rng.interesting_i32();
            let mut b = rng.interesting_i32();
            if traps(a, b) {
                b = 5;
            }
            let cv = p.c.call_op(op, a, b, 0, 0);
            let rv = p.r.call_op(op, a, b, 0, 0);
            assert_eq!(cv, rv, "get_operation_func({op})({a},{b})");
        }
    }
}

// ===========================================================================
// Rows 13..17 — parse_operation
// ===========================================================================

// row 13
#[test]
fn row13_parse_operation_single_operator() {
    let (_g, p) = harness();
    for (s, want) in [
        (&b"+"[..], 1),
        (&b"*"[..], 2),
        (&b"-"[..], 3),
        (&b"/"[..], 4),
        (&b"%"[..], 5),
    ] {
        let cv = p.c.parse(s);
        let rv = p.r.parse(s);
        assert_eq!(cv, rv, "parse_operation({:?})", s);
        assert_eq!(cv, want, "C reference sanity for {:?}", s);
    }
}

// row 14
#[test]
fn row14_parse_operation_operator_not_first() {
    let (_g, p) = harness();
    for s in [
        &b"a+"[..],
        &b"xx*"[..],
        &b"..-"[..],
        &b"zz/"[..],
        &b"q%"[..],
        &b"0123456789abcdef+"[..],
        &b"trailing%"[..],
    ] {
        assert_eq!(p.c.parse(s), p.r.parse(s), "parse_operation({:?})", s);
    }
}

// row 15
#[test]
fn row15_parse_operation_precedence() {
    let (_g, p) = harness();
    let ops = [b'+', b'*', b'-', b'/', b'%'];
    let mut cases: Vec<Vec<u8>> = vec![b"+*-/%".to_vec(), b"%/-*+".to_vec()];
    for i in 0..5 {
        for j in 0..5 {
            cases.push(vec![ops[i], ops[j]]);
            cases.push(vec![b'x', ops[i], b'y', ops[j], b'z']);
        }
    }
    for s in &cases {
        assert_eq!(p.c.parse(s), p.r.parse(s), "parse_operation({:?})", s);
    }
}

// row 16
#[test]
fn row16_parse_operation_random_ascii() {
    let (_g, p) = harness();
    let mut rng = Rng::new(SEED ^ 16);
    let alphabet: &[u8] = b"+*-/%az0 ";
    for _ in 0..4000 {
        let len = rng.below(25);
        let s = rng.bytes(len, alphabet);
        assert_eq!(p.c.parse(&s), p.r.parse(&s), "parse_operation({:?})", s);
    }
}

// row 17
#[test]
fn row17_parse_operation_random_high_bit_bytes() {
    let (_g, p) = harness();
    let mut rng = Rng::new(SEED ^ 17);
    for _ in 0..4000 {
        let len = 1 + rng.below(24);
        // any non-zero byte, heavily weighted towards >= 0x80
        let s: Vec<u8> = (0..len)
            .map(|_| {
                let b = (rng.next_u64() & 0xff) as u8;
                if b == 0 {
                    0x80
                } else {
                    b
                }
            })
            .collect();
        assert_eq!(p.c.parse(&s), p.r.parse(&s), "parse_operation({:?})", s);
    }
}

// ===========================================================================
// Rows 18..23 — find_node_by_id
// ===========================================================================

// row 18
#[test]
fn row18_find_empty_table() {
    let (_g, p) = harness();
    for id in [0, 1, -1, 42, i32::MIN, i32::MAX] {
        p.diff(&format!("find({id}) empty"), |l| l.find(id));
    }
}

// row 19
#[test]
fn row19_find_single_node() {
    let (_g, p) = harness();
    p.diff("add root", |l| l.add(7, 100, -1, b"root"));
    for id in [7, 8, 0, -1, i32::MIN] {
        p.diff(&format!("find({id}) one node"), |l| l.find(id));
    }
}

// row 20
#[test]
fn row20_find_ten_nodes_positions() {
    let (_g, p) = harness();
    for i in 0..10i32 {
        p.diff(&format!("add {i}"), |l| l.add(i, i * 3, -1, b"n"));
    }
    for id in [0, 5, 9, 10, -1, 100] {
        p.diff(&format!("find({id}) ten nodes"), |l| l.find(id));
    }
}

// row 21
#[test]
fn row21_find_full_table() {
    let (_g, p) = harness();
    for i in 0..MAX_NODES as i32 {
        p.diff(&format!("add {i}"), |l| l.add(i, i, -1, b"x"));
    }
    assert_eq!(p.c.count(), 50);
    for id in 0..MAX_NODES as i32 {
        p.diff(&format!("find({id}) full"), |l| l.find(id));
    }
    for id in [50, 51, -1, i32::MAX] {
        p.diff(&format!("find({id}) absent"), |l| l.find(id));
    }
}

// row 22
#[test]
fn row22_find_duplicate_ids_first_wins() {
    let (_g, p) = harness();
    for k in 0..3 {
        p.diff(&format!("dup add {k}"), |l| l.add(7, 10 + k, -1, b"dup"));
    }
    p.diff("find dup", |l| l.find(7));
    assert_eq!(p.c.find(7), Some(0), "C reference: first match wins");
}

// row 23
#[test]
fn row23_find_random_ids() {
    let (_g, p) = harness();
    let mut rng = Rng::new(SEED ^ 23);
    for trial in 0..200 {
        p.reset();
        let n = 1 + rng.below(MAX_NODES);
        let mut ids = Vec::new();
        for _ in 0..n {
            let id = rng.interesting_i32();
            ids.push(id);
            let v = rng.interesting_i32();
            p.diff(&format!("t{trial} add {id}"), |l| l.add(id, v, -1, b"r"));
        }
        for &id in &ids {
            p.diff(&format!("t{trial} find({id})"), |l| l.find(id));
        }
        for _ in 0..5 {
            let id = rng.interesting_i32();
            p.diff(&format!("t{trial} find rand({id})"), |l| l.find(id));
        }
    }
}

// ===========================================================================
// Rows 24..31 — add_tree_node
// ===========================================================================

// row 24
#[test]
fn row24_add_root_only() {
    let (_g, p) = harness();
    p.diff("root", |l| l.add(1, 12345, -1, b"root"));
    assert_eq!(p.c.count(), 1);
    assert_eq!(p.c.node_bytes(0), p.r.node_bytes(0));
}

// row 25
#[test]
fn row25_add_child_left_slot() {
    let (_g, p) = harness();
    p.diff("root", |l| l.add(1, 5, -1, b"root"));
    p.diff("left", |l| l.add(2, 6, 1, b"left"));
}

// row 26
#[test]
fn row26_add_child_right_slot() {
    let (_g, p) = harness();
    p.diff("root", |l| l.add(1, 5, -1, b"root"));
    p.diff("left", |l| l.add(2, 6, 1, b"left"));
    p.diff("right", |l| l.add(3, 7, 1, b"right"));
}

// row 27
#[test]
fn row27_add_third_child_link_dropped() {
    let (_g, p) = harness();
    p.diff("root", |l| l.add(1, 5, -1, b"root"));
    p.diff("left", |l| l.add(2, 6, 1, b"left"));
    p.diff("right", |l| l.add(3, 7, 1, b"right"));
    p.diff("third", |l| l.add(4, 8, 1, b"third"));
    // C reference: parent's links unchanged, third child still inserted
    assert_eq!(p.c.count(), 4);
    p.assert_state("row27");
}

// row 28
#[test]
fn row28_add_label_lengths() {
    let (_g, p) = harness();
    for len in [0usize, 1, 5, 29, 30, 31, 32, 33, 40, 64] {
        p.reset();
        let label: Vec<u8> = (0..len).map(|i| b'a' + (i % 26) as u8).collect();
        p.diff(&format!("label len {len}"), |l| l.add(9, 1, -1, &label));
        assert_eq!(
            p.c.node_bytes(0),
            p.r.node_bytes(0),
            "node bytes differ for label len {len}"
        );
    }
}

// row 29
#[test]
fn row29_add_labels_with_special_chars() {
    let (_g, p) = harness();
    for label in [
        &b"root"[..],
        &b"left"[..],
        &b"L"[..],
        &b"no-ell-here"[..],
        &b"xyz"[..],
        &b"+"[..],
        &b"*-/%"[..],
        &b"l"[..],
        &b"\x80\xffl"[..],
        &b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaal"[..], // 'l' at index 30
        &b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaal"[..], // 'l' at index 31 -> truncated away
    ] {
        p.reset();
        p.diff(&format!("label {:?}", label), |l| l.add(1, 1, -1, label));
        assert_eq!(p.c.node_bytes(0), p.r.node_bytes(0));
    }
}

// row 30
#[test]
fn row30_fill_table_step_by_step() {
    let (_g, p) = harness();
    for i in 0..MAX_NODES as i32 {
        p.diff(&format!("insert {i}"), |l| l.add(i + 1, i * 7 - 3, -1, b"fill"));
        assert_eq!(p.c.count(), i + 1);
        p.assert_state(&format!("after insert {i}"));
    }
    // one past the limit (ERRORS row 6 too, checked here for the state)
    p.diff("insert 51st", |l| l.add(999, 999, -1, b"overflow"));
    assert_eq!(p.c.count(), 50);
    p.assert_state("after 51st");
}

// row 31
#[test]
fn row31_random_insert_sequences() {
    let (_g, p) = harness();
    let mut rng = Rng::new(SEED ^ 31);
    let labels: [&[u8]; 6] = [b"root", b"left", b"right", b"left-left", b"", b"zzz"];
    for trial in 0..300 {
        p.reset();
        let n = 1 + rng.below(MAX_NODES + 3);
        let mut used: Vec<i32> = Vec::new();
        for k in 0..n {
            let id = if used.is_empty() || rng.below(4) != 0 {
                rng.range_i32(-4, 60)
            } else {
                used[rng.below(used.len())]
            };
            let parent = match rng.below(4) {
                0 => -1,
                1 if !used.is_empty() => used[rng.below(used.len())],
                2 => rng.range_i32(-4, 60),
                _ => -1,
            };
            let value = rng.interesting_i32();
            let label = labels[rng.below(labels.len())];
            let ctx = format!("t{trial} step{k} add({id},{value},{parent},{label:?})");
            p.diff(&ctx, |l| l.add(id, value, parent, label));
            used.push(id);
        }
        // and probe the resulting table
        for _ in 0..8 {
            let id = rng.range_i32(-6, 62);
            p.diff(&format!("t{trial} find({id})"), |l| l.find(id));
        }
    }
}

// ===========================================================================
// Rows 32..39 — calculate_tree_sum
// ===========================================================================

// row 32
#[test]
fn row32_sum_single_node() {
    let (_g, p) = harness();
    p.diff("root", |l| l.add(1, 41, -1, b"root"));
    p.diff("sum(1)", |l| l.sum(1));
}

// row 33
#[test]
fn row33_sum_left_child_only() {
    let (_g, p) = harness();
    p.diff("root", |l| l.add(1, 10, -1, b"root"));
    p.diff("l", |l| l.add(2, 20, 1, b"l"));
    p.diff("sum(1)", |l| l.sum(1));
    p.diff("sum(2)", |l| l.sum(2));
}

// row 34
#[test]
fn row34_sum_right_child_only() {
    let (_g, p) = harness();
    p.diff("root", |l| l.add(1, 10, -1, b"root"));
    // occupy the left slot with a node, then clear it so only right is set
    p.diff("l", |l| l.add(2, 20, 1, b"l"));
    p.diff("r", |l| l.add(3, 30, 1, b"r"));
    // clear the left link in both libraries identically
    for l in [&p.c, &p.r] {
        l.poke_node(0, 1, 10, -1, -1, 3, b"root");
    }
    p.assert_state("poked");
    p.diff("sum(1) right only", |l| l.sum(1));
}

// row 35
#[test]
fn row35_sum_full_binary_trees() {
    let (_g, p) = harness();
    // depth 2
    p.diff("1", |l| l.add(1, 1, -1, b"a"));
    p.diff("2", |l| l.add(2, 2, 1, b"b"));
    p.diff("3", |l| l.add(3, 4, 1, b"c"));
    p.diff("sum d2", |l| l.sum(1));
    // depth 3
    p.diff("4", |l| l.add(4, 8, 2, b"d"));
    p.diff("5", |l| l.add(5, 16, 2, b"e"));
    p.diff("6", |l| l.add(6, 32, 3, b"f"));
    p.diff("7", |l| l.add(7, 64, 3, b"g"));
    p.diff("sum d3", |l| l.sum(1));
    for id in 1..=7 {
        p.diff(&format!("sum({id})"), |l| l.sum(id));
    }
}

// row 36
#[test]
fn row36_sum_deep_chain() {
    let (_g, p) = harness();
    p.diff("root", |l| l.add(1, 1, -1, b"c"));
    for i in 2..=MAX_NODES as i32 {
        p.diff(&format!("chain {i}"), |l| l.add(i, i, i - 1, b"c"));
    }
    assert_eq!(p.c.count(), 50);
    for id in 1..=MAX_NODES as i32 {
        p.diff(&format!("sum({id})"), |l| l.sum(id));
    }
}

// row 37
#[test]
fn row37_sum_dangling_child_id() {
    let (_g, p) = harness();
    p.diff("root", |l| l.add(1, 100, -1, b"root"));
    for l in [&p.c, &p.r] {
        l.poke_node(0, 1, 100, -1, 77, 88, b"root");
    }
    p.assert_state("poked dangling");
    p.diff("sum(1) dangling", |l| l.sum(1));
}

// row 38
#[test]
fn row38_sum_overflow_and_negative_values() {
    let (_g, p) = harness();
    for (a, b, c) in [
        (i32::MAX, i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN, i32::MIN),
        (i32::MAX, 1, 1),
        (i32::MIN, -1, -1),
        (-5, -6, -7),
        (0, 0, 0),
    ] {
        p.reset();
        p.diff("root", |l| l.add(1, a, -1, b"root"));
        p.diff("l", |l| l.add(2, b, 1, b"l"));
        p.diff("r", |l| l.add(3, c, 1, b"r"));
        p.diff(&format!("sum overflow ({a},{b},{c})"), |l| l.sum(1));
    }
}

// row 39
#[test]
fn row39_sum_random_trees() {
    let (_g, p) = harness();
    let mut rng = Rng::new(SEED ^ 39);
    for trial in 0..300 {
        p.reset();
        let n = 1 + rng.below(MAX_NODES);
        let mut ids: Vec<i32> = Vec::new();
        for k in 0..n {
            let id = k as i32 + 1;
            let parent = if ids.is_empty() || rng.below(5) == 0 {
                -1
            } else {
                ids[rng.below(ids.len())]
            };
            let value = rng.interesting_i32();
            p.diff(&format!("t{trial} add {id}"), |l| l.add(id, value, parent, b"n"));
            ids.push(id);
        }
        for id in 0..=(n as i32 + 2) {
            p.diff(&format!("t{trial} sum({id})"), |l| l.sum(id));
        }
    }
}

// ===========================================================================
// Row 40 — composed low-level pipeline
// ===========================================================================

#[test]
fn row40_composed_pipeline() {
    let (_g, p) = harness();
    let mut rng = Rng::new(SEED ^ 40);
    let op_chars: &[u8] = b"+*-/%";
    for trial in 0..400 {
        p.reset();
        let n = 1 + rng.below(12);
        for k in 0..n {
            let id = k as i32 + 1;
            let parent = if k == 0 { -1 } else { rng.range_i32(0, k as i32) };
            let value = rng.range_i32(-1000, 1000);
            p.diff(&format!("t{trial} add {id}"), |l| l.add(id, value, parent, b"node"));
        }
        let probe = rng.range_i32(0, n as i32 + 1);
        p.diff(&format!("t{trial} find({probe})"), |l| l.find(probe));
        p.diff(&format!("t{trial} sum({probe})"), |l| l.sum(probe));

        // parse -> get_operation_func -> invoke, driven by the computed sum
        let sum_c = p.c.sum(1);
        let sum_r = p.r.sum(1);
        assert_eq!(sum_c, sum_r, "t{trial} sum(1)");
        let idx = (sum_c.rem_euclid(5)) as usize;
        let s = [op_chars[idx]];
        let op_c = p.c.parse(&s);
        let op_r = p.r.parse(&s);
        assert_eq!(op_c, op_r, "t{trial} parse({:?})", s);
        assert_eq!(p.c.op_index(op_c), p.r.op_index(op_r), "t{trial} op_index");
        let mut b = probe;
        if traps(sum_c, b) {
            b = 1;
        }
        let rc = p.c.call_op(op_c, sum_c, b, 0, 0);
        let rr = p.r.call_op(op_r, sum_r, b, 0, 0);
        assert_eq!(rc, rr, "t{trial} call_op({op_c},{sum_c},{b})");
        p.assert_state(&format!("t{trial} pipeline"));
    }
}

// ===========================================================================
// Rows 41..47 — inreftree
// ===========================================================================

// row 41
#[test]
fn row41_inreftree_all_zero() {
    let (_g, p) = harness();
    p.diff("inreftree(0,0,0,0)", |l| l.inreftree(0, 0, 0, 0));
    assert_eq!(p.c.inreftree(0, 0, 0, 0), 1, "C reference sanity");
}

// row 42 / 43
#[test]
fn row42_43_inreftree_residues() {
    let (_g, p) = harness();
    // param2 != 0  -> target_id = 2 ; param2 == 0 -> target_id = 1
    for residue in 0..4i32 {
        for &p2 in &[0i32, 1] {
            // choose p1 so that p1+p2+p3+p4 == residue
            let (p3, p4) = (0, 0);
            let p1 = residue - p2 - p3 - p4;
            let ctx = format!("inreftree({p1},{p2},{p3},{p4}) residue {residue}");
            p.diff(&ctx, |l| l.inreftree(p1, p2, p3, p4));
        }
    }
}

// row 44
#[test]
fn row44_inreftree_negative_residues() {
    let (_g, p) = harness();
    for sum in [-1i32, -2, -3, -5, -6, -7, -9, -10, -11, -100, -1001] {
        for &p2 in &[0i32, 1, -1] {
            let p1 = sum - p2;
            let ctx = format!("inreftree({p1},{p2},0,0) sum {sum}");
            p.diff(&ctx, |l| l.inreftree(p1, p2, 0, 0));
        }
    }
}

// row 45
#[test]
fn row45_inreftree_exhaustive_boundary_grid() {
    let (_g, p) = harness();
    let vals = [i32::MIN, i32::MIN + 1, -2, -1, 0, 1, 2, i32::MAX - 1, i32::MAX];
    for &a in &vals {
        for &b in &vals {
            for &c in &vals {
                for &d in &vals {
                    let cv = p.c.inreftree(a, b, c, d);
                    let rv = p.r.inreftree(a, b, c, d);
                    assert_eq!(cv, rv, "inreftree({a},{b},{c},{d})");
                }
            }
        }
    }
    p.assert_state("row45");
}

// row 46
#[test]
fn row46_inreftree_randomized() {
    let (_g, p) = harness();
    let mut rng = Rng::new(SEED ^ 46);
    for _ in 0..20000 {
        let (a, b, c, d) = (
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
        let cv = p.c.inreftree(a, b, c, d);
        let rv = p.r.inreftree(a, b, c, d);
        assert_eq!(cv, rv, "inreftree({a},{b},{c},{d})");
    }
    p.assert_state("row46");
}

// row 47
#[test]
fn row47_inreftree_over_dirty_state() {
    let (_g, p) = harness();
    // dirty the table with 50 unrelated nodes so that stale bytes remain in
    // slots 4..49 after inreftree resets node_count to 0.
    for i in 0..MAX_NODES as i32 {
        p.diff(&format!("dirty {i}"), |l| {
            l.add(100 + i, i * 13, -1, b"dirty-label-with-l")
        });
    }
    assert_eq!(p.c.count(), 50);
    p.diff("inreftree over dirty", |l| l.inreftree(3, 4, 5, 6));
    p.diff("inreftree again", |l| l.inreftree(3, 4, 5, 6));
    p.diff("inreftree third", |l| l.inreftree(-7, 0, 0, 0));
    // interleave with low-level calls
    p.diff("add after", |l| l.add(77, 7, -1, b"after"));
    p.diff("find 77", |l| l.find(77));
    p.diff("sum 1", |l| l.sum(1));
    p.diff("inreftree fourth", |l| l.inreftree(1, 1, 1, 1));
}

// ===========================================================================
// Row 48 — the exported data symbols themselves
// ===========================================================================

#[test]
fn row48_exported_globals_start_zeroed_and_track() {
    let (_g, p) = harness();
    assert_eq!(p.c.count(), 0);
    assert_eq!(p.r.count(), 0);
    assert!(p.c.table().iter().all(|&b| b == 0));
    assert!(p.r.table().iter().all(|&b| b == 0));
    p.assert_state("pristine");

    // writing node_count from outside must be observed by both libraries
    p.diff("add a", |l| l.add(1, 1, -1, b"a"));
    p.diff("add b", |l| l.add(2, 2, 1, b"b"));
    for l in [&p.c, &p.r] {
        l.set_count(1);
    }
    p.diff("find(2) with count=1", |l| l.find(2));
    p.diff("sum(1) with count=1", |l| l.sum(1));
    for l in [&p.c, &p.r] {
        l.set_count(2);
    }
    p.diff("find(2) with count=2", |l| l.find(2));
    p.diff("sum(1) with count=2", |l| l.sum(1));
}

// ===========================================================================
// Rows 50..53 — extra state/shape combinations found while auditing the C
// ===========================================================================

// row 50: node_count larger than the number of real inserts, so the scan walks
// into still-zeroed slots (whose id field is 0).
#[test]
fn row50_count_beyond_real_inserts_scans_zeroed_slots() {
    let (_g, p) = harness();
    for i in 1..=3i32 {
        p.diff(&format!("add {i}"), |l| l.add(i, i * 10, -1, b"n"));
    }
    for count in [3i32, 4, 10, 49, 50] {
        for l in [&p.c, &p.r] {
            l.set_count(count);
        }
        for id in [0i32, 1, 2, 3, 4] {
            p.diff(&format!("count={count} find({id})"), |l| l.find(id));
            // sum(0) would recurse forever once a zeroed slot (id 0, children 0)
            // becomes visible -- shared UB, guarded by `diff_sum`.
            p.diff_sum(&format!("count={count} sum({id})"), id);
        }
    }
    for l in [&p.c, &p.r] {
        l.set_count(3);
    }
    p.assert_state("row50");
}

// row 51: inreftree must reset node_count no matter what it was set to.
#[test]
fn row51_inreftree_resets_arbitrary_node_count() {
    let (_g, p) = harness();
    for i in 1..=6i32 {
        p.diff(&format!("add {i}"), |l| l.add(i, i, -1, b"pre"));
    }
    for preset in [0i32, 1, 6, 49, 50, -1, -7, i32::MIN, 123] {
        for l in [&p.c, &p.r] {
            l.set_count(preset);
        }
        p.diff(&format!("inreftree after count={preset}"), |l| {
            l.inreftree(9, 8, 7, 6)
        });
        assert_eq!(p.c.count(), 4, "C: inreftree always ends with 4 nodes");
    }
}

// row 52: label buffer with no NUL inside the 31 bytes strncpy may read.
#[test]
fn row52_label_without_terminator_within_31_bytes() {
    let (_g, p) = harness();
    let mut rng = Rng::new(SEED ^ 52);
    for trial in 0..200 {
        p.reset();
        // exactly 31 readable non-NUL bytes, no terminator: strncpy stops at n
        let buf: Vec<u8> = (0..31)
            .map(|_| {
                let b = (rng.next_u64() & 0x7f) as u8;
                if b == 0 {
                    b'x'
                } else {
                    b
                }
            })
            .collect();
        let cv = unsafe { p.c.add_raw(1, 2, -1, buf.as_ptr() as *const std::ffi::c_char) };
        let rv = unsafe { p.r.add_raw(1, 2, -1, buf.as_ptr() as *const std::ffi::c_char) };
        assert_eq!(cv, rv, "t{trial} add_tree_node with unterminated label");
        p.assert_state(&format!("t{trial} unterminated label"));
    }
}

// row 53: long randomised mixed-API session — every entry point, interleaved,
// with the full global state compared after every single call.
#[test]
fn row53_long_mixed_api_session() {
    let (_g, p) = harness();
    let mut rng = Rng::new(SEED ^ 53);
    let labels: [&[u8]; 7] = [b"root", b"left", b"right", b"left-left", b"", b"l", b"NOELL"];
    for step in 0..8000 {
        match rng.below(8) {
            0 => {
                let (id, v) = (rng.range_i32(-3, 55), rng.interesting_i32());
                let par = if rng.below(3) == 0 { -1 } else { rng.range_i32(-3, 55) };
                let lab = labels[rng.below(labels.len())];
                p.diff(&format!("s{step} add({id},{v},{par})"), |l| l.add(id, v, par, lab));
            }
            1 => {
                let id = rng.range_i32(-5, 60);
                p.diff(&format!("s{step} find({id})"), |l| l.find(id));
            }
            2 => {
                let id = rng.range_i32(-5, 60);
                p.diff_sum(&format!("s{step} sum({id})"), id);
            }
            3 => {
                let n = rng.below(8);
                let s = rng.bytes(n, b"+*-/%abc");
                p.diff(&format!("s{step} parse({s:?})"), |l| l.parse(&s));
            }
            4 => {
                let op = if rng.below(2) == 0 {
                    rng.range_i32(1, 5)
                } else {
                    rng.next_i32()
                };
                p.diff(&format!("s{step} op_index({op})"), |l| l.op_index(op));
            }
            5 => {
                let (a, b, c, d) = (
                    rng.interesting_i32(),
                    rng.interesting_i32(),
                    rng.interesting_i32(),
                    rng.interesting_i32(),
                );
                p.diff(&format!("s{step} inreftree({a},{b},{c},{d})"), |l| {
                    l.inreftree(a, b, c, d)
                });
            }
            6 => {
                let mut op = rng.range_i32(1, 5);
                let a = rng.interesting_i32();
                let mut b = rng.interesting_i32();
                if traps(a, b) {
                    if op == 4 || op == 5 {
                        b = 6;
                    } else {
                        op = 1;
                    }
                }
                p.diff(&format!("s{step} call_op({op},{a},{b})"), |l| {
                    l.call_op(op, a, b, 0, 0)
                });
            }
            _ => {
                if rng.below(6) == 0 {
                    p.reset();
                } else {
                    let c = rng.range_i32(0, 50);
                    for l in [&p.c, &p.r] {
                        l.set_count(c);
                    }
                    p.assert_state(&format!("s{step} set_count({c})"));
                }
            }
        }
    }
}
