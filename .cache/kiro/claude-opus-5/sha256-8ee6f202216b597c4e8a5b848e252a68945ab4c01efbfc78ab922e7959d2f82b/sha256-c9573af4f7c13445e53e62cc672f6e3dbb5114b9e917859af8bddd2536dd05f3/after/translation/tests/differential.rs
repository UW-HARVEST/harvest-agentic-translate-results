//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every call goes through a `.so` export loaded with `libloading`; nothing here
//! links against the Rust crate directly. See `tests/harness/mod.rs` for why all
//! tests serialize on a single lock (the library carries mutable global state
//! that must advance in lockstep across the two `.so`s).

mod harness;

use harness::*;
use std::ffi::c_int;

// ===========================================================================
// Rows 1–4: pure integer helpers
// ===========================================================================

#[test]
fn row01_add_three_random_full_domain() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..20_000 {
        both_add_three(rng.i32_any(), rng.i32_any(), rng.i32_any());
    }
    for _ in 0..20_000 {
        both_add_three(rng.i32_interesting(), rng.i32_interesting(), rng.i32_interesting());
    }
}

#[test]
fn row02_add_three_boundary_cross_product() {
    let _g = lock();
    for &a in &EDGE {
        for &b in &EDGE {
            for &c in &EDGE {
                both_add_three(a, b, c);
            }
        }
    }
}

#[test]
fn row03_multiply_add_random_full_domain() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..20_000 {
        both_multiply_add(rng.i32_any(), rng.i32_any(), rng.i32_any());
    }
    for _ in 0..20_000 {
        both_multiply_add(rng.i32_interesting(), rng.i32_interesting(), rng.i32_interesting());
    }
}

#[test]
fn row04_multiply_add_boundary_cross_product() {
    let _g = lock();
    for &a in &EDGE {
        for &b in &EDGE {
            for &c in &EDGE {
                both_multiply_add(a, b, c);
            }
        }
    }
    // The classic signed-overflow pairs.
    both_multiply_add(i32::MIN, -1, 0);
    both_multiply_add(-1, i32::MIN, 0);
    both_multiply_add(i32::MAX, i32::MAX, i32::MAX);
    both_multiply_add(i32::MIN, i32::MIN, i32::MIN);
}

// ===========================================================================
// Rows 5–9: the two global-state mutators (observed through the readers)
// ===========================================================================

/// `complex_calc(x, x, 0)` == `global_counter`, so it is an exact probe of A1
/// without needing the `static` to be exported.
#[track_caller]
fn probe_counter() -> c_int {
    both_complex_calc(7, 7, 0)
}

/// `process_pointer_data(0, 0)` == `global_accumulator`, an exact probe of A2.
#[track_caller]
fn probe_accumulator() -> c_int {
    both_process_pointer_data(0, 0)
}

#[test]
fn row05_increment_counter_positive_accumulation() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..500 {
        both_increment_counter(rng.range(1, 100_000), 0);
        probe_counter();
    }
}

#[test]
fn row06_increment_counter_negative_and_wrapping() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..500 {
        both_increment_counter(-rng.range(1, 100_000), 0);
        probe_counter();
    }
    // Drive A1 across the INT_MAX / INT_MIN wrap points repeatedly.
    for _ in 0..200 {
        both_increment_counter(i32::MAX, 0);
        probe_counter();
        both_increment_counter(i32::MIN, 0);
        probe_counter();
        both_increment_counter(rng.i32_any(), 0);
        probe_counter();
    }
}

#[test]
fn row07_increment_counter_ignores_unused_param() {
    let _g = lock();
    let mut rng = rng();
    for &unused in &EDGE {
        both_increment_counter(0, unused);
        probe_counter();
    }
    for _ in 0..2_000 {
        both_increment_counter(rng.i32_any(), rng.i32_any());
        probe_counter();
    }
}

#[test]
fn row08_update_accumulator_doubling_to_wrap() {
    let _g = lock();
    let mut rng = rng();
    // A2 doubles every call, so 40 calls guarantee it wraps past INT_MAX.
    for _ in 0..40 {
        both_update_accumulator(rng.i32_small(), 0);
        probe_accumulator();
    }
    for _ in 0..2_000 {
        both_update_accumulator(rng.i32_any(), 0);
        probe_accumulator();
    }
    for &v in &EDGE {
        both_update_accumulator(v, 0);
        probe_accumulator();
    }
}

#[test]
fn row09_update_accumulator_ignores_unused_param() {
    let _g = lock();
    let mut rng = rng();
    for &unused in &EDGE {
        both_update_accumulator(0, unused);
        probe_accumulator();
    }
    for _ in 0..2_000 {
        both_update_accumulator(rng.i32_any(), rng.i32_any());
        probe_accumulator();
    }
}

// ===========================================================================
// Rows 10–13: complex_calc across A1 states
// ===========================================================================

#[test]
fn row10_complex_calc_pristine_ish_counter() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..20_000 {
        both_complex_calc(rng.i32_any(), rng.i32_any(), rng.i32_any());
    }
}

#[test]
fn row11_complex_calc_positive_counter() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..300 {
        both_increment_counter(rng.range(1, 1_000_000), 0);
        for _ in 0..20 {
            both_complex_calc(rng.i32_any(), rng.i32_any(), rng.i32_any());
        }
    }
}

#[test]
fn row12_complex_calc_negative_and_wrapped_counter() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..300 {
        both_increment_counter(-rng.range(1, 1_000_000), 0);
        for _ in 0..10 {
            both_complex_calc(rng.i32_any(), rng.i32_any(), rng.i32_any());
        }
        both_increment_counter(i32::MIN, 0);
        for _ in 0..10 {
            both_complex_calc(rng.i32_any(), rng.i32_any(), rng.i32_any());
        }
    }
}

#[test]
fn row13_complex_calc_boundary_args() {
    let _g = lock();
    let mut rng = rng();
    for round in 0..4 {
        both_increment_counter(if round == 0 { 0 } else { rng.i32_any() }, 0);
        for &a in &EDGE {
            for &b in &EDGE {
                for &c in &EDGE {
                    both_complex_calc(a, b, c);
                }
            }
        }
    }
}

// ===========================================================================
// Rows 14–16: process_pointer_data across A2 states
// ===========================================================================

#[test]
fn row14_process_pointer_data_pristine_ish_accumulator() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..20_000 {
        both_process_pointer_data(rng.i32_any(), rng.i32_any());
    }
}

#[test]
fn row15_process_pointer_data_varied_accumulator() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..500 {
        both_update_accumulator(rng.i32_any(), 0);
        for _ in 0..20 {
            both_process_pointer_data(rng.i32_any(), rng.i32_any());
        }
    }
}

#[test]
fn row16_process_pointer_data_boundary_args() {
    let _g = lock();
    let mut rng = rng();
    for round in 0..4 {
        if round > 0 {
            both_update_accumulator(rng.i32_any(), 0);
        }
        for &v in &EDGE {
            for &m in &EDGE {
                both_process_pointer_data(v, m);
            }
        }
    }
}

// ===========================================================================
// Rows 17–21: apply_operation with every function-pointer configuration
// ===========================================================================

#[test]
fn row17_apply_operation_add_three_same_library() {
    let _g = lock();
    let p = libs();
    let mut rng = rng();
    for _ in 0..10_000 {
        let (a, b, c) = (rng.i32_any(), rng.i32_any(), rng.i32_any());
        both_apply_operation_with(
            "add_three@self",
            Some(p.c.add_three),
            Some(p.r.add_three),
            a,
            b,
            c,
        );
    }
    for &a in &EDGE {
        for &b in &EDGE {
            for &c in &EDGE {
                both_apply_operation_with(
                    "add_three@self",
                    Some(p.c.add_three),
                    Some(p.r.add_three),
                    a,
                    b,
                    c,
                );
            }
        }
    }
}

#[test]
fn row18_apply_operation_multiply_add_same_library() {
    let _g = lock();
    let p = libs();
    let mut rng = rng();
    for _ in 0..10_000 {
        let (a, b, c) = (rng.i32_any(), rng.i32_any(), rng.i32_any());
        both_apply_operation_with(
            "multiply_add@self",
            Some(p.c.multiply_add),
            Some(p.r.multiply_add),
            a,
            b,
            c,
        );
    }
    for &a in &EDGE {
        for &b in &EDGE {
            for &c in &EDGE {
                both_apply_operation_with(
                    "multiply_add@self",
                    Some(p.c.multiply_add),
                    Some(p.r.multiply_add),
                    a,
                    b,
                    c,
                );
            }
        }
    }
}

#[test]
fn row19_apply_operation_complex_calc_same_library_varied_counter() {
    let _g = lock();
    let p = libs();
    let mut rng = rng();
    for _ in 0..400 {
        both_increment_counter(rng.i32_any(), 0);
        for _ in 0..25 {
            let (a, b, c) = (rng.i32_any(), rng.i32_any(), rng.i32_any());
            both_apply_operation_with(
                "complex_calc@self",
                Some(p.c.complex_calc),
                Some(p.r.complex_calc),
                a,
                b,
                c,
            );
        }
    }
}

#[test]
fn row20_apply_operation_cross_library_function_pointer() {
    let _g = lock();
    let p = libs();
    let mut rng = rng();
    // C's apply_operation invoking Rust's callback, and vice versa.
    for _ in 0..10_000 {
        let (a, b, c) = (rng.i32_any(), rng.i32_any(), rng.i32_any());
        both_apply_operation_with(
            "add_three@other",
            Some(p.r.add_three),
            Some(p.c.add_three),
            a,
            b,
            c,
        );
        both_apply_operation_with(
            "multiply_add@other",
            Some(p.r.multiply_add),
            Some(p.c.multiply_add),
            a,
            b,
            c,
        );
    }
    // complex_calc reads its *own* library's global_counter; because the two
    // globals are kept in lockstep, the crossed calls must still agree.
    for _ in 0..200 {
        both_increment_counter(rng.i32_any(), 0);
        for _ in 0..20 {
            let (a, b, c) = (rng.i32_any(), rng.i32_any(), rng.i32_any());
            both_apply_operation_with(
                "complex_calc@other",
                Some(p.r.complex_calc),
                Some(p.c.complex_calc),
                a,
                b,
                c,
            );
        }
    }
}

/// A callback that lives in neither `.so`. Deliberately not expressible as any
/// of the library's own operations, so a hypothetical "inline the known
/// callback" shortcut in the translation would show up immediately.
unsafe extern "C" fn caller_supplied_op(a: c_int, b: c_int, c: c_int) -> c_int {
    let x = (a as u32).rotate_left(7) ^ (b as u32).wrapping_mul(0x9E37_79B1);
    (x.wrapping_add(c as u32) ^ 0x5A5A_5A5A) as c_int
}

unsafe extern "C" fn caller_supplied_const(_a: c_int, _b: c_int, _c: c_int) -> c_int {
    i32::MIN
}

#[test]
fn row21_apply_operation_caller_supplied_callback() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..10_000 {
        let (a, b, c) = (rng.i32_any(), rng.i32_any(), rng.i32_any());
        both_apply_operation_with(
            "caller_supplied_op",
            Some(caller_supplied_op),
            Some(caller_supplied_op),
            a,
            b,
            c,
        );
        both_apply_operation_with(
            "caller_supplied_const",
            Some(caller_supplied_const),
            Some(caller_supplied_const),
            a,
            b,
            c,
        );
    }
    for &a in &EDGE {
        for &b in &EDGE {
            for &c in &EDGE {
                both_apply_operation_with(
                    "caller_supplied_op",
                    Some(caller_supplied_op),
                    Some(caller_supplied_op),
                    a,
                    b,
                    c,
                );
            }
        }
    }
}

// ===========================================================================
// Rows 22–28: shift_array_data valid shapes
// ===========================================================================

fn random_ints(rng: &mut Rng, n: usize) -> Vec<c_int> {
    (0..n).map(|_| rng.i32_any()).collect()
}

#[test]
fn row22_shift_array_interior() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..4_000 {
        let size = rng.range(2, 64);
        let shift_by = rng.range(1, size - 1);
        let data = random_ints(&mut rng, size as usize);
        both_shift_array_data(&data, size, shift_by);
    }
}

#[test]
fn row23_shift_array_shift_by_one() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..2_000 {
        let size = rng.range(2, 128);
        let data = random_ints(&mut rng, size as usize);
        both_shift_array_data(&data, size, 1);
    }
}

#[test]
fn row24_shift_array_shift_by_size_minus_one() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..2_000 {
        let size = rng.range(2, 128);
        let data = random_ints(&mut rng, size as usize);
        let out = both_shift_array_data(&data, size, size - 1);
        // One element moved down, everything above it zeroed.
        assert_eq!(out[0], data[(size - 1) as usize]);
        for v in &out[1..] {
            assert_eq!(*v, 0);
        }
    }
}

#[test]
fn row25_shift_array_size_one() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..500 {
        let data = random_ints(&mut rng, 1);
        for shift_by in [-1, 0, 1, 2, i32::MIN, i32::MAX] {
            let out = both_shift_array_data(&data, 1, shift_by);
            assert_eq!(out, data, "size==1 admits no valid shift");
        }
    }
}

#[test]
fn row26_shift_array_size_two_shift_one() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..2_000 {
        let data = random_ints(&mut rng, 2);
        let out = both_shift_array_data(&data, 2, 1);
        assert_eq!(out, vec![data[1], 0]);
    }
}

#[test]
fn row27_shift_array_large() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..40 {
        let size = 4096;
        let shift_by = rng.range(1, size - 1);
        let data = random_ints(&mut rng, size as usize);
        both_shift_array_data(&data, size, shift_by);
    }
}

#[test]
fn row28_shift_array_respects_size_slack() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..2_000 {
        let size = rng.range(2, 32);
        let slack = rng.range(1, 32);
        let total = (size + slack) as usize;
        let shift_by = rng.range(1, size - 1);
        let data = random_ints(&mut rng, total);
        let out = both_shift_array_data(&data, size, shift_by);
        assert_eq!(
            &out[size as usize..],
            &data[size as usize..],
            "shift_array_data wrote past `size` (size={size}, shift_by={shift_by})"
        );
    }
}

// ===========================================================================
// Rows 29–35: manipulate_records valid shapes
// ===========================================================================

#[test]
fn row29_manipulate_records_interior() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..3_000 {
        let n = rng.range(2, 32);
        let shift = rng.range(1, n - 1);
        let recs = random_records(&mut rng, n as usize);
        both_manipulate_records(&recs, n, shift);
    }
}

#[test]
fn row30_manipulate_records_shift_one() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..2_000 {
        let n = rng.range(2, 64);
        let recs = random_records(&mut rng, n as usize);
        let (total, out) = both_manipulate_records(&recs, n, 1);
        let expect: c_int = recs[1..n as usize]
            .iter()
            .fold(0i32, |acc, r| acc.wrapping_add(r.value));
        assert_eq!(total, expect, "shift==1 must sum records[1..n]");
        for i in 0..(n as usize - 1) {
            assert_eq!(out[i], recs[i + 1], "record {i} not relocated correctly");
        }
    }
}

#[test]
fn row31_manipulate_records_shift_n_minus_one() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..2_000 {
        let n = rng.range(2, 64);
        let recs = random_records(&mut rng, n as usize);
        let (total, out) = both_manipulate_records(&recs, n, n - 1);
        assert_eq!(total, recs[(n - 1) as usize].value);
        assert_eq!(out[0], recs[(n - 1) as usize]);
    }
}

#[test]
fn row32_manipulate_records_shift_zero() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..3_000 {
        let n = rng.range(1, 48);
        let recs = random_records(&mut rng, n as usize);
        let (total, out) = both_manipulate_records(&recs, n, 0);
        assert_eq!(out, recs, "shift==0 must not move anything");
        let expect: c_int = recs.iter().fold(0i32, |acc, r| acc.wrapping_add(r.value));
        assert_eq!(total, expect);
    }
}

#[test]
fn row33_manipulate_records_single() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..1_000 {
        let recs = random_records(&mut rng, 1);
        let (total, out) = both_manipulate_records(&recs, 1, 0);
        assert_eq!(total, recs[0].value);
        assert_eq!(out, recs);
    }
}

#[test]
fn row34_manipulate_records_large() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..40 {
        let n = 512;
        let shift = rng.range(1, n - 1);
        let recs = random_records(&mut rng, n as usize);
        both_manipulate_records(&recs, n, shift);
    }
}

#[test]
fn row35_manipulate_records_full_struct_payload_moves() {
    let _g = lock();
    assert_eq!(
        std::mem::size_of::<DataRecord>(),
        48,
        "DataRecord layout must match the C struct"
    );
    let mut rng = rng();
    for _ in 0..2_000 {
        let n = rng.range(2, 24);
        let shift = rng.range(1, n - 1);
        let mut recs = random_records(&mut rng, n as usize);
        // Distinct, fully populated payload per record so a partial or
        // wrongly-strided memmove is detectable in every field.
        for (i, r) in recs.iter_mut().enumerate() {
            r.id = 0x1000_0000 + i as c_int;
            r.timestamp = 0x0BAD_F00D_0000_0000u64 as i64 + i as i64;
            for (j, b) in r.name.iter_mut().enumerate() {
                *b = ((i * 31 + j * 7 + 1) & 0x7f) as std::ffi::c_char;
            }
        }
        let (_, out) = both_manipulate_records(&recs, n, shift);
        for i in 0..(n - shift) as usize {
            assert_eq!(
                out[i],
                recs[i + shift as usize],
                "all 48 bytes of record {i} must be relocated"
            );
        }
    }
}

// ===========================================================================
// Rows 36–39: compute_with_dynamic_memory
// ===========================================================================

#[test]
fn row36_compute_dynamic_memory_random_counts() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..4_000 {
        both_compute_with_dynamic_memory(rng.i32_any(), rng.range(1, 4096));
    }
}

#[test]
fn row37_compute_dynamic_memory_count_one() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..2_000 {
        let base = rng.i32_any();
        let v = both_compute_with_dynamic_memory(base, 1);
        assert_eq!(v, base);
    }
    for &base in &EDGE {
        both_compute_with_dynamic_memory(base, 1);
    }
}

#[test]
fn row38_compute_dynamic_memory_large_count() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..20 {
        both_compute_with_dynamic_memory(rng.i32_any(), 65_536);
    }
    both_compute_with_dynamic_memory(0, 65_536);
    both_compute_with_dynamic_memory(i32::MAX, 65_536);
    both_compute_with_dynamic_memory(i32::MIN, 65_536);
}

#[test]
fn row39_compute_dynamic_memory_overflowing_base() {
    let _g = lock();
    for &base in &EDGE {
        for count in 1..=64 {
            both_compute_with_dynamic_memory(base, count);
        }
    }
}

// ===========================================================================
// Rows 40–44: get_time_based_value
// ===========================================================================

#[test]
fn row40_time_based_value_zero() {
    let _g = lock();
    for _ in 0..100 {
        let v = both_get_time_based_value(0);
        assert_eq!(v, 0);
    }
}

#[test]
fn row41_time_based_value_positive_no_overflow() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..5_000 {
        both_get_time_based_value(rng.range(1, 596_523));
    }
}

#[test]
fn row42_time_based_value_negative_no_overflow() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..5_000 {
        both_get_time_based_value(-rng.range(1, 596_523));
    }
}

#[test]
fn row43_time_based_value_overflowing_seed() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..20_000 {
        both_get_time_based_value(rng.i32_any());
    }
    for _ in 0..5_000 {
        both_get_time_based_value(rng.i32_interesting());
    }
}

#[test]
fn row44_time_based_value_overflow_threshold() {
    let _g = lock();
    for seed in [
        i32::MIN,
        i32::MIN + 1,
        -596_524,
        -596_523,
        -596_522,
        -1,
        0,
        1,
        596_522,
        596_523,
        596_524,
        i32::MAX - 1,
        i32::MAX,
    ] {
        both_get_time_based_value(seed);
    }
    // Sweep the whole neighbourhood of the int-overflow boundary.
    for seed in 596_400..596_700 {
        both_get_time_based_value(seed);
        both_get_time_based_value(-seed);
    }
}

// ===========================================================================
// Rows 45–48: hatch (the public one-shot entry point)
// ===========================================================================

#[test]
fn row45_hatch_random_params() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..4_000 {
        both_hatch(rng.i32_any(), rng.i32_any(), rng.i32_any(), rng.i32_any());
    }
    for _ in 0..4_000 {
        both_hatch(
            rng.i32_interesting(),
            rng.i32_interesting(),
            rng.i32_interesting(),
            rng.i32_interesting(),
        );
    }
    for _ in 0..2_000 {
        both_hatch(
            rng.i32_small(),
            rng.i32_small(),
            rng.i32_small(),
            rng.i32_small(),
        );
    }
}

#[test]
fn row46_hatch_repeated_calls_accumulate_state() {
    let _g = lock();
    let mut rng = rng();
    // global_accumulator doubles inside each hatch, so this sweeps it through
    // repeated wraparound while global_counter drifts.
    for _ in 0..64 {
        both_hatch(1, 1, 1, 1);
    }
    for _ in 0..64 {
        let (a, b, c, d) = (rng.i32_small(), rng.i32_small(), rng.i32_small(), rng.i32_small());
        for _ in 0..8 {
            both_hatch(a, b, c, d);
        }
    }
}

#[test]
fn row47_hatch_boundary_params() {
    let _g = lock();
    let vals = [0, 1, -1, i32::MIN, i32::MAX];
    for &a in &vals {
        for &b in &vals {
            for &c in &vals {
                for &d in &vals {
                    both_hatch(a, b, c, d);
                }
            }
        }
    }
}

#[test]
fn row48_hatch_after_external_global_mutation() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..600 {
        both_increment_counter(rng.i32_any(), rng.i32_any());
        both_update_accumulator(rng.i32_any(), rng.i32_any());
        both_hatch(rng.i32_any(), rng.i32_any(), rng.i32_any(), rng.i32_any());
        probe_counter();
        probe_accumulator();
    }
}

// ===========================================================================
// Rows 49–50: composed pipelines through the low-level exports
// ===========================================================================

#[test]
fn row49_composed_pipeline_low_level() {
    let _g = lock();
    let p = libs();
    let mut rng = rng();

    for _ in 0..1_500 {
        let (p1, p2, p3, p4) = (
            rng.i32_any(),
            rng.i32_any(),
            rng.i32_any(),
            rng.i32_any(),
        );

        both_increment_counter(p1, rng.i32_any());
        both_update_accumulator(p2, rng.i32_any());

        both_apply_operation_with(
            "add_three",
            Some(p.c.add_three),
            Some(p.r.add_three),
            p1,
            p2,
            p3,
        );
        both_apply_operation_with(
            "multiply_add",
            Some(p.c.multiply_add),
            Some(p.r.multiply_add),
            p2,
            p3,
            p4,
        );
        both_apply_operation_with(
            "complex_calc",
            Some(p.c.complex_calc),
            Some(p.r.complex_calc),
            p1,
            p3,
            p4,
        );

        // Shared int buffer walked through both entry points, like `hatch` does.
        let n = rng.range(4, 40);
        let data = random_ints(&mut rng, n as usize);
        let idx = rng.range(0, n - 1);
        both_process_pointer_data(data[idx as usize], p2);
        let shifted = both_shift_array_data(&data, n, rng.range(1, n - 1));
        both_process_pointer_data(shifted[0], p3);

        // Shared record buffer.
        let nr = rng.range(2, 20);
        let recs = random_records(&mut rng, nr as usize);
        let (_, moved) = both_manipulate_records(&recs, nr, rng.range(1, nr - 1));
        both_process_pointer_data(moved[0].value, p4);

        both_get_time_based_value(p3);
        both_compute_with_dynamic_memory(p1, rng.range(1, 64));

        probe_counter();
        probe_accumulator();
    }
}

#[test]
fn row50_hatch_call_sequence_replicated_via_low_level_exports() {
    let _g = lock();
    let p = libs();
    let mut rng = rng();

    // Reproduce exactly what `hatch` does internally, but driving each step
    // through the individual `.so` exports, and diff C against Rust at every
    // step (a whole-pipeline diff, not a per-wrapper diff). The final
    // `global_counter + global_accumulator` term is covered by the probes.
    for _ in 0..1_000 {
        let (p1, p2, p3, p4) = (
            rng.i32_interesting(),
            rng.i32_interesting(),
            rng.i32_interesting(),
            rng.i32_interesting(),
        );

        let mut result: c_int = 0;

        both_increment_counter(p1, 999);
        both_update_accumulator(p2, 888);

        result = result.wrapping_add(both_apply_operation_with(
            "add_three",
            Some(p.c.add_three),
            Some(p.r.add_three),
            p1,
            p2,
            p3,
        ));
        result = result.wrapping_add(both_apply_operation_with(
            "multiply_add",
            Some(p.c.multiply_add),
            Some(p.r.multiply_add),
            p2,
            p3,
            p4,
        ));
        result = result.wrapping_add(both_apply_operation_with(
            "complex_calc",
            Some(p.c.complex_calc),
            Some(p.r.complex_calc),
            p1,
            p3,
            p4,
        ));

        let dynamic: Vec<c_int> = (0..10).map(|i| p1.wrapping_add(i)).collect();
        result = result.wrapping_add(both_process_pointer_data(dynamic[5], p2));
        let shifted = both_shift_array_data(&dynamic, 10, 3);
        result = result.wrapping_add(shifted[0]);

        result = result.wrapping_add(both_get_time_based_value(p3));

        let recs: Vec<DataRecord> = (0..5)
            .map(|i| {
                let mut r = DataRecord::zeroed();
                r.id = i;
                r.value = p4.wrapping_add(i.wrapping_mul(10));
                r
            })
            .collect();
        let (total, _) = both_manipulate_records(&recs, 5, 2);
        result = result.wrapping_add(total);

        result = result.wrapping_add(both_compute_with_dynamic_memory(p1, 8));

        // Globals, read back through their exact probes.
        let counter = probe_counter();
        let accumulator = probe_accumulator();
        let _ = result.wrapping_add(counter.wrapping_add(accumulator));
    }
}
