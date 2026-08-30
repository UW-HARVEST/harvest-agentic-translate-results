// Phase B — valid-path differential tests.
//
// One test per row of CONFIGS.md (C1..C15, minus C11 which needs a virgin
// process and lives in tests/first_call.rs). Both libraries are driven only
// through their exported `driver` symbol.

use crate::common::*;
use crate::Case;

// --- C1: the one and only accepting configuration ---------------------------

fn c1_success_path_x1_y2_z3() {
    for _ in 0..64 {
        assert_same_and_eq(1, 2, 3, "Ok!\nResult: 0\n");
    }
}

// --- C2: x != 1, y and z fully random --------------------------------------

fn c2_x_invalid_y_z_random() {
    let mut rng = Rng::new(SEED ^ 0xC2);
    for _ in 0..2000 {
        let x = rng.i32_except(1);
        let y = rng.next_i32();
        let z = rng.next_i32();
        assert_same(x, y, z);
    }
}

// --- C3: only x invalid, rest valid ---------------------------------------

fn c3_only_x_invalid() {
    let mut rng = Rng::new(SEED ^ 0xC3);
    for _ in 0..1000 {
        let x = rng.i32_except(1);
        assert_same_and_eq(x, 2, 3, "Error: x != 1\nOperation failed\nResult: 1\n");
    }
}

// --- C4: x valid, y invalid, z random -------------------------------------

fn c4_y_invalid_z_random() {
    let mut rng = Rng::new(SEED ^ 0xC4);
    for _ in 0..2000 {
        let y = rng.i32_except(2);
        let z = rng.next_i32();
        assert_same(1, y, z);
    }
}

// --- C5: only y invalid ---------------------------------------------------

fn c5_only_y_invalid() {
    let mut rng = Rng::new(SEED ^ 0xC5);
    for _ in 0..1000 {
        let y = rng.i32_except(2);
        assert_same_and_eq(
            1,
            y,
            3,
            "Error: x == 1 but y != 2\nOperation failed\nResult: 2\n",
        );
    }
}

// --- C6: only z invalid ---------------------------------------------------

fn c6_only_z_invalid() {
    let mut rng = Rng::new(SEED ^ 0xC6);
    for _ in 0..2000 {
        let z = rng.i32_except(3);
        assert_same_and_eq(
            1,
            2,
            z,
            "Error: x == 1 and y == 2, but z != 3\nOperation failed\nResult: 3\n",
        );
    }
}

// --- C7: fully unconstrained random triples -------------------------------

fn c7_unconstrained_random_triples() {
    let mut rng = Rng::new(SEED ^ 0xC7);
    for _ in 0..4000 {
        let (x, y, z) = (rng.next_i32(), rng.next_i32(), rng.next_i32());
        assert_same(x, y, z);
    }
}

// --- C8: exhaustive small neighbourhood around the accepting point --------

fn c8_exhaustive_small_neighbourhood() {
    let mut seen_ok = 0usize;
    for x in -4..=8 {
        for y in -4..=8 {
            for z in -4..=8 {
                let out = assert_same(x, y, z);
                if out == b"Ok!\nResult: 0\n" {
                    seen_ok += 1;
                }
            }
        }
    }
    // Exactly one point in the cube may succeed: (1, 2, 3).
    assert_eq!(seen_ok, 1, "exactly one accepting point expected in the cube");
}

// --- C9: boundary / extreme value cross-product ---------------------------

const EXTREMES: [i32; 12] = [
    i32::MIN,
    i32::MIN + 1,
    -2,
    -1,
    0,
    1,
    2,
    3,
    4,
    123,
    i32::MAX - 1,
    i32::MAX,
];

fn c9_extreme_value_cross_product() {
    for &x in &EXTREMES {
        for &y in &EXTREMES {
            for &z in &EXTREMES {
                assert_same(x, y, z);
            }
        }
    }
}

// --- C10: randomized but biased toward the interesting constants ----------

fn c10_biased_random_mixed_validity() {
    let mut rng = Rng::new(SEED ^ 0xCA);
    let mut outcomes = [0usize; 4];
    let mut validity_patterns = [0usize; 8];

    for i in 0..4000u32 {
        // Pick the per-argument validity pattern first (sweeping all 8
        // combinations of valid/invalid x, y, z), then draw a random *value*
        // consistent with it. This guarantees every validity combination —
        // including the all-valid one — is densely covered, instead of relying
        // on three independent lucky draws.
        let pattern = (i % 8) as u8;
        let x = if pattern & 0b100 != 0 {
            1
        } else {
            rng.i32_except(1)
        };
        let y = if pattern & 0b010 != 0 {
            2
        } else {
            rng.i32_except(2)
        };
        let z = if pattern & 0b001 != 0 {
            3
        } else {
            rng.i32_except(3)
        };
        validity_patterns[pattern as usize] += 1;

        let out = assert_same(x, y, z);
        let idx = match out.as_slice() {
            b"Ok!\nResult: 0\n" => 0,
            o if o.starts_with(b"Error: x != 1\n") => 1,
            o if o.starts_with(b"Error: x == 1 but y != 2\n") => 2,
            _ => 3,
        };
        outcomes[idx] += 1;
    }

    // Also feed purely value-biased triples (extremes, 123, small ints), which
    // is where value-dependent bugs would hide.
    for _ in 0..2000 {
        let x = rng.interesting_i32();
        let y = rng.interesting_i32();
        let z = rng.interesting_i32();
        assert_same(x, y, z);
    }

    for (p, &n) in validity_patterns.iter().enumerate() {
        assert!(n > 0, "validity pattern {p:03b} never generated");
    }
    // All four outcome classes of `multi_stage` must have been reached.
    for (i, &n) in outcomes.iter().enumerate() {
        assert!(n > 0, "outcome class {i} never exercised: {outcomes:?}");
    }
}

// --- C12: randomized multi-call sequences (persistent `static y`) ----------

fn c12_randomized_call_sequences() {
    let mut rng = Rng::new(SEED ^ 0x12);
    for _ in 0..200 {
        let calls: Vec<(i32, i32, i32)> = (0..64)
            .map(|_| {
                (
                    rng.interesting_i32(),
                    rng.interesting_i32(),
                    rng.interesting_i32(),
                )
            })
            .collect();
        let out = assert_same_seq(&calls);
        // The output must be exactly the concatenation of the per-call model:
        // no state may leak between calls in either implementation.
        let model: String = calls
            .iter()
            .map(|&(x, y, z)| expected_output(x, y, z))
            .collect();
        assert_eq!(
            String::from_utf8_lossy(&out),
            model,
            "sequence output diverged from the per-call model"
        );
    }
}

// --- C13: success/failure alternation -------------------------------------

fn c13_success_failure_alternation() {
    let patterns: Vec<Vec<(i32, i32, i32)>> = vec![
        vec![(1, 2, 3), (0, 0, 0), (1, 2, 3)],
        vec![(0, 0, 0), (1, 2, 3), (0, 0, 0)],
        vec![(1, 2, 3), (1, 999, 3), (1, 2, 3)],
        vec![(1, 999, 3), (1, 2, 3)],
        vec![(1, 2, 999), (1, 2, 3), (1, 2, 999)],
        vec![(1, 2, 3), (1, 2, 3), (1, 2, 3), (1, 2, 3)],
        vec![(1, 123, 3), (1, 2, 3)], // y clobbered with the C initialiser
        vec![(i32::MIN, i32::MAX, 0), (1, 2, 3), (1, i32::MIN, 3)],
    ];
    for p in &patterns {
        let out = assert_same_seq(p);
        let model: String = p.iter().map(|&(x, y, z)| expected_output(x, y, z)).collect();
        assert_eq!(String::from_utf8_lossy(&out), model, "pattern {p:?}");
    }
}

// --- C14: the `Result: %d` conversion for every reachable status code ------

fn c14_result_conversion_all_status_codes() {
    let cases: [(i32, i32, i32, i32); 4] = [
        (1, 2, 3, 0),
        (7, 2, 3, 1),
        (1, 7, 3, 2),
        (1, 2, 7, 3),
    ];
    for (x, y, z, code) in cases {
        let out = assert_same(x, y, z);
        let s = String::from_utf8(out).expect("ascii output");
        assert!(
            s.ends_with(&format!("Result: {code}\n")),
            "driver({x}, {y}, {z}) should end with `Result: {code}`, got {s:?}"
        );
    }
}

// --- C15: exact byte content, no stray prefix/suffix ----------------------

fn c15_exact_bytes_no_stray_output() {
    // Every distinct message the C source can emit, pinned byte-for-byte.
    assert_same_and_eq(1, 2, 3, "Ok!\nResult: 0\n");
    assert_same_and_eq(2, 2, 3, "Error: x != 1\nOperation failed\nResult: 1\n");
    assert_same_and_eq(
        1,
        3,
        3,
        "Error: x == 1 but y != 2\nOperation failed\nResult: 2\n",
    );
    assert_same_and_eq(
        1,
        2,
        4,
        "Error: x == 1 and y == 2, but z != 3\nOperation failed\nResult: 3\n",
    );

    // Lengths agree too (guards against trailing NUL / missing newline bugs).
    let mut rng = Rng::new(SEED ^ 0x15);
    for _ in 0..500 {
        let (x, y, z) = (
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
        let c_out = run_one(c_lib(), x, y, z);
        let r_out = run_one(rust_lib(), x, y, z);
        assert_eq!(
            c_out.len(),
            r_out.len(),
            "byte-length mismatch for driver({x}, {y}, {z})"
        );
        assert_eq!(c_out, r_out);
        assert!(c_out.ends_with(b"\n"), "output must end with a newline");
        assert!(!c_out.contains(&0), "output must not contain a NUL byte");
    }
}

/// Registry of this module's cases, in execution order.
pub fn cases() -> Vec<Case> {
    vec![
        ("c1_success_path_x1_y2_z3", c1_success_path_x1_y2_z3 as fn()),
        ("c2_x_invalid_y_z_random", c2_x_invalid_y_z_random as fn()),
        ("c3_only_x_invalid", c3_only_x_invalid as fn()),
        ("c4_y_invalid_z_random", c4_y_invalid_z_random as fn()),
        ("c5_only_y_invalid", c5_only_y_invalid as fn()),
        ("c6_only_z_invalid", c6_only_z_invalid as fn()),
        ("c7_unconstrained_random_triples", c7_unconstrained_random_triples as fn()),
        ("c8_exhaustive_small_neighbourhood", c8_exhaustive_small_neighbourhood as fn()),
        ("c9_extreme_value_cross_product", c9_extreme_value_cross_product as fn()),
        ("c10_biased_random_mixed_validity", c10_biased_random_mixed_validity as fn()),
        ("c12_randomized_call_sequences", c12_randomized_call_sequences as fn()),
        ("c13_success_failure_alternation", c13_success_failure_alternation as fn()),
        ("c14_result_conversion_all_status_codes", c14_result_conversion_all_status_codes as fn()),
        ("c15_exact_bytes_no_stray_output", c15_exact_bytes_no_stray_output as fn()),
    ]
}
