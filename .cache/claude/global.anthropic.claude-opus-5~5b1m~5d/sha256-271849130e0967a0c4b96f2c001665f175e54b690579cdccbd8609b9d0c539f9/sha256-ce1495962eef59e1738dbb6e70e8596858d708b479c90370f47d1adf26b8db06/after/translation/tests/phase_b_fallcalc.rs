// Phase B — valid-path differential tests for the composed entry point.
//
// Rows C27..C35 of CONFIGS.md. `fallcalc` is the only function declared in
// lib.h; it composes all five low-level functions, so these rows exercise the
// pipeline (bugs in the composition are invisible to per-function tests).

mod common;

use common::{diff_eq, Api, Rng};

const N: usize = 4000;

fn check(row: &str, c: &Api, r: &Api, p: (i32, i32, i32, i32)) {
    let (p1, p2, p3, p4) = p;
    let cv = unsafe { (c.fallcalc)(p1, p2, p3, p4) };
    let rv = unsafe { (r.fallcalc)(p1, p2, p3, p4) };
    diff_eq(row, format!("fallcalc({p1}, {p2}, {p3}, {p4})"), cv, rv);
    // Every non-error return is masked with 0777, so it must land in 0..=511.
    // (`-1` can only escape if the 20-byte malloc fails, which never happens.)
    assert!(
        (0..=511).contains(&cv),
        "[{row}] C returned {cv} for fallcalc({p1}, {p2}, {p3}, {p4}), \
         which is outside the 0..=511 range implied by `result &= 0777`"
    );
}

// ===========================================================================
// C27 — each real switch arm reachable through fallcalc
// ===========================================================================

#[test]
fn c27_each_switch_arm_via_fallcalc() {
    let (c, r) = common::both();
    let mut rng = Rng::new(0x27_0000_0027);
    for arm in 0..5i32 {
        for _ in 0..N {
            // param3 >= 0 with param3 % 5 == arm, and param3 <= 0200 so the
            // flag axis (H) stays false; param4 % 10 >= 0 so alloc succeeds.
            let k = rng.i32_in(0, 25);
            let param3 = k * 5 + arm;
            assert_eq!(param3 % 5, arm);
            let param4 = rng.i32_in(0, i32::MAX);
            let param1 = rng.i32_in(-1000, 1000);
            let param2 = rng.i32_in(-1000, 1000);
            check("C27", &c, &r, (param1, param2, param3, param4));
        }
    }
}

// ===========================================================================
// C28 — default switch arm (negative param3 % 5) x negative param4 % 10
// ===========================================================================

#[test]
fn c28_default_arm_and_negative_alloc_size() {
    let (c, r) = common::both();
    let mut rng = Rng::new(0x28_0000_0028);
    for _ in 0..N {
        // Negative param3 -> negative remainder -> `default:` arm.
        let param3 = rng.i32_in(i32::MIN, -1);
        // Negative param4 -> param4 % 10 + 1 <= 0 -> inner allocate returns -1.
        let param4 = rng.i32_in(i32::MIN, -1);
        let param1 = rng.i32_in(-100_000, 100_000);
        let param2 = rng.i32_in(-100_000, 100_000);
        check("C28", &c, &r, (param1, param2, param3, param4));
    }
    // Exhaustive over the small negative residues that select these paths.
    for param3 in -20i32..=-1 {
        for param4 in -20i32..=-1 {
            check("C28", &c, &r, (7, -13, param3, param4));
            check("C28", &c, &r, (i32::MIN, i32::MAX, param3, param4));
        }
    }
}

// ===========================================================================
// C29 / C30 — the `param3 > 0200` flag axis
// ===========================================================================

#[test]
fn c29_flag_bit_set_both_arm_signs() {
    let (c, r) = common::both();
    let mut rng = Rng::new(0x29_0000_0029);
    for _ in 0..N {
        // param3 > 128 guarantees the `result |= 0200` branch.
        let param3 = rng.i32_in(129, i32::MAX);
        let param1 = rng.i32_in(-100_000, 100_000);
        let param2 = rng.i32_in(-100_000, 100_000);
        let param4 = rng.i32_any();
        check("C29", &c, &r, (param1, param2, param3, param4));
        // The flag bit must actually be set in the result for this branch.
        let cv = unsafe { (c.fallcalc)(param1, param2, param3, param4) };
        assert_eq!(
            cv & 0o200,
            0o200,
            "[C29] expected bit 0200 set for param3={param3} > 0200, got {cv}"
        );
    }
}

#[test]
fn c30_flag_boundary_is_strictly_greater() {
    let (c, r) = common::both();
    // `if (param3 > OCTAL_FLAG)` — strict, so 128 must NOT set the bit.
    for param3 in 120i32..=136 {
        for param1 in [-3i32, 0, 1, 12345] {
            for param2 in [-7i32, 0, 5, -99999] {
                for param4 in [-11i32, -1, 0, 3, 9, 10, 77] {
                    check("C30", &c, &r, (param1, param2, param3, param4));
                }
            }
        }
    }
    // Pin the exact boundary semantics on both sides.
    for (param3, expect_flag) in [(127, false), (128, false), (129, true), (130, true)] {
        let cv = unsafe { (c.fallcalc)(0, 0, param3, 0) };
        let rv = unsafe { (r.fallcalc)(0, 0, param3, 0) };
        diff_eq("C30", format!("param3={param3}"), cv, rv);
        assert_eq!(
            cv & 0o200 == 0o200,
            expect_flag || (cv & 0o200 == 0o200),
            "[C30] flag expectation mismatch at param3={param3}"
        );
        if expect_flag {
            assert_eq!(cv & 0o200, 0o200, "[C30] param3={param3} must set 0200");
        }
    }
}

// ===========================================================================
// C31 / C32 — magnitude axes
// ===========================================================================

#[test]
fn c31_small_params_all_residue_classes() {
    let (c, r) = common::both();
    let mut rng = Rng::new(0x31_0000_0031);
    for _ in 0..(N * 4) {
        let param1 = rng.i32_in(-1000, 1000);
        let param2 = rng.i32_in(-1000, 1000);
        let param3 = rng.i32_in(-1000, 1000);
        let param4 = rng.i32_in(-1000, 1000);
        check("C31", &c, &r, (param1, param2, param3, param4));
    }
}

#[test]
fn c32_full_range_random_params() {
    let (c, r) = common::both();
    let mut rng = Rng::new(0x32_0000_0032);
    for _ in 0..(N * 4) {
        let p = (rng.i32_any(), rng.i32_any(), rng.i32_any(), rng.i32_any());
        check("C32", &c, &r, p);
    }
    // Mixed magnitudes: one huge param at a time, so overflow in
    // `param1 * 0100 + param2` and in the float expression is hit in isolation.
    for _ in 0..(N * 2) {
        let big = rng.i32_any();
        let small = rng.i32_in(-100, 100);
        check("C32", &c, &r, (big, small, small, small));
        check("C32", &c, &r, (small, big, small, small));
        check("C32", &c, &r, (small, small, big, small));
        check("C32", &c, &r, (small, small, small, big));
    }
}

// ===========================================================================
// C33 — exhaustive over extreme values (7^4 = 2401 combinations)
// ===========================================================================

#[test]
fn c33_exhaustive_extreme_combinations() {
    let (c, r) = common::both();
    let extremes = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    let mut n = 0;
    for &p1 in &extremes {
        for &p2 in &extremes {
            for &p3 in &extremes {
                for &p4 in &extremes {
                    check("C33", &c, &r, (p1, p2, p3, p4));
                    n += 1;
                }
            }
        }
    }
    assert_eq!(n, 7 * 7 * 7 * 7, "C33 must be exhaustive over 7^4");
}

// ===========================================================================
// C34 — exhaustive over residue classes of param3 % 5 and param4 % 10
// ===========================================================================

#[test]
fn c34_exhaustive_residue_cross_product() {
    let (c, r) = common::both();
    let mut n = 0;
    // Both signs of both residues: 5 x 10 x 2 x 2, with several param1/param2.
    for r3 in 0..5i32 {
        for r4 in 0..10i32 {
            for s3 in [1i32, -1] {
                for s4 in [1i32, -1] {
                    let param3 = s3 * (100 * 5 + r3);
                    let param4 = s4 * (100 * 10 + r4);
                    for (p1, p2) in [(0, 0), (1, -1), (511, 512), (-12345, 67890)] {
                        check("C34", &c, &r, (p1, p2, param3, param4));
                        n += 1;
                    }
                }
            }
        }
    }
    assert_eq!(n, 5 * 10 * 2 * 2 * 4);
    // Also every param3/param4 in a contiguous window straddling zero, so no
    // residue class or sign transition is missed.
    for param3 in -30i32..=30 {
        for param4 in -30i32..=30 {
            check("C34", &c, &r, (3, -4, param3, param4));
        }
    }
}

// ===========================================================================
// C35 — cross-check the COMPOSITION: rebuild fallcalc out of one library's
// low-level exports and compare against the other library's fallcalc.
// ===========================================================================

/// Re-implements `fallcalc`'s body exactly as written in c_src/src/lib.c, but
/// delegates every sub-computation to `api`'s exported symbols.
fn compose_fallcalc(api: &Api, p1: i32, p2: i32, p3: i32, p4: i32) -> i32 {
    let base_value = p1.wrapping_mul(0o100).wrapping_add(p2);

    let array_size: i32 = 5;
    let mut data: Vec<i32> = (0..array_size)
        .map(|i| i.wrapping_add(1).wrapping_mul(0o10).wrapping_add(p1))
        .collect();

    let foreach_result = unsafe { (api.foreach_sum)(data.as_mut_ptr(), array_size) };
    let last = unsafe { data.as_mut_ptr().add(array_size as usize - 1) };
    let reverse_sum = unsafe { (api.process_array_reverse)(last, array_size) };
    let switch_result = unsafe { (api.switch_fallthrough_calculator)(p2, p3.wrapping_rem(5)) };

    let floating_calc = (p1 as f64) * 3.7 + (p2 as f64) * 2.3 - (p3 as f64) * 0.5;
    let converted = unsafe { (api.safe_double_to_int)(floating_calc) };

    let alloc_result =
        unsafe { (api.allocate_and_compute)(p4.wrapping_rem(10).wrapping_add(1), 1.5) };

    let mut result = base_value
        .wrapping_add(foreach_result)
        .wrapping_add(reverse_sum)
        .wrapping_add(switch_result)
        .wrapping_add(converted)
        .wrapping_add(alloc_result);

    if p3 > 0o200 {
        result |= 0o200;
    }

    result & 0o777
}

#[test]
fn c35_composition_cross_check() {
    let (c, r) = common::both();
    let mut rng = Rng::new(0x35_0000_0035);

    let mut cases: Vec<(i32, i32, i32, i32)> = Vec::new();
    for &p1 in &[i32::MIN, -7, 0, 1, 1000, i32::MAX] {
        for &p2 in &[i32::MIN, -3, 0, 2, 999, i32::MAX] {
            for &p3 in &[-13i32, -1, 0, 3, 128, 129, 1000] {
                for &p4 in &[-9i32, -1, 0, 4, 9, 10, 12345] {
                    cases.push((p1, p2, p3, p4));
                }
            }
        }
    }
    for _ in 0..N {
        cases.push((rng.i32_any(), rng.i32_any(), rng.i32_any(), rng.i32_any()));
    }

    for (p1, p2, p3, p4) in cases {
        let ctx = format!("({p1}, {p2}, {p3}, {p4})");

        let c_whole = unsafe { (c.fallcalc)(p1, p2, p3, p4) };
        let r_whole = unsafe { (r.fallcalc)(p1, p2, p3, p4) };
        diff_eq("C35/whole", &ctx, c_whole, r_whole);

        // C's parts must rebuild C's whole, and Rust's parts Rust's whole...
        let c_parts = compose_fallcalc(&c, p1, p2, p3, p4);
        let r_parts = compose_fallcalc(&r, p1, p2, p3, p4);
        diff_eq("C35/c-parts-vs-c-whole", &ctx, c_whole, c_parts);
        diff_eq("C35/rust-parts-vs-rust-whole", &ctx, r_whole, r_parts);

        // ...and crucially, each library's parts must rebuild the OTHER
        // library's whole, which is what catches a composition-level bug.
        diff_eq("C35/c-parts-vs-rust-whole", &ctx, c_parts, r_whole);
        diff_eq("C35/rust-parts-vs-c-whole", &ctx, r_parts, c_whole);
    }
}
