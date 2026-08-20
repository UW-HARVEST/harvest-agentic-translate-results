//! Phase B — rows C7..C25 of CONFIGS.md: the array primitives, driven directly
//! through the `.so` exports (not through the `arrayfunc` convenience wrapper).

mod common;

use common::*;

// ---------------------------------------------------------------------------
// init_result_array (C7..C13)
// ---------------------------------------------------------------------------

fn drive_init(row: &str, mut start: impl FnMut() -> CResultArray, values: &[i32], count: i32) {
    let (c, r) = both();
    let mut ca = start();
    let mut ra = start();
    unsafe { (c.init_result_array)(&mut ca, values.as_ptr(), count) };
    unsafe { (r.init_result_array)(&mut ra, values.as_ptr(), count) };
    eq_arrays(row, (count, values.len()), &ca, &ra);
}

#[test]
fn c7_init_count_zero() {
    drive_init("C7 init count=0", CResultArray::zeroed, &[7; 10], 0);
    drive_init("C7 init count=0/poison", || CResultArray::poisoned(-77), &[7; 10], 0);
}

#[test]
fn c8_init_count_one() {
    drive_init("C8 init count=1", || CResultArray::poisoned(5), &[0x1234_5678; 10], 1);
    drive_init("C8 init count=1/min", || CResultArray::poisoned(5), &[i32::MIN; 10], 1);
    drive_init("C8 init count=1/max", CResultArray::zeroed, &[i32::MAX; 10], 1);
}

#[test]
fn c9_init_count_nine_random() {
    let mut rng = Rng::new(0xC9_0000);
    for _ in 0..2048 {
        let vals: Vec<i32> = (0..10).map(|_| rng.interesting_i32()).collect();
        drive_init("C9 init count=9", || CResultArray::poisoned(3), &vals, 9);
    }
}

#[test]
fn c10_init_count_ten_exact_clamp() {
    let mut rng = Rng::new(0xCA_0000);
    for _ in 0..2048 {
        let vals: Vec<i32> = (0..10).map(|_| rng.interesting_i32()).collect();
        drive_init("C10 init count=10", CResultArray::zeroed, &vals, 10);
        drive_init("C10 init count=10/poison", || CResultArray::poisoned(-1), &vals, 10);
    }
}

#[test]
fn c11_init_count_above_clamp() {
    let mut rng = Rng::new(0xCB_0000);
    for count in [11i32, 12, 100, 1000, i32::MAX] {
        for _ in 0..64 {
            // Only the first 10 entries may ever be read.
            let vals: Vec<i32> = (0..10).map(|_| rng.interesting_i32()).collect();
            drive_init("C11 init count>10", || CResultArray::poisoned(0), &vals, count);
        }
    }
}

#[test]
fn c12_init_count_negative() {
    for count in [-1i32, -2, -1000, i32::MIN, i32::MIN + 1] {
        drive_init("C12 init count<0", CResultArray::zeroed, &[9; 10], count);
        drive_init(
            "C12 init count<0/poison",
            || CResultArray::poisoned(4),
            &[9; 10],
            count,
        );
    }
}

#[test]
fn c13_init_twice_decreasing_counts() {
    let (c, r) = both();
    let mut rng = Rng::new(0xCD_0000);
    for _ in 0..512 {
        let first: Vec<i32> = (0..10).map(|_| rng.interesting_i32()).collect();
        let second: Vec<i32> = (0..10).map(|_| rng.interesting_i32()).collect();
        let n1 = rng.range_i32(0, 10);
        let n2 = rng.range_i32(0, 10);
        let mut ca = CResultArray::poisoned(0);
        let mut ra = CResultArray::poisoned(0);
        unsafe {
            (c.init_result_array)(&mut ca, first.as_ptr(), n1);
            (r.init_result_array)(&mut ra, first.as_ptr(), n1);
        }
        eq_arrays("C13 init pass1", (n1, n2), &ca, &ra);
        unsafe {
            (c.init_result_array)(&mut ca, second.as_ptr(), n2);
            (r.init_result_array)(&mut ra, second.as_ptr(), n2);
        }
        eq_arrays("C13 init pass2", (n1, n2), &ca, &ra);
    }
}

// ---------------------------------------------------------------------------
// compare_results_in_array (C14..C16)
// ---------------------------------------------------------------------------

fn drive_compare(row: &str, count: i32, idx1: i32, idx2: i32) {
    let (c, r) = both();
    let mut ca = CResultArray::poisoned(count);
    let mut ra = CResultArray::poisoned(count);
    let cv = unsafe { (c.compare_results_in_array)(&mut ca, idx1, idx2) };
    let rv = unsafe { (r.compare_results_in_array)(&mut ra, idx1, idx2) };
    eq_i32(row, (count, idx1, idx2), cv, rv);
    // The function must not modify the array in either implementation.
    eq_arrays(row, (count, idx1, idx2), &ca, &ra);
}

#[test]
fn c14_compare_all_pairs_count_ten() {
    for i in 0..10 {
        for j in 0..10 {
            drive_compare("C14 compare", 10, i, j);
        }
    }
}

#[test]
fn c15_compare_full_grid() {
    for count in [0i32, 1, 2, 5, 9, 10] {
        let idxs = [
            -1000,
            i32::MIN,
            i32::MIN + 1,
            -3,
            -1,
            0,
            1,
            count - 1,
            count,
            count + 1,
            9,
            10,
            11,
            1000,
            i32::MAX - 1,
            i32::MAX,
        ];
        for &a in &idxs {
            for &b in &idxs {
                drive_compare("C15 compare grid", count, a, b);
            }
        }
    }
}

#[test]
fn c16_compare_huge_count() {
    for count in [i32::MAX, i32::MAX - 1, 1 << 20] {
        for &a in &[-5i32, 0, 1, 7, 4096, 1 << 19, 1 << 20] {
            for &b in &[-5i32, 0, 1, 7, 4096, 1 << 19, 1 << 20] {
                drive_compare("C16 compare huge count", count, a, b);
            }
        }
    }
    // Randomised sweep over the whole (count, idx1, idx2) space.
    let mut rng = Rng::new(0xC16_000);
    for _ in 0..20000 {
        let count = rng.interesting_i32();
        let a = rng.interesting_i32();
        let b = rng.interesting_i32();
        let (c, r) = both();
        let mut ca = CResultArray::poisoned(count);
        let mut ra = CResultArray::poisoned(count);
        let cv = unsafe { (c.compare_results_in_array)(&mut ca, a, b) };
        let rv = unsafe { (r.compare_results_in_array)(&mut ra, a, b) };
        eq_i32("C16 compare random", (count, a, b), cv, rv);
    }
}

// ---------------------------------------------------------------------------
// compute_weighted_sum (C17..C20)
// ---------------------------------------------------------------------------

fn drive_weighted(row: &str, arr: CResultArray) {
    let (c, r) = both();
    let mut ca = arr;
    let mut ra = arr;
    let cv = unsafe { (c.compute_weighted_sum)(&mut ca) };
    let rv = unsafe { (r.compute_weighted_sum)(&mut ra) };
    eq_i32(row, (ca.count,), cv, rv);
    eq_arrays(row, (ca.count,), &ca, &ra);
}

#[test]
fn c17_weighted_sum_zero_and_one() {
    drive_weighted("C17 weighted count=0", CResultArray::from_values(&[]));
    drive_weighted("C17 weighted count=0/poison", CResultArray::poisoned(0));
    for v in [0i32, 1, -1, 1000, i32::MIN, i32::MAX, i32::MIN + 1] {
        drive_weighted("C17 weighted count=1", CResultArray::from_values(&[v]));
    }
}

#[test]
fn c18_weighted_sum_two_to_ten_random() {
    let mut rng = Rng::new(0xC18_000);
    for n in 2..=10usize {
        for _ in 0..1024 {
            let vals: Vec<i32> = (0..n).map(|_| rng.interesting_i32()).collect();
            drive_weighted("C18 weighted", CResultArray::from_values(&vals));
        }
    }
}

#[test]
fn c19_weighted_sum_saturating() {
    for v in [i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1] {
        drive_weighted("C19 weighted sat", CResultArray::from_values(&[v; 10]));
    }
    let extremes = [i32::MIN, i32::MAX, 0, 1, -1, 1 << 30, -(1 << 30)];
    let mut rng = Rng::new(0xC19_000);
    for _ in 0..4096 {
        let vals: Vec<i32> = (0..10)
            .map(|_| extremes[(rng.next_u64() % extremes.len() as u64) as usize])
            .collect();
        drive_weighted("C19 weighted sat/mix", CResultArray::from_values(&vals));
    }
}

#[test]
fn c20_weighted_sum_poisoned_prefix() {
    for n in 0..=10 {
        drive_weighted("C20 weighted poison", CResultArray::poisoned(n));
    }
}

// ---------------------------------------------------------------------------
// process_with_foreach (C21..C25)
// ---------------------------------------------------------------------------

/// Callbacks defined *in the test binary* — pointers neither library has ever
/// seen. ERRORS.md's "out-of-range enum" analogue for `operation_func`.
unsafe extern "C" fn cb_first(a: i32, _b: i32, _c: i32, _d: i32) -> i32 {
    a
}
unsafe extern "C" fn cb_int_max(_a: i32, _b: i32, _c: i32, _d: i32) -> i32 {
    i32::MAX
}
unsafe extern "C" fn cb_int_min(_a: i32, _b: i32, _c: i32, _d: i32) -> i32 {
    i32::MIN
}
unsafe extern "C" fn cb_overflow_mul(a: i32, b: i32, _c: i32, _d: i32) -> i32 {
    a.wrapping_mul(b).wrapping_mul(0x4321)
}
unsafe extern "C" fn cb_huge_alternating(a: i32, b: i32, _c: i32, _d: i32) -> i32 {
    if (a ^ b) & 1 == 0 {
        i32::MAX / 2 + 12345
    } else {
        i32::MIN / 2 - 12345
    }
}
unsafe extern "C" fn cb_uses_unused(a: i32, b: i32, c: i32, d: i32) -> i32 {
    a.wrapping_add(b).wrapping_add(c).wrapping_add(d)
}

fn drive_foreach(row: &str, arr: CResultArray, cop: OpFn, rop: OpFn, tag: &str) {
    let (c, r) = both();
    let mut ca = arr;
    let mut ra = arr;
    let cv = unsafe { (c.process_with_foreach)(&mut ca, Some(cop)) };
    let rv = unsafe { (r.process_with_foreach)(&mut ra, Some(rop)) };
    eq_i32(row, (tag, ca.count), cv, rv);
    eq_arrays(row, (tag, ca.count), &ca, &ra);
}

#[test]
fn c21_foreach_builtin_ops() {
    let (c, r) = both();
    let ops: [(&str, OpFn, OpFn); 4] = [
        ("add", c.add_operation, r.add_operation),
        ("mul", c.multiply_operation, r.multiply_operation),
        ("sub", c.subtract_operation, r.subtract_operation),
        ("mod", c.modulo_operation, r.modulo_operation),
    ];
    let mut rng = Rng::new(0xC21_000);
    for (tag, cop, rop) in ops {
        for n in [0usize, 1, 2, 9, 10] {
            for _ in 0..512 {
                let vals: Vec<i32> = (0..n).map(|_| rng.interesting_i32()).collect();
                drive_foreach("C21 foreach builtin", CResultArray::from_values(&vals), cop, rop, tag);
            }
            // Also with a poisoned backing array so untouched tail elements are
            // compared for real.
            let mut a = CResultArray::poisoned(n as i32);
            for i in 0..n {
                a.data[i].rank = i as i32;
            }
            drive_foreach("C21 foreach builtin/poison", a, cop, rop, tag);
        }
    }
}

#[test]
fn c22_foreach_foreign_callbacks() {
    let cbs: [(&str, OpFn); 6] = [
        ("first", cb_first),
        ("int_max", cb_int_max),
        ("int_min", cb_int_min),
        ("overflow_mul", cb_overflow_mul),
        ("huge_alt", cb_huge_alternating),
        ("uses_unused", cb_uses_unused),
    ];
    let mut rng = Rng::new(0xC22_000);
    for (tag, cb) in cbs {
        for n in [0usize, 1, 2, 5, 10] {
            for _ in 0..256 {
                let vals: Vec<i32> = (0..n).map(|_| rng.interesting_i32()).collect();
                drive_foreach(
                    "C22 foreach foreign cb",
                    CResultArray::from_values(&vals),
                    cb,
                    cb,
                    tag,
                );
            }
        }
    }
}

#[test]
fn c23_foreach_writeback_saturation() {
    // Callbacks returning INT_MAX / INT_MIN force `safe_double_to_int(r*0.75)`
    // through both saturation arms (ERRORS.md E19).
    for (tag, cb) in [("int_max", cb_int_max as OpFn), ("int_min", cb_int_min as OpFn)] {
        drive_foreach(
            "C23 foreach writeback sat",
            CResultArray::from_values(&[i32::MAX; 10]),
            cb,
            cb,
            tag,
        );
        drive_foreach(
            "C23 foreach writeback sat",
            CResultArray::from_values(&[i32::MIN; 10]),
            cb,
            cb,
            tag,
        );
    }
    let (c, r) = both();
    // multiply_operation on huge values overflows into every sign combination.
    let mut rng = Rng::new(0xC23_000);
    for _ in 0..4096 {
        let vals: Vec<i32> = (0..10).map(|_| rng.interesting_i32()).collect();
        drive_foreach(
            "C23 foreach mul sat",
            CResultArray::from_values(&vals),
            c.multiply_operation,
            r.multiply_operation,
            "mul",
        );
    }
}

/// Row C24 — `count > 10` makes the `FOREACH` macro walk one `Result` past
/// `data[10]`, overlapping the `count` field. Both libraries must touch exactly
/// the same bytes of the padded buffer.
#[test]
fn c24_foreach_past_data_end() {
    let (c, r) = both();
    let ops: [(&str, OpFn, OpFn); 4] = [
        ("add", c.add_operation, r.add_operation),
        ("mul", c.multiply_operation, r.multiply_operation),
        ("sub", c.subtract_operation, r.subtract_operation),
        ("mod", c.modulo_operation, r.modulo_operation),
    ];
    let mut rng = Rng::new(0xC24_000);
    for (tag, cop, rop) in ops {
        for count in [11i32, 12] {
            for _ in 0..256 {
                let mk = |seed: u64| {
                    let mut rng2 = Rng::new(seed);
                    let mut p = PaddedArray::new_filled(0x5C);
                    for i in 0..13usize {
                        let v = rng2.interesting_i32();
                        p.set_elem(i, v, v as f64 * 1.5, i as i32);
                    }
                    // `count` overlaps element 10's `value` field; write it last.
                    p.set_count(count);
                    p
                };
                let seed = rng.next_u64();
                let mut cp = mk(seed);
                let mut rp = mk(seed);
                let cv = unsafe { (c.process_with_foreach)(cp.as_arr_ptr(), Some(cop)) };
                let rv = unsafe { (r.process_with_foreach)(rp.as_arr_ptr(), Some(rop)) };
                eq_i32("C24 foreach OOB", (tag, count), cv, rv);
                assert_eq!(
                    cp.bytes, rp.bytes,
                    "C24 foreach OOB ({tag}, count={count}): padded buffer bytes differ\n\
                     first difference at index {:?}",
                    cp.bytes.iter().zip(rp.bytes.iter()).position(|(a, b)| a != b)
                );
            }
        }
    }
}

/// Row C25 — repeated application so each pass sees the previous pass's mutated
/// `value`/`scaled` state (the composed-pipeline case).
#[test]
fn c25_foreach_repeated_passes() {
    let (c, r) = both();
    let ops: [(&str, OpFn, OpFn); 4] = [
        ("add", c.add_operation, r.add_operation),
        ("mul", c.multiply_operation, r.multiply_operation),
        ("sub", c.subtract_operation, r.subtract_operation),
        ("mod", c.modulo_operation, r.modulo_operation),
    ];
    let mut rng = Rng::new(0xC25_000);
    for (tag, cop, rop) in ops {
        for _ in 0..1024 {
            let n = rng.range_i32(0, 10) as usize;
            let vals: Vec<i32> = (0..n).map(|_| rng.interesting_i32()).collect();
            let mut ca = CResultArray::from_values(&vals);
            let mut ra = CResultArray::from_values(&vals);
            for pass in 0..6 {
                let cv = unsafe { (c.process_with_foreach)(&mut ca, Some(cop)) };
                let rv = unsafe { (r.process_with_foreach)(&mut ra, Some(rop)) };
                eq_i32("C25 foreach repeated", (tag, n, pass), cv, rv);
                eq_arrays("C25 foreach repeated", (tag, n, pass), &ca, &ra);
            }
        }
    }
}
