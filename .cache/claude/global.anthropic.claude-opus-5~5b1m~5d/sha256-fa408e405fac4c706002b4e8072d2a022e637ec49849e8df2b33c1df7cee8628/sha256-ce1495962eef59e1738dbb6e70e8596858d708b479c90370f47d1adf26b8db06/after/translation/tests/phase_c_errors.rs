//! Phase C — error-path differential tests.
//!
//! One test per row of ERRORS.md. Each test constructs the exact invalid
//! input/condition, calls BOTH `.so`s and asserts the SAME sentinel comes back
//! (identical value, not merely "both failed"), plus identical global state.

mod common;

use common::{harness, Rng, MAX_NODES, NODE_BYTES};

const SEED: u64 = 0xE770_0000_1234_5678;

// ---------------------------------------------------------------------------
// row 1, 2 — division / modulo by zero
// ---------------------------------------------------------------------------

#[test]
fn err01_divide_by_zero_returns_zero() {
    let (_g, p) = harness();
    for a in [0i32, 1, -1, 7, -7, i32::MIN, i32::MAX] {
        let cv = unsafe { (p.c.divide_op)(a, 0, 0, 0) };
        let rv = unsafe { (p.r.divide_op)(a, 0, 0, 0) };
        assert_eq!(cv, rv, "divide_op({a},0)");
        assert_eq!(cv, 0, "C sentinel for divide_op({a},0)");
    }
}

#[test]
fn err02_modulo_by_zero_returns_zero() {
    let (_g, p) = harness();
    for a in [0i32, 1, -1, 7, -7, i32::MIN, i32::MAX] {
        let cv = unsafe { (p.c.modulo_op)(a, 0, 0, 0) };
        let rv = unsafe { (p.r.modulo_op)(a, 0, 0, 0) };
        assert_eq!(cv, rv, "modulo_op({a},0)");
        assert_eq!(cv, 0, "C sentinel for modulo_op({a},0)");
    }
}

// ---------------------------------------------------------------------------
// rows 3, 4, 5 — find_node_by_id returns NULL
// ---------------------------------------------------------------------------

#[test]
fn err03_find_absent_id_returns_null() {
    let (_g, p) = harness();
    for i in 0..5i32 {
        p.diff("populate", |l| l.add(i, i, -1, b"n"));
    }
    for id in [5i32, 6, -1, -2, i32::MIN, i32::MAX] {
        p.diff(&format!("find({id})"), |l| l.find(id));
        assert_eq!(p.c.find(id), None, "C sentinel: NULL for absent id {id}");
    }
}

#[test]
fn err04_find_on_empty_table_returns_null() {
    let (_g, p) = harness();
    for id in [0i32, 1, -1, i32::MIN, i32::MAX] {
        p.diff(&format!("find({id}) empty"), |l| l.find(id));
        assert_eq!(p.c.find(id), None);
    }
}

#[test]
fn err05_find_with_negative_node_count_returns_null() {
    let (_g, p) = harness();
    for i in 0..3i32 {
        p.diff("populate", |l| l.add(i, i, -1, b"n"));
    }
    for bad in [-1i32, -5, i32::MIN] {
        for l in [&p.c, &p.r] {
            l.set_count(bad);
        }
        for id in [0i32, 1, 2, 99] {
            p.diff(&format!("find({id}) count={bad}"), |l| l.find(id));
            assert_eq!(p.c.find(id), None, "C sentinel: NULL when count={bad}");
        }
        // calculate_tree_sum inherits the same guard
        p.diff(&format!("sum(0) count={bad}"), |l| l.sum(0));
    }
    for l in [&p.c, &p.r] {
        l.set_count(3);
    }
}

// ---------------------------------------------------------------------------
// row 6 — table full
// ---------------------------------------------------------------------------

#[test]
fn err06_add_when_table_full_returns_minus_one_and_changes_nothing() {
    let (_g, p) = harness();
    for i in 0..MAX_NODES as i32 {
        p.diff("fill", |l| l.add(i, i, -1, b"f"));
    }
    assert_eq!(p.c.count(), MAX_NODES as i32);
    let before_c = p.c.state();
    let before_r = p.r.state();
    for k in 0..5 {
        let ctx = format!("overflow add {k}");
        p.diff(&ctx, |l| l.add(1000 + k, k, -1, b"overflow"));
        assert_eq!(p.c.add(1000 + k, k, -1, b"overflow"), -1, "C sentinel -1");
    }
    assert_eq!(p.c.state(), before_c, "C: table untouched when full");
    assert_eq!(p.r.state(), before_r, "Rust: table untouched when full");
    p.assert_state("err06");
}

#[test]
fn err06b_add_at_exactly_the_limit_boundary() {
    let (_g, p) = harness();
    for i in 0..(MAX_NODES as i32 - 1) {
        p.diff("fill", |l| l.add(i, i, -1, b"f"));
    }
    assert_eq!(p.c.count(), 49);
    p.diff("50th (last allowed)", |l| l.add(49, 49, -1, b"last"));
    assert_eq!(p.c.count(), 50);
    p.diff("51st (rejected)", |l| l.add(50, 50, -1, b"nope"));
    assert_eq!(p.c.count(), 50);
}

// ---------------------------------------------------------------------------
// rows 7, 8 — parent lookup failure (slot already written!)
// ---------------------------------------------------------------------------

#[test]
fn err07_add_with_missing_parent_returns_minus_one_but_dirties_slot() {
    let (_g, p) = harness();
    p.diff("root", |l| l.add(1, 11, -1, b"root"));
    // parent 42 does not exist
    p.diff("bad parent", |l| l.add(2, 22, 42, b"orphan"));
    assert_eq!(p.c.add(2, 22, 42, b"orphan"), -1, "C sentinel -1");
    assert_eq!(p.c.count(), 1, "C: node_count NOT incremented");
    // slot 1 was written before the parent check failed
    let slot_c = p.c.node_bytes(1);
    let slot_r = p.r.node_bytes(1);
    assert_eq!(slot_c, slot_r, "dirty slot bytes differ");
    assert_ne!(
        slot_c,
        vec![0u8; NODE_BYTES],
        "C reference: the slot really was written before the failure"
    );
    p.assert_state("err07");

    // a subsequent successful add must overwrite that same slot identically
    p.diff("good add after failure", |l| l.add(3, 33, 1, b"child"));
    p.assert_state("err07 after recovery");
}

#[test]
fn err08_add_parent_id_mismatch_branch_is_dead_code() {
    let (_g, p) = harness();
    // `find_node_by_id` can only ever return a node whose id == the argument,
    // so `parent->id != parent_id` is unreachable. Prove it: for a large random
    // population, "parent found" is exactly equivalent to "id present", and the
    // two libraries agree on every outcome.
    let mut rng = Rng::new(SEED ^ 8);
    for trial in 0..300 {
        p.reset();
        let mut present: Vec<i32> = Vec::new();
        for k in 0..12 {
            let id = rng.range_i32(-3, 20);
            let parent = rng.range_i32(-3, 20);
            let ctx = format!("t{trial} s{k} add({id},{parent})");
            let expect_ok = parent == -1 || present.contains(&parent);
            let cv = p.c.add(id, k, parent, b"x");
            let rv = p.r.add(id, k, parent, b"x");
            assert_eq!(cv, rv, "{ctx}");
            assert_eq!(
                cv != -1,
                expect_ok,
                "{ctx}: 'parent found' must equal 'id present' (dead-code branch never fires)"
            );
            if cv != -1 {
                present.push(id);
            }
            p.assert_state(&ctx);
        }
    }
}

// ---------------------------------------------------------------------------
// rows 9, 10, 11 — label handling
// ---------------------------------------------------------------------------

#[test]
fn err09_label_longer_than_31_is_truncated() {
    let (_g, p) = harness();
    for len in [32usize, 33, 40, 64, 200] {
        p.reset();
        let label: Vec<u8> = vec![b'Z'; len];
        p.diff(&format!("len {len}"), |l| l.add(1, 1, -1, &label));
        let slot = p.c.node_bytes(0);
        assert_eq!(&slot[20..51], &vec![b'Z'; 31][..], "C: 31 bytes copied");
        assert_eq!(slot[51], 0, "C: label[31] forced to NUL");
        assert_eq!(slot, p.r.node_bytes(0));
    }
}

#[test]
fn err10_label_exactly_31_bytes() {
    let (_g, p) = harness();
    let label: Vec<u8> = (0..31).map(|i| b'a' + i as u8).collect();
    p.diff("len 31", |l| l.add(1, 1, -1, &label));
    let slot = p.c.node_bytes(0);
    assert_eq!(&slot[20..51], &label[..]);
    assert_eq!(slot[51], 0);
    assert_eq!(slot, p.r.node_bytes(0));
}

#[test]
fn err11_empty_label_nul_fills_field() {
    let (_g, p) = harness();
    // pre-dirty the slot so padding is observable
    for l in [&p.c, &p.r] {
        l.poke_node(0, 9, 9, 9, 9, 9, b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
    }
    p.assert_state("pre-dirty");
    p.diff("empty label", |l| l.add(1, 1, -1, b""));
    let slot = p.c.node_bytes(0);
    assert_eq!(&slot[20..52], &[0u8; 32][..], "C: whole label field NUL");
    assert_eq!(slot, p.r.node_bytes(0));
}

// ---------------------------------------------------------------------------
// rows 12..15 — parent-link edge cases
// ---------------------------------------------------------------------------

#[test]
fn err12_parent_sentinel_minus_one_skips_lookup() {
    let (_g, p) = harness();
    for k in 0..5i32 {
        p.diff(&format!("root {k}"), |l| l.add(k, k, -1, b"r"));
        assert_eq!(p.c.count(), k + 1, "C: -1 parent always succeeds");
    }
}

#[test]
fn err13_both_child_slots_taken_drops_link() {
    let (_g, p) = harness();
    p.diff("root", |l| l.add(1, 1, -1, b"root"));
    p.diff("c1", |l| l.add(2, 2, 1, b"c1"));
    p.diff("c2", |l| l.add(3, 3, 1, b"c2"));
    let root_before = p.c.node_bytes(0);
    p.diff("c3", |l| l.add(4, 4, 1, b"c3"));
    assert_eq!(p.c.count(), 4, "C: the third child is still inserted");
    assert_eq!(
        p.c.node_bytes(0),
        root_before,
        "C: parent links unchanged by the dropped child"
    );
    p.diff("c4", |l| l.add(5, 5, 1, b"c4"));
    assert_eq!(p.c.count(), 5, "C: still returns/inserts");
    assert_eq!(p.c.node_bytes(0), root_before);
    p.diff("c5", |l| l.add(6, 6, 1, b"c5"));
    p.assert_state("err13");
    // the dropped children are unreachable from the sum
    p.diff("sum(1)", |l| l.sum(1));
}

#[test]
fn err14_duplicate_ids_no_rejection() {
    let (_g, p) = harness();
    for k in 0..4i32 {
        p.diff(&format!("dup {k}"), |l| l.add(5, k, -1, b"dup"));
        assert_eq!(p.c.count(), k + 1, "C: duplicate ids accepted, no rejection");
    }
    p.diff("find(5)", |l| l.find(5));
    p.diff("sum(5)", |l| l.sum(5));
    // duplicate id used as a parent -> links onto the FIRST match
    p.diff("child of dup", |l| l.add(6, 100, 5, b"child"));
    p.assert_state("err14");
}

#[test]
fn err15_parent_id_equal_to_own_id_is_rejected() {
    let (_g, p) = harness();
    p.diff("root", |l| l.add(1, 1, -1, b"root"));
    // node 7 names itself as parent; it lives at index node_count, which the
    // parent scan (i < node_count) cannot see -> "not found" -> -1
    p.diff("self parent", |l| l.add(7, 7, 7, b"self"));
    assert_eq!(p.c.add(7, 7, 7, b"self"), -1, "C sentinel -1");
    assert_eq!(p.c.count(), 1);
    p.assert_state("err15");
}

// ---------------------------------------------------------------------------
// rows 16..19 — calculate_tree_sum sentinels
// ---------------------------------------------------------------------------

#[test]
fn err16_sum_of_absent_node_is_zero() {
    let (_g, p) = harness();
    p.diff("root", |l| l.add(1, 1234, -1, b"root"));
    for id in [2i32, 0, -5, i32::MIN, i32::MAX] {
        p.diff(&format!("sum({id})"), |l| l.sum(id));
        assert_eq!(p.c.sum(id), 0, "C sentinel 0 for absent id {id}");
    }
}

#[test]
fn err17_sum_of_child_sentinel_minus_one() {
    let (_g, p) = harness();
    p.diff("root", |l| l.add(1, 7, -1, b"root"));
    p.diff("sum(-1)", |l| l.sum(-1));
    assert_eq!(p.c.sum(-1), 0);
    // ... but a real node with id -1 IS summed
    p.diff("add id -1", |l| l.add(-1, 99, -1, b"neg"));
    p.diff("sum(-1) again", |l| l.sum(-1));
    assert_eq!(p.c.sum(-1), 99, "C: id -1 is a perfectly valid id");
}

#[test]
fn err18_sum_id_mismatch_branch_is_dead_code() {
    let (_g, p) = harness();
    // Same argument as err08: find_node_by_id guarantees the id matches, so the
    // second disjunct of the guard never fires. Verified over random tables by
    // checking that sum(id)==0 exactly when id is absent OR its subtree sums 0.
    let mut rng = Rng::new(SEED ^ 18);
    for trial in 0..200 {
        p.reset();
        let n = 1 + rng.below(10);
        let mut present: Vec<i32> = Vec::new();
        for k in 0..n {
            let id = k as i32 + 1;
            let parent = if present.is_empty() {
                -1
            } else {
                present[rng.below(present.len())]
            };
            p.diff("add", |l| l.add(id, 1, parent, b"n"));
            present.push(id);
        }
        for id in -2..(n as i32 + 3) {
            let ctx = format!("t{trial} sum({id})");
            p.diff(&ctx, |l| l.sum(id));
            if !present.contains(&id) {
                assert_eq!(p.c.sum(id), 0, "{ctx}: absent -> 0");
            } else {
                assert!(p.c.sum(id) >= 1, "{ctx}: present, all values 1 -> >=1");
            }
        }
    }
}

#[test]
fn err19_sum_on_empty_table_is_zero() {
    let (_g, p) = harness();
    for id in [0i32, 1, -1, i32::MIN, i32::MAX] {
        p.diff(&format!("sum({id}) empty"), |l| l.sum(id));
        assert_eq!(p.c.sum(id), 0);
    }
}

// ---------------------------------------------------------------------------
// rows 20, 21, 22 — parse_operation
// ---------------------------------------------------------------------------

#[test]
fn err20_parse_operation_null_pointer_is_accepted() {
    let (_g, p) = harness();
    let cv = unsafe { p.c.parse_raw(std::ptr::null()) };
    let rv = unsafe { p.r.parse_raw(std::ptr::null()) };
    assert_eq!(cv, rv, "parse_operation(NULL)");
    assert_eq!(cv, 1, "C: NULL short-circuits to OP_ADD");
}

#[test]
fn err21_parse_operation_no_operator_falls_through_to_add() {
    let (_g, p) = harness();
    for s in [
        &b"abc"[..],
        &b"0123456789"[..],
        &b"   "[..],
        &b"^&#@!~"[..],
        &b"\x01\x7f\x80\xff"[..],
    ] {
        let cv = p.c.parse(s);
        let rv = p.r.parse(s);
        assert_eq!(cv, rv, "parse_operation({:?})", s);
        assert_eq!(cv, 1, "C fallback OP_ADD for {:?}", s);
    }
}

#[test]
fn err22_parse_operation_empty_string() {
    let (_g, p) = harness();
    let cv = p.c.parse(b"");
    let rv = p.r.parse(b"");
    assert_eq!(cv, rv);
    assert_eq!(cv, 1, "C: \"\" -> OP_ADD");
}

// ---------------------------------------------------------------------------
// rows 23..26 — out-of-range Operation values across the FFI boundary
// ---------------------------------------------------------------------------

#[test]
fn err23_26_get_operation_func_out_of_range_enum() {
    let (_g, p) = harness();
    let mut rng = Rng::new(SEED ^ 26);
    let mut cases: Vec<i32> = vec![
        0,
        6,
        7,
        -1,
        -2,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
        1 << 16,
        -(1 << 16),
    ];
    for _ in 0..2000 {
        let v = rng.next_i32();
        if !(1..=5).contains(&v) {
            cases.push(v);
        }
    }
    for &op in &cases {
        let ci = p.c.op_index(op);
        let ri = p.r.op_index(op);
        assert_eq!(ci, ri, "get_operation_func({op}) resolved differently");
        assert_eq!(
            ci, 0,
            "C default: out-of-range Operation {op} must map to add_op"
        );
        // and the returned pointer must behave like add_op in both
        let cv = p.c.call_op(op, 3, 4, 0, 0);
        let rv = p.r.call_op(op, 3, 4, 0, 0);
        assert_eq!(cv, rv);
        assert_eq!(cv, 7, "add_op(3,4)");
    }
}

// ---------------------------------------------------------------------------
// rows 27..31 — inreftree fallbacks, OOB .rodata read, overflow
// ---------------------------------------------------------------------------

/// Independent model of `inreftree` derived from the C source, including the
/// out-of-bounds read of the three `.rodata` bytes that precede `"+*-%"`
/// (`'f'`, `'t'`, `'\0'` — the tail of the `"left-left"` literal) and the
/// implicit target-id fallback. Used to *prove* which branches the C takes.
fn model_inreftree(p1: i32, p2: i32, p3: i32, p4: i32) -> (i32, i32, i32) {
    // (result, chosen target_id, chosen Operation)
    let tree_sum = p1
        .wrapping_add(p2)
        .wrapping_add(p4)
        .wrapping_add(p3);
    // the 'l' scan always finds "left" at index 1 => id 2
    let mut target_id = 2;
    if p2 == 0 {
        target_id = 1;
    }
    let op_char = match tree_sum % 4 {
        0 => b'+',
        1 => b'*',
        2 => b'-',
        3 => b'%',
        -1 => 0u8,   // op_string[-1] : NUL terminating "left-left"
        -2 => b't',  // op_string[-2]
        -3 => b'f',  // op_string[-3]
        other => panic!("impossible residue {other}"),
    };
    let op = match op_char {
        b'+' => 1,
        b'*' => 2,
        b'-' => 3,
        b'/' => 4,
        b'%' => 5,
        _ => 1, // no operator char found -> OP_ADD
    };
    let result = match op {
        1 => tree_sum.wrapping_add(target_id),
        2 => tree_sum.wrapping_mul(target_id),
        3 => tree_sum.wrapping_sub(target_id),
        4 => {
            if target_id == 0 {
                0
            } else {
                tree_sum.wrapping_div(target_id)
            }
        }
        5 => {
            if target_id == 0 {
                0
            } else {
                tree_sum.wrapping_rem(target_id)
            }
        }
        _ => unreachable!(),
    };
    (result, target_id, op)
}

#[test]
fn err27_inreftree_null_target_branch_is_dead_code() {
    let (_g, p) = harness();
    // The label scan is driven by the hard-coded literals "root", "left",
    // "right", "left-left"; "left" always contains 'l', so target_id is never
    // left at -1 and `target == NULL` never fires. Prove it by showing the
    // model (which assumes target_id in {1,2}) reproduces C exactly.
    let mut rng = Rng::new(SEED ^ 27);
    for _ in 0..5000 {
        let (a, b, c, d) = (
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
        let (want, tid, _op) = model_inreftree(a, b, c, d);
        assert!(tid == 1 || tid == 2);
        let cv = p.c.inreftree(a, b, c, d);
        let rv = p.r.inreftree(a, b, c, d);
        assert_eq!(cv, want, "C vs model for inreftree({a},{b},{c},{d})");
        assert_eq!(rv, want, "Rust vs model for inreftree({a},{b},{c},{d})");
    }
}

#[test]
fn err28_inreftree_zero_target_value_falls_back_to_id_1() {
    let (_g, p) = harness();
    // param2 == 0 makes node 2's value 0 -> target_id forced from 2 to 1
    for p1 in -12..=12i32 {
        for p3 in [-1i32, 0, 1] {
            for p4 in [-1i32, 0, 1] {
                let (want, tid, _) = model_inreftree(p1, 0, p3, p4);
                assert_eq!(tid, 1, "param2 == 0 must force target_id 1");
                let cv = p.c.inreftree(p1, 0, p3, p4);
                let rv = p.r.inreftree(p1, 0, p3, p4);
                assert_eq!(cv, rv, "inreftree({p1},0,{p3},{p4})");
                assert_eq!(cv, want, "C vs model inreftree({p1},0,{p3},{p4})");
                // contrast: the same sum but with a non-zero param2 keeps id 2
                let (want2, tid2, _) = model_inreftree(p1 - 1, 1, p3, p4);
                assert_eq!(tid2, 2);
                assert_eq!(p.c.inreftree(p1 - 1, 1, p3, p4), want2);
                assert_eq!(p.r.inreftree(p1 - 1, 1, p3, p4), want2);
            }
        }
    }
}

#[test]
fn err29_inreftree_negative_residue_oob_rodata_read() {
    let (_g, p) = harness();
    // every negative residue, with both target-id fallbacks
    for sum in (-400..0).step_by(1) {
        for &p2 in &[0i32, 1, -1, 7] {
            let p1 = sum - p2;
            let (want, _tid, op) = model_inreftree(p1, p2, 0, 0);
            assert_eq!(
                op, 1,
                "every negative residue must decode to OP_ADD (bytes 'f','t','\\0')"
            );
            let cv = p.c.inreftree(p1, p2, 0, 0);
            let rv = p.r.inreftree(p1, p2, 0, 0);
            assert_eq!(cv, rv, "inreftree({p1},{p2},0,0) sum={sum}");
            assert_eq!(cv, want, "C vs model, sum={sum}");
        }
    }
}

#[test]
fn err30_inreftree_int_min_sum() {
    let (_g, p) = harness();
    for (a, b, c, d) in [
        (i32::MIN, 0, 0, 0),
        (i32::MIN, 1, -1, 0),
        (i32::MIN + 1, -1, 0, 0),
        (i32::MIN / 2, i32::MIN / 2, 0, 0),
    ] {
        let (want, _, _) = model_inreftree(a, b, c, d);
        let cv = p.c.inreftree(a, b, c, d);
        let rv = p.r.inreftree(a, b, c, d);
        assert_eq!(cv, rv, "inreftree({a},{b},{c},{d})");
        assert_eq!(cv, want, "C vs model inreftree({a},{b},{c},{d})");
    }
}

#[test]
fn err31_inreftree_sum_overflow_wraps_identically() {
    let (_g, p) = harness();
    let big = [i32::MAX, i32::MAX - 1, i32::MIN, i32::MIN + 1, 1 << 30, -(1 << 30)];
    for &a in &big {
        for &b in &big {
            for &c in &big {
                for &d in &big {
                    let (want, _, _) = model_inreftree(a, b, c, d);
                    let cv = p.c.inreftree(a, b, c, d);
                    let rv = p.r.inreftree(a, b, c, d);
                    assert_eq!(cv, rv, "inreftree({a},{b},{c},{d})");
                    assert_eq!(cv, want, "C vs model inreftree({a},{b},{c},{d})");
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// generic FFI boundary sweeps (required even though not in the table)
// ---------------------------------------------------------------------------

#[test]
fn generic_null_and_extreme_arguments() {
    let (_g, p) = harness();
    // NULL to parse_operation (the only function that documents a NULL check)
    assert_eq!(
        unsafe { p.c.parse_raw(std::ptr::null()) },
        unsafe { p.r.parse_raw(std::ptr::null()) }
    );
    // extreme ids everywhere
    for id in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
        p.reset();
        p.diff(&format!("add id {id}"), |l| l.add(id, id, -1, b"x"));
        p.diff(&format!("find {id}"), |l| l.find(id));
        p.diff(&format!("sum {id}"), |l| l.sum(id));
        p.diff(&format!("add parent {id}"), |l| l.add(1, 1, id, b"y"));
    }
    // one step past the valid Operation range in both directions
    for op in [0i32, 6] {
        assert_eq!(p.c.op_index(op), p.r.op_index(op), "op {op}");
    }
}

// ---------------------------------------------------------------------------
// row 35 — a self-referential child link IS reachable through the public API
// ---------------------------------------------------------------------------

#[test]
fn err35_duplicate_id_as_own_parent_creates_a_cycle() {
    let (_g, p) = harness();
    // node 1 is a root; then insert *another* node also called 1 whose parent is
    // 1. `find_node_by_id(1)` returns slot 0, whose left slot is free, so
    // slot0.left_child_id becomes 1 -- a link to itself.
    p.diff("root", |l| l.add(1, 10, -1, b"root"));
    p.diff("dup child of itself", |l| l.add(1, 20, 1, b"dup"));
    let (id, _v, _par, left, right) = p.c.decode(0);
    assert_eq!((id, left, right), (1, 1, -1), "C builds the self-cycle");
    assert_eq!(p.c.decode(0), p.r.decode(0), "both build the same cycle");
    p.assert_state("err35");
    // Both libraries would now recurse for ever in calculate_tree_sum(1); that
    // shared UB is not executed (see ERRORS.md). Assert they *agree it is a
    // cycle*, which is the observable part.
    assert!(!p.c.sum_terminates(1), "C table is cyclic");
    assert!(!p.r.sum_terminates(1), "Rust table is cyclic");
    // ... and that the rest of the API still behaves identically on it
    p.diff("find(1)", |l| l.find(1));
    p.diff("add another", |l| l.add(2, 30, 1, b"x"));
    p.diff("inreftree resets it all", |l| l.inreftree(2, 3, 4, 5));
}
