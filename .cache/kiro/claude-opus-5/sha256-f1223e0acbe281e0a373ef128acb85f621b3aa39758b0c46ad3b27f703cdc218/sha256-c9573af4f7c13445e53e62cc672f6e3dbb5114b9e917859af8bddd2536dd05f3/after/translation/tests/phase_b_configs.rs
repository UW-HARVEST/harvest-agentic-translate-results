//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md` (rows 1..44). Every test drives BOTH the C
//! `.so` and the Rust `.so` through `dlsym` and compares results bit-for-bit
//! across MANY randomized inputs (fixed seed `common::SEED`).
//!
//! Tests are ordered lowest-level entry point first, as required.

mod common;
use common::*;

// ===========================================================================
// safe_double_to_int  (rows 1..5)
// ===========================================================================

#[test]
fn cfg_01_sdti_in_range_integral() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED);
    for _ in 0..20_000 {
        p.safe_double_to_int(rng.next_i32() as f64);
    }
    // plus every power-of-two magnitude
    for k in 0..31 {
        p.safe_double_to_int((1i64 << k) as f64);
        p.safe_double_to_int(-((1i64 << k) as f64));
    }
}

#[test]
fn cfg_02_sdti_fractional_positive() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..20_000 {
        let base = (rng.next_u32() >> 1) as f64; // [0, 2^31)
        let frac = (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
        p.safe_double_to_int(base + frac);
    }
    for f in [0.0, 0.1, 0.5, 0.9, 0.999_999_999, 1.5, 2.5, 1e-300] {
        p.safe_double_to_int(f);
        p.safe_double_to_int(1.0 + f);
        p.safe_double_to_int(2_147_483_646.0 + f.fract());
    }
}

#[test]
fn cfg_03_sdti_fractional_negative() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..20_000 {
        let base = -(((rng.next_u32() >> 1) as f64) + 1.0);
        let frac = (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
        p.safe_double_to_int(base + frac);
    }
    for f in [-0.0, -0.1, -0.5, -0.9, -1.5, -2.5, -1e-300, -0.999_999_999] {
        p.safe_double_to_int(f);
        p.safe_double_to_int(-1.0 + f);
        p.safe_double_to_int(-2_147_483_647.0 + f);
    }
}

#[test]
fn cfg_04_sdti_random_bit_patterns() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 4);
    // Arbitrary bit patterns: sweeps in NaN, +/-inf, subnormals, huge finites.
    for _ in 0..60_000 {
        p.safe_double_to_int(rng.next_f64_bits());
    }
    for _ in 0..20_000 {
        p.safe_double_to_int(rng.next_finite_f64());
    }
    for _ in 0..20_000 {
        p.safe_double_to_int(rng.next_in_int_range());
    }
}

#[test]
fn cfg_05_sdti_boundary_neighbourhoods() {
    let p = Pair::fresh();
    let anchors = [
        i32::MAX as f64,
        i32::MIN as f64,
        0.0f64,
        -0.0f64,
        1.0f64,
        -1.0f64,
        2_147_483_648.0f64,
        -2_147_483_649.0f64,
        4_294_967_296.0f64,
        f64::MIN_POSITIVE,
        f64::MAX,
        f64::MIN,
    ];
    for a in anchors {
        let bits = a.to_bits();
        for delta in -64i64..=64 {
            let b = (bits as i64).wrapping_add(delta) as u64;
            p.safe_double_to_int(f64::from_bits(b));
        }
        // and additive neighbourhoods
        for d in [-2.0, -1.5, -1.0, -0.5, 0.0, 0.5, 1.0, 1.5, 2.0] {
            p.safe_double_to_int(a + d);
        }
    }
}

// ===========================================================================
// process_string  (rows 6..9)
// ===========================================================================

#[test]
fn cfg_06_process_string_ascii() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..4_000 {
        let len = rng.below(48) as usize + 1;
        let s = rng.bytes(len, 0x20, 0x7E);
        p.process_string(&s);
    }
}

#[test]
fn cfg_07_process_string_full_byte_range() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..8_000 {
        let len = rng.below(48) as usize + 1;
        let s = rng.bytes(len, 0x01, 0xFF);
        p.process_string(&s);
    }
    // every single byte value on its own
    for b in 1u8..=255 {
        p.process_string(&[b]);
        p.process_string(&[b, b]);
        p.process_string(&[0xFF, b, 0x01]);
    }
}

#[test]
fn cfg_08_process_string_length_boundaries() {
    let p = Pair::fresh();
    for len in [1usize, 47, 48, 49, 50, 51, 99, 100] {
        for fill in [0x01u8, 0x7F, 0x80, 0xFF, b'a'] {
            p.process_string(&vec![fill; len]);
        }
    }
}

#[test]
fn cfg_09_process_string_long() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 9);
    for len in [255usize, 1024, 4096] {
        for _ in 0..20 {
            let s = rng.bytes(len, 0x01, 0xFF);
            p.process_string(&s);
        }
        p.process_string(&vec![0x7Fu8; len]);
        p.process_string(&vec![0x80u8; len]);
    }
}

// ===========================================================================
// add_node + find_node_by_id  (rows 10..17)
// ===========================================================================

#[test]
fn cfg_10_add_find_single_empty_name() {
    let p = Pair::fresh();
    assert_eq!(p.add_node(7, -1, b"", 3.25), 0);
    let (cp, _) = p.find_node_by_id(7).expect("node 7");
    assert_eq!(unsafe { (*cp).name[0] }, 0);
    p.get_children_count(-1);
    p.calculate_subtree_sum(7);
}

#[test]
fn cfg_11_add_find_name_length_matrix() {
    let mut rng = Rng::new(SEED ^ 11);
    for len in [0usize, 1, 2, 47, 48, 49, 50, 51, 200] {
        for &(lo, hi) in &[(b'a', b'z'), (0x01, 0x7F), (0x80, 0xFF), (0x01, 0xFF)] {
            let p = Pair::fresh();
            let name = rng.bytes(len, lo, hi);
            let value = rng.next_finite_f64();
            assert_eq!(p.add_node(1, -1, &name, value), 0);
            p.find_node_by_id(1).expect("node 1");
            p.process_node_name(1);
            p.calculate_subtree_sum(1);
        }
    }
    // interior NUL: strncpy stops there, the rest must stay zero
    let p = Pair::fresh();
    p.add_node(1, -1, b"ab\0cdefgh", 1.0);
    p.find_node_by_id(1).expect("node 1");
    p.process_node_name(1);
    // non-finite values stored verbatim
    let p2 = Pair::fresh();
    for (i, v) in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.0, f64::MIN_POSITIVE]
        .into_iter()
        .enumerate()
    {
        p2.add_node(i as i32 + 1, -1, b"x", v);
    }
    p2.assert_store_agrees(&[1, 2, 3, 4, 5, -1]);
}

#[test]
fn cfg_12_add_find_random_ids() {
    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..60 {
        let p = Pair::fresh();
        let n = rng.below(60) as usize + 1;
        let mut ids: Vec<i32> = Vec::new();
        for k in 0..n {
            let id = match k % 8 {
                0 => i32::MIN,
                1 => i32::MAX,
                2 => 0,
                3 => -1,
                _ => rng.next_i32(),
            };
            // keep ids distinct so the "first match" rule is not in play here
            let id = if ids.contains(&id) { id.wrapping_add(k as i32 + 1) } else { id };
            ids.push(id);
            let nlen = rng.below(50) as usize;
            let name = rng.bytes(nlen, 0x01, 0xFF);
            p.add_node(id, if k == 0 { -1 } else { ids[k / 2] }, &name, rng.next_finite_f64());
        }
        for &id in &ids {
            p.find_node_by_id(id);
            p.get_children_count(id);
        }
        for _ in 0..200 {
            p.find_node_by_id(rng.next_i32());
        }
    }
}

#[test]
fn cfg_13_find_duplicate_ids_first_wins() {
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..40 {
        let p = Pair::fresh();
        let id = rng.next_i32();
        let dup_count = rng.below(5) as usize + 2;
        for k in 0..dup_count {
            p.add_node(id, -1, format!("dup{k}").as_bytes(), (k as f64) + 0.5);
        }
        // first match must win -> value 0.5, name "dup0"
        let (cp, _) = p.find_node_by_id(id).expect("dup id");
        assert_eq!(unsafe { (*cp).value }, 0.5, "first match should win");
        // mutate through the pointer in both libs and re-read
        p.set_value(id, 1234.5);
        p.find_node_by_id(id);
        p.get_children_count(-1);
        p.calculate_subtree_sum(id);
    }
}

#[test]
fn cfg_14_add_to_capacity() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 14);
    for k in 0..MAX_NODES {
        let nlen = rng.below(52) as usize;
        let name = rng.bytes(nlen, 0x01, 0xFF);
        let idx = p.add_node(k as i32 + 1, (k as i32) / 3, &name, rng.next_finite_f64());
        assert_eq!(idx, k as i32, "insertion {k} index");
    }
    for k in 0..MAX_NODES {
        p.find_node_by_id(k as i32 + 1);
    }
    for parent in -2..40 {
        p.get_children_count(parent);
    }
    // 101st and beyond must be rejected identically
    for _ in 0..5 {
        assert_eq!(p.add_node(999, 0, b"overflow", 1.0), -1);
    }
    p.find_node_by_id(999);
}

#[test]
fn cfg_15_node_struct_layout_readback() {
    let mut rng = Rng::new(SEED ^ 15);
    let p = Pair::fresh();
    let mut recs: Vec<(i32, i32, Vec<u8>, f64)> = Vec::new();
    for k in 0..MAX_NODES {
        let id = k as i32 * 7 + 1;
        let parent = rng.range_i32(-3, 30);
        let nlen = rng.below(52) as usize;
        let name = rng.bytes(nlen, 0x01, 0xFF);
        let value = rng.next_f64_bits();
        p.add_node(id, parent, &name, value);
        recs.push((id, parent, name, value));
    }
    for (id, parent, name, value) in &recs {
        let (cp, rp) = p.find_node_by_id(*id).expect("node");
        for ptr in [cp, rp] {
            let n = unsafe { &*ptr };
            assert_eq!(n.id, *id);
            assert_eq!(n.parent_id, *parent);
            assert_eq!(n.value.to_bits(), value.to_bits());
            assert_eq!(n.active, 1);
            let want: Vec<u8> = {
                let mut v: Vec<u8> = name.iter().copied().take_while(|&b| b != 0).collect();
                v.truncate(MAX_NAME_LEN - 1);
                v
            };
            let got: Vec<u8> = n
                .name
                .iter()
                .map(|&b| b as u8)
                .take_while(|&b| b != 0)
                .collect();
            assert_eq!(got, want, "stored name for id {id}");
            assert_eq!(n.name[MAX_NAME_LEN - 1], 0, "name must be NUL at [49]");
        }
    }
}

#[test]
fn cfg_16_deactivate_through_pointer() {
    let mut rng = Rng::new(SEED ^ 16);
    for _ in 0..30 {
        let p = Pair::fresh();
        let n = rng.below(20) as usize + 2;
        for k in 0..n {
            p.add_node(k as i32 + 1, if k == 0 { -1 } else { 1 }, b"n", (k as f64) + 1.0);
        }
        for k in 0..n {
            if rng.next_u64() & 1 == 0 {
                p.set_active(k as i32 + 1, 0);
            }
        }
        for k in 0..=n {
            p.find_node_by_id(k as i32 + 1);
            p.get_children_count(k as i32 + 1);
            p.calculate_subtree_sum(k as i32 + 1);
        }
        p.get_children_count(-1);
        p.get_children_count(1);
    }
}

#[test]
fn cfg_17_active_truthy_nonone() {
    let truthy = [2i32, -1, i32::MIN, i32::MAX, 0x0100_0000, -7];
    for a in truthy {
        let p = Pair::fresh();
        p.add_node(1, -1, b"root", 1.5);
        p.add_node(2, 1, b"kid", 2.5);
        p.set_active(2, a);
        p.find_node_by_id(2).expect("node 2 still visible");
        assert_eq!(p.get_children_count(1), 1);
        p.calculate_subtree_sum(1);
        p.set_active(1, a);
        p.find_node_by_id(1);
        p.calculate_subtree_sum(1);
    }
}

// ===========================================================================
// get_children_count  (rows 18..23)
// ===========================================================================

#[test]
fn cfg_18_children_count_trivial() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 18);
    for _ in 0..500 {
        p.get_children_count(rng.next_i32());
    }
    p.add_node(1, -1, b"only", 1.0);
    for _ in 0..500 {
        p.get_children_count(rng.next_i32());
    }
    p.get_children_count(-1);
    p.get_children_count(1);
}

#[test]
fn cfg_19_children_count_single_parent() {
    let mut rng = Rng::new(SEED ^ 19);
    for kids in 1..=30 {
        let p = Pair::fresh();
        p.add_node(1000, -1, b"parent", 1.0);
        for k in 0..kids {
            p.add_node(k + 1, 1000, b"kid", rng.next_finite_f64());
        }
        assert_eq!(p.get_children_count(1000), kids);
        p.get_children_count(-1);
        p.get_children_count(1);
        for _ in 0..50 {
            p.get_children_count(rng.next_i32());
        }
        p.calculate_subtree_sum(1000);
    }
}

#[test]
fn cfg_20_children_count_forest() {
    let mut rng = Rng::new(SEED ^ 20);
    for _ in 0..40 {
        let p = Pair::fresh();
        let pool: Vec<i32> = (0..6).map(|_| rng.range_i32(-5, 5)).collect();
        let n = rng.below(90) as usize + 1;
        for k in 0..n {
            let parent = pool[rng.below(pool.len() as u64) as usize];
            p.add_node(k as i32 + 1, parent, b"x", rng.next_finite_f64());
        }
        for &parent in &pool {
            p.get_children_count(parent);
        }
        for q in -8..=8 {
            p.get_children_count(q);
        }
    }
}

#[test]
fn cfg_21_children_count_all_same_parent() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 21);
    for k in 0..MAX_NODES {
        p.add_node(k as i32 + 1, 42, b"same", rng.next_finite_f64());
    }
    assert_eq!(p.get_children_count(42), MAX_NODES as i32);
    p.get_children_count(41);
    p.get_children_count(43);
    // node id 42 exists (index 41) and is its own parent's target -> subtree
    // recursion would be infinite, so only the count is probed here.
}

#[test]
fn cfg_22_children_count_mixed_active() {
    let mut rng = Rng::new(SEED ^ 22);
    for _ in 0..40 {
        let p = Pair::fresh();
        let n = rng.below(60) as usize + 5;
        for k in 0..n {
            p.add_node(k as i32 + 1, (k as i32) % 5, b"m", rng.next_finite_f64());
        }
        for k in 0..n {
            match rng.below(3) {
                0 => p.set_active(k as i32 + 1, 0),
                1 => p.set_active(k as i32 + 1, rng.next_i32()),
                _ => {}
            }
        }
        for q in -2..8 {
            p.get_children_count(q);
        }
        for k in 0..n {
            p.find_node_by_id(k as i32 + 1);
        }
    }
}

#[test]
fn cfg_23_children_count_sentinel_parents() {
    let p = Pair::fresh();
    let sentinels = [-1i32, 0, i32::MIN, i32::MAX, 1];
    for (k, s) in sentinels.iter().enumerate() {
        for j in 0..=k {
            p.add_node((k * 10 + j) as i32 + 1, *s, b"s", (k + j) as f64);
        }
    }
    for s in sentinels {
        p.get_children_count(s);
    }
    p.get_children_count(i32::MIN + 1);
    p.get_children_count(i32::MAX - 1);
}

// ===========================================================================
// calculate_subtree_sum  (rows 24..31)
// ===========================================================================

#[test]
fn cfg_24_subtree_sum_leaf() {
    let mut rng = Rng::new(SEED ^ 24);
    for _ in 0..300 {
        let p = Pair::fresh();
        let id = rng.next_i32();
        p.add_node(id, id.wrapping_sub(1), b"leaf", rng.next_f64_bits());
        p.calculate_subtree_sum(id);
        p.calculate_subtree_sum(id.wrapping_add(1));
    }
}

#[test]
fn cfg_25_subtree_sum_two_level() {
    let mut rng = Rng::new(SEED ^ 25);
    for kids in 0..=25 {
        let p = Pair::fresh();
        p.add_node(1, -1, b"root", rng.next_finite_f64());
        for k in 0..kids {
            p.add_node(k + 2, 1, b"kid", rng.next_finite_f64());
        }
        p.calculate_subtree_sum(1);
        for k in 0..kids {
            p.calculate_subtree_sum(k + 2);
        }
    }
}

#[test]
fn cfg_26_subtree_sum_three_level() {
    let mut rng = Rng::new(SEED ^ 26);
    for _ in 0..200 {
        let p = Pair::fresh();
        // exactly the shape maxnmin builds, random values
        p.add_node(1, -1, b"root", rng.next_finite_f64());
        p.add_node(2, 1, b"child1", rng.next_finite_f64());
        p.add_node(3, 1, b"child2", rng.next_finite_f64());
        p.add_node(4, 2, b"grandchild1", rng.next_finite_f64());
        p.add_node(5, 2, b"grandchild2", rng.next_finite_f64());
        p.add_node(6, 3, b"grandchild3", rng.next_finite_f64());
        for id in 0..=7 {
            p.calculate_subtree_sum(id);
            p.get_children_count(id);
        }
    }
}

#[test]
fn cfg_27_subtree_sum_deep_chain() {
    let mut rng = Rng::new(SEED ^ 27);
    for depth in [1usize, 2, 5, 10, 20, 40, 99] {
        let p = Pair::fresh();
        // ids strictly increasing along parent->child: no cycles
        p.add_node(1, -1, b"c", rng.next_finite_f64());
        for k in 1..depth {
            p.add_node(k as i32 + 1, k as i32, b"c", rng.next_finite_f64());
        }
        for id in 1..=(depth as i32 + 1) {
            p.calculate_subtree_sum(id);
        }
    }
}

#[test]
fn cfg_28_subtree_sum_random_forest() {
    let mut rng = Rng::new(SEED ^ 28);
    for _ in 0..80 {
        let p = Pair::fresh();
        let n = rng.below(70) as usize + 1;
        // parent id < own id keeps the id-graph acyclic (C recurses on ids)
        for k in 0..n {
            let id = k as i32 + 1;
            let parent = if k == 0 { -1 } else { rng.range_i32(0, k as i32) };
            p.add_node(id, parent, b"f", rng.next_finite_f64());
        }
        for id in -1..=(n as i32 + 2) {
            p.calculate_subtree_sum(id);
            p.get_children_count(id);
        }
    }
}

#[test]
fn cfg_29_subtree_sum_duplicate_ids() {
    let mut rng = Rng::new(SEED ^ 29);
    for _ in 0..60 {
        let p = Pair::fresh();
        // 10 is the root; several children all share id 20 (a duplicate id).
        // C's recursion looks children up *by id*, so find_node_by_id(20)
        // returns the FIRST id-20 node every time -> its value is counted once
        // per id-20 child edge, and the later duplicates' values never count.
        p.add_node(10, -1, b"root", rng.next_finite_f64());
        let dups = rng.below(4) as usize + 2;
        for k in 0..dups {
            p.add_node(20, 10, format!("d{k}").as_bytes(), rng.next_finite_f64());
        }
        // a distinct deeper level hanging off 20, ids strictly increasing
        for k in 0..rng.below(3) + 1 {
            p.add_node(30 + k as i32, 20, b"gk", rng.next_finite_f64());
        }
        p.calculate_subtree_sum(10);
        p.calculate_subtree_sum(20);
        for k in 0..3 {
            p.calculate_subtree_sum(30 + k);
        }
        p.get_children_count(10);
        p.get_children_count(20);
        p.find_node_by_id(20);
    }
}

#[test]
fn cfg_30_subtree_sum_fp_association() {
    // Values chosen so that a different summation order gives a different result.
    let sets: [&[f64]; 6] = [
        &[1e16, 1.0, -1e16],
        &[-1e16, 1e16, 1.0],
        &[1.0, 1e16, -1e16, 1.0],
        &[f64::MAX, f64::MAX, -f64::MAX],
        &[1e300, 1e-300, -1e300, 1e-300],
        &[0.1, 0.2, 0.3, -0.6, 1e17, 1.0, -1e17],
    ];
    for s in sets {
        let p = Pair::fresh();
        p.add_node(1, -1, b"root", s[0]);
        for (k, v) in s.iter().enumerate().skip(1) {
            p.add_node(k as i32 + 1, 1, b"k", *v);
        }
        p.calculate_subtree_sum(1);
        // same values, nested one level deeper (different association)
        let p2 = Pair::fresh();
        p2.add_node(1, -1, b"root", s[0]);
        for (k, v) in s.iter().enumerate().skip(1) {
            p2.add_node(k as i32 + 1, k as i32, b"k", *v);
        }
        p2.calculate_subtree_sum(1);
    }
}

#[test]
fn cfg_31_subtree_sum_active_mask() {
    let mut rng = Rng::new(SEED ^ 31);
    for _ in 0..80 {
        let p = Pair::fresh();
        p.add_node(1, -1, b"root", rng.next_finite_f64());
        p.add_node(2, 1, b"c1", rng.next_finite_f64());
        p.add_node(3, 1, b"c2", rng.next_finite_f64());
        p.add_node(4, 2, b"g1", rng.next_finite_f64());
        p.add_node(5, 2, b"g2", rng.next_finite_f64());
        p.add_node(6, 3, b"g3", rng.next_finite_f64());
        for id in 1..=6 {
            match rng.below(4) {
                0 => p.set_active(id, 0),
                1 => p.set_active(id, rng.next_i32()),
                _ => {}
            }
        }
        for id in 0..=7 {
            p.calculate_subtree_sum(id);
            p.get_children_count(id);
            p.find_node_by_id(id);
        }
    }
}

// ===========================================================================
// maxnmin  (rows 32..38)
// ===========================================================================

#[test]
fn cfg_32_maxnmin_residue_cross_product() {
    let p = Pair::fresh();
    for r1 in 0..6i32 {
        for r2 in 0..6i32 {
            for r4 in 0..3i32 {
                for mult in [0i32, 1, 2, 17] {
                    let p1 = r1 + 6 * mult;
                    let p2 = r2 + 6 * mult;
                    let p4 = r4 + 3 * mult;
                    p.maxnmin(p1, p2, 1, p4);
                }
            }
        }
    }
}

#[test]
fn cfg_33_maxnmin_negative_residues() {
    let p = Pair::fresh();
    for r1 in -5..=0i32 {
        for r2 in -5..=0i32 {
            for r4 in -2..=0i32 {
                for mult in [0i32, 1, 2, 17] {
                    p.maxnmin(r1 - 6 * mult, r2 - 6 * mult, 1, r4 - 3 * mult);
                }
            }
        }
    }
}

#[test]
fn cfg_34_maxnmin_param3_classes() {
    let p = Pair::fresh();
    let p3s = [i32::MIN, i32::MIN + 1, -3, -2, -1, 0, 1, 2, 3, i32::MAX - 1, i32::MAX];
    for p3 in p3s {
        for p1 in [-7i32, -1, 0, 1, 5, 6, 12] {
            for p2 in [-7i32, -1, 0, 1, 5, 6, 12] {
                for p4 in [-4i32, -1, 0, 1, 2, 3, 9] {
                    p.maxnmin(p1, p2, p3, p4);
                }
            }
        }
    }
}

#[test]
fn cfg_35_maxnmin_param4_classes() {
    let p = Pair::fresh();
    let p4s = [i32::MIN, i32::MIN + 1, -3, -2, -1, 0, 1, 2, 3, i32::MAX - 1, i32::MAX];
    for p4 in p4s {
        for p1 in [i32::MIN, -6, -1, 0, 1, 6, i32::MAX] {
            for p2 in [i32::MIN, -6, -1, 0, 1, 6, i32::MAX] {
                for p3 in [i32::MIN, -1, 0, 1, i32::MAX] {
                    p.maxnmin(p1, p2, p3, p4);
                }
            }
        }
    }
}

#[test]
fn cfg_36_maxnmin_random_quadruples() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 36);
    for _ in 0..20_000 {
        p.maxnmin(rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32());
    }
    // narrow-range randoms hit the small-residue paths much more often
    for _ in 0..20_000 {
        p.maxnmin(
            rng.range_i32(-20, 20),
            rng.range_i32(-20, 20),
            rng.range_i32(-20, 20),
            rng.range_i32(-20, 20),
        );
    }
}

#[test]
fn cfg_37_maxnmin_boundary_pool_cross_product() {
    let p = Pair::fresh();
    const POOL: [i32; 14] = [
        0,
        1,
        -1,
        2,
        -2,
        5,
        -5,
        6,
        -6,
        7,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
    ];
    for a in POOL {
        for b in POOL {
            for c in POOL {
                for d in POOL {
                    p.maxnmin(a, b, c, d);
                }
            }
        }
    }
}

#[test]
fn cfg_38_maxnmin_final_term_saturation() {
    let p = Pair::fresh();
    // (p1+p2)/(p3+1)*p4 landing on / past the int range
    let cases: &[(i32, i32, i32, i32)] = &[
        (i32::MAX, 0, 0, 1),
        (i32::MAX, 0, 0, 2),
        (i32::MIN, 0, 0, 1),
        (i32::MIN, 0, 0, 2),
        (i32::MAX, i32::MAX, 0, i32::MAX),
        (i32::MIN, i32::MIN, 0, i32::MAX),
        (1, 0, 0, i32::MAX),
        (1, 0, 0, i32::MIN),
        (2_147_483_646, 0, 0, 1),
        (2_147_483_646, 1, 0, 1),
        (i32::MAX, 0, -1, 0),
        (i32::MAX, 0, -1, 1),
        (i32::MAX, 0, -1, -1),
        (0, 0, -1, 0),
        (0, 0, -1, 7),
        (6, -6, -1, 3),
        (i32::MAX, 1, -1, 5),
        (i32::MIN, 0, i32::MAX, 1),
        (i32::MAX, i32::MAX, i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN, i32::MIN, i32::MIN),
    ];
    for &(a, b, c, d) in cases {
        p.maxnmin(a, b, c, d);
    }
    // sweep p4 for a fixed huge quotient
    for d in -40..=40 {
        p.maxnmin(i32::MAX, 0, 0, d);
        p.maxnmin(i32::MIN, 0, 0, d);
        p.maxnmin(i32::MAX, 0, -1, d);
    }
}

// ===========================================================================
// Statefulness / composition  (rows 39..44)
// ===========================================================================

#[test]
fn cfg_39_random_interleaved_call_sequence() {
    let mut rng = Rng::new(SEED ^ 39);
    for _round in 0..40 {
        let p = Pair::fresh();
        // Track ids we created so parent_id < id keeps the id-graph acyclic and
        // calculate_subtree_sum always terminates (same in C and Rust).
        let mut next_id: i32 = 1;
        for _step in 0..400 {
            match rng.below(7) {
                0 => {
                    let parent = if next_id == 1 { -1 } else { rng.range_i32(0, next_id - 1) };
                    let nlen = rng.below(55) as usize;
                    let name = rng.bytes(nlen, 0x01, 0xFF);
                    let idx = p.add_node(next_id, parent, &name, rng.next_finite_f64());
                    if idx >= 0 {
                        next_id += 1;
                    }
                }
                1 => {
                    p.find_node_by_id(rng.range_i32(-2, next_id + 2));
                }
                2 => {
                    p.get_children_count(rng.range_i32(-2, next_id + 2));
                }
                3 => {
                    p.calculate_subtree_sum(rng.range_i32(-2, next_id + 2));
                }
                4 => {
                    p.safe_double_to_int(rng.next_f64_bits());
                }
                5 => {
                    let id = rng.range_i32(1, next_id.max(1));
                    if p.find_node_by_id(id).is_some() {
                        match rng.below(3) {
                            0 => p.set_active(id, 0),
                            1 => p.set_active(id, rng.next_i32()),
                            _ => p.set_value(id, rng.next_f64_bits()),
                        }
                    }
                }
                _ => {
                    // maxnmin RESETS node_count to 0 and rebuilds the 6-node tree
                    p.maxnmin(rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32());
                    next_id = 7;
                }
            }
        }
    }
}

#[test]
fn cfg_40_state_add_then_maxnmin() {
    let mut rng = Rng::new(SEED ^ 40);
    for n in [0usize, 1, 5, 6, 7, 50, 99, 100] {
        let p = Pair::fresh();
        for k in 0..n {
            p.add_node(k as i32 + 100, 99, b"pre", rng.next_finite_f64());
        }
        let v = p.maxnmin(4, 5, 3, 2);
        // node_count was reset to 0 and rebuilt with exactly 6 nodes
        for id in -2..=8 {
            p.find_node_by_id(id);
            p.get_children_count(id);
            p.calculate_subtree_sum(id);
        }
        // the pre-existing nodes are now invisible (node_count == 6)
        for k in 0..n {
            assert!(p.find_node_by_id(k as i32 + 100).is_none() || k < 6);
        }
        // and the result is the same as from a pristine library
        let fresh = Pair::fresh();
        assert_eq!(fresh.maxnmin(4, 5, 3, 2), v);
    }
}

#[test]
fn cfg_41_state_maxnmin_then_add() {
    let mut rng = Rng::new(SEED ^ 41);
    let p = Pair::fresh();
    p.maxnmin(1, 2, 3, 4);
    for k in 0..20 {
        let idx = p.add_node(k + 7, 1, b"post", rng.next_finite_f64());
        assert_eq!(idx, 6 + k, "append index after maxnmin");
    }
    assert_eq!(p.get_children_count(1), 22); // child1, child2 + 20 new
    for id in -2..=30 {
        p.find_node_by_id(id);
        p.get_children_count(id);
        p.calculate_subtree_sum(id);
    }
    // another maxnmin wipes them again
    p.maxnmin(1, 2, 3, 4);
    assert_eq!(p.get_children_count(1), 2);
}

#[test]
fn cfg_42_maxnmin_idempotent_repeat() {
    let mut rng = Rng::new(SEED ^ 42);
    let p = Pair::fresh();
    for _ in 0..2_000 {
        let (a, b, c, d) = (rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32());
        let v1 = p.maxnmin(a, b, c, d);
        let v2 = p.maxnmin(a, b, c, d);
        let v3 = p.maxnmin(a, b, c, d);
        assert_eq!(v1, v2, "maxnmin not idempotent");
        assert_eq!(v2, v3, "maxnmin not idempotent");
    }
}

#[test]
fn cfg_43_state_recover_after_capacity() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 43);
    for k in 0..MAX_NODES {
        p.add_node(k as i32 + 1, -1, b"fill", rng.next_finite_f64());
    }
    for _ in 0..10 {
        assert_eq!(p.add_node(12345, -1, b"nope", 1.0), -1);
    }
    p.maxnmin(2, 3, 4, 5);
    assert_eq!(p.add_node(77, 1, b"after", 2.5), 6);
    p.find_node_by_id(77);
    assert_eq!(p.get_children_count(1), 3);
    for id in -2..=10 {
        p.calculate_subtree_sum(id);
    }
}

#[test]
fn cfg_44_compose_find_then_process_string() {
    let mut rng = Rng::new(SEED ^ 44);
    for _ in 0..300 {
        let p = Pair::fresh();
        let n = rng.below(10) as usize + 1;
        for k in 0..n {
            let len = rng.below(60) as usize;
            let mut name = rng.bytes(len, 0x01, 0xFF);
            if rng.next_u64() & 3 == 0 && !name.is_empty() {
                // force some high-bit bytes
                name[0] = 0x80 | (name[0] & 0x7F);
            }
            p.add_node(k as i32 + 1, if k == 0 { -1 } else { 1 }, &name, rng.next_finite_f64());
        }
        for k in 0..n {
            p.process_node_name(k as i32 + 1);
        }
    }
    // and through the exact maxnmin composition: node names are fixed there
    let p = Pair::fresh();
    p.maxnmin(0, 0, 1, 0);
    for id in 1..=6 {
        p.process_node_name(id);
    }
}

// ===========================================================================
// Row 45 — NaN / non-finite propagation through the FP accumulation
// ===========================================================================

/// The pool of doubles that makes `addsd`'s NaN-propagation rule observable.
fn fp_pool() -> Vec<f64> {
    let mut v = vec![
        0.0f64,
        -0.0,
        1.0,
        -1.0,
        0.5,
        f64::MAX,
        f64::MIN,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::from_bits(1),          // smallest positive subnormal
        f64::from_bits(1 | 1 << 63), // smallest negative subnormal
        f64::INFINITY,
        f64::NEG_INFINITY,
        1e300,
        -1e300,
        1e-300,
    ];
    // NaNs: quiet/signalling, both signs, distinct payloads
    for payload in [1u64, 2, 0x0000_0000_00FF_FFFF, 0x0007_FFFF_FFFF_FFFF] {
        for quiet in [true, false] {
            for sign in [0u64, 1 << 63] {
                let mut bits = sign | 0x7FF0_0000_0000_0000 | payload;
                if quiet {
                    bits |= 0x0008_0000_0000_0000;
                }
                let d = f64::from_bits(bits);
                if d.is_nan() {
                    v.push(d);
                }
            }
        }
    }
    v.push(f64::NAN);
    v.push(-f64::NAN);
    v
}

#[test]
fn cfg_45_subtree_sum_nan_propagation_exhaustive_pairs() {
    let pool = fp_pool();
    // Full cross-product as (root, single child): the two-operand `addsd` case.
    for &a in &pool {
        for &b in &pool {
            let p = Pair::fresh();
            p.add_node(1, -1, b"r", a);
            p.add_node(2, 1, b"k", b);
            p.calculate_subtree_sum(1);
            p.calculate_subtree_sum(2);
            p.safe_double_to_int(p.calculate_subtree_sum(1));
        }
    }
}

#[test]
fn cfg_46_subtree_sum_nan_propagation_multi_child() {
    let pool = fp_pool();
    let mut rng = Rng::new(SEED ^ 46);
    // Several children under one root: the accumulator is fed repeatedly, so the
    // per-step operand order (child = dst, accumulator = src) is what decides
    // which NaN survives.
    for _ in 0..3_000 {
        let p = Pair::fresh();
        let kids = rng.below(6) as usize + 1;
        p.add_node(1, -1, b"r", pool[rng.below(pool.len() as u64) as usize]);
        for k in 0..kids {
            p.add_node(
                k as i32 + 2,
                1,
                b"k",
                pool[rng.below(pool.len() as u64) as usize],
            );
        }
        p.calculate_subtree_sum(1);
    }
    // deterministic 3-deep shapes over every ordered triple of the "interesting"
    // NaN subset
    let nans: Vec<f64> = pool.iter().copied().filter(|d| d.is_nan()).collect();
    let others = [f64::INFINITY, f64::NEG_INFINITY, 1.0, -0.0, f64::MAX];
    for &a in &nans {
        for &b in &nans {
            for &c in &others {
                let p = Pair::fresh();
                p.add_node(1, -1, b"r", a);
                p.add_node(2, 1, b"c1", b);
                p.add_node(3, 1, b"c2", c);
                p.add_node(4, 2, b"g1", b);
                p.add_node(5, 3, b"g2", a);
                p.calculate_subtree_sum(1);
                p.calculate_subtree_sum(2);
                p.calculate_subtree_sum(3);
            }
        }
    }
}

#[test]
fn cfg_47_subtree_sum_nan_propagation_deep_and_masked() {
    let pool = fp_pool();
    let mut rng = Rng::new(SEED ^ 47);
    for _ in 0..2_000 {
        let p = Pair::fresh();
        // acyclic id-graph: parent id < own id
        let n = rng.below(24) as usize + 1;
        for k in 0..n {
            let parent = if k == 0 { -1 } else { rng.range_i32(0, k as i32) };
            p.add_node(
                k as i32 + 1,
                parent,
                b"n",
                pool[rng.below(pool.len() as u64) as usize],
            );
        }
        // random active mask
        for k in 0..n {
            if rng.below(4) == 0 {
                p.set_active(k as i32 + 1, if rng.next_u64() & 1 == 0 { 0 } else { rng.next_i32() });
            }
        }
        for id in 0..=(n as i32 + 1) {
            let s = p.calculate_subtree_sum(id);
            p.safe_double_to_int(s);
        }
    }
}
