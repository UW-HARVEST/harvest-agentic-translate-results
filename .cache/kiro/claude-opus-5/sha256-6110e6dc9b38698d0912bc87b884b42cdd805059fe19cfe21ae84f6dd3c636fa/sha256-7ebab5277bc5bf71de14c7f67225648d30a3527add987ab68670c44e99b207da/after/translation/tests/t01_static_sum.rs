//! Lowest level of the API: `int static_sum(int update)`.
//!
//! Exactly one `#[test]` lives in this file - see the note in `common/mod.rs`
//! about the process-wide accumulator and lockstep comparison.

mod common;

use common::{interesting_i32, Pair};

#[test]
fn static_sum_matches_c() {
    let pair = Pair::load();
    let (c_static_sum, rs_static_sum) = pair.static_sum_fns();

    // Independently tracked expectation of the running total, so a divergence
    // report can say what the accumulator should have been.
    let mut expected: i32 = 0;
    let mut step = 0usize;

    let check = |update: i32, expected: &mut i32, step: &mut usize| {
        *expected = expected.wrapping_add(update);
        let c_out = unsafe { c_static_sum(update) };
        let rs_out = unsafe { rs_static_sum(update) };
        assert_eq!(
            c_out, rs_out,
            "static_sum mismatch at step {} with update {}: C returned {}, Rust returned {} \
             (running total should be {})",
            *step, update, c_out, rs_out, *expected
        );
        assert_eq!(
            c_out, *expected,
            "C static_sum at step {} returned {} but the running total should be {}",
            *step, c_out, *expected
        );
        *step += 1;
    };

    // 1. First call from a pristine process: `sum` starts at 0.
    check(0, &mut expected, &mut step);

    // 2. The hand-picked interesting values, in order.
    for update in interesting_i32() {
        check(update, &mut expected, &mut step);
    }

    // 3. The same values again, so each is also exercised against a non-zero
    //    (and by now wrapped) accumulator.
    for update in interesting_i32() {
        check(update, &mut expected, &mut step);
    }

    // 4. A long deterministic pseudo-random sweep (xorshift32) to hammer the
    //    accumulator through many wraparounds.
    let mut state: u32 = 0x1234_5678;
    for _ in 0..20_000 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        check(state as i32, &mut expected, &mut step);
    }

    // 5. Repeatedly add i32::MAX / i32::MIN to force overflow in both
    //    directions.
    for _ in 0..64 {
        check(i32::MAX, &mut expected, &mut step);
        check(i32::MIN, &mut expected, &mut step);
        check(-1, &mut expected, &mut step);
    }

    assert!(step > 20_000, "expected a substantial number of comparisons");
}
