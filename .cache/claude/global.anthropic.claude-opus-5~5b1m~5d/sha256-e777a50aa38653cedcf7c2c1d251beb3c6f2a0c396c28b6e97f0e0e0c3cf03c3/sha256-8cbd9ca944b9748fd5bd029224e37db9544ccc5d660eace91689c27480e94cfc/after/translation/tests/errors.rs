// Phase C — error-path differential tests.
//
// One test per row of ERRORS.md. Rows whose C behaviour is a fatal signal
// (NULL dereference, unbounded recursion) are executed in a re-exec'd child
// process so the *signal* can be compared between the two libraries.

mod common;

use common::*;

const SEED: u64 = 0x5EED_1234_ABCD_0002;

// ---------------------------------------------------------------------------
// Rows 1 & 2 — add_node capacity check
// ---------------------------------------------------------------------------

#[test]
fn err_01_add_node_full() {
    let p = Pair::fresh();
    let mut rng = Rng::new(SEED ^ 1);

    for k in 0..MAX_NODES {
        let id = k as i32 + 1;
        let name = rand_name_upto(&mut rng, 0, 55);
        let cv = p.c.add(id, if k == 0 { -1 } else { id - 1 }, &name, k as f64);
        let rv = p.r.add(id, if k == 0 { -1 } else { id - 1 }, &name, k as f64);
        eq_i32(&format!("add_node #{k}"), cv, rv);
        assert_eq!(cv, k as i32, "row 2: add_node #{k} must return {k}");
    }
    // Row 2 boundary: the 100th add returned 99 (asserted above).
    // Row 1: every further add must return -1 and must not disturb the table.
    let before_c: Vec<Option<NodeSnap>> = (1..=MAX_NODES as i32).map(|i| p.c.find_snap(i)).collect();
    let before_r: Vec<Option<NodeSnap>> = (1..=MAX_NODES as i32).map(|i| p.r.find_snap(i)).collect();
    assert_eq!(before_c, before_r, "table state before overflow adds");

    for k in 0..25 {
        let name = rand_name_upto(&mut rng, 0, 60);
        let cv = p.c.add(9000 + k, 1, &name, 1.5);
        let rv = p.r.add(9000 + k, 1, &name, 1.5);
        eq_i32(&format!("row 1: overflow add #{k}"), cv, rv);
        assert_eq!(cv, -1, "row 1: C must return -1 when the table is full");
        assert_eq!(rv, -1, "row 1: Rust must return -1 when the table is full");
        assert_eq!(
            p.c.find_snap(9000 + k),
            None,
            "row 1: rejected node must not be stored (C)"
        );
        assert_eq!(
            p.r.find_snap(9000 + k),
            None,
            "row 1: rejected node must not be stored (Rust)"
        );
    }
    let after_c: Vec<Option<NodeSnap>> = (1..=MAX_NODES as i32).map(|i| p.c.find_snap(i)).collect();
    let after_r: Vec<Option<NodeSnap>> = (1..=MAX_NODES as i32).map(|i| p.r.find_snap(i)).collect();
    assert_eq!(before_c, after_c, "row 1: C table unchanged by rejected adds");
    assert_eq!(after_c, after_r, "row 1: table state after overflow adds");

    // and maxnmin must still work identically from the full state (it resets)
    for (a, b, c, d) in [(0i32, 0i32, 1i32, 0i32), (5, 5, 5, 5), (-1, -1, -1, -1)] {
        eq_i32(
            &format!("maxnmin from full table ({a},{b},{c},{d})"),
            p.c.run(a, b, c, d),
            p.r.run(a, b, c, d),
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 3 & 4 — name truncation / empty name
// ---------------------------------------------------------------------------

#[test]
fn err_03_add_node_name_truncation() {
    let mut rng = Rng::new(SEED ^ 3);
    for len in [49usize, 50, 51, 52, 64, 100, 4096] {
        for rep in 0..10 {
            let p = Pair::fresh();
            let name = rand_name(&mut rng, len);
            eq_i32(
                &format!("add_node(len {len}) rep{rep}"),
                p.c.add(1, -1, &name, 7.5),
                p.r.add(1, -1, &name, 7.5),
            );
            let cs = p.c.find_snap(1).expect("stored");
            let rs = p.r.find_snap(1).expect("stored");
            assert_eq!(cs, rs, "row 3: truncated name bytes (len {len}) rep{rep}");
            assert_eq!(
                cs.name[MAX_NAME_LEN - 1],
                0,
                "row 3: C must NUL-terminate name[49]"
            );
            assert_eq!(
                &cs.name[..MAX_NAME_LEN - 1],
                &name[..MAX_NAME_LEN - 1],
                "row 3: C keeps the first 49 bytes verbatim"
            );
        }
    }
}

#[test]
fn err_04_add_node_empty_name() {
    let p = Pair::fresh();
    eq_i32("add_node(\"\")", p.c.add(1, -1, b"", 4.25), p.r.add(1, -1, b"", 4.25));
    let cs = p.c.find_snap(1).expect("stored");
    let rs = p.r.find_snap(1).expect("stored");
    assert_eq!(cs, rs, "row 4: empty name must be fully zero-filled");
    assert_eq!(cs.name, [0u8; MAX_NAME_LEN], "row 4: C zero-fills the name");
    let mut cb: Vec<u8> = cs.name.to_vec();
    let mut rb: Vec<u8> = rs.name.to_vec();
    eq_i32(
        "row 4: process_string of the empty stored name",
        p.c.process(&mut cb),
        p.r.process(&mut rb),
    );
    assert_eq!(p.c.process(&mut cb), 0, "row 4: sum of empty string is 0");
}

// ---------------------------------------------------------------------------
// Rows 6-9 — find_node_by_id / get_children_count sentinels
// ---------------------------------------------------------------------------

#[test]
fn err_06_find_node_not_found() {
    // Row 6 (empty storage) + row 8 (extreme ids)
    {
        let p = Pair::fresh();
        let mut probes = interesting_ints();
        probes.extend([12345, -12345]);
        for id in probes {
            let cp = p.c.find_ptr(id);
            let rp = p.r.find_ptr(id);
            assert!(cp.is_null(), "row 6: C must return NULL on empty storage");
            assert_eq!(
                cp.is_null(),
                rp.is_null(),
                "row 6: find_node_by_id({id}) NULL-ness on empty storage"
            );
        }
    }
    // Row 6 with a populated table: ids that are simply absent
    let mut rng = Rng::new(SEED ^ 6);
    for iter in 0..200 {
        let p = Pair::fresh();
        let n = rng.range_usize(1, 20);
        for k in 0..n {
            let id = 100 + k as i32;
            eq_i32(
                &format!("iter{iter} add#{k}"),
                p.c.add(id, -1, b"n", 1.0),
                p.r.add(id, -1, b"n", 1.0),
            );
        }
        for id in [
            0i32,
            -1,
            1,
            99,
            100 + n as i32,
            i32::MIN,
            i32::MIN + 1,
            i32::MAX,
            i32::MAX - 1,
            rng.next_i32(),
        ] {
            let cp = p.c.find_ptr(id);
            let rp = p.r.find_ptr(id);
            assert_eq!(
                cp.is_null(),
                rp.is_null(),
                "iter{iter} row 6/8: find_node_by_id({id}) NULL-ness"
            );
            assert_eq!(p.c.find_snap(id), p.r.find_snap(id), "iter{iter} snapshot");
        }
    }
}

#[test]
fn err_07_find_node_inactive_unreachable() {
    // `add_node` always sets active = 1, so the `&& active` guard can never
    // reject via the public API. Assert that invariant holds for BOTH libraries
    // for every reachable node, in every state the API can produce.
    let mut rng = Rng::new(SEED ^ 7);
    for iter in 0..100 {
        let p = Pair::fresh();
        let n = rng.range_usize(1, 30);
        let mut ids = Vec::new();
        for _ in 0..n {
            let id = rng.next_i32();
            ids.push(id);
            let name = rand_name_upto(&mut rng, 0, 55);
            let pid = rng.next_i32();
            let v = rng.finite_f64(1.0);
            eq_i32("add", p.c.add(id, pid, &name, v), p.r.add(id, pid, &name, v));
        }
        for id in ids {
            let cs = p.c.find_snap(id).expect("row 7: C must find an added id");
            let rs = p.r.find_snap(id).expect("row 7: Rust must find an added id");
            assert_eq!(cs.active, 1, "row 7: C stores active = 1");
            assert_eq!(rs.active, 1, "row 7: Rust stores active = 1");
            assert_eq!(cs, rs, "iter{iter} row 7 snapshot for {id}");
        }
        // and after maxnmin, the six internal nodes are all active in both
        eq_i32("maxnmin", p.c.run(1, 2, 3, 4), p.r.run(1, 2, 3, 4));
        for id in 1..=6 {
            let cs = p.c.find_snap(id).expect("post-maxnmin node");
            let rs = p.r.find_snap(id).expect("post-maxnmin node");
            assert_eq!(cs.active, 1);
            assert_eq!(cs, rs, "row 7 post-maxnmin snapshot {id}");
        }
    }
}

#[test]
fn err_09_children_count_none() {
    let mut rng = Rng::new(SEED ^ 9);
    // empty storage
    {
        let p = Pair::fresh();
        for id in interesting_ints() {
            let cv = p.c.children(id);
            eq_i32(&format!("row 9: children({id}) empty"), cv, p.r.children(id));
            assert_eq!(cv, 0, "row 9: C returns 0, never an error code");
        }
    }
    // populated, but probing parent ids nobody uses
    for iter in 0..200 {
        let p = Pair::fresh();
        let n = rng.range_usize(1, 40);
        for k in 0..n {
            eq_i32(
                &format!("iter{iter} add#{k}"),
                p.c.add(k as i32, 7, b"n", 1.0),
                p.r.add(k as i32, 7, b"n", 1.0),
            );
        }
        for id in [8i32, -7, 0, -1, i32::MIN, i32::MAX, rng.next_i32()] {
            let cv = p.c.children(id);
            let rv = p.r.children(id);
            eq_i32(&format!("iter{iter} row 9: children({id})"), cv, rv);
            if id != 7 {
                assert!(cv >= 0, "row 9: count is never negative");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 10 & 11 — calculate_subtree_sum sentinel 0.0
// ---------------------------------------------------------------------------

#[test]
fn err_10_subtree_sum_missing() {
    // Row 11: empty storage
    {
        let p = Pair::fresh();
        for id in interesting_ints() {
            let cb = p.c.subtree_bits(id);
            eq_bits(
                &format!("row 11: subtree_sum({id}) on empty storage"),
                cb,
                p.r.subtree_bits(id),
            );
            assert_eq!(cb, 0.0f64.to_bits(), "row 11: C returns +0.0 sentinel");
        }
    }
    // Row 10: populated but the id is absent
    let mut rng = Rng::new(SEED ^ 10);
    for iter in 0..200 {
        let p = Pair::fresh();
        let n = rng.range_usize(1, 30);
        for k in 0..n {
            let id = k as i32 + 1;
            let pid = if k == 0 { -1 } else { rng.range_i32(-1, id - 1) };
            let v = rng.finite_f64(100.0);
            eq_i32(
                &format!("iter{iter} add#{k}"),
                p.c.add(id, pid, b"n", v),
                p.r.add(id, pid, b"n", v),
            );
        }
        for id in [
            0i32,
            -1,
            -2,
            n as i32 + 1,
            n as i32 + 100,
            i32::MIN,
            i32::MAX,
        ] {
            let cb = p.c.subtree_bits(id);
            eq_bits(
                &format!("iter{iter} row 10: subtree_sum({id})"),
                cb,
                p.r.subtree_bits(id),
            );
            assert_eq!(cb, 0.0f64.to_bits(), "row 10: C returns the +0.0 sentinel");
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 14-16 — process_string
// ---------------------------------------------------------------------------

#[test]
fn err_14_process_string_empty() {
    let p = Pair::fresh();
    let mut cb = vec![0u8];
    let mut rb = vec![0u8];
    let cv = p.c.process(&mut cb);
    let rv = p.r.process(&mut rb);
    eq_i32("row 14: process_string(\"\")", cv, rv);
    assert_eq!(cv, 0, "row 14: C returns 0 for the empty string");

    // a NUL preceded by junk still stops immediately when the pointer is at NUL
    let mut buf = vec![0u8, b'A', b'B', 0u8];
    let mut buf2 = buf.clone();
    eq_i32(
        "row 14: process_string(leading NUL)",
        p.c.process(&mut buf),
        p.r.process(&mut buf2),
    );
}

#[test]
fn err_15_process_string_negative_bytes() {
    let p = Pair::fresh();
    for b in 0x80u8..=0xFF {
        let cv = p.c.process_str(&[b]);
        let rv = p.r.process_str(&[b]);
        eq_i32(&format!("row 15: process_string([{b:#04x}])"), cv, rv);
        assert!(
            cv < 0,
            "row 15: byte {b:#04x} must contribute negatively (signed char), got {cv}"
        );
        assert_eq!(cv, (b as i8) as i32, "row 15: exact sign-extended value");
    }
    // mixed strings
    let mut rng = Rng::new(SEED ^ 15);
    for i in 0..2_000 {
        let len = rng.range_usize(1, 80);
        let s: Vec<u8> = (0..len).map(|_| rng.byte_nonzero()).collect();
        eq_i32(
            &format!("#{i} row 15: process_string mixed"),
            p.c.process_str(&s),
            p.r.process_str(&s),
        );
    }
}

#[test]
fn err_16_process_string_overflow() {
    let p = Pair::fresh();
    // 0x7F sums to +127 per byte; 2^24 * 127 > INT_MAX, so overflow the sum.
    // Use lengths that straddle the int boundary in both directions.
    for &(byte, len) in &[
        (0x7Fu8, 16_909_321usize), // 127 * len just over INT_MAX -> wraps
        (0x7F, 16_909_320),
        (0x7F, 16_909_319),
        (0x80, 16_777_217), // -128 * len just under INT_MIN -> wraps
        (0x80, 16_777_216), // -128 * len == INT_MIN exactly
        (0x80, 16_777_215),
        (0xFF, 40_000_000), // large negative, no wrap
    ] {
        let s = vec![byte; len];
        eq_i32(
            &format!("row 16: process_string({byte:#04x} x {len})"),
            p.c.process_str(&s),
            p.r.process_str(&s),
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 17-22 — safe_double_to_int clamps
// ---------------------------------------------------------------------------

#[test]
fn err_17_sdti_over_max() {
    let p = Pair::fresh();
    let max = i32::MAX as f64;
    // Row 17: strictly greater than (double)INT_MAX -> INT_MAX
    let over = [
        max + 1.0,
        max + 0.5,
        f64::from_bits(max.to_bits() + 1),
        2147483648.0,
        2147483647.5,
        2147483647.0000005,
        1e300,
        f64::MAX,
        f64::INFINITY,
        4e9,
        1e18,
    ];
    for d in over {
        let cv = p.c.sdti(d);
        eq_i32(&format!("row 17: sdti({d:?})"), cv, p.r.sdti(d));
        assert_eq!(cv, i32::MAX, "row 17: C clamps {d:?} to INT_MAX");
    }
    // Row 18: exactly (double)INT_MAX and one step inside -> not clamped
    for d in [max, max - 1.0, max - 0.5, f64::from_bits(max.to_bits() - 1)] {
        let cv = p.c.sdti(d);
        eq_i32(&format!("row 18: sdti({d:?})"), cv, p.r.sdti(d));
    }
    assert_eq!(p.c.sdti(max), i32::MAX, "row 18: (int)2147483647.0");
}

#[test]
fn err_19_sdti_under_min() {
    let p = Pair::fresh();
    let min = i32::MIN as f64;
    let under = [
        min - 1.0,
        min - 0.5,
        min - 2.0,
        f64::from_bits(min.to_bits() + 1), // more negative
        -2147483649.0,
        -1e300,
        f64::MIN,
        f64::NEG_INFINITY,
        -4e9,
        -1e18,
    ];
    for d in under {
        let cv = p.c.sdti(d);
        eq_i32(&format!("row 19: sdti({d:?})"), cv, p.r.sdti(d));
        assert_eq!(cv, i32::MIN, "row 19: C clamps {d:?} to INT_MIN");
    }
    // Row 20: exactly (double)INT_MIN and one step inside
    for d in [min, min + 1.0, min + 0.5, f64::from_bits(min.to_bits() - 1)] {
        let cv = p.c.sdti(d);
        eq_i32(&format!("row 20: sdti({d:?})"), cv, p.r.sdti(d));
    }
    assert_eq!(p.c.sdti(min), i32::MIN, "row 20: (int)-2147483648.0");
}

#[test]
fn err_21_sdti_nan() {
    let p = Pair::fresh();
    let nans = [
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF8_0000_0000_0000), // quiet NaN
        f64::from_bits(0xFFF8_0000_0000_0000), // negative quiet NaN
        f64::from_bits(0x7FF0_0000_0000_0001), // signalling NaN
        f64::from_bits(0xFFF0_0000_0000_0001),
        f64::from_bits(0x7FFF_FFFF_FFFF_FFFF),
        f64::from_bits(0xFFF8_0000_DEAD_BEEF),
        f64::from_bits(0x7FF8_0000_0000_0001),
    ];
    for d in nans {
        assert!(d.is_nan(), "test setup: {:#018x} should be NaN", d.to_bits());
        let cv = p.c.sdti(d);
        eq_i32(
            &format!("row 21: sdti(NaN {:#018x})", d.to_bits()),
            cv,
            p.r.sdti(d),
        );
        assert_eq!(cv, 0, "row 21: C returns 0 for NaN");
    }
    // and randomized NaN payloads
    let mut rng = Rng::new(SEED ^ 21);
    for _ in 0..5_000 {
        let bits = (rng.next_u64() & 0x800F_FFFF_FFFF_FFFF) | 0x7FF0_0000_0000_0000;
        let d = f64::from_bits(bits);
        if !d.is_nan() {
            continue; // pure infinity, covered by rows 17/19
        }
        let cv = p.c.sdti(d);
        eq_i32(&format!("row 21: sdti(NaN {bits:#018x})"), cv, p.r.sdti(d));
        assert_eq!(cv, 0, "row 21: C returns 0 for NaN {bits:#018x}");
    }
}

#[test]
fn err_22_sdti_tiny() {
    let p = Pair::fresh();
    let tiny = [
        0.0,
        -0.0,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        5e-324,
        -5e-324,
        1e-300,
        -1e-300,
        0.4999999999999999,
        -0.4999999999999999,
        0.5,
        -0.5,
        0.9999999999999999,
        -0.9999999999999999,
        1.0,
        -1.0,
        1.5,
        -1.5,
        -0.000001,
    ];
    for d in tiny {
        let cv = p.c.sdti(d);
        eq_i32(&format!("row 22: sdti({d:?})"), cv, p.r.sdti(d));
    }
    assert_eq!(p.c.sdti(-0.0), 0, "row 22: -0.0 -> 0");
    assert_eq!(p.c.sdti(-0.5), 0, "row 22: truncation toward zero");
    let mut rng = Rng::new(SEED ^ 22);
    for _ in 0..5_000 {
        let d = rng.finite_f64(1.0);
        eq_i32(&format!("row 22: sdti(tiny {d:?})"), p.c.sdti(d), p.r.sdti(d));
    }
}

// ---------------------------------------------------------------------------
// Rows 23-29 — maxnmin degenerate parameter paths
// ---------------------------------------------------------------------------

#[test]
fn err_23_maxnmin_negative_node_id() {
    let p = Pair::fresh();
    // param1 < 0 => param1 % 6 in -5..0 => node_id in -4..1
    for p1 in [-1i32, -2, -3, -4, -5, -6, -7, -11, -12, i32::MIN, i32::MIN + 1] {
        for p2 in [0i32, 1, 5] {
            for p3 in [1i32, 3] {
                for p4 in [0i32, 2] {
                    eq_i32(
                        &format!("row 23: maxnmin({p1},{p2},{p3},{p4})"),
                        p.c.run(p1, p2, p3, p4),
                        p.r.run(p1, p2, p3, p4),
                    );
                }
            }
        }
    }
    let mut rng = Rng::new(SEED ^ 23);
    for _ in 0..5_000 {
        let p1 = -(rng.range_i32(1, 1_000_000));
        let p2 = rng.range_i32(0, 1_000);
        let p3 = rng.range_i32(1, 1_000);
        let p4 = rng.range_i32(0, 1_000);
        eq_i32(
            &format!("row 23: maxnmin({p1},{p2},{p3},{p4})"),
            p.c.run(p1, p2, p3, p4),
            p.r.run(p1, p2, p3, p4),
        );
    }
}

#[test]
fn err_24_maxnmin_negative_second_id() {
    let p = Pair::fresh();
    for p2 in [-1i32, -2, -3, -4, -5, -6, -12, i32::MIN, i32::MIN + 1] {
        for p1 in [0i32, 1, 5, -1] {
            for p3 in [1i32, -3, 0] {
                for p4 in [0i32, 1, -1] {
                    eq_i32(
                        &format!("row 24: maxnmin({p1},{p2},{p3},{p4})"),
                        p.c.run(p1, p2, p3, p4),
                        p.r.run(p1, p2, p3, p4),
                    );
                }
            }
        }
    }
}

#[test]
fn err_25_maxnmin_div_by_zero() {
    let p = Pair::fresh();
    // param3 == -1 => divisor (param3 + 1) == 0 => +/-inf or NaN
    for p1 in interesting_ints() {
        for p2 in [0i32, 1, -1, 5, -5, i32::MAX, i32::MIN] {
            for p4 in [0i32, 1, -1, 2, i32::MAX, i32::MIN] {
                eq_i32(
                    &format!("row 25: maxnmin({p1},{p2},-1,{p4})"),
                    p.c.run(p1, p2, -1, p4),
                    p.r.run(p1, p2, -1, p4),
                );
            }
        }
    }
    // the sum being 0 too makes 0.0/0.0 -> NaN, then * param4 -> NaN -> 0
    for (p1, p2) in [(0i32, 0i32), (5, -5), (-7, 7), (i32::MIN, i32::MIN)] {
        for p4 in [0i32, 1, -1, 100, i32::MAX, i32::MIN] {
            eq_i32(
                &format!("row 25 NaN: maxnmin({p1},{p2},-1,{p4})"),
                p.c.run(p1, p2, -1, p4),
                p.r.run(p1, p2, -1, p4),
            );
        }
    }
}

#[test]
fn err_26_maxnmin_param3_int_max() {
    let p = Pair::fresh();
    for p3 in [i32::MAX, i32::MAX - 1, i32::MIN, i32::MIN + 1] {
        for p1 in interesting_ints() {
            for p2 in [0i32, 3, -3, i32::MAX, i32::MIN] {
                for p4 in [0i32, 1, -1, i32::MAX, i32::MIN] {
                    eq_i32(
                        &format!("row 26: maxnmin({p1},{p2},{p3},{p4})"),
                        p.c.run(p1, p2, p3, p4),
                        p.r.run(p1, p2, p3, p4),
                    );
                }
            }
        }
    }
}

#[test]
fn err_27_maxnmin_sum_overflow() {
    let p = Pair::fresh();
    let extremes = [
        i32::MAX,
        i32::MAX - 1,
        i32::MAX / 2 + 1,
        i32::MIN,
        i32::MIN + 1,
        i32::MIN / 2 - 1,
    ];
    for &p1 in &extremes {
        for &p2 in &extremes {
            for p3 in [0i32, 1, -1, 2, 6, i32::MAX, i32::MIN] {
                for p4 in [0i32, 1, -1, 3, i32::MAX, i32::MIN] {
                    eq_i32(
                        &format!("row 27: maxnmin({p1},{p2},{p3},{p4})"),
                        p.c.run(p1, p2, p3, p4),
                        p.r.run(p1, p2, p3, p4),
                    );
                }
            }
        }
    }
}

#[test]
fn err_28_maxnmin_negative_parent() {
    let p = Pair::fresh();
    // param4 < 0 => param4 % 3 in -2..0 => parent probe in -1..1
    for p4 in [-1i32, -2, -3, -4, -5, -6, -9, i32::MIN, i32::MIN + 1] {
        for p1 in [0i32, 1, 2, 5, -1] {
            for p2 in [0i32, 1, 3, -1] {
                for p3 in [1i32, 2, -1, 0] {
                    eq_i32(
                        &format!("row 28: maxnmin({p1},{p2},{p3},{p4})"),
                        p.c.run(p1, p2, p3, p4),
                        p.r.run(p1, p2, p3, p4),
                    );
                }
            }
        }
    }
    // and directly: get_children_count on the exact probes maxnmin would use
    eq_i32("maxnmin setup", p.c.run(0, 0, 1, 0), p.r.run(0, 0, 1, 0));
    for probe in [-2i32, -1, 0, 1, 2, 3, 4] {
        eq_i32(
            &format!("row 28: get_children_count({probe})"),
            p.c.children(probe),
            p.r.children(probe),
        );
    }
}

#[test]
fn err_29_maxnmin_result_overflow() {
    let p = Pair::fresh();
    // value * param3 grows past INT_MAX/INT_MIN => per-term clamping, then the
    // accumulation itself wraps.
    let p3s = [
        100_000_000i32,
        -100_000_000,
        2_000_000_000,
        -2_000_000_000,
        i32::MAX,
        i32::MIN,
        1_000_000,
    ];
    for p3 in p3s {
        for p1 in [0i32, 1, 2, 3, 4, 5] {
            for p2 in [0i32, 1, 2, 3, 4, 5] {
                for p4 in [0i32, 1, 2, i32::MAX, i32::MIN, -1] {
                    eq_i32(
                        &format!("row 29: maxnmin({p1},{p2},{p3},{p4})"),
                        p.c.run(p1, p2, p3, p4),
                        p.r.run(p1, p2, p3, p4),
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 30 — every int is in range across the FFI boundary ("out-of-range enum")
// ---------------------------------------------------------------------------

#[test]
fn err_30_int_boundary_cross_product() {
    let p = Pair::fresh();
    let vals = [
        i32::MIN,
        i32::MIN + 1,
        -1,
        0,
        1,
        i32::MAX - 1,
        i32::MAX,
    ];
    for &a in &vals {
        for &b in &vals {
            for &c in &vals {
                for &d in &vals {
                    eq_i32(
                        &format!("row 30: maxnmin({a},{b},{c},{d})"),
                        p.c.run(a, b, c, d),
                        p.r.run(a, b, c, d),
                    );
                }
            }
        }
    }
    // the single-int entry points, on both empty and populated storage
    for &v in &vals {
        eq_i32(
            &format!("row 30: children({v}) empty"),
            p.c.children(v),
            p.r.children(v),
        );
        eq_bits(
            &format!("row 30: subtree({v}) empty"),
            p.c.subtree_bits(v),
            p.r.subtree_bits(v),
        );
        let cp = p.c.find_ptr(v);
        let rp = p.r.find_ptr(v);
        assert_eq!(cp.is_null(), rp.is_null(), "row 30: find({v}) NULL-ness");
    }
    for &a in &vals {
        for &b in &vals {
            let q = Pair::fresh();
            eq_i32(
                &format!("row 30: add_node({a},{b})"),
                q.c.add(a, b, b"x", 1.0),
                q.r.add(a, b, b"x", 1.0),
            );
            q.assert_find(a, a, "row 30");
            eq_i32(
                &format!("row 30: children({b})"),
                q.c.children(b),
                q.r.children(b),
            );
            // `a == b` would make the node its own parent, i.e. the unbounded
            // recursion of ERRORS.md row 12 — covered separately in a child
            // process, so it must not be triggered in-process here.
            if a != b {
                eq_bits(
                    &format!("row 30: subtree({a})"),
                    q.c.subtree_bits(a),
                    q.r.subtree_bits(a),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 5, 12, 13 — fatal signals, compared in a child process
// ---------------------------------------------------------------------------

mod fatal {
    use super::common::*;
    use std::os::unix::process::ExitStatusExt;
    use std::process::{Command, Stdio};

    /// Env var names used to steer the re-exec'd worker.
    const LIB_VAR: &str = "DIFF_FATAL_LIB";
    const KIND_VAR: &str = "DIFF_FATAL_KIND";

    fn run_worker(which: &str, kind: &str) -> (Option<i32>, Option<i32>) {
        let exe = std::env::current_exe().expect("current_exe");
        let out = Command::new(exe)
            .args(["fatal::fatal_worker", "--exact", "--ignored", "--nocapture"])
            .env(LIB_VAR, which)
            .env(KIND_VAR, kind)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .expect("spawn worker");
        (out.status.code(), out.status.signal())
    }

    #[track_caller]
    fn assert_same_fatal(kind: &str) {
        let (c_code, c_sig) = run_worker("c", kind);
        let (r_code, r_sig) = run_worker("rust", kind);
        assert_eq!(
            (c_code, c_sig),
            (r_code, r_sig),
            "{kind}: C exited (code {c_code:?}, signal {c_sig:?}) but \
             Rust exited (code {r_code:?}, signal {r_sig:?})"
        );
        assert!(
            c_sig.is_some(),
            "{kind}: expected the C library to die from a signal, got code {c_code:?}"
        );
    }

    /// Row 5 — `add_node(.., NULL, ..)`: `strncpy` dereferences NULL.
    #[test]
    fn err_05_add_node_null_name() {
        assert_same_fatal("add_node_null_name");
    }

    /// Row 12 — self-parent node: `calculate_subtree_sum` recurses forever.
    #[test]
    fn err_12_subtree_sum_self_cycle() {
        assert_same_fatal("subtree_self_cycle");
    }

    /// Row 13 — `process_string(NULL)`: `*str` dereferences NULL.
    #[test]
    fn err_13_process_string_null() {
        assert_same_fatal("process_string_null");
    }

    /// The worker. Ignored so it never runs as part of a normal test pass; the
    /// tests above invoke it explicitly via `--ignored --exact`.
    #[test]
    #[ignore = "re-exec'd on purpose by the fatal-signal differential tests"]
    fn fatal_worker() {
        let which = std::env::var(LIB_VAR).unwrap_or_default();
        let kind = std::env::var(KIND_VAR).unwrap_or_default();
        if which.is_empty() || kind.is_empty() {
            return;
        }
        let p = Pair::fresh();
        let lib: &Lib = match which.as_str() {
            "c" => &p.c,
            "rust" => &p.r,
            other => panic!("unknown lib {other}"),
        };
        match kind.as_str() {
            "add_node_null_name" => {
                let r = unsafe { (lib.add_node)(1, -1, std::ptr::null(), 1.0) };
                println!("survived add_node(NULL) -> {r}");
            }
            "process_string_null" => {
                let r = unsafe { (lib.process_string)(std::ptr::null_mut()) };
                println!("survived process_string(NULL) -> {r}");
            }
            "subtree_self_cycle" => {
                // id == parent_id: the child scan finds the node itself.
                assert_eq!(lib.add(7, 7, b"cycle", 1.0), 0);
                let r = lib.subtree_bits(7);
                println!("survived subtree cycle -> {r:#018x}");
            }
            other => panic!("unknown kind {other}"),
        }
        // If we get here the operation did NOT crash; exit 0 so the comparison
        // still sees identical (code 0, no signal) for both libraries.
    }
}
