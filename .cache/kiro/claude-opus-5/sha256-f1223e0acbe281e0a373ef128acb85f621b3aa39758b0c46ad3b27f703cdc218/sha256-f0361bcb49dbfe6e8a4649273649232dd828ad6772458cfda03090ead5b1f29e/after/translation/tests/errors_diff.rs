// Phase C -- error-path differential tests. One test (or one clearly-labelled
// block) per row of ERRORS.md. Each asserts C and Rust return the SAME
// error/sentinel value, not merely that both "failed somehow".

mod common;
use common::*;

// ---------------------------------------------------------------------------
// E01..E06 -- add_node
// ---------------------------------------------------------------------------

/// E01: `node_count >= MAX_NODES` -> returns -1, and the rejection must not
/// disturb the stored state (further adds keep returning -1, lookups still work).
#[test]
fn e01_add_node_rejects_past_capacity() {
    let p = fresh_pair();
    let mut rng = Rng::new(0xE01);
    for i in 0..MAX_NODES {
        let nlen = rng.below(50) as usize;
        let name = rng.ascii_name(nlen);
        let v = rng.tame_f64();
        let (a, b) = (
            p.c.add_node(i as i32, -1, &name, v),
            p.r.add_node(i as i32, -1, &name, v),
        );
        eq_i32("E01/fill", i, a, b);
    }
    // the 101st .. 110th adds must all be rejected with exactly -1
    for k in 0..10 {
        let name = rng.ascii_name(8);
        let (a, b) = (
            p.c.add_node(1000 + k, 5, &name, 3.5),
            p.r.add_node(1000 + k, 5, &name, 3.5),
        );
        eq_i32("E01/reject", k, a, b);
        assert_eq!(a, -1, "[E01] expected -1 from a full store, got {a}");
        // the rejected node must not be findable, and state must be intact
        eq_node("E01/not-stored", k, &p.c.find_snap(1000 + k), &p.r.find_snap(1000 + k));
        assert!(p.c.find_snap(1000 + k).is_none());
        eq_node("E01/intact", k, &p.c.find_snap(99), &p.r.find_snap(99));
        eq_i32("E01/children", k, p.c.children(-1), p.r.children(-1));
    }
}

/// E02/E03/E04: `MAX_NAME_LEN` truncation boundary, and the empty name.
#[test]
fn e02_e03_e04_add_node_name_truncation_boundary() {
    let mut rng = Rng::new(0xE02);
    for len in [0usize, 1, 48, 49, 50, 51, 200] {
        let p = fresh_pair();
        let name = rng.full_byte_name(len);
        let (a, b) = (
            p.c.add_node(1, -1, &name, 2.0),
            p.r.add_node(1, -1, &name, 2.0),
        );
        eq_i32("E02", len, a, b);
        let cs = p.c.find_snap(1).unwrap();
        let rs = p.r.find_snap(1).unwrap();
        assert_eq!(cs, rs, "[E02] stored name differs for source length {len}");

        // the exact C contract: min(len, 49) bytes copied, byte 49 always NUL
        let kept = len.min(MAX_NAME_LEN - 1);
        assert_eq!(&cs.name[..kept], &name[..kept], "[E02] first {kept} bytes");
        assert_eq!(cs.name[MAX_NAME_LEN - 1], 0, "[E02] name[49] must be NUL");
        for k in kept..MAX_NAME_LEN {
            assert_eq!(cs.name[k], 0, "[E02] byte {k} must be NUL-padded");
        }
        eq_i32(
            "E02/process",
            len,
            p.c.process_name_in_place(1).unwrap(),
            p.r.process_name_in_place(1).unwrap(),
        );
    }
}

/// E05: NaN / infinite `value` is accepted verbatim, not rejected.
/// E06: `id` / `parent_id` at the `int` extremes are accepted verbatim.
#[test]
fn e05_e06_add_node_accepts_extreme_inputs() {
    let extremes = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    let values = [
        f64::NAN,
        -f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        0.0,
        -0.0,
        f64::MAX,
        f64::MIN,
    ];
    for &id in &extremes {
        for &pid in &extremes {
            for &v in &values {
                let p = fresh_pair();
                let (a, b) = (
                    p.c.add_node(id, pid, b"x", v),
                    p.r.add_node(id, pid, b"x", v),
                );
                eq_i32("E05/E06", (id, pid, v.to_bits()), a, b);
                assert_eq!(a, 0, "[E06] first add must return index 0");
                eq_node("E05/E06", (id, pid), &p.c.find_snap(id), &p.r.find_snap(id));
                eq_i32("E06/children", (id, pid), p.c.children(pid), p.r.children(pid));
                // `id == pid` makes the node its own parent, which is the
                // documented infinite-recursion case (both libraries would
                // stack-overflow identically); skip the sum for those.
                if id != pid {
                    eq_bits("E05/sum", (id, pid), p.c.subtree_bits(id), p.r.subtree_bits(id));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E07..E10 -- find_node_by_id rejections
// ---------------------------------------------------------------------------

/// E07: pristine library (`node_count == 0`) -> every lookup is NULL.
#[test]
fn e07_find_on_empty_storage_is_null() {
    let p = fresh_pair();
    for id in [i32::MIN, -1, 0, 1, 6, 100, i32::MAX] {
        let (cp, rp) = (p.c.find_raw(id), p.r.find_raw(id));
        assert!(cp.is_null(), "[E07] C returned non-NULL for id {id} on empty storage");
        assert!(rp.is_null(), "[E07] Rust returned non-NULL for id {id} on empty storage");
        // the dependent rejections that follow from it
        eq_bits("E07/sum", id, p.c.subtree_bits(id), p.r.subtree_bits(id));
        assert_eq!(
            p.c.subtree_bits(id),
            0.0f64.to_bits(),
            "[E07] expected +0.0 exactly"
        );
        eq_i32("E07/children", id, p.c.children(id), p.r.children(id));
        assert_eq!(p.c.children(id), 0);
    }
}

/// E08: id present in no node -> NULL on both sides.
#[test]
fn e08_find_absent_id_is_null() {
    let p = fresh_pair();
    let mut rng = Rng::new(0xE08);
    for k in 0..40 {
        p.c.add_node(k * 2, -1, b"n", rng.tame_f64());
    }
    // NOTE: re-run the same sequence on Rust with the same values
    let p = fresh_pair();
    let mut rng = Rng::new(0xE08);
    for k in 0..40 {
        let v = rng.tame_f64();
        let (a, b) = (
            p.c.add_node(k * 2, -1, b"n", v),
            p.r.add_node(k * 2, -1, b"n", v),
        );
        eq_i32("E08/setup", k, a, b);
    }
    for id in (1..80).step_by(2).chain([-1, -100, 1000, i32::MIN, i32::MAX]) {
        let (cp, rp) = (p.c.find_raw(id), p.r.find_raw(id));
        assert_eq!(cp.is_null(), rp.is_null(), "[E08] NULL-ness differs for {id}");
        assert!(cp.is_null(), "[E08] odd id {id} should be absent");
        eq_bits("E08/sum", id, p.c.subtree_bits(id), p.r.subtree_bits(id));
        assert_eq!(p.c.subtree_bits(id), 0.0f64.to_bits());
    }
}

/// E09: the node exists but `active == 0` -> skipped, so NULL.
#[test]
fn e09_find_inactive_node_is_null() {
    let p = fresh_pair();
    p.c.add_node(42, -1, b"only", 7.5);
    p.r.add_node(42, -1, b"only", 7.5);
    eq_node("E09/before", 42, &p.c.find_snap(42), &p.r.find_snap(42));

    assert!(p.c.set_active(42, 0));
    assert!(p.r.set_active(42, 0));

    let (cp, rp) = (p.c.find_raw(42), p.r.find_raw(42));
    assert!(cp.is_null(), "[E09] C still found the inactive node");
    assert!(rp.is_null(), "[E09] Rust still found the inactive node");
    eq_bits("E09/sum", 42, p.c.subtree_bits(42), p.r.subtree_bits(42));
    assert_eq!(p.c.subtree_bits(42), 0.0f64.to_bits());
    eq_i32("E09/children", -1, p.c.children(-1), p.r.children(-1));
    assert_eq!(p.c.children(-1), 0, "[E09] inactive node must not be counted");

    // any non-zero `active` (not just 1) makes it findable again
    for act in [1i32, 2, -1, i32::MIN, i32::MAX] {
        let p = fresh_pair();
        p.c.add_node(42, -1, b"only", 7.5);
        p.r.add_node(42, -1, b"only", 7.5);
        p.c.set_active(42, act);
        p.r.set_active(42, act);
        eq_node("E09/reactivate", act, &p.c.find_snap(42), &p.r.find_snap(42));
        eq_i32("E09/children", act, p.c.children(-1), p.r.children(-1));
    }
}

/// E10: duplicate ids -> the FIRST (lowest-index) active match is returned.
#[test]
fn e10_find_duplicate_returns_first_match() {
    let p = fresh_pair();
    for k in 0..5i32 {
        let name = format!("dup{k}").into_bytes();
        p.c.add_node(9, -1, &name, k as f64);
        p.r.add_node(9, -1, &name, k as f64);
    }
    eq_node("E10", 9, &p.c.find_snap(9), &p.r.find_snap(9));
    let s = p.c.find_snap(9).unwrap();
    assert_eq!(&s.name[..4], b"dup0", "[E10] expected the first match");
    assert_eq!(s.value_bits, 0.0f64.to_bits());

    // deactivating the first must promote the second, identically on both sides
    for expect in 1..5i32 {
        assert!(p.c.set_active(9, 0));
        assert!(p.r.set_active(9, 0));
        eq_node("E10/promote", expect, &p.c.find_snap(9), &p.r.find_snap(9));
        let s = p.c.find_snap(9).unwrap();
        assert_eq!(&s.name[..4], format!("dup{expect}").as_bytes());
    }
    // after all five are inactive: NULL
    assert!(p.c.set_active(9, 0));
    assert!(p.r.set_active(9, 0));
    assert!(p.c.find_raw(9).is_null() && p.r.find_raw(9).is_null());
    assert!(!p.c.set_active(9, 1) && !p.r.set_active(9, 1));
}

// ---------------------------------------------------------------------------
// E11..E12 -- get_children_count "nothing found"
// ---------------------------------------------------------------------------

#[test]
fn e11_e12_children_count_zero_cases() {
    let p = fresh_pair();
    // E11: nothing matches
    for id in [i32::MIN, -5, 0, 1, i32::MAX] {
        eq_i32("E11/empty", id, p.c.children(id), p.r.children(id));
        assert_eq!(p.c.children(id), 0);
    }
    for k in 0..6i32 {
        p.c.add_node(k + 1, 1, b"c", 1.0);
        p.r.add_node(k + 1, 1, b"c", 1.0);
    }
    for id in [i32::MIN, -5, 0, 2, 7, i32::MAX] {
        eq_i32("E11/no-match", id, p.c.children(id), p.r.children(id));
        assert_eq!(p.c.children(id), 0, "[E11] id {id} should have no children");
    }
    eq_i32("E11/match", 1, p.c.children(1), p.r.children(1));
    assert_eq!(p.c.children(1), 6);

    // E12: all candidate children deactivated -> 0, indistinguishable from E11
    for k in 0..6i32 {
        p.c.set_active(k + 1, 0);
        p.r.set_active(k + 1, 0);
    }
    eq_i32("E12", 1, p.c.children(1), p.r.children(1));
    assert_eq!(p.c.children(1), 0, "[E12] inactive children must not be counted");
}

// ---------------------------------------------------------------------------
// E13..E14 -- calculate_subtree_sum
// ---------------------------------------------------------------------------

/// E13: node not found -> exactly `+0.0` (bit pattern 0x0000000000000000).
#[test]
fn e13_subtree_sum_not_found_is_positive_zero() {
    let p = fresh_pair();
    // empty storage
    for id in [i32::MIN, -1, 0, 1, i32::MAX] {
        eq_bits("E13/empty", id, p.c.subtree_bits(id), p.r.subtree_bits(id));
        assert_eq!(p.c.subtree_bits(id), 0x0000_0000_0000_0000);
    }
    // present-but-inactive, and absent
    p.c.add_node(5, -1, b"n", -3.5);
    p.r.add_node(5, -1, b"n", -3.5);
    p.c.set_active(5, 0);
    p.r.set_active(5, 0);
    for id in [5, 6, -1] {
        eq_bits("E13/inactive", id, p.c.subtree_bits(id), p.r.subtree_bits(id));
        assert_eq!(
            p.c.subtree_bits(id),
            0x0000_0000_0000_0000,
            "[E13] must be +0.0, not -0.0 and not the node's value"
        );
    }
}

/// E14: NaN / inf in a descendant propagates; it is not rejected.
#[test]
fn e14_subtree_sum_propagates_nan_and_inf() {
    for v in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, f64::MAX] {
        let p = fresh_pair();
        let build = |l: &Lib| {
            l.add_node(1, -1, b"root", 1.0);
            l.add_node(2, 1, b"mid", 2.0);
            l.add_node(3, 2, b"deep", v);
            l.add_node(4, 1, b"sib", v);
        };
        build(&p.c);
        build(&p.r);
        for id in [1, 2, 3, 4] {
            eq_bits("E14", (v.to_bits(), id), p.c.subtree_bits(id), p.r.subtree_bits(id));
        }
        let cs = unsafe { (p.c.calculate_subtree_sum)(1) };
        assert!(
            !cs.is_finite(),
            "[E14] expected NaN/inf to reach the root sum, got {cs}"
        );
        eq_i32("E14/d2i", v.to_bits(), p.c.safe_d2i(cs), p.r.safe_d2i(cs));
    }
}

// ---------------------------------------------------------------------------
// E15..E17 -- process_string
// ---------------------------------------------------------------------------

#[test]
fn e15_process_string_empty_is_zero() {
    let p = fresh_pair();
    eq_i32("E15", 0, p.c.process_string(b""), p.r.process_string(b""));
    assert_eq!(p.c.process_string(b""), 0);
    // and a stored empty name, reached the same way maxnmin does
    p.c.add_node(1, -1, b"", 0.0);
    p.r.add_node(1, -1, b"", 0.0);
    eq_i32(
        "E15/in-place",
        1,
        p.c.process_name_in_place(1).unwrap(),
        p.r.process_name_in_place(1).unwrap(),
    );
    assert_eq!(p.c.process_name_in_place(1).unwrap(), 0);
}

#[test]
fn e16_process_string_signed_char_negative_sums() {
    let p = fresh_pair();
    // 0x80 == -128 as a signed char
    for b in [0x80u8, 0x81, 0xFE, 0xFF] {
        let s = [b];
        eq_i32("E16", b, p.c.process_string(&s), p.r.process_string(&s));
        assert!(
            p.c.process_string(&s) < 0,
            "[E16] byte {b:#02x} must contribute a NEGATIVE value (signed char)"
        );
    }
    // a whole string of high-bit bytes: strongly negative total
    let s = vec![0xFFu8; 100];
    eq_i32("E16/many", 100, p.c.process_string(&s), p.r.process_string(&s));
    assert_eq!(p.c.process_string(&s), -100);
}

#[test]
fn e17_process_string_accumulator_wraps() {
    let p = fresh_pair();
    // positive overflow
    let s = vec![0x7Fu8; 16_909_321];
    let (a, b) = (p.c.process_string(&s), p.r.process_string(&s));
    eq_i32("E17/pos", s.len(), a, b);
    assert!(a < 0, "[E17] positive overflow must wrap to a negative int, got {a}");
    // negative overflow
    let s = vec![0x80u8; 16_777_217];
    let (a, b) = (p.c.process_string(&s), p.r.process_string(&s));
    eq_i32("E17/neg", s.len(), a, b);
    assert!(a > 0, "[E17] negative overflow must wrap to a positive int, got {a}");
}

// ---------------------------------------------------------------------------
// E18..E26 -- safe_double_to_int, every branch and every boundary
// ---------------------------------------------------------------------------

#[test]
fn e18_to_e26_safe_double_to_int_all_branches() {
    let p = fresh_pair();
    // (input, expected C result, row)
    let cases: &[(f64, i32, &str)] = &[
        // E18 -- above INT_MAX
        (2147483648.0, i32::MAX, "E18"),
        (2147483647.5, i32::MAX, "E18"),
        (1e300, i32::MAX, "E18"),
        (f64::MAX, i32::MAX, "E18"),
        // E19 -- +inf takes the same branch
        (f64::INFINITY, i32::MAX, "E19"),
        // E20 -- below INT_MIN
        (-2147483649.0, i32::MIN, "E20"),
        (-2147483648.5, i32::MIN, "E20"),
        (-1e300, i32::MIN, "E20"),
        (f64::MIN, i32::MIN, "E20"),
        // E21 -- -inf takes the same branch
        (f64::NEG_INFINITY, i32::MIN, "E21"),
        // E22 -- NaN falls through BOTH range compares, then `d != d` -> 0
        (f64::NAN, 0, "E22"),
        (-f64::NAN, 0, "E22"),
        (f64::from_bits(0x7FF0_0000_0000_0001), 0, "E22"), // sNaN
        (f64::from_bits(0xFFF8_0000_DEAD_BEEF), 0, "E22"), // qNaN, payload
        // E23/E24 -- exactly ON the boundary: the clamp does NOT fire
        (2147483647.0, i32::MAX, "E23"),
        (-2147483648.0, i32::MIN, "E24"),
        // one representable step inside
        (2147483646.9999998, 2147483646, "E23"),
        (-2147483647.9999995, -2147483647, "E24"),
        // E25 -- truncation toward zero, not floor
        (-2.7, -2, "E25"),
        (-0.5, 0, "E25"),
        (-0.9999999, 0, "E25"),
        (2.7, 2, "E25"),
        (0.9999999, 0, "E25"),
        // E26 -- signed zero
        (-0.0, 0, "E26"),
        (0.0, 0, "E26"),
        (f64::MIN_POSITIVE, 0, "E26"),
        (-f64::MIN_POSITIVE, 0, "E26"),
    ];
    for &(d, expect, row) in cases {
        let (a, b) = (p.c.safe_d2i(d), p.r.safe_d2i(d));
        eq_i32(row, (d, d.to_bits()), a, b);
        assert_eq!(
            a, expect,
            "[{row}] C returned {a} for {d} (bits {:#018x}), test expected {expect}",
            d.to_bits()
        );
    }
    // one ULP either side of each clamp threshold -- "one step past the range"
    for base in [i32::MAX as f64, i32::MIN as f64] {
        for delta in [-2i64, -1, 0, 1, 2] {
            let d = if delta >= 0 {
                f64::from_bits(base.to_bits().wrapping_add(delta as u64))
            } else {
                f64::from_bits(base.to_bits().wrapping_sub((-delta) as u64))
            };
            eq_i32("E18/E20-ulp", (base, delta), p.c.safe_d2i(d), p.r.safe_d2i(d));
        }
    }
}

// ---------------------------------------------------------------------------
// E27..E38 -- maxnmin rejection / edge paths
// ---------------------------------------------------------------------------

/// E27/E28: negative `param1`/`param2` make `node_id <= 0`, so the corresponding
/// block is skipped entirely.
#[test]
fn e27_e28_maxnmin_null_node_blocks_skipped() {
    let p = fresh_pair();
    // param1 in -1..-5 => node_id in 0..-4 => NULL => first block contributes 0.
    // Compare against a param1 whose residue also gives a NULL node: the answer
    // must be identical, proving the block really was skipped.
    for p1a in [-1i32, -2, -3, -4, -5] {
        for p1b in [-7i32, -8, -9, -10, -11] {
            // both NULL-producing; only equal when the residues coincide
            let (p2, p3, p4) = (0, 1, 0);
            let ra = p.c.maxnmin(p1a, p2, p3, p4);
            let rb = p.r.maxnmin(p1a, p2, p3, p4);
            eq_i32("E27", (p1a, p1b), ra, rb);
        }
    }
    // exhaustive over the residues, both signs, for BOTH params
    for p1 in -12..=12i32 {
        for p2 in -12..=12i32 {
            for p3 in [-2i32, -1, 0, 1, 5] {
                for p4 in [-2i32, -1, 0, 1, 2] {
                    eq_i32(
                        "E27/E28",
                        (p1, p2, p3, p4),
                        p.c.maxnmin(p1, p2, p3, p4),
                        p.r.maxnmin(p1, p2, p3, p4),
                    );
                }
            }
        }
    }
    // and the sentinel property: for a NULL first node the first block adds
    // nothing, so param1 = -1 and param1 = -7 (same residue class -1) agree
    for p2 in -6..=6 {
        for p3 in [-1i32, 0, 3] {
            let a = p.c.maxnmin(-1, p2, p3, 0);
            let b = p.c.maxnmin(-7, p2, p3, 0);
            assert_eq!(a, b, "[E27] same residue must give the same answer");
            let ra = p.r.maxnmin(-1, p2, p3, 0);
            let rb = p.r.maxnmin(-7, p2, p3, 0);
            eq_i32("E27/residue", (p2, p3), a, ra);
            eq_i32("E27/residue", (p2, p3), b, rb);
        }
    }
}

/// E29/E30: `param3 == -1` makes the denominator `0.0` -> ±inf, or NaN for 0/0.
#[test]
fn e29_e30_maxnmin_division_by_zero() {
    let p = fresh_pair();
    // 0.0 / 0.0 -> NaN -> safe_double_to_int -> 0 (E30)
    for p4 in [-3i32, -1, 0, 1, 7, i32::MIN, i32::MAX] {
        for (p1, p2) in [(0i32, 0i32), (5, -5), (-5, 5), (12, -12), (i32::MIN, i32::MIN)] {
            // param1 + param2 == 0 (the last pair wraps to 0)
            let sum = p1.wrapping_add(p2);
            let (a, b) = (p.c.maxnmin(p1, p2, -1, p4), p.r.maxnmin(p1, p2, -1, p4));
            eq_i32("E30", (p1, p2, p4, sum), a, b);
        }
    }
    // non-zero / 0.0 -> ±inf -> clamps to INT_MAX / INT_MIN, or NaN if param4==0
    let mut rng = Rng::new(0xE29);
    for _ in 0..20_000 {
        let p1 = rng.next_i32();
        let p2 = rng.next_i32();
        let p4 = rng.next_i32();
        eq_i32(
            "E29",
            (p1, p2, p4),
            p.c.maxnmin(p1, p2, -1, p4),
            p.r.maxnmin(p1, p2, -1, p4),
        );
    }
    // inf * 0 -> NaN -> 0
    for p1 in 1..=6i32 {
        eq_i32("E29/inf-times-zero", p1, p.c.maxnmin(p1, 1, -1, 0), p.r.maxnmin(p1, 1, -1, 0));
    }
}

/// E31/E32: signed overflow in `param3 + 1` and in `param1 + param2`.
#[test]
fn e31_e32_maxnmin_signed_overflow_wraps() {
    let p = fresh_pair();
    // param3 == INT_MAX => param3 + 1 wraps to INT_MIN
    for p1 in [0i32, 1, 5, -1, -5, i32::MIN, i32::MAX] {
        for p2 in [0i32, 1, 5, -1, -5, i32::MIN, i32::MAX] {
            for p4 in [0i32, 1, -1, i32::MIN, i32::MAX] {
                for p3 in [i32::MAX, i32::MAX - 1, i32::MIN, i32::MIN + 1] {
                    eq_i32(
                        "E31/E32",
                        (p1, p2, p3, p4),
                        p.c.maxnmin(p1, p2, p3, p4),
                        p.r.maxnmin(p1, p2, p3, p4),
                    );
                }
            }
        }
    }
    // param1 + param2 overflow specifically
    let overflow_pairs = [
        (i32::MAX, i32::MAX),
        (i32::MAX, 1),
        (i32::MIN, i32::MIN),
        (i32::MIN, -1),
        (i32::MAX, i32::MIN),
        (2_000_000_000, 2_000_000_000),
        (-2_000_000_000, -2_000_000_000),
    ];
    for (p1, p2) in overflow_pairs {
        for p3 in [-2i32, -1, 0, 1, 2, i32::MAX, i32::MIN] {
            for p4 in [-2i32, -1, 0, 1, 2, i32::MAX, i32::MIN] {
                eq_i32(
                    "E32",
                    (p1, p2, p3, p4),
                    p.c.maxnmin(p1, p2, p3, p4),
                    p.r.maxnmin(p1, p2, p3, p4),
                );
            }
        }
    }
}

/// E33/E34: `parent_id = (param4 % 3) + 1` can be `-1` (matching the seeded
/// root's `parent_id`) or `0` (matching nothing).
#[test]
fn e33_e34_maxnmin_parent_id_negative_and_zero() {
    let p = fresh_pair();
    // param4 = -2 => parent_id = -1 => root counted => +10
    // param4 = -1 => parent_id =  0 => nothing      => +0
    // The two must differ by exactly 10 with everything else held fixed.
    for p1 in [0i32, 1, 2, 3, 4, 5, -1, -2] {
        for p3 in [0i32, 1, 2, -3] {
            let a = p.c.maxnmin(p1, p1, p3, -2);
            let b = p.c.maxnmin(p1, p1, p3, -1);
            let ra = p.r.maxnmin(p1, p1, p3, -2);
            let rb = p.r.maxnmin(p1, p1, p3, -1);
            eq_i32("E33", (p1, p3), a, ra);
            eq_i32("E34", (p1, p3), b, rb);
        }
    }
    // sweep all residues of param4 including negatives, over many param3
    for p4 in -30..=30i32 {
        for p3 in [-1i32, 0, 1, 2, 3] {
            for p1 in -6..=6i32 {
                eq_i32(
                    "E33/E34",
                    (p1, p3, p4),
                    p.c.maxnmin(p1, p1, p3, p4),
                    p.r.maxnmin(p1, p1, p3, p4),
                );
            }
        }
    }
    // isolate the get_children_count(-1) == 1 fact through the low-level API
    let p = fresh_pair();
    p.c.maxnmin(0, 0, 1, 0);
    p.r.maxnmin(0, 0, 1, 0);
    eq_i32("E33/root", -1, p.c.children(-1), p.r.children(-1));
    assert_eq!(p.c.children(-1), 1, "[E33] the seeded root has parent_id -1");
    eq_i32("E34/zero", 0, p.c.children(0), p.r.children(0));
    assert_eq!(p.c.children(0), 0);
}

/// E35: `second_node->value * param3` overflowing `int` clamps to INT_MAX/MIN.
#[test]
fn e35_maxnmin_value_times_param3_clamps() {
    let p = fresh_pair();
    for p3 in [i32::MAX, i32::MIN, 1_000_000_000, -1_000_000_000, 204522253] {
        for p2 in 0..6i32 {
            for p1 in [0i32, 3] {
                eq_i32(
                    "E35",
                    (p3, p2, p1),
                    p.c.maxnmin(p1, p2, p3, 0),
                    p.r.maxnmin(p1, p2, p3, 0),
                );
            }
        }
    }
    // confirm the clamp really is reached: 10.5 * INT_MAX > INT_MAX
    eq_i32(
        "E35/clamp",
        0,
        p.c.safe_d2i(10.5 * i32::MAX as f64),
        p.r.safe_d2i(10.5 * i32::MAX as f64),
    );
    assert_eq!(p.c.safe_d2i(10.5 * i32::MAX as f64), i32::MAX);
}

/// E36: all four parameters at the `int` extremes simultaneously.
#[test]
fn e36_maxnmin_all_extremes() {
    let p = fresh_pair();
    let ex = [i32::MIN, i32::MIN + 1, -2, -1, 0, 1, 2, i32::MAX - 1, i32::MAX];
    for &a in &ex {
        for &b in &ex {
            for &c in &ex {
                for &d in &ex {
                    eq_i32(
                        "E36",
                        (a, b, c, d),
                        p.c.maxnmin(a, b, c, d),
                        p.r.maxnmin(a, b, c, d),
                    );
                }
            }
        }
    }
}

/// E37/E38: `maxnmin` resets `node_count` at entry; `add_node` afterwards sees
/// the six seeds and appends at index 6.
#[test]
fn e37_e38_maxnmin_state_reset_and_leftovers() {
    let mut rng = Rng::new(0xE37);
    for _ in 0..200 {
        // E37: pre-loading arbitrary nodes must not change maxnmin's answer
        let clean = fresh_pair();
        let dirty = fresh_pair();
        let n = rng.below(99) as usize;
        for i in 0..n {
            let nlen = rng.below(50) as usize;
            let name = rng.ascii_name(nlen);
            let v = rng.tame_f64();
            dirty.c.add_node(rng.next_i32(), rng.next_i32(), &name, v);
            dirty.r.add_node(i as i32, -1, &name, v);
        }
        let (a, b, c, d) = (
            rng.next_i32(),
            rng.next_i32(),
            rng.range_i32(-5, 5),
            rng.next_i32(),
        );
        let cc = clean.c.maxnmin(a, b, c, d);
        let cd = dirty.c.maxnmin(a, b, c, d);
        let rc = clean.r.maxnmin(a, b, c, d);
        let rd = dirty.r.maxnmin(a, b, c, d);
        assert_eq!(cc, cd, "[E37] C: maxnmin depends on prior state ({a},{b},{c},{d})");
        assert_eq!(rc, rd, "[E37] Rust: maxnmin depends on prior state ({a},{b},{c},{d})");
        eq_i32("E37", (a, b, c, d), cc, rc);

        // E38: append after maxnmin lands at index 6
        let nlen = rng.below(60) as usize;
        let name = rng.full_byte_name(nlen);
        let v = rng.mixed_f64();
        let (x, y) = (
            clean.c.add_node(500, 1, &name, v),
            clean.r.add_node(500, 1, &name, v),
        );
        eq_i32("E38", (a, b, c, d), x, y);
        assert_eq!(x, 6, "[E38] expected index 6 after maxnmin, got {x}");
        eq_node("E38/node", 500, &clean.c.find_snap(500), &clean.r.find_snap(500));
        eq_i32("E38/children", 1, clean.c.children(1), clean.r.children(1));
    }
}

// ---------------------------------------------------------------------------
// Generic boundaries: NULL pointers across the FFI boundary.
//
// Both libraries dereference the pointer unconditionally, so both must die with
// the SAME signal. The deref is executed in a forked child (a re-exec of this
// test binary) so the crash does not take the test process down with it.
// ---------------------------------------------------------------------------

const CRASH_ENV: &str = "HARVEST_CRASH_CASE";

#[test]
#[ignore = "internal crash harness, spawned by null_pointer_crash_parity"]
fn crash_harness() {
    let case = std::env::var(CRASH_ENV).expect("crash harness invoked without a case");
    let p = fresh_pair();
    let (which, op) = case.split_once(':').unwrap();
    let lib = match which {
        "c" => &p.c,
        "rust" => &p.r,
        _ => panic!("bad lib {which}"),
    };
    match op {
        "process_string_null" => unsafe {
            let r = (lib.process_string)(std::ptr::null_mut());
            println!("SURVIVED {r}");
        },
        "add_node_null_name" => unsafe {
            let r = (lib.add_node)(1, -1, std::ptr::null(), 1.0);
            println!("SURVIVED {r}");
        },
        _ => panic!("bad op {op}"),
    }
}

fn run_crash_case(case: &str) -> (Option<i32>, Option<i32>) {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().expect("current_exe");
    let out = std::process::Command::new(exe)
        .args(["--exact", "crash_harness", "--ignored", "--nocapture"])
        .env(CRASH_ENV, case)
        .output()
        .expect("spawn crash harness");
    (out.status.code(), out.status.signal())
}

#[test]
fn null_pointer_crash_parity() {
    for op in ["process_string_null", "add_node_null_name"] {
        let (c_code, c_sig) = run_crash_case(&format!("c:{op}"));
        let (r_code, r_sig) = run_crash_case(&format!("rust:{op}"));
        assert_eq!(
            (c_code, c_sig),
            (r_code, r_sig),
            "[NULL-ptr] {op}: C exited with (code {c_code:?}, signal {c_sig:?}) but \
             Rust exited with (code {r_code:?}, signal {r_sig:?})"
        );
        assert_eq!(
            c_sig,
            Some(11),
            "[NULL-ptr] {op}: expected both to die with SIGSEGV, got signal {c_sig:?}"
        );
    }
}
