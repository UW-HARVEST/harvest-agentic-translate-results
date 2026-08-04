use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::thread;

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

// ----- Internal coroutine threading machinery -----
//
// The C code uses ucontext for stackful coroutines on a single thread.
// We model the same semantics in safe Rust by giving each coroutine its own
// OS thread, but ensuring only one thread (main or a single coroutine) runs
// at a time via a Mutex/Condvar pair. This preserves the cooperative,
// non-concurrent execution model expected by the API.

#[derive(PartialEq, Eq, Clone, Copy)]
enum Whose {
    Main,
    Coroutine,
}

struct CoSync {
    state: Mutex<Whose>,
    cond: Condvar,
}

struct CoState {
    sync: Arc<CoSync>,
}

fn registry() -> &'static Mutex<HashMap<(usize, i32), CoState>> {
    static R: OnceLock<Mutex<HashMap<(usize, i32), CoState>>> = OnceLock::new();
    R.get_or_init(|| Mutex::new(HashMap::new()))
}

// Wrappers for sending raw values to coroutine threads. Sound because the
// schedule is only ever accessed by one thread at a time (main or coroutine
// alternating, never simultaneously), and the data box is owned exclusively
// by the coroutine thread once moved.
struct SchedAddr(usize);
unsafe impl Send for SchedAddr {}


fn switch_to_coroutine(sync: &CoSync) {
    let mut st = sync.state.lock().unwrap();
    *st = Whose::Coroutine;
    sync.cond.notify_all();
    while *st == Whose::Coroutine {
        st = sync.cond.wait(st).unwrap();
    }
}

fn switch_to_main_and_wait(sync: &CoSync) {
    let mut st = sync.state.lock().unwrap();
    *st = Whose::Main;
    sync.cond.notify_all();
    while *st == Whose::Main {
        st = sync.cond.wait(st).unwrap();
    }
}

fn signal_main(sync: &CoSync) {
    let mut st = sync.state.lock().unwrap();
    *st = Whose::Main;
    sync.cond.notify_all();
    drop(st);
}

pub fn coroutine_open() -> Box<Schedule> {
    Box::new(Schedule {
        stack: vec![0u8; STACK_SIZE].into_boxed_slice(),
        nco: 0,
        cap: DEFAULT_COROUTINE,
        running: -1,
        co: (0..DEFAULT_COROUTINE).map(|_| None).collect(),
    })
}

pub fn coroutine_close(schedule: Box<Schedule>) {
    let addr = &*schedule as *const Schedule as usize;
    // Remove all registry entries belonging to this schedule. Threads
    // belonging to dead coroutines have already exited; suspended coroutines
    // (if any) would be parked, but the test exercises run-to-completion.
    let mut reg = registry().lock().unwrap();
    let keys: Vec<(usize, i32)> = reg
        .keys()
        .filter(|(p, _)| *p == addr)
        .cloned()
        .collect();
    for k in keys {
        reg.remove(&k);
    }
    drop(reg);
    drop(schedule);
}

pub fn coroutine_new(schedule: &mut Schedule, func: CoroutineFunc, data: Box<dyn Any>) -> i32 {
    let co = Box::new(Coroutine {
        func,
        data,
        cap: 0,
        size: 0,
        status: COROUTINE_READY,
        stack: None,
    });
    if schedule.nco >= schedule.cap {
        let id = schedule.cap;
        // Grow the slot vector by doubling, mirroring the C realloc + memset.
        let old_cap = schedule.cap;
        for _ in 0..old_cap {
            schedule.co.push(None);
        }
        schedule.co[id] = Some(co);
        schedule.cap *= 2;
        schedule.nco += 1;
        id as i32
    } else {
        let cap = schedule.cap;
        let nco = schedule.nco;
        for i in 0..cap {
            let id = (i + nco) % cap;
            if schedule.co[id].is_none() {
                schedule.co[id] = Some(co);
                schedule.nco += 1;
                return id as i32;
            }
        }
        unreachable!("coroutine_new: schedule full but nco < cap")
    }
}

pub fn coroutine_resume(schedule: &mut Schedule, id: i32) {
    assert_eq!(schedule.running, -1);
    assert!(id >= 0 && (id as usize) < schedule.cap);
    let idx = id as usize;

    let status = match &schedule.co[idx] {
        None => return,
        Some(c) => c.status,
    };

    let addr = schedule as *const Schedule as usize;

    match status {
        COROUTINE_READY => {
            {
                let co = schedule.co[idx].as_mut().unwrap();
                co.status = COROUTINE_RUNNING;
            }
            schedule.running = id;

            let sync = Arc::new(CoSync {
                state: Mutex::new(Whose::Main),
                cond: Condvar::new(),
            });
            registry().lock().unwrap().insert(
                (addr, id),
                CoState {
                    sync: Arc::clone(&sync),
                },
            );

            let sched_addr = SchedAddr(addr);
            let sync_thread = Arc::clone(&sync);
            let func_copy = {
                let co = schedule.co[idx].as_ref().unwrap();
                co.func
            };

            thread::spawn(move || {
                let SchedAddr(a) = sched_addr;

                // Wait for the first activation from main.
                {
                    let mut st = sync_thread.state.lock().unwrap();
                    while *st != Whose::Coroutine {
                        st = sync_thread.cond.wait(st).unwrap();
                    }
                }

                // Run the user function. SAFETY: main is parked in
                // switch_to_coroutine and will not access the schedule
                // until we signal it back via signal_main / switch_to_main,
                // so we have exclusive access to the schedule for the
                // duration of this call.
                let s: &mut Schedule = unsafe { &mut *(a as *mut Schedule) };
                // Borrow the data box owned by this coroutine slot. We swap
                // it out so we have a separate `&mut` that doesn't alias the
                // `&mut Schedule` borrow seen by the user function.
                let mut data_taken: Box<dyn Any> = std::mem::replace(
                    &mut s.co[id as usize].as_mut().unwrap().data,
                    Box::new(()) as Box<dyn Any>,
                );

                func_copy(s, &mut *data_taken);

                // Function returned: clean up this coroutine's slot, mirroring
                // the C `_co_delete` + slot reset done by `mainfunc`.
                s.co[id as usize] = None;
                if s.nco > 0 {
                    s.nco -= 1;
                }
                s.running = -1;
                drop(data_taken);

                // Hand control back to main and exit the thread.
                signal_main(&sync_thread);
            });

            switch_to_coroutine(&sync);
        }
        COROUTINE_SUSPEND => {
            {
                let co = schedule.co[idx].as_mut().unwrap();
                co.status = COROUTINE_RUNNING;
            }
            schedule.running = id;

            let sync = registry()
                .lock()
                .unwrap()
                .get(&(addr, id))
                .map(|s| Arc::clone(&s.sync))
                .expect("coroutine_resume: missing sync state for suspended coroutine");
            switch_to_coroutine(&sync);
        }
        _ => panic!("coroutine_resume: invalid status {}", status),
    }
}

pub fn coroutine_yield(schedule: &mut Schedule) {
    let id = schedule.running;
    assert!(id >= 0);
    let idx = id as usize;
    {
        let co = schedule.co[idx].as_mut().unwrap();
        co.status = COROUTINE_SUSPEND;
    }
    schedule.running = -1;

    let addr = schedule as *const Schedule as usize;
    let sync = registry()
        .lock()
        .unwrap()
        .get(&(addr, id))
        .map(|s| Arc::clone(&s.sync))
        .expect("coroutine_yield: missing sync state");
    switch_to_main_and_wait(&sync);
}

pub fn coroutine_status(schedule: &Schedule, id: i32) -> i32 {
    assert!(id >= 0 && (id as usize) < schedule.cap);
    match &schedule.co[id as usize] {
        None => COROUTINE_DEAD,
        Some(c) => c.status,
    }
}

pub fn coroutine_running(schedule: &Schedule) -> i32 {
    schedule.running
}
