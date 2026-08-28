//! Level 2: the functions that read or write the two file-scope statics
//! (`global_counter`, `global_accumulator`).
//!
//! Both libraries start with the statics at 0 and receive exactly the same call
//! sequence, so their hidden state must stay in lockstep. This lives in its own
//! integration-test binary (its own process) so that no other test can perturb
//! the sequence.

mod common;

use common::*;
use std::ffi::c_int;

#[test]
fn global_state_functions_match() {
    let libs = load();

    let (inc_c, inc_r) = libs.pair::<ModifierFunc>("increment_counter");
    let (upd_c, upd_r) = libs.pair::<ModifierFunc>("update_accumulator");
    let (calc_c, calc_r) = libs.pair::<FnTernary>("complex_calc");
    let (ppd_c, ppd_r) = libs.pair::<FnProcessPtr>("process_pointer_data");
    let (apply_c, apply_r) = libs.pair::<FnApplyOperation>("apply_operation");

    // `complex_calc(0, 0, 0)` == global_counter, and
    // `process_pointer_data(&0, 0)` == global_accumulator: non-destructive
    // probes for the otherwise invisible statics.
    let zero: c_int = 0;
    macro_rules! probe {
        ($ctx:expr) => {{
            let counter = unsafe { (calc_c(0, 0, 0), calc_r(0, 0, 0)) };
            assert_eq!(counter.0, counter.1, "global_counter after {}", $ctx);
            let acc = unsafe { (ppd_c(&zero, 0), ppd_r(&zero, 0)) };
            assert_eq!(acc.0, acc.1, "global_accumulator after {}", $ctx);
            (counter.0, acc.0)
        }};
    }

    // Both statics must start at zero.
    assert_eq!(probe!("startup"), (0, 0));

    // --- increment_counter: accumulates, ignores its second argument.
    for (i, &v) in INTS.iter().enumerate() {
        unsafe {
            inc_c(v, 999);
            inc_r(v, 999);
        }
        probe!(format!("increment_counter({v}) #{i}"));

        // The unused parameter must genuinely be unused.
        unsafe {
            inc_c(v, i as c_int * 7 - 3);
            inc_r(v, -(i as c_int));
        }
        probe!(format!("increment_counter({v}, <unused>) #{i}"));
    }

    // --- update_accumulator: acc = acc * 2 + value (wraps).
    for (i, &v) in INTS.iter().enumerate() {
        unsafe {
            upd_c(v, 888);
            upd_r(v, 888);
        }
        probe!(format!("update_accumulator({v}) #{i}"));
    }

    // --- complex_calc across the interesting inputs, with the counter being
    //     perturbed in between so several distinct global values are covered.
    for &a in INTS {
        for &b in INTS {
            for &c in INTS {
                let ec = unsafe { calc_c(a, b, c) };
                let er = unsafe { calc_r(a, b, c) };
                assert_eq!(ec, er, "complex_calc({a}, {b}, {c})");

                // Also through the function-pointer indirection.
                let ec = unsafe { apply_c(*calc_c, a, b, c) };
                let er = unsafe { apply_r(*calc_r, a, b, c) };
                assert_eq!(ec, er, "apply_operation(complex_calc, {a}, {b}, {c})");
            }
        }
        unsafe {
            inc_c(a, 0);
            inc_r(a, 0);
            upd_c(a, 0);
            upd_r(a, 0);
        }
        probe!(format!("perturb with {a}"));
    }

    // --- process_pointer_data over pointers into a shared buffer, including
    //     interior pointers (the C caller passes `&dynamic_data[5]`).
    let buf: Vec<c_int> = INTS.to_vec();
    for off in 0..buf.len() {
        for &m in INTS {
            let p = unsafe { buf.as_ptr().add(off) };
            let ec = unsafe { ppd_c(p, m) };
            let er = unsafe { ppd_r(p, m) };
            assert_eq!(ec, er, "process_pointer_data(&buf[{off}] = {}, {m})", buf[off]);
        }
        unsafe {
            upd_c(off as c_int, 0);
            upd_r(off as c_int, 0);
        }
        probe!(format!("perturb accumulator with {off}"));
    }
}
