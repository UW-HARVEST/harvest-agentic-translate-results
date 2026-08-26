//! Phase B — CONFIGS.md rows C8..C24: the stateful low-level entry points
//! `add_node`, `find_node_by_id`, `get_children_count` and the recursive
//! `calculate_subtree_sum`, driven directly (not through `maxnmin`).

mod common;
use common::*;

/// Interesting `parent_id` / `node_id` probe set used by many rows.
fn probe_ids() -> Vec<i32> {
    vec![
        INT_MIN,
        INT_MIN + 1,
        -1000,
        -7,
        -2,
        -1,
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        99,
        100,
        101,
        1000,
        INT_MAX - 1,
        INT_MAX,
    ]
}

// ----------------------------------------------------------------------- C8

/// C8 — one orphan node: return index, then every field read back at its C
/// offset through the returned `Node*` (the struct-layout contract).
#[test]
fn c8_single_node_fields_and_layout() {
    let p = Pair::new("C8");
    assert_eq!(p.add_node(42, -1, "solo", 3.5), 0, "first index must be 0");
    let v = p.node_view(42).expect("node 42");
    assert_eq!(v.id, 42);
    assert_eq!(v.parent_id, -1);
    assert_eq!(&v.name[..5], b"solo\0");
    assert_eq!(&v.name[5..], &[0u8; MAX_NAME_LEN - 5][..], "tail must be NUL padded");
    assert_eq!(f64::from_bits(v.value_bits), 3.5);
    assert_eq!(v.active, 1, "add_node sets .active = 1");

    // Second slot: verify sizeof(Node) stride agrees between C and Rust.
    assert_eq!(p.add_node(43, 42, "next", -0.0), 1);
    let (cs, rs) = p.observed_stride(42, 43);
    assert_eq!(cs, SIZEOF_NODE as isize);
    assert_eq!(rs, cs);
    // -0.0 must be stored as -0.0 (bit pattern), not 0.0.
    assert_eq!(p.node_view(43).unwrap().value_bits, (-0.0f64).to_bits());

    p.probe_all(&probe_ids());
}

// ----------------------------------------------------------------------- C9

/// C9 — axis C x A: `strncpy(name, MAX_NAME_LEN-1)` truncation and the forced
/// `name[MAX_NAME_LEN-1] = '\0'`, verified over the whole 50-byte buffer.
#[test]
fn c9_name_shapes_stored_verbatim() {
    let p = Pair::new("C9");
    let mut rng = Rng::new(0xC9C9_1111_2222_3333);

    let mut expect_names: Vec<(i32, Vec<u8>)> = Vec::new();
    let mut next_id = 1;
    for len in [0usize, 1, 2, 47, 48, 49, 50, 51, 60, 200] {
        for &(lo, hi) in &[(b'a', b'z'), (0x01, 0x7F), (0x80, 0xFF), (0x01, 0xFF)] {
            let raw = rng.bytes(len, lo, hi);
            let mut arg = raw.clone();
            arg.push(0);
            let id = next_id;
            next_id += 1;
            let idx = p.add_node_raw(id, -1, &arg, len as f64);
            assert_eq!(idx, (id - 1) as i32);

            // Model of strncpy(dst, src, 49) into a zeroed 50-byte buffer,
            // followed by dst[49] = 0.
            let mut model = [0u8; MAX_NAME_LEN];
            let n = raw.len().min(MAX_NAME_LEN - 1);
            model[..n].copy_from_slice(&raw[..n]);
            model[MAX_NAME_LEN - 1] = 0;
            expect_names.push((id, model.to_vec()));
        }
    }
    for (id, model) in &expect_names {
        let v = p.node_view(*id).expect("node");
        assert_eq!(
            &v.name[..],
            &model[..],
            "stored name mismatch for id={id}\n got={:02x?}\nwant={:02x?}",
            &v.name[..],
            &model[..]
        );
    }

    // A name containing an interior NUL: strncpy stops at it and NUL-pads.
    let idx = p.add_node_raw(9001, -1, b"ab\0cdefgh\0", 1.0);
    assert!(idx >= 0);
    let v = p.node_view(9001).unwrap();
    let mut want = [0u8; MAX_NAME_LEN];
    want[0] = b'a';
    want[1] = b'b';
    assert_eq!(&v.name[..], &want[..], "strncpy must stop at the interior NUL");

    // Names fed straight back into process_string (this is what maxnmin does).
    for (id, _) in &expect_names {
        if let Some(v) = p.node_view(*id) {
            let mut buf = v.name.to_vec();
            buf.push(0);
            p.process_string(&buf);
        }
    }
}

// ---------------------------------------------------------------------- C10

/// C10 — axis A: fill storage to exactly MAX_NODES, checking every returned
/// index 0..=99.
#[test]
fn c10_fill_to_max_nodes() {
    let p = Pair::new("C10");
    for i in 0..MAX_NODES {
        let idx = p.add_node(i as i32 + 1, -1, &format!("n{i}"), i as f64);
        assert_eq!(idx, i as i32, "add_node #{i} returned {idx}");
    }
    // Every node is findable and correct.
    for i in 0..MAX_NODES {
        let v = p.node_view(i as i32 + 1).expect("node");
        assert_eq!(v.id, i as i32 + 1);
        assert_eq!(f64::from_bits(v.value_bits), i as f64);
    }
    assert_eq!(p.get_children_count(-1), MAX_NODES as i32);
    p.probe_all(&probe_ids());
}

// ---------------------------------------------------------------------- C11

/// C11 — axis B: duplicate ids. `find_node_by_id` scans forward, so the FIRST
/// matching node wins; every dependent function inherits that.
#[test]
fn c11_duplicate_ids_first_match_wins() {
    let p = Pair::new("C11");
    p.add_node(5, -1, "first", 1.0);
    p.add_node(5, -1, "second", 2.0);
    p.add_node(5, -1, "third", 4.0);
    let v = p.node_view(5).expect("node 5");
    assert_eq!(&v.name[..6], b"first\0");
    assert_eq!(f64::from_bits(v.value_bits), 1.0);
    assert_eq!(p.get_children_count(-1), 3);
    // No node has parent_id == 5, so the sum is just the first node's value.
    assert_eq!(p.calculate_subtree_sum(5), 1.0);

    // Deactivating the first makes the second visible.
    p.set_active(5, 0);
    let v2 = p.node_view(5).expect("node 5 (second)");
    assert_eq!(&v2.name[..7], b"second\0");
    assert_eq!(p.get_children_count(-1), 2);
    assert_eq!(p.calculate_subtree_sum(5), 2.0);
    p.probe_all(&probe_ids());
}

// ---------------------------------------------------------------------- C12

/// C12 — axis B: extreme ids and parent ids stored and looked up verbatim.
#[test]
fn c12_extreme_ids() {
    let p = Pair::new("C12");
    let extremes = [INT_MIN, INT_MIN + 1, -1, 0, 1, INT_MAX - 1, INT_MAX];
    // NOTE: parents must be dangling here. Wiring these ids to each other would
    // create a parent/child cycle, which the C code has no guard against and
    // which therefore recurses until the stack is exhausted (that is exercised
    // deliberately, out of process, as ERRORS.md row E12).
    for (i, &id) in extremes.iter().enumerate() {
        p.add_node(id, -424_242, "x", i as f64);
    }
    for &id in &extremes {
        let v = p.node_view(id).unwrap_or_else(|| panic!("id {id}"));
        assert_eq!(v.id, id);
    }
    // Near misses.
    for &id in &[INT_MIN + 2, -2, 2, INT_MAX - 2, 12345] {
        assert!(p.find_node_by_id(id).is_none(), "id {id} must not be found");
    }
    for &pid in &extremes {
        p.get_children_count(pid);
        p.calculate_subtree_sum(pid);
    }
    assert_eq!(p.get_children_count(-424_242), extremes.len() as i32);
    p.probe_all(&probe_ids());

    // Extreme ids wired into an acyclic chain (INT_MIN -> 0 -> INT_MAX).
    let q = Pair::new("C12b");
    q.add_node(INT_MIN, -1, "a", 1.0);
    q.add_node(0, INT_MIN, "b", 2.0);
    q.add_node(INT_MAX, 0, "c", 4.0);
    assert_eq!(q.calculate_subtree_sum(INT_MIN), 7.0);
    assert_eq!(q.calculate_subtree_sum(0), 6.0);
    assert_eq!(q.calculate_subtree_sum(INT_MAX), 4.0);
    assert_eq!(q.get_children_count(INT_MIN), 1);
    assert_eq!(q.get_children_count(0), 1);
    assert_eq!(q.get_children_count(INT_MAX), 0);
    q.probe_all(&probe_ids());
}

// ---------------------------------------------------------------- C13, C14

/// C13/C14 — axis A = 0: a freshly loaded library has `node_count == 0`, so all
/// lookups miss and all counts are zero.
#[test]
fn c13_c14_empty_storage() {
    let p = Pair::new("C13/C14");
    let mut rng = Rng::new(0x1313_1414_0000_0001);
    for &id in &probe_ids() {
        assert!(p.find_node_by_id(id).is_none());
        assert_eq!(p.get_children_count(id), 0);
        assert_eq!(p.calculate_subtree_sum(id), 0.0);
        // must be *positive* zero, exactly as `return 0.0;`
        assert_eq!(p.calculate_subtree_sum(id).to_bits(), 0u64);
    }
    for _ in 0..64 {
        let id = rng.next_i32();
        assert!(p.find_node_by_id(id).is_none());
        assert_eq!(p.get_children_count(id), 0);
        assert_eq!(p.calculate_subtree_sum(id).to_bits(), 0u64);
    }
}

// ---------------------------------------------------------------------- C15

/// C15 — axis B: flat fan-out with 0 / 1 / 2 / 50 children.
#[test]
fn c15_flat_fanout_children_counts() {
    let p = Pair::new("C15");
    // parent 1 has 0 children, parent 2 has 1, parent 3 has 2, parent 4 has 50
    p.add_node(1, -1, "p1", 1.0);
    p.add_node(2, -1, "p2", 1.0);
    p.add_node(3, -1, "p3", 1.0);
    p.add_node(4, -1, "p4", 1.0);
    let mut next = 100;
    for (parent, n) in [(2, 1), (3, 2), (4, 50)] {
        for _ in 0..n {
            p.add_node(next, parent, "c", 0.5);
            next += 1;
        }
    }
    assert_eq!(p.get_children_count(1), 0);
    assert_eq!(p.get_children_count(2), 1);
    assert_eq!(p.get_children_count(3), 2);
    assert_eq!(p.get_children_count(4), 50);
    assert_eq!(p.get_children_count(-1), 4);
    assert_eq!(p.get_children_count(0), 0);
    for &pid in &probe_ids() {
        p.get_children_count(pid);
    }
    p.probe_all(&probe_ids());
}

// ---------------------------------------------------------------------- C16

/// C16 — axis B: randomized forests, every distinct parent_id queried.
#[test]
fn c16_random_forest_children_counts() {
    let mut rng = Rng::new(0x1616_ABCD_0000_0002);
    for iter in 0..400 {
        let p = Pair::new(&format!("C16#{iter}"));
        let n = rng.below(MAX_NODES as u64 + 1) as usize; // 0..=100
        let mut ids: Vec<i32> = Vec::new();
        for i in 0..n {
            // Unique ids (acyclic-by-construction below relies on this).
            let id = (i as i32) * 3 + 1;
            // parent: an earlier id, or a dangling value
            let parent = if !ids.is_empty() && rng.next_u64() % 4 != 0 {
                ids[rng.below(ids.len() as u64) as usize]
            } else {
                [-1, 0, 999_999, INT_MIN, INT_MAX][rng.below(5) as usize]
            };
            p.add_node(id, parent, "x", rng.next_f64_spread());
            ids.push(id);
        }
        for &pid in &ids {
            p.get_children_count(pid);
        }
        for &pid in &[-1, 0, 999_999, INT_MIN, INT_MAX, 1, 2, 4] {
            p.get_children_count(pid);
        }
    }
}

// ---------------------------------------------------------------------- C17

/// C17 — axis D: a childless node's sum is exactly its `value`, bit-for-bit,
/// for every double class (including NaN, ±inf, -0.0, subnormal).
#[test]
fn c17_leaf_sum_value_classes() {
    for (i, v) in double_classes().into_iter().enumerate() {
        let p = Pair::new(&format!("C17#{i}"));
        p.add_node(7, -1, "leaf", v);
        let got = p.calculate_subtree_sum(7);
        assert_eq!(
            got.to_bits(),
            v.to_bits(),
            "leaf sum must be the stored value verbatim (v={v:?})"
        );
        // and the multiply path used by maxnmin
        p.safe_double_to_int(v);
    }
}

// ---------------------------------------------------------------------- C18

/// C18 — axis B x D: the `+=` accumulation ORDER is observable when the
/// magnitudes differ enough that addition is not associative.
#[test]
fn c18_accumulation_order_is_observable() {
    let p = Pair::new("C18");
    // root 1e16, then eight children of 1.0: (((1e16+1)+1)+... ) loses each 1.0,
    // whereas summing the children first would not. Bit-exact comparison pins
    // the order down.
    p.add_node(1, -1, "root", 1e16);
    for i in 0..8 {
        p.add_node(10 + i, 1, "c", 1.0);
    }
    let got = p.calculate_subtree_sum(1);
    let mut model = 1e16f64;
    for _ in 0..8 {
        model += 1.0;
    }
    assert_eq!(got.to_bits(), model.to_bits(), "got {got:?} want {model:?}");
    assert_eq!(got, 1e16, "the 1.0s must be lost to rounding, proving order");

    // Reversed magnitudes: small root, huge children, mixed signs.
    let q = Pair::new("C18b");
    q.add_node(1, -1, "root", 1.0);
    q.add_node(2, 1, "a", 1e16);
    q.add_node(3, 1, "b", -1e16);
    q.add_node(4, 1, "c", 1.0);
    let g2 = q.calculate_subtree_sum(1);
    let m2 = ((1.0f64 + 1e16) + -1e16) + 1.0;
    assert_eq!(g2.to_bits(), m2.to_bits());
}

// ---------------------------------------------------------------------- C19

/// C19 — axis B: a 99-deep chain, i.e. recursion depth 99.
#[test]
fn c19_deep_chain() {
    let mut rng = Rng::new(0x1919_0000_DEAD_0003);
    for iter in 0..20 {
        let p = Pair::new(&format!("C19#{iter}"));
        let depth = 99;
        let mut vals = Vec::new();
        for i in 0..depth {
            let v = rng.next_f64_spread();
            vals.push(v);
            let parent = if i == 0 { -1 } else { i as i32 };
            p.add_node(i as i32 + 1, parent, "n", v);
        }
        // sum from every level
        for i in 0..depth {
            p.calculate_subtree_sum(i as i32 + 1);
        }
        // the full chain, checked against the same left-to-right model
        let mut model = 0.0f64;
        for v in vals.iter().rev() {
            model = v + model;
        }
        assert_eq!(p.calculate_subtree_sum(1).to_bits(), model.to_bits());
        p.probe_all(&probe_ids());
    }
}

// ---------------------------------------------------------------------- C20

/// C20 — axis B: balanced tree, depth 3, fan-out 3 (1 + 3 + 9 + 27 = 40 nodes).
#[test]
fn c20_balanced_tree() {
    let mut rng = Rng::new(0x2020_0000_BEEF_0004);
    for iter in 0..40 {
        let p = Pair::new(&format!("C20#{iter}"));
        let mut next_id = 1;
        let mut level = vec![{
            let id = next_id;
            next_id += 1;
            p.add_node(id, -1, "root", rng.next_f64_spread());
            id
        }];
        for _ in 0..3 {
            let mut nxt = Vec::new();
            for &parent in &level {
                for _ in 0..3 {
                    let id = next_id;
                    next_id += 1;
                    let v = if rng.next_u64() % 16 == 0 {
                        // occasionally a special value
                        double_classes()[rng.below(double_classes().len() as u64) as usize]
                    } else {
                        rng.next_f64_spread()
                    };
                    p.add_node(id, parent, "c", v);
                    nxt.push(id);
                }
            }
            level = nxt;
        }
        for id in 1..next_id {
            p.calculate_subtree_sum(id);
            p.get_children_count(id);
            p.find_node_by_id(id);
        }
        p.probe_all(&probe_ids());
    }
}

// ---------------------------------------------------------------------- C21

/// C21 — axis B: duplicate ids make a subtree be counted MORE THAN ONCE, because
/// the recursion re-resolves by id and always lands on the first match. Bounded,
/// so no stack exhaustion.
#[test]
fn c21_duplicate_id_double_counting() {
    let p = Pair::new("C21");
    p.add_node(1, -1, "root", 1.0);
    p.add_node(2, 1, "dupA", 2.0);
    p.add_node(2, 1, "dupB", 4.0);
    // sum(1) = 1.0 + sum(2) + sum(2) where sum(2) always resolves to dupA (2.0)
    assert_eq!(p.calculate_subtree_sum(1), 1.0 + 2.0 + 2.0);
    assert_eq!(p.calculate_subtree_sum(2), 2.0);
    assert_eq!(p.get_children_count(1), 2);

    // Three-way duplicate under two different parents.
    let q = Pair::new("C21b");
    q.add_node(1, -1, "r", 1.0);
    q.add_node(2, 1, "a", 10.0);
    q.add_node(3, 1, "b", 100.0);
    q.add_node(4, 2, "x", 1000.0);
    q.add_node(4, 3, "x2", 2000.0);
    // sum(1) = 1 + sum(2) + sum(3); sum(2) = 10 + sum(4)=1000; sum(3) = 100 + sum(4)=1000
    assert_eq!(q.calculate_subtree_sum(1), 1.0 + (10.0 + 1000.0) + (100.0 + 1000.0));

    // A self-parented node that is never reached from itself is fine as long as
    // it is not the query root; query only its (distinct-id) parent chain.
    let r = Pair::new("C21c");
    r.add_node(1, -1, "r", 1.0);
    r.add_node(2, 3, "selfless", 2.0); // parent 3 does not exist
    assert_eq!(r.calculate_subtree_sum(1), 1.0);
    assert_eq!(r.calculate_subtree_sum(2), 2.0);
    r.probe_all(&probe_ids());
}

// ---------------------------------------------------------------------- C22

/// C22 — randomized acyclic forests of up to 100 nodes with random values
/// (including special doubles), summed from every id.
#[test]
fn c22_random_forest_subtree_sums() {
    let mut rng = Rng::new(0x2222_0000_F00D_0005);
    let classes = double_classes();
    for iter in 0..200 {
        let p = Pair::new(&format!("C22#{iter}"));
        let n = rng.below(MAX_NODES as u64 + 1) as usize;
        let mut ids: Vec<i32> = Vec::new();
        for i in 0..n {
            let id = i as i32 * 2 + 1; // unique
            let parent = if !ids.is_empty() && rng.next_u64() % 5 != 0 {
                ids[rng.below(ids.len() as u64) as usize] // strictly earlier => acyclic
            } else {
                [-1, 0, 777, INT_MIN, INT_MAX][rng.below(5) as usize]
            };
            let v = if rng.next_u64() % 12 == 0 {
                classes[rng.below(classes.len() as u64) as usize]
            } else {
                rng.next_f64_spread()
            };
            p.add_node(id, parent, "x", v);
            ids.push(id);
        }
        for &id in &ids {
            p.calculate_subtree_sum(id);
        }
        for &id in &probe_ids() {
            p.calculate_subtree_sum(id);
        }
        // Feed the sums into safe_double_to_int, the way maxnmin does.
        for &id in &ids {
            let s = p.calculate_subtree_sum(id);
            p.safe_double_to_int(s);
        }
    }
}

// ---------------------------------------------------------------------- C23

/// C23 — axis D: inf + (-inf) must produce NaN identically, and NaN must
/// propagate through three levels with the same bit pattern.
#[test]
fn c23_inf_and_nan_propagation() {
    let p = Pair::new("C23");
    p.add_node(1, -1, "root", f64::INFINITY);
    p.add_node(2, 1, "a", f64::NEG_INFINITY);
    let s = p.calculate_subtree_sum(1);
    assert!(s.is_nan(), "inf + -inf must be NaN, got {s:?}");
    p.safe_double_to_int(s);

    let q = Pair::new("C23b");
    q.add_node(1, -1, "root", 1.0);
    q.add_node(2, 1, "a", 2.0);
    q.add_node(3, 2, "b", f64::NAN);
    q.add_node(4, 3, "c", 3.0);
    let s2 = q.calculate_subtree_sum(1);
    assert!(s2.is_nan());
    q.safe_double_to_int(s2);
    assert_eq!(q.safe_double_to_int(s2), 0, "NaN -> 0 per `if (d != d)`");

    let r = Pair::new("C23c");
    r.add_node(1, -1, "root", 1e308);
    r.add_node(2, 1, "a", 1e308);
    r.add_node(3, 1, "b", 1e308);
    let s3 = r.calculate_subtree_sum(1);
    assert_eq!(s3, f64::INFINITY, "overflow to +inf");
    assert_eq!(r.safe_double_to_int(s3), INT_MAX);
}

/// C23b — the NaN-PAYLOAD tie-break. When `sum += child` has NaN on BOTH sides,
/// IEEE 754 leaves the resulting payload to the implementation and x86 resolves
/// it by operand position (`ADDSD` returns SRC1). Every ordering of distinct NaN
/// payloads, signs and signalling/quiet flavours is checked bit-for-bit, since
/// this is exactly the class of difference an optimiser can introduce by
/// commuting a commutative `fadd`.
#[test]
fn c23b_nan_payload_tiebreak_is_bit_exact() {
    let nans: [u64; 10] = [
        0x7FF8_0000_0000_0000, // +qNaN, zero payload (x86 default is the -ve one)
        0xFFF8_0000_0000_0000, // -qNaN, zero payload == x86 "indefinite"
        0x7FF8_0000_0000_00FF,
        0xFFF8_0000_0000_00FF,
        0x7FF8_0000_DEAD_BEEF,
        0xFFF8_0000_CAFE_F00D,
        0x7FFF_FFFF_FFFF_FFFF,
        0xFFFF_FFFF_FFFF_FFFF,
        0x7FF0_0000_0000_0001, // +sNaN
        0xFFF0_0000_0000_0001, // -sNaN
    ];
    // Two children under one root: root value, child A value, child B value.
    for (i, &a) in nans.iter().enumerate() {
        for (j, &b) in nans.iter().enumerate() {
            let p = Pair::new(&format!("C23b[{i},{j}]"));
            // root is a plain number, so the first `+=` mixes number+NaN and the
            // second mixes NaN+NaN.
            p.add_node(1, -1, "root", 1.5);
            p.add_node(2, 1, "a", f64::from_bits(a));
            p.add_node(3, 1, "b", f64::from_bits(b));
            let s = p.calculate_subtree_sum(1);
            assert!(s.is_nan(), "must be NaN");
            p.safe_double_to_int(s);

            // Root itself a NaN: now the very first `+=` is NaN+NaN.
            let q = Pair::new(&format!("C23b-root[{i},{j}]"));
            q.add_node(1, -1, "root", f64::from_bits(a));
            q.add_node(2, 1, "a", f64::from_bits(b));
            assert!(q.calculate_subtree_sum(1).is_nan());

            // NaN mixed with infinities, which can also manufacture a NaN.
            let r = Pair::new(&format!("C23b-inf[{i},{j}]"));
            r.add_node(1, -1, "root", f64::INFINITY);
            r.add_node(2, 1, "a", f64::NEG_INFINITY);
            r.add_node(3, 1, "b", f64::from_bits(a));
            r.add_node(4, 1, "c", f64::from_bits(b));
            assert!(r.calculate_subtree_sum(1).is_nan());
        }
    }

    // Deeper: NaN payloads meeting at several levels of the recursion.
    let mut rng = Rng::new(0x23B0_0000_5AA0_0007);
    for iter in 0..60 {
        let p = Pair::new(&format!("C23b-deep#{iter}"));
        p.add_node(1, -1, "root", f64::from_bits(nans[rng.below(10) as usize]));
        for k in 2..=12i32 {
            let parent = 1 + rng.below((k - 1) as u64) as i32;
            let v = match rng.below(4) {
                0 => f64::from_bits(nans[rng.below(10) as usize]),
                1 => f64::INFINITY,
                2 => f64::NEG_INFINITY,
                _ => rng.next_f64_spread(),
            };
            p.add_node(k, parent, "x", v);
        }
        for id in 1..=12 {
            let s = p.calculate_subtree_sum(id);
            p.safe_double_to_int(s);
        }
    }
}

// ---------------------------------------------------------------------- C24

/// C24 — axis E: `active` written through the returned `Node*` to 1 / 0 / 2 /
/// -1 / INT_MIN / 0x100. The C guards are `&& node.active`, so ANY non-zero int
/// is truthy — an "out-of-range boolean" arriving across the FFI boundary.
#[test]
fn c24_active_flag_arbitrary_ints() {
    for &val in &[1i32, 0, 2, -1, INT_MIN, INT_MAX, 0x100, 0x1_0000] {
        let p = Pair::new(&format!("C24[active={val}]"));
        p.add_node(1, -1, "root", 1.0);
        p.add_node(2, 1, "kid", 2.0);
        p.add_node(3, 1, "kid2", 4.0);

        // set_active writes into both libraries' storage through the pointer
        p.set_active(2, val);

        let found = p.find_node_by_id(2).is_some();
        assert_eq!(found, val != 0, "visibility must follow C truthiness");
        assert_eq!(p.get_children_count(1), if val != 0 { 2 } else { 1 });
        let expect = if val != 0 { 1.0 + 2.0 + 4.0 } else { 1.0 + 4.0 };
        assert_eq!(p.calculate_subtree_sum(1), expect);
        p.probe_all(&probe_ids());

        // Restore, then deactivate the root instead.
        p.poke_i32(1, OFF_ACTIVE, 1);
        p.set_active(1, 0);
        assert!(p.find_node_by_id(1).is_none());
        assert_eq!(p.calculate_subtree_sum(1).to_bits(), 0u64);
        // ...but children still count against parent_id 1
        assert_eq!(p.get_children_count(1), if val != 0 { 2 } else { 1 });
    }
}

/// C24b — randomized `active` toggling interleaved with lookups.
#[test]
fn c24b_random_active_toggling() {
    let mut rng = Rng::new(0x2424_0000_5EED_0006);
    for iter in 0..100 {
        let p = Pair::new(&format!("C24b#{iter}"));
        let n = 1 + rng.below(40) as usize;
        let mut ids = Vec::new();
        for i in 0..n {
            let id = i as i32 + 1;
            let parent = if i == 0 {
                -1
            } else {
                ids[rng.below(ids.len() as u64) as usize]
            };
            p.add_node(id, parent, "x", rng.next_f64_spread());
            ids.push(id);
        }
        for _ in 0..30 {
            let id = ids[rng.below(ids.len() as u64) as usize];
            let v = [0i32, 1, 2, -1, INT_MIN, INT_MAX][rng.below(6) as usize];
            // find_node_by_id may now miss (node deactivated earlier), so poke
            // through a direct scan instead of asserting presence.
            if p.find_node_by_id(id).is_some() {
                p.set_active(id, v);
            }
            for &q in &ids {
                p.find_node_by_id(q);
                p.get_children_count(q);
                p.calculate_subtree_sum(q);
            }
        }
    }
}
