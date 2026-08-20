//! Phase B — CONFIGS.md rows C25..C33: the `maxnmin` convenience entry point,
//! its interaction with the low-level entry points, and a whole-API operation
//! sequence fuzzer.

mod common;
use common::*;

/// `maxnmin` resets `node_count` to 0 on entry, so a single Pair can serve many
/// calls without state leaking between them.
fn probe_ids() -> Vec<i32> {
    vec![-2, -1, 0, 1, 2, 3, 4, 5, 6, 7, INT_MIN, INT_MAX]
}

// ---------------------------------------------------------------------- C25

/// C25 — axis G: the pruned cross-product of the argument classes the C code
/// actually branches on (`% 6`, `% 6`, `+1 == 0` / overflow, `% 3`).
#[test]
fn c25_maxnmin_argument_class_crossproduct() {
    let p = Pair::new("C25");
    let p1s = [INT_MIN, -7, -6, -5, -1, 0, 1, 5, 6, 7, INT_MAX];
    let p2s = [INT_MIN, -7, -6, -5, -1, 0, 1, 5, 6, 7, INT_MAX];
    let p3s = [INT_MIN, -2, -1, 0, 1, 2, INT_MAX];
    let p4s = [INT_MIN, -4, -3, -2, -1, 0, 1, 2, 3, INT_MAX];
    let mut n = 0u64;
    for &a in &p1s {
        for &b in &p2s {
            for &c in &p3s {
                for &d in &p4s {
                    p.maxnmin(a, b, c, d);
                    n += 1;
                }
            }
        }
    }
    assert_eq!(n, (p1s.len() * p2s.len() * p3s.len() * p4s.len()) as u64);
    assert!(n >= 4_800, "expected >= 4800 combinations, ran {n}");
}

// ---------------------------------------------------------------------- C26

/// C26 — randomized over the full `i32` range for all four parameters. Hits
/// `param1 + param2` / `param3 + 1` signed overflow and `result` accumulation
/// overflow, all of which the C code performs unchecked.
#[test]
fn c26_maxnmin_random_full_range() {
    let p = Pair::new("C26");
    let mut rng = Rng::new(0x2626_0000_1234_5678);
    for _ in 0..20_000 {
        p.maxnmin(rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32());
    }
    // Mixed magnitude: some params extreme, some tiny.
    let extremes = [INT_MIN, INT_MIN + 1, -1, 0, 1, INT_MAX - 1, INT_MAX];
    for _ in 0..20_000 {
        let pick = |r: &mut Rng| {
            if r.next_u64() % 2 == 0 {
                extremes[r.below(extremes.len() as u64) as usize]
            } else {
                r.next_i32()
            }
        };
        let (a, b, c, d) = (pick(&mut rng), pick(&mut rng), pick(&mut rng), pick(&mut rng));
        p.maxnmin(a, b, c, d);
    }
}

// ---------------------------------------------------------------------- C27

/// C27 — small parameters, so every `% 6` and `% 3` residue (including the
/// negative ones that make `find_node_by_id` return NULL) is hit many times.
#[test]
fn c27_maxnmin_small_params_exhaustive() {
    let p = Pair::new("C27");
    // Exhaustive over -16..=16 for param1/param4 x a spread of param2/param3.
    for a in -16..=16 {
        for d in -16..=16 {
            for &b in &[-13, -7, -6, -1, 0, 1, 6, 7, 13] {
                for &c in &[-3, -2, -1, 0, 1, 2, 3] {
                    p.maxnmin(a, b, c, d);
                }
            }
        }
    }
    let mut rng = Rng::new(0x2727_0000_9999_0007);
    for _ in 0..20_000 {
        p.maxnmin(
            rng.range_i32(-16, 16),
            rng.range_i32(-16, 16),
            rng.range_i32(-16, 16),
            rng.range_i32(-16, 16),
        );
    }
}

// ---------------------------------------------------------------------- C28

/// C28 — `param3 == -1` makes the divisor `(double)(param3 + 1)` exactly 0.0,
/// so the C code performs an IEEE division by zero. Crossed with the sign of
/// `param1 + param2` (0 -> NaN, >0 -> +inf, <0 -> -inf) and `param4`
/// (0 -> inf*0 == NaN).
#[test]
fn c28_maxnmin_division_by_zero() {
    let p = Pair::new("C28");
    // param1 + param2 == 0 -> 0.0/0.0 -> NaN -> safe_double_to_int -> 0
    for &(a, b) in &[
        (0, 0),
        (5, -5),
        (-5, 5),
        (6, -6),
        (12, -12),
        (INT_MAX, INT_MIN + 1),
        (INT_MIN + 1, INT_MAX),
    ] {
        for &d in &[INT_MIN, -3, -2, -1, 0, 1, 2, 3, INT_MAX] {
            p.maxnmin(a, b, -1, d);
        }
    }
    // param1 + param2 != 0 -> +/-inf, then * param4
    for &(a, b) in &[(1, 0), (0, 1), (6, 6), (-1, 0), (0, -1), (-6, -6), (12, 1), (-12, -1)] {
        for &d in &[INT_MIN, -3, -2, -1, 0, 1, 2, 3, INT_MAX] {
            p.maxnmin(a, b, -1, d);
        }
    }
    // Everything crossed with param3 == -1 and random param1/param2/param4.
    let mut rng = Rng::new(0x2828_0000_D100_0008);
    for _ in 0..5_000 {
        p.maxnmin(rng.next_i32(), rng.next_i32(), -1, rng.next_i32());
    }
    for _ in 0..5_000 {
        let a = rng.range_i32(-24, 24);
        p.maxnmin(a, -a, -1, rng.range_i32(-24, 24)); // forces the 0/0 -> NaN case
    }
}

// ---------------------------------------------------------------------- C29

/// C29 — the overflow configurations: `param3 + 1` wrapping at INT_MAX,
/// `param1 + param2` wrapping, and the `result` accumulator wrapping because
/// two of its terms saturate to INT_MAX.
#[test]
fn c29_maxnmin_overflow_configurations() {
    let p = Pair::new("C29");
    // param3 == INT_MAX: param3 + 1 wraps to INT_MIN (UB in C, wraps in practice)
    for &a in &[INT_MIN, -7, -1, 0, 1, 6, 7, INT_MAX] {
        for &b in &[INT_MIN, -7, -1, 0, 1, 6, 7, INT_MAX] {
            for &d in &[INT_MIN, -2, -1, 0, 1, 2, INT_MAX] {
                p.maxnmin(a, b, INT_MAX, d);
                p.maxnmin(a, b, INT_MIN, d);
            }
        }
    }
    // param1 + param2 overflow in both directions.
    for &d in &[INT_MIN, -2, -1, 0, 1, 2, INT_MAX] {
        for &c in &[INT_MIN, -2, -1, 0, 1, 2, 1_000_000, INT_MAX] {
            p.maxnmin(INT_MAX, INT_MAX, c, d);
            p.maxnmin(INT_MIN, INT_MIN, c, d);
            p.maxnmin(INT_MAX, 1, c, d);
            p.maxnmin(INT_MIN, -1, c, d);
        }
    }
    // result accumulation overflow: huge param3 saturates the multiply term to
    // INT_MAX, and a big quotient saturates the final term too.
    for &c in &[1_000_000_000, 2_000_000_000, INT_MAX, -1_000_000_000, INT_MIN] {
        for &d in &[1, 2, 1_000_000_000, INT_MAX, -1, INT_MIN] {
            p.maxnmin(1, 1, c, d);
            p.maxnmin(6, 6, c, d);
            p.maxnmin(INT_MAX, INT_MAX, c, d);
        }
    }
}

// ---------------------------------------------------------------------- C30

/// C30 — axis H: probe a fresh library, then run `maxnmin`, then verify the six
/// nodes it seeded through EVERY low-level entry point.
#[test]
fn c30_state_after_maxnmin_via_low_level_api() {
    let p = Pair::new("C30");
    // Fresh: nothing there.
    for &id in &probe_ids() {
        assert!(p.find_node_by_id(id).is_none());
        assert_eq!(p.get_children_count(id), 0);
        assert_eq!(p.calculate_subtree_sum(id).to_bits(), 0u64);
    }

    p.maxnmin(1, 2, 3, 4);

    // The six seeded nodes, byte-for-byte (values read from c_src/src/lib.c).
    let seeded: [(i32, i32, &str, f64); 6] = [
        (1, -1, "root", 10.5),
        (2, 1, "child1", 20.7),
        (3, 1, "child2", 15.3),
        (4, 2, "grandchild1", 5.9),
        (5, 2, "grandchild2", 8.2),
        (6, 3, "grandchild3", 12.4),
    ];
    for &(id, parent, name, value) in &seeded {
        let v = p.node_view(id).unwrap_or_else(|| panic!("seeded node {id} missing"));
        assert_eq!(v.id, id);
        assert_eq!(v.parent_id, parent);
        assert_eq!(v.active, 1);
        assert_eq!(f64::from_bits(v.value_bits), value);
        let mut want = [0u8; MAX_NAME_LEN];
        want[..name.len()].copy_from_slice(name.as_bytes());
        assert_eq!(&v.name[..], &want[..], "name of node {id}");
        // process_string over the stored name, exactly as maxnmin does
        let mut buf = v.name.to_vec();
        buf.push(0);
        let expect: i32 = name.bytes().map(|b| b as i32).sum();
        assert_eq!(p.process_string(&buf), expect);
    }
    // No 7th node.
    assert!(p.find_node_by_id(7).is_none());
    assert!(p.find_node_by_id(0).is_none());

    // Children counts of the seeded shape.
    assert_eq!(p.get_children_count(-1), 1);
    assert_eq!(p.get_children_count(1), 2);
    assert_eq!(p.get_children_count(2), 2);
    assert_eq!(p.get_children_count(3), 1);
    assert_eq!(p.get_children_count(4), 0);
    assert_eq!(p.get_children_count(0), 0);

    // Subtree sums of the seeded shape, bit-exact against the C order of `+=`.
    let total = (10.5f64 + (20.7 + 5.9 + 8.2)) + (15.3 + 12.4);
    assert_eq!(p.calculate_subtree_sum(1).to_bits(), total.to_bits());
    assert_eq!(
        p.calculate_subtree_sum(2).to_bits(),
        (20.7f64 + 5.9 + 8.2).to_bits()
    );
    assert_eq!(p.calculate_subtree_sum(3).to_bits(), (15.3f64 + 12.4).to_bits());
    for id in 4..=6 {
        p.calculate_subtree_sum(id);
    }
    for &id in &probe_ids() {
        p.find_node_by_id(id);
        p.get_children_count(id);
        p.calculate_subtree_sum(id);
    }

    // The 7th add_node lands at index 6 (node_count == 6 after maxnmin).
    assert_eq!(p.add_node(77, 1, "extra", 1.0), 6);
    assert_eq!(p.get_children_count(1), 3);
}

// ---------------------------------------------------------------------- C31

/// C31 — axis H: pre-load state, run `maxnmin` (which clobbers `node_count` to 0
/// and re-seeds 6 nodes), then re-probe; then fill the remaining 94 slots.
#[test]
fn c31_state_carryover_around_maxnmin() {
    let p = Pair::new("C31");
    let mut rng = Rng::new(0x3131_0000_CAFE_0009);
    // 40 random pre-existing nodes (unique ids, acyclic).
    let mut ids = Vec::new();
    for i in 0..40 {
        let id = 1000 + i as i32;
        let parent = if ids.is_empty() || rng.next_u64() % 4 == 0 {
            -1
        } else {
            ids[rng.below(ids.len() as u64) as usize]
        };
        assert_eq!(p.add_node(id, parent, "pre", rng.next_f64_spread()), i);
        ids.push(id);
    }
    for &id in &ids {
        p.find_node_by_id(id);
        p.get_children_count(id);
        p.calculate_subtree_sum(id);
    }

    // maxnmin resets to 0 then adds 6, so indices 0..5 get overwritten and
    // node_count becomes 6 -- the stale nodes at index 6..40 become invisible.
    p.maxnmin(3, 4, 5, 6);
    for &id in &ids {
        assert!(
            p.find_node_by_id(id).is_none(),
            "stale pre-existing node {id} must be invisible after maxnmin"
        );
    }
    assert_eq!(p.get_children_count(-1), 1);
    for id in 1..=6 {
        assert!(p.find_node_by_id(id).is_some());
    }

    // Fill the remaining 94 slots; every index must match.
    for i in 0..94 {
        assert_eq!(p.add_node(5000 + i, 1, "post", i as f64), 6 + i);
    }
    assert_eq!(p.get_children_count(1), 2 + 94);
    // Storage is now full.
    assert_eq!(p.add_node(9999, -1, "overflow", 0.0), -1);
    // ...and maxnmin still works, because it resets node_count first.
    p.maxnmin(1, 1, 1, 1);
    assert_eq!(p.get_children_count(1), 2);
    for &id in &probe_ids() {
        p.find_node_by_id(id);
        p.get_children_count(id);
        p.calculate_subtree_sum(id);
    }
}

// ---------------------------------------------------------------------- C32

/// C32 — axis H: repeated / interleaved `maxnmin` calls. The reset makes it
/// idempotent; verify that, and that arbitrary sequences agree step by step.
#[test]
fn c32_maxnmin_repeated_and_interleaved() {
    let p = Pair::new("C32");
    let mut rng = Rng::new(0x3232_0000_5151_000A);
    for _ in 0..512 {
        let (a, b, c, d) = (rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32());
        let first = p.maxnmin(a, b, c, d);
        assert_eq!(p.maxnmin(a, b, c, d), first, "maxnmin must be idempotent");
        assert_eq!(p.maxnmin(a, b, c, d), first);
        // interleave a different call, then repeat the original
        p.maxnmin(rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32());
        assert_eq!(p.maxnmin(a, b, c, d), first);
    }
    // Interleaved with low-level mutation: add_node between maxnmin calls must
    // not change the next maxnmin result.
    for _ in 0..256 {
        let (a, b, c, d) = (
            rng.range_i32(-20, 20),
            rng.range_i32(-20, 20),
            rng.range_i32(-20, 20),
            rng.range_i32(-20, 20),
        );
        let first = p.maxnmin(a, b, c, d);
        for k in 0..5 {
            p.add_node(7000 + k, 1, "noise", rng.next_f64_spread());
        }
        assert_eq!(p.maxnmin(a, b, c, d), first);
    }
}

// ---------------------------------------------------------------------- C33

/// C33 — whole-API operation-sequence fuzzer. Random programs of random ops over
/// all seven exported entry points, with every return value (and, after each
/// mutation, every `Node` field) compared between C and Rust.
///
/// Node ids are strictly increasing and every `parent_id` is either negative or
/// an id that already exists (hence smaller), so the graph is acyclic by
/// construction and `calculate_subtree_sum` always terminates. Cycles are UB in
/// the C source (no guard) and are covered separately, out of process, by
/// ERRORS.md row E12.
#[test]
fn c33_random_operation_sequences() {
    let mut rng = Rng::new(0x3333_0000_F1FE_000B);
    let classes = double_classes();
    for prog in 0..300 {
        let p = Pair::new(&format!("C33#{prog}"));
        let mut next_id: i32 = 7; // maxnmin owns 1..=6
        let mut live: Vec<i32> = Vec::new();
        for _step in 0..60 {
            match rng.below(9) {
                0 | 1 => {
                    // add_node
                    let id = next_id;
                    next_id += 1;
                    let parent = match rng.below(4) {
                        0 => -1,
                        1 => -999,
                        _ if !live.is_empty() => live[rng.below(live.len() as u64) as usize],
                        _ => -1,
                    };
                    let value = if rng.next_u64() % 10 == 0 {
                        classes[rng.below(classes.len() as u64) as usize]
                    } else {
                        rng.next_f64_spread()
                    };
                    let nlen = rng.below(60) as usize;
                    let mut nm = rng.bytes(nlen, 0x01, 0xFF);
                    nm.push(0);
                    if p.add_node_raw(id, parent, &nm, value) >= 0 {
                        live.push(id);
                    }
                }
                2 => {
                    let id = if !live.is_empty() && rng.next_u64() % 4 != 0 {
                        live[rng.below(live.len() as u64) as usize]
                    } else {
                        rng.next_i32()
                    };
                    p.find_node_by_id(id);
                }
                3 => {
                    let id = if !live.is_empty() && rng.next_u64() % 4 != 0 {
                        live[rng.below(live.len() as u64) as usize]
                    } else {
                        rng.next_i32()
                    };
                    p.get_children_count(id);
                }
                4 => {
                    let id = if !live.is_empty() && rng.next_u64() % 4 != 0 {
                        live[rng.below(live.len() as u64) as usize]
                    } else {
                        rng.next_i32()
                    };
                    let s = p.calculate_subtree_sum(id);
                    p.safe_double_to_int(s);
                }
                5 => {
                    // mutate `active` through the returned Node*
                    if !live.is_empty() {
                        let id = live[rng.below(live.len() as u64) as usize];
                        if p.find_node_by_id(id).is_some() {
                            let v = [0i32, 1, 2, -1, INT_MIN, INT_MAX][rng.below(6) as usize];
                            p.set_active(id, v);
                        }
                    }
                }
                6 => {
                    let len = rng.below(200) as usize;
                    let mut b = rng.bytes(len, 0x00, 0xFF);
                    b.push(0);
                    p.process_string(&b);
                }
                7 => {
                    p.safe_double_to_int(if rng.next_u64() % 4 == 0 {
                        rng.next_f64_bits()
                    } else {
                        rng.next_f64_spread()
                    });
                }
                _ => {
                    // maxnmin wipes the graph back to its six seeded nodes
                    p.maxnmin(
                        rng.range_i32(-30, 30),
                        rng.range_i32(-30, 30),
                        rng.range_i32(-30, 30),
                        rng.range_i32(-30, 30),
                    );
                    live.retain(|_| false);
                    live.extend(1..=6);
                }
            }
        }
        // Final full-state comparison.
        for &id in &live {
            p.find_node_by_id(id);
            p.get_children_count(id);
            p.calculate_subtree_sum(id);
        }
        for &id in &probe_ids() {
            p.find_node_by_id(id);
            p.get_children_count(id);
            p.calculate_subtree_sum(id);
        }
    }
}
