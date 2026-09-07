//! Phase B — valid-path differential tests for `perform_computation_with_history`,
//! the lowest-level *stateful* entry point. CONFIGS.md rows 14-18 (and 25's
//! composed-pipeline half).

mod common;
use common::*;
use std::os::raw::c_int;

const SEED: u64 = 0x5EED_1234_ABCD_0002;

/// Read `n` slots out of a history buffer as raw bytes (layout-agnostic compare).
unsafe fn img(p: *mut ComputationResult, n: usize) -> Vec<u8> {
    std::slice::from_raw_parts(p as *const u8, n * std::mem::size_of::<ComputationResult>())
        .to_vec()
}

unsafe fn slots(p: *mut ComputationResult, n: usize) -> Vec<ComputationResult> {
    std::slice::from_raw_parts(p as *const ComputationResult, n).to_vec()
}

// ---------------------------------------------------------------------------
// Row 14 — *history == NULL: allocates, RESETS *history_count to 0, writes [0]
// ---------------------------------------------------------------------------
#[test]
fn row_14_null_history_allocates_and_resets_count() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 14);

    // For each of the 5 valid ops, and for several garbage initial counts.
    for op in 1..=5i32 {
        for &garbage in &[0i32, 7, 42, -3, i32::MAX] {
            for _ in 0..40 {
                let a = rng.i32_interesting();
                let b = rng.nonzero_i32();
                if is_div_trap(a, b) {
                    continue;
                }

                let mut hc: *mut ComputationResult = std::ptr::null_mut();
                let mut hr: *mut ComputationResult = std::ptr::null_mut();
                let mut cc: c_int = garbage;
                let mut cr: c_int = garbage;

                let vc = unsafe { p.c.perform_computation_with_history(a, b, op, &mut hc, &mut cc) };
                let vr =
                    unsafe { p.rs.perform_computation_with_history(a, b, op, &mut hr, &mut cr) };

                assert_eq!(vc, vr, "return value for op={op} a={a} b={b}");
                assert_eq!(cc, cr, "history_count for op={op} a={a} b={b}");
                assert_eq!(cc, 1, "NULL history must reset count to 0 then write slot 0");
                assert!(!hc.is_null() && !hr.is_null(), "both must allocate");
                assert_eq!(
                    unsafe { img(hc, 10) },
                    unsafe { img(hr, 10) },
                    "10-slot history image differs for op={op} a={a} b={b}"
                );
                // The 9 untouched slots must still be calloc-zero.
                let rest = unsafe { slots(hc, 10) };
                assert!(
                    rest[1..].iter().all(|s| *s == ComputationResult::default()),
                    "slots 1..9 must remain zeroed"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 15 — pre-allocated buffer, filling slots 0..8 one at a time
// ---------------------------------------------------------------------------
#[test]
fn row_15_preallocated_fill_slots_incrementally() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 15);

    for trial in 0..40 {
        let mut hc = p.c.allocate_results(10);
        let mut hr = p.rs.allocate_results(10);
        assert!(!hc.is_null() && !hr.is_null());
        let mut cc: c_int = 0;
        let mut cr: c_int = 0;

        for step in 0..9 {
            let op = (rng.next_u64() % 5) as i32 + 1;
            let a = rng.i32_interesting();
            let b = rng.nonzero_i32();
            if is_div_trap(a, b) {
                continue;
            }
            let vc = unsafe { p.c.perform_computation_with_history(a, b, op, &mut hc, &mut cc) };
            let vr = unsafe { p.rs.perform_computation_with_history(a, b, op, &mut hr, &mut cr) };
            assert_eq!(vc, vr, "trial {trial} step {step}: op={op} a={a} b={b}");
            assert_eq!(cc, cr, "trial {trial} step {step}: history_count");
            assert_eq!(
                unsafe { img(hc, 10) },
                unsafe { img(hr, 10) },
                "trial {trial} step {step}: buffer image (op={op} a={a} b={b})"
            );
            // Every written slot must carry STATUS_SUCCESS == 0.
            for s in unsafe { slots(hc, cc.max(0) as usize) } {
                assert_eq!(s.status, 0, "status must always be STATUS_SUCCESS");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 16 — drive 10 times, then keep going past the `< 10` bound
// ---------------------------------------------------------------------------
#[test]
fn row_16_fills_then_saturates_at_ten() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 16);

    for trial in 0..25 {
        let mut hc = p.c.allocate_results(10);
        let mut hr = p.rs.allocate_results(10);
        let mut cc: c_int = 0;
        let mut cr: c_int = 0;

        for step in 0..16 {
            let op = (rng.next_u64() % 5) as i32 + 1;
            let a = rng.i32_interesting();
            let b = rng.nonzero_i32();
            if is_div_trap(a, b) {
                continue;
            }
            let vc = unsafe { p.c.perform_computation_with_history(a, b, op, &mut hc, &mut cc) };
            let vr = unsafe { p.rs.perform_computation_with_history(a, b, op, &mut hr, &mut cr) };
            assert_eq!(vc, vr, "trial {trial} step {step} return");
            assert_eq!(cc, cr, "trial {trial} step {step} count");
            assert!(cc <= 10, "count must never exceed 10, got {cc}");
            assert_eq!(
                unsafe { img(hc, 10) },
                unsafe { img(hr, 10) },
                "trial {trial} step {step} buffer"
            );
        }
        assert_eq!(cc, 10, "after >=10 recording calls the count must sit at 10");
    }
}

// ---------------------------------------------------------------------------
// Row 17 — seeded counts at the exact boundary (9 writable, 10 rejecting)
// ---------------------------------------------------------------------------
#[test]
fn row_17_boundary_counts_nine_and_ten() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 17);

    for &seed_count in &[8i32, 9, 10] {
        for op in 1..=5i32 {
            for _ in 0..30 {
                let a = rng.i32_interesting();
                let b = rng.nonzero_i32();
                if is_div_trap(a, b) {
                    continue;
                }
                let mut hc = p.c.allocate_results(10);
                let mut hr = p.rs.allocate_results(10);
                let mut cc: c_int = seed_count;
                let mut cr: c_int = seed_count;

                let vc = unsafe { p.c.perform_computation_with_history(a, b, op, &mut hc, &mut cc) };
                let vr =
                    unsafe { p.rs.perform_computation_with_history(a, b, op, &mut hr, &mut cr) };

                assert_eq!(vc, vr, "count={seed_count} op={op} a={a} b={b} return");
                assert_eq!(cc, cr, "count={seed_count} op={op} count after");
                let expect = if seed_count < 10 { seed_count + 1 } else { seed_count };
                assert_eq!(cc, expect, "count={seed_count}: `< 10` bound behaviour");
                assert_eq!(
                    unsafe { img(hc, 10) },
                    unsafe { img(hr, 10) },
                    "count={seed_count} op={op} buffer"
                );
                if seed_count == 10 {
                    assert!(
                        unsafe { slots(hc, 10) }.iter().all(|s| *s == ComputationResult::default()),
                        "count==10 must write nothing at all"
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 18 — out-of-enum op values must behave as addition
// ---------------------------------------------------------------------------
#[test]
fn row_18_out_of_enum_op_records_sum() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 18);
    let mut ops = vec![0i32, 6, 7, -1, -2, -5, 99, i32::MAX, i32::MIN];
    for _ in 0..200 {
        let v = rng.i32_interesting();
        if !(1..=5).contains(&v) {
            ops.push(v);
        }
    }
    for op in ops {
        for _ in 0..6 {
            let a = rng.i32_interesting();
            let b = rng.i32_interesting();
            let mut hc: *mut ComputationResult = std::ptr::null_mut();
            let mut hr: *mut ComputationResult = std::ptr::null_mut();
            let mut cc: c_int = 0;
            let mut cr: c_int = 0;
            let vc = unsafe { p.c.perform_computation_with_history(a, b, op, &mut hc, &mut cc) };
            let vr = unsafe { p.rs.perform_computation_with_history(a, b, op, &mut hr, &mut cr) };
            assert_eq!(vc, vr, "op={op} a={a} b={b}");
            assert_eq!(vc, a.wrapping_add(b), "default: branch must add");
            assert_eq!(cc, cr);
            assert_eq!(unsafe { img(hc, 10) }, unsafe { img(hr, 10) }, "op={op} buffer");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 25 (part) — composed pipeline: select_operation -> returned pointer ->
// perform_computation_with_history on a caller-owned buffer, all in one flow.
// ---------------------------------------------------------------------------
#[test]
fn row_25_composed_pipeline_select_then_record() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 25);

    let mut hc = p.c.allocate_results(10);
    let mut hr = p.rs.allocate_results(10);
    let mut cc: c_int = 0;
    let mut cr: c_int = 0;

    for step in 0..300 {
        // Pick op from the whole i32 space so both valid and default: paths run.
        let op = if step % 3 == 0 { (step % 7) as i32 } else { rng.i32_interesting() };
        let a = rng.i32_interesting();
        let b = rng.nonzero_i32();
        if is_div_trap(a, b) {
            continue;
        }

        // 1. lowest level: resolve the kernel through select_operation
        let fc = p.c.select_operation(op);
        let fr = p.rs.select_operation(op);
        let dc = unsafe { fc(a, b, 0) };
        let dr = unsafe { fr(a, b, 0) };
        assert_eq!(dc, dr, "step {step}: direct kernel call op={op}");

        // 2. next level: the same op through the recording wrapper
        let vc = unsafe { p.c.perform_computation_with_history(a, b, op, &mut hc, &mut cc) };
        let vr = unsafe { p.rs.perform_computation_with_history(a, b, op, &mut hr, &mut cr) };
        assert_eq!(vc, vr, "step {step}: recorded op={op}");
        assert_eq!(vc, dc, "wrapper must return exactly what the kernel returned");
        assert_eq!(cc, cr, "step {step}: count");
        assert_eq!(unsafe { img(hc, 10) }, unsafe { img(hr, 10) }, "step {step}: buffer");

        // 3. and the priority helper on the same op value
        assert_eq!(
            p.c.get_operation_priority(op),
            p.rs.get_operation_priority(op),
            "step {step}: priority op={op}"
        );
    }
    assert_eq!(cc, 10, "buffer should have saturated");
}
