//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Rows 1–9 cover the leaf-level pure functions and the dispatch layer.
//! Rows 10–15 drive the table-backed low-level entry points.
//! Rows 16–21 compose the whole pipeline.

mod common;

use common::{cstr, Pair, Rng, TreeNode, MAX_NODES};
use std::ffi::c_int;

const ITERS: usize = 4000;

// ---------------------------------------------------------------------------
// Row 1–3: wrapping arithmetic
// ---------------------------------------------------------------------------

#[test]
fn configs_row_01_add_op() {
    let p = Pair::load();
    let mut r = Rng::new(0x1001);
    for i in 0..ITERS {
        let (a, b) = (r.interesting_i32(), r.interesting_i32());
        let (u1, u2) = (r.i32(), r.i32());
        let ctx = format!("row1 iter {i}: add_op({a},{b},{u1},{u2})");
        p.eq(&ctx, p.c.add_op(a, b, u1, u2), p.rs.add_op(a, b, u1, u2));
    }
    // Explicit overflow boundaries.
    for &(a, b) in &[
        (i32::MAX, 1),
        (i32::MAX, i32::MAX),
        (i32::MIN, -1),
        (i32::MIN, i32::MIN),
        (0, 0),
    ] {
        let ctx = format!("row1 corner add_op({a},{b})");
        p.eq(&ctx, p.c.add_op(a, b, 0, 0), p.rs.add_op(a, b, 0, 0));
    }
}

#[test]
fn configs_row_02_multiply_op() {
    let p = Pair::load();
    let mut r = Rng::new(0x1002);
    for i in 0..ITERS {
        let (a, b) = (r.interesting_i32(), r.interesting_i32());
        let (u1, u2) = (r.i32(), r.i32());
        let ctx = format!("row2 iter {i}: multiply_op({a},{b},{u1},{u2})");
        p.eq(
            &ctx,
            p.c.multiply_op(a, b, u1, u2),
            p.rs.multiply_op(a, b, u1, u2),
        );
    }
    for &(a, b) in &[
        (i32::MAX, 2),
        (i32::MIN, -1),
        (i32::MIN, 2),
        (65536, 65536),
        (-1, i32::MIN),
    ] {
        let ctx = format!("row2 corner multiply_op({a},{b})");
        p.eq(&ctx, p.c.multiply_op(a, b, 0, 0), p.rs.multiply_op(a, b, 0, 0));
    }
}

#[test]
fn configs_row_03_subtract_op() {
    let p = Pair::load();
    let mut r = Rng::new(0x1003);
    for i in 0..ITERS {
        let (a, b) = (r.interesting_i32(), r.interesting_i32());
        let (u1, u2) = (r.i32(), r.i32());
        let ctx = format!("row3 iter {i}: subtract_op({a},{b},{u1},{u2})");
        p.eq(
            &ctx,
            p.c.subtract_op(a, b, u1, u2),
            p.rs.subtract_op(a, b, u1, u2),
        );
    }
    for &(a, b) in &[(i32::MIN, 1), (i32::MIN, i32::MAX), (i32::MAX, -1), (0, i32::MIN)] {
        let ctx = format!("row3 corner subtract_op({a},{b})");
        p.eq(&ctx, p.c.subtract_op(a, b, 0, 0), p.rs.subtract_op(a, b, 0, 0));
    }
}

/// `INT_MIN / -1` and `INT_MIN % -1` trap with SIGFPE in the C (ERRORS.md E1/E2),
/// so they are excluded from the randomized divide/modulo rows.
fn div_trap(a: c_int, b: c_int) -> bool {
    a == i32::MIN && b == -1
}

#[test]
fn configs_row_04_divide_op_nonzero_divisor() {
    let p = Pair::load();
    let mut r = Rng::new(0x1004);
    let mut quadrants = [0usize; 4];
    let mut done = 0;
    while done < ITERS {
        let a = r.interesting_i32();
        let b = r.interesting_i32();
        if b == 0 || div_trap(a, b) {
            continue;
        }
        quadrants[(((a < 0) as usize) << 1) | (b < 0) as usize] += 1;
        let ctx = format!("row4 iter {done}: divide_op({a},{b})");
        p.eq(&ctx, p.c.divide_op(a, b, 0, 0), p.rs.divide_op(a, b, 0, 0));
        done += 1;
    }
    // All four sign quadrants must have been exercised (C truncates toward zero).
    assert!(
        quadrants.iter().all(|&n| n > 100),
        "row4: sign quadrants under-covered: {quadrants:?}"
    );
    for &(a, b) in &[(7, 2), (-7, 2), (7, -2), (-7, -2), (i32::MIN, 1), (i32::MIN, 2), (i32::MAX, -1)] {
        let ctx = format!("row4 corner divide_op({a},{b})");
        p.eq(&ctx, p.c.divide_op(a, b, 0, 0), p.rs.divide_op(a, b, 0, 0));
    }
}

#[test]
fn configs_row_05_modulo_op_nonzero_divisor() {
    let p = Pair::load();
    let mut r = Rng::new(0x1005);
    let mut quadrants = [0usize; 4];
    let mut done = 0;
    while done < ITERS {
        let a = r.interesting_i32();
        let b = r.interesting_i32();
        if b == 0 || div_trap(a, b) {
            continue;
        }
        quadrants[(((a < 0) as usize) << 1) | (b < 0) as usize] += 1;
        let ctx = format!("row5 iter {done}: modulo_op({a},{b})");
        p.eq(&ctx, p.c.modulo_op(a, b, 0, 0), p.rs.modulo_op(a, b, 0, 0));
        done += 1;
    }
    assert!(
        quadrants.iter().all(|&n| n > 100),
        "row5: sign quadrants under-covered: {quadrants:?}"
    );
    for &(a, b) in &[(7, 4), (-7, 4), (7, -4), (-7, -4), (i32::MIN, 4), (i32::MIN, 1)] {
        let ctx = format!("row5 corner modulo_op({a},{b})");
        p.eq(&ctx, p.c.modulo_op(a, b, 0, 0), p.rs.modulo_op(a, b, 0, 0));
    }
}

// ---------------------------------------------------------------------------
// Row 6: get_operation_func + invoking the returned pointer
// ---------------------------------------------------------------------------

#[test]
fn configs_row_06_get_operation_func_dispatch() {
    let p = Pair::load();
    let mut r = Rng::new(0x1006);
    for op in 1..=5 {
        let cf = p.c.get_operation_func(op);
        let rf = p.rs.get_operation_func(op);
        assert!(!(cf as usize == 0), "row6: C returned NULL fn ptr for op {op}");
        assert!(!(rf as usize == 0), "row6: Rust returned NULL fn ptr for op {op}");
        for i in 0..ITERS {
            let a = r.interesting_i32();
            let mut b = r.interesting_i32();
            // op 4/5 are divide/modulo; keep them out of the trapping corner.
            if (op == 4 || op == 5) && div_trap(a, b) {
                b = 3;
            }
            let ctx = format!("row6 op {op} iter {i}: dispatch({a},{b})");
            p.eq(&ctx, p.c.call_op(cf, a, b, 0, 0), p.rs.call_op(rf, a, b, 0, 0));
        }
        // The dispatch must also agree with calling the named export directly,
        // proving the pointer identifies the right function and not just a
        // coincidentally-equal one.
        let (a, b) = (91, 7);
        let direct = match op {
            1 => p.c.add_op(a, b, 0, 0),
            2 => p.c.multiply_op(a, b, 0, 0),
            3 => p.c.subtract_op(a, b, 0, 0),
            4 => p.c.divide_op(a, b, 0, 0),
            _ => p.c.modulo_op(a, b, 0, 0),
        };
        assert_eq!(
            p.rs.call_op(rf, a, b, 0, 0),
            direct,
            "row6: Rust dispatch for op {op} does not match the C named export"
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 7–9: parse_operation
// ---------------------------------------------------------------------------

const OPERATORS: [u8; 5] = [b'+', b'*', b'-', b'/', b'%'];

#[test]
fn configs_row_07_parse_single_operator() {
    let p = Pair::load();
    let mut r = Rng::new(0x1007);
    for i in 0..ITERS {
        let opb = OPERATORS[r.usize(OPERATORS.len())];
        let len = r.usize(12);
        let mut s: Vec<u8> = (0..len).map(|_| r.plain_byte()).collect();
        let pos = r.usize(len + 1);
        s.insert(pos, opb);
        let s = cstr(&s);
        let ctx = format!("row7 iter {i}: parse_operation({:?})", String::from_utf8_lossy(&s));
        p.eq(&ctx, p.c.parse_operation(&s), p.rs.parse_operation(&s));
    }
}

#[test]
fn configs_row_08_parse_multiple_operators_precedence() {
    let p = Pair::load();
    let mut r = Rng::new(0x1008);
    for i in 0..ITERS {
        // 2..=5 distinct operators, shuffled, with random filler between them.
        let mut ops = OPERATORS;
        for k in (1..ops.len()).rev() {
            let j = r.usize(k + 1);
            ops.swap(k, j);
        }
        let n = 2 + r.usize(4);
        let mut s: Vec<u8> = Vec::new();
        for &o in &ops[..n] {
            for _ in 0..r.usize(3) {
                s.push(r.plain_byte());
            }
            s.push(o);
        }
        let s = cstr(&s);
        let ctx = format!("row8 iter {i}: parse_operation({:?})", String::from_utf8_lossy(&s));
        p.eq(&ctx, p.c.parse_operation(&s), p.rs.parse_operation(&s));
    }
    // Fixed cases pinning the check order: '+' beats '*' beats '-' beats '/' beats '%'.
    for s in [
        &b"%/-*+"[..], &b"/-*+"[..], &b"-*+"[..], &b"*+"[..], &b"+"[..],
        &b"%/-*"[..], &b"%/-"[..], &b"%/"[..], &b"%"[..],
    ] {
        let s = cstr(s);
        let ctx = format!("row8 fixed parse_operation({:?})", String::from_utf8_lossy(&s));
        p.eq(&ctx, p.c.parse_operation(&s), p.rs.parse_operation(&s));
    }
}

#[test]
fn configs_row_09_parse_all_byte_values_and_no_operator() {
    let p = Pair::load();
    // Every possible single byte, including 0 (empty string).
    for b in 0u16..=255 {
        let s = if b == 0 { vec![0u8] } else { vec![b as u8, 0] };
        let ctx = format!("row9 single byte {b}");
        p.eq(&ctx, p.c.parse_operation(&s), p.rs.parse_operation(&s));
    }
    let mut r = Rng::new(0x1009);
    for i in 0..ITERS {
        let len = r.usize(40);
        let s: Vec<u8> = (0..len).map(|_| r.plain_byte()).collect();
        let s = cstr(&s);
        let ctx = format!("row9 iter {i}: len {len}");
        p.eq(&ctx, p.c.parse_operation(&s), p.rs.parse_operation(&s));
    }
}

// ---------------------------------------------------------------------------
// Rows 10–11: find_node_by_id
// ---------------------------------------------------------------------------

/// Write an identical arbitrary table image into both libraries.
fn seed_table(p: &Pair, rows: &[TreeNode], count: c_int) {
    p.reset_both();
    for (i, n) in rows.iter().enumerate() {
        p.c.set_node(i, *n);
        p.rs.set_node(i, *n);
    }
    p.c.set_node_count(count);
    p.rs.set_node_count(count);
    p.assert_state_eq("seed_table");
}

fn rand_node(r: &mut Rng, id: c_int) -> TreeNode {
    let mut n = TreeNode::zeroed();
    n.id = id;
    n.value = r.interesting_i32();
    n.parent_id = r.small();
    n.left_child_id = -1;
    n.right_child_id = -1;
    for k in 0..31 {
        n.label[k] = r.plain_byte() as i8;
    }
    n.label[31] = 0;
    n
}

#[test]
fn configs_row_10_find_node_positions_and_counts() {
    let p = Pair::load();
    let mut r = Rng::new(0x1010);
    for &count in &[1usize, 2, 25, MAX_NODES] {
        for i in 0..400 {
            let ids: Vec<c_int> = (0..count)
                .map(|k| match r.next_u64() % 5 {
                    0 => i32::MIN + k as i32,
                    1 => i32::MAX - k as i32,
                    2 => -(k as i32) - 1,
                    _ => (k as i32) * 3 + 1,
                })
                .collect();
            let rows: Vec<TreeNode> = ids.iter().map(|&id| rand_node(&mut r, id)).collect();
            seed_table(&p, &rows, count as c_int);
            // first / middle / last / absent
            let mut probes = vec![ids[0], ids[count / 2], ids[count - 1]];
            probes.push(r.i32());
            probes.push(-1);
            probes.push(0);
            for probe in probes {
                let ctx = format!("row10 count {count} iter {i}: find_node_by_id({probe})");
                p.eq(&ctx, p.c.find_node_by_id(probe), p.rs.find_node_by_id(probe));
            }
            p.assert_state_eq("row10 after probes");
        }
    }
}

#[test]
fn configs_row_11_find_node_duplicates_and_beyond_count() {
    let p = Pair::load();
    let mut r = Rng::new(0x1011);
    for i in 0..1000 {
        let count = 1 + r.usize(MAX_NODES - 1);
        let dup_id = r.small();
        let mut rows: Vec<TreeNode> = (0..MAX_NODES)
            .map(|k| rand_node(&mut r, 10_000 + k as c_int))
            .collect();
        // Two duplicates inside the live range: the first must win.
        let a = r.usize(count);
        let b = r.usize(count);
        rows[a].id = dup_id;
        rows[b].id = dup_id;
        // A matching row past node_count must be invisible.
        if count < MAX_NODES {
            let beyond = count + r.usize(MAX_NODES - count);
            rows[beyond].id = dup_id;
        }
        seed_table(&p, &rows, count as c_int);
        let ctx = format!("row11 iter {i}: count {count}, find_node_by_id({dup_id})");
        let cf = p.c.find_node_by_id(dup_id);
        let rf = p.rs.find_node_by_id(dup_id);
        p.eq(&ctx, cf, rf);
        assert_eq!(cf, Some(a.min(b)), "{ctx}: expected first duplicate slot");
        p.assert_state_eq("row11");
    }
}

// ---------------------------------------------------------------------------
// Rows 12–14: add_tree_node
// ---------------------------------------------------------------------------

fn rand_label(r: &mut Rng, len: usize) -> Vec<u8> {
    let mut v: Vec<u8> = (0..len).map(|_| r.plain_byte()).collect();
    v.push(0);
    v
}

/// Random NUL-terminated label of a random length in `0..max`.
fn rand_label_upto(r: &mut Rng, max: usize) -> Vec<u8> {
    let len = r.usize(max);
    rand_label(r, len)
}

#[test]
fn configs_row_12_add_root_label_shapes() {
    let p = Pair::load();
    let mut r = Rng::new(0x1012);
    // 0 / short / 30 / 31 / 40 / 100 byte labels.
    for &len in &[0usize, 1, 5, 30, 31, 32, 40, 100] {
        for i in 0..250 {
            p.reset_both();
            let label = rand_label(&mut r, len);
            let (id, value) = (r.interesting_i32(), r.interesting_i32());
            let ctx = format!("row12 len {len} iter {i}: add_tree_node({id},{value},-1,..)");
            p.eq(
                &ctx,
                p.c.add_tree_node(id, value, -1, &label),
                p.rs.add_tree_node(id, value, -1, &label),
            );
            p.assert_state_eq(&ctx);
            assert_eq!(p.c.node_count(), 1, "{ctx}: node_count");
            // All 52 bytes of slot 0 already compared by assert_state_eq; also
            // check the fields decoded, to catch a same-bytes-wrong-meaning bug.
            let n = p.rs.node(0);
            assert_eq!((n.id, n.value, n.parent_id), (id, value, -1), "{ctx}: fields");
            assert_eq!(n.left_child_id, -1, "{ctx}: left");
            assert_eq!(n.right_child_id, -1, "{ctx}: right");
            assert_eq!(n.label[31], 0, "{ctx}: label[31] must be NUL");
        }
    }
}

/// `strncpy(label, src, 31)` never touches `label[31]`, so the explicit
/// `node->label[31] = '\0'` store is only *observable* when that byte was
/// already non-zero — i.e. over a dirty slot. Reached by poking `node_table`
/// directly, which is exactly what an out-of-band writer of the exported data
/// symbol can do.
#[test]
fn configs_row_12b_label31_forced_nul_over_dirty_slot() {
    let p = Pair::load();
    let mut r = Rng::new(0x1012B);
    for &len in &[0usize, 1, 5, 30, 31, 32, 64] {
        for i in 0..200 {
            p.reset_both();
            // Fill every byte of slot 0 (label[31] included) with non-zero junk.
            let mut dirty = TreeNode::zeroed();
            dirty.id = r.small();
            dirty.value = r.i32();
            dirty.parent_id = r.small();
            dirty.left_child_id = r.small();
            dirty.right_child_id = r.small();
            for k in 0..32 {
                dirty.label[k] = r.plain_byte() as i8;
            }
            assert_ne!(dirty.label[31], 0, "setup: label[31] must start non-zero");
            p.c.set_node(0, dirty);
            p.rs.set_node(0, dirty);
            p.assert_state_eq("row12b dirty slot seeded");

            let label = rand_label(&mut r, len);
            let (id, value) = (r.interesting_i32(), r.interesting_i32());
            let ctx = format!("row12b len {len} iter {i}: add over dirty slot");
            p.eq(
                &ctx,
                p.c.add_tree_node(id, value, -1, &label),
                p.rs.add_tree_node(id, value, -1, &label),
            );
            p.assert_state_eq(&ctx);
            assert_eq!(
                p.c.node(0).label[31],
                0,
                "{ctx}: C must force label[31] to NUL"
            );
        }
    }
}

#[test]
fn configs_row_13_add_children_link_progression() {    let p = Pair::load();
    let mut r = Rng::new(0x1013);
    for i in 0..1500 {
        p.reset_both();
        // -1 means "no parent" to add_tree_node, so it must not be used as an id.
        let root_id = { let v = r.small(); if v == -1 { 5 } else { v } };
        let root_label = rand_label_upto(&mut r, 20);
        let ctx = format!("row13 iter {i}");
        let rv = r.interesting_i32();
        p.eq(
            &format!("{ctx} root"),
            p.c.add_tree_node(root_id, rv, -1, &root_label),
            p.rs.add_tree_node(root_id, rv, -1, &root_label),
        );
        p.assert_state_eq(&format!("{ctx} after root"));

        // child 1 -> left, child 2 -> right, child 3 -> appended unlinked
        for k in 0..3 {
            let cid = root_id.wrapping_add(k + 1);
            let cv = r.interesting_i32();
            let cl = rand_label_upto(&mut r, 35);
            let c = format!("{ctx} child {k}");
            p.eq(
                &c,
                p.c.add_tree_node(cid, cv, root_id, &cl),
                p.rs.add_tree_node(cid, cv, root_id, &cl),
            );
            p.assert_state_eq(&c);
        }
        assert_eq!(p.rs.node_count(), 4, "{ctx}: node_count after 3 children");
        let root = p.rs.node(0);
        // Only the first two children get linked; the third is silently dropped.
        assert_eq!(root.left_child_id, root_id.wrapping_add(1), "{ctx}: left link");
        assert_eq!(root.right_child_id, root_id.wrapping_add(2), "{ctx}: right link");
    }
}

#[test]
fn configs_row_14_fill_table_to_max_nodes() {
    let p = Pair::load();
    let mut r = Rng::new(0x1014);
    for i in 0..60 {
        p.reset_both();
        let base_id = r.small();
        for k in 0..MAX_NODES {
            let id = base_id.wrapping_add(k as c_int);
            let value = r.interesting_i32();
            let label = rand_label_upto(&mut r, 40);
            // Chain parents so links are exercised across the whole table.
            let parent = if k == 0 { -1 } else { base_id.wrapping_add((k - 1) as c_int) };
            let ctx = format!("row14 iter {i} slot {k}: add_tree_node({id},{value},{parent})");
            p.eq(
                &ctx,
                p.c.add_tree_node(id, value, parent, &label),
                p.rs.add_tree_node(id, value, parent, &label),
            );
            p.assert_state_eq(&ctx);
        }
        assert_eq!(p.rs.node_count(), MAX_NODES as c_int, "row14 iter {i}: full");
        // The 51st insert must be rejected identically (also ERRORS row 6).
        let label = rand_label(&mut r, 4);
        let ctx = format!("row14 iter {i}: 51st insert");
        p.eq(&ctx, p.c.add_tree_node(1, 1, -1, &label), p.rs.add_tree_node(1, 1, -1, &label));
        p.assert_state_eq(&ctx);
    }
}

// ---------------------------------------------------------------------------
// Row 15: calculate_tree_sum over every tree shape
// ---------------------------------------------------------------------------

#[test]
fn configs_row_15_tree_sum_shapes() {
    let p = Pair::load();
    let mut r = Rng::new(0x1015);

    for i in 0..1200 {
        // Build a random tree directly in the table so shapes the public
        // constructor cannot produce (right-only child, deep chains) are covered.
        let count = 1 + r.usize(MAX_NODES);
        let mut rows: Vec<TreeNode> = (0..MAX_NODES).map(|k| rand_node(&mut r, k as c_int + 1)).collect();
        let shape = r.usize(6);
        match shape {
            0 => {} // all leaves
            1 => rows[0].left_child_id = 2,
            2 => rows[0].right_child_id = 2,
            3 => {
                rows[0].left_child_id = 2;
                rows[0].right_child_id = 3;
            }
            4 => {
                // depth-3 chain
                rows[0].left_child_id = 2;
                rows[1].left_child_id = 3;
                rows[2].left_child_id = 4;
            }
            _ => {
                // wide binary tree over the whole live range
                for k in 0..count {
                    let l = 2 * k + 1;
                    let rr = 2 * k + 2;
                    rows[k].left_child_id = if l < count { l as c_int + 1 } else { -1 };
                    rows[k].right_child_id = if rr < count { rr as c_int + 1 } else { -1 };
                }
            }
        }
        // Keep links inside the live range and strictly forward-pointing (slot
        // k holds id k+1), so no cycle / unbounded recursion can occur
        // (ERRORS E4). Dangling links are covered by ERRORS row 15.
        for k in 0..MAX_NODES {
            let keep = |link: c_int| -> c_int {
                if link != -1 && link >= 1 && (link as usize) <= count && (link as usize) > k + 1 {
                    link
                } else {
                    -1
                }
            };
            rows[k].left_child_id = keep(rows[k].left_child_id);
            rows[k].right_child_id = keep(rows[k].right_child_id);
        }
        seed_table(&p, &rows, count as c_int);
        for probe in [1, 2, count as c_int, count as c_int + 1, -1, 0, r.i32()] {
            let ctx = format!("row15 iter {i} shape {shape} count {count}: sum({probe})");
            p.eq(&ctx, p.c.calculate_tree_sum(probe), p.rs.calculate_tree_sum(probe));
        }
        p.assert_state_eq("row15");
    }
}

// ---------------------------------------------------------------------------
// Rows 16–18: inreftree
// ---------------------------------------------------------------------------

#[test]
fn configs_row_16_negative_tree_sum_modulus() {
    let p = Pair::load();
    let mut r = Rng::new(0x1016);
    // Residue coverage counters for tree_sum % 4 in -3..=3.
    let mut seen = [0usize; 7];
    for i in 0..ITERS {
        let (p1, p2, p3, p4) = (
            r.interesting_i32(),
            r.interesting_i32(),
            r.interesting_i32(),
            r.interesting_i32(),
        );
        let ctx = format!("row16 iter {i}: inreftree({p1},{p2},{p3},{p4})");
        p.eq(&ctx, p.c.inreftree(p1, p2, p3, p4), p.rs.inreftree(p1, p2, p3, p4));
        p.assert_state_eq(&ctx);
        let sum = p1
            .wrapping_add(p2)
            .wrapping_add(p3)
            .wrapping_add(p4);
        seen[(sum.wrapping_rem(4) + 3) as usize] += 1;
    }
    // Every residue, negative ones included (the out-of-bounds op_string index).
    for (k, &n) in seen.iter().enumerate() {
        assert!(n > 0, "row16: residue {} never exercised (counts {seen:?})", k as i32 - 3);
    }
    // Deterministic sums hitting each residue exactly.
    for target in [-3i32, -2, -1, 0, 1, 2, 3, -7, -6, -5, -4, 4, 5, 6, 7] {
        let (p1, p2, p3, p4) = (target, 1, 1, -2);
        let ctx = format!("row16 fixed sum {target}: inreftree({p1},{p2},{p3},{p4})");
        p.eq(&ctx, p.c.inreftree(p1, p2, p3, p4), p.rs.inreftree(p1, p2, p3, p4));
        p.assert_state_eq(&ctx);
    }
    // Overflow-producing sums.
    for &(p1, p2, p3, p4) in &[
        (i32::MAX, i32::MAX, i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN, i32::MIN, i32::MIN),
        (i32::MIN, -1, -1, -1),
        (i32::MAX, 1, 1, 1),
    ] {
        let ctx = format!("row16 overflow inreftree({p1},{p2},{p3},{p4})");
        p.eq(&ctx, p.c.inreftree(p1, p2, p3, p4), p.rs.inreftree(p1, p2, p3, p4));
        p.assert_state_eq(&ctx);
    }
}

#[test]
fn configs_row_17_inreftree_param2_zero_target_reset() {
    let p = Pair::load();
    let mut r = Rng::new(0x1017);
    for i in 0..ITERS {
        let (p1, p3, p4) = (r.interesting_i32(), r.interesting_i32(), r.interesting_i32());
        let ctx = format!("row17 iter {i}: inreftree({p1},0,{p3},{p4})");
        p.eq(&ctx, p.c.inreftree(p1, 0, p3, p4), p.rs.inreftree(p1, 0, p3, p4));
        p.assert_state_eq(&ctx);
    }
    // param2 == 0 crossed with every residue of tree_sum % 4.
    for target in -6i32..=6 {
        let (p1, p3, p4) = (target, 0, 0);
        let ctx = format!("row17 residue sum {target}");
        p.eq(&ctx, p.c.inreftree(p1, 0, p3, p4), p.rs.inreftree(p1, 0, p3, p4));
        p.assert_state_eq(&ctx);
    }
}

#[test]
fn configs_row_18_inreftree_repeated_calls_stale_state() {
    let p = Pair::load();
    let mut r = Rng::new(0x1018);
    // No reset between calls: node_count is reset to 0 by inreftree itself, but
    // node_table keeps the previous call's bytes.
    for i in 0..ITERS {
        let (p1, p2, p3, p4) = (r.interesting_i32(), r.small(), r.small(), r.interesting_i32());
        let ctx = format!("row18 iter {i}: inreftree({p1},{p2},{p3},{p4})");
        p.eq(&ctx, p.c.inreftree(p1, p2, p3, p4), p.rs.inreftree(p1, p2, p3, p4));
        p.assert_state_eq(&ctx);
    }
    // Interleave a table-filling sequence, then call inreftree again: it must
    // reset node_count and produce the same answer in both libraries.
    for i in 0..200 {
        for k in 0..MAX_NODES {
            let l = rand_label_upto(&mut r, 35);
            let v = r.interesting_i32();
            let par = if k == 0 { -1 } else { 1 };
            p.c.add_tree_node(k as c_int + 1, v, par, &l);
            p.rs.add_tree_node(k as c_int + 1, v, par, &l);
        }
        p.assert_state_eq("row18 after fill");
        let (p1, p2, p3, p4) = (r.small(), r.small(), r.small(), r.small());
        let ctx = format!("row18 post-fill iter {i}: inreftree({p1},{p2},{p3},{p4})");
        p.eq(&ctx, p.c.inreftree(p1, p2, p3, p4), p.rs.inreftree(p1, p2, p3, p4));
        p.assert_state_eq(&ctx);
    }
}

// ---------------------------------------------------------------------------
// Row 19: composed pipeline — low-level calls on inreftree's leftover state
// ---------------------------------------------------------------------------

#[test]
fn configs_row_19_low_level_calls_after_inreftree() {
    let p = Pair::load();
    let mut r = Rng::new(0x1019);
    for i in 0..1500 {
        let (p1, p2, p3, p4) = (
            r.interesting_i32(),
            r.interesting_i32(),
            r.interesting_i32(),
            r.interesting_i32(),
        );
        let ctx = format!("row19 iter {i}: inreftree({p1},{p2},{p3},{p4})");
        p.eq(&ctx, p.c.inreftree(p1, p2, p3, p4), p.rs.inreftree(p1, p2, p3, p4));
        p.assert_state_eq(&ctx);

        for probe in [1, 2, 3, 4, 5, -1, 0, r.i32()] {
            p.eq(
                &format!("{ctx} then find({probe})"),
                p.c.find_node_by_id(probe),
                p.rs.find_node_by_id(probe),
            );
            p.eq(
                &format!("{ctx} then sum({probe})"),
                p.c.calculate_tree_sum(probe),
                p.rs.calculate_tree_sum(probe),
            );
        }
        // Append onto the leftover 4-node tree.
        let l = rand_label_upto(&mut r, 35);
        let v = r.interesting_i32();
        let par = [-1, 1, 2, 3, 4, 99][r.usize(6)];
        let c = format!("{ctx} then add(parent {par})");
        p.eq(&c, p.c.add_tree_node(7, v, par, &l), p.rs.add_tree_node(7, v, par, &l));
        p.assert_state_eq(&c);
        p.eq(
            &format!("{c} then sum(1)"),
            p.c.calculate_tree_sum(1),
            p.rs.calculate_tree_sum(1),
        );
        p.assert_state_eq(&c);
    }
}

// ---------------------------------------------------------------------------
// Rows 20–21: whole-table image parity under a long mixed command sequence
// ---------------------------------------------------------------------------

/// Write a cycle-free "safe pattern" into all 50 rows of both libraries.
///
/// A freshly zeroed row has `id == 0` and `left_child_id == 0` (not `-1`), so if
/// `node_count` is later raised over it, `calculate_tree_sum(0)` would recurse on
/// itself forever in BOTH libraries (ERRORS.md E4). Seeding every row as a
/// uniquely-identified leaf makes any `node_count` in `0..=50` safe to probe.
fn seed_safe_pattern(p: &Pair, r: &mut Rng) {
    for slot in 0..MAX_NODES {
        let mut n = TreeNode::zeroed();
        n.id = 1000 + slot as c_int;
        n.value = r.interesting_i32();
        n.parent_id = -1;
        n.left_child_id = -1;
        n.right_child_id = -1;
        for k in 0..31 {
            n.label[k] = r.plain_byte() as i8;
        }
        n.label[31] = 0;
        p.c.set_node(slot, n);
        p.rs.set_node(slot, n);
    }
}

#[test]
fn configs_row_20_21_mixed_command_sequence() {
    let p = Pair::load();
    let mut r = Rng::new(0x1021);
    p.reset_both();
    seed_safe_pattern(&p, &mut r);
    p.assert_state_eq("row21 initial safe pattern");

    // Ids handed to add_tree_node / row pokes come from this counter, so they
    // never collide with the safe-pattern ids (1000..1049) or with inreftree's
    // ids (1..4). Unique ids keep every parent link pointing strictly forward,
    // which is what rules out cycles in calculate_tree_sum.
    let mut next_id: c_int = 100_000;
    let mut live_ids: Vec<c_int> = Vec::new();

    for step in 0..40_000 {
        let ctx = format!("row21 step {step}");
        match r.next_u64() % 10 {
            0 => {
                let l = rand_label_upto(&mut r, 40);
                let id = next_id;
                next_id += 1;
                let v = r.interesting_i32();
                let par = match r.next_u64() % 4 {
                    0 => -1,
                    1 if !live_ids.is_empty() => live_ids[r.usize(live_ids.len())],
                    2 => 1000 + r.usize(MAX_NODES) as c_int, // an existing safe-pattern id
                    _ => r.small(),                          // usually absent
                };
                let cr = p.c.add_tree_node(id, v, par, &l);
                let rr = p.rs.add_tree_node(id, v, par, &l);
                p.eq(&format!("{ctx} add({id},{v},{par})"), cr, rr);
                if cr >= 0 {
                    live_ids.push(id);
                }
            }
            1 => {
                let id = if !live_ids.is_empty() && r.next_u64() % 2 == 0 {
                    live_ids[r.usize(live_ids.len())]
                } else {
                    r.interesting_i32()
                };
                p.eq(
                    &format!("{ctx} find({id})"),
                    p.c.find_node_by_id(id),
                    p.rs.find_node_by_id(id),
                );
            }
            2 => {
                let id = match r.next_u64() % 5 {
                    0 => 1,
                    1 => 1000 + r.usize(MAX_NODES) as c_int,
                    2 if !live_ids.is_empty() => live_ids[r.usize(live_ids.len())],
                    3 => -1,
                    _ => r.small(),
                };
                p.eq(
                    &format!("{ctx} sum({id})"),
                    p.c.calculate_tree_sum(id),
                    p.rs.calculate_tree_sum(id),
                );
            }
            3 => {
                let s = rand_label_upto(&mut r, 10);
                p.eq(
                    &format!("{ctx} parse"),
                    p.c.parse_operation(&s),
                    p.rs.parse_operation(&s),
                );
            }
            4 => {
                let op = r.interesting_i32();
                let cf = p.c.get_operation_func(op);
                let rf = p.rs.get_operation_func(op);
                let a = r.interesting_i32();
                let b = if r.next_u64() % 8 == 0 { 0 } else { r.small() | 1 };
                p.eq(
                    &format!("{ctx} dispatch({op})({a},{b})"),
                    p.c.call_op(cf, a, b, 0, 0),
                    p.rs.call_op(rf, a, b, 0, 0),
                );
            }
            5 => {
                let (p1, p2, p3, p4) = (r.small(), r.small(), r.small(), r.small());
                p.eq(
                    &format!("{ctx} inreftree({p1},{p2},{p3},{p4})"),
                    p.c.inreftree(p1, p2, p3, p4),
                    p.rs.inreftree(p1, p2, p3, p4),
                );
                // inreftree resets node_count to 4 and rewrites slots 0..3.
                live_ids.retain(|_| false);
            }
            6 => {
                // Poke node_count anywhere in the valid range 0..=MAX_NODES.
                let v = r.usize(MAX_NODES + 1) as c_int;
                p.c.set_node_count(v);
                p.rs.set_node_count(v);
            }
            7 => {
                // Poke a whole row identically in both tables, as a unique leaf.
                let slot = r.usize(MAX_NODES);
                let id = next_id;
                next_id += 1;
                let n = rand_node(&mut r, id);
                p.c.set_node(slot, n);
                p.rs.set_node(slot, n);
            }
            8 => {
                let a = r.interesting_i32();
                let b = r.interesting_i32();
                let bb = if div_trap(a, b) { 3 } else { b };
                p.eq(&format!("{ctx} add_op"), p.c.add_op(a, bb, 0, 0), p.rs.add_op(a, bb, 0, 0));
                p.eq(&format!("{ctx} mul_op"), p.c.multiply_op(a, bb, 0, 0), p.rs.multiply_op(a, bb, 0, 0));
                p.eq(&format!("{ctx} sub_op"), p.c.subtract_op(a, bb, 0, 0), p.rs.subtract_op(a, bb, 0, 0));
                p.eq(&format!("{ctx} div_op"), p.c.divide_op(a, bb, 0, 0), p.rs.divide_op(a, bb, 0, 0));
                p.eq(&format!("{ctx} mod_op"), p.c.modulo_op(a, bb, 0, 0), p.rs.modulo_op(a, bb, 0, 0));
            }
            _ => {
                p.reset_both();
                seed_safe_pattern(&p, &mut r);
                live_ids.clear();
            }
        }
        // Row 20: the full 2600-byte table image plus node_count, every step.
        p.assert_state_eq(&ctx);
    }
}
