use coroutine::coroutine::{
    coroutine_close, coroutine_new, coroutine_open, coroutine_resume, coroutine_running,
    coroutine_status, coroutine_yield, COROUTINE_DEAD, COROUTINE_READY, COROUTINE_RUNNING,
    COROUTINE_SUSPEND, DEFAULT_COROUTINE, STACK_SIZE,
};
use std::any::Any;
use std::sync::{Mutex, MutexGuard};

static OUTPUT: Mutex<Vec<String>> = Mutex::new(Vec::new());
// Serialize tests that touch the shared OUTPUT (or test ordering of side effects).
static TEST_LOCK: Mutex<()> = Mutex::new(());

fn lock_test() -> MutexGuard<'static, ()> {
    match TEST_LOCK.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

fn clear_output() {
    OUTPUT.lock().unwrap().clear();
}

fn record(s: String) {
    OUTPUT.lock().unwrap().push(s);
}

fn snapshot_output() -> Vec<String> {
    OUTPUT.lock().unwrap().clone()
}

#[derive(Clone, Copy)]
struct Args {
    n: i32,
}

fn foo(s: &mut coroutine::coroutine::Schedule, ud: &mut dyn Any) {
    let arg = ud.downcast_mut::<Args>().unwrap();
    let start = arg.n;
    for i in 0..5 {
        record(format!("coroutine {} : {}", coroutine_running(s), start + i));
        coroutine_yield(s);
    }
}

#[test]
fn test_constants_match_c() {
    assert_eq!(COROUTINE_DEAD, 0);
    assert_eq!(COROUTINE_READY, 1);
    assert_eq!(COROUTINE_RUNNING, 2);
    assert_eq!(COROUTINE_SUSPEND, 3);
    assert_eq!(STACK_SIZE, 1024 * 1024);
    assert_eq!(DEFAULT_COROUTINE, 16);
}

#[test]
fn test_open_initial_state() {
    let s = coroutine_open();
    assert_eq!(s.nco, 0);
    assert_eq!(s.cap, DEFAULT_COROUTINE);
    assert_eq!(s.running, -1);
    assert_eq!(s.co.len(), DEFAULT_COROUTINE);
    for slot in s.co.iter() {
        assert!(slot.is_none());
    }
    assert_eq!(s.stack.len(), STACK_SIZE);
    assert_eq!(coroutine_running(&s), -1);
    coroutine_close(s);
}

#[test]
fn test_new_returns_sequential_ids() {
    let mut s = coroutine_open();
    let id0 = coroutine_new(&mut s, foo, Box::new(Args { n: 0 }));
    let id1 = coroutine_new(&mut s, foo, Box::new(Args { n: 100 }));
    let id2 = coroutine_new(&mut s, foo, Box::new(Args { n: 200 }));
    assert_eq!(id0, 0);
    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
    assert_eq!(s.nco, 3);
    assert_eq!(s.cap, DEFAULT_COROUTINE);
    coroutine_close(s);
}

#[test]
fn test_status_ready_after_new() {
    let mut s = coroutine_open();
    let id = coroutine_new(&mut s, foo, Box::new(Args { n: 0 }));
    assert_eq!(coroutine_status(&s, id), COROUTINE_READY);
    coroutine_close(s);
}

#[test]
fn test_status_dead_for_empty_slot() {
    let s = coroutine_open();
    // Slots that have never been used are DEAD.
    assert_eq!(coroutine_status(&s, 0), COROUTINE_DEAD);
    assert_eq!(coroutine_status(&s, 5), COROUTINE_DEAD);
    assert_eq!(coroutine_status(&s, 15), COROUTINE_DEAD);
    coroutine_close(s);
}

#[test]
fn test_running_minus_one_when_idle() {
    let s = coroutine_open();
    assert_eq!(coroutine_running(&s), -1);
    assert_eq!(s.running, -1);
    coroutine_close(s);
}

#[test]
fn test_resume_then_suspend_status() {
    let _g = lock_test();
    clear_output();
    let mut s = coroutine_open();
    let id = coroutine_new(&mut s, foo, Box::new(Args { n: 0 }));
    assert_eq!(coroutine_status(&s, id), COROUTINE_READY);
    coroutine_resume(&mut s, id);
    // foo yielded after first iteration
    assert_eq!(coroutine_status(&s, id), COROUTINE_SUSPEND);
    assert_eq!(coroutine_running(&s), -1);
    let out = snapshot_output();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0], "coroutine 0 : 0");
    coroutine_close(s);
}

#[test]
fn test_full_lifecycle_matches_c() {
    // This mirrors c_src/tests/test.c output exactly.
    let _g = lock_test();
    clear_output();
    let mut s = coroutine_open();
    let co1 = coroutine_new(&mut s, foo, Box::new(Args { n: 0 }));
    let co2 = coroutine_new(&mut s, foo, Box::new(Args { n: 100 }));
    assert_eq!(co1, 0);
    assert_eq!(co2, 1);
    record("main start".to_string());
    while coroutine_status(&s, co1) != COROUTINE_DEAD
        && coroutine_status(&s, co2) != COROUTINE_DEAD
    {
        coroutine_resume(&mut s, co1);
        coroutine_resume(&mut s, co2);
    }
    record("main end".to_string());

    let expected: Vec<String> = vec![
        "main start".into(),
        "coroutine 0 : 0".into(),
        "coroutine 1 : 100".into(),
        "coroutine 0 : 1".into(),
        "coroutine 1 : 101".into(),
        "coroutine 0 : 2".into(),
        "coroutine 1 : 102".into(),
        "coroutine 0 : 3".into(),
        "coroutine 1 : 103".into(),
        "coroutine 0 : 4".into(),
        "coroutine 1 : 104".into(),
        "main end".into(),
    ];
    let actual = snapshot_output();
    assert_eq!(actual, expected);

    // After the loop, both coroutines have completed.
    assert_eq!(coroutine_status(&s, co1), COROUTINE_DEAD);
    assert_eq!(coroutine_status(&s, co2), COROUTINE_DEAD);
    assert_eq!(s.running, -1);
    assert_eq!(s.nco, 0);

    coroutine_close(s);
}

fn just_finish(_s: &mut coroutine::coroutine::Schedule, _ud: &mut dyn Any) {
    // Returns immediately, no yield.
}

#[test]
fn test_coroutine_completes_without_yield() {
    let mut s = coroutine_open();
    let id = coroutine_new(&mut s, just_finish, Box::new(()));
    assert_eq!(coroutine_status(&s, id), COROUTINE_READY);
    assert_eq!(s.nco, 1);
    coroutine_resume(&mut s, id);
    // After running with no yield, the slot is cleared and counted.
    assert_eq!(coroutine_status(&s, id), COROUTINE_DEAD);
    assert_eq!(s.running, -1);
    assert_eq!(s.nco, 0);
    coroutine_close(s);
}

#[test]
fn test_resume_dead_id_is_noop() {
    // C: coroutine_resume returns when slot is NULL, leaving `running` unchanged.
    let mut s = coroutine_open();
    coroutine_resume(&mut s, 0);
    assert_eq!(coroutine_running(&s), -1);
    coroutine_close(s);
}

#[test]
fn test_id_reuse_after_completion() {
    // Once a coroutine finishes, its slot becomes empty and the next new
    // coroutine reuses an empty slot. With nco=0 after a single completion,
    // the search starts at id 0 again.
    let mut s = coroutine_open();
    let id0 = coroutine_new(&mut s, just_finish, Box::new(()));
    assert_eq!(id0, 0);
    coroutine_resume(&mut s, id0);
    assert_eq!(s.nco, 0);
    let id1 = coroutine_new(&mut s, just_finish, Box::new(()));
    // Should reuse slot 0.
    assert_eq!(id1, 0);
    assert_eq!(s.nco, 1);
    coroutine_resume(&mut s, id1);
    coroutine_close(s);
}

#[test]
fn test_grow_capacity_when_full() {
    // C: when nco >= cap, capacity doubles and id == old cap is returned.
    let _g = lock_test();
    clear_output();
    let mut s = coroutine_open();
    // Fill all DEFAULT_COROUTINE slots with sleeping coroutines.
    let mut ids = Vec::new();
    for i in 0..DEFAULT_COROUTINE {
        let id = coroutine_new(&mut s, foo, Box::new(Args { n: i as i32 * 1000 }));
        assert_eq!(id, i as i32);
        ids.push(id);
    }
    assert_eq!(s.nco, DEFAULT_COROUTINE);
    assert_eq!(s.cap, DEFAULT_COROUTINE);

    // Resume each so they go to SUSPEND (not DEAD), so slots stay occupied.
    for id in ids.iter() {
        coroutine_resume(&mut s, *id);
        assert_eq!(coroutine_status(&s, *id), COROUTINE_SUSPEND);
    }

    // Now adding another should grow the capacity.
    let new_id = coroutine_new(&mut s, foo, Box::new(Args { n: 9999 }));
    assert_eq!(new_id, DEFAULT_COROUTINE as i32);
    assert_eq!(s.cap, DEFAULT_COROUTINE * 2);
    assert_eq!(s.nco, DEFAULT_COROUTINE + 1);

    // The newly-grown half (other than the new slot) should be all None.
    for i in (DEFAULT_COROUTINE + 1)..(DEFAULT_COROUTINE * 2) {
        assert!(s.co[i].is_none());
        assert_eq!(coroutine_status(&s, i as i32), COROUTINE_DEAD);
    }

    coroutine_close(s);
}

#[test]
fn test_running_id_during_execution() {
    // While inside the coroutine, coroutine_running(S) should equal the id.
    static SEEN_RUNNING_ID: Mutex<i32> = Mutex::new(-99);

    fn record_running(s: &mut coroutine::coroutine::Schedule, _ud: &mut dyn Any) {
        *SEEN_RUNNING_ID.lock().unwrap() = coroutine_running(s);
    }

    let mut s = coroutine_open();
    let id = coroutine_new(&mut s, record_running, Box::new(()));
    coroutine_resume(&mut s, id);
    assert_eq!(*SEEN_RUNNING_ID.lock().unwrap(), id);
    assert_eq!(coroutine_running(&s), -1);
    coroutine_close(s);
}

fn main() {}
