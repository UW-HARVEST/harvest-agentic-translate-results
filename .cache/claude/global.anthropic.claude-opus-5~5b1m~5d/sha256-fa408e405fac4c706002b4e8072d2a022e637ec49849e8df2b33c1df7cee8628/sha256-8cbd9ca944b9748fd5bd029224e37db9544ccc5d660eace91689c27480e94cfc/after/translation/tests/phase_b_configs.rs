//! Phase B — valid-path differential tests, one test per CONFIGS.md row.
//! Both libraries are driven exclusively through their `.so` exports.

mod harness;
use harness::*;
use std::ffi::CString;

// ===========================================================================
// Rows 1-5 — the five leaf operation functions (lowest level entry points)
// ===========================================================================

fn diff_op(row: &str, pick: fn(&Lib) -> Fn4, seed: u64, allow: fn(i32, i32) -> bool) {
    let (p, _g) = fresh();
    let (cf, rf) = (pick(&p.c), pick(&p.r));
    let mut rng = Rng::new(seed);
    let mut n = 0;
    // exhaustive corner grid first
    const CORNERS: [i32; 14] = [
        0, 1, -1, 2, -2, 3, -3, 7, -7, 100, -100, i32::MIN, i32::MIN + 1, i32::MAX,
    ];
    for &a in &CORNERS {
        for &b in &CORNERS {
            if !allow(a, b) {
                continue;
            }
            let (c, r) = unsafe { (cf(a, b, 0, 0), rf(a, b, 0, 0)) };
            assert_eq!(c, r, "{row}: f({a},{b}) C={c} Rust={r}");
            n += 1;
        }
    }
    for _ in 0..20_000 {
        let (a, b) = (rng.spicy_i32(), rng.spicy_i32());
        if !allow(a, b) {
            continue;
        }
        // unused params are randomized too: they must stay unused
        let (u1, u2) = (rng.i32(), rng.i32());
        let (c, r) = unsafe { (cf(a, b, u1, u2), rf(a, b, u1, u2)) };
        assert_eq!(c, r, "{row}: f({a},{b},{u1},{u2}) C={c} Rust={r}");
        n += 1;
    }
    assert!(n > 15_000, "{row}: only {n} cases exercised");
}

fn any(_a: i32, _b: i32) -> bool {
    true
}
/// Skip the two operand pairs that make the C `idiv` trap (covered in Phase C).
fn div_ok(a: i32, b: i32) -> bool {
    !(a == i32::MIN && b == -1)
}

#[test]
fn cfg01_add_op() {
    diff_op("row1 add_op", |l| l.add_op, 0x0101, any);
}

#[test]
fn cfg02_multiply_op() {
    diff_op("row2 multiply_op", |l| l.multiply_op, 0x0202, any);
}

#[test]
fn cfg03_subtract_op() {
    diff_op("row3 subtract_op", |l| l.subtract_op, 0x0303, any);
}

#[test]
fn cfg04_divide_op() {
    diff_op("row4 divide_op", |l| l.divide_op, 0x0404, div_ok);
}

#[test]
fn cfg05_modulo_op() {
    diff_op("row5 modulo_op", |l| l.modulo_op, 0x0505, div_ok);
}

// ===========================================================================
// Row 6 — find_node_by_id over directly-seeded global state
// ===========================================================================

fn rand_node(rng: &mut Rng, ids: &[i32]) -> TreeNode {
    let mut label = [0u8; 32];
    let len = rng.below(33) as usize;
    for i in 0..len {
        label[i] = (rng.below(255) + 1) as u8;
    }
    TreeNode {
        id: ids[rng.below(ids.len() as u64) as usize],
        value: rng.spicy_i32(),
        parent_id: rng.spicy_i32(),
        left_child_id: rng.spicy_i32(),
        right_child_id: rng.spicy_i32(),
        label,
    }
}

#[test]
fn cfg06_find_node_by_id_seeded_state() {
    let (p, _g) = fresh();
    let mut rng = Rng::new(0x0606);
    // ids drawn from a small pool so duplicates and misses both happen a lot
    let ids: Vec<i32> = vec![-3, -2, -1, 0, 1, 2, 3, 4, 5, 17, i32::MIN, i32::MAX];

    for &count in &[0i32, 1, 2, 7, 25, 49, 50] {
        for trial in 0..40 {
            let nodes: Vec<TreeNode> = (0..MAX_NODES).map(|_| rand_node(&mut rng, &ids)).collect();
            p.c.set_table(&nodes);
            p.r.set_table(&nodes);
            p.c.set_count(count);
            p.r.set_count(count);
            assert_state_eq(p, "row6 seed");
            for &id in ids.iter().chain([-999, 999, 6, 7].iter()) {
                let (ci, ri) = (p.c.find_index(id), p.r.find_index(id));
                assert_eq!(
                    ci, ri,
                    "row6: find_node_by_id({id}) count={count} trial={trial}: C={ci:?} Rust={ri:?}"
                );
            }
            // state must be unchanged by a pure lookup
            assert_state_eq(p, "row6 after lookup");
        }
    }
}

// ===========================================================================
// Rows 7-12 — add_tree_node, one row per parent/child-slot configuration
// ===========================================================================

fn add(l: &Lib, id: i32, v: i32, parent: i32, label: &[u8]) -> i32 {
    let c = CString::new(label).unwrap();
    unsafe { (l.add_tree_node)(id, v, parent, c.as_ptr()) }
}

#[track_caller]
fn add_both(p: &Pair, id: i32, v: i32, parent: i32, label: &[u8], ctx: &str) -> i32 {
    let rc = add(&p.c, id, v, parent, label);
    let rr = add(&p.r, id, v, parent, label);
    assert_eq!(rc, rr, "{ctx}: add_tree_node({id},{v},{parent},{label:?}) ret C={rc} Rust={rr}");
    assert_state_eq(p, ctx);
    rc
}

#[test]
fn cfg07_add_root_sentinel_parent() {
    let (p, _g) = fresh();
    let mut rng = Rng::new(0x0707);
    for _ in 0..300 {
        p.c.zero_state();
        p.r.zero_state();
        let (id, v) = (rng.spicy_i32(), rng.spicy_i32());
        let idx = add_both(p, id, v, -1, b"root", "row7");
        assert_eq!(idx, 0, "row7: first insert must be index 0");
        assert_eq!(p.c.count(), 1);
    }
}

#[test]
fn cfg08_add_links_left_child() {
    let (p, _g) = fresh();
    let mut rng = Rng::new(0x0808);
    for _ in 0..300 {
        p.c.zero_state();
        p.r.zero_state();
        add_both(p, 1, rng.spicy_i32(), -1, b"root", "row8/root");
        let cid = rng.spicy_i32();
        add_both(p, cid, rng.spicy_i32(), 1, b"left", "row8/child");
        assert_eq!(p.c.table()[0].left_child_id, cid, "row8: left slot not linked");
        assert_eq!(p.c.table()[0].right_child_id, -1);
    }
}

#[test]
fn cfg09_add_links_right_child() {
    let (p, _g) = fresh();
    let mut rng = Rng::new(0x0909);
    for _ in 0..300 {
        p.c.zero_state();
        p.r.zero_state();
        add_both(p, 1, rng.spicy_i32(), -1, b"root", "row9/root");
        add_both(p, 2, rng.spicy_i32(), 1, b"a", "row9/l");
        let cid = rng.spicy_i32();
        add_both(p, cid, rng.spicy_i32(), 1, b"b", "row9/r");
        assert_eq!(p.c.table()[0].left_child_id, 2);
        assert_eq!(p.c.table()[0].right_child_id, cid, "row9: right slot not linked");
    }
}

#[test]
fn cfg10_add_third_child_link_dropped() {
    let (p, _g) = fresh();
    let mut rng = Rng::new(0x0a0a);
    for _ in 0..200 {
        p.c.zero_state();
        p.r.zero_state();
        add_both(p, 1, rng.spicy_i32(), -1, b"root", "row10/root");
        add_both(p, 2, rng.spicy_i32(), 1, b"a", "row10/l");
        add_both(p, 3, rng.spicy_i32(), 1, b"b", "row10/r");
        // third and fourth children: added but unlinked
        let i3 = add_both(p, 4, rng.spicy_i32(), 1, b"c", "row10/3rd");
        let i4 = add_both(p, 5, rng.spicy_i32(), 1, b"d", "row10/4th");
        assert_eq!((i3, i4), (3, 4));
        assert_eq!(p.c.table()[0].left_child_id, 2);
        assert_eq!(p.c.table()[0].right_child_id, 3);
        assert_eq!(p.c.count(), 5);
    }
}

#[test]
fn cfg11_add_label_shapes() {
    let (p, _g) = fresh();
    let mut rng = Rng::new(0x0b0b);
    // deterministic length sweep 0..=64, then random high-bit payloads
    for len in 0..=64usize {
        p.c.zero_state();
        p.r.zero_state();
        let label: Vec<u8> = (0..len).map(|i| b'A' + (i % 26) as u8).collect();
        add_both(p, 1, 42, -1, &label, &format!("row11/len{len}"));
        // the 32 label bytes must be byte-identical (assert_state_eq covers it,
        // but pin the documented C contract explicitly)
        let l = p.c.table()[0].label;
        assert_eq!(l[31], 0, "row11: label[31] must be forced NUL");
        let kept = len.min(31);
        assert_eq!(&l[..kept], &label[..kept]);
        if len < 31 {
            assert!(l[len..].iter().all(|&b| b == 0), "row11: strncpy must NUL-pad");
        }
    }
    for _ in 0..500 {
        p.c.zero_state();
        p.r.zero_state();
        let len = rng.below(70) as usize;
        // non-zero bytes only (CString forbids interior NUL); full 1..=255 range
        let label: Vec<u8> = (0..len).map(|_| (rng.below(255) + 1) as u8).collect();
        add_both(p, rng.spicy_i32(), rng.spicy_i32(), -1, &label, "row11/rand");
    }
}

#[test]
fn cfg12_add_fills_table_to_capacity() {
    let (p, _g) = fresh();
    let mut rng = Rng::new(0x0c0c);
    add_both(p, 1, rng.spicy_i32(), -1, b"root", "row12/root");
    for i in 2..=MAX_NODES as i32 {
        let idx = add_both(p, i, rng.spicy_i32(), 1, format!("n{i}").as_bytes(), "row12/fill");
        assert_eq!(idx, i - 1);
    }
    assert_eq!(p.c.count(), MAX_NODES as i32);
    // 51st is rejected (ERRORS row 1, re-checked here as the valid boundary)
    let idx = add_both(p, 99, 7, 1, b"overflow", "row12/overflow");
    assert_eq!(idx, -1);
    assert_eq!(p.c.count(), MAX_NODES as i32);
}

#[test]
fn cfg13_add_tree_node_random_op_sequences() {
    let (p, _g) = fresh();
    let mut rng = Rng::new(0x0d0d);
    for seq in 0..60 {
        p.c.zero_state();
        p.r.zero_state();
        for step in 0..200 {
            // ids from a small pool -> duplicates; parents often missing
            let id = (rng.below(9) as i32) - 2;
            let parent = match rng.below(4) {
                0 => -1,
                1 => (rng.below(9) as i32) - 2,
                2 => rng.spicy_i32(),
                _ => (rng.below(3) as i32) + 1,
            };
            let len = rng.below(36) as usize;
            let label: Vec<u8> = (0..len).map(|_| (rng.below(255) + 1) as u8).collect();
            add_both(p, id, rng.spicy_i32(), parent, &label, &format!("row13/seq{seq}/step{step}"));
        }
    }
}

// ===========================================================================
// Rows 14-18 — calculate_tree_sum over increasingly nasty tree shapes
// ===========================================================================

#[track_caller]
fn sum_both(p: &Pair, id: i32, ctx: &str) -> i32 {
    let (c, r) = unsafe { ((p.c.calculate_tree_sum)(id), (p.r.calculate_tree_sum)(id)) };
    assert_eq!(c, r, "{ctx}: calculate_tree_sum({id}) C={c} Rust={r}");
    assert_state_eq(p, ctx);
    c
}

#[test]
fn cfg14_sum_single_root() {
    let (p, _g) = fresh();
    let mut rng = Rng::new(0x0e0e);
    for _ in 0..500 {
        p.c.zero_state();
        p.r.zero_state();
        let v = rng.spicy_i32();
        add_both(p, 1, v, -1, b"root", "row14");
        assert_eq!(sum_both(p, 1, "row14"), v);
    }
}

#[test]
fn cfg15_sum_root_plus_children() {
    let (p, _g) = fresh();
    let mut rng = Rng::new(0x0f0f);
    for shape in 0..3 {
        for _ in 0..300 {
            p.c.zero_state();
            p.r.zero_state();
            let v1 = rng.spicy_i32();
            add_both(p, 1, v1, -1, b"root", "row15");
            let mut expect = v1;
            if shape != 1 {
                let v = rng.spicy_i32();
                add_both(p, 2, v, 1, b"l", "row15");
                expect = expect.wrapping_add(v);
            }
            if shape != 0 {
                let v = rng.spicy_i32();
                add_both(p, 3, v, 1, b"r", "row15");
                expect = expect.wrapping_add(v);
            }
            assert_eq!(sum_both(p, 1, &format!("row15/shape{shape}")), expect);
        }
    }
}

#[test]
fn cfg16_sum_deep_chains() {
    let (p, _g) = fresh();
    let mut rng = Rng::new(0x1010);
    for depth in [3usize, 10, 49] {
        for _ in 0..60 {
            p.c.zero_state();
            p.r.zero_state();
            let mut expect: i32 = 0;
            for i in 1..=depth as i32 {
                let v = rng.spicy_i32();
                expect = expect.wrapping_add(v);
                add_both(p, i, v, if i == 1 { -1 } else { i - 1 }, b"n", "row16");
            }
            assert_eq!(sum_both(p, 1, &format!("row16/depth{depth}")), expect);
            // every intermediate subtree too
            for i in 1..=depth as i32 {
                sum_both(p, i, "row16/sub");
            }
        }
    }
}

#[test]
fn cfg17_sum_dangling_dup_overflow() {
    let (p, _g) = fresh();
    let mut rng = Rng::new(0x1111);
    for trial in 0..400 {
        p.c.zero_state();
        p.r.zero_state();
        // Seed the table directly so dangling child ids and duplicate ids exist.
        // Child ids are constrained to {-1} ∪ already-visited ids to stay acyclic,
        // pointing "backwards" is what would cycle, so point only to higher ids
        // or to non-existent ids.
        let n = 1 + rng.below(12) as usize;
        let mut nodes: Vec<TreeNode> = Vec::new();
        for i in 0..n {
            let mut nd = TreeNode::default();
            nd.id = i as i32; // duplicates injected below
            nd.value = if trial % 3 == 0 {
                // values chosen to overflow int
                if rng.below(2) == 0 { i32::MAX - 3 } else { i32::MIN + 3 }
            } else {
                rng.spicy_i32()
            };
            nd.parent_id = -1;
            // strictly increasing child ids => acyclic; some are dangling
            nd.left_child_id = match rng.below(3) {
                0 => -1,
                1 => (i as i32) + 1 + rng.below(3) as i32,
                _ => 500 + rng.below(50) as i32, // dangling
            };
            nd.right_child_id = match rng.below(3) {
                0 => -1,
                1 => (i as i32) + 1 + rng.below(3) as i32,
                _ => 500 + rng.below(50) as i32, // dangling
            };
            nd.label = [0; 32];
            nodes.push(nd);
        }
        if n > 2 && rng.below(2) == 0 {
            nodes[n - 1].id = nodes[0].id; // duplicate id
        }
        p.c.set_table(&nodes);
        p.r.set_table(&nodes);
        p.c.set_count(n as i32);
        p.r.set_count(n as i32);
        assert_state_eq(p, "row17 seed");
        for id in -2..(n as i32 + 4) {
            sum_both(p, id, &format!("row17/trial{trial}"));
        }
        sum_both(p, 600, "row17/absent");
    }
}

#[test]
fn cfg18_sum_every_id_of_random_tree() {
    let (p, _g) = fresh();
    let mut rng = Rng::new(0x1212);
    for trial in 0..150 {
        p.c.zero_state();
        p.r.zero_state();
        let n = 1 + rng.below(20) as i32;
        for i in 1..=n {
            let parent = if i == 1 { -1 } else { 1 + rng.below(i as u64 - 1) as i32 };
            add_both(p, i, rng.spicy_i32(), parent, format!("n{i}").as_bytes(), "row18");
        }
        for id in -2..(n + 3) {
            sum_both(p, id, &format!("row18/trial{trial}"));
        }
    }
}

// ===========================================================================
// Rows 19-21 — parse_operation
// ===========================================================================

#[track_caller]
fn parse_both(p: &Pair, bytes: &[u8], ctx: &str) -> i32 {
    let s = CString::new(bytes).unwrap();
    let (c, r) = unsafe { ((p.c.parse_operation)(s.as_ptr()), (p.r.parse_operation)(s.as_ptr())) };
    assert_eq!(c, r, "{ctx}: parse_operation({bytes:?}) C={c} Rust={r}");
    c
}

#[test]
fn cfg19_parse_single_operators() {
    let (p, _g) = fresh();
    for (ch, want) in [
        (b'+', OP_ADD),
        (b'*', OP_MULTIPLY),
        (b'-', OP_SUBTRACT),
        (b'/', OP_DIVIDE),
        (b'%', OP_MODULO),
    ] {
        assert_eq!(parse_both(p, &[ch], "row19"), want, "row19: {}", ch as char);
        // also with padding around it
        assert_eq!(parse_both(p, &[b'x', ch, b'y'], "row19/pad"), want);
    }
}

#[test]
fn cfg20_parse_all_permutations_of_operators() {
    let (p, _g) = fresh();
    let ops = [b'+', b'*', b'-', b'/', b'%'];
    // all 120 permutations: result depends on the fixed check order, not position
    let mut idx = [0usize, 1, 2, 3, 4];
    let mut perms = 0;
    permute(&mut idx, 0, &mut |o: &[usize]| {
        let s: Vec<u8> = o.iter().map(|&i| ops[i]).collect();
        let got = parse_both(p, &s, "row20");
        // C order: + then * then - then / then %
        let want = if s.contains(&b'+') {
            OP_ADD
        } else if s.contains(&b'*') {
            OP_MULTIPLY
        } else if s.contains(&b'-') {
            OP_SUBTRACT
        } else if s.contains(&b'/') {
            OP_DIVIDE
        } else {
            OP_MODULO
        };
        assert_eq!(got, want, "row20: {:?}", String::from_utf8_lossy(&s));
        perms += 1;
    });
    assert_eq!(perms, 120);
    // every non-empty subset too
    for mask in 1u32..32 {
        let s: Vec<u8> = (0..5).filter(|i| mask >> i & 1 == 1).map(|i| ops[i]).collect();
        parse_both(p, &s, "row20/subset");
    }
}

fn permute(a: &mut [usize], k: usize, f: &mut impl FnMut(&[usize])) {
    if k == a.len() {
        f(a);
        return;
    }
    for i in k..a.len() {
        a.swap(k, i);
        permute(a, k + 1, f);
        a.swap(k, i);
    }
}

#[test]
fn cfg21_parse_random_strings() {
    let (p, _g) = fresh();
    let mut rng = Rng::new(0x1515);
    let alphabet: Vec<u8> = b"+*-/%abcXYZ019 \t.,;\\\x7f\x80\xfe\xff".to_vec();
    for _ in 0..5000 {
        let len = rng.below(12) as usize;
        let s: Vec<u8> = (0..len)
            .map(|_| alphabet[rng.below(alphabet.len() as u64) as usize])
            .collect();
        parse_both(p, &s, "row21");
    }
    // NUL-terminated early: strchr must stop at the first NUL, so the bytes
    // after it are invisible. Build the buffer by hand (CString forbids this).
    for tail in [b"+", b"*", b"-", b"/", b"%"] {
        let mut buf: Vec<u8> = vec![b'z', 0];
        buf.extend_from_slice(tail);
        buf.push(0);
        let (c, r) = unsafe {
            (
                (p.c.parse_operation)(buf.as_ptr() as *const i8),
                (p.r.parse_operation)(buf.as_ptr() as *const i8),
            )
        };
        assert_eq!(c, r, "row21/embedded-nul {buf:?}");
        assert_eq!(c, OP_ADD, "row21: bytes past the NUL must be ignored");
    }
}

// ===========================================================================
// Rows 22-23 — get_operation_func
// ===========================================================================

#[test]
fn cfg22_get_operation_func_returns_matching_symbol() {
    let (p, _g) = fresh();
    for (op, want) in [
        (OP_ADD, "add_op"),
        (OP_MULTIPLY, "multiply_op"),
        (OP_SUBTRACT, "subtract_op"),
        (OP_DIVIDE, "divide_op"),
        (OP_MODULO, "modulo_op"),
    ] {
        let (c, r) = (p.c.op_func_name(op), p.r.op_func_name(op));
        assert_eq!(c, want, "row22: C get_operation_func({op}) -> {c}");
        assert_eq!(r, want, "row22: Rust get_operation_func({op}) -> {r}");
    }
}

#[test]
fn cfg23_call_through_returned_function_pointer() {
    let (p, _g) = fresh();
    let mut rng = Rng::new(0x1717);
    for op in 1..=5i32 {
        let cf = unsafe { (p.c.get_operation_func)(op) };
        let rf = unsafe { (p.r.get_operation_func)(op) };
        for _ in 0..4000 {
            let (a, b) = (rng.spicy_i32(), rng.spicy_i32());
            if (op == OP_DIVIDE || op == OP_MODULO) && a == i32::MIN && b == -1 {
                continue; // traps in C; Phase C
            }
            let (c, r) = unsafe { (cf(a, b, 0, 0), rf(a, b, 0, 0)) };
            assert_eq!(c, r, "row23: op={op} f({a},{b}) C={c} Rust={r}");
        }
    }
}

// ===========================================================================
// Rows 24-32 — inreftree, the composed pipeline
// ===========================================================================

#[track_caller]
fn inreftree_both(p: &Pair, a: i32, b: i32, c: i32, d: i32, ctx: &str) -> i32 {
    let (rc, rr) = unsafe { ((p.c.inreftree)(a, b, c, d), (p.r.inreftree)(a, b, c, d)) };
    assert_eq!(rc, rr, "{ctx}: inreftree({a},{b},{c},{d}) C={rc} Rust={rr}");
    // the composed pipeline must also leave identical global state behind
    assert_state_eq(p, ctx);
    rc
}

/// Build params whose sum has the requested residue mod 4 (C truncating `%`).
fn params_with_residue(rng: &mut Rng, residue: i32, p2_zero: bool) -> (i32, i32, i32, i32) {
    loop {
        let p1 = rng.spicy_i32();
        let p2 = if p2_zero { 0 } else { rng.spicy_i32() };
        let p3 = rng.spicy_i32();
        if p2_zero && p2 != 0 {
            continue;
        }
        let partial = p1.wrapping_add(p2).wrapping_add(p3);
        // choose p4 so that (partial + p4) % 4 == residue with the right sign
        for _ in 0..64 {
            let p4 = rng.spicy_i32();
            let s = partial.wrapping_add(p4);
            if s % 4 == residue {
                return (p1, p2, p3, p4);
            }
        }
    }
}

fn residue_row(row: &str, residue: i32, p2_zero: bool, seed: u64, want_op: Option<i32>) {
    let (p, _g) = fresh();
    let mut rng = Rng::new(seed);
    for i in 0..400 {
        let (a, b, c, d) = params_with_residue(&mut rng, residue, p2_zero);
        let got = inreftree_both(p, a, b, c, d, &format!("{row}/{i}"));
        // independently recompute what the C must have done
        let sum = a.wrapping_add(b).wrapping_add(c).wrapping_add(d);
        assert_eq!(sum % 4, residue);
        let target_id = if b == 0 { 1 } else { 2 };
        let op = match residue {
            0 => OP_ADD,
            1 => OP_MULTIPLY,
            2 => OP_SUBTRACT,
            3 => OP_MODULO,
            _ => OP_ADD, // negative residue -> OOB read of '\0' / 't' / 'f'
        };
        if let Some(w) = want_op {
            assert_eq!(op, w, "{row}: residue {residue} must select op {w}");
        }
        let expect = match op {
            OP_ADD => sum.wrapping_add(target_id),
            OP_MULTIPLY => sum.wrapping_mul(target_id),
            OP_SUBTRACT => sum.wrapping_sub(target_id),
            OP_MODULO => sum.wrapping_rem(target_id),
            _ => unreachable!(),
        };
        assert_eq!(got, expect, "{row}: model mismatch for ({a},{b},{c},{d}) sum={sum}");
    }
}

#[test]
fn cfg24_inreftree_residue_0_add() {
    residue_row("row24", 0, false, 0x2400, Some(OP_ADD));
}

#[test]
fn cfg25_inreftree_residue_1_multiply() {
    residue_row("row25", 1, false, 0x2500, Some(OP_MULTIPLY));
}

#[test]
fn cfg26_inreftree_residue_2_subtract() {
    residue_row("row26", 2, false, 0x2600, Some(OP_SUBTRACT));
}

#[test]
fn cfg27_inreftree_residue_3_modulo() {
    residue_row("row27", 3, false, 0x2700, Some(OP_MODULO));
}

#[test]
fn cfg28_inreftree_negative_residues_oob_rodata_read() {
    // tree_sum % 4 in {-1,-2,-3} indexes op_string[-1..-3]: an out-of-bounds
    // read of the C .rodata. Those bytes are the tail of "left-left" ('\0','t','f'),
    // none of which is an operator, so parse_operation falls through to OP_ADD.
    for residue in [-1i32, -2, -3] {
        residue_row(
            &format!("row28/res{residue}"),
            residue,
            false,
            0x2800 + residue.unsigned_abs() as u64,
            Some(OP_ADD),
        );
    }
}

#[test]
fn cfg29_inreftree_target_reset_crossed_with_residues() {
    for residue in [0i32, 1, 2, 3, -1, -2, -3] {
        residue_row(
            &format!("row29/res{residue}"),
            residue,
            true, // param2 == 0 -> target->value == 0 -> target_id reset 2 -> 1
            0x2900 + (residue + 8) as u64,
            None,
        );
    }
}

#[test]
fn cfg30_inreftree_zero_and_overflowing_sums() {
    let (p, _g) = fresh();
    let corners = [
        (0, 0, 0, 0),
        (1, -1, 0, 0),
        (i32::MAX, 1, 0, 0),
        (i32::MAX, i32::MAX, 0, 0),
        (i32::MIN, -1, 0, 0),
        (i32::MIN, i32::MIN, i32::MIN, i32::MIN),
        (i32::MAX, i32::MAX, i32::MAX, i32::MAX),
        (i32::MIN, 0, 0, 0),
        (0, i32::MIN, 0, 0),
        (i32::MIN + 1, 0, 0, 0),
        (-1, -1, -1, -1),
        (-1, 0, 0, 0),
        (2, 0, 0, 0),
        (3, 1, 0, 0),
        (0, 1, 0, 0),
        (0, 2, 0, 0),
        (0, 3, 0, 0),
        (0, -1, 0, 0),
        (0, -2, 0, 0),
        (0, -3, 0, 0),
    ];
    for (i, &(a, b, c, d)) in corners.iter().enumerate() {
        inreftree_both(p, a, b, c, d, &format!("row30/{i}"));
    }
    // full 4-way corner cross-product
    let vals = [i32::MIN, i32::MIN + 1, -3, -2, -1, 0, 1, 2, 3, i32::MAX - 1, i32::MAX];
    for &a in &vals {
        for &b in &vals {
            for &c in &vals {
                for &d in &vals {
                    inreftree_both(p, a, b, c, d, "row30/cross");
                }
            }
        }
    }
}

#[test]
fn cfg31_inreftree_randomized_bulk() {
    let (p, _g) = fresh();
    let mut rng = Rng::new(0x3131);
    for i in 0..100_000 {
        let (a, b, c, d) = (rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
        let (rc, rr) = unsafe { ((p.c.inreftree)(a, b, c, d), (p.r.inreftree)(a, b, c, d)) };
        assert_eq!(rc, rr, "row31/{i}: inreftree({a},{b},{c},{d}) C={rc} Rust={rr}");
        if i % 997 == 0 {
            assert_state_eq(p, "row31");
        }
    }
    assert_state_eq(p, "row31/final");
}

#[test]
fn cfg32_inreftree_ignores_stale_state_and_is_repeatable() {
    let (p, _g) = fresh();
    let mut rng = Rng::new(0x3232);
    for i in 0..300 {
        // pre-corrupt both libraries identically
        let nodes: Vec<TreeNode> = (0..MAX_NODES)
            .map(|_| rand_node(&mut rng, &[-1, 0, 1, 2, 3, 4, 5]))
            .collect();
        p.c.set_table(&nodes);
        p.r.set_table(&nodes);
        let bogus = [-7i32, 0, 1, 17, 49, 50, 123][(i % 7) as usize];
        p.c.set_count(bogus);
        p.r.set_count(bogus);
        assert_state_eq(p, "row32 seed");

        let (a, b, c, d) = (rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
        let first = inreftree_both(p, a, b, c, d, &format!("row32/{i}/1st"));
        let second = inreftree_both(p, a, b, c, d, &format!("row32/{i}/2nd"));
        assert_eq!(first, second, "row32: inreftree must be self-resetting");
        assert_eq!(p.c.count(), 4, "row32: inreftree always leaves 4 nodes");
    }
}

// ===========================================================================
// Row 33 — data-symbol ABI
// ===========================================================================

#[test]
fn cfg33_node_table_abi_layout_matches() {
    let (p, _g) = fresh();
    assert_eq!(std::mem::size_of::<TreeNode>(), 52, "harness TreeNode size");

    // Probe the stride the *library* uses: write through node_table[i] via the
    // library's own add_tree_node and check where find_node_by_id lands.
    for lib in [&p.c, &p.r] {
        lib.zero_state();
        for i in 1..=MAX_NODES as i32 {
            let s = CString::new(format!("n{i}")).unwrap();
            unsafe { (lib.add_tree_node)(i, i * 3, -1, s.as_ptr()) };
        }
        for i in 1..=MAX_NODES as i32 {
            assert_eq!(
                lib.find_index(i),
                Some(i as isize - 1),
                "{}: node stride is not 52 bytes at id {i}",
                lib.name
            );
        }
    }
    assert_state_eq(p, "row33 fill");

    // Field offsets: write a distinct pattern through each library and compare
    // the raw bytes of the whole array.
    for lib in [&p.c, &p.r] {
        lib.zero_state();
        lib.set_count(MAX_NODES as i32);
        let nodes: Vec<TreeNode> = (0..MAX_NODES)
            .map(|i| TreeNode {
                id: 0x1100_0000 | i as i32,
                value: 0x2200_0000 | i as i32,
                parent_id: 0x3300_0000 | i as i32,
                left_child_id: 0x4400_0000 | i as i32,
                right_child_id: 0x5500_0000 | i as i32,
                label: [(i as u8) | 0x80; 32],
            })
            .collect();
        lib.set_table(&nodes);
    }
    assert_state_eq(p, "row33 pattern");
    assert_eq!(p.c.table_bytes().len(), 2600);
    // and both agree on where each field lives, as observed through find/sum
    for id in 0x1100_0000..0x1100_0000 + MAX_NODES as i32 {
        assert_eq!(p.c.find_index(id), p.r.find_index(id), "row33 find({id:#x})");
    }
}
