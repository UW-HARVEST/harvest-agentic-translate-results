use std::any::Any;
use std::sync::{Arc, Mutex};

use coroutine::coroutine::{
    coroutine_close, coroutine_new, coroutine_open, coroutine_resume, coroutine_running,
    coroutine_status, coroutine_yield, Schedule, COROUTINE_DEAD,
};

struct Args {
    n: i32,
    log: Arc<Mutex<Vec<String>>>,
}

fn foo(s: &mut Schedule, ud: &mut dyn Any) {
    let arg = ud.downcast_mut::<Args>().expect("downcast Args");
    let start = arg.n;
    let log = arg.log.clone();
    for i in 0..5 {
        let id = coroutine_running(s);
        log.lock().unwrap().push(format!("coroutine {} : {}", id, start + i));
        coroutine_yield(s);
    }
}

#[test]
fn matches_c_test_output() {
    let mut s = coroutine_open();
    let log = Arc::new(Mutex::new(Vec::<String>::new()));
    let arg1: Box<dyn Any> = Box::new(Args { n: 0, log: log.clone() });
    let arg2: Box<dyn Any> = Box::new(Args { n: 100, log: log.clone() });

    let co1 = coroutine_new(&mut s, foo, arg1);
    let co2 = coroutine_new(&mut s, foo, arg2);
    log.lock().unwrap().push("main start".to_string());
    while coroutine_status(&s, co1) != COROUTINE_DEAD
        && coroutine_status(&s, co2) != COROUTINE_DEAD
    {
        coroutine_resume(&mut s, co1);
        coroutine_resume(&mut s, co2);
    }
    log.lock().unwrap().push("main end".to_string());

    coroutine_close(s);

    let lines = log.lock().unwrap().clone();
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
    assert_eq!(lines, expected);
}

#[test]
fn growth_when_capacity_exceeded() {
    let mut s = coroutine_open();
    // Fill more than DEFAULT_COROUTINE coroutines to exercise the resize path.
    let n = 20;
    let log = Arc::new(Mutex::new(Vec::<String>::new()));
    let mut ids = Vec::new();
    for i in 0..n {
        let arg: Box<dyn Any> = Box::new(Args { n: (i as i32) * 10, log: log.clone() });
        let id = coroutine_new(&mut s, foo, arg);
        ids.push(id);
    }
    // Run all coroutines to completion.
    let mut alive = ids.clone();
    while !alive.is_empty() {
        alive.retain(|id| {
            if coroutine_status(&s, *id) != COROUTINE_DEAD {
                coroutine_resume(&mut s, *id);
                coroutine_status(&s, *id) != COROUTINE_DEAD
            } else {
                false
            }
        });
    }
    coroutine_close(s);
    // Each coroutine yields 5 times, producing 5 lines.
    assert_eq!(log.lock().unwrap().len(), n * 5);
}
