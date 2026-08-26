//! Phase C — one differential test per row of ERRORS.md (all rows except E6,
//! E12 and E17, which terminate the process and live in `tests/crash.rs`).
//!
//! Every test constructs the exact invalid input / boundary condition, calls the
//! C export and the Rust export, and asserts they return the SAME sentinel or
//! error value — not merely "both failed somehow".

mod common;
use common::*;

// ---------------------------------------------------------------------------
// An independent third implementation of the C semantics, used to pin down the
// exact expected value (so a "both wrong the same way" translation cannot pass
// silently). Derived by hand from c_src/src/lib.c.
// ---------------------------------------------------------------------------

type Seed = (i32, i32, &'static str, f64);
const SEEDED: [Seed; 6] = [
    (1, -1, "root", 10.5),
    (2, 1, "child1", 20.7),
    (3, 1, "child2", 15.3),
    (4, 2, "grandchild1", 5.9),
    (5, 2, "grandchild2", 8.2),
    (6, 3, "grandchild3", 12.4),
];

fn model_safe_double_to_int(d: f64) -> i32 {
    if d > INT_MAX as f64 {
        return INT_MAX;
    }
    if d < INT_MIN as f64 {
        return INT_MIN;
    }
    if d != d {
        return 0;
    }
    d as i32 // range already checked, so `as` == C's truncating (int) cast
}

fn model_find(id: i32) -> Option<usize> {
    SEEDED.iter().position(|n| n.0 == id)
}

fn model_subtree(id: i32) -> f64 {
    match model_find(id) {
        None => 0.0,
        Some(i) => {
            let mut s = SEEDED[i].3;
            for n in SEEDED.iter() {
                if n.1 == id {
                    s += model_subtree(n.0);
                }
            }
            s
        }
    }
}

fn model_maxnmin(p1: i32, p2: i32, p3: i32, p4: i32) -> i32 {
    let mut result: i32 = 0;
    let node_id = (p1 % 6).wrapping_add(1);
    if let Some(i) = model_find(node_id) {
        let name = SEEDED[i].2;
        if !name.is_empty() {
            result = result.wrapping_add(
                name.bytes()
                    .fold(0i32, |a, b| a.wrapping_add(b as i8 as i32)),
            );
        }
        result = result.wrapping_add(model_safe_double_to_int(model_subtree(node_id)));
    }
    let second = (p2 % 6).wrapping_add(1);
    if let Some(i) = model_find(second) {
        result = result.wrapping_add(model_safe_double_to_int(SEEDED[i].3 * p3 as f64));
    }
    let parent = (p4 % 3).wrapping_add(1);
    let children = SEEDED.iter().filter(|n| n.1 == parent).count() as i32;
    result = result.wrapping_add(children.wrapping_mul(10));
    let mut calc = (p1.wrapping_add(p2)) as f64 / (p3.wrapping_add(1)) as f64;
    calc *= p4 as f64;
    result.wrapping_add(model_safe_double_to_int(calc))
}

// ---------------------------------------------------------------------- E1, E2

/// E1 — `add_node` past `MAX_NODES` returns -1 and leaves the store untouched.
/// E2 — the last legal slot (call #100) returns 99.
#[test]
fn e1_e2_add_node_at_and_past_max_nodes() {
    let p = Pair::new("E1/E2");
    for i in 0..(MAX_NODES - 1) {
        assert_eq!(p.add_node(i as i32 + 1, -1, "n", 1.0), i as i32);
    }
    // E2: the 100th call fills the last slot and returns MAX_NODES - 1.
    assert_eq!(
        p.add_node(MAX_NODES as i32, -1, "last", 1.0),
        MAX_NODES as i32 - 1
    );
    assert_eq!(p.get_children_count(-1), MAX_NODES as i32);

    // E1: every further call is rejected with exactly -1, repeatedly, and
    // nothing is appended (the rejected id stays unfindable).
    for k in 0..5 {
        assert_eq!(
            p.add_node(900_000 + k, -1, "rejected", 7.0),
            -1,
            "add_node #{} past MAX_NODES must return -1",
            MAX_NODES + 1 + k as usize
        );
        assert!(
            p.find_node_by_id(900_000 + k).is_none(),
            "a rejected add_node must not have written to storage"
        );
        // node_count must be unchanged: still exactly MAX_NODES parents of -1.
        assert_eq!(p.get_children_count(-1), MAX_NODES as i32);
    }
    // Extreme arguments are still rejected identically.
    assert_eq!(p.add_node(INT_MIN, INT_MAX, "", f64::NAN), -1);
    assert_eq!(p.add_node(INT_MAX, INT_MIN, "x", f64::INFINITY), -1);
    p.probe_all(&[-1, 0, 1, 50, 100, 101, INT_MIN, INT_MAX]);
}

// ------------------------------------------------------------ E3, E4, E5, E37

/// E3 — a name of 50+ bytes is silently truncated to 49 with a forced NUL.
/// E4 — a name of exactly 49 bytes is stored verbatim.
/// E5 — an empty name is accepted and stored as all-NUL.
#[test]
fn e3_e4_e5_name_boundaries() {
    let p = Pair::new("E3/E4/E5");

    // E5: empty name.
    assert_eq!(p.add_node_raw(1, -1, b"\0", 1.0), 0);
    assert_eq!(&p.node_view(1).unwrap().name[..], &[0u8; MAX_NAME_LEN][..]);
    // `process_string` over that empty name returns 0 (the `if (*str)` exit).
    assert_eq!(p.process_string(b"\0"), 0);

    // E4: exactly MAX_NAME_LEN - 1 == 49 bytes.
    let n49: Vec<u8> = (0..49).map(|i| b'a' + (i % 26) as u8).collect();
    let mut arg = n49.clone();
    arg.push(0);
    assert_eq!(p.add_node_raw(2, -1, &arg, 1.0), 1);
    let v = p.node_view(2).unwrap();
    assert_eq!(&v.name[..49], &n49[..], "49 bytes must be stored verbatim");
    assert_eq!(v.name[49], 0, "name[MAX_NAME_LEN-1] is forced to NUL");

    // E3: 50, 51 and 200 bytes are truncated to the first 49.
    for (id, len) in [(3i32, 50usize), (4, 51), (5, 200), (6, 1000)] {
        let long: Vec<u8> = (0..len).map(|i| b'A' + (i % 26) as u8).collect();
        let mut a = long.clone();
        a.push(0);
        p.add_node_raw(id, -1, &a, 1.0);
        let v = p.node_view(id).unwrap();
        assert_eq!(
            &v.name[..49],
            &long[..49],
            "id={id}: first 49 bytes must survive"
        );
        assert_eq!(v.name[49], 0, "id={id}: byte 49 must be NUL");
    }

    // E37: no validation of id / parent_id / value at all.
    // NOTE: the parent ids here must be dangling. Wiring these nodes to each
    // other (or to themselves) makes a parent/child cycle, which the C code has
    // no guard against; that is ERRORS.md row E12 and is exercised out of
    // process in tests/crash.rs.
    for (i, &(id, parent)) in [
        (INT_MIN, -424_242),
        (INT_MAX, -424_243),
        (0, -424_244),
        (-1, -424_245),
    ]
    .iter()
    .enumerate()
    {
        let value = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.0][i];
        let idx = p.add_node(id, parent, "extreme", value);
        assert!(idx >= 0);
        let v = p.node_view(id).unwrap();
        assert_eq!(v.id, id);
        assert_eq!(v.parent_id, parent);
        assert_eq!(v.value_bits, value.to_bits(), "value stored verbatim");
        // A NaN value poisons the subtree sum, and safe_double_to_int maps it to 0.
        let s = p.calculate_subtree_sum(id);
        p.safe_double_to_int(s);
    }
}

// ------------------------------------------------------------------- E7, E8

/// E7 — `find_node_by_id` on an id that was never added returns NULL.
/// E8 — `find_node_by_id` on empty storage returns NULL for every id.
#[test]
fn e7_e8_find_node_not_found_returns_null() {
    // E8: fresh library, node_count == 0.
    let empty = Pair::new("E8");
    for &id in INT_CLASSES {
        assert!(
            empty.find_node_by_id(id).is_none(),
            "empty storage must not find id {id}"
        );
    }

    // E7: populated, but query the ids that are absent.
    let p = Pair::new("E7");
    for i in 1..=10 {
        p.add_node(i * 10, -1, "n", i as f64);
    }
    for &id in INT_CLASSES {
        let present = (1..=10).any(|i| i * 10 == id);
        assert_eq!(
            p.find_node_by_id(id).is_some(),
            present,
            "presence of id {id}"
        );
    }
    // Off-by-one around every present id.
    for i in 1..=10 {
        assert!(p.find_node_by_id(i * 10 - 1).is_none());
        assert!(p.find_node_by_id(i * 10 + 1).is_none());
    }
}

// ------------------------------------------------------------------- E9, E13

/// E9  — a matching node whose `active` is 0 is skipped, so lookup returns NULL.
/// E13 — `active` is tested with `&&`, so ANY non-zero int is truthy. This is the
///       "out-of-range enum/bool value crossing the FFI boundary" case: the field
///       is an `int` and a caller can legally store any of the 2^32 values in it.
#[test]
fn e9_e13_active_flag_out_of_range_values() {
    for &val in &[
        0i32, 1, 2, -1, 3, -2, 255, 256, 0x1_0000, INT_MIN, INT_MAX, 0x7FFF_FFFE,
    ] {
        let p = Pair::new(&format!("E9/E13[{val}]"));
        p.add_node(1, -1, "root", 1.0);
        p.add_node(2, 1, "kid", 2.0);
        p.set_active(2, val);

        let truthy = val != 0;
        // E9 / E13 on find_node_by_id
        assert_eq!(
            p.find_node_by_id(2).is_some(),
            truthy,
            "active={val}: find_node_by_id"
        );
        // E13 on get_children_count
        assert_eq!(
            p.get_children_count(1),
            if truthy { 1 } else { 0 },
            "active={val}: get_children_count"
        );
        // E11 / E13 on calculate_subtree_sum
        assert_eq!(
            p.calculate_subtree_sum(1),
            if truthy { 3.0 } else { 1.0 },
            "active={val}: calculate_subtree_sum(parent)"
        );
        assert_eq!(
            p.calculate_subtree_sum(2).to_bits(),
            if truthy { 2.0f64.to_bits() } else { 0u64 },
            "active={val}: calculate_subtree_sum(self)"
        );
    }
}

// ----------------------------------------------------------------- E10, E11

/// E10 — `calculate_subtree_sum` on a missing id returns positive 0.0.
/// E11 — ... and the same for a node that exists but is inactive.
#[test]
fn e10_e11_subtree_sum_not_found_returns_positive_zero() {
    let p = Pair::new("E10/E11");
    p.add_node(1, -1, "root", 10.5);
    p.add_node(2, 1, "kid", -20.0);

    for &id in INT_CLASSES {
        if id == 1 || id == 2 {
            continue;
        }
        let s = p.calculate_subtree_sum(id);
        assert_eq!(
            s.to_bits(),
            0u64,
            "missing id {id} must yield +0.0 (bits 0), got {s:?}"
        );
        assert!(!s.is_sign_negative(), "must be +0.0, not -0.0");
    }

    // E11: existing but inactive.
    p.set_active(2, 0);
    assert_eq!(p.calculate_subtree_sum(2).to_bits(), 0u64);
    // The now-inactive node is also excluded from its parent's sum.
    assert_eq!(p.calculate_subtree_sum(1), 10.5);

    // A node whose only value is -0.0 still returns -0.0, distinguishing the
    // "found, value == -0.0" case from the "not found" case.
    let q = Pair::new("E10b");
    q.add_node(1, -1, "z", -0.0);
    assert_eq!(q.calculate_subtree_sum(1).to_bits(), (-0.0f64).to_bits());
    assert_eq!(q.calculate_subtree_sum(2).to_bits(), 0u64);
}

// ------------------------------------------------------------ E14, E15, E16

/// E14 — `get_children_count` with an unreferenced parent id returns 0.
/// E15 — ... and 0 on empty storage.
/// E16 — `parent_id == -1` is the sentinel `maxnmin`'s root uses; it is a normal
///        value to this function and counts the -1-parented nodes.
#[test]
fn e14_e15_e16_children_count_edges() {
    // E15
    let empty = Pair::new("E15");
    for &pid in INT_CLASSES {
        assert_eq!(
            empty.get_children_count(pid),
            0,
            "empty storage, parent {pid}"
        );
    }

    // E14 / E16
    let p = Pair::new("E14/E16");
    p.add_node(1, -1, "r1", 1.0);
    p.add_node(2, -1, "r2", 1.0);
    p.add_node(3, 1, "c", 1.0);
    p.add_node(4, INT_MIN, "x", 1.0);
    p.add_node(5, INT_MAX, "y", 1.0);
    p.add_node(6, 0, "z", 1.0);
    assert_eq!(p.get_children_count(-1), 2, "E16: -1 sentinel");
    assert_eq!(p.get_children_count(1), 1);
    assert_eq!(p.get_children_count(INT_MIN), 1);
    assert_eq!(p.get_children_count(INT_MAX), 1);
    assert_eq!(p.get_children_count(0), 1);
    // E14: unreferenced parents.
    for &pid in &[-2, -3, 2, 3, 4, 5, 6, 7, 12345, INT_MIN + 1, INT_MAX - 1] {
        assert_eq!(p.get_children_count(pid), 0, "E14: parent {pid}");
    }
    // After maxnmin the -1 sentinel matches exactly one node (the seeded root).
    p.maxnmin(0, 0, 0, 0);
    assert_eq!(p.get_children_count(-1), 1, "E16 after maxnmin");
}

// ----------------------------------------------------------- E18, E19, E20

/// E18 — `process_string("")` returns 0 via the `if (*str)` early exit.
/// E19 — high-bit bytes are negative because `char` is signed here.
/// E20 — the accumulator overflows and wraps (no overflow check in the C).
#[test]
fn e18_e19_e20_process_string_edges() {
    let p = Pair::new("E18/E19/E20");

    // E18
    assert_eq!(p.process_string(b"\0"), 0);
    // and a buffer whose very first byte is NUL but which has data after it
    assert_eq!(p.process_string(b"\0abcdef\0"), 0);

    // E19
    assert_eq!(p.process_string(&[0x80, 0]), -128);
    assert_eq!(p.process_string(&[0x81, 0]), -127);
    assert_eq!(p.process_string(&[0xFE, 0]), -2);
    assert_eq!(p.process_string(&[0xFF, 0]), -1);
    assert_eq!(p.process_string(&[0x7F, 0]), 127);
    // sum of all 255 non-zero byte values, as signed chars
    let all: Vec<u8> = (1u8..=255).collect();
    let mut allz = all.clone();
    allz.push(0);
    let want: i32 = all.iter().map(|&b| b as i8 as i32).sum();
    assert_eq!(p.process_string(&allz), want);
    assert!(want < 0, "signed-char sum of 1..=255 must be negative");

    // E20: positive overflow.
    const N: usize = 20_000_000; // 20e6 * 127 == 2_540_000_000 > INT_MAX
    let mut big = vec![0x7Fu8; N];
    big.push(0);
    let expect = (N as i64 * 127) as u32 as i32;
    let got = p.process_string(&big);
    assert_eq!(got, expect, "overflow must wrap to {expect}");
    assert!(got < 0);
    // E20: negative overflow.
    let mut big2 = vec![0x80u8; N];
    big2.push(0);
    assert_eq!(p.process_string(&big2), (N as i64 * -128) as u32 as i32);
}

// -------------------------------------------- E21, E22, E23, E24, E25, E26

/// E21/E22 — the `d > (double)INT_MAX` guard and the value one step inside it.
/// E23/E24 — the `d < (double)INT_MIN` guard and the value one step inside it.
/// E25     — the `d != d` NaN guard.
/// E26     — truncation toward zero, -0.0 and subnormals.
#[test]
fn e21_to_e26_safe_double_to_int_guards() {
    let p = Pair::new("E21..E26");

    // E21: strictly greater than INT_MAX.
    for d in [
        2147483648.0,
        2147483647.5,
        f64::from_bits((INT_MAX as f64).to_bits() + 1),
        4294967296.0,
        1e300,
        f64::MAX,
        f64::INFINITY,
    ] {
        assert_eq!(p.safe_double_to_int(d), INT_MAX, "E21 d={d:?}");
    }
    // E22: exactly INT_MAX (one step inside) falls through to the cast.
    assert_eq!(p.safe_double_to_int(INT_MAX as f64), INT_MAX, "E22");
    assert_eq!(
        p.safe_double_to_int(f64::from_bits((INT_MAX as f64).to_bits() - 1)),
        2147483646,
        "E22 one ulp below INT_MAX"
    );

    // E23: strictly less than INT_MIN.
    for d in [
        -2147483648.5,
        -2147483649.0,
        f64::from_bits((INT_MIN as f64).to_bits() + 1),
        -4294967296.0,
        -1e300,
        f64::MIN,
        f64::NEG_INFINITY,
    ] {
        assert_eq!(p.safe_double_to_int(d), INT_MIN, "E23 d={d:?}");
    }
    // E24: exactly INT_MIN (one step inside).
    assert_eq!(p.safe_double_to_int(INT_MIN as f64), INT_MIN, "E24");
    assert_eq!(
        p.safe_double_to_int(-2147483647.5),
        -2147483647,
        "E24 truncation toward zero just inside INT_MIN"
    );

    // E25: every flavour of NaN -> 0. Note the ORDER of the C guards: the two
    // range compares are false for NaN, so the `d != d` guard is what fires.
    for bits in [
        0x7FF8_0000_0000_0000u64, // +qNaN
        0xFFF8_0000_0000_0000,    // -qNaN
        0x7FF0_0000_0000_0001,    // +sNaN
        0xFFF0_0000_0000_0001,    // -sNaN
        0x7FFF_FFFF_FFFF_FFFF,
        0xFFF8_0000_DEAD_BEEF,
    ] {
        let d = f64::from_bits(bits);
        assert!(d.is_nan());
        assert_eq!(p.safe_double_to_int(d), 0, "E25 bits={bits:#018x}");
    }

    // E26: truncation toward zero (NOT floor), -0.0, subnormals.
    for (d, want) in [
        (0.0f64, 0i32),
        (-0.0, 0),
        (f64::MIN_POSITIVE, 0),
        (-f64::MIN_POSITIVE, 0),
        (5e-324, 0),
        (-5e-324, 0),
        (0.5, 0),
        (-0.5, 0),
        (0.999999999, 0),
        (-0.999999999, 0),
        (1.0, 1),
        (-1.0, -1),
        (1.5, 1),
        (-1.5, -1),
        (1.9, 1),
        (-1.9, -1),
        (2.5, 2),
        (-2.5, -2),
        (-3.7, -3),
        (2147483646.9, 2147483646),
        (-2147483647.9, -2147483647),
    ] {
        assert_eq!(p.safe_double_to_int(d), want, "E26 d={d:?}");
    }

    // Cross-check the model on a large random sample.
    let mut rng = Rng::new(0xE21E_2600_0000_0011);
    for _ in 0..50_000 {
        let d = if rng.next_u64() % 3 == 0 {
            rng.next_f64_bits()
        } else {
            rng.next_f64_spread()
        };
        let got = p.safe_double_to_int(d);
        assert_eq!(got, model_safe_double_to_int(d), "model mismatch for {d:?}");
    }
}

// ------------------------------------------------------ E27, E28, E32, E33, E34

/// E27 — `param1` making `(param1 % 6) + 1 <= 0` skips the first whole block.
/// E28 — `param2` making `(param2 % 6) + 1 <= 0` skips the multiply block.
/// E32 — `INT_MIN % 6 == -2` / `INT_MIN % 3 == -2` (C `%` truncates toward zero).
/// E33 — `param4 == -1` -> `get_children_count(0)` -> 0 children.
/// E34 — `param4 == -2` -> `get_children_count(-1)` -> the seeded root.
#[test]
fn e27_e28_e32_e33_e34_maxnmin_null_and_modulo_branches() {
    let p = Pair::new("E27..E34");

    // E27 + E28 + E33: both node lookups miss and get_children_count(0) == 0,
    // so the result is only the (double) tail term. Hand-computed from the C.
    //   node_id        = (-1 % 6) + 1 = 0  -> NULL, first block skipped
    //   second_node_id = (-1 % 6) + 1 = 0  -> NULL, multiply block skipped
    //   parent_id      = ( 0 % 3) + 1 = 1  -> 2 children -> +20
    //   calculation    = (-2)/1 = -2.0; *= 0 -> -0.0 -> 0
    assert_eq!(p.maxnmin(-1, -1, 0, 0), 20, "E27/E28: both blocks skipped");

    // E33: param4 == -1 -> parent_id == 0 -> zero children.
    //   node_id = 0 -> NULL; second = 0 -> NULL; children(0) = 0
    //   calculation = (-2)/1 * -1 = 2.0 -> 2
    assert_eq!(p.maxnmin(-1, -1, 0, -1), 2, "E33: get_children_count(0) == 0");

    // E34: param4 == -2 -> parent_id == -1 -> the seeded root matches.
    //   children(-1) = 1 -> +10; calculation = (-2)/1 * -2 = 4.0 -> 4
    assert_eq!(p.maxnmin(-1, -1, 0, -2), 14, "E34: get_children_count(-1) == 1");

    // E27/E28 individually: only one of the two lookups misses.
    for &p1 in &[-1i32, -2, -3, -4, -5, -7, -8, -11, -13, INT_MIN] {
        for &p2 in &[1i32, 2, 5, 6, 7, 12] {
            let got = p.maxnmin(p1, p2, 1, 1);
            assert_eq!(got, model_maxnmin(p1, p2, 1, 1), "E27 p1={p1} p2={p2}");
        }
    }
    for &p2 in &[-1i32, -2, -3, -4, -5, -7, -8, -11, -13, INT_MIN] {
        for &p1 in &[1i32, 2, 5, 6, 7, 12] {
            let got = p.maxnmin(p1, p2, 1, 1);
            assert_eq!(got, model_maxnmin(p1, p2, 1, 1), "E28 p1={p1} p2={p2}");
        }
    }

    // E32: INT_MIN % 6 == -2 and INT_MIN % 3 == -2 in C (truncated division).
    assert_eq!(INT_MIN % 6, -2);
    assert_eq!(INT_MIN % 3, -2);
    // node_id = -1 -> NULL, second = -1 -> NULL, parent_id = -1 -> 1 child.
    for &p3 in &[0i32, 1, 2, -2, INT_MAX] {
        for &p4 in &[INT_MIN, -2, -1, 0, 1, 2, INT_MAX] {
            let got = p.maxnmin(INT_MIN, INT_MIN, p3, p4);
            assert_eq!(got, model_maxnmin(INT_MIN, INT_MIN, p3, p4));
        }
    }
    // multiples of 6/3 stay non-negative
    for &p1 in &[-6i32, -12, -60, -600, -6 * 357913941] {
        let got = p.maxnmin(p1, p1, 1, p1);
        assert_eq!(got, model_maxnmin(p1, p1, 1, p1), "p1={p1}");
    }
}

// ------------------------------------------------------------------ E29, E36

/// E29 — `param3 == -1` makes the divisor exactly 0.0 (IEEE division by zero).
/// E36 — `value * param3` overflowing `int` range saturates via
///        `safe_double_to_int`.
#[test]
fn e29_e36_maxnmin_division_by_zero_and_saturation() {
    let p = Pair::new("E29/E36");

    // param1 + param2 == 0 -> 0.0/0.0 -> NaN -> 0
    for &(a, b) in &[(0i32, 0i32), (6, -6), (12, -12), (-30, 30)] {
        for &d in &[INT_MIN, -2, -1, 0, 1, 2, INT_MAX] {
            let got = p.maxnmin(a, b, -1, d);
            assert_eq!(got, model_maxnmin(a, b, -1, d), "0/0 a={a} b={b} d={d}");
        }
    }
    // Concretely, hand-computed from the C source:
    //   maxnmin(-1,-1,-1,-1): node_id = (-1%6)+1 = 0 -> NULL, second = 0 -> NULL,
    //   parent_id = (-1%3)+1 = 0 -> 0 children -> +0,
    //   calculation = (double)(-2)/(double)0 = -inf; -inf * -1 = +inf -> INT_MAX.
    assert_eq!(p.maxnmin(-1, -1, -1, -1), INT_MAX, "E29: -inf * -1 -> INT_MAX");
    //   maxnmin(-1,-1,-1,0): same NULL branches, but parent_id = (0%3)+1 = 1 ->
    //   2 children -> +20, and -inf * 0 = NaN -> 0. So the whole result is 20,
    //   which proves the division-by-zero term contributed exactly nothing.
    assert_eq!(p.maxnmin(-1, -1, -1, 0), 20, "E29: inf * 0 -> NaN -> 0");
    //   maxnmin(-6,6,-1,5): param1+param2 == 0 so the division is 0.0/0.0 = NaN,
    //   NaN * 5 = NaN -> 0. Here both node lookups DO succeed (node_id = 1), so
    //   compare against the independent model rather than a hand-computed sum.
    let got = p.maxnmin(-6, 6, -1, 5);
    assert_eq!(got, model_maxnmin(-6, 6, -1, 5), "E29: 0.0/0.0 -> NaN -> 0");
    // ...and the tail term really is 0: the same call with a different param4
    // (which only scales the NaN) must give the identical result.
    assert_eq!(p.maxnmin(-6, 6, -1, 5), p.maxnmin(-6, 6, -1, 2));
    assert_eq!(p.maxnmin(-6, 6, -1, 5), p.maxnmin(-6, 6, -1, 8));

    // param1 + param2 != 0 -> +/- inf.
    for &(a, b) in &[(1i32, 0i32), (-1, 0), (6, 6), (-6, -6), (7, 5)] {
        for &d in &[INT_MIN, -2, -1, 0, 1, 2, INT_MAX] {
            let got = p.maxnmin(a, b, -1, d);
            assert_eq!(got, model_maxnmin(a, b, -1, d), "inf a={a} b={b} d={d}");
        }
    }

    // E36: huge param3 saturates the `value * param3` term.
    for &p3 in &[
        1_000_000_000,
        2_000_000_000,
        INT_MAX,
        INT_MAX - 1,
        -1_000_000_000,
        -2_000_000_000,
        INT_MIN,
        INT_MIN + 1,
    ] {
        for &p2 in &[1i32, 2, 3, 4, 5, 6] {
            let got = p.maxnmin(1, p2, p3, 1);
            assert_eq!(got, model_maxnmin(1, p2, p3, 1), "E36 p2={p2} p3={p3}");
        }
    }
}

// ------------------------------------------------------------------ E30, E31, E35

/// E30 — `param3 + 1` overflows at `param3 == INT_MAX` (UB in C; wraps in the
///        generated code, so the divisor becomes -2147483648.0).
/// E31 — `param1 + param2` overflows.
/// E35 — the `result` accumulator overflows.
#[test]
fn e30_e31_e35_maxnmin_integer_overflow() {
    let p = Pair::new("E30/E31/E35");

    // E30: the divisor after wrapping is INT_MIN, i.e. NEGATIVE, which flips the
    // sign of the final term compared with a saturating implementation.
    for &a in &[INT_MIN, -7, -1, 0, 1, 6, 7, INT_MAX] {
        for &b in &[INT_MIN, -7, -1, 0, 1, 6, 7, INT_MAX] {
            for &d in &[INT_MIN, -2, -1, 0, 1, 2, INT_MAX] {
                let got = p.maxnmin(a, b, INT_MAX, d);
                assert_eq!(got, model_maxnmin(a, b, INT_MAX, d), "E30 {a} {b} {d}");
            }
        }
    }
    // Concretely: param3 = INT_MAX -> divisor = (double)INT_MIN.
    assert_eq!(
        p.maxnmin(-1, -1, INT_MAX, 1),
        model_maxnmin(-1, -1, INT_MAX, 1)
    );

    // E31: param1 + param2 overflow, both directions.
    for &(a, b) in &[
        (INT_MAX, INT_MAX),
        (INT_MAX, 1),
        (1, INT_MAX),
        (INT_MIN, INT_MIN),
        (INT_MIN, -1),
        (-1, INT_MIN),
        (INT_MAX, INT_MIN),
        (INT_MAX - 5, 100),
    ] {
        for &c in &[INT_MIN, -2, -1, 0, 1, 2, 1_000_000, INT_MAX] {
            for &d in &[INT_MIN, -1, 0, 1, INT_MAX] {
                let got = p.maxnmin(a, b, c, d);
                assert_eq!(got, model_maxnmin(a, b, c, d), "E31 {a} {b} {c} {d}");
            }
        }
    }

    // E35: force at least two INT_MAX-sized terms into `result` so it wraps.
    let mut wrapped = false;
    for &c in &[1_000_000_000, 2_000_000_000, INT_MAX] {
        for &d in &[1_000_000_000, 2_000_000_000, INT_MAX] {
            let got = p.maxnmin(1, 1, c, d);
            assert_eq!(got, model_maxnmin(1, 1, c, d), "E35 c={c} d={d}");
            if got < 0 {
                wrapped = true;
            }
        }
    }
    assert!(
        wrapped,
        "E35: expected at least one configuration where `result` wraps negative"
    );

    // Broad randomized cross-check of the whole function against the model.
    let mut rng = Rng::new(0xE303_1350_0000_0022);
    for _ in 0..30_000 {
        let (a, b, c, d) = (
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
        );
        let got = p.maxnmin(a, b, c, d);
        assert_eq!(got, model_maxnmin(a, b, c, d), "model: maxnmin({a},{b},{c},{d})");
    }
    for _ in 0..30_000 {
        let (a, b, c, d) = (
            rng.range_i32(-40, 40),
            rng.range_i32(-40, 40),
            rng.range_i32(-40, 40),
            rng.range_i32(-40, 40),
        );
        let got = p.maxnmin(a, b, c, d);
        assert_eq!(got, model_maxnmin(a, b, c, d), "model: maxnmin({a},{b},{c},{d})");
    }
}

// ---------------------------------------------------------------------- E38

/// E38 — `maxnmin` resets `node_count` to 0 on entry, so repeated calls are
/// idempotent and storage always ends with exactly the 6 seeded nodes.
#[test]
fn e38_maxnmin_state_reset_is_idempotent() {
    let p = Pair::new("E38");
    // Dirty the store first, including filling it completely.
    for i in 0..MAX_NODES {
        p.add_node(50_000 + i as i32, -1, "dirt", i as f64);
    }
    assert_eq!(p.add_node(1, -1, "full", 0.0), -1, "store must be full");

    for round in 0..4 {
        let a = p.maxnmin(2, 3, 4, 5);
        let b = p.maxnmin(2, 3, 4, 5);
        assert_eq!(a, b, "round {round}: maxnmin must be idempotent");
        // exactly 6 nodes, ids 1..=6
        for id in 1..=6 {
            assert!(p.find_node_by_id(id).is_some(), "seeded node {id}");
        }
        assert!(p.find_node_by_id(7).is_none(), "no 7th node");
        assert!(p.find_node_by_id(50_000).is_none(), "dirt must be invisible");
        // next add_node lands at index 6 => node_count was exactly 6
        assert_eq!(p.add_node(4242, -1, "probe", 0.0), 6);
        assert_eq!(p.add_node(4243, -1, "probe", 0.0), 7);
    }
}
