// Phase B -- valid-path differential tests for the LOW-LEVEL entry points.
// Rows C01..C29 and C38 of CONFIGS.md.
//
// Both libraries are driven only through their `.so` exports. Each test gets a
// pristine pair (`node_count == 0`) via `fresh_pair()`.

mod common;

use common::*;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct NodeSpec {
    id: i32,
    parent_id: i32,
    name: Vec<u8>,
    value: f64,
}

/// Build an ACYCLIC forest with unique ids: a node's parent is either `-1`, the
/// id of an earlier-index node, or an id guaranteed absent (orphan).
///
/// Acyclicity matters: `calculate_subtree_sum` is unbounded recursion in C, so a
/// cycle would stack-overflow *both* libraries and prove nothing.
fn random_forest(rng: &mut Rng, n: usize, wild_values: bool) -> Vec<NodeSpec> {
    let mut ids: Vec<i32> = Vec::with_capacity(n);
    while ids.len() < n {
        let cand = rng.range_i32(-40, 400);
        if cand != -1 && !ids.contains(&cand) {
            ids.push(cand);
        }
    }
    // An id that is definitely not stored, for orphan parents.
    let absent = (1..)
        .map(|k| 100_000 + k)
        .find(|k| !ids.contains(k))
        .unwrap();

    (0..n)
        .map(|i| {
            let parent_id = match rng.below(10) {
                0..=1 => -1,
                2 => absent,
                _ if i == 0 => -1,
                _ => ids[rng.below(i as u64) as usize],
            };
            let len = rng.below(55) as usize;
            let name = if rng.below(4) == 0 {
                rng.full_byte_name(len)
            } else {
                rng.ascii_name(len)
            };
            let value = if wild_values {
                rng.mixed_f64()
            } else {
                rng.tame_f64()
            };
            NodeSpec {
                id: ids[i],
                parent_id,
                name,
                value,
            }
        })
        .collect()
}

/// Feed the identical node list into both libraries, asserting every `add_node`
/// return value matches.
fn load_forest(row: &str, p: &Pair, spec: &[NodeSpec]) {
    for (i, s) in spec.iter().enumerate() {
        let rc = p.c.add_node(s.id, s.parent_id, &s.name, s.value);
        let rr = p.r.add_node(s.id, s.parent_id, &s.name, s.value);
        eq_i32(row, ("add_node", i, s.id, s.parent_id, s.name.len()), rc, rr);
    }
}

/// Ids to probe: every stored id, plus a batch of ids known to be absent, plus
/// the generic boundaries.
fn probe_ids(spec: &[NodeSpec]) -> Vec<i32> {
    let mut v: Vec<i32> = spec.iter().map(|s| s.id).collect();
    v.extend([-1, 0, 1, 7, 99, 100_001, i32::MIN, i32::MAX]);
    v
}

// ---------------------------------------------------------------------------
// C01..C03 -- safe_double_to_int (lowest level, stateless)
// ---------------------------------------------------------------------------

#[test]
fn c01_safe_double_to_int_full_bit_space() {
    let p = fresh_pair();
    let mut rng = Rng::new(0xC01_5EED);
    for _ in 0..200_000 {
        let d = rng.any_f64();
        eq_i32("C01", d.to_bits(), p.c.safe_d2i(d), p.r.safe_d2i(d));
    }
    // plus the mixed pool, which over-samples NaN/inf/zero
    for _ in 0..50_000 {
        let d = rng.mixed_f64();
        eq_i32("C01", d.to_bits(), p.c.safe_d2i(d), p.r.safe_d2i(d));
    }
}

#[test]
fn c02_safe_double_to_int_near_int_boundaries() {
    let p = fresh_pair();
    let mut rng = Rng::new(0xC02_5EED);
    for _ in 0..100_000 {
        let d = rng.boundary_f64();
        eq_i32("C02", d, p.c.safe_d2i(d), p.r.safe_d2i(d));
    }
    // exhaustive walk of the exact ulps straddling both clamps
    for base in [i32::MAX as f64, i32::MIN as f64] {
        let mut d = base;
        for _ in 0..64 {
            d = next_down(d);
        }
        for _ in 0..128 {
            eq_i32("C02", d, p.c.safe_d2i(d), p.r.safe_d2i(d));
            d = next_up(d);
        }
    }
}

fn next_up(d: f64) -> f64 {
    if d.is_nan() || d == f64::INFINITY {
        return d;
    }
    if d == 0.0 {
        return f64::from_bits(1);
    }
    if d > 0.0 {
        f64::from_bits(d.to_bits() + 1)
    } else {
        f64::from_bits(d.to_bits() - 1)
    }
}

fn next_down(d: f64) -> f64 {
    if d.is_nan() || d == f64::NEG_INFINITY {
        return d;
    }
    if d == 0.0 {
        return -f64::from_bits(1);
    }
    if d > 0.0 {
        f64::from_bits(d.to_bits() - 1)
    } else {
        f64::from_bits(d.to_bits() + 1)
    }
}

#[test]
fn c03_safe_double_to_int_truncation_toward_zero() {
    let p = fresh_pair();
    let mut rng = Rng::new(0xC03_5EED);
    for _ in 0..100_000 {
        let d = rng.tame_f64();
        eq_i32("C03", d, p.c.safe_d2i(d), p.r.safe_d2i(d));
    }
    // dense sweep of quarter-steps across zero
    let mut k = -2000i64;
    while k <= 2000 {
        let d = k as f64 / 4.0;
        eq_i32("C03", d, p.c.safe_d2i(d), p.r.safe_d2i(d));
        k += 1;
    }
}

// ---------------------------------------------------------------------------
// C04..C07 -- process_string
// ---------------------------------------------------------------------------

#[test]
fn c04_process_string_random_ascii() {
    let p = fresh_pair();
    let mut rng = Rng::new(0xC04_5EED);
    for _ in 0..20_000 {
        let len = rng.below(65) as usize;
        let s = rng.ascii_name(len);
        eq_i32("C04", len, p.c.process_string(&s), p.r.process_string(&s));
    }
}

#[test]
fn c05_process_string_high_bit_bytes() {
    let p = fresh_pair();
    let mut rng = Rng::new(0xC05_5EED);
    for _ in 0..20_000 {
        let len = rng.below(65) as usize;
        let s = rng.full_byte_name(len);
        let (a, b) = (p.c.process_string(&s), p.r.process_string(&s));
        eq_i32("C05", &s, a, b);
    }
    // every single byte value 1..=255 in isolation: pins down char signedness
    for b in 1u8..=255 {
        let s = [b];
        eq_i32("C05", b, p.c.process_string(&s), p.r.process_string(&s));
    }
}

#[test]
fn c06_process_string_accumulator_overflow() {
    let p = fresh_pair();
    let mut rng = Rng::new(0xC06_5EED);
    for _ in 0..300 {
        let len = 1 + rng.below(4096) as usize;
        let s = rng.full_byte_name(len);
        eq_i32("C06", len, p.c.process_string(&s), p.r.process_string(&s));
    }
    // deterministic overflow: 0x7F repeated enough to exceed INT_MAX
    // (0x7F * 16_909_320 == 2_147_483_640 <= INT_MAX; one more byte wraps)
    for len in [16_909_320usize, 16_909_321, 20_000_000] {
        let s = vec![0x7Fu8; len];
        eq_i32("C06", len, p.c.process_string(&s), p.r.process_string(&s));
    }
}

#[test]
fn c07_process_string_on_stored_name_in_place() {
    let p = fresh_pair();
    let mut rng = Rng::new(0xC07_5EED);
    let spec = random_forest(&mut rng, 60, false);
    load_forest("C07", &p, &spec);
    for id in probe_ids(&spec) {
        let a = p.c.process_name_in_place(id);
        let b = p.r.process_name_in_place(id);
        assert_eq!(a.is_some(), b.is_some(), "[C07] NULL-ness differs for id {id}");
        if let (Some(a), Some(b)) = (a, b) {
            eq_i32("C07", id, a, b);
        }
    }
}

// ---------------------------------------------------------------------------
// C08..C13 -- add_node
// ---------------------------------------------------------------------------

#[test]
fn c08_add_node_single_random() {
    let mut rng = Rng::new(0xC08_5EED);
    for _ in 0..2_000 {
        let p = fresh_pair();
        let id = rng.next_i32();
        let parent = rng.next_i32();
        let len = rng.below(60) as usize;
        let name = rng.full_byte_name(len);
        let value = rng.mixed_f64();
        let a = p.c.add_node(id, parent, &name, value);
        let b = p.r.add_node(id, parent, &name, value);
        eq_i32("C08", (id, parent, len), a, b);
        // and the node is readable back identically
        eq_node("C08", id, &p.c.find_snap(id), &p.r.find_snap(id));
    }
}

#[test]
fn c09_add_node_many_sequential_indices() {
    let mut rng = Rng::new(0xC09_5EED);
    for _ in 0..60 {
        let p = fresh_pair();
        let n = 1 + rng.below(99) as usize;
        let spec = random_forest(&mut rng, n, true);
        load_forest("C09", &p, &spec);
        for s in &spec {
            eq_node("C09", s.id, &p.c.find_snap(s.id), &p.r.find_snap(s.id));
        }
    }
}

#[test]
fn c10_add_node_exactly_at_capacity() {
    let p = fresh_pair();
    let mut rng = Rng::new(0xC10_5EED);
    for i in 0..MAX_NODES {
        let nlen = rng.below(50) as usize;
        let name = rng.ascii_name(nlen);
        let v = rng.tame_f64();
        let a = p.c.add_node(i as i32, (i as i32) - 1, &name, v);
        let b = p.r.add_node(i as i32, (i as i32) - 1, &name, v);
        eq_i32("C10", i, a, b);
        assert_eq!(a, i as i32, "[C10] expected index {i}, got {a}");
    }
    // upper boundary reached: last legal add returned 99
}

#[test]
fn c11_add_node_name_length_boundaries() {
    let mut rng = Rng::new(0xC11_5EED);
    for len in [0usize, 1, 2, 47, 48, 49, 50, 51, 52, 64, 120, 500] {
        for _ in 0..80 {
            let p = fresh_pair();
            let name = if rng.below(2) == 0 {
                rng.ascii_name(len)
            } else {
                rng.full_byte_name(len)
            };
            let a = p.c.add_node(7, -1, &name, 1.5);
            let b = p.r.add_node(7, -1, &name, 1.5);
            eq_i32("C11", len, a, b);
            eq_node("C11", len, &p.c.find_snap(7), &p.r.find_snap(7));
            // the stored name must also sum identically through process_string
            let (x, y) = (
                p.c.process_name_in_place(7).unwrap(),
                p.r.process_name_in_place(7).unwrap(),
            );
            eq_i32("C11", ("process", len), x, y);
        }
    }
}

#[test]
fn c12_add_node_name_with_interior_nul_and_high_bits() {
    let mut rng = Rng::new(0xC12_5EED);
    for _ in 0..2_000 {
        let p = fresh_pair();
        let len = rng.below(80) as usize;
        let mut name = rng.full_byte_name(len);
        if !name.is_empty() {
            // plant a NUL somewhere: strncpy must stop there
            let at = rng.below(name.len() as u64) as usize;
            name[at] = 0;
        }
        let a = p.c.add_node(3, -1, &name, -0.0);
        let b = p.r.add_node(3, -1, &name, -0.0);
        eq_i32("C12", len, a, b);
        eq_node("C12", &name, &p.c.find_snap(3), &p.r.find_snap(3));
    }
}

#[test]
fn c13_add_node_special_double_values() {
    let specials: Vec<f64> = vec![
        0.0,
        -0.0,
        f64::NAN,
        -f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MIN_POSITIVE,
        f64::MIN_POSITIVE / 2.0, // subnormal
        f64::MAX,
        f64::MIN,
        1e300,
        -1e300,
        i32::MAX as f64,
        i32::MIN as f64,
        2147483648.0,
        -2147483649.0,
        f64::from_bits(0x7FF8_0000_0000_0001), // NaN with payload
        f64::from_bits(0xFFF0_0000_0000_0001),
    ];
    let mut rng = Rng::new(0xC13_5EED);
    for (i, &v) in specials.iter().enumerate() {
        let p = fresh_pair();
        let name = rng.ascii_name(10);
        let a = p.c.add_node(1, -1, &name, v);
        let b = p.r.add_node(1, -1, &name, v);
        eq_i32("C13", i, a, b);
        // stored bit pattern (incl. NaN payload) must be identical
        eq_node("C13", (i, v.to_bits()), &p.c.find_snap(1), &p.r.find_snap(1));
        // and it must flow through subtree_sum identically
        eq_bits("C13", (i, v), p.c.subtree_bits(1), p.r.subtree_bits(1));
        eq_i32("C13", (i, v), p.c.safe_d2i(v), p.r.safe_d2i(v));
    }
}

// ---------------------------------------------------------------------------
// C14..C17 -- find_node_by_id
// ---------------------------------------------------------------------------

#[test]
fn c14_find_node_random_storage() {
    let mut rng = Rng::new(0xC14_5EED);
    for _ in 0..120 {
        let p = fresh_pair();
        let n = 1 + rng.below(60) as usize;
        let spec = random_forest(&mut rng, n, true);
        load_forest("C14", &p, &spec);

        // relative index of the returned pointer must agree across libraries
        let cbase = p.c.find_raw(spec[0].id);
        let rbase = p.r.find_raw(spec[0].id);
        assert!(!cbase.is_null() && !rbase.is_null(), "[C14] base lookup failed");

        for id in probe_ids(&spec) {
            let cp = p.c.find_raw(id);
            let rp = p.r.find_raw(id);
            assert_eq!(
                cp.is_null(),
                rp.is_null(),
                "[C14] NULL-ness differs for id {id}"
            );
            if !cp.is_null() {
                let coff = (cp as isize) - (cbase as isize);
                let roff = (rp as isize) - (rbase as isize);
                assert_eq!(
                    coff, roff,
                    "[C14] returned a different storage slot for id {id}: \
                     C offset {coff}, Rust offset {roff}"
                );
                eq_node("C14", id, &p.c.find_snap(id), &p.r.find_snap(id));
            }
        }
    }
}

#[test]
fn c15_find_node_duplicate_ids_first_match_wins() {
    let mut rng = Rng::new(0xC15_5EED);
    for _ in 0..500 {
        let p = fresh_pair();
        let dup_id = rng.range_i32(-5, 5);
        let n = 2 + rng.below(20) as usize;
        // Every node gets the SAME id, distinct names/values: the returned node
        // must be the lowest-index one on both sides.
        let mut names = Vec::new();
        for i in 0..n {
            let name = rng.ascii_name(4 + (i % 20));
            let v = rng.tame_f64();
            let a = p.c.add_node(dup_id, -1, &name, v);
            let b = p.r.add_node(dup_id, -1, &name, v);
            eq_i32("C15", i, a, b);
            names.push(name);
        }
        eq_node("C15", dup_id, &p.c.find_snap(dup_id), &p.r.find_snap(dup_id));
        let got = p.c.find_snap(dup_id).unwrap();
        let want: Vec<u8> = names[0].clone();
        assert_eq!(
            &got.name[..want.len()],
            &want[..],
            "[C15] expected first-match-wins"
        );
    }
}

#[test]
fn c16_find_node_after_deactivation_in_place() {
    let mut rng = Rng::new(0xC16_5EED);
    for _ in 0..150 {
        let p = fresh_pair();
        let n = 2 + rng.below(40) as usize;
        let spec = random_forest(&mut rng, n, false);
        load_forest("C16", &p, &spec);

        // deactivate a random subset through the returned Node*
        for s in &spec {
            if rng.below(3) == 0 {
                let a = p.c.set_active(s.id, 0);
                let b = p.r.set_active(s.id, 0);
                assert_eq!(a, b, "[C16] set_active reachability differs for {}", s.id);
            }
        }
        for id in probe_ids(&spec) {
            eq_node("C16", id, &p.c.find_snap(id), &p.r.find_snap(id));
            eq_i32("C16", ("children", id), p.c.children(id), p.r.children(id));
            eq_bits("C16", ("sum", id), p.c.subtree_bits(id), p.r.subtree_bits(id));
        }
        // reactivate and re-check
        for s in &spec {
            p.c.set_active(s.id, 1);
            p.r.set_active(s.id, 1);
        }
        for id in probe_ids(&spec) {
            eq_node("C16", ("reactivated", id), &p.c.find_snap(id), &p.r.find_snap(id));
        }
    }
}

#[test]
fn c17_find_node_post_maxnmin_state() {
    let mut rng = Rng::new(0xC17_5EED);
    for _ in 0..300 {
        let p = fresh_pair();
        let (a, b, c, d) = (
            rng.next_i32(),
            rng.next_i32(),
            rng.range_i32(-8, 8),
            rng.next_i32(),
        );
        eq_i32("C17", (a, b, c, d), p.c.maxnmin(a, b, c, d), p.r.maxnmin(a, b, c, d));
        // `maxnmin` leaves node_count == 6 with ids 1..6 -- inspect that state
        for id in [-1, 0, 1, 2, 3, 4, 5, 6, 7, i32::MIN, i32::MAX] {
            eq_node("C17", id, &p.c.find_snap(id), &p.r.find_snap(id));
            eq_i32("C17", ("children", id), p.c.children(id), p.r.children(id));
            eq_bits("C17", ("sum", id), p.c.subtree_bits(id), p.r.subtree_bits(id));
        }
    }
}

// ---------------------------------------------------------------------------
// C18..C21 -- get_children_count
// ---------------------------------------------------------------------------

#[test]
fn c18_children_count_random_storage() {
    let mut rng = Rng::new(0xC18_5EED);
    for _ in 0..200 {
        let p = fresh_pair();
        let n = 1 + rng.below(70) as usize;
        let spec = random_forest(&mut rng, n, true);
        load_forest("C18", &p, &spec);
        let mut probes = probe_ids(&spec);
        probes.extend(spec.iter().map(|s| s.parent_id));
        for id in probes {
            eq_i32("C18", id, p.c.children(id), p.r.children(id));
        }
    }
}

#[test]
fn c19_children_count_fanout_shapes() {
    let mut rng = Rng::new(0xC19_5EED);
    for fanout in [0usize, 1, 2, 3, 7, 20, 50, 90] {
        let p = fresh_pair();
        let vals: Vec<f64> = (0..=fanout).map(|_| rng.tame_f64()).collect();
        p.c.add_node(1, -1, b"root", vals[0]);
        p.r.add_node(1, -1, b"root", vals[0]);
        for k in 0..fanout {
            let name = rng.ascii_name(5);
            let a = p.c.add_node(100 + k as i32, 1, &name, vals[k + 1]);
            let b = p.r.add_node(100 + k as i32, 1, &name, vals[k + 1]);
            eq_i32("C19", (fanout, k), a, b);
        }
        eq_i32("C19", fanout, p.c.children(1), p.r.children(1));
        eq_i32("C19", (fanout, "root"), p.c.children(-1), p.r.children(-1));
        eq_bits("C19", fanout, p.c.subtree_bits(1), p.r.subtree_bits(1));
    }
}

#[test]
fn c20_children_count_with_inactive_children() {
    let mut rng = Rng::new(0xC20_5EED);
    for _ in 0..200 {
        let p = fresh_pair();
        let n = 2 + rng.below(30) as usize;
        let spec = random_forest(&mut rng, n, false);
        load_forest("C20", &p, &spec);
        // deactivate ALL children of one chosen parent
        let target = spec[rng.below(n as u64) as usize].parent_id;
        for s in &spec {
            if s.parent_id == target {
                p.c.set_active(s.id, 0);
                p.r.set_active(s.id, 0);
            }
        }
        eq_i32("C20", target, p.c.children(target), p.r.children(target));
        for id in probe_ids(&spec) {
            eq_i32("C20", id, p.c.children(id), p.r.children(id));
        }
    }
}

#[test]
fn c21_children_count_at_storage_limit() {
    let mut rng = Rng::new(0xC21_5EED);
    for _ in 0..25 {
        let p = fresh_pair();
        let spec = random_forest(&mut rng, MAX_NODES, false);
        load_forest("C21", &p, &spec);
        let mut probes = probe_ids(&spec);
        probes.extend(spec.iter().map(|s| s.parent_id));
        for id in probes {
            eq_i32("C21", id, p.c.children(id), p.r.children(id));
            eq_bits("C21", id, p.c.subtree_bits(id), p.r.subtree_bits(id));
        }
    }
}

// ---------------------------------------------------------------------------
// C22..C29 -- calculate_subtree_sum
// ---------------------------------------------------------------------------

#[test]
fn c22_subtree_sum_single_leaf() {
    let mut rng = Rng::new(0xC22_5EED);
    for _ in 0..3_000 {
        let p = fresh_pair();
        let v = rng.mixed_f64();
        let id = rng.next_i32();
        p.c.add_node(id, -1, b"leaf", v);
        p.r.add_node(id, -1, b"leaf", v);
        eq_bits("C22", (id, v.to_bits()), p.c.subtree_bits(id), p.r.subtree_bits(id));
    }
}

#[test]
fn c23_subtree_sum_order_sensitive_fanout() {
    // Values are chosen so that a different summation order changes the result in
    // the low mantissa bits -- this makes the C traversal order observable.
    let mut rng = Rng::new(0xC23_5EED);
    for _ in 0..400 {
        let p = fresh_pair();
        let n = 2 + rng.below(40) as usize;
        p.c.add_node(1, -1, b"root", 1.0);
        p.r.add_node(1, -1, b"root", 1.0);
        for k in 0..n {
            // magnitudes straddling the ulp of 1.0 (2^-52)
            let scale = match rng.below(4) {
                0 => f64::EPSILON / 2.0,
                1 => f64::EPSILON,
                2 => f64::EPSILON * 4.0,
                _ => 1.0,
            };
            let v = scale * (1 + rng.below(9)) as f64 * if rng.below(2) == 0 { 1.0 } else { -1.0 };
            p.c.add_node(200 + k as i32, 1, b"c", v);
            p.r.add_node(200 + k as i32, 1, b"c", v);
        }
        eq_bits("C23", n, p.c.subtree_bits(1), p.r.subtree_bits(1));
    }
}

#[test]
fn c24_subtree_sum_deep_chain() {
    let mut rng = Rng::new(0xC24_5EED);
    for depth in 1..=40usize {
        let p = fresh_pair();
        for k in 0..depth {
            let v = rng.tame_f64();
            let parent = if k == 0 { -1 } else { k as i32 };
            let a = p.c.add_node(k as i32 + 1, parent, b"n", v);
            let b = p.r.add_node(k as i32 + 1, parent, b"n", v);
            eq_i32("C24", (depth, k), a, b);
        }
        for id in 0..=(depth as i32 + 1) {
            eq_bits("C24", (depth, id), p.c.subtree_bits(id), p.r.subtree_bits(id));
        }
    }
}

#[test]
fn c25_subtree_sum_random_forest() {
    let mut rng = Rng::new(0xC25_5EED);
    for _ in 0..300 {
        let p = fresh_pair();
        let n = 1 + rng.below(80) as usize;
        let spec = random_forest(&mut rng, n, false);
        load_forest("C25", &p, &spec);
        for id in probe_ids(&spec) {
            eq_bits("C25", id, p.c.subtree_bits(id), p.r.subtree_bits(id));
        }
    }
}

#[test]
fn c26_subtree_sum_duplicate_ids() {
    // Duplicated ids make C sum the SAME node twice (it recurses on
    // `node_storage[i].id`, and `find_node_by_id` always resolves to the first
    // match). Shapes are hand-built to stay acyclic.
    let mut rng = Rng::new(0xC26_5EED);
    for _ in 0..600 {
        let p = fresh_pair();
        let vals: Vec<f64> = (0..6).map(|_| rng.tame_f64()).collect();
        let build = |l: &Lib| {
            l.add_node(1, -1, b"root", vals[0]);
            l.add_node(2, 1, b"dupA", vals[1]);
            l.add_node(2, 1, b"dupB", vals[2]); // same id, also a child of 1
            l.add_node(3, 2, b"gk", vals[3]);
            l.add_node(3, 2, b"gk2", vals[4]); // duplicated leaf id too
            l.add_node(4, 1, b"other", vals[5]);
        };
        build(&p.c);
        build(&p.r);
        for id in [-1, 0, 1, 2, 3, 4, 5] {
            eq_bits("C26", id, p.c.subtree_bits(id), p.r.subtree_bits(id));
            eq_i32("C26", id, p.c.children(id), p.r.children(id));
            eq_node("C26", id, &p.c.find_snap(id), &p.r.find_snap(id));
        }
    }
}

#[test]
fn c27_subtree_sum_nan_inf_values() {
    let mut rng = Rng::new(0xC27_5EED);
    for _ in 0..600 {
        let p = fresh_pair();
        let n = 1 + rng.below(25) as usize;
        let spec = random_forest(&mut rng, n, true); // wild_values => NaN/inf/huge
        load_forest("C27", &p, &spec);
        for id in probe_ids(&spec) {
            eq_bits("C27", id, p.c.subtree_bits(id), p.r.subtree_bits(id));
            // and the clamped integer view of that sum
            let cs = unsafe { (p.c.calculate_subtree_sum)(id) };
            let rs = unsafe { (p.r.calculate_subtree_sum)(id) };
            eq_i32("C27", ("d2i", id), p.c.safe_d2i(cs), p.r.safe_d2i(rs));
        }
    }
}

#[test]
fn c28_subtree_sum_with_pruned_branches() {
    let mut rng = Rng::new(0xC28_5EED);
    for _ in 0..300 {
        let p = fresh_pair();
        let n = 3 + rng.below(40) as usize;
        let spec = random_forest(&mut rng, n, false);
        load_forest("C28", &p, &spec);
        for s in &spec {
            if rng.below(4) == 0 {
                p.c.set_active(s.id, 0);
                p.r.set_active(s.id, 0);
            }
        }
        for id in probe_ids(&spec) {
            eq_bits("C28", id, p.c.subtree_bits(id), p.r.subtree_bits(id));
        }
        // now rewrite parents in place (reshaping the tree) -- still acyclic
        // because each node is re-parented onto the first node only.
        let root = spec[0].id;
        for s in spec.iter().skip(1) {
            if rng.below(3) == 0 {
                p.c.set_parent(s.id, root);
                p.r.set_parent(s.id, root);
            }
        }
        for id in probe_ids(&spec) {
            eq_bits("C28", ("reparented", id), p.c.subtree_bits(id), p.r.subtree_bits(id));
            eq_i32("C28", ("children", id), p.c.children(id), p.r.children(id));
        }
    }
}

#[test]
fn c29_subtree_sum_maxnmin_shape() {
    // Exactly the six nodes `maxnmin` seeds, driven through the low-level API.
    let p = fresh_pair();
    let build = |l: &Lib| {
        l.add_node(1, -1, b"root", 10.5);
        l.add_node(2, 1, b"child1", 20.7);
        l.add_node(3, 1, b"child2", 15.3);
        l.add_node(4, 2, b"grandchild1", 5.9);
        l.add_node(5, 2, b"grandchild2", 8.2);
        l.add_node(6, 3, b"grandchild3", 12.4);
    };
    build(&p.c);
    build(&p.r);
    for id in [-1, 0, 1, 2, 3, 4, 5, 6, 7, i32::MIN, i32::MAX] {
        eq_bits("C29", id, p.c.subtree_bits(id), p.r.subtree_bits(id));
        eq_i32("C29", ("children", id), p.c.children(id), p.r.children(id));
        eq_node("C29", id, &p.c.find_snap(id), &p.r.find_snap(id));
        if id >= 1 && id <= 6 {
            eq_i32(
                "C29",
                ("name", id),
                p.c.process_name_in_place(id).unwrap(),
                p.r.process_name_in_place(id).unwrap(),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C38 -- Node ABI / struct layout parity
// ---------------------------------------------------------------------------

#[test]
fn c38_node_struct_layout_parity() {
    let p = fresh_pair();
    p.c.add_node(11, -1, b"first", 1.25);
    p.r.add_node(11, -1, b"first", 1.25);
    p.c.add_node(22, 11, b"second", -2.5);
    p.r.add_node(22, 11, b"second", -2.5);

    let (c0, c1) = (p.c.find_raw(11), p.c.find_raw(22));
    let (r0, r1) = (p.r.find_raw(11), p.r.find_raw(22));
    assert!(!c0.is_null() && !c1.is_null() && !r0.is_null() && !r1.is_null());

    let cstride = (c1 as isize) - (c0 as isize);
    let rstride = (r1 as isize) - (r0 as isize);
    assert_eq!(
        cstride, rstride,
        "[C38] sizeof(Node) differs: C stride {cstride}, Rust stride {rstride}"
    );
    assert_eq!(
        cstride,
        std::mem::size_of::<Node>() as isize,
        "[C38] Rust mirror struct size {} does not match observed C stride {cstride}",
        std::mem::size_of::<Node>()
    );

    // Compare the raw byte images IN FULL, including the two padding holes
    // (58..64 after `name`, 76..80 after `active`). The C standard leaves padding
    // contents unspecified, so this is a stronger claim than the ABI requires --
    // but both sides do in fact produce zeroed padding, so assert it rather than
    // silently masking bytes a caller could read back through the `Node*`.
    let sz = cstride as usize;
    let cb = unsafe { std::slice::from_raw_parts(c0 as *const u8, sz) };
    let rb = unsafe { std::slice::from_raw_parts(r0 as *const u8, sz) };
    assert_eq!(cb, rb, "[C38] full {sz}-byte Node image differs");

    // Field offsets, observed rather than assumed.
    assert_eq!(std::mem::offset_of!(Node, id), 0);
    assert_eq!(std::mem::offset_of!(Node, parent_id), 4);
    assert_eq!(std::mem::offset_of!(Node, name), 8);
    assert_eq!(std::mem::offset_of!(Node, value), 64);
    assert_eq!(std::mem::offset_of!(Node, active), 72);
    assert_eq!(sz, 80);
}

/// C38b: the full 80-byte image must match for randomized nodes too, not just
/// one hand-picked pair -- padding included.
#[test]
fn c38b_node_full_byte_image_randomized() {
    let mut rng = Rng::new(0x38B_5EED);
    for round in 0..200 {
        let p = fresh_pair();
        let n = 1 + rng.below(20) as usize;
        let mut ids = Vec::new();
        for i in 0..n {
            let len = rng.below(60) as usize;
            let name = rng.full_byte_name(len);
            let v = rng.mixed_f64();
            let id = 1000 + i as i32;
            let (a, b) = (
                p.c.add_node(id, -1, &name, v),
                p.r.add_node(id, -1, &name, v),
            );
            eq_i32("C38b", (round, i), a, b);
            ids.push(id);
        }
        for id in ids {
            let cp = p.c.find_raw(id) as *const u8;
            let rp = p.r.find_raw(id) as *const u8;
            assert!(!cp.is_null() && !rp.is_null());
            let cb = unsafe { std::slice::from_raw_parts(cp, 80) };
            let rb = unsafe { std::slice::from_raw_parts(rp, 80) };
            assert_eq!(cb, rb, "[C38b] round {round} id {id}: 80-byte image differs");
        }
    }
}
