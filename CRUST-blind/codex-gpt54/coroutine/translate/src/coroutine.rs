use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::ptr;

use corosensei::{Coroutine as StackCoroutine, CoroutineResult, Yielder};
pub const COROUTINE_DEAD: i32 = 0;
pub const COROUTINE_READY: i32 = 1;
pub const COROUTINE_RUNNING: i32 = 2;
pub const COROUTINE_SUSPEND: i32 = 3;
pub const STACK_SIZE: usize = 1024 * 1024;
pub const DEFAULT_COROUTINE: usize = 16;
pub type CoroutineFunc = fn(schedule: &mut Schedule, data: &mut dyn Any);
pub struct Coroutine {
    pub func: CoroutineFunc,
    pub data: Box<dyn Any>,
    pub cap: isize,
    pub size: isize,
    pub status: i32,
    pub stack: Option<Box<[u8]>>,
}
pub struct Schedule {
    pub stack: Box<[u8]>,
    pub nco: usize,
    pub cap: usize,
    pub running: i32,
    pub co: Vec<Option<Box<Coroutine>>>,
}

thread_local! {
    static RUNTIMES: RefCell<HashMap<usize, StackCoroutine<*mut Schedule, (), ()>>> =
        RefCell::new(HashMap::new());
    static CURRENT_YIELDER: Cell<*const Yielder<*mut Schedule, ()>> = Cell::new(ptr::null());
}

fn coroutine_key(coroutine: &Coroutine) -> usize {
    coroutine as *const Coroutine as usize
}

fn insert_runtime(key: usize, runtime: StackCoroutine<*mut Schedule, (), ()>) {
    RUNTIMES.with(|runtimes| {
        let previous = runtimes.borrow_mut().insert(key, runtime);
        assert!(previous.is_none());
    });
}

fn remove_runtime(key: usize) -> Option<StackCoroutine<*mut Schedule, (), ()>> {
    RUNTIMES.with(|runtimes| runtimes.borrow_mut().remove(&key))
}

fn make_runtime(func: CoroutineFunc, data: Box<dyn Any>) -> StackCoroutine<*mut Schedule, (), ()> {
    let mut data = data;
    StackCoroutine::new(move |yielder, schedule_ptr: *mut Schedule| {
        CURRENT_YIELDER.with(|current| current.set(yielder as *const _));
        let schedule = unsafe {
            schedule_ptr
                .as_mut()
                .expect("coroutine runtime received a null schedule pointer")
        };
        func(schedule, data.as_mut());
        CURRENT_YIELDER.with(|current| current.set(ptr::null()));
    })
}

fn cleanup_finished(schedule: &mut Schedule, id: usize) {
    if schedule.co[id].take().is_some() {
        schedule.nco -= 1;
    }
    schedule.running = -1;
}

pub fn coroutine_open() -> Box<Schedule> {
    Box::new(Schedule {
        stack: vec![0; STACK_SIZE].into_boxed_slice(),
        nco: 0,
        cap: DEFAULT_COROUTINE,
        running: -1,
        co: std::iter::repeat_with(|| None)
            .take(DEFAULT_COROUTINE)
            .collect(),
    })
}
pub fn coroutine_close(schedule: Box<Schedule>) {
    for coroutine in schedule.co.iter().flatten() {
        remove_runtime(coroutine_key(coroutine));
    }
}
pub fn coroutine_new(schedule: &mut Schedule, func: CoroutineFunc, data: Box<dyn Any>) -> i32 {
    let coroutine = Box::new(Coroutine {
        func,
        data: Box::new(()),
        cap: 0,
        size: 0,
        status: COROUTINE_READY,
        stack: None,
    });
    let key = coroutine_key(&coroutine);
    insert_runtime(key, make_runtime(func, data));

    if schedule.nco >= schedule.cap {
        let id = schedule.cap;
        schedule.co.resize_with(schedule.cap * 2, || None);
        schedule.co[id] = Some(coroutine);
        schedule.cap *= 2;
        schedule.nco += 1;
        return id as i32;
    }

    for i in 0..schedule.cap {
        let id = (i + schedule.nco) % schedule.cap;
        if schedule.co[id].is_none() {
            schedule.co[id] = Some(coroutine);
            schedule.nco += 1;
            return id as i32;
        }
    }

    unreachable!("coroutine slot allocation failed")
}
pub fn coroutine_resume(schedule: &mut Schedule, id: i32) {
    assert!(schedule.running == -1);
    assert!(id >= 0 && (id as usize) < schedule.cap);

    let id = id as usize;
    let Some(coroutine) = schedule.co[id].as_mut() else {
        return;
    };

    match coroutine.status {
        COROUTINE_READY | COROUTINE_SUSPEND => {}
        _ => unreachable!("invalid coroutine status"),
    }

    coroutine.status = COROUTINE_RUNNING;
    schedule.running = id as i32;

    let key = coroutine_key(coroutine);
    let mut runtime = remove_runtime(key).expect("missing runtime for coroutine");
    let result = catch_unwind(AssertUnwindSafe(|| runtime.resume(schedule as *mut Schedule)));

    match result {
        Ok(CoroutineResult::Yield(())) => {
            insert_runtime(key, runtime);
        }
        Ok(CoroutineResult::Return(())) => {
            cleanup_finished(schedule, id);
        }
        Err(payload) => {
            cleanup_finished(schedule, id);
            resume_unwind(payload);
        }
    }
}
pub fn coroutine_yield(schedule: &mut Schedule) {
    let id = schedule.running;
    assert!(id >= 0);

    let coroutine = schedule.co[id as usize]
        .as_mut()
        .expect("running coroutine slot is empty");
    coroutine.status = COROUTINE_SUSPEND;
    schedule.running = -1;

    CURRENT_YIELDER.with(|current| {
        let yielder = current.get();
        assert!(!yielder.is_null());
        let next_schedule = unsafe { (&*yielder).suspend(()) };
        assert_eq!(next_schedule, schedule as *mut Schedule);
    });
}
pub fn coroutine_status(schedule: &Schedule, id: i32) -> i32 {
    assert!(id >= 0 && (id as usize) < schedule.cap);
    match &schedule.co[id as usize] {
        Some(coroutine) => coroutine.status,
        None => COROUTINE_DEAD,
    }
}
pub fn coroutine_running(schedule: &Schedule) -> i32 {
    schedule.running
}
