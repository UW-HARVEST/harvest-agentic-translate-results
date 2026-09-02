//! Long-running interleaved fuzz: a random walk over *all* twelve exports,
//! in a random order, so that global state, buffer state and value-dependent
//! paths interact the way they would in a real consumer.
//!
//! Per-function tests can miss divergences that only appear at a particular
//! combination of `global_counter` / `global_accumulator` and argument values.
//! This test drives ~1.5M mixed operations from a fixed seed and compares every
//! single result and every post-call buffer.

mod harness;

use harness::*;
use std::ffi::c_int;

const STEPS: usize = 1_500_000;

#[test]
fn interleaved_random_walk_over_every_export() {
    let _g = lock();
    let p = libs();
    let mut rng = rng();

    let ops: [unsafe extern "C" fn(c_int, c_int, c_int) -> c_int; 3] =
        [p.c.add_three, p.c.multiply_add, p.c.complex_calc];
    let rops: [unsafe extern "C" fn(c_int, c_int, c_int) -> c_int; 3] =
        [p.r.add_three, p.r.multiply_add, p.r.complex_calc];

    for step in 0..STEPS {
        let a = rng.i32_interesting();
        let b = rng.i32_interesting();
        let c = rng.i32_interesting();
        let d = rng.i32_interesting();

        match rng.next_u64() % 12 {
            0 => {
                both_add_three(a, b, c);
            }
            1 => {
                both_multiply_add(a, b, c);
            }
            2 => {
                both_complex_calc(a, b, c);
            }
            3 => both_increment_counter(a, b),
            4 => both_update_accumulator(a, b),
            5 => {
                both_process_pointer_data(a, b);
            }
            6 => {
                let k = (rng.next_u64() % 3) as usize;
                both_apply_operation_with(
                    ["add_three", "multiply_add", "complex_calc"][k],
                    Some(ops[k]),
                    Some(rops[k]),
                    a,
                    b,
                    c,
                );
            }
            7 => {
                both_compute_with_dynamic_memory(a, rng.range(-64, 512));
            }
            8 => {
                both_get_time_based_value(a);
            }
            9 => {
                let n = rng.range(1, 24);
                let slack = rng.range(0, 8);
                let data: Vec<c_int> = (0..(n + slack) as usize).map(|_| rng.i32_any()).collect();
                // Only shapes the C survives: any shift, but size within the buffer.
                let shift_by = rng.range(-4, n + 4);
                both_shift_array_data(&data, n, shift_by);
            }
            10 => {
                let n = rng.range(1, 16);
                let slack = rng.range(0, 16);
                let recs = random_records(&mut rng, (n + slack) as usize);
                // Keep the wrapped loop bound inside the buffer.
                let shift = rng.range(-(slack), n + 4);
                let bound = n.wrapping_sub(shift);
                if bound <= (n + slack) && !(shift > 0 && shift < n && n > n + slack) {
                    both_manipulate_records(&recs, n, shift);
                }
            }
            _ => {
                both_hatch(a, b, c, d);
            }
        }

        // Periodically re-read both hidden globals through their exact probes so
        // a state divergence is caught close to where it happened rather than
        // only when it eventually leaks into a return value.
        if step % 64 == 0 {
            both_complex_calc(0, 0, 0); // == global_counter
            both_process_pointer_data(0, 0); // == global_accumulator
        }
    }
}
