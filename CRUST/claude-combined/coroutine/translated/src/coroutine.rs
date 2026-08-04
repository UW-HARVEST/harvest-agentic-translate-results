use std::any::Any;
use std::cell::RefCell;
use std::sync::{Arc, Condvar, Mutex};
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

// ---------------- Internal threading infrastructure ----------------
//
// Since Rust does not provide a safe equivalent of ucontext_t, we implement
// stackful coroutines using one OS thread per coroutine.  Cooperative
// scheduling is enforced via a Mutex/Condvar pair: only one of (main thread)
// or (the currently-resumed coroutine thread) ever executes user code at a
// time, so even though they share a `Schedule` via a raw pointer, there is
// never a real data race.

#[derive(Clone, Copy, PartialEq, Eq)]
enum CoroPhase {
    MainTurn,
    CoroTurn,
    Finished,
}

struct CoroSync {
    phase: Mutex<CoroPhase>,
    cond: Condvar,
}

struct CoroData {
    user_data: Box<dyn Any>,
    sync: Arc<CoroSync>,
    thread_handle: Option<JoinHandle<()>>,
}

// Send-wrappers for the raw pointers we ferry between main and coroutine
// threads.  Because of the cooperative scheduling, these pointers are only
// dereferenced while the "other" thread is parked on the condvar, so there
// is never simultaneous access.
struct SchedulePtr(*mut Schedule);
unsafe impl Send for SchedulePtr {}

struct UserDataPtr(*mut dyn Any);
unsafe impl Send for UserDataPtr {}

thread_local! {
    static CURRENT_SYNC: RefCell<Option<Arc<CoroSync>>> = const { RefCell::new(None) };
}

fn coroutine_thread_body(
    func: CoroutineFunc,
    schedule_ptr: SchedulePtr,
    user_data_ptr: UserDataPtr,
    sync: Arc<CoroSync>,
) {
    // Stash the sync handle in a thread-local so coroutine_yield can find it.
    CURRENT_SYNC.with(|cs| {
        *cs.borrow_mut() = Some(sync.clone());
    });

    // Wait until the main thread signals us to start.
    {
        let mut phase = sync.phase.lock().unwrap();
        while *phase != CoroPhase::CoroTurn {
            phase = sync.cond.wait(phase).unwrap();
        }
    }

    // SAFETY: the main thread is parked on the condvar while we run, so
    // we have exclusive access to *schedule_ptr.0 and *user_data_ptr.0
    // for the duration of `func`.
    let schedule_ref: &mut Schedule = unsafe { &mut *schedule_ptr.0 };
    let user_data_ref: &mut dyn Any = unsafe { &mut *user_data_ptr.0 };

    func(schedule_ref, user_data_ref);

    // Function returned normally: tell the main thread we're finished.
    {
        let mut phase = sync.phase.lock().unwrap();
        *phase = CoroPhase::Finished;
        sync.cond.notify_all();
    }

    CURRENT_SYNC.with(|cs| {
        *cs.borrow_mut() = None;
    });
}

// ---------------- Public API ----------------

pub fn coroutine_open() -> Box<Schedule> {
    Box::new(Schedule {
        stack: vec![0u8; STACK_SIZE].into_boxed_slice(),
        nco: 0,
        cap: DEFAULT_COROUTINE,
        running: -1,
        co: (0..DEFAULT_COROUTINE).map(|_| None).collect(),
    })
}

pub fn coroutine_close(mut schedule: Box<Schedule>) {
    // Drop every coroutine still in the schedule.  In the typical test
    // flow all slots are already None, but be defensive: detach any thread
    // handles so dropping the JoinHandle doesn't try to join.
    for slot in schedule.co.iter_mut() {
        if let Some(mut co) = slot.take() {
            if let Some(cdata) = co.data.downcast_mut::<CoroData>() {
                if let Some(handle) = cdata.thread_handle.take() {
                    drop(handle);
                }
            }
        }
    }
    // schedule is dropped here.
}

pub fn coroutine_new(
    schedule: &mut Schedule,
    func: CoroutineFunc,
    data: Box<dyn Any>,
) -> i32 {
    let cdata = CoroData {
        user_data: data,
        sync: Arc::new(CoroSync {
            phase: Mutex::new(CoroPhase::MainTurn),
            cond: Condvar::new(),
        }),
        thread_handle: None,
    };
    let co = Box::new(Coroutine {
        func,
        data: Box::new(cdata),
        cap: 0,
        size: 0,
        status: COROUTINE_READY,
        stack: None,
    });

    if schedule.nco >= schedule.cap {
        // Grow the slot vector and place the new coroutine at the old end.
        let id = schedule.cap as i32;
        let new_cap = schedule.cap * 2;
        schedule.co.resize_with(new_cap, || None);
        schedule.co[schedule.cap] = Some(co);
        schedule.cap = new_cap;
        schedule.nco += 1;
        id
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
        unreachable!("nco < cap implies a free slot exists");
    }
}

pub fn coroutine_resume(schedule: &mut Schedule, id: i32) {
    assert_eq!(schedule.running, -1);
    assert!(id >= 0 && (id as usize) < schedule.cap);

    if schedule.co[id as usize].is_none() {
        return;
    }

    // Form the raw pointer once and reuse.  This is just a pointer cast;
    // it does not extend the borrow of `schedule`.
    let schedule_ptr = SchedulePtr(schedule as *mut Schedule);

    // First: prepare the coroutine (spawn its thread on the very first
    // resume) and snapshot the Arc<CoroSync> so we can drop the &mut
    // borrow on `schedule.co[id]` before manipulating other fields.
    let sync: Arc<CoroSync> = {
        let co = schedule.co[id as usize].as_mut().unwrap();
        let status = co.status;
        let func = co.func;
        let cdata = co.data.downcast_mut::<CoroData>().expect("CoroData");
        let sync = cdata.sync.clone();

        match status {
            COROUTINE_READY => {
                // Spawn the worker thread the first time around.
                let user_data_ptr =
                    UserDataPtr(&mut *cdata.user_data as *mut dyn Any);
                let sync_for_thread = sync.clone();
                let handle = thread::spawn(move || {
                    coroutine_thread_body(
                        func,
                        schedule_ptr,
                        user_data_ptr,
                        sync_for_thread,
                    );
                });
                cdata.thread_handle = Some(handle);
                co.status = COROUTINE_RUNNING;
            }
            COROUTINE_SUSPEND => {
                co.status = COROUTINE_RUNNING;
            }
            _ => panic!("invalid coroutine status: {}", status),
        }
        sync
    };

    schedule.running = id;

    // Hand control over to the coroutine thread and wait for it to either
    // yield (-> MainTurn) or complete (-> Finished).
    let is_finished = {
        let mut phase = sync.phase.lock().unwrap();
        *phase = CoroPhase::CoroTurn;
        sync.cond.notify_all();
        while *phase == CoroPhase::CoroTurn {
            phase = sync.cond.wait(phase).unwrap();
        }
        *phase == CoroPhase::Finished
    };

    schedule.running = -1;

    if is_finished {
        // The coroutine ran to completion.  Join its thread, free the
        // slot, and decrement the live-coroutine count.
        if let Some(mut co) = schedule.co[id as usize].take() {
            if let Some(cdata) = co.data.downcast_mut::<CoroData>() {
                if let Some(handle) = cdata.thread_handle.take() {
                    let _ = handle.join();
                }
            }
        }
        if schedule.nco > 0 {
            schedule.nco -= 1;
        }
    }
}

pub fn coroutine_yield(schedule: &mut Schedule) {
    let id = schedule.running;
    assert!(id >= 0);

    if let Some(co) = schedule.co[id as usize].as_mut() {
        co.status = COROUTINE_SUSPEND;
    }
    schedule.running = -1;

    let sync = CURRENT_SYNC.with(|cs| {
        cs.borrow()
            .as_ref()
            .expect("coroutine_yield called outside a coroutine")
            .clone()
    });

    let mut phase = sync.phase.lock().unwrap();
    *phase = CoroPhase::MainTurn;
    sync.cond.notify_all();
    while *phase != CoroPhase::CoroTurn {
        phase = sync.cond.wait(phase).unwrap();
    }
    // Resumed: control returns to the user function.
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
