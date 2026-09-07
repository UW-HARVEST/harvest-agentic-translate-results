//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! Each test constructs the exact rejection condition, calls BOTH `.so`s, and
//! asserts they agree on the *specific* sentinel (`-1`, `NULL`, `0`, `OP_ADD`),
//! not merely that both "failed".

mod common;

use common::{cstr, Pair, Rng, TreeNode, MAX_NODES};
use std::ffi::c_int;

const OP_ADD: c_int = 1;
const OP_MULTIPLY: c_int = 2;
const OP_SUBTRACT: c_int = 3;
const OP_DIVIDE: c_int = 4;
const OP_MODULO: c_int = 5;

fn leaf(r: &mut Rng, id: c_int) -> TreeNode {
    let mut n = TreeNode::zeroed();
    n.id = id;
    n.value = r.interesting_i32();
    n.parent_id = -1;
    n.left_child_id = -1;
    n.right_child_id = -1;
    for k in 0..31 {
        n.label[k] = r.plain_byte() as i8;
    }
    n.label[31] = 0;
    n
}

fn seed(p: &Pair, rows: &[TreeNode], count: c_int) {
    p.reset_both();
    for (i, n) in rows.iter().enumerate() {
        p.c.set_node(i, *n);
        p.rs.set_node(i, *n);
    }
    p.c.set_node_count(count);
    p.rs.set_node_count(count);
    p.assert_state_eq("seed");
}

// --- Row 1: divide_op, b == 0 ---------------------------------------------

#[test]
fn err_row_01_divide_by_zero() {
    let p = Pair::load();
    let mut r = Rng::new(0xC001);
    for a in [0i32, 1, -1, i32::MIN, i32::MAX, i32::MIN + 1] {
        let ctx = format!("err1 divide_op({a},0)");
        let c = p.c.divide_op(a, 0, 0, 0);
        let rs = p.rs.divide_op(a, 0, 0, 0);
        p.eq(&ctx, c, rs);
        assert_eq!(c, 0, "{ctx}: C sentinel must be 0");
    }
    for i in 0..2000 {
        let a = r.interesting_i32();
        let (u1, u2) = (r.i32(), r.i32());
        let ctx = format!("err1 iter {i}: divide_op({a},0,{u1},{u2})");
        let c = p.c.divide_op(a, 0, u1, u2);
        p.eq(&ctx, c, p.rs.divide_op(a, 0, u1, u2));
        assert_eq!(c, 0, "{ctx}: sentinel");
    }
}

// --- Row 2: modulo_op, b == 0 ---------------------------------------------

#[test]
fn err_row_02_modulo_by_zero() {
    let p = Pair::load();
    let mut r = Rng::new(0xC002);
    for a in [0i32, 1, -1, i32::MIN, i32::MAX, i32::MIN + 1] {
        let ctx = format!("err2 modulo_op({a},0)");
        let c = p.c.modulo_op(a, 0, 0, 0);
        p.eq(&ctx, c, p.rs.modulo_op(a, 0, 0, 0));
        assert_eq!(c, 0, "{ctx}: C sentinel must be 0");
    }
    for i in 0..2000 {
        let a = r.interesting_i32();
        let (u1, u2) = (r.i32(), r.i32());
        let ctx = format!("err2 iter {i}: modulo_op({a},0,{u1},{u2})");
        let c = p.c.modulo_op(a, 0, u1, u2);
        p.eq(&ctx, c, p.rs.modulo_op(a, 0, u1, u2));
        assert_eq!(c, 0, "{ctx}: sentinel");
    }
}

// --- Row 3: find_node_by_id, id absent -----------------------------------

#[test]
fn err_row_03_find_node_absent() {
    let p = Pair::load();
    let mut r = Rng::new(0xC003);
    for i in 0..2000 {
        let count = 1 + r.usize(MAX_NODES);
        // ids 0..count-1 so any id outside that range is guaranteed absent.
        let rows: Vec<TreeNode> = (0..MAX_NODES).map(|k| leaf(&mut r, k as c_int)).collect();
        seed(&p, &rows, count as c_int);
        for probe in [
            count as c_int,
            MAX_NODES as c_int + 1,
            -1,
            i32::MIN,
            i32::MAX,
            -(r.usize(1_000_000) as c_int) - 1,
        ] {
            let ctx = format!("err3 iter {i} count {count}: find_node_by_id({probe})");
            let c = p.c.find_node_by_id(probe);
            p.eq(&ctx, c, p.rs.find_node_by_id(probe));
            assert_eq!(c, None, "{ctx}: C must return NULL");
        }
        p.assert_state_eq("err3");
    }
}

// --- Row 4: find_node_by_id, node_count == 0 -----------------------------

#[test]
fn err_row_04_find_node_empty_table() {
    let p = Pair::load();
    let mut r = Rng::new(0xC004);
    // Stale rows that WOULD match, but node_count == 0 hides them.
    let rows: Vec<TreeNode> = (0..MAX_NODES).map(|k| leaf(&mut r, k as c_int + 7)).collect();
    seed(&p, &rows, 0);
    for i in 0..2000 {
        let probe = if i < 60 { i as c_int } else { r.interesting_i32() };
        let ctx = format!("err4 find_node_by_id({probe}) with node_count=0");
        let c = p.c.find_node_by_id(probe);
        p.eq(&ctx, c, p.rs.find_node_by_id(probe));
        assert_eq!(c, None, "{ctx}: C must return NULL");
    }
    p.assert_state_eq("err4");
}

// --- Row 5: find_node_by_id, node_count < 0 ------------------------------

#[test]
fn err_row_05_find_node_negative_count() {
    let p = Pair::load();
    let mut r = Rng::new(0xC005);
    let rows: Vec<TreeNode> = (0..MAX_NODES).map(|k| leaf(&mut r, k as c_int + 7)).collect();
    for &nc in &[-1i32, -2, -50, -1000, i32::MIN] {
        seed(&p, &rows, nc);
        for i in 0..300 {
            let probe = if i < 60 { i as c_int + 7 } else { r.interesting_i32() };
            let ctx = format!("err5 node_count={nc}: find_node_by_id({probe})");
            let c = p.c.find_node_by_id(probe);
            p.eq(&ctx, c, p.rs.find_node_by_id(probe));
            assert_eq!(c, None, "{ctx}: C must return NULL");
        }
        p.assert_state_eq("err5");
    }
}

// --- Row 6: add_tree_node, table exactly full ----------------------------

#[test]
fn err_row_06_add_node_table_full() {
    let p = Pair::load();
    let mut r = Rng::new(0xC006);
    let rows: Vec<TreeNode> = (0..MAX_NODES).map(|k| leaf(&mut r, k as c_int + 1)).collect();
    for i in 0..1000 {
        seed(&p, &rows, MAX_NODES as c_int);
        let before = p.c.table_bytes();
        let label = cstr(&(0..r.usize(40)).map(|_| r.plain_byte()).collect::<Vec<u8>>());
        let (id, v) = (r.interesting_i32(), r.interesting_i32());
        let par = [-1i32, 1, 2, 999][r.usize(4)];
        let ctx = format!("err6 iter {i}: add_tree_node({id},{v},{par}) at node_count=50");
        let c = p.c.add_tree_node(id, v, par, &label);
        p.eq(&ctx, c, p.rs.add_tree_node(id, v, par, &label));
        assert_eq!(c, -1, "{ctx}: C sentinel must be -1");
        assert_eq!(p.c.node_count(), MAX_NODES as c_int, "{ctx}: node_count unchanged");
        assert_eq!(p.c.table_bytes(), before, "{ctx}: table must be untouched");
        p.assert_state_eq(&ctx);
    }
}

// --- Row 7: add_tree_node, node_count > MAX_NODES ------------------------

#[test]
fn err_row_07_add_node_count_over_max() {
    let p = Pair::load();
    let mut r = Rng::new(0xC007);
    let rows: Vec<TreeNode> = (0..MAX_NODES).map(|k| leaf(&mut r, k as c_int + 1)).collect();
    for &nc in &[51i32, 60, 1000, i32::MAX] {
        for i in 0..250 {
            seed(&p, &rows, nc);
            let before = p.c.table_bytes();
            let label = cstr(b"x");
            let (id, v) = (r.interesting_i32(), r.interesting_i32());
            let ctx = format!("err7 node_count={nc} iter {i}: add_tree_node({id},{v},-1)");
            let c = p.c.add_tree_node(id, v, -1, &label);
            p.eq(&ctx, c, p.rs.add_tree_node(id, v, -1, &label));
            assert_eq!(c, -1, "{ctx}: C sentinel must be -1");
            assert_eq!(p.c.node_count(), nc, "{ctx}: node_count unchanged");
            assert_eq!(p.c.table_bytes(), before, "{ctx}: table untouched");
            p.assert_state_eq(&ctx);
        }
    }
}

// --- Row 8: add_tree_node, parent missing -------------------------------

#[test]
fn err_row_08_add_node_missing_parent() {
    let p = Pair::load();
    let mut r = Rng::new(0xC008);
    for i in 0..2000 {
        p.reset_both();
        // A populated table whose ids are all in 0..count, so any other
        // parent_id is guaranteed missing.
        let count = r.usize(MAX_NODES);
        let rows: Vec<TreeNode> = (0..MAX_NODES).map(|k| leaf(&mut r, k as c_int)).collect();
        seed(&p, &rows, count as c_int);

        let par = match r.next_u64() % 4 {
            0 => count as c_int + 1,
            1 => i32::MIN,
            2 => i32::MAX,
            _ => -(r.usize(1_000_000) as c_int) - 2, // never -1
        };
        assert_ne!(par, -1);
        let label = cstr(&(0..r.usize(40)).map(|_| r.plain_byte()).collect::<Vec<u8>>());
        let (id, v) = (r.interesting_i32(), r.interesting_i32());
        let ctx = format!("err8 iter {i} count {count}: add_tree_node({id},{v},{par})");
        let c = p.c.add_tree_node(id, v, par, &label);
        p.eq(&ctx, c, p.rs.add_tree_node(id, v, par, &label));
        assert_eq!(c, -1, "{ctx}: C sentinel must be -1");
        assert_eq!(p.c.node_count(), count as c_int, "{ctx}: node_count NOT incremented");
        // The C writes the row BEFORE the parent lookup fails, so the slot at
        // node_count is left half-initialised. Both libraries must leave the
        // SAME bytes behind — this is the quirk assert_state_eq pins down.
        if count < MAX_NODES {
            let n = p.c.node(count);
            assert_eq!(
                (n.id, n.value, n.parent_id, n.left_child_id, n.right_child_id),
                (id, v, par, -1, -1),
                "{ctx}: C leaves the row written even though it returned -1"
            );
        }
        p.assert_state_eq(&ctx);
    }
}

// --- Row 9: add_tree_node, parent already has both children -------------

#[test]
fn err_row_09_add_node_parent_full() {
    let p = Pair::load();
    let mut r = Rng::new(0xC009);
    for i in 0..2000 {
        p.reset_both();
        // -1 means "no parent" to add_tree_node, so it must not be used as an id.
        let root = { let v = r.small(); if v == -1 { 5 } else { v } };
        let lbl = cstr(b"root");
        let rv = r.interesting_i32();
        p.c.add_tree_node(root, rv, -1, &lbl);
        p.rs.add_tree_node(root, rv, -1, &lbl);

        for k in 1..=2 {
            let v = r.interesting_i32();
            let l = cstr(&[b'c', b'0' + k as u8]);
            p.c.add_tree_node(root.wrapping_add(k), v, root, &l);
            p.rs.add_tree_node(root.wrapping_add(k), v, root, &l);
        }
        p.assert_state_eq("err9 parent saturated");
        let pn = p.c.node(0);
        assert_ne!(pn.left_child_id, -1, "err9: left must be linked");
        assert_ne!(pn.right_child_id, -1, "err9: right must be linked");

        // Third child: succeeds (returns the new index) but is never linked.
        let v = r.interesting_i32();
        let l = cstr(b"c3");
        let ctx = format!("err9 iter {i}: third child of saturated parent");
        let c = p.c.add_tree_node(root.wrapping_add(3), v, root, &l);
        p.eq(&ctx, c, p.rs.add_tree_node(root.wrapping_add(3), v, root, &l));
        assert_eq!(c, 3, "{ctx}: C returns node_count-1 == 3 (SUCCESS, not -1)");
        let pn2 = p.c.node(0);
        assert_eq!(
            (pn2.left_child_id, pn2.right_child_id),
            (pn.left_child_id, pn.right_child_id),
            "{ctx}: parent links must be unchanged — the child is silently dropped"
        );
        p.assert_state_eq(&ctx);
    }
}

// --- Rows 10-12: label boundary shapes ----------------------------------

#[test]
fn err_row_10_label_truncation() {
    let p = Pair::load();
    let mut r = Rng::new(0xC010);
    for &len in &[32usize, 33, 40, 64, 200, 1000] {
        for i in 0..200 {
            p.reset_both();
            let label = cstr(&(0..len).map(|_| r.plain_byte()).collect::<Vec<u8>>());
            let ctx = format!("err10 len {len} iter {i}");
            let c = p.c.add_tree_node(1, 5, -1, &label);
            p.eq(&ctx, c, p.rs.add_tree_node(1, 5, -1, &label));
            assert_eq!(c, 0, "{ctx}: success");
            let n = p.c.node(0);
            for k in 0..31 {
                assert_eq!(n.label[k] as u8, label[k], "{ctx}: label byte {k}");
            }
            assert_eq!(n.label[31], 0, "{ctx}: label[31] forced to NUL");
            p.assert_state_eq(&ctx);
        }
    }
}

#[test]
fn err_row_11_label_exactly_31() {
    let p = Pair::load();
    let mut r = Rng::new(0xC011);
    for i in 0..500 {
        p.reset_both();
        let body: Vec<u8> = (0..31).map(|_| r.plain_byte()).collect();
        let label = cstr(&body);
        let ctx = format!("err11 iter {i}: 31-byte label");
        let c = p.c.add_tree_node(1, 5, -1, &label);
        p.eq(&ctx, c, p.rs.add_tree_node(1, 5, -1, &label));
        let n = p.c.node(0);
        for k in 0..31 {
            assert_eq!(n.label[k] as u8, body[k], "{ctx}: byte {k}");
        }
        assert_eq!(n.label[31], 0, "{ctx}: strncpy copies no NUL; label[31]=0 supplies it");
        p.assert_state_eq(&ctx);
    }
}

#[test]
fn err_row_12_label_empty() {
    let p = Pair::load();
    // A previously-written row must be fully zero-padded by strncpy.
    let mut r = Rng::new(0xC012);
    for i in 0..500 {
        p.reset_both();
        let dirty = cstr(&(0..31).map(|_| r.plain_byte()).collect::<Vec<u8>>());
        p.c.add_tree_node(1, 1, -1, &dirty);
        p.rs.add_tree_node(1, 1, -1, &dirty);
        p.c.set_node_count(0);
        p.rs.set_node_count(0);
        let empty = cstr(b"");
        let ctx = format!("err12 iter {i}: empty label over a dirty slot");
        let c = p.c.add_tree_node(2, 2, -1, &empty);
        p.eq(&ctx, c, p.rs.add_tree_node(2, 2, -1, &empty));
        let n = p.c.node(0);
        assert!(n.label.iter().all(|&b| b == 0), "{ctx}: all 32 label bytes zeroed");
        p.assert_state_eq(&ctx);
    }
}

// --- Rows 13-15: calculate_tree_sum returning 0 -------------------------

#[test]
fn err_row_13_tree_sum_unknown_id() {
    let p = Pair::load();
    let mut r = Rng::new(0xC013);
    for i in 0..1500 {
        let count = 1 + r.usize(MAX_NODES);
        let rows: Vec<TreeNode> = (0..MAX_NODES).map(|k| leaf(&mut r, k as c_int)).collect();
        seed(&p, &rows, count as c_int);
        for probe in [count as c_int, i32::MIN, i32::MAX, -1, -12345] {
            let ctx = format!("err13 iter {i} count {count}: calculate_tree_sum({probe})");
            let c = p.c.calculate_tree_sum(probe);
            p.eq(&ctx, c, p.rs.calculate_tree_sum(probe));
            assert_eq!(c, 0, "{ctx}: C sentinel must be 0");
        }
        p.assert_state_eq("err13");
    }
}

#[test]
fn err_row_14_tree_sum_empty_table() {
    let p = Pair::load();
    let mut r = Rng::new(0xC014);
    let rows: Vec<TreeNode> = (0..MAX_NODES).map(|k| leaf(&mut r, k as c_int)).collect();
    for &nc in &[0i32, -1, -100, i32::MIN] {
        seed(&p, &rows, nc);
        for i in 0..400 {
            let probe = if i < 60 { i as c_int } else { r.interesting_i32() };
            let ctx = format!("err14 node_count={nc}: calculate_tree_sum({probe})");
            let c = p.c.calculate_tree_sum(probe);
            p.eq(&ctx, c, p.rs.calculate_tree_sum(probe));
            assert_eq!(c, 0, "{ctx}: C sentinel must be 0");
        }
        p.assert_state_eq("err14");
    }
}

#[test]
fn err_row_15_tree_sum_dangling_child() {
    let p = Pair::load();
    let mut r = Rng::new(0xC015);
    for i in 0..1500 {
        let count = 1 + r.usize(MAX_NODES);
        let mut rows: Vec<TreeNode> = (0..MAX_NODES).map(|k| leaf(&mut r, k as c_int)).collect();
        // Dangling links: != -1 but naming ids that do not exist in 0..count.
        let dangling_l = match r.next_u64() % 3 {
            0 => count as c_int + 5,
            1 => i32::MAX,
            _ => -777,
        };
        let dangling_r = match r.next_u64() % 3 {
            0 => count as c_int + 9,
            1 => i32::MIN,
            _ => -888,
        };
        rows[0].left_child_id = dangling_l;
        rows[0].right_child_id = dangling_r;
        seed(&p, &rows, count as c_int);
        let ctx = format!("err15 iter {i}: sum(0) with dangling {dangling_l}/{dangling_r}");
        let c = p.c.calculate_tree_sum(0);
        p.eq(&ctx, c, p.rs.calculate_tree_sum(0));
        // Both dangling branches contribute 0, so the answer is the node's value.
        assert_eq!(c, rows[0].value, "{ctx}: dangling branches contribute 0");
        p.assert_state_eq("err15");
    }
}

// --- Rows 16-17: parse_operation fallbacks ------------------------------

#[test]
fn err_row_16_parse_null() {
    let p = Pair::load();
    for i in 0..100 {
        let ctx = format!("err16 iter {i}: parse_operation(NULL)");
        let c = p.c.parse_operation_null();
        p.eq(&ctx, c, p.rs.parse_operation_null());
        assert_eq!(c, OP_ADD, "{ctx}: NULL must yield OP_ADD (1), not a crash");
    }
}

#[test]
fn err_row_17_parse_no_operator() {
    let p = Pair::load();
    let mut r = Rng::new(0xC017);
    let empty = cstr(b"");
    let c = p.c.parse_operation(&empty);
    p.eq("err17 empty string", c, p.rs.parse_operation(&empty));
    assert_eq!(c, OP_ADD, "err17: empty string falls through to OP_ADD");
    for i in 0..3000 {
        let len = r.usize(64);
        let s = cstr(&(0..len).map(|_| r.plain_byte()).collect::<Vec<u8>>());
        let ctx = format!("err17 iter {i}: len {len}");
        let c = p.c.parse_operation(&s);
        p.eq(&ctx, c, p.rs.parse_operation(&s));
        assert_eq!(c, OP_ADD, "{ctx}: no operator present -> OP_ADD");
    }
}

// --- Row 18: get_operation_func with out-of-range enum values -----------

#[test]
fn err_row_18_get_op_func_out_of_range() {
    let p = Pair::load();
    let mut r = Rng::new(0xC018);

    // Every valid variant must map to its own function, and the ADD fallback
    // must be indistinguishable from add_op.
    let probe = |op: c_int, p: &Pair, a: c_int, b: c_int| -> (c_int, c_int) {
        let cf = p.c.get_operation_func(op);
        let rf = p.rs.get_operation_func(op);
        (p.c.call_op(cf, a, b, 0, 0), p.rs.call_op(rf, a, b, 0, 0))
    };

    // Exhaustive small range around the enum, plus the extremes.
    let mut oob: Vec<c_int> = (-32..=64).collect();
    oob.extend_from_slice(&[
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
        0,
        6,
        -1,
        1 << 16,
        -(1 << 16),
        0x7FFF_FFFE,
    ]);
    for _ in 0..2000 {
        oob.push(r.i32());
    }

    for op in oob {
        let a = r.interesting_i32();
        let b = {
            let b = r.interesting_i32();
            // Guard the trapping corner in case op selects divide/modulo.
            if a == i32::MIN && b == -1 { 3 } else { b }
        };
        let ctx = format!("err18 get_operation_func({op}) then call({a},{b})");
        let (c, rs) = probe(op, &p, a, b);
        p.eq(&ctx, c, rs);
        if !(OP_ADD..=OP_MODULO).contains(&op) {
            // default: -> add_op
            let expect = p.c.add_op(a, b, 0, 0);
            assert_eq!(c, expect, "{ctx}: out-of-range op must fall back to add_op");
        }
    }

    // Each valid variant maps to a distinct, correct operation.
    let (a, b) = (100, 7);
    for (op, expect) in [
        (OP_ADD, p.c.add_op(a, b, 0, 0)),
        (OP_MULTIPLY, p.c.multiply_op(a, b, 0, 0)),
        (OP_SUBTRACT, p.c.subtract_op(a, b, 0, 0)),
        (OP_DIVIDE, p.c.divide_op(a, b, 0, 0)),
        (OP_MODULO, p.c.modulo_op(a, b, 0, 0)),
    ] {
        let (c, rs) = probe(op, &p, a, b);
        p.eq(&format!("err18 valid op {op}"), c, rs);
        assert_eq!(c, expect, "err18: op {op} must select its own function");
    }
}

// --- Rows 19-20: inreftree target reset ---------------------------------

#[test]
fn err_row_19_inreftree_target_reset() {
    let p = Pair::load();
    let mut r = Rng::new(0xC019);
    // param2 == 0 makes the 'l'-labelled target (node id 2) have value 0, so
    // target_id is reset from 2 to 1, changing the final operand.
    for i in 0..3000 {
        let (p1, p3, p4) = (r.interesting_i32(), r.interesting_i32(), r.interesting_i32());
        let ctx = format!("err19 iter {i}: inreftree({p1},0,{p3},{p4})");
        p.eq(&ctx, p.c.inreftree(p1, 0, p3, p4), p.rs.inreftree(p1, 0, p3, p4));
        p.assert_state_eq(&ctx);
    }
    // Pin the observable difference: with the same tree_sum, param2==0 vs !=0
    // must give different results in BOTH libraries (target_id 1 vs 2).
    // sum = 4 -> residue 0 -> '+' -> add(sum, target_id).
    let with_zero_c = p.c.inreftree(4, 0, 0, 0);
    let with_zero_r = p.rs.inreftree(4, 0, 0, 0);
    p.eq("err19 sum4 param2=0", with_zero_c, with_zero_r);
    assert_eq!(with_zero_c, 5, "err19: 4 + target_id(1) == 5");
    let with_nz_c = p.c.inreftree(0, 4, 0, 0);
    let with_nz_r = p.rs.inreftree(0, 4, 0, 0);
    p.eq("err19 sum4 param2=4", with_nz_c, with_nz_r);
    assert_eq!(with_nz_c, 6, "err19: 4 + target_id(2) == 6");
}

// --- Row 21: negative op_string index -----------------------------------

#[test]
fn err_row_21_negative_modulus_index() {
    let p = Pair::load();
    let mut r = Rng::new(0xC021);
    // tree_sum % 4 in {-1,-2,-3}: the C indexes op_string[-1..-3], reading the
    // .rodata bytes before "+*-%" ('\0', 't', 'f'). None is an operator, so
    // parse_operation returns OP_ADD; the Rust must reproduce the same bytes.
    for residue in [-1i32, -2, -3] {
        for k in 0..500 {
            // Build a sum with the desired negative residue.
            let sum = residue - 4 * (k as i32) - 4;
            assert_eq!(sum % 4, residue, "test setup: residue");
            let p2 = 1 + r.usize(50) as i32; // nonzero -> target_id stays 2
            let p3 = r.usize(50) as i32;
            let p4 = -(r.usize(50) as i32);
            let p1 = sum - p2 - p3 - p4;
            let ctx = format!("err21 residue {residue} k {k}: inreftree({p1},{p2},{p3},{p4})");
            let c = p.c.inreftree(p1, p2, p3, p4);
            p.eq(&ctx, c, p.rs.inreftree(p1, p2, p3, p4));
            // The out-of-bounds byte is never an operator -> OP_ADD -> sum + 2.
            assert_eq!(c, sum + 2, "{ctx}: negative index must resolve to OP_ADD");
            p.assert_state_eq(&ctx);
        }
    }
}

// --- Generic FFI boundary conditions -----------------------------------

#[test]
fn err_generic_null_pointer_arguments() {
    let p = Pair::load();
    // parse_operation(NULL) — covered by row 16, re-asserted here alongside the
    // other null-tolerant boundary.
    p.eq(
        "generic parse_operation(NULL)",
        p.c.parse_operation_null(),
        p.rs.parse_operation_null(),
    );
    // add_tree_node with a NULL label would be dereferenced by strncpy in the C
    // (no null check exists), so it is NOT exercised: it segfaults in both.
    // Instead: the shortest legal label, which is the boundary next to NULL.
    p.reset_both();
    let empty = cstr(b"");
    p.eq(
        "generic add_tree_node empty label",
        p.c.add_tree_node(1, 1, -1, &empty),
        p.rs.add_tree_node(1, 1, -1, &empty),
    );
    p.assert_state_eq("generic empty label");
}

#[test]
fn err_generic_boundaries_one_past_valid_range() {
    let p = Pair::load();
    let mut r = Rng::new(0xC0FF);

    // node_count boundary: 49 accepts, 50 rejects.
    let rows: Vec<TreeNode> = (0..MAX_NODES).map(|k| leaf(&mut r, k as c_int + 1)).collect();
    for &nc in &[48i32, 49, 50, 51] {
        seed(&p, &rows, nc);
        let label = cstr(b"boundary");
        let ctx = format!("generic node_count={nc} add_tree_node");
        let c = p.c.add_tree_node(9999, 1, -1, &label);
        p.eq(&ctx, c, p.rs.add_tree_node(9999, 1, -1, &label));
        let expected = if nc >= MAX_NODES as c_int { -1 } else { nc };
        assert_eq!(c, expected, "{ctx}: boundary sentinel");
        p.assert_state_eq(&ctx);
    }

    // Operation enum boundary: one below the first variant and one above the last.
    for op in [0i32, 6] {
        let cf = p.c.get_operation_func(op);
        let rf = p.rs.get_operation_func(op);
        let ctx = format!("generic get_operation_func({op})");
        p.eq(&ctx, p.c.call_op(cf, 20, 6, 0, 0), p.rs.call_op(rf, 20, 6, 0, 0));
        assert_eq!(
            p.c.call_op(cf, 20, 6, 0, 0),
            26,
            "{ctx}: must behave as add_op"
        );
    }

    // Oversized "length": a label far longer than the 32-byte field.
    p.reset_both();
    let huge = cstr(&vec![b'Z'; 100_000]);
    let ctx = "generic 100000-byte label";
    p.eq(ctx, p.c.add_tree_node(1, 1, -1, &huge), p.rs.add_tree_node(1, 1, -1, &huge));
    p.assert_state_eq(ctx);

    // Zero-valued everything.
    p.reset_both();
    let z = cstr(b"");
    p.eq("generic zeros", p.c.add_tree_node(0, 0, -1, &z), p.rs.add_tree_node(0, 0, -1, &z));
    p.assert_state_eq("generic zeros");
}
