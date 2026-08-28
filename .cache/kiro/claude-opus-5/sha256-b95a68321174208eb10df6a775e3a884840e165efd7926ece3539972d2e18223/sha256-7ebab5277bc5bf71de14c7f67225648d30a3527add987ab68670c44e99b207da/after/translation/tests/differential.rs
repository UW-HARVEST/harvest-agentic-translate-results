//! Differential tests: every function of the C `.so` is compared against the
//! same export of the Rust `.so`, bottom-up through the call hierarchy.
//!
//!   level 0: add/multiply/subtract/modulo_operation, safe_double_to_int
//!   level 1: compute_scaled_value, init_result_array, compare_results_in_array
//!   level 2: process_with_foreach, compute_weighted_sum
//!   level 3: arrayfunc

mod common;

use common::*;

// ---------------------------------------------------------------------------
// Callbacks defined here, so both libraries get a byte-identical `op`.
// ---------------------------------------------------------------------------

unsafe extern "C" fn op_identity_a(a: i32, _b: i32, _u1: i32, _u2: i32) -> i32 {
    a
}
unsafe extern "C" fn op_zero(_a: i32, _b: i32, _u1: i32, _u2: i32) -> i32 {
    0
}
unsafe extern "C" fn op_huge(_a: i32, _b: i32, _u1: i32, _u2: i32) -> i32 {
    i32::MAX
}
unsafe extern "C" fn op_min(_a: i32, _b: i32, _u1: i32, _u2: i32) -> i32 {
    i32::MIN
}
unsafe extern "C" fn op_sum_all(a: i32, b: i32, u1: i32, u2: i32) -> i32 {
    a.wrapping_add(b).wrapping_add(u1).wrapping_add(u2)
}

const OP_NAMES: [&str; 4] = [
    "add_operation",
    "multiply_operation",
    "subtract_operation",
    "modulo_operation",
];

// ---------------------------------------------------------------------------
// Level 0 — leaf operations
// ---------------------------------------------------------------------------

#[test]
fn level0_operations_match() {
    let p = load();
    let vals = interesting_i32();
    let unused = [0i32, 1, -1, i32::MIN, i32::MAX];

    let mut checked = 0usize;
    for &a in &vals {
        for &b in &vals {
            for &u in &unused {
                assert_eq!(
                    p.c.add_operation(a, b, u, u),
                    p.rs.add_operation(a, b, u, u),
                    "add_operation({a}, {b}, {u}, {u})"
                );
                assert_eq!(
                    p.c.multiply_operation(a, b, u, u),
                    p.rs.multiply_operation(a, b, u, u),
                    "multiply_operation({a}, {b}, {u}, {u})"
                );
                assert_eq!(
                    p.c.subtract_operation(a, b, u, u),
                    p.rs.subtract_operation(a, b, u, u),
                    "subtract_operation({a}, {b}, {u}, {u})"
                );
                // `a % b` with a == INT_MIN and b == -1 traps on x86, and the C
                // code does not guard it; only b == 0 is guarded. Skip the one
                // input where the C itself would crash.
                if !(a == i32::MIN && b == -1) {
                    assert_eq!(
                        p.c.modulo_operation(a, b, u, u),
                        p.rs.modulo_operation(a, b, u, u),
                        "modulo_operation({a}, {b}, {u}, {u})"
                    );
                }
                checked += 1;
            }
        }
    }

    let mut rng = Rng::new(0x5EED_1234);
    for _ in 0..20_000 {
        let (a, b, u1, u2) = (
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
        );
        assert_eq!(p.c.add_operation(a, b, u1, u2), p.rs.add_operation(a, b, u1, u2));
        assert_eq!(
            p.c.multiply_operation(a, b, u1, u2),
            p.rs.multiply_operation(a, b, u1, u2)
        );
        assert_eq!(
            p.c.subtract_operation(a, b, u1, u2),
            p.rs.subtract_operation(a, b, u1, u2)
        );
        if !(a == i32::MIN && b == -1) {
            assert_eq!(
                p.c.modulo_operation(a, b, u1, u2),
                p.rs.modulo_operation(a, b, u1, u2)
            );
        }
        checked += 1;
    }
    assert!(checked > 20_000);
}

#[test]
fn level0_safe_double_to_int_matches() {
    let p = load();

    for &d in &interesting_f64() {
        assert_eq!(
            p.c.safe_double_to_int(d),
            p.rs.safe_double_to_int(d),
            "safe_double_to_int({d:?} / bits 0x{:016x})",
            d.to_bits()
        );
    }

    // Dense sweep either side of both clamp boundaries.
    for base in [i32::MAX as f64, i32::MIN as f64, 0.0f64] {
        for step in -600..=600 {
            let d = base + (step as f64) * 0.25;
            assert_eq!(
                p.c.safe_double_to_int(d),
                p.rs.safe_double_to_int(d),
                "safe_double_to_int({d:?})"
            );
        }
    }

    // ULP-level sweep at the boundaries: the C uses `>=` / `<=` against the
    // exactly representable limits, so the neighbours decide clamp vs cast.
    for base in [i32::MAX as f64, -(i32::MIN as f64), i32::MIN as f64] {
        let bits = base.to_bits();
        for delta in -64i64..=64 {
            let b = (bits as i64).wrapping_add(delta) as u64;
            let d = f64::from_bits(b);
            assert_eq!(
                p.c.safe_double_to_int(d),
                p.rs.safe_double_to_int(d),
                "safe_double_to_int(bits 0x{b:016x} = {d:?})"
            );
        }
    }

    // Random bit patterns: hits NaNs, subnormals and huge exponents.
    let mut rng = Rng::new(0xD00D_F00D);
    for _ in 0..200_000 {
        let d = f64::from_bits(rng.next_u64());
        assert_eq!(
            p.c.safe_double_to_int(d),
            p.rs.safe_double_to_int(d),
            "safe_double_to_int(bits 0x{:016x} = {d:?})",
            d.to_bits()
        );
    }

    // Random values inside the int range, where truncation direction matters.
    for _ in 0..200_000 {
        let d = (rng.next_i32() as f64) + (rng.next_u64() % 1000) as f64 / 1000.0;
        assert_eq!(p.c.safe_double_to_int(d), p.rs.safe_double_to_int(d));
        let d = -d;
        assert_eq!(p.c.safe_double_to_int(d), p.rs.safe_double_to_int(d));
    }
}

// ---------------------------------------------------------------------------
// Level 1
// ---------------------------------------------------------------------------

#[test]
fn level1_compute_scaled_value_matches() {
    let p = load();

    for &base in &interesting_i32() {
        for &scale in &interesting_f64() {
            assert_eq!(
                p.c.compute_scaled_value(base, scale),
                p.rs.compute_scaled_value(base, scale),
                "compute_scaled_value({base}, {scale:?} / bits 0x{:016x})",
                scale.to_bits()
            );
        }
    }

    let mut rng = Rng::new(0xBEEF_CAFE);
    for _ in 0..200_000 {
        let base = rng.next_i32();
        let scale = f64::from_bits(rng.next_u64());
        assert_eq!(
            p.c.compute_scaled_value(base, scale),
            p.rs.compute_scaled_value(base, scale),
            "compute_scaled_value({base}, bits 0x{:016x})",
            scale.to_bits()
        );
    }
    // Plausible scale factors, including the ones used internally.
    for _ in 0..100_000 {
        let base = rng.next_i32();
        let scale = (rng.next_i32() as f64) / 1024.0;
        assert_eq!(
            p.c.compute_scaled_value(base, scale),
            p.rs.compute_scaled_value(base, scale)
        );
        for k in [0.75f64, 1.5, 0.8, 0.333, -1.0, 0.0] {
            assert_eq!(
                p.c.compute_scaled_value(base, k),
                p.rs.compute_scaled_value(base, k)
            );
        }
    }
}

#[test]
fn level1_init_result_array_matches() {
    let p = load();
    let mut rng = Rng::new(0x1111_2222);

    // `count` is clamped to 10, and the C only ever reads values[0..count),
    // so a 16-slot buffer is always safe to hand over.
    let counts: Vec<i32> = vec![-100, -3, -1, 0, 1, 2, 5, 9, 10, 11, 15, 100, i32::MAX];

    for &count in &counts {
        for trial in 0..40 {
            let mut vals: Vec<i32> = (0..16).map(|_| rng.next_i32()).collect();
            if trial % 3 == 0 {
                let interesting = interesting_i32();
                for (i, v) in vals.iter_mut().enumerate() {
                    *v = interesting[(i + trial) % interesting.len()];
                }
            }

            let mut c_arr = CResultArray::default();
            let mut rs_arr = CResultArray::default();
            let mut c_vals = vals.clone();
            let mut rs_vals = vals.clone();

            p.c.init_result_array(&mut c_arr, &mut c_vals, count);
            p.rs.init_result_array(&mut rs_arr, &mut rs_vals, count);

            assert!(
                arrays_bit_equal(&c_arr, &rs_arr),
                "init_result_array(count={count}) mismatch\nvalues={vals:?}\nC:  {}\nRS: {}",
                describe(&c_arr),
                describe(&rs_arr)
            );
            assert_eq!(c_vals, rs_vals, "input buffer must not be modified");
        }
    }

    // Pre-populated destination: slots beyond `count` must be left untouched
    // identically by both.
    for &count in &[0i32, 1, 3, 7, 10] {
        let mut c_arr = CResultArray::default();
        for i in 0..MAX_RESULTS {
            c_arr.data[i] = CResult {
                value: (i as i32) * 37 - 11,
                scaled: (i as f64) * -3.25,
                rank: 99 - i as i32,
            };
        }
        c_arr.count = 4;
        let mut rs_arr = c_arr;

        let mut vals: Vec<i32> = (0..16).map(|_| rng.small_i32()).collect();
        let mut vals2 = vals.clone();

        p.c.init_result_array(&mut c_arr, &mut vals, count);
        p.rs.init_result_array(&mut rs_arr, &mut vals2, count);

        assert!(
            arrays_bit_equal(&c_arr, &rs_arr),
            "init_result_array(count={count}) on pre-filled array\nC:  {}\nRS: {}",
            describe(&c_arr),
            describe(&rs_arr)
        );
    }
}

#[test]
fn level1_compare_results_in_array_matches() {
    let p = load();

    // The C only bounds-checks the upper end (`idx >= count`); negative
    // indices flow straight into the pointer comparison. Cover both.
    for count in -2i32..=10 {
        let mut c_arr = CResultArray::default();
        c_arr.count = count;
        let mut rs_arr = c_arr;

        for i1 in -4i32..=12 {
            for i2 in -4i32..=12 {
                let c = p.c.compare_results_in_array(&mut c_arr, i1, i2);
                let r = p.rs.compare_results_in_array(&mut rs_arr, i1, i2);
                assert_eq!(
                    c, r,
                    "compare_results_in_array(count={count}, {i1}, {i2}): C={c} RS={r}"
                );
            }
        }
        assert!(arrays_bit_equal(&c_arr, &rs_arr), "array must be unchanged");
    }

    // The comparison is on addresses, so the stored payload is irrelevant —
    // verify that too.
    let mut rng = Rng::new(0x9999_0001);
    for _ in 0..200 {
        let mut c_arr = CResultArray::default();
        for i in 0..MAX_RESULTS {
            c_arr.data[i] = CResult {
                value: rng.next_i32(),
                scaled: f64::from_bits(rng.next_u64()),
                rank: rng.next_i32(),
            };
        }
        c_arr.count = (rng.next_u64() % 14) as i32 - 2;
        let mut rs_arr = c_arr;
        for i1 in -2i32..=11 {
            for i2 in -2i32..=11 {
                assert_eq!(
                    p.c.compare_results_in_array(&mut c_arr, i1, i2),
                    p.rs.compare_results_in_array(&mut rs_arr, i1, i2),
                    "compare_results_in_array(count={}, {i1}, {i2})",
                    c_arr.count
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Level 2
// ---------------------------------------------------------------------------

fn random_array(rng: &mut Rng, count: i32, wild: bool) -> CResultArray {
    let mut a = CResultArray::default();
    for i in 0..MAX_RESULTS {
        a.data[i] = CResult {
            value: if wild { rng.next_i32() } else { rng.small_i32() },
            scaled: if wild {
                f64::from_bits(rng.next_u64())
            } else {
                (rng.small_i32() as f64) * 1.5
            },
            rank: if wild { rng.next_i32() } else { i as i32 },
        };
    }
    a.count = count;
    a
}

#[test]
fn level2_process_with_foreach_matches() {
    let p = load();
    let mut rng = Rng::new(0xABCD_0007);

    // Callbacks defined in this crate: identical code for both libraries.
    let local_ops: [(&str, OpFn); 5] = [
        ("identity_a", op_identity_a),
        ("zero", op_zero),
        ("huge", op_huge),
        ("min", op_min),
        ("sum_all", op_sum_all),
    ];

    // `count` must stay within 0..=10; outside that the C macro walks off the
    // end of `data`, which is undefined and not a behaviour to pin down.
    for count in 0i32..=MAX_RESULTS as i32 {
        for wild in [false, true] {
            for trial in 0..12 {
                let base = random_array(&mut rng, count, wild);

                // Each library invoked with its OWN exported operation, as
                // `arrayfunc` does internally.
                for name in OP_NAMES {
                    let mut c_arr = base;
                    let mut rs_arr = base;
                    let c_tot = p.c.process_with_foreach(&mut c_arr, p.c.op_ptr(name));
                    let rs_tot = p.rs.process_with_foreach(&mut rs_arr, p.rs.op_ptr(name));
                    assert_eq!(
                        c_tot, rs_tot,
                        "process_with_foreach({name}, count={count}, wild={wild}, trial={trial}) total"
                    );
                    assert!(
                        arrays_bit_equal(&c_arr, &rs_arr),
                        "process_with_foreach({name}, count={count}, wild={wild}) array\nC:  {}\nRS: {}",
                        describe(&c_arr),
                        describe(&rs_arr)
                    );
                }

                // Cross-linked: C's callback driven by the Rust library and
                // vice versa, plus locally defined callbacks.
                for name in OP_NAMES {
                    let mut c_arr = base;
                    let mut rs_arr = base;
                    let c_tot = p.c.process_with_foreach(&mut c_arr, p.rs.op_ptr(name));
                    let rs_tot = p.rs.process_with_foreach(&mut rs_arr, p.c.op_ptr(name));
                    assert_eq!(c_tot, rs_tot, "cross-linked {name} count={count}");
                    assert!(arrays_bit_equal(&c_arr, &rs_arr), "cross-linked {name} array");
                }

                for (label, op) in local_ops {
                    let mut c_arr = base;
                    let mut rs_arr = base;
                    let c_tot = p.c.process_with_foreach(&mut c_arr, op);
                    let rs_tot = p.rs.process_with_foreach(&mut rs_arr, op);
                    assert_eq!(
                        c_tot, rs_tot,
                        "process_with_foreach(local {label}, count={count}, wild={wild}) total"
                    );
                    assert!(
                        arrays_bit_equal(&c_arr, &rs_arr),
                        "process_with_foreach(local {label}, count={count}) array\nC:  {}\nRS: {}",
                        describe(&c_arr),
                        describe(&rs_arr)
                    );
                }
            }
        }
    }

    // Repeated application, the way `arrayfunc` chains all four operations
    // over the same array.
    for trial in 0..300 {
        let count = (trial % 11) as i32;
        let mut c_arr = random_array(&mut rng, count, trial % 2 == 0);
        let mut rs_arr = c_arr;
        for round in 0..4 {
            let name = OP_NAMES[round];
            let c_tot = p.c.process_with_foreach(&mut c_arr, p.c.op_ptr(name));
            let rs_tot = p.rs.process_with_foreach(&mut rs_arr, p.rs.op_ptr(name));
            assert_eq!(c_tot, rs_tot, "chained {name} trial={trial}");
            assert!(
                arrays_bit_equal(&c_arr, &rs_arr),
                "chained {name} trial={trial}\nC:  {}\nRS: {}",
                describe(&c_arr),
                describe(&rs_arr)
            );
        }
    }
}

#[test]
fn level2_compute_weighted_sum_matches() {
    let p = load();
    let mut rng = Rng::new(0x2468_ACE0);

    for count in 0i32..=MAX_RESULTS as i32 {
        for wild in [false, true] {
            for _ in 0..200 {
                let mut c_arr = random_array(&mut rng, count, wild);
                let mut rs_arr = c_arr;
                assert_eq!(
                    p.c.compute_weighted_sum(&mut c_arr),
                    p.rs.compute_weighted_sum(&mut rs_arr),
                    "compute_weighted_sum(count={count}, wild={wild})\n{}",
                    describe(&c_arr)
                );
                assert!(arrays_bit_equal(&c_arr, &rs_arr), "array must be unchanged");
            }
        }
    }

    // Saturating paths: values large enough that value*weight*0.8 clamps.
    for count in 1i32..=MAX_RESULTS as i32 {
        for &v in &interesting_i32() {
            let mut c_arr = CResultArray::default();
            for i in 0..MAX_RESULTS {
                c_arr.data[i] = CResult {
                    value: v,
                    scaled: 0.0,
                    rank: i as i32,
                };
            }
            c_arr.count = count;
            let mut rs_arr = c_arr;
            assert_eq!(
                p.c.compute_weighted_sum(&mut c_arr),
                p.rs.compute_weighted_sum(&mut rs_arr),
                "compute_weighted_sum(count={count}, all values={v})"
            );
        }
    }

    // count == 0 with a negative count too: the C loop simply does not run.
    for count in [-1i32, -5, i32::MIN] {
        let mut c_arr = random_array(&mut rng, count, false);
        let mut rs_arr = c_arr;
        assert_eq!(
            p.c.compute_weighted_sum(&mut c_arr),
            p.rs.compute_weighted_sum(&mut rs_arr),
            "compute_weighted_sum(count={count})"
        );
    }
}

// ---------------------------------------------------------------------------
// Level 3 — public entry point
// ---------------------------------------------------------------------------

#[test]
fn level3_arrayfunc_matches_interesting_inputs() {
    let p = load();
    let vals = interesting_i32();

    for &p1 in &vals {
        for &p2 in &vals {
            for &p3 in &vals {
                for &p4 in &vals {
                    // `param4 / 2` is fine for every i32, and `param3 * 2` and
                    // `param1 + param2` merely wrap; no input is skipped here.
                    let c = p.c.arrayfunc(p1, p2, p3, p4);
                    let r = p.rs.arrayfunc(p1, p2, p3, p4);
                    assert_eq!(c, r, "arrayfunc({p1}, {p2}, {p3}, {p4}): C={c} RS={r}");
                }
            }
        }
    }
}

#[test]
fn level3_arrayfunc_matches_random_inputs() {
    let p = load();
    let mut rng = Rng::new(0x7777_1357);

    for _ in 0..150_000 {
        let (a, b, c4, d) = (
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
        );
        assert_eq!(
            p.c.arrayfunc(a, b, c4, d),
            p.rs.arrayfunc(a, b, c4, d),
            "arrayfunc({a}, {b}, {c4}, {d})"
        );
    }

    for _ in 0..150_000 {
        let (a, b, c4, d) = (
            rng.small_i32(),
            rng.small_i32(),
            rng.small_i32(),
            rng.small_i32(),
        );
        assert_eq!(
            p.c.arrayfunc(a, b, c4, d),
            p.rs.arrayfunc(a, b, c4, d),
            "arrayfunc({a}, {b}, {c4}, {d})"
        );
    }

    // Mixed magnitudes: one wild parameter among small ones.
    for _ in 0..100_000 {
        let mut v = [
            rng.small_i32(),
            rng.small_i32(),
            rng.small_i32(),
            rng.small_i32(),
        ];
        v[(rng.next_u64() % 4) as usize] = rng.next_i32();
        assert_eq!(
            p.c.arrayfunc(v[0], v[1], v[2], v[3]),
            p.rs.arrayfunc(v[0], v[1], v[2], v[3]),
            "arrayfunc({:?})",
            v
        );
    }
}

#[test]
fn level3_arrayfunc_dense_small_grid() {
    let p = load();
    // Exhaustive over a contiguous small cube, to catch off-by-one behaviour
    // in the FOREACH walk and the rank/weight arithmetic.
    for p1 in -6i32..=6 {
        for p2 in -6i32..=6 {
            for p3 in -6i32..=6 {
                for p4 in -6i32..=6 {
                    assert_eq!(
                        p.c.arrayfunc(p1, p2, p3, p4),
                        p.rs.arrayfunc(p1, p2, p3, p4),
                        "arrayfunc({p1}, {p2}, {p3}, {p4})"
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Exported-symbol parity
// ---------------------------------------------------------------------------

#[test]
fn every_c_export_is_present_in_rust() {
    let p = load();
    // Both libraries must resolve each documented export; `sym` panics if not.
    for name in [
        "add_operation",
        "multiply_operation",
        "subtract_operation",
        "modulo_operation",
        "safe_double_to_int",
        "compute_scaled_value",
        "compare_results_in_array",
        "init_result_array",
        "process_with_foreach",
        "compute_weighted_sum",
        "arrayfunc",
    ] {
        let _ = p.c.op_ptr(name);
        let _ = p.rs.op_ptr(name);
    }
}
