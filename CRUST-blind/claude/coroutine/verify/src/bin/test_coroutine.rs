#![allow(unused_imports, dead_code)]
use coroutine::coroutine::{
    coroutine_close, coroutine_new, coroutine_open, coroutine_resume, coroutine_running,
    coroutine_status, coroutine_yield, Schedule, COROUTINE_DEAD, COROUTINE_READY, COROUTINE_RUNNING,
    COROUTINE_SUSPEND, DEFAULT_COROUTINE, STACK_SIZE,
};
use std::any::Any;
use std::sync::{Arc, Mutex};

// ---------- helpers / shared coroutine functions ----------

struct Args {
    n: i32,
    log: Arc<Mutex<Vec<(i32, i32)>>>,
}

fn foo(s: &mut Schedule, data: &mut dyn Any) {
    let (start, log) = {
        let args = data.downcast_ref::<Args>().unwrap();
        (args.n, args.log.clone())
    };
    for i in 0..5 {
        let id = coroutine_running(s);
        log.lock().unwrap().push((id, start + i));
        coroutine_yield(s);
    }
}

struct StatusProbe {
    log: Arc<Mutex<Vec<i32>>>,
    self_id: i32,
}

fn probe_running(s: &mut Schedule, data: &mut dyn Any) {
    // record `running` at multiple points.
    let (log, self_id) = {
        let p = data.downcast_ref::<StatusProbe>().unwrap();
        (p.log.clone(), p.self_id)
    };
    log.lock().unwrap().push(coroutine_running(s));
    coroutine_yield(s);
    log.lock().unwrap().push(coroutine_running(s));
    coroutine_yield(s);
    // Also record what we believe our own id is, indirectly.
    log.lock().unwrap().push(self_id);
}

struct StatusObserver {
    log: Arc<Mutex<Vec<i32>>>,
}

fn observe_self_status(s: &mut Schedule, data: &mut dyn Any) {
    // Record our own status as observed by coroutine_status while running,
    // then yield, then on resume record again.
    let (log, my_id) = {
        let p = data.downcast_ref::<StatusObserver>().unwrap();
        (p.log.clone(), coroutine_running(s))
    };
    log.lock().unwrap().push(coroutine_status(s, my_id));
    coroutine_yield(s);
    log.lock().unwrap().push(coroutine_status(s, my_id));
}

fn no_yield(s: &mut Schedule, data: &mut dyn Any) {
    // Just record the running id and exit.
    let log = {
        let l = data.downcast_ref::<Arc<Mutex<Vec<i32>>>>().unwrap();
        l.clone()
    };
    log.lock().unwrap().push(coroutine_running(s));
}

fn yields_three_times(s: &mut Schedule, data: &mut dyn Any) {
    let log = {
        let l = data.downcast_ref::<Arc<Mutex<Vec<i32>>>>().unwrap();
        l.clone()
    };
    for i in 0..3 {
        log.lock().unwrap().push(i);
        coroutine_yield(s);
    }
    log.lock().unwrap().push(99);
}

// ---------- constants ----------

#[test]
fn test_constants_match_c() {
    // Ground truth from c_src/src/coroutine.h and coroutine.c.
    assert_eq!(COROUTINE_DEAD, 0);
    assert_eq!(COROUTINE_READY, 1);
    assert_eq!(COROUTINE_RUNNING, 2);
    assert_eq!(COROUTINE_SUSPEND, 3);
    assert_eq!(STACK_SIZE, 1024 * 1024);
    assert_eq!(DEFAULT_COROUTINE, 16);
}

// ---------- coroutine_open / initial state ----------

#[test]
fn test_open_initial_state() {
    let s = coroutine_open();
    assert_eq!(s.nco, 0);
    assert_eq!(s.cap, 16);
    assert_eq!(s.running, -1);
    assert_eq!(s.co.len(), 16);
    for slot in s.co.iter() {
        assert!(slot.is_none());
    }
    assert_eq!(s.stack.len(), STACK_SIZE);
    coroutine_close(s);
}

#[test]
fn test_running_initial_is_minus_one() {
    let s = coroutine_open();
    assert_eq!(coroutine_running(&s), -1);
    coroutine_close(s);
}

#[test]
fn test_status_dead_for_all_empty_slots() {
    let s = coroutine_open();
    for id in 0..16 {
        assert_eq!(coroutine_status(&s, id), COROUTINE_DEAD);
        assert_eq!(coroutine_status(&s, id), 0);
    }
    coroutine_close(s);
}

// ---------- coroutine_new ----------

#[test]
fn test_new_first_returns_zero_and_status_ready() {
    let mut s = coroutine_open();
    let log: Arc<Mutex<Vec<i32>>> = Arc::new(Mutex::new(Vec::new()));
    let id = coroutine_new(&mut s, no_yield, Box::new(log.clone()));
    assert_eq!(id, 0);
    assert_eq!(coroutine_status(&s, 0), COROUTINE_READY);
    assert_eq!(coroutine_status(&s, 0), 1);
    assert_eq!(s.nco, 1);
    assert_eq!(s.cap, 16);
    assert_eq!(s.running, -1);
    coroutine_close(s);
}

#[test]
fn test_new_multiple_sequential_ids() {
    let mut s = coroutine_open();
    let log: Arc<Mutex<Vec<i32>>> = Arc::new(Mutex::new(Vec::new()));
    for expected in 0..5 {
        let id = coroutine_new(&mut s, no_yield, Box::new(log.clone()));
        assert_eq!(id, expected);
        assert_eq!(coroutine_status(&s, id), COROUTINE_READY);
    }
    assert_eq!(s.nco, 5);
    assert_eq!(s.cap, 16);
    coroutine_close(s);
}

#[test]
fn test_new_grows_capacity_when_full() {
    // The C code grows when nco >= cap. With cap=16 this means after 16
    // coroutines the next one triggers realloc to cap*2=32 and returns id=16.
    let mut s = coroutine_open();
    let log: Arc<Mutex<Vec<i32>>> = Arc::new(Mutex::new(Vec::new()));
    for expected in 0..16 {
        let id = coroutine_new(&mut s, no_yield, Box::new(log.clone()));
        assert_eq!(id, expected);
    }
    assert_eq!(s.cap, 16);
    assert_eq!(s.nco, 16);

    let id17 = coroutine_new(&mut s, no_yield, Box::new(log.clone()));
    assert_eq!(id17, 16);
    assert_eq!(s.cap, 32);
    assert_eq!(s.nco, 17);
    assert_eq!(s.co.len(), 32);

    let id18 = coroutine_new(&mut s, no_yield, Box::new(log.clone()));
    // Now nco=17 < cap=32, search starts at (0+17) % 32 = 17 which is empty.
    assert_eq!(id18, 17);
    assert_eq!(s.cap, 32);
    assert_eq!(s.nco, 18);

    coroutine_close(s);
}

// ---------- main behavioral test (mirrors c_src/tests/test.c) ----------

#[test]
fn test_two_coroutines_alternating_matches_c_test() {
    // Mirrors c_src/tests/test.c. The C executable prints:
    //   coroutine 0 : 0
    //   coroutine 1 : 100
    //   coroutine 0 : 1
    //   coroutine 1 : 101
    //   ...
    //   coroutine 0 : 4
    //   coroutine 1 : 104
    let log: Arc<Mutex<Vec<(i32, i32)>>> = Arc::new(Mutex::new(Vec::new()));
    let mut s = coroutine_open();

    let arg1 = Box::new(Args { n: 0, log: log.clone() });
    let arg2 = Box::new(Args { n: 100, log: log.clone() });

    let co1 = coroutine_new(&mut s, foo, arg1);
    let co2 = coroutine_new(&mut s, foo, arg2);

    assert_eq!(co1, 0);
    assert_eq!(co2, 1);

    while coroutine_status(&s, co1) != COROUTINE_DEAD
        && coroutine_status(&s, co2) != COROUTINE_DEAD
    {
        coroutine_resume(&mut s, co1);
        coroutine_resume(&mut s, co2);
    }

    coroutine_close(s);

    let expected: Vec<(i32, i32)> = vec![
        (0, 0), (1, 100),
        (0, 1), (1, 101),
        (0, 2), (1, 102),
        (0, 3), (1, 103),
        (0, 4), (1, 104),
    ];
    let log_data = log.lock().unwrap();
    assert_eq!(log_data.len(), 10);
    assert_eq!(*log_data, expected);
}

// ---------- coroutine_resume / coroutine_yield / status transitions ----------

#[test]
fn test_resume_dead_slot_is_noop() {
    // After a coroutine completes its slot becomes None (DEAD). Resuming a
    // None slot should not panic and should not change `running`.
    let mut s = coroutine_open();
    let log: Arc<Mutex<Vec<i32>>> = Arc::new(Mutex::new(Vec::new()));
    let id = coroutine_new(&mut s, no_yield, Box::new(log.clone()));
    assert_eq!(id, 0);
    coroutine_resume(&mut s, id);
    assert_eq!(coroutine_status(&s, id), COROUTINE_DEAD);
    assert_eq!(coroutine_running(&s), -1);

    // Now resume the dead slot — should be a no-op.
    coroutine_resume(&mut s, id);
    assert_eq!(coroutine_status(&s, id), COROUTINE_DEAD);
    assert_eq!(coroutine_running(&s), -1);

    // The function ran exactly once.
    let recorded = log.lock().unwrap();
    assert_eq!(*recorded, vec![0]);
    drop(recorded);
    coroutine_close(s);
}

#[test]
fn test_status_transitions_during_yield_resume() {
    let mut s = coroutine_open();
    let log: Arc<Mutex<Vec<i32>>> = Arc::new(Mutex::new(Vec::new()));
    let id = coroutine_new(&mut s, yields_three_times, Box::new(log.clone()));
    assert_eq!(id, 0);

    // Initially READY.
    assert_eq!(coroutine_status(&s, id), COROUTINE_READY);
    assert_eq!(coroutine_running(&s), -1);

    // First resume -> runs until first yield -> SUSPEND.
    coroutine_resume(&mut s, id);
    assert_eq!(coroutine_status(&s, id), COROUTINE_SUSPEND);
    assert_eq!(coroutine_running(&s), -1);
    assert_eq!(*log.lock().unwrap(), vec![0]);

    // Second resume -> runs until next yield.
    coroutine_resume(&mut s, id);
    assert_eq!(coroutine_status(&s, id), COROUTINE_SUSPEND);
    assert_eq!(coroutine_running(&s), -1);
    assert_eq!(*log.lock().unwrap(), vec![0, 1]);

    // Third resume.
    coroutine_resume(&mut s, id);
    assert_eq!(coroutine_status(&s, id), COROUTINE_SUSPEND);
    assert_eq!(*log.lock().unwrap(), vec![0, 1, 2]);

    // Fourth resume -> runs to completion -> DEAD.
    coroutine_resume(&mut s, id);
    assert_eq!(coroutine_status(&s, id), COROUTINE_DEAD);
    assert_eq!(coroutine_running(&s), -1);
    assert_eq!(*log.lock().unwrap(), vec![0, 1, 2, 99]);

    coroutine_close(s);
}

#[test]
fn test_running_id_visible_to_coroutine() {
    let log: Arc<Mutex<Vec<i32>>> = Arc::new(Mutex::new(Vec::new()));
    let mut s = coroutine_open();
    let id_a = coroutine_new(
        &mut s,
        probe_running,
        Box::new(StatusProbe {
            log: log.clone(),
            self_id: -42,
        }),
    );
    let id_b = coroutine_new(
        &mut s,
        probe_running,
        Box::new(StatusProbe {
            log: log.clone(),
            self_id: -99,
        }),
    );

    assert_eq!(id_a, 0);
    assert_eq!(id_b, 1);

    coroutine_resume(&mut s, id_a); // pushes 0 (running id), yields
    coroutine_resume(&mut s, id_b); // pushes 1, yields
    coroutine_resume(&mut s, id_a); // pushes 0, yields
    coroutine_resume(&mut s, id_b); // pushes 1, yields
    coroutine_resume(&mut s, id_a); // pushes -42 (self_id), exits
    coroutine_resume(&mut s, id_b); // pushes -99, exits

    let recorded = log.lock().unwrap();
    assert_eq!(*recorded, vec![0, 1, 0, 1, -42, -99]);
    drop(recorded);

    assert_eq!(coroutine_status(&s, id_a), COROUTINE_DEAD);
    assert_eq!(coroutine_status(&s, id_b), COROUTINE_DEAD);
    assert_eq!(coroutine_running(&s), -1);
    coroutine_close(s);
}

#[test]
fn test_self_status_is_running_then_suspend_then_resumes() {
    // While running, coroutine_status(my_id) == RUNNING.
    // After yield+resume, the status the coroutine sees of itself when it
    // re-runs is RUNNING again (the resume code sets it back).
    let mut s = coroutine_open();
    let log: Arc<Mutex<Vec<i32>>> = Arc::new(Mutex::new(Vec::new()));
    let id = coroutine_new(
        &mut s,
        observe_self_status,
        Box::new(StatusObserver { log: log.clone() }),
    );
    assert_eq!(id, 0);

    coroutine_resume(&mut s, id);
    // Coroutine ran, observed RUNNING, yielded.
    assert_eq!(coroutine_status(&s, id), COROUTINE_SUSPEND);
    coroutine_resume(&mut s, id);
    // Coroutine resumed, observed RUNNING again, returned -> DEAD.
    assert_eq!(coroutine_status(&s, id), COROUTINE_DEAD);

    let recorded = log.lock().unwrap();
    assert_eq!(*recorded, vec![COROUTINE_RUNNING, COROUTINE_RUNNING]);
    drop(recorded);
    coroutine_close(s);
}

#[test]
fn test_close_with_live_coroutines() {
    // coroutine_close should clean up everything even if coroutines have
    // been suspended (and never resumed to completion).
    let mut s = coroutine_open();
    let log: Arc<Mutex<Vec<i32>>> = Arc::new(Mutex::new(Vec::new()));
    let id1 = coroutine_new(&mut s, yields_three_times, Box::new(log.clone()));
    let id2 = coroutine_new(&mut s, yields_three_times, Box::new(log.clone()));

    coroutine_resume(&mut s, id1);
    coroutine_resume(&mut s, id2);
    assert_eq!(coroutine_status(&s, id1), COROUTINE_SUSPEND);
    assert_eq!(coroutine_status(&s, id2), COROUTINE_SUSPEND);

    // Close while suspended; should not hang or panic.
    coroutine_close(s);
}

#[test]
fn test_capacity_growth_first_id_after_growth() {
    // Same ground truth as test_new_grows_capacity_when_full but isolated:
    // when the table is full and we grow, the returned id is the OLD cap.
    let mut s = coroutine_open();
    let log: Arc<Mutex<Vec<i32>>> = Arc::new(Mutex::new(Vec::new()));
    let mut last = -1;
    for _ in 0..16 {
        last = coroutine_new(&mut s, no_yield, Box::new(log.clone()));
    }
    assert_eq!(last, 15);
    assert_eq!(s.cap, 16);
    let grown = coroutine_new(&mut s, no_yield, Box::new(log.clone()));
    assert_eq!(grown, 16);
    assert_eq!(s.cap, 32);
    coroutine_close(s);
}

fn main() {}
