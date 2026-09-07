// Phase B — valid-path differential tests.
//
// One test per row of CONFIGS.md. Every call goes through the `.so` exports of
// BOTH the C library and the Rust cdylib; nothing calls the Rust crate directly.

mod common;

use common::*;

const SEED: u64 = 0x5EED_1234_ABCD_0001;

/// The exact 6-node tree that `maxnmin` builds internally.
const MAXNMIN_TREE: [(i32, i32, &[u8], f64); 6] = [
    (1, -1, b"root", 10.5),
    (2, 1, b"child1", 20.7),
    (3, 1, b"child2", 15.3),
    (4, 2, b"grandchild1", 5.9),
    (5, 2, b"grandchild2", 8.2),
    (6, 3, b"grandchild3", 12.4),
];

fn build_maxnmin_tree(p: &Pair) {
    for (id, pid, name, v) in MAXNMIN_TREE {
        let cv = p.c.add(id, pid, name, v);
        let rv = p.r.add(id, pid, name, v);
        eq_i32(&format!("add_node({id})"), cv, rv);
    }
}

// ---------------------------------------------------------------------------
// Row 1-3: safe_double_to_int
// ---------------------------------------------------------------------------

#[test]
fn cfg_01_sdti_random_bits() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 1);

    for d in interesting_doubles() {
        eq_i32(
            &format!("sdti({d:?} bits {:#018x})", d.to_bits()),
            p.c.sdti(d),
            p.r.sdti(d),
        );
    }
    for i in 0..20_000 {
        let d = rng.any_f64();
        eq_i32(
            &format!("#{i} sdti(bits {:#018x})", d.to_bits()),
            p.c.sdti(d),
            p.r.sdti(d),
        );
    }
}

#[test]
fn cfg_02_sdti_random_near_edges() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 2);
    let anchors = [i32::MAX as f64, i32::MIN as f64, 0.0, -1.0, 1.0];

    for a in anchors {
        // +/- a few ulps
        let mut bits = a.to_bits();
        for _ in 0..64 {
            for d in [f64::from_bits(bits), -f64::from_bits(bits)] {
                eq_i32(
                    &format!("sdti ulp {d:?} ({:#018x})", d.to_bits()),
                    p.c.sdti(d),
                    p.r.sdti(d),
                );
            }
            bits = bits.wrapping_add(1);
        }
        // +/- random small deltas
        for _ in 0..2_000 {
            let delta = rng.finite_f64(4.0);
            let d = a + delta;
            eq_i32(&format!("sdti edge {d:?}"), p.c.sdti(d), p.r.sdti(d));
        }
    }
}

#[test]
fn cfg_03_sdti_random_in_range() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..20_000 {
        let d = rng.finite_f64(2_147_483_648.0);
        eq_i32(&format!("sdti in-range {d:?}"), p.c.sdti(d), p.r.sdti(d));
        let d2 = rng.finite_f64(1.0);
        eq_i32(&format!("sdti frac {d2:?}"), p.c.sdti(d2), p.r.sdti(d2));
        let d3 = rng.finite_f64(1e10);
        eq_i32(&format!("sdti wide {d3:?}"), p.c.sdti(d3), p.r.sdti(d3));
    }
}

// ---------------------------------------------------------------------------
// Rows 4-6: process_string
// ---------------------------------------------------------------------------

#[test]
fn cfg_04_process_string_random() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 4);

    // every single byte value on its own
    for b in 1u8..=255 {
        eq_i32(
            &format!("process_string([{b:#04x}])"),
            p.c.process_str(&[b]),
            p.r.process_str(&[b]),
        );
    }
    // the empty string (the `if (*str)` guard)
    eq_i32("process_string(\"\")", p.c.process_str(b""), p.r.process_str(b""));

    for i in 0..5_000 {
        let len = rng.range_usize(0, 64);
        let s = rand_name(&mut rng, len);
        eq_i32(
            &format!("#{i} process_string(len {len})"),
            p.c.process_str(&s),
            p.r.process_str(&s),
        );
    }
}

#[test]
fn cfg_05_process_string_long() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 5);
    for i in 0..300 {
        let len = rng.range_usize(1, 4096);
        // bias towards high-bit bytes so the signed-char sum goes very negative
        let s: Vec<u8> = (0..len)
            .map(|_| {
                if rng.next_u64() & 1 == 0 {
                    0x80 | (rng.next_u64() as u8 & 0x7f) | 0x80
                } else {
                    rng.byte_nonzero()
                }
            })
            .map(|b| if b == 0 { 1 } else { b })
            .collect();
        eq_i32(
            &format!("#{i} process_string(long len {len})"),
            p.c.process_str(&s),
            p.r.process_str(&s),
        );
    }
    // all-0xFF strings of increasing length (monotonic negative accumulation)
    for len in [1usize, 2, 49, 50, 51, 255, 256, 1000, 4096] {
        let s = vec![0xFFu8; len];
        eq_i32(
            &format!("process_string(0xFF x {len})"),
            p.c.process_str(&s),
            p.r.process_str(&s),
        );
    }
}

#[test]
fn cfg_06_process_string_embedded_nul() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 6);
    for i in 0..500 {
        let head = rand_name_upto(&mut rng, 0, 20);
        let tail = rand_name_upto(&mut rng, 0, 20);
        let mut buf = head.clone();
        buf.push(0);
        buf.extend_from_slice(&tail);
        buf.push(0);
        let mut cbuf = buf.clone();
        let mut rbuf = buf.clone();
        eq_i32(
            &format!("#{i} process_string(embedded NUL after {})", head.len()),
            p.c.process(&mut cbuf),
            p.r.process(&mut rbuf),
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 7-12: add_node / find_node_by_id
// ---------------------------------------------------------------------------

#[test]
fn cfg_07_add_then_find_single() {
    let mut rng = Rng::new(SEED ^ 7);
    for i in 0..500 {
        let p = Pair::fresh(); // pristine: node_count == 0
        let id = rng.next_i32();
        let pid = rng.next_i32();
        let len = rng.range_usize(0, 60);
        let name = rand_name(&mut rng, len);
        let value = if i % 3 == 0 {
            rng.any_f64()
        } else {
            rng.finite_f64(1e6)
        };

        eq_i32(
            &format!("#{i} add_node ret"),
            p.c.add(id, pid, &name, value),
            p.r.add(id, pid, &name, value),
        );

        let cs = p.c.find_snap(id);
        let rs = p.r.find_snap(id);
        assert_eq!(cs, rs, "#{i} find_node_by_id({id}) snapshot mismatch");
        assert!(cs.is_some(), "#{i} freshly added node must be findable");

        assert_eq!(
            p.c.find_rel_index(id, id),
            p.r.find_rel_index(id, id),
            "#{i} relative index"
        );
    }
}

#[test]
fn cfg_08_add_tree_find_all() {
    let p = Pair::fresh();
    build_maxnmin_tree(&p);
    for id in -3..=12 {
        assert_eq!(
            p.c.find_snap(id),
            p.r.find_snap(id),
            "find_node_by_id({id}) snapshot"
        );
        assert_eq!(
            p.c.find_rel_index(id, 1),
            p.r.find_rel_index(id, 1),
            "find_node_by_id({id}) relative index"
        );
    }
    for id in [i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1, 0, -1] {
        assert_eq!(
            p.c.find_snap(id),
            p.r.find_snap(id),
            "find_node_by_id({id}) extreme"
        );
    }
}

#[test]
fn cfg_09_duplicate_and_extreme_ids() {
    let mut rng = Rng::new(SEED ^ 9);
    for iter in 0..200 {
        let p = Pair::fresh();
        // sentinel at index 0 with a unique id so relative indices are comparable
        let sentinel = 0x4242_4242;
        eq_i32("sentinel add", p.c.add(sentinel, -1, b"s", 1.0), p.r.add(sentinel, -1, b"s", 1.0));

        let pool = [
            0i32,
            -1,
            1,
            7,
            i32::MIN,
            i32::MIN + 1,
            i32::MAX,
            i32::MAX - 1,
        ];
        let n = rng.range_usize(1, 20);
        for k in 0..n {
            let id = pool[rng.range_usize(0, pool.len() - 1)];
            let pid = pool[rng.range_usize(0, pool.len() - 1)];
            let v = rng.finite_f64(100.0);
            let name = rand_name_upto(&mut rng, 0, 10);
            eq_i32(
                &format!("iter{iter} add#{k}"),
                p.c.add(id, pid, &name, v),
                p.r.add(id, pid, &name, v),
            );
        }
        for id in pool {
            assert_eq!(
                p.c.find_snap(id),
                p.r.find_snap(id),
                "iter{iter} find({id}) snapshot (duplicate ids => first match)"
            );
            assert_eq!(
                p.c.find_rel_index(id, sentinel),
                p.r.find_rel_index(id, sentinel),
                "iter{iter} find({id}) index"
            );
            eq_i32(
                &format!("iter{iter} children({id})"),
                p.c.children(id),
                p.r.children(id),
            );
        }
    }
}

#[test]
fn cfg_10_add_node_name_shapes() {
    let mut rng = Rng::new(SEED ^ 10);
    let lens = [0usize, 1, 2, 47, 48, 49, 50, 51, 60, 200, 1000];
    for (i, len) in lens.iter().copied().enumerate() {
        // ASCII
        {
            let p = Pair::fresh();
            let name: Vec<u8> = (0..len).map(|k| b'a' + (k % 26) as u8).collect();
            let id = 1000 + i as i32;
            eq_i32(
                &format!("add_node ascii len {len}"),
                p.c.add(id, -1, &name, 3.5),
                p.r.add(id, -1, &name, 3.5),
            );
            assert_eq!(
                p.c.find_snap(id),
                p.r.find_snap(id),
                "stored name bytes, ascii len {len}"
            );
        }
        // high-bit bytes
        {
            let p = Pair::fresh();
            let name: Vec<u8> = (0..len).map(|k| 0x80 | ((k as u8) & 0x7f) | 0x80).collect();
            let name: Vec<u8> = name.into_iter().map(|b| if b == 0 { 0xFF } else { b }).collect();
            let id = 2000 + i as i32;
            eq_i32(
                &format!("add_node highbit len {len}"),
                p.c.add(id, -1, &name, 3.5),
                p.r.add(id, -1, &name, 3.5),
            );
            let cs = p.c.find_snap(id).expect("found");
            let rs = p.r.find_snap(id).expect("found");
            assert_eq!(cs, rs, "stored name bytes, highbit len {len}");
            // and the stored name must round-trip through process_string identically
            let mut cbuf: Vec<u8> = cs.name.to_vec();
            let mut rbuf: Vec<u8> = rs.name.to_vec();
            eq_i32(
                &format!("process_string(stored name len {len})"),
                p.c.process(&mut cbuf),
                p.r.process(&mut rbuf),
            );
        }
        // random
        for rep in 0..20 {
            let p = Pair::fresh();
            let name = rand_name(&mut rng, len);
            let id = 3000 + i as i32;
            eq_i32(
                &format!("add_node rand len {len} rep{rep}"),
                p.c.add(id, -1, &name, 3.5),
                p.r.add(id, -1, &name, 3.5),
            );
            assert_eq!(
                p.c.find_snap(id),
                p.r.find_snap(id),
                "stored name bytes, rand len {len} rep{rep}"
            );
        }
    }
    // embedded NUL in the source buffer: strncpy stops at it and zero-pads
    for cut in [0usize, 1, 10, 48, 49] {
        let p = Pair::fresh();
        let mut buf: Vec<u8> = vec![b'X'; 80];
        buf[cut] = 0;
        eq_i32(
            &format!("add_node nul@{cut}"),
            p.c.add_raw(9, -1, &buf, 1.0),
            p.r.add_raw(9, -1, &buf, 1.0),
        );
        assert_eq!(
            p.c.find_snap(9),
            p.r.find_snap(9),
            "stored name with source NUL at {cut}"
        );
    }
}

#[test]
fn cfg_11_add_node_value_shapes() {
    for (i, v) in interesting_doubles().into_iter().enumerate() {
        let p = Pair::fresh();
        let id = 1;
        eq_i32(
            &format!("add_node value {v:?}"),
            p.c.add(id, -1, b"v", v),
            p.r.add(id, -1, b"v", v),
        );
        assert_eq!(
            p.c.find_snap(id),
            p.r.find_snap(id),
            "#{i} stored value bits for {v:?}"
        );
        eq_bits(
            &format!("#{i} subtree_sum leaf value {v:?}"),
            p.c.subtree_bits(id),
            p.r.subtree_bits(id),
        );
        eq_i32(
            &format!("#{i} sdti of stored value {v:?}"),
            p.c.sdti(v),
            p.r.sdti(v),
        );
    }
    let mut rng = Rng::new(SEED ^ 11);
    for i in 0..500 {
        let p = Pair::fresh();
        let v = rng.any_f64();
        eq_i32(
            &format!("#{i} add_node rand-bits value"),
            p.c.add(1, -1, b"v", v),
            p.r.add(1, -1, b"v", v),
        );
        assert_eq!(p.c.find_snap(1), p.r.find_snap(1), "#{i} stored value bits");
    }
}

#[test]
fn cfg_12_add_node_fill_to_max() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 12);
    for k in 0..MAX_NODES {
        let id = k as i32 + 1;
        let pid = if k == 0 { -1 } else { rng.range_i32(1, id - 1) };
        let name = rand_name_upto(&mut rng, 0, 55);
        let v = rng.finite_f64(1000.0);
        let cv = p.c.add(id, pid, &name, v);
        let rv = p.r.add(id, pid, &name, v);
        eq_i32(&format!("add_node #{k}"), cv, rv);
        assert_eq!(cv, k as i32, "C add_node #{k} should return {k}");
    }
    for id in 1..=MAX_NODES as i32 {
        assert_eq!(p.c.find_snap(id), p.r.find_snap(id), "find({id}) at full");
        assert_eq!(
            p.c.find_rel_index(id, 1),
            p.r.find_rel_index(id, 1),
            "index({id}) at full"
        );
        eq_i32(
            &format!("children({id}) at full"),
            p.c.children(id),
            p.r.children(id),
        );
        eq_bits(
            &format!("subtree({id}) at full"),
            p.c.subtree_bits(id),
            p.r.subtree_bits(id),
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 13-16: get_children_count
// ---------------------------------------------------------------------------

#[test]
fn cfg_13_children_count_empty() {
    let p = Pair::fresh();
    let mut probes = interesting_ints();
    probes.extend([42, -42, 100, -100]);
    for id in probes {
        eq_i32(
            &format!("children({id}) on empty storage"),
            p.c.children(id),
            p.r.children(id),
        );
    }
}

#[test]
fn cfg_14_children_count_flat() {
    let mut rng = Rng::new(SEED ^ 14);
    for iter in 0..200 {
        let p = Pair::fresh();
        let parent = rng.next_i32();
        let n = rng.range_usize(1, 40);
        for k in 0..n {
            let id = rng.next_i32();
            let name = rand_name_upto(&mut rng, 0, 12);
            let v = rng.finite_f64(10.0);
            eq_i32(
                &format!("iter{iter} add#{k}"),
                p.c.add(id, parent, &name, v),
                p.r.add(id, parent, &name, v),
            );
        }
        for probe in [
            parent,
            parent.wrapping_add(1),
            parent.wrapping_sub(1),
            0,
            -1,
            i32::MIN,
            i32::MAX,
        ] {
            eq_i32(
                &format!("iter{iter} children({probe}) flat"),
                p.c.children(probe),
                p.r.children(probe),
            );
        }
    }
}

#[test]
fn cfg_15_children_count_random_forest() {
    let mut rng = Rng::new(SEED ^ 15);
    for iter in 0..200 {
        let p = Pair::fresh();
        let pool: Vec<i32> = (0..6).map(|_| rng.range_i32(-8, 8)).collect();
        let n = rng.range_usize(0, 60);
        for k in 0..n {
            let id = rng.range_i32(-10, 10);
            let pid = pool[rng.range_usize(0, pool.len() - 1)];
            let name = rand_name_upto(&mut rng, 0, 55);
            let v = rng.finite_f64(1e4);
            eq_i32(
                &format!("iter{iter} add#{k}"),
                p.c.add(id, pid, &name, v),
                p.r.add(id, pid, &name, v),
            );
        }
        for probe in -12..=12 {
            eq_i32(
                &format!("iter{iter} children({probe})"),
                p.c.children(probe),
                p.r.children(probe),
            );
        }
        for probe in [i32::MIN, i32::MAX, 1_000_000] {
            eq_i32(
                &format!("iter{iter} children({probe})"),
                p.c.children(probe),
                p.r.children(probe),
            );
        }
    }
}

#[test]
fn cfg_16_children_count_full() {
    let mut rng = Rng::new(SEED ^ 16);
    for iter in 0..20 {
        let p = Pair::fresh();
        let pool = [0i32, -1, 1, 2, i32::MIN, i32::MAX, 7, -7];
        for k in 0..MAX_NODES {
            let id = rng.range_i32(-5, 5);
            let pid = pool[rng.range_usize(0, pool.len() - 1)];
            let name = rand_name_upto(&mut rng, 0, 49);
            let v = rng.finite_f64(1.0);
            eq_i32(
                &format!("iter{iter} add#{k}"),
                p.c.add(id, pid, &name, v),
                p.r.add(id, pid, &name, v),
            );
        }
        // 101st add must be rejected identically
        eq_i32(
            &format!("iter{iter} add#100 (overflow)"),
            p.c.add(1, 1, b"x", 1.0),
            p.r.add(1, 1, b"x", 1.0),
        );
        for probe in pool {
            eq_i32(
                &format!("iter{iter} children({probe}) full"),
                p.c.children(probe),
                p.r.children(probe),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 17-23: calculate_subtree_sum
// ---------------------------------------------------------------------------

#[test]
fn cfg_17_subtree_sum_leaf() {
    for v in interesting_doubles() {
        let p = Pair::fresh();
        eq_i32("add", p.c.add(1, -1, b"leaf", v), p.r.add(1, -1, b"leaf", v));
        eq_bits(
            &format!("subtree_sum leaf {v:?}"),
            p.c.subtree_bits(1),
            p.r.subtree_bits(1),
        );
        eq_bits(
            "subtree_sum missing id",
            p.c.subtree_bits(2),
            p.r.subtree_bits(2),
        );
    }
}

#[test]
fn cfg_18_subtree_sum_tree() {
    let p = Pair::fresh();
    build_maxnmin_tree(&p);
    for id in -3..=10 {
        eq_bits(
            &format!("subtree_sum({id}) on maxnmin tree"),
            p.c.subtree_bits(id),
            p.r.subtree_bits(id),
        );
    }
    for id in [i32::MIN, i32::MAX, 0, -1] {
        eq_bits(
            &format!("subtree_sum({id}) extreme"),
            p.c.subtree_bits(id),
            p.r.subtree_bits(id),
        );
    }
}

#[test]
fn cfg_19_subtree_sum_random_forest() {
    let mut rng = Rng::new(SEED ^ 19);
    for iter in 0..400 {
        let p = Pair::fresh();
        let n = rng.range_usize(1, 60);
        for k in 0..n {
            let id = k as i32 + 1;
            // parent_id < id keeps the graph acyclic (see ERRORS.md row 12)
            let pid = rng.range_i32(-1, id - 1);
            let name = rand_name_upto(&mut rng, 0, 55);
            let v = if iter % 5 == 0 {
                rng.finite_f64(1e150)
            } else {
                rng.finite_f64(1e3)
            };
            eq_i32(
                &format!("iter{iter} add#{k}"),
                p.c.add(id, pid, &name, v),
                p.r.add(id, pid, &name, v),
            );
        }
        for id in -2..=(n as i32 + 2) {
            eq_bits(
                &format!("iter{iter} subtree_sum({id})"),
                p.c.subtree_bits(id),
                p.r.subtree_bits(id),
            );
            eq_i32(
                &format!("iter{iter} sdti(subtree_sum({id}))"),
                p.c.sdti(f64::from_bits(p.c.subtree_bits(id))),
                p.r.sdti(f64::from_bits(p.r.subtree_bits(id))),
            );
        }
    }
}

#[test]
fn cfg_20_subtree_sum_deep_chain() {
    let mut rng = Rng::new(SEED ^ 20);
    for iter in 0..20 {
        let p = Pair::fresh();
        let n = 90;
        for k in 0..n {
            let id = k as i32 + 1;
            let pid = if k == 0 { -1 } else { id - 1 };
            let v = rng.finite_f64(1e6);
            let name = rand_name_upto(&mut rng, 0, 49);
            eq_i32(
                &format!("iter{iter} add#{k}"),
                p.c.add(id, pid, &name, v),
                p.r.add(id, pid, &name, v),
            );
        }
        for id in 1..=n as i32 {
            eq_bits(
                &format!("iter{iter} deep subtree_sum({id})"),
                p.c.subtree_bits(id),
                p.r.subtree_bits(id),
            );
        }
    }
}

#[test]
fn cfg_21_subtree_sum_duplicate_ids() {
    let mut rng = Rng::new(SEED ^ 21);
    for iter in 0..300 {
        let p = Pair::fresh();
        // ids drawn from a small pool but always parent_id < id => acyclic
        let n = rng.range_usize(2, 40);
        for k in 0..n {
            let id = rng.range_i32(2, 8);
            let pid = rng.range_i32(-1, id - 1);
            let v = rng.finite_f64(50.0);
            let name = rand_name_upto(&mut rng, 0, 20);
            eq_i32(
                &format!("iter{iter} add#{k}"),
                p.c.add(id, pid, &name, v),
                p.r.add(id, pid, &name, v),
            );
        }
        for id in -2..=10 {
            eq_bits(
                &format!("iter{iter} dup subtree_sum({id})"),
                p.c.subtree_bits(id),
                p.r.subtree_bits(id),
            );
            eq_i32(
                &format!("iter{iter} dup children({id})"),
                p.c.children(id),
                p.r.children(id),
            );
            assert_eq!(
                p.c.find_snap(id),
                p.r.find_snap(id),
                "iter{iter} dup find({id})"
            );
        }
    }
}

#[test]
fn cfg_22_subtree_sum_orphans() {
    let mut rng = Rng::new(SEED ^ 22);
    for iter in 0..200 {
        let p = Pair::fresh();
        let n = rng.range_usize(1, 30);
        for k in 0..n {
            let id = k as i32 + 1;
            // parents far outside the id space -> orphans
            let pid = rng.range_i32(500, 600);
            let v = rng.finite_f64(9.0);
            eq_i32(
                &format!("iter{iter} add#{k}"),
                p.c.add(id, pid, b"orph", v),
                p.r.add(id, pid, b"orph", v),
            );
        }
        for id in [-1i32, 0, 1, 2, n as i32, n as i32 + 1, 550, i32::MIN, i32::MAX] {
            eq_bits(
                &format!("iter{iter} orphan subtree_sum({id})"),
                p.c.subtree_bits(id),
                p.r.subtree_bits(id),
            );
        }
    }
}

#[test]
fn cfg_23_subtree_sum_inf_nan() {
    let cases: Vec<(f64, f64, f64)> = vec![
        (1e308, 1e308, 1e308),
        (f64::INFINITY, f64::NEG_INFINITY, 1.0),
        (f64::NEG_INFINITY, f64::INFINITY, 0.0),
        (f64::NAN, 1.0, 2.0),
        (1.0, f64::NAN, 2.0),
        (f64::MAX, f64::MAX, f64::MAX),
        (-f64::MAX, -f64::MAX, -f64::MAX),
        (f64::INFINITY, f64::INFINITY, f64::INFINITY),
        (0.1, 0.2, 0.3),
        (1e-300, 1e300, -1e300),
    ];
    for (i, (a, b, c)) in cases.into_iter().enumerate() {
        let p = Pair::fresh();
        for (id, pid, v) in [(1i32, -1i32, a), (2, 1, b), (3, 1, c)] {
            eq_i32(
                &format!("#{i} add({id})"),
                p.c.add(id, pid, b"n", v),
                p.r.add(id, pid, b"n", v),
            );
        }
        for id in 1..=3 {
            eq_bits(
                &format!("#{i} inf/nan subtree_sum({id})"),
                p.c.subtree_bits(id),
                p.r.subtree_bits(id),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 24-31: maxnmin and state interaction
// ---------------------------------------------------------------------------

#[test]
fn cfg_24_maxnmin_residue_cross() {
    let p = Pair::fresh();
    for r1 in 0..6i32 {
        for r2 in 0..6i32 {
            for (p3, p4) in [(1i32, 0i32), (1, 1), (2, 2), (0, 3), (3, 5)] {
                let (a, b) = (r1, r2);
                eq_i32(
                    &format!("maxnmin({a},{b},{p3},{p4})"),
                    p.c.run(a, b, p3, p4),
                    p.r.run(a, b, p3, p4),
                );
                // same residues, larger magnitudes
                let (a2, b2) = (r1 + 60, r2 + 600);
                eq_i32(
                    &format!("maxnmin({a2},{b2},{p3},{p4})"),
                    p.c.run(a2, b2, p3, p4),
                    p.r.run(a2, b2, p3, p4),
                );
                // negative residues
                let (a3, b3) = (-r1, -r2);
                eq_i32(
                    &format!("maxnmin({a3},{b3},{p3},{p4})"),
                    p.c.run(a3, b3, p3, p4),
                    p.r.run(a3, b3, p3, p4),
                );
            }
        }
    }
}

#[test]
fn cfg_25_maxnmin_param4_residues() {
    let p = Pair::fresh();
    for p4 in -12..=12i32 {
        for p1 in [0i32, 1, 2, 3, 4, 5, -1, -5] {
            for p3 in [1i32, 2, -3] {
                eq_i32(
                    &format!("maxnmin({p1},{p1},{p3},{p4})"),
                    p.c.run(p1, p1, p3, p4),
                    p.r.run(p1, p1, p3, p4),
                );
            }
        }
    }
}

#[test]
fn cfg_26_maxnmin_param3_shapes() {
    let p = Pair::fresh();
    let p3s = [
        -2i32,
        -1,
        0,
        1,
        2,
        6,
        -6,
        1000,
        -1000,
        i32::MAX,
        i32::MAX - 1,
        i32::MIN,
        i32::MIN + 1,
        1 << 20,
        -(1 << 20),
    ];
    for p3 in p3s {
        for p1 in [0i32, 1, 5, -1, -5, i32::MAX, i32::MIN] {
            for p2 in [0i32, 2, 4, -2, i32::MAX, i32::MIN] {
                for p4 in [0i32, 1, 2, -1, i32::MAX, i32::MIN] {
                    eq_i32(
                        &format!("maxnmin({p1},{p2},{p3},{p4})"),
                        p.c.run(p1, p2, p3, p4),
                        p.r.run(p1, p2, p3, p4),
                    );
                }
            }
        }
    }
}

#[test]
fn cfg_27_maxnmin_random_full_range() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 27);
    for i in 0..20_000 {
        let (a, b, c, d) = (rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32());
        eq_i32(
            &format!("#{i} maxnmin({a},{b},{c},{d})"),
            p.c.run(a, b, c, d),
            p.r.run(a, b, c, d),
        );
    }
}

#[test]
fn cfg_28_maxnmin_random_small() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 28);
    for i in 0..20_000 {
        let a = rng.range_i32(-20, 20);
        let b = rng.range_i32(-20, 20);
        let c = rng.range_i32(-20, 20);
        let d = rng.range_i32(-20, 20);
        eq_i32(
            &format!("#{i} maxnmin({a},{b},{c},{d})"),
            p.c.run(a, b, c, d),
            p.r.run(a, b, c, d),
        );
    }
    // medium magnitudes too
    for i in 0..20_000 {
        let a = rng.range_i32(-100_000, 100_000);
        let b = rng.range_i32(-100_000, 100_000);
        let c = rng.range_i32(-100_000, 100_000);
        let d = rng.range_i32(-100_000, 100_000);
        eq_i32(
            &format!("med#{i} maxnmin({a},{b},{c},{d})"),
            p.c.run(a, b, c, d),
            p.r.run(a, b, c, d),
        );
    }
}

#[test]
fn cfg_29_maxnmin_boundary_cross() {
    let p = Pair::fresh();
    let vals = [
        i32::MIN,
        i32::MIN + 1,
        -6,
        -1,
        0,
        1,
        6,
        i32::MAX - 1,
        i32::MAX,
    ];
    for &a in &vals {
        for &b in &vals {
            for &c in &vals {
                for &d in &vals {
                    eq_i32(
                        &format!("maxnmin({a},{b},{c},{d})"),
                        p.c.run(a, b, c, d),
                        p.r.run(a, b, c, d),
                    );
                }
            }
        }
    }
}

#[test]
fn cfg_30_maxnmin_resets_state() {
    let mut rng = Rng::new(SEED ^ 30);
    for pre in [0usize, 1, 6, 7, 99, 100] {
        let p = Pair::fresh();
        for k in 0..pre {
            let id = rng.range_i32(-10, 10);
            let pid = rng.range_i32(-10, 10);
            let name = rand_name_upto(&mut rng, 0, 55);
            let v = rng.finite_f64(1e5);
            eq_i32(
                &format!("pre{pre} add#{k}"),
                p.c.add(id, pid, &name, v),
                p.r.add(id, pid, &name, v),
            );
        }
        for (a, b, c, d) in [
            (0i32, 0i32, 1i32, 0i32),
            (5, 3, 2, 1),
            (-1, -1, -1, -1),
            (i32::MAX, i32::MIN, 7, 4),
        ] {
            eq_i32(
                &format!("pre{pre} maxnmin({a},{b},{c},{d})"),
                p.c.run(a, b, c, d),
                p.r.run(a, b, c, d),
            );
        }
        // After maxnmin, node_count must be 6 in both: adding must land at index 6
        eq_i32(
            &format!("pre{pre} add after maxnmin"),
            p.c.add(77, 1, b"after", 1.25),
            p.r.add(77, 1, b"after", 1.25),
        );
        assert_eq!(
            p.c.find_snap(77),
            p.r.find_snap(77),
            "pre{pre} node added after maxnmin"
        );
        assert_eq!(
            p.c.find_rel_index(77, 1),
            p.r.find_rel_index(77, 1),
            "pre{pre} index of node added after maxnmin"
        );
    }
}

#[test]
fn cfg_31_state_after_maxnmin() {
    let p = Pair::fresh();
    eq_i32("maxnmin", p.c.run(3, 4, 5, 6), p.r.run(3, 4, 5, 6));
    for id in -3..=10 {
        assert_eq!(p.c.find_snap(id), p.r.find_snap(id), "post find({id})");
        assert_eq!(
            p.c.find_rel_index(id, 1),
            p.r.find_rel_index(id, 1),
            "post index({id})"
        );
        eq_i32(
            &format!("post children({id})"),
            p.c.children(id),
            p.r.children(id),
        );
        eq_bits(
            &format!("post subtree_sum({id})"),
            p.c.subtree_bits(id),
            p.r.subtree_bits(id),
        );
    }
    // the names maxnmin stored must hash identically through process_string
    for id in 1..=6 {
        let cs = p.c.find_snap(id).expect("node");
        let rs = p.r.find_snap(id).expect("node");
        assert_eq!(cs, rs, "post snapshot({id})");
        let mut cb: Vec<u8> = cs.name.to_vec();
        let mut rb: Vec<u8> = rs.name.to_vec();
        eq_i32(
            &format!("post process_string(name of {id})"),
            p.c.process(&mut cb),
            p.r.process(&mut rb),
        );
    }
}

// ---------------------------------------------------------------------------
// Row 32: randomized interleaved sequences over the whole export surface
// ---------------------------------------------------------------------------

#[test]
fn cfg_32_random_op_sequences() {
    let mut rng = Rng::new(SEED ^ 32);
    for seq in 0..300 {
        let p = Pair::fresh();
        // sentinel at index 0 with a unique id, so relative indices are comparable
        let sentinel = 0x7FFF_0001;
        eq_i32(
            &format!("seq{seq} sentinel"),
            p.c.add(sentinel, -1, b"sentinel", 0.0),
            p.r.add(sentinel, -1, b"sentinel", 0.0),
        );
        let mut next_id: i32 = 1;
        // `maxnmin` wipes the table (node_count = 0) and rebuilds ids 1..6, so
        // the sentinel disappears; track a base id that is actually present.
        let mut base_id: i32 = sentinel;
        for step in 0..60 {
            let what = format!("seq{seq} step{step}");
            match rng.next_u64() % 7 {
                0 => {
                    // add_node: parent_id < id keeps the forest acyclic
                    let id = next_id;
                    next_id = next_id.wrapping_add(rng.range_i32(1, 3));
                    let pid = rng.range_i32(-2, id - 1);
                    let len = rng.range_usize(0, 60);
                    let name = rand_name(&mut rng, len);
                    let v = if rng.next_u64() % 8 == 0 {
                        rng.any_f64()
                    } else {
                        rng.finite_f64(1e6)
                    };
                    eq_i32(
                        &format!("{what} add_node({id},{pid},len {len})"),
                        p.c.add(id, pid, &name, v),
                        p.r.add(id, pid, &name, v),
                    );
                }
                1 => {
                    let id = rng.range_i32(-4, next_id + 4);
                    p.assert_find(id, base_id, &what);
                }
                2 => {
                    let id = rng.range_i32(-4, next_id + 4);
                    eq_i32(
                        &format!("{what} get_children_count({id})"),
                        p.c.children(id),
                        p.r.children(id),
                    );
                }
                3 => {
                    let id = rng.range_i32(-4, next_id + 4);
                    eq_bits(
                        &format!("{what} calculate_subtree_sum({id})"),
                        p.c.subtree_bits(id),
                        p.r.subtree_bits(id),
                    );
                }
                4 => {
                    let len = rng.range_usize(0, 200);
                    let s = rand_name(&mut rng, len);
                    eq_i32(
                        &format!("{what} process_string(len {len})"),
                        p.c.process_str(&s),
                        p.r.process_str(&s),
                    );
                }
                5 => {
                    let d = if rng.next_u64() % 4 == 0 {
                        rng.any_f64()
                    } else {
                        rng.finite_f64(4e9)
                    };
                    eq_i32(
                        &format!("{what} safe_double_to_int({d:?})"),
                        p.c.sdti(d),
                        p.r.sdti(d),
                    );
                }
                _ => {
                    let (a, b, c, d) = if rng.next_u64() % 3 == 0 {
                        (rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32())
                    } else {
                        (
                            rng.range_i32(-30, 30),
                            rng.range_i32(-30, 30),
                            rng.range_i32(-30, 30),
                            rng.range_i32(-30, 30),
                        )
                    };
                    eq_i32(
                        &format!("{what} maxnmin({a},{b},{c},{d})"),
                        p.c.run(a, b, c, d),
                        p.r.run(a, b, c, d),
                    );
                    // maxnmin reset node_count to 6; keep the invariant that
                    // subsequently added ids stay above the tree's ids, and
                    // move the index base onto a node that still exists.
                    next_id = next_id.max(7);
                    base_id = 1;
                }
            }
        }
    }
}
