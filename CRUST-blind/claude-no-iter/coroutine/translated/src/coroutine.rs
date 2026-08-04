use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Mutex, OnceLock};
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

// ---------------------------------------------------------------------------
// Internal coordination state.
//
// The C implementation uses `ucontext` to perform stack switching between the
// scheduler ("main") context and a coroutine context.  We cannot call into
// libc from this implementation, so we instead simulate the same cooperative
// behaviour by hosting each coroutine on its own OS thread and using a
// rendezvous (sync_channel of size 0) to ensure that exactly one of the two
// sides (the scheduler or the coroutine) is making progress at any moment.
//
// A global registry keyed by the coroutine's stable heap address is used to
// look up the per-coroutine channel/thread pair when resuming a previously
// suspended coroutine.
// ---------------------------------------------------------------------------

#[derive(Debug)]
enum CoMsg {
    Yielded,
    Done,
}

struct ChannelPair {
    /// Scheduler -> coroutine: carries the address of the active `Schedule`.
    to_co: SyncSender<usize>,
    /// Coroutine -> scheduler: signals yield or completion.
    from_co: Receiver<CoMsg>,
    /// JoinHandle for the coroutine's thread (cleared when joined).
    handle: Option<JoinHandle<()>>,
}

fn registry() -> &'static Mutex<HashMap<usize, ChannelPair>> {
    static REGISTRY: OnceLock<Mutex<HashMap<usize, ChannelPair>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

thread_local! {
    /// On a coroutine thread, this stores the channel ends used by
    /// `coroutine_yield` to signal back to the scheduler and wait to be
    /// resumed.  It is `None` outside of a coroutine.
    static CO_YIELD: RefCell<Option<(SyncSender<CoMsg>, Receiver<usize>)>> = RefCell::new(None);
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
    let mut sched = schedule;
    let cap = sched.cap;
    for i in 0..cap {
        if let Some(co) = sched.co[i].take() {
            // Stable heap address of the Coroutine: used as registry key.
            let key = &*co as *const Coroutine as usize;
            // Remove and shut down any thread we still have running for it.
            let pair_opt = registry().lock().unwrap().remove(&key);
            if let Some(pair) = pair_opt {
                // Dropping the senders/receivers will cause the suspended
                // coroutine thread (which is parked inside `coroutine_yield`'s
                // `recv`) to receive an error and unwind.
                drop(pair.to_co);
                drop(pair.from_co);
                if let Some(handle) = pair.handle {
                    let _ = handle.join();
                }
            }
            // Drop the coroutine itself.
            drop(co);
        }
    }
    sched.co.clear();
    drop(sched);
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
        // Grow the table: double capacity, place new coroutine at old end.
        let id = schedule.cap;
        let new_cap = schedule.cap * 2;
        schedule.co.resize_with(new_cap, || None);
        schedule.co[id] = Some(co);
        schedule.cap = new_cap;
        schedule.nco += 1;
        return id as i32;
    }

    let cap = schedule.cap;
    for i in 0..cap {
        let id = (i + schedule.nco) % cap;
        if schedule.co[id].is_none() {
            schedule.co[id] = Some(co);
            schedule.nco += 1;
            return id as i32;
        }
    }
    // Unreachable: nco < cap implies at least one free slot exists.
    -1
}

/// Body of a coroutine thread.  Waits for the first resume signal, runs the
/// user function, then signals completion.
fn coro_thread(
    rx: Receiver<usize>,
    tx: SyncSender<CoMsg>,
    func: CoroutineFunc,
    co_ptr: usize,
) {
    // Wait for the first resume.
    let schedule_ptr = match rx.recv() {
        Ok(p) => p,
        Err(_) => return,
    };

    // Stash a clone of `tx` together with `rx` so that `coroutine_yield`,
    // which only has access to `&mut Schedule`, can locate them.
    let tx_clone = tx.clone();
    CO_YIELD.with(|cell| {
        *cell.borrow_mut() = Some((tx_clone, rx));
    });

    // Run the user function.  Catch panics so we can still notify the
    // scheduler that the coroutine is finished and avoid poisoning the
    // global registry.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // SAFETY: While this thread is the "active" coroutine the scheduler
        // is parked in `coroutine_resume` waiting on `from_co.recv()`, so the
        // raw pointers do not race with any access on the scheduler thread.
        // The synchronisation is provided by the rendezvous sync_channels.
        let schedule: &mut Schedule = unsafe { &mut *(schedule_ptr as *mut Schedule) };
        let co: &mut Coroutine = unsafe { &mut *(co_ptr as *mut Coroutine) };
        let data_ref: &mut dyn Any = &mut *co.data;
        (func)(schedule, data_ref);
    }));

    // Drop the thread-local state before notifying the scheduler.
    CO_YIELD.with(|cell| {
        *cell.borrow_mut() = None;
    });

    let _ = tx.send(CoMsg::Done);
    // Discard any panic so the JoinHandle is happy.
    let _ = result;
}

pub fn coroutine_resume(schedule: &mut Schedule, id: i32) {
    assert_eq!(schedule.running, -1);
    assert!(id >= 0);
    let idx = id as usize;
    assert!(idx < schedule.cap);

    // Determine the registry key (stable Coroutine heap address) and current
    // status; bail if the slot is empty (matches C: silent return).
    let key = match schedule.co[idx].as_ref() {
        Some(c) => &**c as *const Coroutine as usize,
        None => return,
    };
    let status = schedule.co[idx].as_ref().unwrap().status;

    let pair = match status {
        COROUTINE_READY => {
            // First time we resume this coroutine: spawn its thread.
            let (tx_main, rx_co) = sync_channel::<usize>(0);
            let (tx_co, rx_main) = sync_channel::<CoMsg>(0);
            let func = schedule.co[idx].as_ref().unwrap().func;
            let co_ptr = key;
            let handle = thread::spawn(move || {
                coro_thread(rx_co, tx_co, func, co_ptr);
            });
            ChannelPair {
                to_co: tx_main,
                from_co: rx_main,
                handle: Some(handle),
            }
        }
        COROUTINE_SUSPEND => {
            // Take the channel/thread pair out of the registry while the
            // coroutine is running so we don't hold the global lock during
            // the rendezvous.
            registry()
                .lock()
                .unwrap()
                .remove(&key)
                .expect("missing channels for suspended coroutine")
        }
        _ => unreachable!("invalid coroutine status"),
    };

    // Mark the coroutine as running.
    schedule.running = id;
    schedule.co[idx].as_mut().unwrap().status = COROUTINE_RUNNING;

    // Hand control to the coroutine.  The cast to a raw pointer / usize
    // launders the borrow so we can hand it across the rendezvous.
    let schedule_ptr = schedule as *mut Schedule as usize;
    pair.to_co
        .send(schedule_ptr)
        .expect("coroutine thread closed");
    let msg = pair.from_co.recv().expect("coroutine thread closed");

    // Coroutine is no longer the active one.
    schedule.running = -1;

    match msg {
        CoMsg::Yielded => {
            schedule.co[idx].as_mut().unwrap().status = COROUTINE_SUSPEND;
            // Re-insert the pair so the next resume can find it.
            registry().lock().unwrap().insert(key, pair);
        }
        CoMsg::Done => {
            // Coroutine finished: clean up the slot and reap its thread.
            if let Some(handle) = pair.handle {
                let _ = handle.join();
            }
            schedule.co[idx] = None;
            schedule.nco -= 1;
        }
    }
}

pub fn coroutine_yield(schedule: &mut Schedule) {
    // The C implementation uses the `Schedule` reference for assertions and
    // for saving the stack; in our threaded model we don't need it directly,
    // but accepting it here matches the published signature.
    let _ = schedule;

    CO_YIELD.with(|cell| {
        let borrow = cell.borrow();
        let pair = borrow
            .as_ref()
            .expect("coroutine_yield called outside a coroutine");
        // Notify the scheduler we are yielding.
        pair.0
            .send(CoMsg::Yielded)
            .expect("scheduler channel closed");
        // Block until the scheduler resumes us with a fresh Schedule address.
        pair.1.recv().expect("scheduler channel closed");
    });
}

pub fn coroutine_status(schedule: &Schedule, id: i32) -> i32 {
    assert!(id >= 0);
    let idx = id as usize;
    assert!(idx < schedule.cap);
    match &schedule.co[idx] {
        None => COROUTINE_DEAD,
        Some(c) => c.status,
    }
}

pub fn coroutine_running(schedule: &Schedule) -> i32 {
    schedule.running
}
