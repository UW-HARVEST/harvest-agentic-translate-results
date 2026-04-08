use std::any::Any;
use coroutine::coroutine::*;

fn dummy_yield(s: &mut Schedule, _ud: &mut dyn Any) {
    coroutine_yield(s);
}

fn yield_five_times(s: &mut Schedule, _ud: &mut dyn Any) {
    for _ in 0..5 {
        coroutine_yield(s);
    }
}

// -- coroutine_open tests --

#[test]
fn test_open_running_is_neg1() {
    let s = coroutine_open();
    assert_eq!(coroutine_running(&s), -1);
    coroutine_close(s);
}

#[test]
fn test_open_nco_is_zero() {
    let s = coroutine_open();
    assert_eq!(s.nco, 0);
    coroutine_close(s);
}

#[test]
fn test_open_cap_is_default() {
    let s = coroutine_open();
    assert_eq!(s.cap, DEFAULT_COROUTINE);
    assert_eq!(s.cap, 16);
    coroutine_close(s);
}

// -- coroutine_new tests --

#[test]
fn test_new_returns_sequential_ids() {
    let mut s = coroutine_open();
    let co1 = coroutine_new(&mut s, dummy_yield, Box::new(()));
    let co2 = coroutine_new(&mut s, dummy_yield, Box::new(()));
    assert_eq!(co1, 0);
    assert_eq!(co2, 1);
    coroutine_close(s);
}

#[test]
fn test_new_initial_status_is_ready() {
    let mut s = coroutine_open();
    let co = coroutine_new(&mut s, dummy_yield, Box::new(()));
    assert_eq!(coroutine_status(&s, co), COROUTINE_READY);
    assert_eq!(coroutine_status(&s, co), 1);
    coroutine_close(s);
}

#[test]
fn test_new_increments_nco() {
    let mut s = coroutine_open();
    assert_eq!(s.nco, 0);
    coroutine_new(&mut s, dummy_yield, Box::new(()));
    assert_eq!(s.nco, 1);
    coroutine_new(&mut s, dummy_yield, Box::new(()));
    assert_eq!(s.nco, 2);
    coroutine_close(s);
}

// -- coroutine_resume / coroutine_yield tests --

#[test]
fn test_resume_then_yield_sets_suspend() {
    let mut s = coroutine_open();
    let co = coroutine_new(&mut s, dummy_yield, Box::new(()));
    coroutine_resume(&mut s, co);
    assert_eq!(coroutine_status(&s, co), COROUTINE_SUSPEND);
    assert_eq!(coroutine_status(&s, co), 3);
    coroutine_close(s);
}

#[test]
fn test_running_neg1_after_yield() {
    let mut s = coroutine_open();
    let co = coroutine_new(&mut s, dummy_yield, Box::new(()));
    coroutine_resume(&mut s, co);
    assert_eq!(coroutine_running(&s), -1);
    coroutine_close(s);
}

#[test]
fn test_coroutine_finishes_becomes_dead() {
    let mut s = coroutine_open();
    let co = coroutine_new(&mut s, dummy_yield, Box::new(()));
    // First resume: coroutine yields -> SUSPEND
    coroutine_resume(&mut s, co);
    assert_eq!(coroutine_status(&s, co), COROUTINE_SUSPEND);
    // Second resume: coroutine returns -> DEAD
    coroutine_resume(&mut s, co);
    assert_eq!(coroutine_status(&s, co), COROUTINE_DEAD);
    assert_eq!(coroutine_status(&s, co), 0);
    coroutine_close(s);
}

#[test]
fn test_running_neg1_after_finish() {
    let mut s = coroutine_open();
    let co = coroutine_new(&mut s, dummy_yield, Box::new(()));
    coroutine_resume(&mut s, co);
    coroutine_resume(&mut s, co);
    assert_eq!(coroutine_running(&s), -1);
    coroutine_close(s);
}

// -- Two coroutines alternating (matches C test.c behavior) --

#[test]
fn test_two_coroutines_alternating() {
    let mut s = coroutine_open();
    let co1 = coroutine_new(&mut s, yield_five_times, Box::new(()));
    let co2 = coroutine_new(&mut s, yield_five_times, Box::new(()));
    assert_eq!(co1, 0);
    assert_eq!(co2, 1);

    // First resume each
    coroutine_resume(&mut s, co1);
    assert_eq!(coroutine_status(&s, co1), COROUTINE_SUSPEND);
    coroutine_resume(&mut s, co2);
    assert_eq!(coroutine_status(&s, co2), COROUTINE_SUSPEND);

    // Rounds 1-4: both yield
    for _ in 0..4 {
        coroutine_resume(&mut s, co1);
        assert_eq!(coroutine_status(&s, co1), COROUTINE_SUSPEND);
        coroutine_resume(&mut s, co2);
        assert_eq!(coroutine_status(&s, co2), COROUTINE_SUSPEND);
    }

    // Round 5: both finish (5th yield was the last, 6th resume finishes)
    coroutine_resume(&mut s, co1);
    assert_eq!(coroutine_status(&s, co1), COROUTINE_DEAD);
    coroutine_resume(&mut s, co2);
    assert_eq!(coroutine_status(&s, co2), COROUTINE_DEAD);

    assert_eq!(coroutine_running(&s), -1);
    coroutine_close(s);
}

// -- Capacity expansion --

#[test]
fn test_capacity_expansion_ids() {
    let mut s = coroutine_open();
    let mut ids = Vec::new();
    for _ in 0..20 {
        ids.push(coroutine_new(&mut s, dummy_yield, Box::new(())));
    }
    // C ground truth: IDs are 0..19 sequentially
    for i in 0..20 {
        assert_eq!(ids[i], i as i32);
    }
    // Cap should have expanded from 16 to 32
    assert_eq!(s.cap, 32);
    // All should be READY
    assert_eq!(coroutine_status(&s, ids[0]), COROUTINE_READY);
    assert_eq!(coroutine_status(&s, ids[15]), COROUTINE_READY);
    assert_eq!(coroutine_status(&s, ids[16]), COROUTINE_READY);
    assert_eq!(coroutine_status(&s, ids[19]), COROUTINE_READY);
    coroutine_close(s);
}

// -- Single coroutine full lifecycle --

#[test]
fn test_single_coroutine_lifecycle() {
    let mut s = coroutine_open();
    let co = coroutine_new(&mut s, dummy_yield, Box::new(()));
    assert_eq!(co, 0);
    assert_eq!(coroutine_status(&s, co), COROUTINE_READY);  // 1

    coroutine_resume(&mut s, co);
    assert_eq!(coroutine_status(&s, co), COROUTINE_SUSPEND); // 3
    assert_eq!(coroutine_running(&s), -1);

    coroutine_resume(&mut s, co);
    assert_eq!(coroutine_status(&s, co), COROUTINE_DEAD);    // 0
    assert_eq!(coroutine_running(&s), -1);
    coroutine_close(s);
}

// -- Constants match C defines --

#[test]
fn test_constants() {
    assert_eq!(COROUTINE_DEAD, 0);
    assert_eq!(COROUTINE_READY, 1);
    assert_eq!(COROUTINE_RUNNING, 2);
    assert_eq!(COROUTINE_SUSPEND, 3);
    assert_eq!(STACK_SIZE, 1024 * 1024);
    assert_eq!(DEFAULT_COROUTINE, 16);
}

// -- nco decrements when coroutine finishes --

#[test]
fn test_nco_decrements_on_finish() {
    let mut s = coroutine_open();
    let co = coroutine_new(&mut s, dummy_yield, Box::new(()));
    assert_eq!(s.nco, 1);
    coroutine_resume(&mut s, co); // yields
    assert_eq!(s.nco, 1);
    coroutine_resume(&mut s, co); // finishes
    assert_eq!(s.nco, 0);
    coroutine_close(s);
}

fn main() {}
