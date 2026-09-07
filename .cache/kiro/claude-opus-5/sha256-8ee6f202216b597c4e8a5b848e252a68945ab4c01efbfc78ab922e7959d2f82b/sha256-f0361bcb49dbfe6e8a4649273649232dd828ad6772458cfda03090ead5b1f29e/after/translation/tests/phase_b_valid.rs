//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Both libraries are driven only through symbols resolved from their `.so`s.

mod common;

use common::*;
use std::os::raw::c_int;

const N: usize = 400;

// ---------------------------------------------------------------------------
// Rows 1-2: pure arithmetic
// ---------------------------------------------------------------------------

#[test]
fn row01_add_three() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 1);
    for _ in 0..N {
        let (a, b, c) = (rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed());
        eq_i32("add_three", (a, b, c), unsafe { (p.c.add_three)(a, b, c) }, unsafe {
            (p.r.add_three)(a, b, c)
        });
    }
    // explicit boundary triples
    for &(a, b, c) in &[
        (i32::MAX, 1, 0),
        (i32::MIN, -1, 0),
        (i32::MAX, i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN, i32::MIN),
        (0, 0, 0),
    ] {
        eq_i32("add_three", (a, b, c), unsafe { (p.c.add_three)(a, b, c) }, unsafe {
            (p.r.add_three)(a, b, c)
        });
    }
}

#[test]
fn row02_multiply_add() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..N {
        let (a, b, c) = (rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed());
        eq_i32("multiply_add", (a, b, c), unsafe { (p.c.multiply_add)(a, b, c) }, unsafe {
            (p.r.multiply_add)(a, b, c)
        });
    }
    for &(a, b, c) in &[
        (i32::MAX, i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN, i32::MIN),
        (i32::MIN, -1, 0),
        (65536, 65536, 0),
        (0, i32::MIN, i32::MAX),
    ] {
        eq_i32("multiply_add", (a, b, c), unsafe { (p.c.multiply_add)(a, b, c) }, unsafe {
            (p.r.multiply_add)(a, b, c)
        });
    }
}

// ---------------------------------------------------------------------------
// Rows 3-4: complex_calc, pristine and accumulated global_counter
// ---------------------------------------------------------------------------

#[test]
fn row03_complex_calc_pristine_counter() {
    let (_g, p) = fresh(); // statics zeroed => global_counter == 0 in both
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..N {
        let (a, b, c) = (rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed());
        eq_i32("complex_calc", (a, b, c), unsafe { (p.c.complex_calc)(a, b, c) }, unsafe {
            (p.r.complex_calc)(a, b, c)
        });
    }
}

#[test]
fn row04_complex_calc_nonzero_counter() {
    let (_g, p) = fresh();
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..N {
        // advance global_counter identically in both libraries
        let bump = rng.i32_mixed();
        unsafe { (p.c.increment_counter)(bump, 999) };
        unsafe { (p.r.increment_counter)(bump, 999) };

        let (a, b, c) = (rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed());
        eq_i32(
            "complex_calc(after increment)",
            (bump, a, b, c),
            unsafe { (p.c.complex_calc)(a, b, c) },
            unsafe { (p.r.complex_calc)(a, b, c) },
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 5-6: the two static-mutating setters, observed through their readers
// ---------------------------------------------------------------------------

#[test]
fn row05_increment_counter_sequence() {
    let (_g, p) = fresh();
    let mut rng = Rng::new(SEED ^ 5);
    // Push global_counter to INT_MAX first, then keep incrementing to force wrap.
    for &v in &[i32::MAX, 1, 1, i32::MAX, i32::MIN, -1] {
        unsafe { (p.c.increment_counter)(v, 0) };
        unsafe { (p.r.increment_counter)(v, 0) };
        eq_i32("global_counter via complex_calc", v, unsafe { (p.c.complex_calc)(0, 0, 0) }, unsafe {
            (p.r.complex_calc)(0, 0, 0)
        });
    }
    for _ in 0..N {
        let v = rng.i32_mixed();
        unsafe { (p.c.increment_counter)(v, rng.i32_any()) };
        unsafe { (p.r.increment_counter)(v, 0) };
        eq_i32("global_counter via complex_calc", v, unsafe { (p.c.complex_calc)(0, 0, 0) }, unsafe {
            (p.r.complex_calc)(0, 0, 0)
        });
    }
}

#[test]
fn row06_update_accumulator_sequence() {
    let (_g, p) = fresh();
    let mut rng = Rng::new(SEED ^ 6);
    let mut probe: c_int = 0;
    // *2 growth saturates/wraps quickly with large seeds.
    for &v in &[i32::MAX, i32::MAX, i32::MIN, 1, -1, 0] {
        unsafe { (p.c.update_accumulator)(v, 0) };
        unsafe { (p.r.update_accumulator)(v, 0) };
        eq_i32(
            "global_accumulator via process_pointer_data",
            v,
            unsafe { (p.c.process_pointer_data)(&mut probe, 0) },
            unsafe { (p.r.process_pointer_data)(&mut probe, 0) },
        );
    }
    for _ in 0..N {
        let v = rng.i32_mixed();
        unsafe { (p.c.update_accumulator)(v, rng.i32_any()) };
        unsafe { (p.r.update_accumulator)(v, 0) };
        eq_i32(
            "global_accumulator via process_pointer_data",
            v,
            unsafe { (p.c.process_pointer_data)(&mut probe, 0) },
            unsafe { (p.r.process_pointer_data)(&mut probe, 0) },
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 7-10: apply_operation dispatch
// ---------------------------------------------------------------------------

#[test]
fn row07_apply_operation_add_three() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..N {
        let (a, b, c) = (rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed());
        eq_i32(
            "apply_operation(add_three)",
            (a, b, c),
            unsafe { (p.c.apply_operation)(Some(p.c.add_three), a, b, c) },
            unsafe { (p.r.apply_operation)(Some(p.r.add_three), a, b, c) },
        );
    }
}

#[test]
fn row08_apply_operation_multiply_add() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..N {
        let (a, b, c) = (rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed());
        eq_i32(
            "apply_operation(multiply_add)",
            (a, b, c),
            unsafe { (p.c.apply_operation)(Some(p.c.multiply_add), a, b, c) },
            unsafe { (p.r.apply_operation)(Some(p.r.multiply_add), a, b, c) },
        );
    }
}

#[test]
fn row09_apply_operation_complex_calc_with_state() {
    let (_g, p) = fresh();
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..N {
        let bump = rng.i32_mixed();
        unsafe { (p.c.increment_counter)(bump, 0) };
        unsafe { (p.r.increment_counter)(bump, 0) };
        let (a, b, c) = (rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed());
        eq_i32(
            "apply_operation(complex_calc)",
            (bump, a, b, c),
            unsafe { (p.c.apply_operation)(Some(p.c.complex_calc), a, b, c) },
            unsafe { (p.r.apply_operation)(Some(p.r.complex_calc), a, b, c) },
        );
    }
}

#[test]
fn row10_apply_operation_cross_library_pointers() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..N {
        let (a, b, c) = (rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed());
        // C's dispatcher calling Rust's callee, and vice versa.
        eq_i32(
            "cross apply_operation(add_three)",
            (a, b, c),
            unsafe { (p.c.apply_operation)(Some(p.r.add_three), a, b, c) },
            unsafe { (p.r.apply_operation)(Some(p.c.add_three), a, b, c) },
        );
        eq_i32(
            "cross apply_operation(multiply_add)",
            (a, b, c),
            unsafe { (p.c.apply_operation)(Some(p.r.multiply_add), a, b, c) },
            unsafe { (p.r.apply_operation)(Some(p.c.multiply_add), a, b, c) },
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 11-14: shift_array_data
// ---------------------------------------------------------------------------

fn shift_case(p: &Pair, data: &[c_int], size: c_int, shift_by: c_int) {
    let mut cb = data.to_vec();
    let mut rb = data.to_vec();
    unsafe { (p.c.shift_array_data)(cb.as_mut_ptr(), size, shift_by) };
    unsafe { (p.r.shift_array_data)(rb.as_mut_ptr(), size, shift_by) };
    eq_bytes("shift_array_data", (size, shift_by, data.len()), &cb, &rb);
}

#[test]
fn row11_shift_array_guard_true_random() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..N {
        let size = rng.range(1, 64);
        let shift_by = rng.range(1, size.max(1));
        let data: Vec<c_int> = (0..size as usize).map(|_| rng.i32_mixed()).collect();
        shift_case(p, &data, size, shift_by);
    }
}

#[test]
fn row12_shift_array_boundary_shifts() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 12);
    for &size in &[2i32, 3, 8, 64] {
        for &shift_by in &[1, size - 1, size / 2] {
            for _ in 0..20 {
                let data: Vec<c_int> = (0..size as usize).map(|_| rng.i32_mixed()).collect();
                shift_case(p, &data, size, shift_by);
            }
        }
    }
}

#[test]
fn row13_shift_array_guard_false() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..N {
        let size = rng.range(1, 64);
        let data: Vec<c_int> = (0..size as usize).map(|_| rng.i32_mixed()).collect();
        for &shift_by in &[0, -1, size, size + 1, i32::MIN, i32::MAX, -size] {
            shift_case(p, &data, size, shift_by);
        }
    }
}

#[test]
fn row14_shift_array_degenerate_shapes() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 14);
    for _ in 0..50 {
        // size 0: allocate a 4-element buffer but tell the library size==0
        let data: Vec<c_int> = (0..4).map(|_| rng.i32_mixed()).collect();
        for &size in &[0i32, 1] {
            for &shift_by in &[-1, 0, 1, 2, i32::MIN, i32::MAX] {
                shift_case(p, &data, size, shift_by);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 15-16: process_pointer_data
// ---------------------------------------------------------------------------

#[test]
fn row15_process_pointer_data_pristine() {
    let (_g, p) = fresh();
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..N {
        let mut cv = rng.i32_mixed();
        let mut rv = cv;
        let m = rng.i32_mixed();
        eq_i32(
            "process_pointer_data",
            (cv, m),
            unsafe { (p.c.process_pointer_data)(&mut cv, m) },
            unsafe { (p.r.process_pointer_data)(&mut rv, m) },
        );
        assert_eq!(cv, rv, "process_pointer_data must not modify *ptr");
    }
}

#[test]
fn row16_process_pointer_data_with_accumulator() {
    let (_g, p) = fresh();
    let mut rng = Rng::new(SEED ^ 16);
    for _ in 0..N {
        let acc = rng.i32_mixed();
        unsafe { (p.c.update_accumulator)(acc, 0) };
        unsafe { (p.r.update_accumulator)(acc, 0) };
        let mut cv = rng.i32_mixed();
        let mut rv = cv;
        let m = rng.i32_mixed();
        eq_i32(
            "process_pointer_data(with accumulator)",
            (acc, cv, m),
            unsafe { (p.c.process_pointer_data)(&mut cv, m) },
            unsafe { (p.r.process_pointer_data)(&mut rv, m) },
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 17-20: compute_with_dynamic_memory
// ---------------------------------------------------------------------------

#[test]
fn row17_dynamic_memory_zero_and_one() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 17);
    for _ in 0..N {
        let base = rng.i32_mixed();
        for &count in &[0, 1] {
            eq_i32(
                "compute_with_dynamic_memory",
                (base, count),
                unsafe { (p.c.compute_with_dynamic_memory)(base, count) },
                unsafe { (p.r.compute_with_dynamic_memory)(base, count) },
            );
        }
    }
}

#[test]
fn row18_dynamic_memory_small_counts() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 18);
    for _ in 0..N {
        let base = rng.i32_mixed();
        let count = rng.range(2, 64);
        eq_i32(
            "compute_with_dynamic_memory",
            (base, count),
            unsafe { (p.c.compute_with_dynamic_memory)(base, count) },
            unsafe { (p.r.compute_with_dynamic_memory)(base, count) },
        );
    }
}

#[test]
fn row19_dynamic_memory_large_counts() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 19);
    for _ in 0..80 {
        let base = rng.i32_mixed();
        let count = rng.range(1_000, 200_000);
        eq_i32(
            "compute_with_dynamic_memory(large)",
            (base, count),
            unsafe { (p.c.compute_with_dynamic_memory)(base, count) },
            unsafe { (p.r.compute_with_dynamic_memory)(base, count) },
        );
    }
}

#[test]
fn row20_dynamic_memory_boundary_base_count8() {
    let (_g, p) = locked();
    for &base in &[i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
        for &count in &[1, 2, 7, 8, 9, 100] {
            eq_i32(
                "compute_with_dynamic_memory(boundary base)",
                (base, count),
                unsafe { (p.c.compute_with_dynamic_memory)(base, count) },
                unsafe { (p.r.compute_with_dynamic_memory)(base, count) },
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 21-22: get_time_based_value
// ---------------------------------------------------------------------------

#[test]
fn row21_time_based_value_no_overflow() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 21);
    eq_i32("get_time_based_value", 0, unsafe { (p.c.get_time_based_value)(0) }, unsafe {
        (p.r.get_time_based_value)(0)
    });
    for _ in 0..N {
        let seed = rng.range(-596_523, 596_523);
        eq_i32(
            "get_time_based_value",
            seed,
            unsafe { (p.c.get_time_based_value)(seed) },
            unsafe { (p.r.get_time_based_value)(seed) },
        );
    }
}

#[test]
fn row22_time_based_value_overflow() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 22);
    for &seed in &[
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
        596_524,
        -596_524,
        1_000_000,
        -1_000_000,
        // exactly at the int-overflow threshold for seed*3600
        596_523,
        -596_523,
    ] {
        eq_i32(
            "get_time_based_value(overflow)",
            seed,
            unsafe { (p.c.get_time_based_value)(seed) },
            unsafe { (p.r.get_time_based_value)(seed) },
        );
    }
    for _ in 0..N {
        let seed = rng.i32_any();
        eq_i32(
            "get_time_based_value(random full range)",
            seed,
            unsafe { (p.c.get_time_based_value)(seed) },
            unsafe { (p.r.get_time_based_value)(seed) },
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 23-28: manipulate_records
// ---------------------------------------------------------------------------

fn rand_records(rng: &mut Rng, n: usize) -> Vec<DataRecord> {
    (0..n)
        .map(|i| {
            let mut name = [0i8; 32];
            for b in name.iter_mut() {
                *b = rng.i8_any();
            }
            DataRecord {
                id: i as c_int,
                value: rng.i32_mixed(),
                timestamp: rng.i64_any(),
                name,
            }
        })
        .collect()
}

/// Runs one `manipulate_records` configuration on both libraries and compares
/// the return value AND the full post-call buffer (raw bytes, padding included).
fn records_case(p: &Pair, base: &[DataRecord], num_records: c_int, shift: c_int) {
    let mut cb = base.to_vec();
    let mut rb = base.to_vec();
    let cr = unsafe { (p.c.manipulate_records)(cb.as_mut_ptr(), num_records, shift) };
    let rr = unsafe { (p.r.manipulate_records)(rb.as_mut_ptr(), num_records, shift) };
    let ctx = (num_records, shift, base.len());
    eq_i32("manipulate_records", ctx, cr, rr);
    eq_bytes("manipulate_records buffer", ctx, as_raw_bytes(&cb), as_raw_bytes(&rb));
}

#[test]
fn row23_manipulate_records_guard_true_random() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 23);
    for _ in 0..N {
        let num_records = rng.range(2, 32);
        let shift = rng.range(1, num_records - 1);
        let recs = rand_records(&mut rng, num_records as usize);
        records_case(p, &recs, num_records, shift);
    }
}

#[test]
fn row24_manipulate_records_shift_zero() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 24);
    for _ in 0..N {
        let num_records = rng.range(1, 32);
        let recs = rand_records(&mut rng, num_records as usize);
        records_case(p, &recs, num_records, 0);
    }
}

#[test]
fn row25_manipulate_records_boundary_shifts() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 25);
    for &num_records in &[2i32, 5, 32] {
        for &shift in &[1, num_records - 1, num_records / 2] {
            for _ in 0..20 {
                let recs = rand_records(&mut rng, num_records as usize);
                records_case(p, &recs, num_records, shift);
            }
        }
    }
}

#[test]
fn row26_manipulate_records_hatch_shape() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 26);
    for _ in 0..N {
        // exactly what hatch does: 5 records, shift 2, value = param4 + i*10
        let param4 = rng.i32_mixed();
        let recs: Vec<DataRecord> = (0..5)
            .map(|i| DataRecord {
                id: i,
                value: param4.wrapping_add(i.wrapping_mul(10)),
                timestamp: 0,
                name: [0; 32],
            })
            .collect();
        records_case(p, &recs, 5, 2);
    }
}

#[test]
fn row27_manipulate_records_total_overflow() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 27);
    for _ in 0..N {
        let num_records = rng.range(2, 16);
        let mut recs = rand_records(&mut rng, num_records as usize);
        for r in recs.iter_mut() {
            r.value = if rng.next_u64() % 2 == 0 { i32::MAX } else { i32::MIN };
        }
        let shift = rng.range(0, num_records - 1);
        records_case(p, &recs, num_records, shift);
    }
}

#[test]
fn row28_manipulate_records_degenerate() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 28);
    for _ in 0..50 {
        // over-allocate so any in-bounds-of-allocation read stays defined
        let recs = rand_records(&mut rng, 8);
        for &num_records in &[0i32, 1] {
            for &shift in &[0i32, 1] {
                records_case(p, &recs, num_records, shift);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 29-32: hatch, the composed pipeline
// ---------------------------------------------------------------------------

#[test]
fn row29_hatch_pristine_single_call_random() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 29);
    for _ in 0..60 {
        // Each iteration needs pristine statics: hatch mutates both, and
        // a "first call from a clean library" is its own configuration.
        p.reset();
        let (a, b, c, d) = (rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed());
        eq_i32("hatch(pristine)", (a, b, c, d), unsafe { (p.c.hatch)(a, b, c, d) }, unsafe {
            (p.r.hatch)(a, b, c, d)
        });
    }
}

#[test]
fn row30_hatch_boundary_params() {
    const V: [i32; 7] =
        [0, 1, -1, i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1];
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 30);
    for _ in 0..80 {
        p.reset();
        let pick = |r: &mut Rng| V[(r.next_u64() % V.len() as u64) as usize];
        let (a, b, c, d) = (pick(&mut rng), pick(&mut rng), pick(&mut rng), pick(&mut rng));
        eq_i32("hatch(boundary)", (a, b, c, d), unsafe { (p.c.hatch)(a, b, c, d) }, unsafe {
            (p.r.hatch)(a, b, c, d)
        });
    }
    // and every single-axis extreme with the others at 0
    for &v in &V {
        for axis in 0..4 {
            p.reset();
            let mut arg = [0i32; 4];
            arg[axis] = v;
            eq_i32(
                "hatch(single-axis extreme)",
                (v, axis),
                unsafe { (p.c.hatch)(arg[0], arg[1], arg[2], arg[3]) },
                unsafe { (p.r.hatch)(arg[0], arg[1], arg[2], arg[3]) },
            );
        }
    }
}

#[test]
fn row31_hatch_stateful_repetition() {
    let (_g, p) = fresh();
    let mut rng = Rng::new(SEED ^ 31);
    for i in 0..200 {
        let (a, b, c, d) = (rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed());
        eq_i32(
            "hatch(repeated)",
            (i, a, b, c, d),
            unsafe { (p.c.hatch)(a, b, c, d) },
            unsafe { (p.r.hatch)(a, b, c, d) },
        );
    }
}

#[test]
fn row32_hatch_interleaved_with_setters() {
    let (_g, p) = fresh();
    let mut rng = Rng::new(SEED ^ 32);
    for i in 0..200 {
        let x = rng.i32_mixed();
        let y = rng.i32_mixed();
        unsafe { (p.c.increment_counter)(x, 999) };
        unsafe { (p.r.increment_counter)(x, 999) };
        unsafe { (p.c.update_accumulator)(y, 888) };
        unsafe { (p.r.update_accumulator)(y, 888) };
        let (a, b, c, d) = (rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed());
        eq_i32(
            "hatch(interleaved)",
            (i, x, y, a, b, c, d),
            unsafe { (p.c.hatch)(a, b, c, d) },
            unsafe { (p.r.hatch)(a, b, c, d) },
        );
    }
}

// ---------------------------------------------------------------------------
// Row 33: mixed random operation stream over every exported entry point
// ---------------------------------------------------------------------------

#[test]
fn row33_mixed_random_operation_stream() {
    let (_g, p) = fresh();
    let mut rng = Rng::new(SEED ^ 33);
    for step in 0..2_000u32 {
        match rng.next_u64() % 12 {
            0 => {
                let v = rng.i32_mixed();
                unsafe { (p.c.increment_counter)(v, rng.i32_any()) };
                unsafe { (p.r.increment_counter)(v, 0) };
            }
            1 => {
                let v = rng.i32_mixed();
                unsafe { (p.c.update_accumulator)(v, rng.i32_any()) };
                unsafe { (p.r.update_accumulator)(v, 0) };
            }
            2 => {
                let (a, b, c) = (rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed());
                let op = rng.next_u64() % 3;
                let (cf, rf) = match op {
                    0 => (p.c.add_three, p.r.add_three),
                    1 => (p.c.multiply_add, p.r.multiply_add),
                    _ => (p.c.complex_calc, p.r.complex_calc),
                };
                eq_i32(
                    "stream apply_operation",
                    (step, op, a, b, c),
                    unsafe { (p.c.apply_operation)(Some(cf), a, b, c) },
                    unsafe { (p.r.apply_operation)(Some(rf), a, b, c) },
                );
            }
            3 => {
                let (a, b, c) = (rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed());
                eq_i32("stream add_three", (step, a, b, c), unsafe { (p.c.add_three)(a, b, c) }, unsafe {
                    (p.r.add_three)(a, b, c)
                });
            }
            4 => {
                let (a, b, c) = (rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed());
                eq_i32("stream multiply_add", (step, a, b, c), unsafe { (p.c.multiply_add)(a, b, c) }, unsafe {
                    (p.r.multiply_add)(a, b, c)
                });
            }
            5 => {
                let (a, b, c) = (rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed());
                eq_i32("stream complex_calc", (step, a, b, c), unsafe { (p.c.complex_calc)(a, b, c) }, unsafe {
                    (p.r.complex_calc)(a, b, c)
                });
            }
            6 => {
                let size = rng.range(0, 32);
                let shift_by = rng.range(-4, 36);
                let data: Vec<c_int> = (0..32).map(|_| rng.i32_mixed()).collect();
                let mut cb = data.clone();
                let mut rb = data;
                unsafe { (p.c.shift_array_data)(cb.as_mut_ptr(), size, shift_by) };
                unsafe { (p.r.shift_array_data)(rb.as_mut_ptr(), size, shift_by) };
                eq_bytes("stream shift_array_data", (step, size, shift_by), &cb, &rb);
            }
            7 => {
                let mut cv = rng.i32_mixed();
                let mut rv = cv;
                let m = rng.i32_mixed();
                eq_i32(
                    "stream process_pointer_data",
                    (step, cv, m),
                    unsafe { (p.c.process_pointer_data)(&mut cv, m) },
                    unsafe { (p.r.process_pointer_data)(&mut rv, m) },
                );
            }
            8 => {
                let base = rng.i32_mixed();
                let count = rng.range(0, 256);
                eq_i32(
                    "stream compute_with_dynamic_memory",
                    (step, base, count),
                    unsafe { (p.c.compute_with_dynamic_memory)(base, count) },
                    unsafe { (p.r.compute_with_dynamic_memory)(base, count) },
                );
            }
            9 => {
                let seed = rng.i32_mixed();
                eq_i32(
                    "stream get_time_based_value",
                    (step, seed),
                    unsafe { (p.c.get_time_based_value)(seed) },
                    unsafe { (p.r.get_time_based_value)(seed) },
                );
            }
            10 => {
                let n = rng.range(0, 16);
                let shift = rng.range(0, 18);
                let recs = rand_records(&mut rng, 16);
                let mut cb = recs.clone();
                let mut rb = recs;
                let cr = unsafe { (p.c.manipulate_records)(cb.as_mut_ptr(), n, shift) };
                let rr = unsafe { (p.r.manipulate_records)(rb.as_mut_ptr(), n, shift) };
                eq_i32("stream manipulate_records", (step, n, shift), cr, rr);
                eq_bytes(
                    "stream manipulate_records buffer",
                    (step, n, shift),
                    as_raw_bytes(&cb),
                    as_raw_bytes(&rb),
                );
            }
            _ => {
                let (a, b, c, d) =
                    (rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed());
                eq_i32(
                    "stream hatch",
                    (step, a, b, c, d),
                    unsafe { (p.c.hatch)(a, b, c, d) },
                    unsafe { (p.r.hatch)(a, b, c, d) },
                );
            }
        }
    }
}
