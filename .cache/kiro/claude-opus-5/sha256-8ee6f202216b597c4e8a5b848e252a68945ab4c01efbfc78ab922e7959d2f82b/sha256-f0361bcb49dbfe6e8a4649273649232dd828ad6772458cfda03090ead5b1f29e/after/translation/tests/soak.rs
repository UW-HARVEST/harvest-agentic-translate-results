//! High-volume soak: the same mixed operation stream as `CONFIGS.md` row 33,
//! but across many independent seeds and far more steps, so value-dependent and
//! index-dependent divergences that a few hundred samples could miss get a real
//! chance to show up. Both libraries are driven in identical order through
//! their `.so` exports only.

mod common;

use common::*;
use std::os::raw::c_int;

fn rand_records(rng: &mut Rng, n: usize) -> Vec<DataRecord> {
    (0..n)
        .map(|i| {
            let mut name = [0i8; 32];
            for b in name.iter_mut() {
                *b = rng.i8_any();
            }
            DataRecord { id: i as c_int, value: rng.i32_mixed(), timestamp: rng.i64_any(), name }
        })
        .collect()
}

fn one_seed(p: &Pair, seed: u64, steps: u32) {
    p.reset();
    let mut rng = Rng::new(seed);
    for step in 0..steps {
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
                let which = rng.next_u64() % 3;
                let (cf, rf) = match which {
                    0 => (p.c.add_three, p.r.add_three),
                    1 => (p.c.multiply_add, p.r.multiply_add),
                    _ => (p.c.complex_calc, p.r.complex_calc),
                };
                eq_i32(
                    "soak apply_operation",
                    (seed, step, which, a, b, c),
                    unsafe { (p.c.apply_operation)(Some(cf), a, b, c) },
                    unsafe { (p.r.apply_operation)(Some(rf), a, b, c) },
                );
            }
            3 => {
                let (a, b, c) = (rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed());
                eq_i32(
                    "soak add_three",
                    (seed, step, a, b, c),
                    unsafe { (p.c.add_three)(a, b, c) },
                    unsafe { (p.r.add_three)(a, b, c) },
                );
            }
            4 => {
                let (a, b, c) = (rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed());
                eq_i32(
                    "soak multiply_add",
                    (seed, step, a, b, c),
                    unsafe { (p.c.multiply_add)(a, b, c) },
                    unsafe { (p.r.multiply_add)(a, b, c) },
                );
            }
            5 => {
                let (a, b, c) = (rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed());
                eq_i32(
                    "soak complex_calc",
                    (seed, step, a, b, c),
                    unsafe { (p.c.complex_calc)(a, b, c) },
                    unsafe { (p.r.complex_calc)(a, b, c) },
                );
            }
            6 => {
                // sizes and shifts straddling every guard boundary
                let size = rng.range(-2, 34);
                let shift_by = rng.range(-4, 38);
                let data: Vec<c_int> = (0..32).map(|_| rng.i32_mixed()).collect();
                let mut cb = data.clone();
                let mut rb = data;
                unsafe { (p.c.shift_array_data)(cb.as_mut_ptr(), size.min(32), shift_by) };
                unsafe { (p.r.shift_array_data)(rb.as_mut_ptr(), size.min(32), shift_by) };
                eq_bytes("soak shift_array_data", (seed, step, size, shift_by), &cb, &rb);
            }
            7 => {
                let mut cv = rng.i32_mixed();
                let mut rv = cv;
                let m = rng.i32_mixed();
                eq_i32(
                    "soak process_pointer_data",
                    (seed, step, cv, m),
                    unsafe { (p.c.process_pointer_data)(&mut cv, m) },
                    unsafe { (p.r.process_pointer_data)(&mut rv, m) },
                );
            }
            8 => {
                let base = rng.i32_mixed();
                let count = rng.range(-4, 128);
                eq_i32(
                    "soak compute_with_dynamic_memory",
                    (seed, step, base, count),
                    unsafe { (p.c.compute_with_dynamic_memory)(base, count) },
                    unsafe { (p.r.compute_with_dynamic_memory)(base, count) },
                );
            }
            9 => {
                let s = rng.i32_mixed();
                eq_i32(
                    "soak get_time_based_value",
                    (seed, step, s),
                    unsafe { (p.c.get_time_based_value)(s) },
                    unsafe { (p.r.get_time_based_value)(s) },
                );
            }
            10 => {
                // 16 slots allocated; n and shift range past them only in the
                // negative-shift direction bounded to stay inside the buffer
                let n = rng.range(0, 8);
                let shift = rng.range(-8, 10);
                let recs = rand_records(&mut rng, 16);
                let mut cb = recs.clone();
                let mut rb = recs;
                let cr = unsafe { (p.c.manipulate_records)(cb.as_mut_ptr(), n, shift) };
                let rr = unsafe { (p.r.manipulate_records)(rb.as_mut_ptr(), n, shift) };
                eq_i32("soak manipulate_records", (seed, step, n, shift), cr, rr);
                eq_bytes(
                    "soak manipulate_records buffer",
                    (seed, step, n, shift),
                    as_raw_bytes(&cb),
                    as_raw_bytes(&rb),
                );
            }
            _ => {
                let (a, b, c, d) =
                    (rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed());
                eq_i32(
                    "soak hatch",
                    (seed, step, a, b, c, d),
                    unsafe { (p.c.hatch)(a, b, c, d) },
                    unsafe { (p.r.hatch)(a, b, c, d) },
                );
                // hatch mutates both statics; they must stay in lockstep
                eq_i32(
                    "soak global_counter after hatch",
                    (seed, step),
                    Pair::counter(&p.c),
                    Pair::counter(&p.r),
                );
                eq_i32(
                    "soak global_accumulator after hatch",
                    (seed, step),
                    Pair::accumulator(&p.c),
                    Pair::accumulator(&p.r),
                );
            }
        }
    }
}

#[test]
fn soak_mixed_stream_many_seeds() {
    let (_g, p) = locked();
    for k in 0..16u64 {
        one_seed(p, SEED.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(k), 20_000);
    }
}

/// Dense sweep of every `(size, shift_by)` pair around the guard boundary,
/// exhaustively rather than randomly.
#[test]
fn soak_shift_array_exhaustive_small_grid() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 0xA11);
    for size in -2..=24i32 {
        for shift_by in -4..=28i32 {
            for _ in 0..3 {
                let data: Vec<c_int> = (0..24).map(|_| rng.i32_mixed()).collect();
                let mut cb = data.clone();
                let mut rb = data;
                unsafe { (p.c.shift_array_data)(cb.as_mut_ptr(), size, shift_by) };
                unsafe { (p.r.shift_array_data)(rb.as_mut_ptr(), size, shift_by) };
                eq_bytes("grid shift_array_data", (size, shift_by), &cb, &rb);
            }
        }
    }
}

/// Dense sweep of every `(num_records, shift)` pair around both the memmove
/// guard and the un-guarded loop bound, exhaustively.
#[test]
fn soak_manipulate_records_exhaustive_small_grid() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 0xB22);
    for n in -2..=12i32 {
        for shift in -12..=16i32 {
            // keep n - shift inside the 32-record allocation
            if (n as i64 - shift as i64) > 32 {
                continue;
            }
            for _ in 0..3 {
                let recs = rand_records(&mut rng, 32);
                let mut cb = recs.clone();
                let mut rb = recs;
                let cr = unsafe { (p.c.manipulate_records)(cb.as_mut_ptr(), n, shift) };
                let rr = unsafe { (p.r.manipulate_records)(rb.as_mut_ptr(), n, shift) };
                eq_i32("grid manipulate_records", (n, shift), cr, rr);
                eq_bytes(
                    "grid manipulate_records buffer",
                    (n, shift),
                    as_raw_bytes(&cb),
                    as_raw_bytes(&rb),
                );
            }
        }
    }
}

/// Exhaustive sweep of `compute_with_dynamic_memory`'s count around 0 and the
/// value `hatch` uses, over a spread of bases.
#[test]
fn soak_dynamic_memory_exhaustive_small_grid() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 0xC33);
    for count in -8..=72i32 {
        for _ in 0..8 {
            let base = rng.i32_mixed();
            eq_i32(
                "grid compute_with_dynamic_memory",
                (base, count),
                unsafe { (p.c.compute_with_dynamic_memory)(base, count) },
                unsafe { (p.r.compute_with_dynamic_memory)(base, count) },
            );
        }
    }
}

/// Exhaustive small-integer sweep of the three pure operations, so no
/// value-dependent branch can hide.
#[test]
fn soak_pure_ops_exhaustive_small_grid() {
    let (_g, p) = locked();
    for a in -6..=6i32 {
        for b in -6..=6i32 {
            for c in -6..=6i32 {
                eq_i32("grid add_three", (a, b, c), unsafe { (p.c.add_three)(a, b, c) }, unsafe {
                    (p.r.add_three)(a, b, c)
                });
                eq_i32("grid multiply_add", (a, b, c), unsafe { (p.c.multiply_add)(a, b, c) }, unsafe {
                    (p.r.multiply_add)(a, b, c)
                });
                eq_i32("grid complex_calc", (a, b, c), unsafe { (p.c.complex_calc)(a, b, c) }, unsafe {
                    (p.r.complex_calc)(a, b, c)
                });
            }
        }
    }
}

/// Exhaustive sweep of `get_time_based_value` right at the `seed * 3600`
/// int-overflow boundary, where the C's UB-wrap is easiest to get wrong.
#[test]
fn soak_get_time_based_value_overflow_boundary() {
    let (_g, p) = locked();
    const T: i32 = 596_523; // largest seed with seed*3600 <= INT_MAX
    for d in -40..=40i32 {
        for &centre in &[T, -T, 0, i32::MAX / 3600, i32::MIN / 3600] {
            let seed = centre.wrapping_add(d);
            eq_i32(
                "boundary get_time_based_value",
                seed,
                unsafe { (p.c.get_time_based_value)(seed) },
                unsafe { (p.r.get_time_based_value)(seed) },
            );
        }
    }
    for &seed in &[i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1] {
        eq_i32(
            "boundary get_time_based_value",
            seed,
            unsafe { (p.c.get_time_based_value)(seed) },
            unsafe { (p.r.get_time_based_value)(seed) },
        );
    }
}

/// `hatch` over a dense small grid plus repeated invocations, comparing the
/// return value and both statics after every call.
#[test]
fn soak_hatch_grid_and_state() {
    let (_g, p) = locked();
    for a in -3..=3i32 {
        for b in -3..=3i32 {
            for c in -3..=3i32 {
                for d in -3..=3i32 {
                    p.reset();
                    eq_i32(
                        "grid hatch",
                        (a, b, c, d),
                        unsafe { (p.c.hatch)(a, b, c, d) },
                        unsafe { (p.r.hatch)(a, b, c, d) },
                    );
                    eq_i32("grid counter", (a, b, c, d), Pair::counter(&p.c), Pair::counter(&p.r));
                    eq_i32(
                        "grid accumulator",
                        (a, b, c, d),
                        Pair::accumulator(&p.c),
                        Pair::accumulator(&p.r),
                    );
                }
            }
        }
    }
}
