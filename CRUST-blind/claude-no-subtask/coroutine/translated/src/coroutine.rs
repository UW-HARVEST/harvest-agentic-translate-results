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

// ----------------------------------------------------------------------------
// Internal coroutine threading machinery.
//
// The original C implementation uses `ucontext_t` to perform stack switching
// between the scheduler and each coroutine.  Pure-Rust does not expose
// `setcontext`/`swapcontext`, so we emulate cooperative scheduling using one
// OS thread per coroutine.  At any point in time, either the calling
// (scheduler) thread or exactly one coroutine thread is making forward
// progress; the other is parked on a condition variable.  This preserves the
// strict mutual-exclusion guarantees needed to safely share a
// `&mut Schedule` between the two via raw pointers.
// ----------------------------------------------------------------------------

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum Signal {
    Idle,
    Run,
    Yielded,
    Done,
}

struct Worker {
    handle: Option<JoinHandle<()>>,
    sync: Arc<(Mutex<Signal>, Condvar)>,
}

// Wrapper to send raw pointers across threads.  The pointers are only ever
// dereferenced while the originating thread is parked, so this is sound.
struct SchedSendPtr(*mut Schedule);
unsafe impl Send for SchedSendPtr {}
impl SchedSendPtr {
    fn get(&self) -> *mut Schedule {
        self.0
    }
}
struct DataSendPtr(*mut dyn Any);
unsafe impl Send for DataSendPtr {}
impl DataSendPtr {
    fn get(&self) -> *mut dyn Any {
        self.0
    }
}

fn workers() -> &'static Mutex<HashMap<usize, Worker>> {
    static W: OnceLock<Mutex<HashMap<usize, Worker>>> = OnceLock::new();
    W.get_or_init(|| Mutex::new(HashMap::new()))
}

fn coroutine_key(co: &Coroutine) -> usize {
    co as *const Coroutine as usize
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
    // Remove any worker entries belonging to coroutines this schedule still
    // owns.  Their threads (if any) will be parked forever waiting for a
    // signal that never comes; this is acceptable because well-behaved code
    // closes a schedule only after all coroutines have finished.
    let mut map = workers().lock().unwrap();
    for slot in schedule.co.iter() {
        if let Some(co) = slot {
            map.remove(&coroutine_key(co));
        }
    }
    drop(map);
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
        schedule.co.resize_with(new_cap, || None);
        schedule.co[id] = Some(co);
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
        // The C version asserts here; this branch is logically unreachable
        // because nco < cap guarantees a free slot exists.
        unreachable!("coroutine_new: no free slot despite nco < cap");
    }
}

// Block the caller until the worker reaches a terminal state for this
// resume cycle (either yielded back to the scheduler or finished).
fn drive_worker(sync: &(Mutex<Signal>, Condvar)) -> Signal {
    let (mtx, cv) = sync;
    let mut s = mtx.lock().unwrap();
    *s = Signal::Run;
    cv.notify_one();
    while *s == Signal::Run {
        s = cv.wait(s).unwrap();
    }
    *s
}

pub fn coroutine_resume(schedule: &mut Schedule, id: i32) {
    assert!(schedule.running == -1);
    assert!(id >= 0 && (id as usize) < schedule.cap);

    let id_usize = id as usize;
    if schedule.co[id_usize].is_none() {
        return;
    }

    let status = schedule.co[id_usize].as_ref().unwrap().status;
    match status {
        COROUTINE_READY => {
            // Mark as running.
            schedule.co[id_usize].as_mut().unwrap().status = COROUTINE_RUNNING;
            schedule.running = id;

            // Capture the function pointer and stable raw pointers for the
            // schedule, the coroutine itself, and its `data` payload.  The
            // `data` pointer is taken from the heap-resident `Box<dyn Any>`
            // and remains valid for the lifetime of the coroutine.
            let co_box: &mut Box<Coroutine> = schedule.co[id_usize].as_mut().unwrap();
            let func = co_box.func;
            let key = coroutine_key(co_box.as_ref());
            let data_ref: &mut dyn Any = co_box.data.as_mut();
            let data_ptr: *mut dyn Any = data_ref as *mut dyn Any;
            let sched_ptr: *mut Schedule = schedule as *mut Schedule;

            let sync: Arc<(Mutex<Signal>, Condvar)> =
                Arc::new((Mutex::new(Signal::Idle), Condvar::new()));
            let sync_for_thread = Arc::clone(&sync);

            let sched_send = SchedSendPtr(sched_ptr);
            let data_send = DataSendPtr(data_ptr);

            let handle = thread::spawn(move || {
                // Wait until the scheduler signals us to begin.
                {
                    let (mtx, cv) = &*sync_for_thread;
                    let mut s = mtx.lock().unwrap();
                    while *s != Signal::Run {
                        s = cv.wait(s).unwrap();
                    }
                }

                // SAFETY: the scheduler thread is parked on the condvar above
                // for the duration of the coroutine body, so it is sound to
                // form a `&mut Schedule` here.  The `data` pointer references
                // a heap allocation owned by the coroutine; that allocation
                // is not moved or freed while the coroutine is alive.
                let sched_ref: &mut Schedule = unsafe { &mut *sched_send.get() };
                let data_ref: &mut dyn Any = unsafe { &mut *data_send.get() };
                func(sched_ref, data_ref);

                // Signal completion to the scheduler.
                let (mtx, cv) = &*sync_for_thread;
                let mut s = mtx.lock().unwrap();
                *s = Signal::Done;
                cv.notify_one();
            });

            workers().lock().unwrap().insert(
                key,
                Worker {
                    handle: Some(handle),
                    sync: Arc::clone(&sync),
                },
            );

            // Hand control to the coroutine and wait for it to yield or finish.
            let outcome = drive_worker(&sync);

            if outcome == Signal::Done {
                // Mirror the C `mainfunc` epilogue: free the coroutine,
                // decrement nco, clear running.
                if let Some(mut worker) = workers().lock().unwrap().remove(&key) {
                    if let Some(h) = worker.handle.take() {
                        let _ = h.join();
                    }
                }
                schedule.co[id_usize] = None;
                schedule.nco -= 1;
                schedule.running = -1;
            }
            // For Signal::Yielded, `coroutine_yield` already updated the
            // status and `running`, so nothing more to do here.
        }
        COROUTINE_SUSPEND => {
            schedule.co[id_usize].as_mut().unwrap().status = COROUTINE_RUNNING;
            schedule.running = id;

            let key = coroutine_key(schedule.co[id_usize].as_ref().unwrap());
            let sync = {
                let map = workers().lock().unwrap();
                Arc::clone(
                    &map.get(&key)
                        .expect("coroutine_resume: missing worker for suspended coroutine")
                        .sync,
                )
            };

            let outcome = drive_worker(&sync);

            if outcome == Signal::Done {
                if let Some(mut worker) = workers().lock().unwrap().remove(&key) {
                    if let Some(h) = worker.handle.take() {
                        let _ = h.join();
                    }
                }
                schedule.co[id_usize] = None;
                schedule.nco -= 1;
                schedule.running = -1;
            }
        }
        _ => {
            // The C code reaches `assert(0)` for any other status.
            unreachable!("coroutine_resume: unexpected status {}", status);
        }
    }
}

pub fn coroutine_yield(schedule: &mut Schedule) {
    let id = schedule.running;
    assert!(id >= 0);
    let id_usize = id as usize;

    let key = coroutine_key(
        schedule.co[id_usize]
            .as_ref()
            .expect("coroutine_yield: running coroutine slot is empty"),
    );

    schedule.co[id_usize].as_mut().unwrap().status = COROUTINE_SUSPEND;
    schedule.running = -1;

    let sync = {
        let map = workers().lock().unwrap();
        Arc::clone(
            &map.get(&key)
                .expect("coroutine_yield: missing worker for running coroutine")
                .sync,
        )
    };

    // Hand control back to the scheduler, then wait until it resumes us.
    let (mtx, cv) = &*sync;
    let mut s = mtx.lock().unwrap();
    *s = Signal::Yielded;
    cv.notify_one();
    while *s == Signal::Yielded {
        s = cv.wait(s).unwrap();
    }
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
