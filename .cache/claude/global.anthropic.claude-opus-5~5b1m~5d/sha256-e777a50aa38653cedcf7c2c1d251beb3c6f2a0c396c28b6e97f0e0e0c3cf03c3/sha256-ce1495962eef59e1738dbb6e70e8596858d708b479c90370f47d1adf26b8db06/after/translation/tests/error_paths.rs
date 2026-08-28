// Phase C -- error-path differential tests.
//
// One test per row of `ERRORS.md` (E1..E38; E39/E40 are documented as
// non-executable identical UB). Every test constructs the exact invalid input
// or condition, calls BOTH shared objects through their exported C symbols and
// asserts they produce the SAME sentinel / error value (not merely "both
// failed").

mod common;
use common::*;

const INT_MAX: i32 = i32::MAX;
const INT_MIN: i32 = i32::MIN;

/// Fills a pristine store with exactly `MAX_NODES` acyclic nodes.
fn fill_to_capacity(p: &Pair, row: &str) {
    for k in 0..MAX_NODES as i32 {
        let idx = both_add(
            p,
            row,
            k + 1,
            if k == 0 { -1 } else { k },
            format!("n{k}").as_bytes(),
            k as f64 + 0.5,
        );
        assert_eq!(idx, k, "[{row}] expected slot {k}");
    }
}

// ---------------------------------------------------------------------------
// E1 / E2 -- add_node past MAX_NODES
// ---------------------------------------------------------------------------

#[test]
fn e1_add_node_rejects_when_full() {
    let p = Pair::fresh();
    fill_to_capacity(&p, "E1");
    // the 101st insert
    let rc = both_add(&p, "E1", 12345, -1, b"overflow", 1.0);
    assert_eq!(rc, -1, "[E1] C must return -1 when node_count >= MAX_NODES");
    // nothing was stored, and the existing store is untouched
    both_query(&p, "E1", 12345);
    for k in 0..MAX_NODES as i32 {
        both_query(&p, "E1", k + 1);
    }
}

#[test]
fn e2_add_node_keeps_rejecting_when_full() {
    let mut rng = Rng::new(0xE002);
    let p = Pair::fresh();
    fill_to_capacity(&p, "E2");
    for k in 0..50 {
        let rc = both_add(
            &p,
            "E2",
            rng.next_i32(),
            rng.next_i32(),
            format!("over{k}").as_bytes(),
            rng.next_f64_bits(),
        );
        assert_eq!(rc, -1, "[E2] insert #{} past capacity must fail", 101 + k);
    }
    // ... and the store is still exactly the 100 original nodes
    for k in 0..MAX_NODES as i32 {
        both_query(&p, "E2", k + 1);
    }
    both_query(&p, "E2", MAX_NODES as i32 + 1);
}

// ---------------------------------------------------------------------------
// E3 / E4 / E5 -- name length boundaries around MAX_NAME_LEN-1 == 49
// ---------------------------------------------------------------------------

#[test]
fn e3_add_node_truncates_oversized_name() {
    let mut rng = Rng::new(0xE003);
    for len in [50usize, 51, 52, 64, 99, 100, 500, 4096] {
        for trial in 0..10 {
            let p = Pair::fresh();
            let name: Vec<u8> = (0..len)
                .map(|i| match trial {
                    0 => b'a' + (i % 26) as u8,
                    1 => 0xff,
                    _ => rng.nonzero_byte(),
                })
                .collect();
            let rc = both_add(&p, "E3", 1, -1, &name, 1.0);
            assert_eq!(rc, 0);
            let (fc, _) = both_find(&p, "E3", 1);
            let s = unsafe { NodeSnap::read(fc) };
            assert_eq!(
                &s.name[..49],
                &name[..49],
                "[E3] first 49 bytes must be copied verbatim"
            );
            assert_eq!(s.name[49], 0, "[E3] name[49] must be forced to NUL");
        }
    }
}

#[test]
fn e4_add_node_name_exactly_at_limit() {
    for len in [47usize, 48, 49] {
        let p = Pair::fresh();
        let name: Vec<u8> = (0..len).map(|i| b'A' + (i % 26) as u8).collect();
        assert_eq!(both_add(&p, "E4", 1, -1, &name, 2.0), 0);
        let (fc, _) = both_find(&p, "E4", 1);
        let s = unsafe { NodeSnap::read(fc) };
        assert_eq!(&s.name[..len], &name[..]);
        for i in len..MAX_NAME_LEN {
            assert_eq!(s.name[i], 0, "[E4] byte {i} must be zero padding");
        }
    }
}

#[test]
fn e5_add_node_empty_name() {
    let p = Pair::fresh();
    assert_eq!(both_add(&p, "E5", 1, -1, b"", 3.0), 0);
    let (fc, _) = both_find(&p, "E5", 1);
    let s = unsafe { NodeSnap::read(fc) };
    assert_eq!(s.name, [0u8; MAX_NAME_LEN], "[E5] name must be all zeros");
    both_query(&p, "E5", 1);
    // and process_string on that empty stored name returns 0 in both
    let (fc, fr) = both_find(&p, "E5", 1);
    let a = unsafe { (p.c.process_string)((*fc).name.as_mut_ptr()) };
    let b = unsafe { (p.rust.process_string)((*fr).name.as_mut_ptr()) };
    eq_i32("E5", "process_string(empty stored name)", a, b);
    assert_eq!(a, 0);
}

// ---------------------------------------------------------------------------
// E7 / E8 -- add_node performs NO validation of id / parent_id / value
// ---------------------------------------------------------------------------

#[test]
fn e7_add_node_extreme_ids_are_accepted() {
    let ids = [0, 1, -1, -2, INT_MAX, INT_MIN, INT_MAX - 1, INT_MIN + 1];
    for &id in &ids {
        for &parent in &ids {
            if id == parent {
                continue; // self-parent => infinite recursion (E39)
            }
            let p = Pair::fresh();
            assert_eq!(both_add(&p, "E7", id, parent, b"x", 1.0), 0);
            both_query_nosum(&p, "E7", id);
            both_query_nosum(&p, "E7", parent);
            both_subtree(&p, "E7", id);
            both_subtree(&p, "E7", parent);
        }
    }
}

#[test]
fn e8_add_node_nonfinite_values_are_accepted() {
    let vals = [
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7ff0_0000_0000_0001), // sNaN
        f64::INFINITY,
        f64::NEG_INFINITY,
        -0.0,
        f64::MAX,
        f64::MIN,
    ];
    for &v in &vals {
        let p = Pair::fresh();
        assert_eq!(both_add(&p, "E8", 1, -1, b"v", v), 0);
        let (fc, _) = both_find(&p, "E8", 1);
        let s = unsafe { NodeSnap::read(fc) };
        assert_eq!(
            s.value_bits,
            v.to_bits(),
            "[E8] value must be stored verbatim"
        );
        both_query(&p, "E8", 1);
    }
}

// ---------------------------------------------------------------------------
// E9 .. E13 -- find_node_by_id returning NULL
// ---------------------------------------------------------------------------

#[test]
fn e9_find_on_empty_store_returns_null() {
    let mut rng = Rng::new(0xE009);
    let p = Pair::fresh();
    for id in [0, 1, -1, 6, 100, INT_MAX, INT_MIN] {
        let (fc, fr) = both_find(&p, "E9", id);
        assert!(fc.is_null(), "[E9] C must return NULL on an empty store");
        assert!(fr.is_null(), "[E9] Rust must return NULL on an empty store");
    }
    for _ in 0..2000 {
        let (fc, fr) = both_find(&p, "E9", rng.next_i32());
        assert!(fc.is_null() && fr.is_null());
    }
}

#[test]
fn e10_find_absent_id_returns_null() {
    let mut rng = Rng::new(0xE010);
    let p = Pair::fresh();
    for k in 0..10 {
        both_add(&p, "E10", k * 2, -1, b"even", k as f64);
    }
    for k in 0..10 {
        let (fc, fr) = both_find(&p, "E10", k * 2 + 1);
        assert!(fc.is_null(), "[E10] odd ids were never inserted");
        assert!(fr.is_null());
    }
    for _ in 0..2000 {
        let id = rng.next_i32() | 1; // odd => never inserted
        let (fc, fr) = both_find(&p, "E10", id);
        assert!(fc.is_null() && fr.is_null(), "[E10] id={id}");
    }
}

#[test]
fn e11_find_inactive_id_returns_null() {
    let p = Pair::fresh();
    for k in 0..5 {
        both_add(&p, "E11", k + 1, -1, b"n", k as f64);
    }
    for k in 0..5 {
        assert!(both_mutate(&p, "E11", k + 1, &|n| unsafe { (*n).active = 0 }));
        let (fc, fr) = both_find(&p, "E11", k + 1);
        assert!(fc.is_null(), "[E11] inactive node must not be found (C)");
        assert!(fr.is_null(), "[E11] inactive node must not be found (Rust)");
        // ... and it disappears from the other two scans as well
        let cc = both_children(&p, "E11", -1);
        assert_eq!(cc, 4 - k, "[E11] children of -1 after {} deactivations", k + 1);
        both_subtree(&p, "E11", k + 1);
    }
}

#[test]
fn e12_find_skips_inactive_duplicate() {
    let p = Pair::fresh();
    both_add(&p, "E12", 100, -1, b"anchor", 0.0);
    both_add(&p, "E12", 42, -1, b"first", 1.0);
    both_add(&p, "E12", 42, -1, b"second", 2.0);
    both_add(&p, "E12", 42, -1, b"third", 3.0);
    // first match wins
    both_delta(&p, "E12", 100, 42);
    both_query_nosum(&p, "E12", 42);
    // deactivate the first -> the second must win (slot delta 2 instead of 1)
    both_mutate(&p, "E12", 42, &|n| unsafe { (*n).active = 0 });
    both_delta(&p, "E12", 100, 42);
    both_query_nosum(&p, "E12", 42);
    both_mutate(&p, "E12", 42, &|n| unsafe { (*n).active = 0 });
    both_delta(&p, "E12", 100, 42);
    both_mutate(&p, "E12", 42, &|n| unsafe { (*n).active = 0 });
    // all three inactive now => NULL
    let (fc, fr) = both_find(&p, "E12", 42);
    assert!(fc.is_null() && fr.is_null(), "[E12] all duplicates inactive");
    both_delta(&p, "E12", 100, 42);
}

#[test]
fn e13_find_extreme_absent_ids() {
    let p = Pair::fresh();
    both_add(&p, "E13", 5, -1, b"five", 1.0);
    for id in [0, INT_MAX, INT_MIN, INT_MAX - 1, INT_MIN + 1, -1, 4, 6] {
        let (fc, fr) = both_find(&p, "E13", id);
        assert!(fc.is_null() && fr.is_null(), "[E13] id={id} must be absent");
        both_query(&p, "E13", id);
    }
}

// ---------------------------------------------------------------------------
// E14 / E15 -- get_children_count returning 0
// ---------------------------------------------------------------------------

#[test]
fn e14_children_count_zero_when_nothing_matches() {
    let mut rng = Rng::new(0xE014);
    // empty store
    let p = Pair::fresh();
    for id in [0, 1, -1, INT_MAX, INT_MIN] {
        assert_eq!(both_children(&p, "E14", id), 0);
    }
    // non-empty store, parent ids that match nothing
    for k in 0..10 {
        both_add(&p, "E14", k + 1, -1, b"n", 1.0);
    }
    for id in [0, 1, 2, INT_MAX, INT_MIN, 1000] {
        assert_eq!(both_children(&p, "E14", id), 0, "[E14] parent_id={id}");
    }
    assert_eq!(both_children(&p, "E14", -1), 10);
    for _ in 0..2000 {
        let id = rng.next_i32();
        let c = both_children(&p, "E14", id);
        if id != -1 {
            assert_eq!(c, 0);
        }
    }
}

#[test]
fn e15_children_count_ignores_inactive() {
    let p = Pair::fresh();
    both_add(&p, "E15", 1, -1, b"root", 1.0);
    for k in 0..7 {
        both_add(&p, "E15", 10 + k, 1, b"kid", 1.0);
    }
    assert_eq!(both_children(&p, "E15", 1), 7);
    for k in 0..7 {
        both_mutate(&p, "E15", 10 + k, &|n| unsafe { (*n).active = 0 });
        assert_eq!(both_children(&p, "E15", 1), 6 - k);
    }
    assert_eq!(both_children(&p, "E15", 1), 0, "[E15] all children inactive");
    both_query(&p, "E15", 1);
}

// ---------------------------------------------------------------------------
// E16 .. E19 -- calculate_subtree_sum
// ---------------------------------------------------------------------------

#[test]
fn e16_subtree_sum_zero_for_missing_node() {
    let mut rng = Rng::new(0xE016);
    let p = Pair::fresh();
    for id in [0, 1, -1, INT_MAX, INT_MIN] {
        let s = both_subtree(&p, "E16", id);
        assert_eq!(s.to_bits(), 0.0f64.to_bits(), "[E16] must be exactly +0.0");
    }
    both_add(&p, "E16", 1, -1, b"root", -7.5);
    both_mutate(&p, "E16", 1, &|n| unsafe { (*n).active = 0 });
    let s = both_subtree(&p, "E16", 1);
    assert_eq!(s.to_bits(), 0.0f64.to_bits(), "[E16] inactive => +0.0");
    for _ in 0..2000 {
        let id = rng.next_i32();
        if id == 1 {
            continue;
        }
        let s = both_subtree(&p, "E16", id);
        assert_eq!(s.to_bits(), 0.0f64.to_bits());
    }
}

#[test]
fn e17_subtree_sum_propagates_nan() {
    for bits in [
        0x7ff8_0000_0000_0000u64,
        0xfff8_0000_0000_0000,
        0x7ff8_0000_dead_beef,
        0x7ff0_0000_0000_0001,
    ] {
        let p = Pair::fresh();
        both_add(&p, "E17", 1, -1, b"root", f64::from_bits(bits));
        both_add(&p, "E17", 2, 1, b"kid", 1.0);
        let s = both_subtree(&p, "E17", 1);
        assert!(s.is_nan(), "[E17] NaN must propagate");
        both_subtree(&p, "E17", 2);
    }
}

#[test]
fn e18_subtree_sum_overflows_to_infinity() {
    let p = Pair::fresh();
    both_add(&p, "E18", 1, -1, b"root", 1e308);
    both_add(&p, "E18", 2, 1, b"a", 1e308);
    both_add(&p, "E18", 3, 1, b"b", 1e308);
    let s = both_subtree(&p, "E18", 1);
    assert_eq!(s, f64::INFINITY, "[E18] accumulator must overflow to +inf");
    // and the clamp in safe_double_to_int then yields INT_MAX
    let c = unsafe { (p.c.safe_double_to_int)(s) };
    let r = unsafe { (p.rust.safe_double_to_int)(s) };
    eq_i32("E18", "safe_double_to_int(+inf)", c, r);
    assert_eq!(c, INT_MAX);
    // negative direction
    let p = Pair::fresh();
    both_add(&p, "E18", 1, -1, b"root", -1e308);
    both_add(&p, "E18", 2, 1, b"a", -1e308);
    both_add(&p, "E18", 3, 1, b"b", -1e308);
    let s = both_subtree(&p, "E18", 1);
    assert_eq!(s, f64::NEG_INFINITY);
    let c = unsafe { (p.c.safe_double_to_int)(s) };
    let r = unsafe { (p.rust.safe_double_to_int)(s) };
    eq_i32("E18", "safe_double_to_int(-inf)", c, r);
    assert_eq!(c, INT_MIN);
}

#[test]
fn e19_subtree_sum_inf_minus_inf_is_nan() {
    // both child orders, and with the infinities at different depths
    for (a, b) in [
        (f64::INFINITY, f64::NEG_INFINITY),
        (f64::NEG_INFINITY, f64::INFINITY),
    ] {
        let p = Pair::fresh();
        both_add(&p, "E19", 1, -1, b"root", 0.0);
        both_add(&p, "E19", 2, 1, b"a", a);
        both_add(&p, "E19", 3, 1, b"b", b);
        let s = both_subtree(&p, "E19", 1);
        assert!(s.is_nan(), "[E19] inf + -inf must be NaN");

        let p = Pair::fresh();
        both_add(&p, "E19", 1, -1, b"root", a);
        both_add(&p, "E19", 2, 1, b"a", b);
        let s = both_subtree(&p, "E19", 1);
        assert!(s.is_nan());
    }
}

// ---------------------------------------------------------------------------
// E20 / E22 / E23 -- process_string
// ---------------------------------------------------------------------------

#[test]
fn e20_process_string_empty_returns_zero() {
    let p = Pair::fresh();
    let mut b1 = CBuf::raw(&[0u8]);
    let mut b2 = CBuf::raw(&[0u8]);
    let c = unsafe { (p.c.process_string)(b1.ptr_mut()) };
    let r = unsafe { (p.rust.process_string)(b2.ptr_mut()) };
    eq_i32("E20", "process_string(\"\")", c, r);
    assert_eq!(c, 0, "[E20] the `if (*str)` guard must yield 0");
    both_process(&p, "E20", b"\0trailing\0");
}

#[test]
fn e22_process_string_signed_char_negative_sum() {
    let p = Pair::fresh();
    // 0x80..0xff sign-extend to negative ints on this ABI
    both_process(&p, "E22", b"\x80\0");
    both_process(&p, "E22", b"\xff\0");
    let mut v = vec![0x80u8; 100];
    v.push(0);
    both_process(&p, "E22", &v);
    let c = unsafe { (p.c.process_string)(CBuf::raw(&v).ptr_mut()) };
    assert_eq!(c, -12800, "[E22] 100 * (-128)");
    // mixtures where the sum crosses zero
    let mut rng = Rng::new(0xE022);
    for _ in 0..5000 {
        let len = 1 + rng.below(300) as usize;
        let mut v: Vec<u8> = (0..len).map(|_| rng.nonzero_byte()).collect();
        v.push(0);
        both_process(&p, "E22", &v);
    }
}

#[test]
fn e23_process_string_accumulator_overflow() {
    let p = Pair::fresh();
    // 20e6 * 127 == 2_540_000_000 > INT_MAX  =>  wraps
    let n = 20_000_000usize;
    let mut v = vec![0x7fu8; n];
    v.push(0);
    both_process(&p, "E23", &v);
    let c = unsafe { (p.c.process_string)(CBuf::raw(&v).ptr_mut()) };
    assert_eq!(
        c,
        (2_540_000_000u32 as i32),
        "[E23] the C accumulator wraps two's complement"
    );
    // negative direction: 20e6 * (-128) == -2_560_000_000 < INT_MIN
    let mut v = vec![0x80u8; n];
    v.push(0);
    both_process(&p, "E23", &v);
}

// ---------------------------------------------------------------------------
// E24 .. E27 -- safe_double_to_int clamps
// ---------------------------------------------------------------------------

#[test]
fn e24_safe_double_to_int_clamps_above_int_max() {
    let p = Pair::fresh();
    for d in [
        2147483647.5,
        2147483648.0,
        2147483649.0,
        4e9,
        1e300,
        f64::MAX,
        f64::INFINITY,
    ] {
        let c = unsafe { (p.c.safe_double_to_int)(d) };
        let r = unsafe { (p.rust.safe_double_to_int)(d) };
        eq_i32("E24", format!("safe_double_to_int({d})"), c, r);
        assert_eq!(c, INT_MAX, "[E24] must clamp to INT_MAX for {d}");
    }
    // exactly one ULP above (double)INT_MAX
    let one_up = f64::from_bits((INT_MAX as f64).to_bits() + 1);
    let c = unsafe { (p.c.safe_double_to_int)(one_up) };
    let r = unsafe { (p.rust.safe_double_to_int)(one_up) };
    eq_i32("E24", "one ULP above (double)INT_MAX", c, r);
    assert_eq!(c, INT_MAX);
}

#[test]
fn e25_safe_double_to_int_clamps_below_int_min() {
    let p = Pair::fresh();
    for d in [
        -2147483648.5,
        -2147483649.0,
        -4e9,
        -1e300,
        f64::MIN,
        f64::NEG_INFINITY,
    ] {
        let c = unsafe { (p.c.safe_double_to_int)(d) };
        let r = unsafe { (p.rust.safe_double_to_int)(d) };
        eq_i32("E25", format!("safe_double_to_int({d})"), c, r);
        assert_eq!(c, INT_MIN, "[E25] must clamp to INT_MIN for {d}");
    }
    let one_down = f64::from_bits((INT_MIN as f64).to_bits() + 1); // more negative
    let c = unsafe { (p.c.safe_double_to_int)(one_down) };
    let r = unsafe { (p.rust.safe_double_to_int)(one_down) };
    eq_i32("E25", "one ULP below (double)INT_MIN", c, r);
    assert_eq!(c, INT_MIN);
}

#[test]
fn e26_safe_double_to_int_nan_returns_zero() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xE026);
    for bits in [
        0x7ff8_0000_0000_0000u64,
        0xfff8_0000_0000_0000,
        0x7ff0_0000_0000_0001,
        0xfff0_0000_0000_0001,
        0x7fff_ffff_ffff_ffff,
        0xffff_ffff_ffff_ffff,
    ] {
        let d = f64::from_bits(bits);
        let c = unsafe { (p.c.safe_double_to_int)(d) };
        let r = unsafe { (p.rust.safe_double_to_int)(d) };
        eq_i32("E26", format!("safe_double_to_int(NaN 0x{bits:016x})"), c, r);
        assert_eq!(c, 0, "[E26] NaN must return 0 (bits 0x{bits:016x})");
    }
    for _ in 0..20_000 {
        let payload = (rng.next_u64() & 0x000F_FFFF_FFFF_FFFF) | 1;
        for base in [0x7ff0_0000_0000_0000u64, 0xfff0_0000_0000_0000] {
            let d = f64::from_bits(base | payload);
            let c = unsafe { (p.c.safe_double_to_int)(d) };
            let r = unsafe { (p.rust.safe_double_to_int)(d) };
            eq_i32("E26", "random NaN", c, r);
            assert_eq!(c, 0);
        }
    }
}

#[test]
fn e27_safe_double_to_int_exact_boundaries_pass_through() {
    let p = Pair::fresh();
    for (d, want) in [
        (INT_MAX as f64, INT_MAX),
        (INT_MIN as f64, INT_MIN),
        (2147483646.999, 2147483646),
        (-2147483647.999, -2147483647),
    ] {
        let c = unsafe { (p.c.safe_double_to_int)(d) };
        let r = unsafe { (p.rust.safe_double_to_int)(d) };
        eq_i32("E27", format!("safe_double_to_int({d})"), c, r);
        assert_eq!(c, want, "[E27] {d} must pass through the strict comparisons");
    }
}

// ---------------------------------------------------------------------------
// E28 .. E36 -- maxnmin branches
// ---------------------------------------------------------------------------

#[test]
fn e28_maxnmin_first_block_skipped() {
    let p = Pair::fresh();
    // (param1 % 6) + 1 <= 0  =>  no node selected
    for a in [-1, -2, -3, -4, -5, -7, -8, -11, INT_MIN, INT_MIN + 1] {
        let node_id = (a % 6) + 1;
        assert!(node_id <= 0, "param1={a} must miss (node_id={node_id})");
        both_maxnmin(&p, "E28", a, 1, 1, 0);
        both_maxnmin(&p, "E28", a, -1, -1, -1);
    }
    // the boundary: a % 6 == 0 selects node 1 (hit)
    for a in [-6, -12, -18, 0, 6, 12] {
        both_maxnmin(&p, "E28", a, 1, 1, 0);
    }
}

#[test]
fn e29_maxnmin_empty_name_branch_is_unreachable_but_consistent() {
    // The six builtin names are never empty, so `if (*name_ptr)` is always
    // true. Verify the observable consequence: process_string of the selected
    // node's name contributes, i.e. the branch is taken identically in both.
    let p = Pair::fresh();
    for a in 0..12 {
        both_maxnmin(&p, "E29", a, a, 1, 0);
        // after the call the builtin names are still intact & non-empty
        for (id, _, name, _) in BUILTINS {
            let (fc, fr) = both_find(&p, "E29", id);
            let sc = unsafe { (p.c.process_string)((*fc).name.as_mut_ptr()) };
            let sr = unsafe { (p.rust.process_string)((*fr).name.as_mut_ptr()) };
            eq_i32("E29", format!("process_string({name})"), sc, sr);
            assert_ne!(sc, 0, "[E29] builtin name must be non-empty");
        }
    }
    // Now force the empty-name condition the only way the API allows: blank the
    // selected node's name in place, then re-run the *inner* calls the way
    // maxnmin does (maxnmin itself re-seeds, so this exercises the branch
    // predicate on identical data in both libraries).
    for (id, _, _, _) in BUILTINS {
        both_mutate(&p, "E29", id, &|n| unsafe {
            for i in 0..MAX_NAME_LEN {
                (*n).name[i] = 0;
            }
        });
        let (fc, fr) = both_find(&p, "E29", id);
        let sc = unsafe { (p.c.process_string)((*fc).name.as_mut_ptr()) };
        let sr = unsafe { (p.rust.process_string)((*fr).name.as_mut_ptr()) };
        eq_i32("E29", "process_string(blanked name)", sc, sr);
        assert_eq!(sc, 0);
    }
}

#[test]
fn e30_maxnmin_second_block_skipped() {
    let p = Pair::fresh();
    for b in [-1, -2, -3, -4, -5, -7, -13, INT_MIN, INT_MIN + 1] {
        let node_id = (b % 6) + 1;
        assert!(node_id <= 0);
        both_maxnmin(&p, "E30", 1, b, 1, 0);
        both_maxnmin(&p, "E30", 0, b, 7, 5);
        // param3 also multiplies the (skipped) value term
        both_maxnmin(&p, "E30", 1, b, INT_MAX, 1);
    }
}

#[test]
fn e31_maxnmin_division_by_zero() {
    let p = Pair::fresh();
    // param3 == -1  =>  (double)(p1+p2) / 0.0
    for a in [-3, -1, 0, 1, 2, 6, 7, INT_MAX, INT_MIN] {
        for b in [-3, -1, 0, 1, 2, 6, 7, INT_MAX, INT_MIN] {
            for d in [-2, -1, 0, 1, 2, INT_MAX, INT_MIN] {
                both_maxnmin(&p, "E31", a, b, -1, d);
            }
        }
    }
    // 0/0 -> NaN -> safe_double_to_int -> 0 ; x/0 -> +-inf -> INT_MAX/INT_MIN
    both_maxnmin(&p, "E31", 0, 0, -1, 1);
    both_maxnmin(&p, "E31", 1, 0, -1, 0); // inf * 0 -> NaN
}

#[test]
fn e32_maxnmin_param3_plus_one_overflows() {
    let p = Pair::fresh();
    for a in [-6, -1, 0, 1, 6, 7, INT_MAX, INT_MIN] {
        for b in [-6, -1, 0, 1, 6, 7, INT_MAX, INT_MIN] {
            for d in [-1, 0, 1, 2, INT_MAX, INT_MIN] {
                both_maxnmin(&p, "E32", a, b, INT_MAX, d);
                both_maxnmin(&p, "E32", a, b, INT_MAX - 1, d);
            }
        }
    }
}

#[test]
fn e33_maxnmin_param1_plus_param2_overflows() {
    let p = Pair::fresh();
    let extremes = [INT_MAX, INT_MAX - 1, INT_MIN, INT_MIN + 1, 2_000_000_000, -2_000_000_000];
    for &a in &extremes {
        for &b in &extremes {
            for d in [-1, 0, 1, 3, INT_MAX, INT_MIN] {
                both_maxnmin(&p, "E33", a, b, 1, d);
                both_maxnmin(&p, "E33", a, b, -1, d);
                both_maxnmin(&p, "E33", a, b, INT_MAX, d);
            }
        }
    }
}

#[test]
fn e34_maxnmin_int_min_modulo() {
    let p = Pair::fresh();
    // C's % truncates toward zero: INT_MIN % 6 == -2, INT_MIN % 3 == -2
    assert_eq!(INT_MIN % 6, -2);
    assert_eq!(INT_MIN % 3, -2);
    for a in [INT_MIN, INT_MIN + 1, INT_MIN + 2] {
        for b in [INT_MIN, INT_MIN + 1, INT_MIN + 2] {
            for c in [INT_MIN, -1, 0, 1, INT_MAX] {
                for d in [INT_MIN, INT_MIN + 1, INT_MIN + 2, -1, 0, 1, INT_MAX] {
                    both_maxnmin(&p, "E34", a, b, c, d);
                }
            }
        }
    }
}

#[test]
fn e35_maxnmin_parent_id_out_of_tree() {
    let p = Pair::fresh();
    // (param4 % 3) + 1 in {-1, 0, 1, 2, 3}
    for d in -12..=12 {
        let pid = (d % 3) + 1;
        assert!((-1..=3).contains(&pid));
        both_maxnmin(&p, "E35", 0, 0, 1, d);
        both_maxnmin(&p, "E35", 5, 5, 5, d);
    }
    // parent_id == -1 matches the root's parent_id => 1 child
    both_maxnmin(&p, "E35", 0, 0, 1, -1);
    both_maxnmin(&p, "E35", 0, 0, 1, -4);
    // parent_id == 0 matches nothing => 0
    both_maxnmin(&p, "E35", 0, 0, 1, -3);
}

#[test]
fn e36_maxnmin_result_accumulator_overflow() {
    let p = Pair::fresh();
    // the (double) term saturates to INT_MAX / INT_MIN and is then added to the
    // string/subtree terms, wrapping the int accumulator
    for a in [0, 1, 2, 3, 4, 5, 6, 11, 12] {
        for b in [0, 1, 2, 3, 4, 5, 6, 11, 12] {
            both_maxnmin(&p, "E36", a, b, 0, INT_MAX);
            both_maxnmin(&p, "E36", a, b, 0, INT_MIN);
            both_maxnmin(&p, "E36", a, b, -1, INT_MAX);
            both_maxnmin(&p, "E36", a, b, -1, INT_MIN);
            both_maxnmin(&p, "E36", INT_MAX, INT_MAX, 0, INT_MAX);
            both_maxnmin(&p, "E36", INT_MIN, INT_MIN, 0, INT_MIN);
        }
    }
}

// ---------------------------------------------------------------------------
// E37 -- out-of-range "enum"-like int crossing the FFI boundary
// ---------------------------------------------------------------------------

#[test]
fn e37_active_accepts_any_nonzero_int() {
    let weird = [
        1i32,
        2,
        -1,
        -2,
        3,
        0x7fff_ffff,
        i32::MIN,
        0x8000_0000u32 as i32,
        0x0000_0100,
        0x0001_0000,
        0x0100_0000,
        0x00ff_ff00,
    ];
    for &a in &weird {
        let p = Pair::fresh();
        both_add(&p, "E37", 1, -1, b"root", 1.5);
        both_add(&p, "E37", 2, 1, b"kid", 2.5);
        both_mutate(&p, "E37", 1, &|n| unsafe { (*n).active = a });
        both_mutate(&p, "E37", 2, &|n| unsafe { (*n).active = a });
        // any non-zero => still visible in all three scans
        let (fc, fr) = both_find(&p, "E37", 1);
        assert!(!fc.is_null() && !fr.is_null(), "[E37] active={a} must be truthy");
        assert_eq!(both_children(&p, "E37", 1), 1);
        both_subtree(&p, "E37", 1);
        both_query(&p, "E37", 2);
    }
    // and the low-16-bits-zero case, which a `short`-sized truthiness test
    // would get wrong
    for &a in &[0x0001_0000i32, 0x0002_0000, i32::MIN] {
        let p = Pair::fresh();
        both_add(&p, "E37", 1, -1, b"root", 1.5);
        both_mutate(&p, "E37", 1, &|n| unsafe { (*n).active = a });
        let (fc, fr) = both_find(&p, "E37", 1);
        assert!(!fc.is_null() && !fr.is_null(), "[E37] active=0x{a:08x}");
        both_query(&p, "E37", 1);
    }
}

// ---------------------------------------------------------------------------
// E38 -- maxnmin silently resets node_count, discarding previous nodes
// ---------------------------------------------------------------------------

#[test]
fn e38_maxnmin_discards_previously_added_nodes() {
    let p = Pair::fresh();
    for k in 0..40 {
        both_add(&p, "E38", 500 + k, -1, b"pre", k as f64);
    }
    both_query(&p, "E38", 500);
    both_maxnmin(&p, "E38", 3, 4, 5, 6);
    // the 40 pre-existing nodes are unreachable now
    for k in 0..40 {
        let (fc, fr) = both_find(&p, "E38", 500 + k);
        assert!(fc.is_null() && fr.is_null(), "[E38] node {} survived", 500 + k);
        both_query(&p, "E38", 500 + k);
    }
    // and the next insert lands in slot 6
    let idx = both_add(&p, "E38", 900, -1, b"post", 1.0);
    assert_eq!(idx, 6, "[E38] node_count must have been reset to 6");
    both_query(&p, "E38", 900);
    // slot 7.. still holds the old bytes but is invisible
    both_query(&p, "E38", 507);
}
