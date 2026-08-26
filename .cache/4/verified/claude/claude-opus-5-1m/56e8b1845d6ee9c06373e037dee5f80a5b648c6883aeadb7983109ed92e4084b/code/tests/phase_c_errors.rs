//! Phase C - error/rejection-path differential tests, one test per row of
//! `ERRORS.md`. Each asserts the C `.so` and the Rust `.so` return the SAME
//! sentinel/error value (and leave the same global state), not merely that both
//! "failed somehow".

mod common;
use common::*;
use std::ffi::{c_char, c_int};

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

#[track_caller]
fn both_add(p: &Pair, id: c_int, value: c_int, parent: c_int, label: &[u8]) -> c_int {
    let cv = p.c.add_node(id, value, parent, label);
    let rv = p.rust.add_node(id, value, parent, label);
    let ctx = format!("add_tree_node({id},{value},{parent},{:?})", String::from_utf8_lossy(label));
    assert_ret_eq(cv, rv, &ctx);
    assert_state_eq(p, &ctx);
    cv
}

#[track_caller]
fn both_sum(p: &Pair, id: c_int) -> c_int {
    let cv = (p.c.calculate_tree_sum)(id);
    let rv = (p.rust.calculate_tree_sum)(id);
    assert_ret_eq(cv, rv, &format!("calculate_tree_sum({id})"));
    cv
}

#[track_caller]
fn both_find(p: &Pair, id: c_int) -> Option<isize> {
    let cv = p.c.find_index(id);
    let rv = p.rust.find_index(id);
    assert_eq!(cv, rv, "find_node_by_id({id}): C={cv:?} Rust={rv:?}");
    cv
}

#[track_caller]
fn both_parse(p: &Pair, s: &[u8]) -> c_int {
    let cv = p.c.parse_op(s);
    let rv = p.rust.parse_op(s);
    assert_ret_eq(cv, rv, &format!("parse_operation({:?})", String::from_utf8_lossy(s)));
    cv
}

#[track_caller]
fn both_inreftree(p: &Pair, a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let cv = (p.c.inreftree)(a, b, c, d);
    let rv = (p.rust.inreftree)(a, b, c, d);
    let ctx = format!("inreftree({a},{b},{c},{d})");
    assert_ret_eq(cv, rv, &ctx);
    assert_state_eq(p, &ctx);
    cv
}

fn inject(p: &Pair, nodes: &[NodeView], count: c_int) {
    p.c.reset();
    p.rust.reset();
    for (i, n) in nodes.iter().enumerate() {
        p.c.set_node(i, n);
        p.rust.set_node(i, n);
    }
    p.c.set_count(count);
    p.rust.set_count(count);
    assert_state_eq(p, "after inject");
}

fn nv(id: i32, value: i32, l: i32, r: i32) -> NodeView {
    NodeView { id, value, parent_id: -1, left_child_id: l, right_child_id: r, label: [0u8; 32] }
}

// ===========================================================================
// Row 1-2: division / modulo by zero -> 0 (NOT a trap)
// ===========================================================================

#[test]
fn err01_divide_by_zero() {
    with_libs(|p| {
        let mut rng = Rng::new(SEED ^ 1);
        for &a in EDGE.iter() {
            assert_eq!((p.c.divide_op)(a, 0, 0, 0), 0, "C divide_op({a},0)");
            assert_eq!((p.rust.divide_op)(a, 0, 0, 0), 0, "Rust divide_op({a},0)");
        }
        for _ in 0..2000 {
            let a = rng.spicy_i32();
            let (u1, u2) = (rng.i32(), rng.i32());
            let cv = (p.c.divide_op)(a, 0, u1, u2);
            let rv = (p.rust.divide_op)(a, 0, u1, u2);
            assert_eq!((cv, rv), (0, 0), "divide_op({a},0,{u1},{u2})");
        }
    });
}

#[test]
fn err02_modulo_by_zero() {
    with_libs(|p| {
        let mut rng = Rng::new(SEED ^ 2);
        for &a in EDGE.iter() {
            assert_eq!((p.c.modulo_op)(a, 0, 0, 0), 0, "C modulo_op({a},0)");
            assert_eq!((p.rust.modulo_op)(a, 0, 0, 0), 0, "Rust modulo_op({a},0)");
        }
        for _ in 0..2000 {
            let a = rng.spicy_i32();
            let (u1, u2) = (rng.i32(), rng.i32());
            let cv = (p.c.modulo_op)(a, 0, u1, u2);
            let rv = (p.rust.modulo_op)(a, 0, u1, u2);
            assert_eq!((cv, rv), (0, 0), "modulo_op({a},0,{u1},{u2})");
        }
    });
}

// ===========================================================================
// Row 3-5: find_node_by_id rejections -> NULL
// ===========================================================================

#[test]
fn err03_find_absent_id() {
    with_libs(|p| {
        let nodes: Vec<NodeView> = (0..10).map(|i| nv(i * 2, i, -1, -1)).collect();
        inject(p, &nodes, 10);
        for id in [1, 3, 5, 19, 20, -1, -2, i32::MIN, i32::MAX] {
            assert_eq!(both_find(p, id), None, "absent id {id} must give NULL");
        }
        // present ones still found (so the NULLs above are meaningful)
        for i in 0..10 {
            assert_eq!(both_find(p, i * 2), Some(i as isize));
        }
    });
}

#[test]
fn err04_find_on_empty_table() {
    with_libs(|p| {
        p.c.set_count(0);
        p.rust.set_count(0);
        // even a row physically present in the table is invisible at count 0
        let n = nv(42, 7, -1, -1);
        p.c.set_node(0, &n);
        p.rust.set_node(0, &n);
        assert_eq!(both_find(p, 42), None, "count==0 -> NULL even for row 0");
        assert_eq!(both_find(p, 0), None, "id 0 (matches zeroed rows) -> NULL");
        assert_eq!(both_sum(p, 42), 0, "and the sum is 0");
    });
}

#[test]
fn err05_find_beyond_node_count() {
    with_libs(|p| {
        let nodes: Vec<NodeView> = (0..10).map(|i| nv(100 + i, i, -1, -1)).collect();
        inject(p, &nodes, 10);
        for count in 0..=10 {
            p.c.set_count(count);
            p.rust.set_count(count);
            for i in 0..10 {
                let got = both_find(p, 100 + i);
                assert_eq!(
                    got,
                    if i < count { Some(i as isize) } else { None },
                    "count={count}, id={} must be {} ",
                    100 + i,
                    if i < count { "found" } else { "NULL (past node_count)" }
                );
            }
        }
    });
}

// ===========================================================================
// Row 6-7: table full -> -1, nothing modified
// ===========================================================================

#[test]
fn err06_table_full() {
    with_libs(|p| {
        for i in 1..=MAX_NODES as c_int {
            assert_eq!(both_add(p, i, i, -1, b"x"), i - 1);
        }
        assert_eq!(p.c.get_count(), MAX_NODES as c_int);
        let img = p.c.table_image();
        let mut rng = Rng::new(SEED ^ 6);
        for _ in 0..200 {
            let r = both_add(p, rng.spicy_i32(), rng.spicy_i32(), -1, b"rejected");
            assert_eq!(r, -1, "insert 51+ must return -1");
            assert_eq!(p.c.get_count(), MAX_NODES as c_int, "node_count unchanged");
            assert_eq!(p.c.table_image(), img, "table unchanged");
            assert_eq!(p.rust.table_image(), img, "table unchanged (Rust)");
        }
        // also with a parent id that exists - still rejected before any write
        assert_eq!(both_add(p, 999, 1, 1, b"rejected"), -1);
        assert_eq!(p.c.table_image(), img);
    });
}

#[test]
fn err07_node_count_over_max() {
    with_libs(|p| {
        for count in [MAX_NODES as c_int, 51, 52, 60, 100, 1000, i32::MAX] {
            p.c.reset();
            p.rust.reset();
            p.c.set_count(count);
            p.rust.set_count(count);
            let img = p.c.table_image();
            let r = both_add(p, 5, 5, -1, b"nope");
            assert_eq!(r, -1, "node_count={count} must reject with -1");
            assert_eq!(p.c.get_count(), count, "node_count must not change");
            assert_eq!(p.rust.get_count(), count);
            assert_eq!(p.c.table_image(), img, "no write may happen");
            assert_eq!(p.rust.table_image(), img);
        }
    });
}

// ===========================================================================
// Row 8-11: add_tree_node parent handling
// ===========================================================================

#[test]
fn err08_parent_not_found_partial_write() {
    with_libs(|p| {
        assert_eq!(both_add(p, 1, 111, -1, b"root"), 0);
        let count_before = p.c.get_count();

        for bad_parent in [999, 0, 2, -2, i32::MIN, i32::MAX] {
            let r = both_add(p, 77, 888, bad_parent, b"orphan");
            assert_eq!(r, -1, "parent {bad_parent} absent -> -1");
            assert_eq!(p.c.get_count(), count_before, "node_count must NOT advance");
            // ...but row `node_count` HAS already been overwritten by the C code
            let n = p.c.node(count_before as usize);
            assert_eq!(
                (n.id, n.value, n.parent_id, n.left_child_id, n.right_child_id),
                (77, 888, bad_parent, -1, -1),
                "the C code writes the row BEFORE validating the parent"
            );
            assert_eq!(&n.label[..7], b"orphan\0");
            assert_eq!(p.rust.node(count_before as usize), n, "partial write must match");
        }
        // the scratch row stays invisible and gets overwritten by the next insert
        assert_eq!(both_find(p, 77), None);
        assert_eq!(both_add(p, 2, 222, 1, b"ok"), 1);
        assert_eq!(p.c.node(1).id, 2);
        assert_eq!(both_find(p, 77), None);
    });
}

#[test]
fn err09_parent_id_mismatch_unreachable() {
    with_libs(|p| {
        // `parent->id != parent_id` (2nd operand of the line-98 `||`) can never be
        // true, because find_node_by_id only ever returns a row whose id matches.
        // Proven over many states, for BOTH libraries.
        let mut rng = Rng::new(SEED ^ 9);
        for _ in 0..300 {
            let n = 1 + rng.below(MAX_NODES as u64) as usize;
            let nodes: Vec<NodeView> = (0..n)
                .map(|_| nv(rng.small(), rng.i32(), -1, -1))
                .collect();
            inject(p, &nodes, n as c_int);
            for _ in 0..20 {
                let q = rng.small();
                for lib in [&p.c, &p.rust] {
                    if let Some(idx) = lib.find_index(q) {
                        assert_eq!(
                            lib.node(idx as usize).id, q,
                            "{}: find_node_by_id({q}) returned a row with a different id",
                            lib.name
                        );
                    }
                }
            }
        }
        // Consequence: a successful lookup always takes the success path, so the
        // only way to get -1 from a non-sentinel parent is "not found" (row 8).
        p.c.reset();
        p.rust.reset();
        both_add(p, 4, 1, -1, b"a");
        assert_eq!(both_add(p, 5, 2, 4, b"b"), 1, "found parent -> success");
        assert_eq!(both_add(p, 6, 3, 9, b"c"), -1, "absent parent -> -1");
    });
}

#[test]
fn err10_parent_sentinel_minus_one() {
    with_libs(|p| {
        // parent_id == -1 skips the lookup entirely: it succeeds even when NO
        // node with id -1 exists, and even on an empty table.
        assert_eq!(both_find(p, -1), None, "no node has id -1");
        assert_eq!(both_add(p, 10, 1, -1, b"a"), 0, "-1 parent always succeeds");
        assert_eq!(both_add(p, 11, 2, -1, b"b"), 1);
        assert_eq!(p.c.node(0).left_child_id, -1, "no parent link written");
        assert_eq!(p.c.node(0).right_child_id, -1);
        assert_eq!(both_sum(p, 10), 1, "the two roots are separate trees");
        assert_eq!(both_sum(p, 11), 2);

        // A node whose *id* is -1 is still not usable as a parent, because the
        // `parent_id != -1` guard short-circuits first.
        p.c.reset();
        p.rust.reset();
        both_add(p, -1, 5, -1, b"weird");
        assert_eq!(both_find(p, -1), Some(0), "row with id -1 exists now");
        assert_eq!(both_add(p, 2, 6, -1, b"kid"), 1, "still treated as 'no parent'");
        assert_eq!(p.c.node(0).left_child_id, -1, "no link to the id==-1 row");
    });
}

#[test]
fn err11_parent_full_link_dropped() {
    with_libs(|p| {
        both_add(p, 1, 10, -1, b"root");
        both_add(p, 2, 20, 1, b"l");
        both_add(p, 3, 30, 1, b"r");
        let parent_before = p.c.node(0);
        for (i, id) in [4, 5, 6, 7].iter().enumerate() {
            let r = both_add(p, *id, 1000, 1, b"extra");
            assert_eq!(r, 3 + i as c_int, "insert SUCCEEDS (no error return)");
            assert_eq!(p.c.node(0), parent_before, "parent row untouched");
            assert_eq!(p.rust.node(0), parent_before);
            assert_eq!(p.c.node(3 + i).parent_id, 1, "child still records parent_id");
        }
        // the dropped children never contribute to the sum
        assert_eq!(both_sum(p, 1), 60, "10+20+30, none of the 4 dropped children");
        for id in [4, 5, 6, 7] {
            assert_eq!(both_sum(p, id), 1000, "dropped child is still reachable by id");
        }
    });
}

// ===========================================================================
// Row 12-14: label handling
// ===========================================================================

/// Poison the destination row so every byte the C writes is observable
/// (`strncpy(dst, src, 31)` never touches byte 31 - only the explicit
/// `node->label[31] = '\0'` does, and a zeroed table would hide a missing write).
fn poison_row0(p: &Pair, fill: u8) {
    let poison = NodeView {
        id: 0x5a5a5a5a,
        value: 0x5a5a5a5a,
        parent_id: 0x5a5a5a5a,
        left_child_id: 0x5a5a5a5a,
        right_child_id: 0x5a5a5a5a,
        label: [fill; 32],
    };
    p.c.set_node(0, &poison);
    p.rust.set_node(0, &poison);
    p.c.set_count(0);
    p.rust.set_count(0);
    assert_state_eq(p, "poisoned row 0");
}

#[test]
fn err12_label_truncation() {
    with_libs(|p| {
        for len in [32usize, 33, 40, 63, 64, 100, 255] {
            p.c.reset();
            p.rust.reset();
            poison_row0(p, 0xFF);
            let label: Vec<u8> = (0..len).map(|i| b'A' + (i % 26) as u8).collect();
            both_add(p, 1, 1, -1, &label);
            let n = p.c.node(0);
            assert_eq!(&n.label[..31], &label[..31], "first 31 bytes copied");
            assert_eq!(n.label[31], 0, "byte 31 forced to NUL even over 0xFF");
            assert_eq!(p.rust.node(0), n, "truncation must match");
        }
    });
}

/// `node->label[31] = '\0'` is a write that `strncpy(dst, src, 31)` can never
/// perform. It is only observable when byte 31 was non-zero beforehand, which is
/// reachable through the exported, writable `node_table`.
#[test]
fn err12b_label_byte31_always_cleared() {
    with_libs(|p| {
        for fill in [0xFFu8, 0x01, 0x80, b'A'] {
            for len in [0usize, 1, 5, 30, 31, 32, 40] {
                p.c.reset();
                p.rust.reset();
                poison_row0(p, fill);
                assert_eq!(p.c.node(0).label[31], fill, "poison took effect");
                let label: Vec<u8> = vec![b'q'; len];
                both_add(p, 1, 1, -1, &label);
                assert_eq!(
                    p.c.node(0).label[31], 0,
                    "C must clear label[31] (fill=0x{fill:02x}, len={len})"
                );
                assert_eq!(
                    p.rust.node(0).label[31], 0,
                    "Rust must clear label[31] too (fill=0x{fill:02x}, len={len})"
                );
                assert_eq!(p.rust.node(0), p.c.node(0));
            }
        }
    });
}

#[test]
fn err13_label_exactly_31() {
    with_libs(|p| {
        let label: Vec<u8> = (0..31).map(|i| b'a' + (i % 26) as u8).collect();
        both_add(p, 1, 1, -1, &label);
        let n = p.c.node(0);
        assert_eq!(&n.label[..31], &label[..]);
        assert_eq!(n.label[31], 0);
        assert_eq!(p.rust.node(0), n);
    });
}

#[test]
fn err14_label_short_zero_padded() {
    with_libs(|p| {
        for len in [0usize, 1, 2, 15, 30] {
            p.c.reset();
            p.rust.reset();
            let label: Vec<u8> = vec![b'z'; len];
            both_add(p, 1, 1, -1, &label);
            let n = p.c.node(0);
            assert_eq!(&n.label[..len], &label[..]);
            for i in len..32 {
                assert_eq!(n.label[i], 0, "byte {i} must be zero-padded (len={len})");
            }
            assert_eq!(p.rust.node(0), n);
        }
    });
}

// ===========================================================================
// Row 15-18: calculate_tree_sum rejections -> 0
// ===========================================================================

#[test]
fn err15_sum_absent_id() {
    with_libs(|p| {
        inject(p, &[nv(1, 100, -1, -1), nv(2, 200, -1, -1)], 2);
        for id in [0, 3, -1, -2, 999, i32::MIN, i32::MAX] {
            assert_eq!(both_sum(p, id), 0, "absent id {id} -> 0");
        }
        assert_eq!(both_sum(p, 1), 100);
        assert_eq!(both_sum(p, 2), 200);
    });
}

#[test]
fn err16_sum_id_mismatch_unreachable() {
    with_libs(|p| {
        // Same argument as row 9: `node->id != node_id` cannot be true.
        let mut rng = Rng::new(SEED ^ 16);
        for _ in 0..300 {
            let n = 1 + rng.below(MAX_NODES as u64) as usize;
            let nodes: Vec<NodeView> = (0..n).map(|_| nv(rng.small(), rng.i32(), -1, -1)).collect();
            inject(p, &nodes, n as c_int);
            for _ in 0..20 {
                let q = rng.small();
                let s = both_sum(p, q);
                // if the id is absent the sum is exactly 0; otherwise it is the
                // FIRST matching row's value (leaf, both children == -1)
                match p.c.find_index(q) {
                    None => assert_eq!(s, 0, "absent {q}"),
                    Some(idx) => assert_eq!(s, p.c.node(idx as usize).value, "present {q}"),
                }
            }
        }
    });
}

#[test]
fn err17_dangling_child_id() {
    with_libs(|p| {
        // children name ids that do not exist -> those subtrees contribute 0
        inject(p, &[nv(1, 7, 555, 666)], 1);
        assert_eq!(both_sum(p, 1), 7, "dangling children contribute 0");
        inject(p, &[nv(1, 7, 2, 666), nv(2, 3, -1, -1)], 2);
        assert_eq!(both_sum(p, 1), 10, "only the live child counts");
        // a child id pointing PAST node_count is dangling too
        inject(p, &[nv(1, 7, 2, -1), nv(2, 3, -1, -1)], 1);
        assert_eq!(both_sum(p, 1), 7, "child hidden by node_count -> 0");
        // dangling child ids that happen to be 0 (the zeroed-row id)
        inject(p, &[nv(1, 7, 0, 0)], 1);
        assert_eq!(both_sum(p, 1), 7);
    });
}

#[test]
fn err18_leaf_sentinels() {
    with_libs(|p| {
        let mut rng = Rng::new(SEED ^ 18);
        for _ in 0..500 {
            let v = rng.spicy_i32();
            inject(p, &[nv(1, v, -1, -1)], 1);
            assert_eq!(both_sum(p, 1), v, "both sentinels -> no recursion");
        }
        // only one side sentinel
        inject(p, &[nv(1, 5, -1, 2), nv(2, 6, -1, -1)], 2);
        assert_eq!(both_sum(p, 1), 11);
        inject(p, &[nv(1, 5, 2, -1), nv(2, 6, -1, -1)], 2);
        assert_eq!(both_sum(p, 1), 11);
    });
}

// ===========================================================================
// Row 19-21: parse_operation fallbacks
// ===========================================================================

#[test]
fn err19_parse_null() {
    with_libs(|p| {
        // The NULL check short-circuits BEFORE strchr, so this must not crash.
        let cv = (p.c.parse_operation)(std::ptr::null::<c_char>());
        let rv = (p.rust.parse_operation)(std::ptr::null::<c_char>());
        assert_eq!(cv, OP_ADD, "C parse_operation(NULL) must be OP_ADD");
        assert_eq!(rv, OP_ADD, "Rust parse_operation(NULL) must be OP_ADD");
        for _ in 0..1000 {
            assert_eq!(p.c.parse_op_null(), p.rust.parse_op_null());
        }
    });
}

#[test]
fn err20_parse_no_operator() {
    with_libs(|p| {
        let cases: &[&[u8]] = &[
            b"", b"a", b"root", b"left", b"right", b"left_left", b"t", b"f",
            b"0123456789", b"\x01", b"\x7f", b"\x80", b"\xff",
            b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ",
        ];
        for s in cases {
            assert_eq!(both_parse(p, s), OP_ADD, "no operator in {:?} -> OP_ADD", String::from_utf8_lossy(s));
        }
        // random operator-free strings
        let mut rng = Rng::new(SEED ^ 20);
        for _ in 0..2000 {
            let len = rng.below(20) as usize;
            let s: Vec<u8> = (0..len)
                .map(|_| loop {
                    let b = (rng.below(255) + 1) as u8;
                    if !b"+*-/%".contains(&b) {
                        return b;
                    }
                })
                .collect();
            assert_eq!(both_parse(p, &s), OP_ADD, "{:?}", s);
        }
    });
}

#[test]
fn err21_parse_precedence() {
    with_libs(|p| {
        // fixed check order + > * > - > / > %, regardless of position
        let cases: &[(&[u8], c_int)] = &[
            (b"%/-*+", OP_ADD),
            (b"%/-*", OP_MULTIPLY),
            (b"%/-", OP_SUBTRACT),
            (b"%/", OP_DIVIDE),
            (b"%", OP_MODULO),
            (b"*+", OP_ADD),
            (b"+*", OP_ADD),
            (b"-*", OP_MULTIPLY),
            (b"/%", OP_DIVIDE),
            (b"%-", OP_SUBTRACT),
            (b"%/*", OP_MULTIPLY),
            (b"----------+", OP_ADD),
            (b"+++", OP_ADD),
        ];
        for (s, want) in cases {
            assert_eq!(both_parse(p, s), *want, "{:?}", String::from_utf8_lossy(s));
        }
        // exhaustive over all 32 subsets of the five operators, reversed order
        for mask in 0u32..32 {
            let mut s = Vec::new();
            for (i, &ch) in b"%/-*+".iter().enumerate() {
                if mask & (1 << i) != 0 {
                    s.push(ch);
                }
            }
            let want = if s.contains(&b'+') { OP_ADD }
                else if s.contains(&b'*') { OP_MULTIPLY }
                else if s.contains(&b'-') { OP_SUBTRACT }
                else if s.contains(&b'/') { OP_DIVIDE }
                else if s.contains(&b'%') { OP_MODULO }
                else { OP_ADD };
            assert_eq!(both_parse(p, &s), want, "mask {mask} = {:?}", String::from_utf8_lossy(&s));
        }
    });
}

// ===========================================================================
// Row 22-23: get_operation_func with out-of-range enum values
// ===========================================================================

#[test]
fn err22_get_func_out_of_range_enum() {
    with_libs(|p| {
        let mut bad: Vec<i32> = (-8..=16).filter(|v| !(1..=5).contains(v)).collect();
        bad.extend_from_slice(&[i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1, 6, 100, -100, 0]);
        let mut rng = Rng::new(SEED ^ 22);
        for _ in 0..500 {
            let v = rng.spicy_i32();
            if !(1..=5).contains(&v) {
                bad.push(v);
            }
        }
        for v in bad {
            let cp = (p.c.get_operation_func)(v) as usize;
            let rp = (p.rust.get_operation_func)(v) as usize;
            assert_ne!(cp, 0, "C get_operation_func({v}) must not be NULL");
            assert_ne!(rp, 0, "Rust get_operation_func({v}) must not be NULL");
            assert_eq!(cp, p.c.op_addr(OP_ADD), "C: default: must return add_op for {v}");
            assert_eq!(rp, p.rust.op_addr(OP_ADD), "Rust: default: must return add_op for {v}");
            let cf: OpFn = unsafe { std::mem::transmute(cp) };
            let rf: OpFn = unsafe { std::mem::transmute(rp) };
            for _ in 0..20 {
                let (a, b) = (rng.spicy_i32(), rng.spicy_i32());
                assert_eq!(cf(a, b, 0, 0), rf(a, b, 0, 0), "default func({a},{b}) for op {v}");
                assert_eq!(cf(a, b, 0, 0), a.wrapping_add(b), "default func must be add_op");
            }
        }
    });
}

#[test]
fn err23_get_func_enum_is_addr_of_add_op() {
    with_libs(|p| {
        // An out-of-range C enum value crossing the FFI boundary must select the
        // very same symbol the in-range OP_ADD selects, in both libraries.
        for v in [0i32, 6, 7, -1, -5, 1000, i32::MIN, i32::MAX] {
            assert_eq!(
                (p.c.get_operation_func)(v) as usize,
                (p.c.get_operation_func)(OP_ADD) as usize,
                "C: op {v} must alias OP_ADD"
            );
            assert_eq!(
                (p.rust.get_operation_func)(v) as usize,
                (p.rust.get_operation_func)(OP_ADD) as usize,
                "Rust: op {v} must alias OP_ADD"
            );
        }
        // and the 5 valid variants must select 5 DISTINCT symbols in both
        let cs: Vec<usize> = (1..=5).map(|v| (p.c.get_operation_func)(v) as usize).collect();
        let rs: Vec<usize> = (1..=5).map(|v| (p.rust.get_operation_func)(v) as usize).collect();
        for i in 0..5 {
            for j in (i + 1)..5 {
                assert_ne!(cs[i], cs[j], "C ops {} and {} must differ", i + 1, j + 1);
                assert_ne!(rs[i], rs[j], "Rust ops {} and {} must differ", i + 1, j + 1);
            }
        }
    });
}

// ===========================================================================
// Row 24-27: inreftree fallbacks and overflow
// ===========================================================================

#[test]
fn err24_inreftree_target_value_zero() {
    with_libs(|p| {
        let mut rng = Rng::new(SEED ^ 24);
        for _ in 0..3000 {
            let (a, c, d) = (rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
            // param2 == 0 -> target->value == 0 -> target_id falls back 2 -> 1
            let with_zero = both_inreftree(p, a, 0, c, d);
            let sum0 = a.wrapping_add(0).wrapping_add(c).wrapping_add(d);
            let expect_op = match sum0.wrapping_rem(4) {
                0 => OP_ADD, 1 => OP_MULTIPLY, 2 => OP_SUBTRACT, 3 => OP_MODULO, _ => OP_ADD,
            };
            let f: OpFn = unsafe { std::mem::transmute((p.c.get_operation_func)(expect_op)) };
            assert_eq!(with_zero, f(sum0, 1, 0, 0), "param2==0 must use target_id 1");
            // ...and with a non-zero param2 that keeps the SAME sum, target_id is 2
            let b2 = if c != 0 { c } else { 1 };
            let a2 = a.wrapping_add(c).wrapping_sub(b2);
            let other = both_inreftree(p, a2, b2, c.wrapping_sub(c), d);
            let sum2 = a2.wrapping_add(b2).wrapping_add(d);
            let expect_op2 = match sum2.wrapping_rem(4) {
                0 => OP_ADD, 1 => OP_MULTIPLY, 2 => OP_SUBTRACT, 3 => OP_MODULO, _ => OP_ADD,
            };
            let f2: OpFn = unsafe { std::mem::transmute((p.c.get_operation_func)(expect_op2)) };
            assert_eq!(other, f2(sum2, 2, 0, 0), "param2!=0 must use target_id 2");
        }
    });
}

#[test]
fn err25_inreftree_target_null_unreachable() {
    with_libs(|p| {
        // The label scan always finds "left" (id 2), and id 2 is always present,
        // so `target == NULL` (1st operand of line 180) is unreachable.
        let mut rng = Rng::new(SEED ^ 25);
        for _ in 0..2000 {
            let (a, b, c, d) = (rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
            both_inreftree(p, a, b, c, d);
            // after the call the rebuilt table still holds id 2 at index 1
            for lib in [&p.c, &p.rust] {
                assert_eq!(lib.find_index(2), Some(1), "{}: id 2 always present", lib.name);
                assert_eq!(lib.node(1).value, b, "{}: target->value == param2", lib.name);
                let end = lib.node(1).label.iter().position(|&x| x == 0).unwrap_or(32);
                assert!(lib.node(1).label[..end].contains(&b'l'), "label 'left' has an 'l'");
                // node 0's label "root" has no 'l', so the scan cannot stop earlier
                let e0 = lib.node(0).label.iter().position(|&x| x == 0).unwrap_or(32);
                assert!(!lib.node(0).label[..e0].contains(&b'l'), "\"root\" has no 'l'");
            }
        }
    });
}

#[test]
fn err26_inreftree_negative_remainder() {
    with_libs(|p| {
        // tree_sum % 4 in {-1,-2,-3} reads op_string[-1..-3], i.e. BEFORE the
        // "+*-%" literal (UB). For this build .rodata is
        // "root\0left\0right\0left-left\0+*-%\0", so the bytes are '\0', 't', 'f'
        // - none an operator - so parse_operation falls back to OP_ADD.
        let mut rng = Rng::new(SEED ^ 26);
        let mut seen = [0usize; 3];
        for _ in 0..6000 {
            let sum: i32 = -(1 + rng.below(1 << 29) as i32);
            let rem = sum.wrapping_rem(4);
            if rem >= 0 {
                continue;
            }
            let b = { let v = rng.small(); if v == 0 { 1 } else { v } };
            let c = rng.small();
            let d = rng.small();
            let a = sum.wrapping_sub(b).wrapping_sub(c).wrapping_sub(d);
            let got = both_inreftree(p, a, b, c, d);
            // strongest possible assertion: the OP_ADD path was taken
            assert_eq!(
                got,
                (p.c.add_op)(sum, 2, 0, 0),
                "negative remainder {rem} must select OP_ADD (sum={sum})"
            );
            assert_eq!(got, (p.rust.add_op)(sum, 2, 0, 0));
            seen[(-rem - 1) as usize] += 1;
        }
        assert!(seen.iter().all(|&n| n > 100), "remainder coverage {seen:?}");
        println!("err26: negative remainders covered {seen:?}");
    });
}

#[test]
fn err27_inreftree_int_min_sum() {
    with_libs(|p| {
        // tree_sum == INT_MIN: INT_MIN % 4 == 0 -> OP_ADD -> add_op overflows
        let cases = [
            (i32::MIN, 1, -1, 0),
            (i32::MIN, 2, -1, -1),
            (i32::MIN, 0, 0, 0),
            (0, i32::MIN, 0, 0),
            (0, 0, i32::MIN, 0),
            (0, 0, 0, i32::MIN),
            (i32::MIN / 2, i32::MIN / 2, 0, 0),
            (i32::MAX, 1, 0, 0),
            (i32::MAX, i32::MAX, 1, 1),
            (i32::MIN, i32::MIN, 0, 0),
        ];
        for (a, b, c, d) in cases {
            let sum = a.wrapping_add(b).wrapping_add(c).wrapping_add(d);
            let target = if b == 0 { 1 } else { 2 };
            let want_op = match sum.wrapping_rem(4) {
                0 => OP_ADD, 1 => OP_MULTIPLY, 2 => OP_SUBTRACT, 3 => OP_MODULO, _ => OP_ADD,
            };
            let f: OpFn = unsafe { std::mem::transmute((p.c.get_operation_func)(want_op)) };
            let got = both_inreftree(p, a, b, c, d);
            assert_eq!(got, f(sum, target, 0, 0), "inreftree({a},{b},{c},{d}) sum={sum}");
        }
    });
}

// ===========================================================================
// Row 28-29: signed overflow wraps identically
// ===========================================================================

#[test]
fn err28_arith_overflow_wraps() {
    with_libs(|p| {
        let cases: &[(i32, i32)] = &[
            (i32::MAX, 1), (1, i32::MAX), (i32::MIN, -1), (-1, i32::MIN),
            (i32::MAX, i32::MAX), (i32::MIN, i32::MIN), (i32::MAX, i32::MIN),
            (i32::MIN, i32::MAX), (0, i32::MIN), (i32::MIN / 2, 2), (65536, 65536),
        ];
        for &(a, b) in cases {
            assert_eq!((p.c.add_op)(a, b, 0, 0), (p.rust.add_op)(a, b, 0, 0), "add {a}+{b}");
            assert_eq!((p.c.add_op)(a, b, 0, 0), a.wrapping_add(b), "add wraps");
            assert_eq!((p.c.subtract_op)(a, b, 0, 0), (p.rust.subtract_op)(a, b, 0, 0), "sub {a}-{b}");
            assert_eq!((p.c.subtract_op)(a, b, 0, 0), a.wrapping_sub(b), "sub wraps");
            assert_eq!((p.c.multiply_op)(a, b, 0, 0), (p.rust.multiply_op)(a, b, 0, 0), "mul {a}*{b}");
            assert_eq!((p.c.multiply_op)(a, b, 0, 0), a.wrapping_mul(b), "mul wraps");
        }
        // divide/modulo: the ONLY overflow case is INT_MIN / -1, see U1
        assert_eq!((p.c.divide_op)(i32::MIN, 1, 0, 0), (p.rust.divide_op)(i32::MIN, 1, 0, 0));
        assert_eq!((p.c.divide_op)(i32::MIN, 2, 0, 0), (p.rust.divide_op)(i32::MIN, 2, 0, 0));
        assert_eq!((p.c.modulo_op)(i32::MIN, 2, 0, 0), (p.rust.modulo_op)(i32::MIN, 2, 0, 0));
        assert_eq!((p.c.modulo_op)(i32::MIN, 1, 0, 0), (p.rust.modulo_op)(i32::MIN, 1, 0, 0));
    });
}

#[test]
fn err29_sum_overflow_wraps() {
    with_libs(|p| {
        // accumulator overflow inside the recursion
        inject(p, &[nv(1, i32::MAX, 2, 3), nv(2, i32::MAX, -1, -1), nv(3, 2, -1, -1)], 3);
        let want = i32::MAX.wrapping_add(i32::MAX).wrapping_add(2);
        assert_eq!(both_sum(p, 1), want, "INT_MAX+INT_MAX+2 must wrap");

        inject(p, &[nv(1, i32::MIN, 2, -1), nv(2, i32::MIN, -1, -1)], 2);
        assert_eq!(both_sum(p, 1), i32::MIN.wrapping_add(i32::MIN));

        // long chain of INT_MAX
        let n = MAX_NODES;
        let nodes: Vec<NodeView> = (0..n)
            .map(|i| nv(i as i32 + 1, i32::MAX, if i + 1 < n { i as i32 + 2 } else { -1 }, -1))
            .collect();
        inject(p, &nodes, n as c_int);
        let want = (0..n).fold(0i32, |a, _| a.wrapping_add(i32::MAX));
        assert_eq!(both_sum(p, 1), want, "50x INT_MAX must wrap identically");

        let mut rng = Rng::new(SEED ^ 29);
        for _ in 0..500 {
            let vals: Vec<i32> = (0..n).map(|_| rng.spicy_i32()).collect();
            let nodes: Vec<NodeView> = (0..n)
                .map(|i| nv(i as i32 + 1, vals[i], if i + 1 < n { i as i32 + 2 } else { -1 }, -1))
                .collect();
            inject(p, &nodes, n as c_int);
            let want = vals.iter().fold(0i32, |a, &b| a.wrapping_add(b));
            assert_eq!(both_sum(p, 1), want);
        }
    });
}

// ===========================================================================
// Row 30-31: no cycle detection / duplicate ids
// ===========================================================================

#[test]
fn err30_diamond_double_count() {
    with_libs(|p| {
        // 1 -> {2,3}, and BOTH 2 and 3 point at 4: node 4 is counted TWICE
        inject(
            p,
            &[nv(1, 1, 2, 3), nv(2, 10, 4, -1), nv(3, 100, 4, -1), nv(4, 1000, -1, -1)],
            4,
        );
        assert_eq!(both_sum(p, 1), 1 + 10 + 1000 + 100 + 1000, "4 counted once per path");

        // a node pointing at the SAME child on both sides: counted twice
        inject(p, &[nv(1, 1, 2, 2), nv(2, 50, -1, -1)], 2);
        assert_eq!(both_sum(p, 1), 101, "same child on both sides -> 1+50+50");

        // 3 levels of doubling: 2^3 leaf visits
        inject(
            p,
            &[nv(1, 0, 2, 2), nv(2, 0, 3, 3), nv(3, 0, 4, 4), nv(4, 1, -1, -1)],
            4,
        );
        assert_eq!(both_sum(p, 1), 8, "leaf visited 2^3 times");
    });
}

#[test]
fn err31_duplicate_ids_first_wins() {
    with_libs(|p| {
        inject(
            p,
            &[nv(9, 1, -1, -1), nv(9, 2, -1, -1), nv(9, 4, -1, -1), nv(3, 8, 9, -1)],
            4,
        );
        assert_eq!(both_find(p, 9), Some(0), "first duplicate wins");
        assert_eq!(both_sum(p, 9), 1, "sum uses the FIRST row's value");
        assert_eq!(both_sum(p, 3), 9, "8 + first-9's value 1");

        // duplicates created through add_tree_node
        p.c.reset();
        p.rust.reset();
        both_add(p, 5, 10, -1, b"a");
        both_add(p, 5, 20, -1, b"b");
        both_add(p, 5, 40, -1, b"c");
        assert_eq!(both_find(p, 5), Some(0));
        assert_eq!(both_sum(p, 5), 10);
        // a later duplicate can never be used as a parent target either
        both_add(p, 6, 1, 5, b"kid");
        assert_eq!(p.c.node(0).left_child_id, 6, "first row got the link");
        assert_eq!(p.c.node(1).left_child_id, -1);
        assert_eq!(p.c.node(2).left_child_id, -1);
        assert_eq!(both_sum(p, 5), 11);
    });
}

// ===========================================================================
// Generic FFI boundary checks (beyond the table)
// ===========================================================================

#[test]
fn err32_generic_boundaries() {
    with_libs(|p| {
        // NULL pointer: the only pointer parameter that the C guards
        assert_eq!((p.c.parse_operation)(std::ptr::null()), (p.rust.parse_operation)(std::ptr::null()));
        // zero-length (empty) string
        assert_eq!(both_parse(p, b""), OP_ADD);
        // zero-length label
        assert_eq!(both_add(p, 1, 1, -1, b""), 0);
        // one step past the valid enum range on both sides
        for v in [0i32, 6] {
            assert_eq!(
                (p.c.get_operation_func)(v) as usize == p.c.op_addr(OP_ADD),
                (p.rust.get_operation_func)(v) as usize == p.rust.op_addr(OP_ADD),
            );
        }
        // one step past the valid node_count range
        p.c.reset();
        p.rust.reset();
        p.c.set_count(MAX_NODES as c_int - 1);
        p.rust.set_count(MAX_NODES as c_int - 1);
        assert_eq!(both_add(p, 1, 1, -1, b"last"), MAX_NODES as c_int - 1, "index 49 is valid");
        assert_eq!(both_add(p, 2, 2, -1, b"over"), -1, "index 50 is rejected");

        // id sentinels at the boundaries.
        // NB: the table must be populated with real rows first. A *zeroed* row is
        // a self-cycle (`id == 0` and `left_child_id == 0`), so
        // calculate_tree_sum(0) would recurse forever - see ERRORS.md U5 / ub02.
        inject(p, &[nv(1, 5, -1, -1), nv(0, 6, -1, -1), nv(-1, 7, -1, -1)], 3);
        for id in [i32::MIN, i32::MIN + 1, -1, 0, 1, 2, i32::MAX - 1, i32::MAX] {
            both_find(p, id);
            both_sum(p, id);
        }
        assert_eq!(both_sum(p, 0), 6, "id 0 is a perfectly valid id");
        assert_eq!(both_sum(p, -1), 7, "so is id -1");
        assert_eq!(both_find(p, -1), Some(2));
    });
}

// ===========================================================================
// ERRORS.md U1: documented hard UB - the C build traps (SIGFPE) on INT_MIN/-1.
// Run in a CHILD PROCESS so the trap cannot take down the test harness.
// ===========================================================================

const TRAP_ENV: &str = "PHASE_C_TRAP_TARGET";

#[test]
fn ub01_int_min_div_traps_in_c() {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().expect("current_exe");
    for target in ["c_div", "c_mod"] {
        let out = std::process::Command::new(&exe)
            .args(["--exact", "ub_helper_trap", "--ignored", "--nocapture", "--test-threads=1"])
            .env(TRAP_ENV, target)
            .output()
            .expect("spawn child");
        let sig = out.status.signal();
        println!(
            "{target}: status={:?} signal={:?} stdout={}",
            out.status.code(),
            sig,
            String::from_utf8_lossy(&out.stdout).trim()
        );
        assert_eq!(
            sig,
            Some(8),
            "the C build must die with SIGFPE on INT_MIN/-1 ({target}); \
             ERRORS.md U1 documents this as untestable UB"
        );
    }
    // The Rust translation cannot reproduce a hardware trap; record what it does
    // instead so the divergence is documented rather than silent.
    for target in ["rust_div", "rust_mod"] {
        let out = std::process::Command::new(&exe)
            .args(["--exact", "ub_helper_trap", "--ignored", "--nocapture", "--test-threads=1"])
            .env(TRAP_ENV, target)
            .output()
            .expect("spawn child");
        println!(
            "{target}: status={:?} signal={:?} stdout={}",
            out.status.code(),
            out.status.signal(),
            String::from_utf8_lossy(&out.stdout).trim()
        );
    }
}

/// ERRORS.md U5: `calculate_tree_sum` has no cycle detection. The cheapest way
/// to build a cycle is to leave `node_table` zeroed and raise `node_count`: a
/// zeroed row has `id == 0` AND `left_child_id == 0`, i.e. it is its own left
/// child. Both libraries must therefore recurse until the stack is exhausted.
#[test]
fn ub02_cycle_overflows_stack_in_both() {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().expect("current_exe");
    let mut outcomes = Vec::new();
    for target in ["c_cycle", "rust_cycle"] {
        let out = std::process::Command::new(&exe)
            .args(["--exact", "ub_helper_trap", "--ignored", "--nocapture", "--test-threads=1"])
            .env(TRAP_ENV, target)
            .output()
            .expect("spawn child");
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        println!(
            "{target}: code={:?} signal={:?} stderr={}",
            out.status.code(),
            out.status.signal(),
            stderr.trim()
        );
        assert!(
            !out.status.success(),
            "{target}: a self-cycle must exhaust the stack, not return"
        );
        // Compare the actual termination (signal + whether the runtime's
        // stack-overflow guard fired) rather than a hard-coded signal number:
        // the observed signal depends on whether a stack guard handler is
        // installed, and the point is that BOTH libraries behave the same.
        outcomes.push((
            out.status.signal(),
            out.status.code(),
            stderr.contains("stack overflow"),
        ));
    }
    assert_eq!(
        outcomes[0], outcomes[1],
        "C and Rust must fail IDENTICALLY on an unbounded recursion (signal, exit code, \
         and stack-overflow diagnostic must all match)"
    );
    assert!(
        outcomes[0].0.is_some() || outcomes[0].1.is_some_and(|c| c != 0),
        "expected abnormal termination from both, got {:?}",
        outcomes[0]
    );
}

#[test]
#[ignore = "helper for ub01/ub02: deliberately triggers UB in a child process"]
fn ub_helper_trap() {
    let target = std::env::var(TRAP_ENV).unwrap_or_default();
    with_libs(|p| {
        let r = match target.as_str() {
            "c_div" => (p.c.divide_op)(i32::MIN, -1, 0, 0),
            "c_mod" => (p.c.modulo_op)(i32::MIN, -1, 0, 0),
            "rust_div" => (p.rust.divide_op)(i32::MIN, -1, 0, 0),
            "rust_mod" => (p.rust.modulo_op)(i32::MIN, -1, 0, 0),
            "c_cycle" => {
                // zeroed table + node_count 1 => row 0 is its own left child
                p.c.set_count(1);
                (p.c.calculate_tree_sum)(0)
            }
            "rust_cycle" => {
                p.rust.set_count(1);
                (p.rust.calculate_tree_sum)(0)
            }
            other => panic!("unknown trap target {other:?}"),
        };
        println!("returned {r}");
    });
}
