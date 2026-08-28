//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md` (C1..C30). Every test drives BOTH shared
//! libraries through their exported symbols and compares results byte-for-byte,
//! using many randomized inputs per row from a fixed-seed PRNG.

mod common;

use common::*;
use std::os::raw::c_int;

/// Counts the C code special-cases (0 = empty, 1 = single, 10 = capacity).
const COUNTS: &[c_int] = &[0, 1, 2, 3, 9, 10];

/// Values worth feeding into the four `int` operations.
const EDGE_INTS: &[c_int] = &[
    0,
    1,
    -1,
    2,
    -2,
    3,
    -3,
    7,
    -7,
    10,
    -10,
    1000,
    -1000,
    65536,
    i32::MAX,
    i32::MIN,
    i32::MAX - 1,
    i32::MIN + 1,
    1 << 30,
    -(1 << 30),
];

// ===========================================================================
// C1..C4 — the four `operation_func` primitives
// ===========================================================================

/// Drives one binary operation over the edge grid + randomized inputs.
/// `skip` lets a row exclude inputs that legitimately trap (covered in Phase C).
fn diff_binop(
    row: &str,
    pick: impl Fn(&Impl) -> OperationFunc,
    skip: impl Fn(c_int, c_int) -> bool,
) {
    let l = libs();
    let cf = pick(&l.c);
    let rf = pick(&l.rust);

    // Exhaustive over the edge grid, with garbage in the two "unused" slots to
    // prove they really are ignored.
    for &a in EDGE_INTS {
        for &b in EDGE_INTS {
            if skip(a, b) {
                continue;
            }
            for &(u1, u2) in &[(0, 0), (1, -1), (i32::MAX, i32::MIN), (-999, 12345)] {
                unsafe {
                    eq_int(
                        &format!("{row} {a} op {b} (unused={u1},{u2})"),
                        cf(a, b, u1, u2),
                        rf(a, b, u1, u2),
                    );
                }
            }
        }
    }

    // Randomized property-style sweep.
    let mut rng = Rng::new(0xC0FFEE_u64 ^ row.len() as u64);
    for _ in 0..400_000 {
        let a = rng.interesting_i32();
        let b = rng.interesting_i32();
        if skip(a, b) {
            continue;
        }
        let u1 = rng.next_i32();
        let u2 = rng.next_i32();
        unsafe {
            eq_int(
                &format!("{row} random {a} op {b}"),
                cf(a, b, u1, u2),
                rf(a, b, u1, u2),
            );
        }
    }
}

#[test]
fn c1_add_operation() {
    diff_binop("C1 add_operation", |i| i.add_operation, |_, _| false);
}

#[test]
fn c2_multiply_operation() {
    diff_binop("C2 multiply_operation", |i| i.multiply_operation, |_, _| false);
}

#[test]
fn c3_subtract_operation() {
    diff_binop("C3 subtract_operation", |i| i.subtract_operation, |_, _| false);
}

#[test]
fn c4_modulo_operation() {
    // `INT_MIN % -1` raises SIGFPE in both implementations; it is a Phase C row
    // (E2) and is verified out-of-process in `crash_probes.rs`.
    diff_binop(
        "C4 modulo_operation",
        |i| i.modulo_operation,
        |a, b| a == i32::MIN && b == -1,
    );
}

// ===========================================================================
// C5, C6 — safe_double_to_int
// ===========================================================================

fn edge_doubles() -> Vec<f64> {
    let imax = i32::MAX as f64;
    let imin = i32::MIN as f64;
    let mut v = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        0.9999999999,
        -0.9999999999,
        1.5,
        -1.5,
        2.5,
        -2.5,
        -2.9,
        2.9,
        imax,
        imax - 0.5,
        imax - 1.0,
        imax + 1.0,
        imax * 2.0,
        imin,
        imin + 0.5,
        imin + 1.0,
        imin - 1.0,
        imin * 2.0,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        f64::MAX,
        f64::MIN,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::EPSILON,
        1e-300,
        -1e-300,
        1e300,
        -1e300,
        // subnormal
        f64::from_bits(1),
        f64::from_bits(0x8000_0000_0000_0001),
        // signalling NaN and a payload-carrying NaN
        f64::from_bits(0x7FF0_0000_0000_0001),
        f64::from_bits(0xFFF4_0000_0000_1234),
    ];
    // Powers of two either side of the clamp boundaries.
    for e in 0..=40 {
        v.push((2.0f64).powi(e));
        v.push(-(2.0f64).powi(e));
        v.push((2.0f64).powi(e) + 0.25);
        v.push(-(2.0f64).powi(e) - 0.25);
    }
    v
}

#[test]
fn c5_safe_double_to_int_all_classes() {
    let l = libs();
    for d in edge_doubles() {
        unsafe {
            eq_int(
                &format!("C5 safe_double_to_int({d:?} bits=0x{:016x})", d.to_bits()),
                (l.c.safe_double_to_int)(d),
                (l.rust.safe_double_to_int)(d),
            );
        }
    }

    let mut rng = Rng::new(0x5AFE_D00D);
    for _ in 0..1_500_000 {
        let d = rng.interesting_f64();
        unsafe {
            eq_int(
                &format!("C5 random safe_double_to_int(bits=0x{:016x})", d.to_bits()),
                (l.c.safe_double_to_int)(d),
                (l.rust.safe_double_to_int)(d),
            );
        }
    }
}

#[test]
fn c6_safe_double_to_int_truncation_toward_zero() {
    let l = libs();
    let mut rng = Rng::new(0x7_C0DE);
    for _ in 0..400_000 {
        // In-range integer plus a fraction of either sign: C truncates toward
        // zero, so -2.7 -> -2 (not -3).
        let base = rng.range_i32(-2_000_000_000, 2_000_000_000) as f64;
        let frac = (rng.next_u32() as f64) / (u32::MAX as f64); // [0,1]
        for d in [base + frac, base - frac] {
            unsafe {
                eq_int(
                    &format!("C6 trunc safe_double_to_int({d})"),
                    (l.c.safe_double_to_int)(d),
                    (l.rust.safe_double_to_int)(d),
                );
            }
        }
    }
}

// ===========================================================================
// C7 — compute_scaled_value (int x double cross-product)
// ===========================================================================

#[test]
fn c7_compute_scaled_value() {
    let l = libs();
    let scales: Vec<f64> = edge_doubles()
        .into_iter()
        .chain([1.5, 0.75, 0.8, 0.333, -1.5, 3.0, 1.0 / 3.0])
        .collect();

    for &base in EDGE_INTS {
        for &s in &scales {
            unsafe {
                eq_int(
                    &format!("C7 compute_scaled_value({base}, bits=0x{:016x})", s.to_bits()),
                    (l.c.compute_scaled_value)(base, s),
                    (l.rust.compute_scaled_value)(base, s),
                );
            }
        }
    }

    let mut rng = Rng::new(0x5CA1E);
    for _ in 0..600_000 {
        let base = rng.interesting_i32();
        let s = rng.interesting_f64();
        unsafe {
            eq_int(
                &format!(
                    "C7 random compute_scaled_value({base}, bits=0x{:016x})",
                    s.to_bits()
                ),
                (l.c.compute_scaled_value)(base, s),
                (l.rust.compute_scaled_value)(base, s),
            );
        }
    }
}

// ===========================================================================
// C8..C11 — init_result_array
// ===========================================================================

/// Runs `init_result_array` on both libs from the identical dirty start state
/// and compares the whole struct.
fn diff_init(row: &str, seed: u64, values: &[c_int], count: c_int) {
    let l = libs();
    let mut ac = ResultArray::dirty(seed);
    let mut ar = ResultArray::dirty(seed);
    let mut vc: Vec<c_int> = values.to_vec();
    let mut vr: Vec<c_int> = values.to_vec();
    unsafe {
        (l.c.init_result_array)(&mut ac, vc.as_mut_ptr(), count);
        (l.rust.init_result_array)(&mut ar, vr.as_mut_ptr(), count);
    }
    eq_array(&format!("{row} count={count}"), &ac, &ar);
    assert_eq!(vc, vr, "{row}: input array was mutated differently");
}

#[test]
fn c8_init_result_array_normal_counts() {
    let mut rng = Rng::new(0x1417);
    for &count in COUNTS {
        for iter in 0..10_000u64 {
            let vals: Vec<c_int> = (0..10.max(count as usize))
                .map(|_| rng.interesting_i32())
                .collect();
            diff_init("C8 init_result_array", 0x100 + iter, &vals, count);
        }
    }
}

#[test]
fn c9_init_result_array_oversized_count_clamps() {
    let mut rng = Rng::new(0x9999);
    for &count in &[11, 12, 13, 20, 100, 10_000, i32::MAX - 1, i32::MAX] {
        for iter in 0..3000u64 {
            // Only the first 10 entries may ever be read; provide 16 so an
            // off-by-one read would still be in bounds of our allocation and
            // would show up as a value difference rather than a crash.
            let vals: Vec<c_int> = (0..16).map(|_| rng.interesting_i32()).collect();
            diff_init("C9 init_result_array oversized", 0x200 + iter, &vals, count);
        }
    }
}

#[test]
fn c10_init_result_array_on_predirtied_struct() {
    // Distinct dirty seeds ensure the untouched tail (elements >= count) differs
    // per run, so "left alone identically" is really being checked.
    let mut rng = Rng::new(0xD1747);
    for &count in COUNTS {
        for seed in 0..10_000u64 {
            let vals: Vec<c_int> = (0..10).map(|_| rng.interesting_i32()).collect();
            diff_init("C10 init on dirty struct", 0xD000 + seed, &vals, count);
        }
    }
}

#[test]
fn c11_init_result_array_values_longer_than_count() {
    let mut rng = Rng::new(0xB16);
    for &count in COUNTS {
        for iter in 0..5000u64 {
            let vals: Vec<c_int> = (0..64).map(|_| rng.interesting_i32()).collect();
            diff_init("C11 init long values[]", 0xB000 + iter, &vals, count);
        }
    }
}

// ===========================================================================
// C12..C19 — process_with_foreach
// ===========================================================================

/// Builds an identical pair of arrays via each library's own `init_result_array`
/// (so the pair is byte-identical before the operation under test).
fn init_pair(seed: u64, count: c_int, rng: &mut Rng) -> (ResultArray, ResultArray) {
    let l = libs();
    let mut ac = ResultArray::dirty(seed);
    let mut ar = ResultArray::dirty(seed);
    let mut vals: Vec<c_int> = (0..16).map(|_| rng.interesting_i32()).collect();
    unsafe {
        (l.c.init_result_array)(&mut ac, vals.as_mut_ptr(), count);
        (l.rust.init_result_array)(&mut ar, vals.as_mut_ptr(), count);
    }
    eq_array("init_pair precondition", &ac, &ar);
    (ac, ar)
}

fn diff_process(row: &str, op_index: usize, seed_base: u64) {
    let l = libs();
    let (cname, cop) = l.c.ops()[op_index];
    let (_, rop) = l.rust.ops()[op_index];
    let mut rng = Rng::new(seed_base);

    for &count in COUNTS {
        for iter in 0..12_000u64 {
            let (mut ac, mut ar) = init_pair(seed_base + iter, count, &mut rng);
            unsafe {
                let rc = (l.c.process_with_foreach)(&mut ac, Some(cop));
                let rr = (l.rust.process_with_foreach)(&mut ar, Some(rop));
                eq_int(&format!("{row} op={cname} count={count} iter={iter}"), rc, rr);
            }
            eq_array(
                &format!("{row} op={cname} count={count} iter={iter} (struct)"),
                &ac,
                &ar,
            );
        }
    }
}

#[test]
fn c12_process_with_foreach_add() {
    diff_process("C12 process_with_foreach", 0, 0x1200);
}

#[test]
fn c13_process_with_foreach_multiply() {
    diff_process("C13 process_with_foreach", 1, 0x1300);
}

#[test]
fn c14_process_with_foreach_subtract() {
    diff_process("C14 process_with_foreach", 2, 0x1400);
}

#[test]
fn c15_process_with_foreach_modulo() {
    // `rank` is the divisor, so element 0 always has `b == 0` and exercises the
    // `modulo_operation` zero-guard on every single run.
    diff_process("C15 process_with_foreach", 3, 0x1500);
}

// --- C16: an arbitrary, caller-supplied function pointer -------------------
// The FFI accepts *any* pointer value here, so these external callbacks are the
// `operation_func` analogue of an out-of-range enum: values with no
// "valid variant" among the library's own four operations.

unsafe extern "C" fn cb_max(_a: c_int, _b: c_int, _u1: c_int, _u2: c_int) -> c_int {
    i32::MAX
}
unsafe extern "C" fn cb_min(_a: c_int, _b: c_int, _u1: c_int, _u2: c_int) -> c_int {
    i32::MIN
}
unsafe extern "C" fn cb_zero(_a: c_int, _b: c_int, _u1: c_int, _u2: c_int) -> c_int {
    0
}
/// Returns `unused1 + unused2`, which pins down that the library passes literal
/// `0, 0` for the two unused parameters.
unsafe extern "C" fn cb_unused_probe(_a: c_int, _b: c_int, u1: c_int, u2: c_int) -> c_int {
    u1.wrapping_mul(31).wrapping_add(u2).wrapping_add(1)
}
/// Strongly value-dependent, so a wrong `item->value` / `item->rank` shows up.
unsafe extern "C" fn cb_mix(a: c_int, b: c_int, _u1: c_int, _u2: c_int) -> c_int {
    (a.rotate_left(7) ^ b.wrapping_mul(0x9E37_79B9u32 as i32)).wrapping_sub(a)
}
/// Near the `* 0.75` saturation threshold.
unsafe extern "C" fn cb_near_clamp(a: c_int, b: c_int, _u1: c_int, _u2: c_int) -> c_int {
    i32::MAX - (a & 3) - b
}

#[test]
fn c16_process_with_foreach_external_callback() {
    let l = libs();
    let cbs: [(&str, OperationFunc); 6] = [
        ("cb_max", cb_max),
        ("cb_min", cb_min),
        ("cb_zero", cb_zero),
        ("cb_unused_probe", cb_unused_probe),
        ("cb_mix", cb_mix),
        ("cb_near_clamp", cb_near_clamp),
    ];
    let mut rng = Rng::new(0x1600);
    for (name, cb) in cbs {
        for &count in COUNTS {
            for iter in 0..5000u64 {
                let (mut ac, mut ar) = init_pair(0x1600 + iter, count, &mut rng);
                unsafe {
                    let rc = (l.c.process_with_foreach)(&mut ac, Some(cb));
                    let rr = (l.rust.process_with_foreach)(&mut ar, Some(cb));
                    eq_int(&format!("C16 {name} count={count} iter={iter}"), rc, rr);
                }
                eq_array(&format!("C16 {name} count={count} iter={iter} (struct)"), &ac, &ar);
            }
        }
    }
}

#[test]
fn c17_process_with_foreach_hand_set_count() {
    // `count` is written directly into the struct field, bypassing
    // `init_result_array`'s clamp, and deliberately disagrees with how many
    // elements were initialised. This pins down that `FOREACH` reads `count`
    // exactly once.
    let l = libs();
    let mut rng = Rng::new(0x1700);
    for init_count in 0..=10i32 {
        for hand_count in 0..=10i32 {
            for iter in 0..200u64 {
                let (mut ac, mut ar) = init_pair(0x1700 + iter, init_count, &mut rng);
                ac.count = hand_count;
                ar.count = hand_count;
                // Keep ranks non-negative-one so `modulo` cannot trap.
                for k in 0..10 {
                    let r = rng.range_i32(0, 9);
                    ac.data[k].rank = r;
                    ar.data[k].rank = r;
                }
                eq_array("C17 precondition", &ac, &ar);
                for op_index in 0..4 {
                    let (name, cop) = l.c.ops()[op_index];
                    let (_, rop) = l.rust.ops()[op_index];
                    let mut ac2 = ac;
                    let mut ar2 = ar;
                    unsafe {
                        let rc = (l.c.process_with_foreach)(&mut ac2, Some(cop));
                        let rr = (l.rust.process_with_foreach)(&mut ar2, Some(rop));
                        eq_int(
                            &format!("C17 {name} init={init_count} hand={hand_count}"),
                            rc,
                            rr,
                        );
                    }
                    eq_array(
                        &format!("C17 {name} init={init_count} hand={hand_count} (struct)"),
                        &ac2,
                        &ar2,
                    );
                }
            }
        }
    }
}

#[test]
fn c18_process_with_foreach_repeated_passes() {
    // State carries over between passes: each pass sees the previous pass's
    // rewritten `value`/`scaled`.
    let l = libs();
    let mut rng = Rng::new(0x1800);
    for &count in COUNTS {
        for iter in 0..8000u64 {
            let (mut ac, mut ar) = init_pair(0x1800 + iter, count, &mut rng);
            let passes = rng.range_i32(2, 5);
            for p in 0..passes {
                let op_index = rng.range_i32(0, 3) as usize;
                let (name, cop) = l.c.ops()[op_index];
                let (_, rop) = l.rust.ops()[op_index];
                unsafe {
                    let rc = (l.c.process_with_foreach)(&mut ac, Some(cop));
                    let rr = (l.rust.process_with_foreach)(&mut ar, Some(rop));
                    eq_int(
                        &format!("C18 pass {p}/{passes} {name} count={count} iter={iter}"),
                        rc,
                        rr,
                    );
                }
                eq_array(
                    &format!("C18 pass {p}/{passes} {name} count={count} iter={iter} (struct)"),
                    &ac,
                    &ar,
                );
            }
        }
    }
}

#[test]
fn c19_process_with_foreach_clamp_boundaries() {
    // Values whose `* 0.75` lands exactly on .0/.25/.5/.75 or on the saturation
    // thresholds of `safe_double_to_int`.
    let l = libs();
    let interesting: &[c_int] = &[
        0,
        1,
        2,
        3,
        4,
        -1,
        -2,
        -3,
        -4,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        // 0.75 * x == INT32_MAX  =>  x ~= 2863311530 (out of int range), so the
        // clamp is only reachable via the callback rows; these still probe the
        // rounding boundaries.
        1431655765,
        -1431655765,
        1431655764,
        -1431655766,
        2863311,
        -2863311,
    ];
    for op_index in 0..4 {
        let (name, cop) = l.c.ops()[op_index];
        let (_, rop) = l.rust.ops()[op_index];
        for &v in interesting {
            for count in 1..=10i32 {
                let mut ac = ResultArray::dirty(0x1900 + count as u64);
                // Fill every element with the same interesting value; ranks 0..n.
                for k in 0..10 {
                    ac.data[k].value = v;
                    ac.data[k].rank = k as c_int;
                    ac.data[k].scaled = v as f64 * 1.5;
                }
                ac.count = count;
                let mut ar = ac;
                unsafe {
                    let rc = (l.c.process_with_foreach)(&mut ac, Some(cop));
                    let rr = (l.rust.process_with_foreach)(&mut ar, Some(rop));
                    eq_int(&format!("C19 {name} v={v} count={count}"), rc, rr);
                }
                eq_array(&format!("C19 {name} v={v} count={count} (struct)"), &ac, &ar);
            }
        }
    }
}

// ===========================================================================
// C20..C23 — compute_weighted_sum
// ===========================================================================

#[test]
fn c20_compute_weighted_sum_fresh() {
    let l = libs();
    let mut rng = Rng::new(0x2000);
    for &count in COUNTS {
        for iter in 0..20_000u64 {
            let (mut ac, mut ar) = init_pair(0x2000 + iter, count, &mut rng);
            unsafe {
                let rc = (l.c.compute_weighted_sum)(&mut ac);
                let rr = (l.rust.compute_weighted_sum)(&mut ar);
                eq_int(&format!("C20 count={count} iter={iter}"), rc, rr);
            }
            eq_array(&format!("C20 count={count} iter={iter} (struct)"), &ac, &ar);
        }
    }
}

#[test]
fn c21_compute_weighted_sum_after_process() {
    // The realistic `arrayfunc` ordering: the array has already been rewritten
    // in place by `process_with_foreach`.
    let l = libs();
    let mut rng = Rng::new(0x2100);
    for &count in COUNTS {
        for iter in 0..12_000u64 {
            let (mut ac, mut ar) = init_pair(0x2100 + iter, count, &mut rng);
            for op_index in 0..4 {
                let (_, cop) = l.c.ops()[op_index];
                let (_, rop) = l.rust.ops()[op_index];
                unsafe {
                    let rc = (l.c.process_with_foreach)(&mut ac, Some(cop));
                    let rr = (l.rust.process_with_foreach)(&mut ar, Some(rop));
                    eq_int(&format!("C21 pre-op {op_index} count={count}"), rc, rr);
                }
            }
            eq_array(&format!("C21 after ops count={count} iter={iter}"), &ac, &ar);
            unsafe {
                let rc = (l.c.compute_weighted_sum)(&mut ac);
                let rr = (l.rust.compute_weighted_sum)(&mut ar);
                eq_int(&format!("C21 count={count} iter={iter}"), rc, rr);
            }
            eq_array(&format!("C21 count={count} iter={iter} (struct)"), &ac, &ar);
        }
    }
}

#[test]
fn c22_compute_weighted_sum_saturating_values() {
    // `weight` grows with `i`, so the same `value` clamps only at high indices.
    let l = libs();
    let extremes: &[c_int] = &[
        i32::MAX,
        i32::MIN,
        i32::MAX / 2,
        i32::MIN / 2,
        i32::MAX / 8,
        i32::MIN / 8,
        268_435_456,
        -268_435_456,
        2_684_354_56 / 2,
        1,
        -1,
        0,
    ];
    let mut rng = Rng::new(0x2200);
    for iter in 0..80_000u64 {
        let mut ac = ResultArray::dirty(0x2200 + iter);
        for k in 0..10 {
            ac.data[k].value = extremes[(rng.next_u64() as usize) % extremes.len()];
            ac.data[k].rank = rng.range_i32(-5, 15);
            ac.data[k].scaled = rng.interesting_f64();
        }
        ac.count = rng.range_i32(0, 10);
        let mut ar = ac;
        unsafe {
            let rc = (l.c.compute_weighted_sum)(&mut ac);
            let rr = (l.rust.compute_weighted_sum)(&mut ar);
            eq_int(&format!("C22 iter={iter} count={}", ac.count), rc, rr);
        }
        eq_array(&format!("C22 iter={iter} (struct)"), &ac, &ar);
    }
}

#[test]
fn c23_compute_weighted_sum_count_one_weight_fallback() {
    // count == 1 exercises ONLY the `weight = 1` fallback branch.
    let l = libs();
    let mut rng = Rng::new(0x2300);
    for iter in 0..80_000u64 {
        let mut ac = ResultArray::dirty(0x2300 + iter);
        for k in 0..10 {
            ac.data[k].value = rng.interesting_i32();
            ac.data[k].rank = rng.range_i32(-3, 12);
            ac.data[k].scaled = rng.interesting_f64();
        }
        ac.count = 1;
        let mut ar = ac;
        unsafe {
            let rc = (l.c.compute_weighted_sum)(&mut ac);
            let rr = (l.rust.compute_weighted_sum)(&mut ar);
            eq_int(&format!("C23 iter={iter} value={}", ac.data[0].value), rc, rr);
        }
        eq_array(&format!("C23 iter={iter} (struct)"), &ac, &ar);
    }
}

// ===========================================================================
// C24, C25 — compare_results_in_array
// ===========================================================================

#[test]
fn c24_compare_results_in_array_full_grid() {
    let l = libs();
    let mut cases = 0u32;
    for count in -1..=11i32 {
        for idx1 in -3..=12i32 {
            for idx2 in -3..=12i32 {
                let mut ac = ResultArray::dirty(0x2400u64.wrapping_add(count as i64 as u64));
                ac.count = count;
                let mut ar = ac;
                unsafe {
                    let rc = (l.c.compare_results_in_array)(&mut ac, idx1, idx2);
                    let rr = (l.rust.compare_results_in_array)(&mut ar, idx1, idx2);
                    eq_int(
                        &format!("C24 count={count} idx1={idx1} idx2={idx2}"),
                        rc,
                        rr,
                    );
                }
                eq_array(
                    &format!("C24 count={count} idx1={idx1} idx2={idx2} (struct)"),
                    &ac,
                    &ar,
                );
                cases += 1;
            }
        }
    }
    assert_eq!(cases, 13 * 16 * 16, "grid size");
}

#[test]
fn c25_compare_results_in_array_randomized() {
    let l = libs();
    let mut rng = Rng::new(0x2500);
    for iter in 0..600_000u64 {
        let mut ac = ResultArray::dirty(0x2500 + (iter % 64));
        ac.count = match iter % 4 {
            0 => rng.range_i32(-2, 12),
            1 => rng.interesting_i32(),
            2 => rng.range_i32(0, 10),
            _ => rng.next_i32(),
        };
        let mut ar = ac;
        // Keep indices in a range whose address arithmetic stays inside our own
        // stack frame; the C never dereferences, but this keeps the pointer
        // comparison meaningful rather than wildly out of range.
        let (idx1, idx2) = match iter % 3 {
            0 => (rng.range_i32(-4, 14), rng.range_i32(-4, 14)),
            1 => {
                let i = rng.range_i32(-4, 14);
                (i, i) // equal indices
            }
            _ => (rng.range_i32(-1, 11), rng.range_i32(-1, 11)),
        };
        unsafe {
            let rc = (l.c.compare_results_in_array)(&mut ac, idx1, idx2);
            let rr = (l.rust.compare_results_in_array)(&mut ar, idx1, idx2);
            eq_int(
                &format!("C25 iter={iter} count={} idx1={idx1} idx2={idx2}", ac.count),
                rc,
                rr,
            );
        }
        eq_array(&format!("C25 iter={iter} (struct)"), &ac, &ar);
    }
}

// ===========================================================================
// C26 — the full pipeline reassembled from the low-level exports
// ===========================================================================

/// Reproduces `arrayfunc`'s body using only the low-level exports of `imp`,
/// comparing the struct after every single step against the other library.
fn pipeline(row: &str, p1: c_int, p2: c_int, p3: c_int, p4: c_int) -> (c_int, c_int) {
    let l = libs();

    let mut vals: Vec<c_int> = vec![
        p1,
        p2,
        p3,
        p4,
        p1.wrapping_add(p2),
        p2.wrapping_sub(p3),
        p3.wrapping_mul(2),
        (p4 / 2).wrapping_add(1),
    ];

    let mut ac = ResultArray::dirty(0x2600);
    let mut ar = ResultArray::dirty(0x2600);
    // `ResultArray arr = {.count = 0};` zero-initialises the whole object.
    for k in 0..10 {
        for a in [&mut ac, &mut ar] {
            a.data[k].value = 0;
            a.data[k].scaled = 0.0;
            a.data[k].rank = 0;
        }
    }
    ac.count = 0;
    ar.count = 0;

    unsafe {
        (l.c.init_result_array)(&mut ac, vals.as_mut_ptr(), 8);
        (l.rust.init_result_array)(&mut ar, vals.as_mut_ptr(), 8);
    }
    eq_array(&format!("{row} after init"), &ac, &ar);

    let mut rc: c_int = 0;
    let mut rr: c_int = 0;

    for op_index in 0..4 {
        let (name, cop) = l.c.ops()[op_index];
        let (_, rop) = l.rust.ops()[op_index];
        unsafe {
            let tc = (l.c.process_with_foreach)(&mut ac, Some(cop));
            let tr = (l.rust.process_with_foreach)(&mut ar, Some(rop));
            eq_int(&format!("{row} process({name}) return"), tc, tr);
            rc = rc.wrapping_add(tc);
            rr = rr.wrapping_add(tr);
        }
        eq_array(&format!("{row} after process({name})"), &ac, &ar);
    }

    unsafe {
        let wc = (l.c.compute_weighted_sum)(&mut ac);
        let wr = (l.rust.compute_weighted_sum)(&mut ar);
        eq_int(&format!("{row} weighted_sum"), wc, wr);
        rc = rc.wrapping_add(wc);
        rr = rr.wrapping_add(wr);
    }
    eq_array(&format!("{row} after weighted_sum"), &ac, &ar);

    let mut i: c_int = 0;
    while i < ac.count.wrapping_sub(1) {
        unsafe {
            let cc = (l.c.compare_results_in_array)(&mut ac, i, i.wrapping_add(1));
            let cr = (l.rust.compare_results_in_array)(&mut ar, i, i.wrapping_add(1));
            eq_int(&format!("{row} compare({i},{})", i + 1), cc, cr);
            rc = rc.wrapping_add(cc);
            rr = rr.wrapping_add(cr);
        }
        i += 1;
    }

    unsafe {
        let fc = (l.c.safe_double_to_int)(rc as f64 * 0.333);
        let fr = (l.rust.safe_double_to_int)(rr as f64 * 0.333);
        eq_int(&format!("{row} final scale"), fc, fr);
        (fc, fr)
    }
}

#[test]
fn c26_full_low_level_pipeline() {
    let l = libs();
    let mut rng = Rng::new(0x2600);
    for iter in 0..15_000u64 {
        let (p1, p2, p3, p4) = if iter % 2 == 0 {
            (
                rng.range_i32(-500, 500),
                rng.range_i32(-500, 500),
                rng.range_i32(-500, 500),
                rng.range_i32(-500, 500),
            )
        } else {
            (
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            )
        };
        let row = format!("C26 pipeline({p1},{p2},{p3},{p4})");
        let (fc, fr) = pipeline(&row, p1, p2, p3, p4);

        // The hand-assembled pipeline must also agree with each library's own
        // one-shot `arrayfunc`.
        unsafe {
            let ac = (l.c.arrayfunc)(p1, p2, p3, p4);
            let ar = (l.rust.arrayfunc)(p1, p2, p3, p4);
            eq_int(&format!("{row} arrayfunc C vs Rust"), ac, ar);
            assert_eq!(fc, ac, "{row}: C pipeline != C arrayfunc");
            assert_eq!(fr, ar, "{row}: Rust pipeline != Rust arrayfunc");
        }
    }
}

// ===========================================================================
// C27..C30 — arrayfunc
// ===========================================================================

fn diff_arrayfunc(row: &str, p1: c_int, p2: c_int, p3: c_int, p4: c_int) {
    let l = libs();
    unsafe {
        eq_int(
            &format!("{row} arrayfunc({p1},{p2},{p3},{p4})"),
            (l.c.arrayfunc)(p1, p2, p3, p4),
            (l.rust.arrayfunc)(p1, p2, p3, p4),
        );
    }
}

#[test]
fn c27_arrayfunc_handpicked_shapes() {
    let extremes: &[c_int] = &[
        0,
        1,
        -1,
        2,
        -2,
        3,
        -3,
        4,
        -4,
        5,
        -5,
        10,
        -10,
        100,
        -100,
        1000,
        -1000,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        i32::MAX / 2,
        i32::MIN / 2,
        1 << 30,
        -(1 << 30),
        65535,
        65536,
        -65536,
    ];
    // all-equal
    for &v in extremes {
        diff_arrayfunc("C27 all-equal", v, v, v, v);
    }
    // one extreme in each of the four positions, rest small
    for &v in extremes {
        for pos in 0..4 {
            let mut p = [7, -3, 11, -9];
            p[pos] = v;
            diff_arrayfunc(&format!("C27 extreme@{pos}"), p[0], p[1], p[2], p[3]);
        }
    }
    // pairwise combinations of the extremes in positions (0,1) and (2,3)
    for &a in extremes {
        for &b in extremes {
            diff_arrayfunc("C27 pair01", a, b, 5, -6);
            diff_arrayfunc("C27 pair23", 5, -6, a, b);
            diff_arrayfunc("C27 pair03", a, 5, -6, b);
        }
    }
    // param4 odd / even / negative -> `/2` truncation toward zero
    for p4 in -25..=25i32 {
        diff_arrayfunc("C27 p4 trunc", 3, -4, 5, p4);
        diff_arrayfunc("C27 p4 trunc big", i32::MIN, i32::MAX, i32::MIN, p4);
    }
    diff_arrayfunc("C27 p4=INT_MIN", 1, 2, 3, i32::MIN);
    diff_arrayfunc("C27 p4=INT_MAX", 1, 2, 3, i32::MAX);
    // param3 overflow via `param3 * 2`
    for &v in extremes {
        diff_arrayfunc("C27 p3 overflow", 1, 2, v, 4);
    }
}

#[test]
fn c28_arrayfunc_random_small() {
    let mut rng = Rng::new(0x2800);
    for _ in 0..150_000 {
        diff_arrayfunc(
            "C28 random small",
            rng.range_i32(-1000, 1000),
            rng.range_i32(-1000, 1000),
            rng.range_i32(-1000, 1000),
            rng.range_i32(-1000, 1000),
        );
    }
}

#[test]
fn c29_arrayfunc_random_full_range() {
    let mut rng = Rng::new(0x2900);
    for _ in 0..80_000 {
        diff_arrayfunc(
            "C29 random full",
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
    }
    for _ in 0..80_000 {
        diff_arrayfunc(
            "C29 uniform full",
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
        );
    }
}

#[test]
fn c30_arrayfunc_exhaustive_small_grid() {
    let vals = [-2i32, -1, 0, 1, 2];
    let mut n = 0u32;
    for &a in &vals {
        for &b in &vals {
            for &c in &vals {
                for &d in &vals {
                    diff_arrayfunc("C30 grid", a, b, c, d);
                    n += 1;
                }
            }
        }
    }
    assert_eq!(n, 625);
}
