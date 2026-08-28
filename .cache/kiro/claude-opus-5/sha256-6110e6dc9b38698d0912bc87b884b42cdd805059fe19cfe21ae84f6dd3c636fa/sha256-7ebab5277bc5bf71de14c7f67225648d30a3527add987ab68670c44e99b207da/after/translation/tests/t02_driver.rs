//! Higher level of the API: `void driver(int stride)`.
//!
//! `driver` has no return value; its entire observable effect is the ten lines
//! it prints via C `stdout`, plus the ten updates it applies to the `static_sum`
//! accumulator. Both are compared here: stdout is captured byte-for-byte at the
//! file-descriptor level, and the accumulator is probed with `static_sum(0)`
//! after each pair of calls.
//!
//! Exactly one `#[test]` lives in this file - see the note in `common/mod.rs`.

mod common;

use common::{capture_stdout, interesting_i32, Pair};

#[test]
fn driver_matches_c() {
    let pair = Pair::load();
    let (c_driver, rs_driver) = pair.driver_fns();
    let (c_static_sum, rs_static_sum) = pair.static_sum_fns();

    let mut step = 0usize;

    let check = |stride: i32, step: &mut usize| {
        // Capture each implementation's output separately so the comparison is
        // of complete, independent byte streams.
        let (_, c_out) = capture_stdout(|| unsafe { c_driver(stride) });
        let (_, rs_out) = capture_stdout(|| unsafe { rs_driver(stride) });

        assert_eq!(
            c_out,
            rs_out,
            "driver({stride}) stdout mismatch at step {}\n  C   : {:?}\n  Rust: {:?}",
            *step,
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&rs_out),
        );

        // `driver` must also have moved the running total identically. A
        // `static_sum(0)` probe reads the accumulator without disturbing it.
        let c_total = unsafe { c_static_sum(0) };
        let rs_total = unsafe { rs_static_sum(0) };
        assert_eq!(
            c_total, rs_total,
            "running total diverged after driver({stride}) at step {}: C {} vs Rust {}",
            *step, c_total, rs_total
        );

        // Sanity check on the captured stream itself: ten newline-terminated
        // decimal lines.
        assert_eq!(
            c_out.iter().filter(|b| **b == b'\n').count(),
            10,
            "driver({stride}) should print exactly 10 lines, got {:?}",
            String::from_utf8_lossy(&c_out)
        );

        *step += 1;
    };

    // 1. stride 0 first: exercises driver against a pristine accumulator.
    check(0, &mut step);

    // 2. Small strides, both signs.
    for stride in -20i32..=20 {
        check(stride, &mut step);
    }

    // 3. The interesting values, including the ones whose `i * stride` products
    //    and running totals overflow.
    for stride in interesting_i32() {
        check(stride, &mut step);
    }

    // 4. Interleave `static_sum` updates with `driver` calls so `driver` is also
    //    seen starting from arbitrary accumulator values.
    let mut state: u32 = 0x9E37_79B9;
    for _ in 0..400 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;

        let update = state as i32;
        let c_out = unsafe { c_static_sum(update) };
        let rs_out = unsafe { rs_static_sum(update) };
        assert_eq!(
            c_out, rs_out,
            "static_sum({update}) mismatch while interleaving with driver at step {}",
            step
        );

        check(update, &mut step);
    }
}
