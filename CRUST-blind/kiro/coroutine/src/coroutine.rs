use std::any::Any;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

pub const COROUTINE_DEAD: i32 = 0;
pub const COROUTINE_READY: i32 = 1;
pub const COROUTINE_RUNNING: i32 = 2;
pub const COROUTINE_SUSPEND: i32 = 3;
pub const STACK_SIZE: usize = 1024 * 1024;
pub const DEFAULT_COROUTINE: usize = 16;
pub type CoroutineFunc = fn(schedule: &mut Schedule, data: &mut dyn Any);

struct CoSync {
    mu: Mutex<bool>, // true = coroutine's turn, false = main's turn
    cv: Condvar,
}

pub struct Coroutine {
    pub func: CoroutineFunc,
    pub data: Box<dyn Any>,
    pub cap: isize,
    pub size: isize,
    pub status: i32,
    pub stack: Option<Box<[u8]>>,
    sync: Arc<CoSync>,
    _handle: Option<thread::JoinHandle<()>>,
}

pub struct Schedule {
    pub stack: Box<[u8]>,
    pub nco: usize,
    pub cap: usize,
    pub running: i32,
    pub co: Vec<Option<Box<Coroutine>>>,
}

pub fn coroutine_open() -> Box<Schedule> {
    let mut co = Vec::with_capacity(DEFAULT_COROUTINE);
    co.resize_with(DEFAULT_COROUTINE, || None);
    Box::new(Schedule {
        stack: vec![0u8; STACK_SIZE].into_boxed_slice(),
        nco: 0,
        cap: DEFAULT_COROUTINE,
        running: -1,
        co,
    })
}

pub fn coroutine_close(schedule: Box<Schedule>) {
    drop(schedule);
}

pub fn coroutine_new(schedule: &mut Schedule, func: CoroutineFunc, data: Box<dyn Any>) -> i32 {
    let sync = Arc::new(CoSync {
        mu: Mutex::new(false),
        cv: Condvar::new(),
    });
    let co = Box::new(Coroutine {
        func,
        data,
        cap: 0,
        size: 0,
        status: COROUTINE_READY,
        stack: None,
        sync,
        _handle: None,
    });

    if schedule.nco >= schedule.cap {
        let id = schedule.cap;
        let old_cap = schedule.cap;
        schedule.co.resize_with(old_cap * 2, || None);
        schedule.co[old_cap] = Some(co);
        schedule.cap *= 2;
        schedule.nco += 1;
        return id as i32;
    } else {
        for i in 0..schedule.cap {
            let id = (i + schedule.nco) % schedule.cap;
            if schedule.co[id].is_none() {
                schedule.co[id] = Some(co);
                schedule.nco += 1;
                return id as i32;
            }
        }
    }
    panic!("no free coroutine slot");
}

fn signal_and_wait(sync: &CoSync, set_to: bool) {
    let mut turn = sync.mu.lock().unwrap();
    *turn = set_to;
    sync.cv.notify_one();
    let target = !set_to;
    while *turn != target {
        turn = sync.cv.wait(turn).unwrap();
    }
}

pub fn coroutine_resume(schedule: &mut Schedule, id: i32) {
    assert!(schedule.running == -1);
    assert!(id >= 0 && (id as usize) < schedule.cap);
    let uid = id as usize;

    let status = match &schedule.co[uid] {
        Some(co) => co.status,
        None => return,
    };

    match status {
        COROUTINE_READY => {
            let mut co = schedule.co[uid].take().unwrap();
            co.status = COROUTINE_RUNNING;
            let func = co.func;
            let sync = co.sync.clone();
            schedule.co[uid] = Some(co);
            schedule.running = id;

            // Encode pointer as usize so it's Send
            let sched_addr = schedule as *mut Schedule as usize;
            let sync2 = sync.clone();

            let handle = thread::spawn(move || {
                // Wait for the signal to start
                {
                    let mut turn = sync2.mu.lock().unwrap();
                    while !*turn {
                        turn = sync2.cv.wait(turn).unwrap();
                    }
                }

                let s = sched_addr as *mut Schedule;
                // Get data pointer to avoid aliasing &mut Schedule and &mut data
                let data_ptr: *mut dyn Any = unsafe {
                    &mut *(&mut (*s).co)[id as usize].as_mut().unwrap().data
                };
                unsafe { func(&mut *s, &mut *data_ptr) };

                // Coroutine finished
                unsafe {
                    (&mut (*s).co)[id as usize].take();
                    (*s).nco -= 1;
                    (*s).running = -1;
                }

                let mut turn = sync2.mu.lock().unwrap();
                *turn = false;
                sync2.cv.notify_one();
            });

            schedule.co[uid].as_mut().unwrap()._handle = Some(handle);

            // Signal coroutine to start, wait for it to yield or finish
            signal_and_wait(&sync, true);
        }
        COROUTINE_SUSPEND => {
            schedule.co[uid].as_mut().unwrap().status = COROUTINE_RUNNING;
            schedule.running = id;
            let sync = schedule.co[uid].as_ref().unwrap().sync.clone();
            signal_and_wait(&sync, true);
        }
        _ => panic!("invalid coroutine status for resume"),
    }
}

pub fn coroutine_yield(schedule: &mut Schedule) {
    let id = schedule.running;
    assert!(id >= 0);
    let uid = id as usize;
    schedule.co[uid].as_mut().unwrap().status = COROUTINE_SUSPEND;
    schedule.running = -1;
    let sync = schedule.co[uid].as_ref().unwrap().sync.clone();
    // Signal main, wait for resume
    signal_and_wait(&sync, false);
}

pub fn coroutine_status(schedule: &Schedule, id: i32) -> i32 {
    assert!(id >= 0 && (id as usize) < schedule.cap);
    match &schedule.co[id as usize] {
        None => COROUTINE_DEAD,
        Some(co) => co.status,
    }
}

pub fn coroutine_running(schedule: &Schedule) -> i32 {
    schedule.running
}
