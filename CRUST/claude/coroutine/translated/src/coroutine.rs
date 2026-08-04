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

// ---------------------------------------------------------------------------
// Internal: thread-based coroutine implementation
// ---------------------------------------------------------------------------
//
// Because Rust does not provide a safe way to switch stacks (no `ucontext`
// equivalent in the standard library and no FFI allowed), we model each
// coroutine as an OS thread.  Cooperative scheduling is enforced through a
// per-coroutine mutex/condvar pair: at any moment either the "main" caller
// thread runs (between coroutine_resume / coroutine_yield) or exactly one
// coroutine thread runs.  The two never execute schedule-touching code
// concurrently.
//
// We must not modify the public `Schedule` / `Coroutine` definitions, so the
// per-coroutine synchronisation state is kept in a process-global side table
// keyed by (schedule address, coroutine id).

#[derive(PartialEq, Eq, Clone, Copy)]
enum CoState {
    Pending, // thread spawned but resume signal not yet sent
    Running, // coroutine is executing user code
    Yielded, // coroutine called coroutine_yield
    Dead,    // coroutine function returned
}

struct CoSync {
    state: Mutex<CoState>,
    cv: Condvar,
}

struct CoThread {
    sync: Arc<CoSync>,
    handle: Option<thread::JoinHandle<()>>,
}

fn thread_map() -> &'static Mutex<HashMap<(usize, i32), CoThread>> {
    static M: OnceLock<Mutex<HashMap<(usize, i32), CoThread>>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(HashMap::new()))
}

// Wrapper around a raw pointer so it can be sent across threads.  Access is
// always synchronised via the (CoSync) condvar protocol described above, so
// the data race semantics are equivalent to the C ucontext implementation.
#[derive(Clone, Copy)]
struct SchedPtr(*mut Schedule);
unsafe impl Send for SchedPtr {}
unsafe impl Sync for SchedPtr {}

fn signal(sync: &CoSync, new_state: CoState) {
    let mut state = sync.state.lock().unwrap();
    *state = new_state;
    sync.cv.notify_all();
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

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
    let sched_addr = (&*schedule) as *const Schedule as usize;

    // Tear down any coroutine threads still associated with this schedule.
    let mut handles_to_join: Vec<thread::JoinHandle<()>> = Vec::new();
    {
        let mut map = thread_map().lock().unwrap();
        let keys: Vec<(usize, i32)> = map
            .keys()
            .filter(|(p, _)| *p == sched_addr)
            .cloned()
            .collect();
        for k in keys {
            if let Some(mut t) = map.remove(&k) {
                if let Some(h) = t.handle.take() {
                    handles_to_join.push(h);
                }
            }
        }
    }
    for h in handles_to_join {
        let _ = h.join();
    }

    // schedule (Box) is dropped here, which in turn drops each remaining
    // Coroutine and its data.
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
        let id = schedule.cap as i32;
        let new_cap = schedule.cap * 2;
        schedule.co.resize_with(new_cap, || None);
        schedule.co[schedule.cap] = Some(co);
        schedule.cap = new_cap;
        schedule.nco += 1;
        id
    } else {
        for i in 0..schedule.cap {
            let id = (i + schedule.nco) % schedule.cap;
            if schedule.co[id].is_none() {
                schedule.co[id] = Some(co);
                schedule.nco += 1;
                return id as i32;
            }
        }
        unreachable!("coroutine_new: schedule has no free slot but nco < cap");
    }
}

pub fn coroutine_resume(schedule: &mut Schedule, id: i32) {
    assert!(schedule.running == -1);
    assert!(id >= 0 && (id as usize) < schedule.cap);

    let status = match schedule.co[id as usize].as_ref() {
        None => return,
        Some(c) => c.status,
    };

    let sched_ptr: *mut Schedule = schedule as *mut Schedule;
    let sched_addr = sched_ptr as usize;

    match status {
        COROUTINE_READY => {
            let sync = Arc::new(CoSync {
                state: Mutex::new(CoState::Pending),
                cv: Condvar::new(),
            });
            let sync_thread = Arc::clone(&sync);
            let sp = SchedPtr(sched_ptr);

            let handle = thread::spawn(move || {
                let sp = sp;
                // Wait for the first "resume" signal from main.
                {
                    let mut state = sync_thread.state.lock().unwrap();
                    while *state == CoState::Pending {
                        state = sync_thread.cv.wait(state).unwrap();
                    }
                }

                // Move the user data out of the Schedule so we can hold a
                // mutable reference to the Schedule and to the data
                // simultaneously without aliasing.
                let mut taken_data: Box<dyn Any> = Box::new(());
                let func: CoroutineFunc;
                // SAFETY: at this point main is parked on the condvar; we
                // are the sole accessor of the Schedule.
                unsafe {
                    let s = &mut *sp.0;
                    let coro = s.co[id as usize].as_mut().expect("coroutine slot");
                    func = coro.func;
                    std::mem::swap(&mut coro.data, &mut taken_data);
                }

                // Run the user coroutine body.
                {
                    // SAFETY: same justification as above; main is parked
                    // until we yield or finish.
                    let s: &mut Schedule = unsafe { &mut *sp.0 };
                    let d: &mut dyn Any = taken_data.as_mut();
                    (func)(s, d);
                }

                // Coroutine function returned; mark it dead.
                // SAFETY: main is parked waiting for our final signal.
                unsafe {
                    let s = &mut *sp.0;
                    // Drop the (now-empty) Coroutine struct in the slot.
                    s.co[id as usize] = None;
                    s.nco -= 1;
                    s.running = -1;
                }

                signal(&sync_thread, CoState::Dead);
                // taken_data dropped here.
            });

            // Register the thread for this coroutine.
            thread_map().lock().unwrap().insert(
                (sched_addr, id),
                CoThread {
                    sync: Arc::clone(&sync),
                    handle: Some(handle),
                },
            );

            // Set up status before handing control over.
            schedule.running = id;
            schedule.co[id as usize].as_mut().unwrap().status = COROUTINE_RUNNING;

            // Wake the coroutine thread.
            signal(&sync, CoState::Running);

            // Wait until it yields or dies.
            {
                let mut state = sync.state.lock().unwrap();
                while *state == CoState::Running {
                    state = sync.cv.wait(state).unwrap();
                }
            }

            // Reap the thread if the coroutine finished.
            let is_dead = *sync.state.lock().unwrap() == CoState::Dead;
            if is_dead {
                let removed = thread_map().lock().unwrap().remove(&(sched_addr, id));
                if let Some(mut t) = removed {
                    if let Some(h) = t.handle.take() {
                        let _ = h.join();
                    }
                }
            }
        }
        COROUTINE_SUSPEND => {
            schedule.running = id;
            schedule.co[id as usize].as_mut().unwrap().status = COROUTINE_RUNNING;

            let sync = {
                let map = thread_map().lock().unwrap();
                Arc::clone(
                    &map.get(&(sched_addr, id))
                        .expect("missing coroutine thread")
                        .sync,
                )
            };

            signal(&sync, CoState::Running);

            {
                let mut state = sync.state.lock().unwrap();
                while *state == CoState::Running {
                    state = sync.cv.wait(state).unwrap();
                }
            }

            let is_dead = *sync.state.lock().unwrap() == CoState::Dead;
            if is_dead {
                let removed = thread_map().lock().unwrap().remove(&(sched_addr, id));
                if let Some(mut t) = removed {
                    if let Some(h) = t.handle.take() {
                        let _ = h.join();
                    }
                }
            }
        }
        _ => panic!("coroutine_resume: invalid status {}", status),
    }
}

pub fn coroutine_yield(schedule: &mut Schedule) {
    let id = schedule.running;
    assert!(id >= 0);
    schedule.co[id as usize].as_mut().unwrap().status = COROUTINE_SUSPEND;
    schedule.running = -1;

    let sched_addr = schedule as *mut Schedule as usize;
    let sync = {
        let map = thread_map().lock().unwrap();
        Arc::clone(
            &map.get(&(sched_addr, id))
                .expect("coroutine_yield without thread")
                .sync,
        )
    };

    // Tell the main thread we yielded, then block until resumed.
    signal(&sync, CoState::Yielded);

    {
        let mut state = sync.state.lock().unwrap();
        while *state == CoState::Yielded {
            state = sync.cv.wait(state).unwrap();
        }
    }
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
