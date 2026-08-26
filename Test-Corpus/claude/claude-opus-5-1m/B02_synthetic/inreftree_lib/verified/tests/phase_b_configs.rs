//! Phase B - valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Both libraries are driven exclusively through their `.so` exports (`dlsym`),
//! in lockstep, and after every mutating call the FULL observable state
//! (`node_count` + the entire 2600-byte `node_table` image) is compared
//! byte-for-byte in addition to the return value.

mod common;
use common::*;
use std::ffi::c_int;

// ===========================================================================
// helpers
// ===========================================================================

/// Drive `add_tree_node` on both libraries and compare return value + state.
#[track_caller]
fn both_add(p: &Pair, id: c_int, value: c_int, parent: c_int, label: &[u8]) -> c_int {
    let cv = p.c.add_node(id, value, parent, label);
    let rv = p.rust.add_node(id, value, parent, label);
    let ctx = format!(
        "add_tree_node({id}, {value}, {parent}, {:?})",
        String::from_utf8_lossy(label)
    );
    assert_ret_eq(cv, rv, &ctx);
    assert_state_eq(p, &ctx);
    cv
}

/// Drive `calculate_tree_sum` on both and compare.
#[track_caller]
fn both_sum(p: &Pair, id: c_int) -> c_int {
    let cv = (p.c.calculate_tree_sum)(id);
    let rv = (p.rust.calculate_tree_sum)(id);
    assert_ret_eq(cv, rv, &format!("calculate_tree_sum({id})"));
    cv
}

/// Drive `find_node_by_id` on both and compare the resolved table INDEX (the
/// raw pointers differ because each library has its own `node_table`).
#[track_caller]
fn both_find(p: &Pair, id: c_int) -> Option<isize> {
    let cv = p.c.find_index(id);
    let rv = p.rust.find_index(id);
    assert_eq!(
        cv, rv,
        "find_node_by_id({id}) index mismatch: C={cv:?} Rust={rv:?}"
    );
    cv
}

#[track_caller]
fn both_parse(p: &Pair, s: &[u8]) -> c_int {
    let cv = p.c.parse_op(s);
    let rv = p.rust.parse_op(s);
    assert_ret_eq(
        cv,
        rv,
        &format!("parse_operation({:?} = {s:02x?})", String::from_utf8_lossy(s)),
    );
    cv
}

#[track_caller]
fn both_inreftree(p: &Pair, a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let cv = (p.c.inreftree)(a, b, c, d);
    let rv = (p.rust.inreftree)(a, b, c, d);
    let ctx = format!("inreftree({a}, {b}, {c}, {d})");
    assert_ret_eq(cv, rv, &ctx);
    assert_state_eq(p, &ctx);
    cv
}

/// Inject an identical `node_table` / `node_count` state into both libraries
/// through their exported data symbols, bypassing `add_tree_node`. This lets the
/// read-only functions be tested against arbitrary states (including ones
/// `add_tree_node` cannot produce).
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
    let mut label = [0u8; 32];
    label[..2].copy_from_slice(b"n\0");
    NodeView {
        id,
        value,
        parent_id: -1,
        left_child_id: l,
        right_child_id: r,
        label,
    }
}

/// The scalar matrix used for the arithmetic entry points.
fn arith_pairs(rng: &mut Rng, n: usize) -> Vec<(i32, i32)> {
    let mut v = Vec::new();
    for &a in EDGE.iter() {
        for &b in EDGE.iter() {
            v.push((a, b));
        }
    }
    for _ in 0..n {
        v.push((rng.spicy_i32(), rng.spicy_i32()));
    }
    v
}

#[track_caller]
fn check_op(name: &str, cf: OpFn, rf: OpFn, skip_div_trap: bool) {
    let mut rng = Rng::new(SEED ^ name.len() as u64);
    let pairs = arith_pairs(&mut rng, 2000);
    let mut n = 0usize;
    for (a, b) in pairs {
        // ERRORS.md U1: INT_MIN / -1 and INT_MIN % -1 raise SIGFPE in the C
        // build (x86 idiv overflow); a trap has no comparable return value.
        if skip_div_trap && a == i32::MIN && b == -1 {
            continue;
        }
        let (u1, u2) = (rng.i32(), rng.i32());
        let cv = cf(a, b, u1, u2);
        let rv = rf(a, b, u1, u2);
        assert_eq!(cv, rv, "{name}({a}, {b}, {u1}, {u2}): C={cv} Rust={rv}");
        // the two trailing parameters must be ignored
        let cv0 = cf(a, b, 0, 0);
        assert_eq!(cv, cv0, "{name}({a},{b}) depends on the unused parameters?!");
        n += 1;
    }
    println!("{name}: {n} input pairs matched");
}

// ===========================================================================
// Rows 1-5: the five arithmetic entry points
// ===========================================================================

#[test]
fn cfg01_add_op() {
    with_libs(|p| check_op("add_op", p.c.add_op, p.rust.add_op, false));
}

#[test]
fn cfg02_multiply_op() {
    with_libs(|p| check_op("multiply_op", p.c.multiply_op, p.rust.multiply_op, false));
}

#[test]
fn cfg03_subtract_op() {
    with_libs(|p| check_op("subtract_op", p.c.subtract_op, p.rust.subtract_op, false));
}

#[test]
fn cfg04_divide_op() {
    with_libs(|p| check_op("divide_op", p.c.divide_op, p.rust.divide_op, true));
}

#[test]
fn cfg05_modulo_op() {
    with_libs(|p| check_op("modulo_op", p.c.modulo_op, p.rust.modulo_op, true));
}

// ===========================================================================
// Rows 6-10: find_node_by_id
// ===========================================================================

#[test]
fn cfg06_find_empty() {
    with_libs(|p| {
        inject(p, &[], 0);
        let mut rng = Rng::new(SEED);
        for &id in EDGE.iter() {
            assert_eq!(both_find(p, id), None, "empty table, id={id}");
        }
        for _ in 0..500 {
            let id = rng.spicy_i32();
            assert_eq!(both_find(p, id), None, "empty table, id={id}");
        }
    });
}

#[test]
fn cfg07_find_single() {
    with_libs(|p| {
        let mut rng = Rng::new(SEED ^ 7);
        for _ in 0..300 {
            let id = rng.spicy_i32();
            inject(p, &[nv(id, rng.i32(), -1, -1)], 1);
            assert_eq!(both_find(p, id), Some(0), "single node id={id}");
            for _ in 0..4 {
                let q = rng.spicy_i32();
                let got = both_find(p, q);
                assert_eq!(got, if q == id { Some(0) } else { None }, "q={q} id={id}");
            }
        }
    });
}

#[test]
fn cfg08_find_first_mid_last() {
    with_libs(|p| {
        // distinct ids 10, 20, ... 10*n : hits loop entry, middle and exit
        for n in 1..=MAX_NODES {
            let nodes: Vec<NodeView> = (0..n).map(|i| nv(10 * (i as i32 + 1), i as i32, -1, -1)).collect();
            inject(p, &nodes, n as c_int);
            assert_eq!(both_find(p, 10), Some(0), "first of {n}");
            assert_eq!(both_find(p, 10 * ((n as i32 + 1) / 2)), Some((n as isize + 1) / 2 - 1), "mid of {n}");
            assert_eq!(both_find(p, 10 * n as i32), Some(n as isize - 1), "last of {n}");
            assert_eq!(both_find(p, 10 * n as i32 + 10), None, "just past last of {n}");
            assert_eq!(both_find(p, 0), None, "absent 0 of {n}");
            assert_eq!(both_find(p, -1), None, "absent -1 of {n}");
        }
    });
}

#[test]
fn cfg09_find_full_table_random() {
    with_libs(|p| {
        let mut rng = Rng::new(SEED ^ 9);
        for _ in 0..200 {
            // ids drawn from a small pool so duplicates, -1 and negatives occur
            let nodes: Vec<NodeView> = (0..MAX_NODES)
                .map(|i| {
                    let id = match rng.below(6) {
                        0 => -1,
                        1 => 0,
                        2 => i32::MIN,
                        3 => i32::MAX,
                        4 => rng.small(),
                        _ => i as i32,
                    };
                    nv(id, rng.i32(), -1, -1)
                })
                .collect();
            inject(p, &nodes, MAX_NODES as c_int);
            for &q in EDGE.iter() {
                both_find(p, q);
            }
            for _ in 0..20 {
                both_find(p, rng.spicy_i32());
            }
        }
    });
}

#[test]
fn cfg10_find_returns_same_index() {
    with_libs(|p| {
        // duplicate ids: the FIRST match must win in both libraries
        let nodes = vec![
            nv(5, 1, -1, -1),
            nv(7, 2, -1, -1),
            nv(5, 3, -1, -1),
            nv(7, 4, -1, -1),
            nv(5, 5, -1, -1),
        ];
        inject(p, &nodes, 5);
        assert_eq!(both_find(p, 5), Some(0));
        assert_eq!(both_find(p, 7), Some(1));
        // shrinking node_count changes which duplicate is reachable
        for count in 0..=5 {
            p.c.set_count(count);
            p.rust.set_count(count);
            let got = both_find(p, 7);
            assert_eq!(got, if count >= 2 { Some(1) } else { None }, "count={count}");
        }
    });
}

// ===========================================================================
// Rows 11-17: add_tree_node
// ===========================================================================

#[test]
fn cfg11_add_root() {
    with_libs(|p| {
        let mut rng = Rng::new(SEED ^ 11);
        for _ in 0..200 {
            p.c.reset();
            p.rust.reset();
            let (id, val) = (rng.spicy_i32(), rng.spicy_i32());
            let r = both_add(p, id, val, -1, b"root");
            assert_eq!(r, 0, "first insert must return index 0");
            assert_eq!(p.c.get_count(), 1);
            let n = p.c.node(0);
            assert_eq!((n.id, n.value, n.parent_id), (id, val, -1));
            assert_eq!((n.left_child_id, n.right_child_id), (-1, -1));
        }
    });
}

#[test]
fn cfg12_add_fills_left_slot() {
    with_libs(|p| {
        both_add(p, 1, 100, -1, b"root");
        assert_eq!(p.c.node(0).left_child_id, -1);
        let r = both_add(p, 2, 200, 1, b"kid");
        assert_eq!(r, 1);
        assert_eq!(p.c.node(0).left_child_id, 2, "left slot must be filled");
        assert_eq!(p.c.node(0).right_child_id, -1, "right slot must stay empty");
        assert_eq!(p.c.node(1).parent_id, 1);
    });
}

#[test]
fn cfg13_add_fills_right_slot() {
    with_libs(|p| {
        both_add(p, 1, 100, -1, b"root");
        both_add(p, 2, 200, 1, b"l");
        let r = both_add(p, 3, 300, 1, b"r");
        assert_eq!(r, 2);
        assert_eq!(p.c.node(0).left_child_id, 2);
        assert_eq!(p.c.node(0).right_child_id, 3, "right slot must be filled");
    });
}

#[test]
fn cfg14_add_third_child_dropped() {
    with_libs(|p| {
        both_add(p, 1, 100, -1, b"root");
        both_add(p, 2, 200, 1, b"l");
        both_add(p, 3, 300, 1, b"r");
        let before = p.c.node(0);
        // third child of a full parent: insert SUCCEEDS but the link is dropped
        let r = both_add(p, 4, 400, 1, b"third");
        assert_eq!(r, 3, "insert still succeeds");
        assert_eq!(p.c.node(0), before, "parent row must be untouched");
        assert_eq!(p.c.node(3).parent_id, 1, "child still records its parent");
        // ... and the dropped child is therefore not part of the sum
        assert_eq!(both_sum(p, 1), 600);
    });
}

#[test]
fn cfg15_add_label_shapes() {
    with_libs(|p| {
        let mut cases: Vec<Vec<u8>> = vec![
            b"".to_vec(),
            b"x".to_vec(),
            b"left".to_vec(),
            b"root".to_vec(),
            b"left-left".to_vec(),
            b"+*-%/".to_vec(),
            vec![b'a'; 30],
            vec![b'a'; 31],
            vec![b'a'; 32],
            vec![b'a'; 33],
            vec![b'a'; 64],
            (0u8..=31).map(|i| i + 1).collect(),          // 31 bytes 0x01..0x1f
            (0u8..64).map(|i| 0x80u8.wrapping_add(i)).collect(), // high bytes
            b"ab\0cd".to_vec(),                            // interior NUL
            vec![0xffu8; 40],
        ];
        let mut rng = Rng::new(SEED ^ 15);
        for _ in 0..300 {
            let len = rng.below(70) as usize;
            cases.push((0..len).map(|_| (rng.below(255) + 1) as u8).collect());
        }
        for label in &cases {
            p.c.reset();
            p.rust.reset();
            // Pre-poison the destination row so that EVERY byte the C writes is
            // observable. Without this, bytes that happen to already be 0 in the
            // zeroed table (notably label[31], which `strncpy(...,31)` never
            // touches - only the explicit `node->label[31] = '\0'` does) would be
            // indistinguishable from a missing write.
            let poison = NodeView {
                id: 0x7f7f7f7f,
                value: 0x7f7f7f7f,
                parent_id: 0x7f7f7f7f,
                left_child_id: 0x7f7f7f7f,
                right_child_id: 0x7f7f7f7f,
                label: [0xAA; 32],
            };
            p.c.set_node(0, &poison);
            p.rust.set_node(0, &poison);
            assert_state_eq(p, "poisoned row");

            both_add(p, 1, 42, -1, label);
            let n = p.c.node(0);
            // C: strncpy(dst, src, 31); dst[31] = '\0'
            assert_eq!(n.label[31], 0, "byte 31 is always overwritten with the terminator");
            assert_eq!((n.id, n.value, n.parent_id), (1, 42, -1), "scalars overwritten");
            assert_eq!((n.left_child_id, n.right_child_id), (-1, -1), "child slots reset");
            let src_eff: Vec<u8> = {
                let upto = label.iter().position(|&b| b == 0).unwrap_or(label.len());
                label[..upto.min(31)].to_vec()
            };
            assert_eq!(&n.label[..src_eff.len()], &src_eff[..], "copied prefix");
            for i in src_eff.len()..31 {
                assert_eq!(n.label[i], 0, "strncpy must zero-pad byte {i}");
            }
        }
        println!("cfg15: {} label shapes matched", cases.len());
    });
}

#[test]
fn cfg16_add_fill_to_capacity() {
    with_libs(|p| {
        // complete binary tree: node i (1-based) has parent i/2
        for i in 1..=MAX_NODES as c_int {
            let parent = if i == 1 { -1 } else { i / 2 };
            let r = both_add(p, i, i * 3, parent, format!("n{i}").as_bytes());
            assert_eq!(r, i - 1, "insert {i} index");
            assert_eq!(p.c.get_count(), i);
        }
        assert_eq!(p.c.get_count(), MAX_NODES as c_int);
        // 51st insert is rejected (ERRORS row 6) and must not change anything
        let img = p.c.table_image();
        let r = both_add(p, 999, 999, -1, b"overflow");
        assert_eq!(r, -1);
        assert_eq!(p.c.get_count(), MAX_NODES as c_int);
        assert_eq!(p.c.table_image(), img, "rejected insert must not mutate");
        // and the whole tree sums identically from every id
        for id in -2..=(MAX_NODES as c_int + 2) {
            both_sum(p, id);
        }
    });
}

#[test]
fn cfg17_add_random_sequences() {
    with_libs(|p| {
        let mut rng = Rng::new(SEED ^ 17);
        for round in 0..30 {
            p.c.reset();
            p.rust.reset();
            // Unique, strictly increasing ids with parent ids drawn from the
            // already-inserted range keep the graph a forest, so
            // calculate_tree_sum always terminates. Absent / -1 / extreme parent
            // ids are mixed in to exercise the rejection paths too.
            let mut next_id: c_int = 1;
            for op in 0..200 {
                let id = next_id;
                let parent = match rng.below(8) {
                    0 => -1,
                    1 => 12345,          // absent -> rejected
                    2 => i32::MIN,       // absent -> rejected
                    3 => 0,              // absent -> rejected
                    _ if next_id > 1 => (rng.below(next_id as u64 - 1) + 1) as c_int,
                    _ => -1,
                };
                let val = rng.spicy_i32();
                let label: Vec<u8> = (0..rng.below(40))
                    .map(|_| (rng.below(255) + 1) as u8)
                    .collect();
                let r = both_add(p, id, val, parent, &label);
                if r >= 0 {
                    next_id += 1;
                }
                // interleave reads against the evolving state
                if op % 7 == 0 {
                    for q in [-1, 0, 1, next_id - 1, next_id, 12345] {
                        both_find(p, q);
                        both_sum(p, q);
                    }
                }
            }
            // final full sweep
            for id in -3..=60 {
                both_sum(p, id);
                both_find(p, id);
            }
            assert_state_eq(p, &format!("round {round} end"));
        }
    });
}

// ===========================================================================
// Rows 18-23: calculate_tree_sum
// ===========================================================================

#[test]
fn cfg18_sum_leaf() {
    with_libs(|p| {
        let mut rng = Rng::new(SEED ^ 18);
        for _ in 0..500 {
            let v = rng.spicy_i32();
            inject(p, &[nv(1, v, -1, -1)], 1);
            assert_eq!(both_sum(p, 1), v, "leaf value {v}");
            assert_eq!(both_sum(p, 2), 0, "absent id");
        }
    });
}

#[test]
fn cfg19_sum_one_and_two_children() {
    with_libs(|p| {
        let mut rng = Rng::new(SEED ^ 19);
        for _ in 0..500 {
            let (a, b, c) = (rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
            // left only
            inject(p, &[nv(1, a, 2, -1), nv(2, b, -1, -1)], 2);
            assert_eq!(both_sum(p, 1), a.wrapping_add(b), "left only");
            // right only
            inject(p, &[nv(1, a, -1, 2), nv(2, b, -1, -1)], 2);
            assert_eq!(both_sum(p, 1), a.wrapping_add(b), "right only");
            // both
            inject(p, &[nv(1, a, 2, 3), nv(2, b, -1, -1), nv(3, c, -1, -1)], 3);
            assert_eq!(
                both_sum(p, 1),
                a.wrapping_add(b).wrapping_add(c),
                "both children"
            );
            assert_eq!(both_sum(p, 2), b);
            assert_eq!(both_sum(p, 3), c);
        }
    });
}

#[test]
fn cfg20_sum_deep_left_chain() {
    with_libs(|p| {
        let mut rng = Rng::new(SEED ^ 20);
        for depth in 1..=MAX_NODES {
            let vals: Vec<i32> = (0..depth).map(|_| rng.spicy_i32()).collect();
            let nodes: Vec<NodeView> = (0..depth)
                .map(|i| {
                    let left = if i + 1 < depth { i as i32 + 2 } else { -1 };
                    nv(i as i32 + 1, vals[i], left, -1)
                })
                .collect();
            inject(p, &nodes, depth as c_int);
            for start in 1..=depth {
                let want = vals[start - 1..].iter().fold(0i32, |a, &b| a.wrapping_add(b));
                assert_eq!(both_sum(p, start as c_int), want, "left chain depth={depth} start={start}");
            }
        }
    });
}

#[test]
fn cfg21_sum_deep_right_chain() {
    with_libs(|p| {
        let mut rng = Rng::new(SEED ^ 21);
        for depth in 1..=MAX_NODES {
            let vals: Vec<i32> = (0..depth).map(|_| rng.spicy_i32()).collect();
            let nodes: Vec<NodeView> = (0..depth)
                .map(|i| {
                    let right = if i + 1 < depth { i as i32 + 2 } else { -1 };
                    nv(i as i32 + 1, vals[i], -1, right)
                })
                .collect();
            inject(p, &nodes, depth as c_int);
            for start in 1..=depth {
                let want = vals[start - 1..].iter().fold(0i32, |a, &b| a.wrapping_add(b));
                assert_eq!(both_sum(p, start as c_int), want, "right chain depth={depth} start={start}");
            }
        }
    });
}

#[test]
fn cfg22_sum_full_tree_every_node() {
    with_libs(|p| {
        let mut rng = Rng::new(SEED ^ 22);
        for _ in 0..100 {
            // complete binary tree over ids 1..=50 built through add_tree_node
            p.c.reset();
            p.rust.reset();
            let mut vals = vec![0i32; MAX_NODES + 1];
            for i in 1..=MAX_NODES as c_int {
                vals[i as usize] = rng.spicy_i32();
                let parent = if i == 1 { -1 } else { i / 2 };
                both_add(p, i, vals[i as usize], parent, format!("n{i}").as_bytes());
            }
            // independent oracle: children of i are 2i and 2i+1 when <= 50
            fn oracle(i: usize, vals: &[i32]) -> i32 {
                let mut s = vals[i];
                if 2 * i <= MAX_NODES {
                    s = s.wrapping_add(oracle(2 * i, vals));
                }
                if 2 * i + 1 <= MAX_NODES {
                    s = s.wrapping_add(oracle(2 * i + 1, vals));
                }
                s
            }
            for id in 1..=MAX_NODES {
                assert_eq!(both_sum(p, id as c_int), oracle(id, &vals), "sum from id {id}");
            }
            for id in [-1, 0, 51, 100, i32::MIN, i32::MAX] {
                assert_eq!(both_sum(p, id), 0, "absent id {id}");
            }
        }
    });
}

#[test]
fn cfg23_sum_random_trees() {
    with_libs(|p| {
        let mut rng = Rng::new(SEED ^ 23);

        // (a) random forests built through add_tree_node (always acyclic)
        for _ in 0..100 {
            p.c.reset();
            p.rust.reset();
            let mut next_id: c_int = 1;
            while next_id <= MAX_NODES as c_int {
                let parent = if next_id == 1 || rng.below(4) == 0 {
                    -1
                } else {
                    (rng.below(next_id as u64 - 1) + 1) as c_int
                };
                if both_add(p, next_id, rng.spicy_i32(), parent, b"x") >= 0 {
                    next_id += 1;
                }
            }
            for id in -2..=52 {
                both_sum(p, id);
            }
        }

        // (b) random DAGs injected directly: child ids are always GREATER than
        //     the parent's id, which guarantees termination while still allowing
        //     diamonds (a child reachable by several paths, counted once per
        //     path - see ERRORS row 30). n is kept small so the path count
        //     cannot explode.
        for _ in 0..400 {
            let n = 1 + rng.below(14) as usize;
            let vals: Vec<i32> = (0..n).map(|_| rng.spicy_i32()).collect();
            let mut kids: Vec<(i32, i32)> = Vec::new();
            let nodes: Vec<NodeView> = (0..n)
                .map(|i| {
                    let pick = |r: &mut Rng| -> i32 {
                        // -1 sentinel, an absent id, or a strictly larger id
                        match r.below(4) {
                            0 => -1,
                            1 => 500 + r.below(10) as i32, // dangling
                            _ if i + 1 < n => (i as i32 + 2) + r.below((n - i - 1) as u64) as i32,
                            _ => -1,
                        }
                    };
                    let l = pick(&mut rng);
                    let r = pick(&mut rng);
                    kids.push((l, r));
                    nv(i as i32 + 1, vals[i], l, r)
                })
                .collect();
            inject(p, &nodes, n as c_int);

            // independent oracle mirroring the C recursion exactly
            fn oracle(id: i32, n: usize, vals: &[i32], kids: &[(i32, i32)]) -> i32 {
                if id < 1 || id as usize > n {
                    return 0;
                }
                let i = id as usize - 1;
                let mut s = vals[i];
                if kids[i].0 != -1 {
                    s = s.wrapping_add(oracle(kids[i].0, n, vals, kids));
                }
                if kids[i].1 != -1 {
                    s = s.wrapping_add(oracle(kids[i].1, n, vals, kids));
                }
                s
            }
            for id in 1..=n as i32 {
                assert_eq!(
                    both_sum(p, id),
                    oracle(id, n, &vals, &kids),
                    "random DAG n={n} id={id} kids={kids:?}"
                );
            }
            for id in [-1, 0, n as i32 + 1, 501, i32::MIN, i32::MAX] {
                assert_eq!(both_sum(p, id), 0, "absent id {id}");
            }
        }
    });
}

// ===========================================================================
// Rows 24-26: parse_operation
// ===========================================================================

#[test]
fn cfg24_parse_single_operators() {
    with_libs(|p| {
        assert_eq!(both_parse(p, b"+"), OP_ADD);
        assert_eq!(both_parse(p, b"*"), OP_MULTIPLY);
        assert_eq!(both_parse(p, b"-"), OP_SUBTRACT);
        assert_eq!(both_parse(p, b"/"), OP_DIVIDE);
        assert_eq!(both_parse(p, b"%"), OP_MODULO);
    });
}

#[test]
fn cfg25_parse_embedded_operator() {
    with_libs(|p| {
        let cases: &[(&[u8], c_int)] = &[
            (b"ab+cd", OP_ADD),
            (b"ab*cd", OP_MULTIPLY),
            (b"ab-cd", OP_SUBTRACT),
            (b"ab/cd", OP_DIVIDE),
            (b"ab%cd", OP_MODULO),
            (b"xx%", OP_MODULO),
            (b"%xx", OP_MODULO),
            (b"zzzzzzzzzzzzzzzz/", OP_DIVIDE),
            (b"left-left", OP_SUBTRACT),
            (b"root", OP_ADD),
            (b"", OP_ADD),
            (b"t", OP_ADD),
            (b"f", OP_ADD),
        ];
        for (s, want) in cases {
            assert_eq!(both_parse(p, s), *want, "parse {:?}", String::from_utf8_lossy(s));
        }
    });
}

#[test]
fn cfg26_parse_random_strings() {
    with_libs(|p| {
        let mut rng = Rng::new(SEED ^ 26);
        // alphabet weighted towards the operators so multi-operator strings
        // (which pin the fixed + > * > - > / > % check order) are common
        let alpha: &[u8] = b"+*-/%ab+*-/%zz\x01\x7f\x80\xff+*-/%";
        for _ in 0..3000 {
            let len = rng.below(17) as usize;
            let s: Vec<u8> = (0..len)
                .map(|_| alpha[rng.below(alpha.len() as u64) as usize])
                .collect();
            let got = both_parse(p, &s);
            // independent oracle: fixed check order, not order of appearance
            let want = if s.contains(&b'+') {
                OP_ADD
            } else if s.contains(&b'*') {
                OP_MULTIPLY
            } else if s.contains(&b'-') {
                OP_SUBTRACT
            } else if s.contains(&b'/') {
                OP_DIVIDE
            } else if s.contains(&b'%') {
                OP_MODULO
            } else {
                OP_ADD
            };
            assert_eq!(got, want, "parse {:?}", String::from_utf8_lossy(&s));
        }
    });
}

// ===========================================================================
// Row 27: get_operation_func (valid variants)
// ===========================================================================

#[test]
fn cfg27_get_func_valid_variants() {
    with_libs(|p| {
        let mut rng = Rng::new(SEED ^ 27);
        for op in [OP_ADD, OP_MULTIPLY, OP_SUBTRACT, OP_DIVIDE, OP_MODULO] {
            let cp = (p.c.get_operation_func)(op) as usize;
            let rp = (p.rust.get_operation_func)(op) as usize;
            assert_ne!(cp, 0, "C get_operation_func({op}) returned NULL");
            assert_ne!(rp, 0, "Rust get_operation_func({op}) returned NULL");
            // each library must return ITS OWN exported *_op symbol
            assert_eq!(cp, p.c.op_addr(op), "C get_operation_func({op}) != &{op}_op");
            assert_eq!(rp, p.rust.op_addr(op), "Rust get_operation_func({op}) != its own op");
            // ... and calling through the returned pointer must agree
            let cf: OpFn = unsafe { std::mem::transmute(cp) };
            let rf: OpFn = unsafe { std::mem::transmute(rp) };
            for _ in 0..500 {
                let (a, mut b) = (rng.spicy_i32(), rng.spicy_i32());
                if (op == OP_DIVIDE || op == OP_MODULO) && a == i32::MIN && b == -1 {
                    b = 1; // ERRORS U1: SIGFPE in C, not comparable
                }
                assert_eq!(
                    cf(a, b, 0, 0),
                    rf(a, b, 0, 0),
                    "func({op})({a},{b}) via get_operation_func"
                );
            }
        }
    });
}

// ===========================================================================
// Rows 28-38: inreftree (the public entry point from lib.h)
// ===========================================================================

/// Independent oracle for `inreftree`, transcribed from `c_src/src/lib.c`.
///
/// The tree is always `1 -> (2 -> (4)), (3)`, so `tree_sum` is the wrapping sum
/// of all four parameters. The label scan always stops at `"left"` (id 2), so
/// `target_id` is 2 unless `param2 == 0`, in which case the `target->value == 0`
/// fallback resets it to 1. `op_string` is `"+*-%"`, so `'/'` / `OP_DIVIDE` is
/// unreachable, and a NEGATIVE `tree_sum % 4` reads BEFORE the literal
/// (ERRORS.md row 26) where `.rodata` holds `'\0'` / `'t'` / `'f'` - none of
/// which is an operator, so `parse_operation` falls back to `OP_ADD`.
fn inreftree_oracle(p1: i32, p2: i32, p3: i32, p4: i32) -> i32 {
    let sum = p1.wrapping_add(p2).wrapping_add(p4).wrapping_add(p3);
    let target_id: i32 = if p2 == 0 { 1 } else { 2 };
    match sum.wrapping_rem(4) {
        0 => sum.wrapping_add(target_id),  // '+' -> OP_ADD
        1 => sum.wrapping_mul(target_id),  // '*' -> OP_MULTIPLY
        2 => sum.wrapping_sub(target_id),  // '-' -> OP_SUBTRACT
        3 => sum.wrapping_rem(target_id),  // '%' -> OP_MODULO
        _ => sum.wrapping_add(target_id),  // negative remainder -> OP_ADD
    }
}

/// Check C == Rust == oracle, and that the resulting table state matches.
#[track_caller]
fn check_inreftree(p: &Pair, p1: i32, p2: i32, p3: i32, p4: i32) -> i32 {
    let got = both_inreftree(p, p1, p2, p3, p4);
    assert_eq!(
        got,
        inreftree_oracle(p1, p2, p3, p4),
        "inreftree({p1},{p2},{p3},{p4}) disagrees with the oracle derived from lib.c"
    );
    // the rebuilt tree must always be exactly these four rows
    assert_eq!(p.c.get_count(), 4);
    let n: Vec<NodeView> = (0..4).map(|i| p.c.node(i)).collect();
    assert_eq!((n[0].id, n[0].value, n[0].left_child_id, n[0].right_child_id), (1, p1, 2, 3));
    assert_eq!((n[1].id, n[1].value, n[1].left_child_id, n[1].right_child_id), (2, p2, 4, -1));
    assert_eq!((n[2].id, n[2].value, n[2].left_child_id, n[2].right_child_id), (3, p3, -1, -1));
    assert_eq!((n[3].id, n[3].value, n[3].left_child_id, n[3].right_child_id), (4, p4, -1, -1));
    assert_eq!(&n[0].label[..5], b"root\0");
    assert_eq!(&n[1].label[..5], b"left\0");
    assert_eq!(&n[2].label[..6], b"right\0");
    assert_eq!(&n[3].label[..10], b"left-left\0");
    got
}

/// Pick parameters whose wrapping sum is exactly `s`, with `param2` zero or not.
fn params_for_sum(rng: &mut Rng, s: i32, p2_zero: bool) -> (i32, i32, i32, i32) {
    let p2 = if p2_zero {
        0
    } else {
        let v = rng.small();
        if v == 0 { 1 } else { v }
    };
    let p3 = rng.small();
    let p4 = rng.small();
    let p1 = s.wrapping_sub(p2).wrapping_sub(p3).wrapping_sub(p4);
    debug_assert_eq!(p1.wrapping_add(p2).wrapping_add(p3).wrapping_add(p4), s);
    (p1, p2, p3, p4)
}

/// Exercise one remainder class, asserting the expected operator was selected.
fn remainder_class(p: &Pair, seed: u64, rem: i32, expect_op: c_int) {
    let mut rng = Rng::new(SEED ^ seed);
    let mut n = 0;
    for _ in 0..800 {
        for p2_zero in [false, true] {
            // magnitude spread across the whole positive/negative range
            let mag = (rng.below(1 << 28) as i32) * 4;
            let s = if rem >= 0 {
                mag.wrapping_add(rem)
            } else {
                mag.wrapping_neg().wrapping_add(rem)
            };
            if s.wrapping_rem(4) != rem {
                continue; // magnitude wrapped; skip
            }
            let (a, b, c, d) = params_for_sum(&mut rng, s, p2_zero);
            let got = check_inreftree(p, a, b, c, d);
            // cross-check that the operator really was `expect_op`
            let target_id: i32 = if b == 0 { 1 } else { 2 };
            let f: OpFn = unsafe { std::mem::transmute((p.c.get_operation_func)(expect_op)) };
            assert_eq!(
                got,
                f(s, target_id, 0, 0),
                "rem {rem}: inreftree({a},{b},{c},{d}) did not use op {expect_op}"
            );
            n += 1;
        }
    }
    assert!(n > 500, "only {n} cases generated for remainder {rem}");
    println!("remainder {rem} (op {expect_op}): {n} cases matched");
}

#[test]
fn cfg28_inreftree_rem0_add() {
    with_libs(|p| remainder_class(p, 28, 0, OP_ADD));
}

#[test]
fn cfg29_inreftree_rem1_multiply() {
    with_libs(|p| remainder_class(p, 29, 1, OP_MULTIPLY));
}

#[test]
fn cfg30_inreftree_rem2_subtract() {
    with_libs(|p| remainder_class(p, 30, 2, OP_SUBTRACT));
}

#[test]
fn cfg31_inreftree_rem3_modulo() {
    with_libs(|p| {
        remainder_class(p, 31, 3, OP_MODULO);
        // '/' is not in op_string, so OP_DIVIDE is unreachable from inreftree;
        // it is still reachable through the low-level API (cfg04 / cfg27).
        assert_eq!(p.c.parse_op(b"+*-%"), OP_ADD);
        for &ch in b"+*-%" {
            assert_ne!(p.c.parse_op(&[ch]), OP_DIVIDE);
            assert_ne!(p.rust.parse_op(&[ch]), OP_DIVIDE);
        }
    });
}

#[test]
fn cfg32_inreftree_negative_remainders() {
    with_libs(|p| {
        remainder_class(p, 320, -1, OP_ADD);
        remainder_class(p, 321, -2, OP_ADD);
        remainder_class(p, 322, -3, OP_ADD);
    });
}

#[test]
fn cfg33_inreftree_param2_zero_x_remainder() {
    with_libs(|p| {
        let mut rng = Rng::new(SEED ^ 33);
        for rem in [-3i32, -2, -1, 0, 1, 2, 3] {
            let mut hits = 0;
            for _ in 0..400 {
                let mag = (rng.below(1 << 26) as i32) * 4;
                let s = if rem >= 0 { mag + rem } else { -mag + rem };
                if s.wrapping_rem(4) != rem {
                    continue;
                }
                // param2 == 0 forces target_id 2 -> 1
                let (a, _, c, d) = params_for_sum(&mut rng, s, true);
                let got = check_inreftree(p, a, 0, c, d);
                let want_op = match rem {
                    0 => OP_ADD,
                    1 => OP_MULTIPLY,
                    2 => OP_SUBTRACT,
                    3 => OP_MODULO,
                    _ => OP_ADD,
                };
                let f: OpFn = unsafe { std::mem::transmute((p.c.get_operation_func)(want_op)) };
                assert_eq!(got, f(s, 1, 0, 0), "param2==0 must give target_id 1 (rem {rem})");
                hits += 1;
            }
            assert!(hits > 200, "rem {rem}: only {hits} cases");
        }
    });
}

#[test]
fn cfg34_inreftree_small_exhaustive() {
    with_libs(|p| {
        let mut n = 0;
        for a in -2..=2 {
            for b in -2..=2 {
                for c in -2..=2 {
                    for d in -2..=2 {
                        check_inreftree(p, a, b, c, d);
                        n += 1;
                    }
                }
            }
        }
        assert_eq!(n, 625);
        println!("cfg34: {n} exhaustive small cases matched");
    });
}

#[test]
fn cfg35_inreftree_extremes_cross_product() {
    with_libs(|p| {
        const V: [i32; 7] = [0, 1, -1, 2, -2, i32::MIN, i32::MAX];
        let mut n = 0;
        for &a in V.iter() {
            for &b in V.iter() {
                for &c in V.iter() {
                    for &d in V.iter() {
                        check_inreftree(p, a, b, c, d);
                        n += 1;
                    }
                }
            }
        }
        assert_eq!(n, 7 * 7 * 7 * 7);
        println!("cfg35: {n} extreme cross-product cases matched");
    });
}

#[test]
fn cfg36_inreftree_random() {
    with_libs(|p| {
        let mut rng = Rng::new(SEED ^ 36);
        for _ in 0..20000 {
            let (a, b, c, d) = (rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
            check_inreftree(p, a, b, c, d);
        }
        println!("cfg36: 20000 random cases matched");
    });
}

#[test]
fn cfg37_inreftree_after_dirty_state() {
    with_libs(|p| {
        let mut rng = Rng::new(SEED ^ 37);
        for _ in 0..100 {
            // fill the table completely with unrelated rows first
            p.c.reset();
            p.rust.reset();
            for i in 1..=MAX_NODES as c_int {
                both_add(p, i * 7, rng.spicy_i32(), if i == 1 { -1 } else { 7 }, b"dirty-l");
            }
            assert_eq!(p.c.get_count(), MAX_NODES as c_int);
            // inreftree must reset node_count and rebuild from scratch
            let (a, b, c, d) = (rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
            check_inreftree(p, a, b, c, d);
            assert_eq!(p.c.get_count(), 4, "node_count must be reset to 4");
            // stale rows 4..50 survive identically in both libraries
            // (assert_state_eq inside both_inreftree already compared all 2600 bytes)
        }
    });
}

#[test]
fn cfg38_inreftree_repeated_calls() {
    with_libs(|p| {
        let mut rng = Rng::new(SEED ^ 38);
        for _ in 0..300 {
            let (a, b, c, d) = (rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
            let r1 = check_inreftree(p, a, b, c, d);
            let r2 = check_inreftree(p, a, b, c, d);
            let r3 = check_inreftree(p, a, b, c, d);
            assert_eq!((r1, r2), (r2, r3), "inreftree must be idempotent");
            // and interleaved with different arguments
            let (e, f, g, h) = (rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
            check_inreftree(p, e, f, g, h);
            assert_eq!(check_inreftree(p, a, b, c, d), r1, "no cross-call state leak");
        }
    });
}

// ===========================================================================
// Row 39: node_table / node_count as ABI data
// ===========================================================================

#[test]
fn cfg39_global_data_abi() {
    with_libs(|p| {
        // freshly reset: both images are all-zero and equal
        assert_eq!(p.c.table_image(), vec![0u8; NODE_TABLE_BYTES]);
        assert_eq!(p.rust.table_image(), vec![0u8; NODE_TABLE_BYTES]);
        assert_eq!(p.c.get_count(), 0);
        assert_eq!(p.rust.get_count(), 0);

        // writing the LAST row through the exported pointer must stay in bounds
        // (proves both objects are >= 2600 bytes and laid out identically)
        let mut rng = Rng::new(SEED ^ 39);
        for idx in 0..MAX_NODES {
            let n = nv(1000 + idx as i32, rng.i32(), -1, -1);
            p.c.set_node(idx, &n);
            p.rust.set_node(idx, &n);
        }
        assert_state_eq(p, "after direct row writes");

        // node_count gates visibility of those rows, identically in both
        for count in 0..=MAX_NODES as c_int {
            p.c.set_count(count);
            p.rust.set_count(count);
            for idx in 0..MAX_NODES as c_int {
                let got = both_find(p, 1000 + idx);
                assert_eq!(got, if idx < count { Some(idx as isize) } else { None });
            }
        }

        // struct layout: id/value/parent/left/right/label at 0,4,8,12,16,20
        p.c.reset();
        p.rust.reset();
        both_add(p, 0x11111111, 0x22222222, -1, b"AB");
        let img = p.c.table_image();
        assert_eq!(&img[0..4], &0x11111111i32.to_ne_bytes());
        assert_eq!(&img[4..8], &0x22222222i32.to_ne_bytes());
        assert_eq!(&img[8..12], &(-1i32).to_ne_bytes());
        assert_eq!(&img[12..16], &(-1i32).to_ne_bytes());
        assert_eq!(&img[16..20], &(-1i32).to_ne_bytes());
        assert_eq!(&img[20..22], b"AB");
        assert_eq!(&img[22..52], &[0u8; 30][..], "rest of label zero-padded");
        // second row starts at exactly offset 52
        both_add(p, 0x33333333, 1, -1, b"C");
        let img = p.c.table_image();
        assert_eq!(&img[52..56], &0x33333333i32.to_ne_bytes());
        assert_eq!(p.rust.table_image(), img);
    });
}

// ===========================================================================
// Row 40: the composed pipeline, driven from the LOW-LEVEL entry points
// ===========================================================================

#[test]
fn cfg40_manual_pipeline_matches_inreftree() {
    with_libs(|p| {
        let mut rng = Rng::new(SEED ^ 40);
        let mut cases: Vec<(i32, i32, i32, i32)> = vec![
            (1, 2, 3, 4),
            (0, 0, 0, 0),
            (5, 0, 7, 9),
            (i32::MAX, 1, 1, 1),
            (i32::MIN, -1, -1, -1),
        ];
        for _ in 0..2000 {
            cases.push((rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32()));
        }

        for (p1, p2, p3, p4) in cases {
            // ---- replay inreftree's body using only the low-level exports ----
            let mut manual = [0i32; 2]; // [C result, Rust result]
            let mut sums = [0i32; 2];
            let mut targets = [0i32; 2];
            let mut ops = [0i32; 2];

            for (which, lib) in [(0usize, &p.c), (1usize, &p.rust)] {
                lib.set_count(0); // `node_count = 0;`
                assert_eq!(lib.add_node(1, p1, -1, b"root"), 0);
                assert_eq!(lib.add_node(2, p2, 1, b"left"), 1);
                assert_eq!(lib.add_node(3, p3, 1, b"right"), 2);
                assert_eq!(lib.add_node(4, p4, 2, b"left-left"), 3);

                // label scan for the first label containing 'l'
                let mut target_id: i32 = -1;
                for i in 0..lib.get_count() {
                    let n = lib.node(i as usize);
                    let end = n.label.iter().position(|&b| b == 0).unwrap_or(32);
                    if n.label[..end].contains(&b'l') {
                        target_id = n.id;
                        break;
                    }
                }
                assert_eq!(target_id, 2, "the label scan always stops at \"left\"");

                match lib.find_index(target_id) {
                    None => target_id = 1,
                    Some(idx) => {
                        if lib.node(idx as usize).value == 0 {
                            target_id = 1;
                        }
                    }
                }

                let tree_sum = (lib.calculate_tree_sum)(1);
                let rem = tree_sum.wrapping_rem(4);
                // op_string == "+*-%"; a negative index reads before the literal
                // (ERRORS row 26) and yields a non-operator byte -> OP_ADD.
                let op_char: &[u8] = match rem {
                    0 => b"+",
                    1 => b"*",
                    2 => b"-",
                    3 => b"%",
                    _ => b"", // the out-of-bounds byte is never an operator
                };
                let op = lib.parse_op(op_char);
                let fp = (lib.get_operation_func)(op);
                assert_ne!(fp as usize, 0);
                assert_eq!(fp as usize, lib.op_addr(op));
                let f: OpFn = unsafe { std::mem::transmute(fp) };

                manual[which] = f(tree_sum, target_id, 0, 0);
                sums[which] = tree_sum;
                targets[which] = target_id;
                ops[which] = op;
            }

            assert_eq!(sums[0], sums[1], "tree_sum mismatch for ({p1},{p2},{p3},{p4})");
            assert_eq!(targets[0], targets[1], "target_id mismatch");
            assert_eq!(ops[0], ops[1], "Operation mismatch");
            assert_eq!(manual[0], manual[1], "composed result mismatch");
            assert_state_eq(p, "manual pipeline");

            // ---- and the one-shot wrapper must agree with the manual replay --
            let one_shot = check_inreftree(p, p1, p2, p3, p4);
            assert_eq!(
                one_shot, manual[0],
                "inreftree({p1},{p2},{p3},{p4}) = {one_shot} but the hand-composed \
                 low-level pipeline gave {}",
                manual[0]
            );
        }
        println!("cfg40: composed pipeline matched inreftree on 2005 cases");
    });
}
