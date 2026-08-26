//! Phase B — rows C26..C33 of CONFIGS.md: composed pipelines assembled by the
//! test out of the low-level `.so` exports, plus the public `arrayfunc` entry
//! point over exhaustive corner matrices and large randomised sweeps.

mod common;

use common::*;

/// Replays `arrayfunc`'s body using only the individually exported functions,
/// comparing the *whole* array state after every stage. `ops` selects the order
/// (and multiplicity) of the operations applied.
fn drive_pipeline(row: &str, values: &[i32], count: i32, ops: &[usize]) {
    let (c, r) = both();
    let cops: [OpFn; 4] = [
        c.add_operation,
        c.multiply_operation,
        c.subtract_operation,
        c.modulo_operation,
    ];
    let rops: [OpFn; 4] = [
        r.add_operation,
        r.multiply_operation,
        r.subtract_operation,
        r.modulo_operation,
    ];

    let mut ca = CResultArray::poisoned(-12345);
    let mut ra = CResultArray::poisoned(-12345);

    // Stage 1: init_result_array
    unsafe {
        (c.init_result_array)(&mut ca, values.as_ptr(), count);
        (r.init_result_array)(&mut ra, values.as_ptr(), count);
    }
    eq_arrays(row, ("after init", count), &ca, &ra);

    let mut cres: i32 = 0;
    let mut rres: i32 = 0;

    // Stage 2: process_with_foreach for each selected operation, in order.
    for (step, &oi) in ops.iter().enumerate() {
        let cv = unsafe { (c.process_with_foreach)(&mut ca, Some(cops[oi])) };
        let rv = unsafe { (r.process_with_foreach)(&mut ra, Some(rops[oi])) };
        eq_i32(row, ("foreach", step, oi, count), cv, rv);
        eq_arrays(row, ("after foreach", step, oi, count), &ca, &ra);
        cres = cres.wrapping_add(cv);
        rres = rres.wrapping_add(rv);
        eq_i32(row, ("running total", step, count), cres, rres);
    }

    // Stage 3: compute_weighted_sum
    let cw = unsafe { (c.compute_weighted_sum)(&mut ca) };
    let rw = unsafe { (r.compute_weighted_sum)(&mut ra) };
    eq_i32(row, ("weighted", count), cw, rw);
    eq_arrays(row, ("after weighted", count), &ca, &ra);
    cres = cres.wrapping_add(cw);
    rres = rres.wrapping_add(rw);

    // Stage 4: the compare loop over `count - 1` adjacent pairs.
    let mut i: i32 = 0;
    while i < ca.count - 1 {
        let cv = unsafe { (c.compare_results_in_array)(&mut ca, i, i + 1) };
        let rv = unsafe { (r.compare_results_in_array)(&mut ra, i, i + 1) };
        eq_i32(row, ("compare", i, count), cv, rv);
        cres = cres.wrapping_add(cv);
        rres = rres.wrapping_add(rv);
        i += 1;
    }
    eq_i32(row, ("before final scale", count), cres, rres);

    // Stage 5: the final saturating scale.
    let cf = unsafe { (c.safe_double_to_int)(cres as f64 * 0.333) };
    let rf = unsafe { (r.safe_double_to_int)(rres as f64 * 0.333) };
    eq_i32(row, ("final", count), cf, rf);
    eq_arrays(row, ("end", count), &ca, &ra);
}

#[test]
fn c26_pipeline_replica_of_arrayfunc() {
    let order = [0usize, 1, 2, 3];
    let mut rng = Rng::new(0xC26_000);
    for _ in 0..2048 {
        let vals: Vec<i32> = (0..8).map(|_| rng.interesting_i32()).collect();
        drive_pipeline("C26 pipeline", &vals, 8, &order);
    }
    // Boundary value arrays.
    for v in [i32::MIN, i32::MIN + 1, -(1 << 30), -1, 0, 1, 1 << 30, i32::MAX - 1, i32::MAX] {
        drive_pipeline("C26 pipeline/uniform", &[v; 8], 8, &order);
    }
    let extremes = [i32::MIN, i32::MAX, 0, 1, -1];
    let mut rng = Rng::new(0xC26_001);
    for _ in 0..4096 {
        let vals: Vec<i32> = (0..8)
            .map(|_| extremes[(rng.next_u64() % extremes.len() as u64) as usize])
            .collect();
        drive_pipeline("C26 pipeline/extremes", &vals, 8, &order);
    }
}

#[test]
fn c27_pipeline_other_counts() {
    let order = [0usize, 1, 2, 3];
    let mut rng = Rng::new(0xC27_000);
    for count in [0i32, 1, 2, 5, 9, 10] {
        for _ in 0..1024 {
            let vals: Vec<i32> = (0..10).map(|_| rng.interesting_i32()).collect();
            drive_pipeline("C27 pipeline count", &vals, count, &order);
        }
    }
}

#[test]
fn c28_pipeline_reordered_and_duplicated_ops() {
    let orders: &[&[usize]] = &[
        &[3, 2, 1, 0],
        &[0, 0, 0, 0],
        &[3, 3, 3, 3],
        &[1, 1, 2, 2],
        &[2, 0, 3, 1, 0, 2],
        &[],
        &[3],
        &[1, 0],
    ];
    let mut rng = Rng::new(0xC28_000);
    for order in orders {
        for count in [0i32, 1, 8, 10] {
            for _ in 0..256 {
                let vals: Vec<i32> = (0..10).map(|_| rng.interesting_i32()).collect();
                drive_pipeline("C28 pipeline order", &vals, count, order);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// arrayfunc — the public entry point (C29..C33)
// ---------------------------------------------------------------------------

#[track_caller]
fn cmp_arrayfunc(row: &str, p: (i32, i32, i32, i32)) {
    let (c, r) = both();
    let cv = unsafe { (c.arrayfunc)(p.0, p.1, p.2, p.3) };
    let rv = unsafe { (r.arrayfunc)(p.0, p.1, p.2, p.3) };
    eq_i32(row, p, cv, rv);
}

#[test]
fn c29_arrayfunc_small_exhaustive() {
    let vs = [0i32, 1, -1, 2, -2];
    for &a in &vs {
        for &b in &vs {
            for &cc in &vs {
                for &d in &vs {
                    cmp_arrayfunc("C29 arrayfunc small", (a, b, cc, d));
                }
            }
        }
    }
}

#[test]
fn c30_arrayfunc_corner_matrix() {
    let vs = [
        i32::MIN,
        i32::MIN + 1,
        -(1 << 30),
        -1,
        0,
        1,
        1 << 30,
        i32::MAX - 1,
        i32::MAX,
    ];
    for &a in &vs {
        for &b in &vs {
            for &cc in &vs {
                for &d in &vs {
                    cmp_arrayfunc("C30 arrayfunc corners", (a, b, cc, d));
                }
            }
        }
    }
}

#[test]
fn c31_arrayfunc_uniform_random() {
    let mut rng = Rng::new(0xC31_000);
    for _ in 0..200_000 {
        let p = (rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32());
        cmp_arrayfunc("C31 arrayfunc random", p);
    }
    // Same count again but from the "interesting magnitude" distribution.
    for _ in 0..100_000 {
        let p = (
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
        cmp_arrayfunc("C31 arrayfunc interesting", p);
    }
}

#[test]
fn c32_arrayfunc_small_magnitude_random() {
    let mut rng = Rng::new(0xC32_000);
    for _ in 0..20_000 {
        let p = (
            rng.range_i32(-1000, 1000),
            rng.range_i32(-1000, 1000),
            rng.range_i32(-1000, 1000),
            rng.range_i32(-1000, 1000),
        );
        cmp_arrayfunc("C32 arrayfunc small-magnitude", p);
    }
    for _ in 0..20_000 {
        let p = (
            rng.range_i32(-30, 30),
            rng.range_i32(-30, 30),
            rng.range_i32(-30, 30),
            rng.range_i32(-30, 30),
        );
        cmp_arrayfunc("C32 arrayfunc tiny", p);
    }
}

#[test]
fn c33_arrayfunc_param4_division_truncation() {
    // `param4 / 2` truncates toward zero: check odd/even/negative-odd and the
    // INT_MIN boundary (ERRORS.md E25) against every sign pattern of p1..p3.
    let p4s = [
        i32::MIN,
        i32::MIN + 1,
        -7,
        -3,
        -2,
        -1,
        0,
        1,
        2,
        3,
        7,
        i32::MAX - 1,
        i32::MAX,
    ];
    let others = [-1000i32, -7, -1, 0, 1, 7, 1000];
    for &d in &p4s {
        for &a in &others {
            for &b in &others {
                for &cc in &others {
                    cmp_arrayfunc("C33 arrayfunc param4", (a, b, cc, d));
                }
            }
        }
    }
}
