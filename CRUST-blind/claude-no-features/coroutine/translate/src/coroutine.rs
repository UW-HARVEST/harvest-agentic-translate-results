use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::thread::{self, JoinHandle};

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

// ----- Internal synchronization machinery -----
//
// Because we cannot modify the `Coroutine` / `Schedule` structs, we keep the
// per-coroutine synchronization state in a global side-table keyed by
// (schedule address, coroutine id).  Each coroutine runs on its own OS thread.
// `coroutine_resume` wakes that thread and waits for it either to yield or to
// finish; `coroutine_yield` does the reverse, signalling the main thread and
// parking until the next resume.

#[derive(PartialEq, Clone, Copy, Debug)]
enum SyncState {
    Running,   // main asked the coroutine to run / it should run
    Suspended, // coroutine yielded, waiting to be resumed
    Finished,  // coroutine completed
}

struct CoroSync {
    state: Mutex<SyncState>,
    cvar: Condvar,
}

struct CoroExtra {
    sync: Arc<CoroSync>,
    handle: Option<JoinHandle<()>>,
}

fn registry() -> &'static Mutex<HashMap<(usize, i32), CoroExtra>> {
    static REGISTRY: OnceLock<Mutex<HashMap<(usize, i32), CoroExtra>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

// A `*mut Schedule` wrapper that is `Send` so we can pass it into the worker
// thread.  Only one of {main, worker} ever uses it at a time, so concurrent
// access is impossible by construction.
struct SchedPtr(*mut Schedule);
unsafe impl Send for SchedPtr {}

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
    let sched_addr = &*schedule as *const Schedule as usize;
    // Collect all entries belonging to this schedule and drop them.  For a
    // finished coroutine, this also detaches its (already-exited) thread.
    // For an unfinished coroutine, the worker thread is parked on the
    // condvar; dropping the JoinHandle simply detaches it.  The thread will
    // be reaped when the process exits.
    let mut reg = registry().lock().unwrap();
    let keys: Vec<(usize, i32)> = reg
        .keys()
        .filter(|(addr, _)| *addr == sched_addr)
        .copied()
        .collect();
    let mut to_join: Vec<JoinHandle<()>> = Vec::new();
    for k in keys {
        if let Some(extra) = reg.remove(&k) {
            // If the coroutine has finished we can join cheaply; otherwise
            // we just detach to avoid deadlocking on a parked worker.
            let is_finished = {
                let st = extra.sync.state.lock().unwrap();
                *st == SyncState::Finished
            };
            if is_finished {
                if let Some(h) = extra.handle {
                    to_join.push(h);
                }
            }
            // else: handle is dropped (detached)
        }
    }
    drop(reg);
    for h in to_join {
        let _ = h.join();
    }
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
        let new_cap = schedule.cap * 2;
        while schedule.co.len() < new_cap {
            schedule.co.push(None);
        }
        schedule.co[schedule.cap] = Some(co);
        schedule.cap = new_cap;
        schedule.nco += 1;
        id as i32
    } else {
        for i in 0..schedule.cap {
            let id = (i + schedule.nco) % schedule.cap;
            if schedule.co[id].is_none() {
                schedule.co[id] = Some(co);
                schedule.nco += 1;
                return id as i32;
            }
        }
        // The C version asserts(0) and returns -1 here; this branch is
        // unreachable because we already verified nco < cap above.
        unreachable!("free slot must exist when nco < cap");
    }
}

pub fn coroutine_resume(schedule: &mut Schedule, id: i32) {
    assert_eq!(schedule.running, -1);
    assert!(id >= 0 && (id as usize) < schedule.cap);

    if schedule.co[id as usize].is_none() {
        return;
    }

    let status = schedule.co[id as usize].as_ref().unwrap().status;
    let sched_addr = schedule as *const Schedule as usize;
    let key = (sched_addr, id);

    match status {
        COROUTINE_READY => {
            // Reusing a slot whose previous occupant finished: clean up the
            // old registry entry (its worker thread has already exited).
            let old = registry().lock().unwrap().remove(&key);
            if let Some(extra) = old {
                if let Some(h) = extra.handle {
                    let _ = h.join();
                }
            }

            // Mutate the schedule before spawning the worker.
            schedule.co[id as usize].as_mut().unwrap().status = COROUTINE_RUNNING;
            schedule.running = id;

            let sync = Arc::new(CoroSync {
                state: Mutex::new(SyncState::Running),
                cvar: Condvar::new(),
            });

            // Insert the registry entry (without handle yet) so that if the
            // worker calls coroutine_yield before we set the handle, the
            // lookup still succeeds.
            registry().lock().unwrap().insert(
                key,
                CoroExtra {
                    sync: sync.clone(),
                    handle: None,
                },
            );

            let sync_for_thread = sync.clone();
            let sched_ptr = SchedPtr(schedule as *mut Schedule);
            let coro_id = id;

            let handle = thread::spawn(move || {
                let sched_ptr = sched_ptr;
                let sync = sync_for_thread;

                // Extract the function pointer and a raw pointer to the user
                // data.  We do this in its own scope so that the temporary
                // `&mut Schedule` borrow ends before we invoke `func`.
                let (func, data_ptr): (CoroutineFunc, *mut dyn Any) = unsafe {
                    let s = &mut *sched_ptr.0;
                    let co = s.co[coro_id as usize].as_mut().unwrap();
                    let dp: *mut dyn Any = &mut *co.data;
                    (co.func, dp)
                };

                // Run the user's coroutine body.
                unsafe {
                    let s = &mut *sched_ptr.0;
                    func(s, &mut *data_ptr);
                }

                // The function has returned: drop the coroutine slot and
                // mark ourselves finished.  This mirrors the C version's
                // post-`func` cleanup in `mainfunc`.
                unsafe {
                    let s = &mut *sched_ptr.0;
                    s.co[coro_id as usize] = None;
                    s.nco = s.nco.saturating_sub(1);
                    s.running = -1;
                }

                let mut state = sync.state.lock().unwrap();
                *state = SyncState::Finished;
                sync.cvar.notify_all();
            });

            // Now record the JoinHandle.
            if let Some(entry) = registry().lock().unwrap().get_mut(&key) {
                entry.handle = Some(handle);
            }

            // Wait for the coroutine to either yield or finish.
            let mut state = sync.state.lock().unwrap();
            while *state == SyncState::Running {
                state = sync.cvar.wait(state).unwrap();
            }
        }
        COROUTINE_SUSPEND => {
            schedule.co[id as usize].as_mut().unwrap().status = COROUTINE_RUNNING;
            schedule.running = id;

            let sync = registry()
                .lock()
                .unwrap()
                .get(&key)
                .expect("registry entry for suspended coroutine must exist")
                .sync
                .clone();

            // Tell the worker thread to run again.
            {
                let mut state = sync.state.lock().unwrap();
                *state = SyncState::Running;
                sync.cvar.notify_all();
            }

            // Wait for it to yield or finish.
            let mut state = sync.state.lock().unwrap();
            while *state == SyncState::Running {
                state = sync.cvar.wait(state).unwrap();
            }
        }
        _ => panic!("coroutine_resume: invalid coroutine status {}", status),
    }
}

pub fn coroutine_yield(schedule: &mut Schedule) {
    let id = schedule.running;
    assert!(id >= 0);

    let sched_addr = schedule as *const Schedule as usize;
    let key = (sched_addr, id);

    schedule.co[id as usize].as_mut().unwrap().status = COROUTINE_SUSPEND;
    schedule.running = -1;

    let sync = registry()
        .lock()
        .unwrap()
        .get(&key)
        .expect("registry entry for running coroutine must exist")
        .sync
        .clone();

    // Tell the main thread we have yielded.
    {
        let mut state = sync.state.lock().unwrap();
        *state = SyncState::Suspended;
        sync.cvar.notify_all();
    }

    // Park until the next resume.
    let mut state = sync.state.lock().unwrap();
    while *state == SyncState::Suspended {
        state = sync.cvar.wait(state).unwrap();
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
