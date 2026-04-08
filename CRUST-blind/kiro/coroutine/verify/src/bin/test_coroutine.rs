use std::any::Any;
use coroutine::coroutine::*;

#[test]
fn test_open_initial_state() {
    let s = coroutine_open();
    assert_eq!(s.running, -1);
    assert_eq!(s.nco, 0);
    assert_eq!(s.cap, 16);
}

#[test]
fn test_running_returns_neg1_initially() {
    let s = coroutine_open();
    assert_eq!(coroutine_running(&s), -1);
}

fn dummy_func(_s: &mut Schedule, _d: &mut dyn Any) {}

#[test]
fn test_new_returns_sequential_ids() {
    let mut s = coroutine_open();
    let id0 = coroutine_new(&mut s, dummy_func, Box::new(()));
    let id1 = coroutine_new(&mut s, dummy_func, Box::new(()));
    assert_eq!(id0, 0);
    assert_eq!(id1, 1);
    assert_eq!(s.nco, 2);
}

#[test]
fn test_new_status_is_ready() {
    let mut s = coroutine_open();
    let id = coroutine_new(&mut s, dummy_func, Box::new(()));
    assert_eq!(coroutine_status(&s, id), COROUTINE_READY);
}

#[test]
fn test_status_dead_after_completion() {
    let mut s = coroutine_open();
    let id = coroutine_new(&mut s, dummy_func, Box::new(()));
    coroutine_resume(&mut s, id);
    assert_eq!(coroutine_status(&s, id), COROUTINE_DEAD);
}

#[test]
fn test_status_suspend_after_yield() {
    fn yielding(s: &mut Schedule, _d: &mut dyn Any) {
        coroutine_yield(s);
    }
    let mut s = coroutine_open();
    let id = coroutine_new(&mut s, yielding, Box::new(()));
    coroutine_resume(&mut s, id);
    assert_eq!(coroutine_status(&s, id), COROUTINE_SUSPEND);
    assert_eq!(coroutine_running(&s), -1);
    coroutine_resume(&mut s, id);
    assert_eq!(coroutine_status(&s, id), COROUTINE_DEAD);
}

#[test]
fn test_running_inside_coroutine() {
    fn check_running(s: &mut Schedule, d: &mut dyn Any) {
        let expected_id = d.downcast_ref::<i32>().unwrap();
        assert_eq!(coroutine_running(s), *expected_id);
    }
    let mut s = coroutine_open();
    let id = coroutine_new(&mut s, check_running, Box::new(0i32));
    coroutine_resume(&mut s, id);
    assert_eq!(coroutine_running(&s), -1);
}

#[test]
fn test_nco_decrements_on_completion() {
    let mut s = coroutine_open();
    let id = coroutine_new(&mut s, dummy_func, Box::new(()));
    assert_eq!(s.nco, 1);
    coroutine_resume(&mut s, id);
    assert_eq!(s.nco, 0);
}

#[test]
fn test_close_does_not_panic() {
    let s = coroutine_open();
    coroutine_close(s);
}

#[test]
fn test_two_coroutines_interleave() {
    fn foo(s: &mut Schedule, _d: &mut dyn Any) {
        for _ in 0..5 {
            coroutine_yield(s);
        }
    }

    let mut s = coroutine_open();
    let co1 = coroutine_new(&mut s, foo, Box::new(0i32));
    let co2 = coroutine_new(&mut s, foo, Box::new(100i32));
    assert_eq!(co1, 0);
    assert_eq!(co2, 1);

    while coroutine_status(&s, co1) != COROUTINE_DEAD
        && coroutine_status(&s, co2) != COROUTINE_DEAD
    {
        coroutine_resume(&mut s, co1);
        assert_eq!(coroutine_running(&s), -1);
        coroutine_resume(&mut s, co2);
        assert_eq!(coroutine_running(&s), -1);
    }
    assert_eq!(coroutine_status(&s, co1), COROUTINE_DEAD);
    assert_eq!(coroutine_status(&s, co2), COROUTINE_DEAD);
    coroutine_close(s);
}

#[test]
fn test_full_c_ground_truth() {
    fn foo(s: &mut Schedule, d: &mut dyn Any) {
        let start = *d.downcast_ref::<i32>().unwrap();
        for i in 0..5 {
            let id = coroutine_running(s);
            assert!(id >= 0);
            let _ = format!("coroutine {} : {}", id, start + i);
            coroutine_yield(s);
        }
    }

    let mut s = coroutine_open();
    let co1 = coroutine_new(&mut s, foo, Box::new(0i32));
    let co2 = coroutine_new(&mut s, foo, Box::new(100i32));

    while coroutine_status(&s, co1) != COROUTINE_DEAD
        && coroutine_status(&s, co2) != COROUTINE_DEAD
    {
        coroutine_resume(&mut s, co1);
        coroutine_resume(&mut s, co2);
    }

    assert_eq!(coroutine_status(&s, co1), COROUTINE_DEAD);
    assert_eq!(coroutine_status(&s, co2), COROUTINE_DEAD);
    assert_eq!(coroutine_running(&s), -1);
    assert_eq!(s.nco, 0);
    coroutine_close(s);
}

#[test]
fn test_capacity_expansion() {
    let mut s = coroutine_open();
    assert_eq!(s.cap, 16);
    for _ in 0..16 {
        coroutine_new(&mut s, dummy_func, Box::new(()));
    }
    assert_eq!(s.nco, 16);
    let id = coroutine_new(&mut s, dummy_func, Box::new(()));
    assert_eq!(id, 16);
    assert_eq!(s.cap, 32);
    assert_eq!(s.nco, 17);
    coroutine_close(s);
}

#[test]
fn test_slot_reuse() {
    let mut s = coroutine_open();
    let id0 = coroutine_new(&mut s, dummy_func, Box::new(()));
    let id1 = coroutine_new(&mut s, dummy_func, Box::new(()));
    assert_eq!(id0, 0);
    assert_eq!(id1, 1);
    coroutine_resume(&mut s, id0);
    assert_eq!(s.nco, 1);
    // C search: (i+nco)%cap -> i=0: slot 1 (taken), i=1: slot 2 (free)
    let id2 = coroutine_new(&mut s, dummy_func, Box::new(()));
    assert_eq!(id2, 2);
    coroutine_close(s);
}

#[test]
fn test_constants() {
    assert_eq!(COROUTINE_DEAD, 0);
    assert_eq!(COROUTINE_READY, 1);
    assert_eq!(COROUTINE_RUNNING, 2);
    assert_eq!(COROUTINE_SUSPEND, 3);
    assert_eq!(DEFAULT_COROUTINE, 16);
    assert_eq!(STACK_SIZE, 1024 * 1024);
}

#[test]
fn test_resume_dead_is_noop() {
    let mut s = coroutine_open();
    let id = coroutine_new(&mut s, dummy_func, Box::new(()));
    coroutine_resume(&mut s, id);
    assert_eq!(coroutine_status(&s, id), COROUTINE_DEAD);
    coroutine_resume(&mut s, id);
    assert_eq!(coroutine_status(&s, id), COROUTINE_DEAD);
}

fn main() {}
