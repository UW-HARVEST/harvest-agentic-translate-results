use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Mutex, OnceLock};
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
// Internal coroutine machinery built on OS threads + channels.
// ---------------------------------------------------------------------------
//
// The C implementation uses ucontext_t for stack switching.  Since we cannot
// FFI into libc and want to avoid hand-rolled assembly, each coroutine here
// runs on its own OS thread.  The scheduler thread and the coroutine thread
// hand control back and forth using a pair of mpsc channels:
//
//   * resume channel  (scheduler -> coroutine):  carries a raw pointer to the
//     Schedule so the coroutine has access to it during its quantum.
//   * yield  channel  (coroutine -> scheduler):  signals that the coroutine
//     either yielded (still alive) or finished.
//
// At any given time only one of the two threads is touching the Schedule
// memory: the other is parked on a `recv()` call.  This keeps the access
// strictly sequential, mirroring the cooperative semantics of the C version.

enum YieldMsg {
    Yielded,
    Finished,
}

struct CoroChannels {
    resume_tx: Sender<usize>,
    yield_rx: Option<Receiver<YieldMsg>>,
}

fn registry() -> &'static Mutex<HashMap<(usize, i32), CoroChannels>> {
    static REGISTRY: OnceLock<Mutex<HashMap<(usize, i32), CoroChannels>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

struct TlsData {
    yield_tx: Sender<YieldMsg>,
    resume_rx: Receiver<usize>,
}

thread_local! {
    static COROUTINE_TLS: RefCell<Option<TlsData>> = RefCell::new(None);
}

/// Wrapper that allows passing a fat pointer to `dyn Any` across thread
/// boundaries.  Sending raw pointers between threads is fine here because the
/// scheduler has gone to sleep on its `recv()` call before the coroutine
/// thread dereferences anything.
struct AnyMutPtr(*mut (dyn Any + 'static));
unsafe impl Send for AnyMutPtr {}

fn run_coroutine_thread(
    func: CoroutineFunc,
    data_ptr: AnyMutPtr,
    yield_tx: Sender<YieldMsg>,
    resume_rx: Receiver<usize>,
) {
    // Wait for the first resume signal.  If the channel is closed before we
    // ever start (e.g. coroutine_close was called before resume), bail out.
    let schedule_ptr = match resume_rx.recv() {
        Ok(p) => p,
        Err(_) => return,
    };

    COROUTINE_TLS.with(|tls| {
        *tls.borrow_mut() = Some(TlsData {
            yield_tx: yield_tx.clone(),
            resume_rx,
        });
    });

    // SAFETY: the scheduler is parked on `yield_rx.recv()` while we run, so
    // we have exclusive access to *schedule_ptr until we yield/finish.  The
    // data pointer references heap memory owned by the Coroutine struct,
    // which lives until the coroutine completes.
    let schedule: &mut Schedule = unsafe { &mut *(schedule_ptr as *mut Schedule) };
    let data_ref: &mut dyn Any = unsafe { &mut *data_ptr.0 };
    func(schedule, data_ref);

    // Clear TLS so a future thread reuse (unlikely with std) is clean.
    COROUTINE_TLS.with(|tls| {
        *tls.borrow_mut() = None;
    });

    let _ = yield_tx.send(YieldMsg::Finished);
}

pub fn coroutine_open() -> Box<Schedule> {
    let mut co = Vec::with_capacity(DEFAULT_COROUTINE);
    for _ in 0..DEFAULT_COROUTINE {
        co.push(None);
    }
    Box::new(Schedule {
        stack: vec![0u8; STACK_SIZE].into_boxed_slice(),
        nco: 0,
        cap: DEFAULT_COROUTINE,
        running: -1,
        co,
    })
}

pub fn coroutine_close(schedule: Box<Schedule>) {
    let key_addr = &*schedule as *const Schedule as usize;
    {
        let mut reg = registry().lock().unwrap();
        let keys: Vec<_> = reg
            .keys()
            .filter(|(addr, _)| *addr == key_addr)
            .cloned()
            .collect();
        for k in keys {
            // Dropping the entry drops resume_tx, which causes any still-
            // suspended coroutine threads to wake from recv() with Err and
            // exit cleanly.
            reg.remove(&k);
        }
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
        schedule.co.resize_with(new_cap, || None);
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
        // The C version asserts(0) here; mirror that with a panic.
        panic!("coroutine_new: no free slot available");
    }
}

pub fn coroutine_resume(schedule: &mut Schedule, id: i32) {
    assert_eq!(schedule.running, -1);
    assert!(id >= 0 && (id as usize) < schedule.cap);

    let status = match schedule.co.get(id as usize).and_then(|c| c.as_ref()) {
        Some(c) => c.status,
        None => return,
    };

    let sched_addr = schedule as *const Schedule as usize;
    let key = (sched_addr, id);
    let sched_ptr = schedule as *mut Schedule as usize;

    let (resume_tx, yield_rx) = match status {
        COROUTINE_READY => {
            let (rtx, rrx) = mpsc::channel::<usize>();
            let (ytx, yrx) = mpsc::channel::<YieldMsg>();

            let func = schedule.co[id as usize].as_ref().unwrap().func;
            // Take a stable raw pointer to the data inside the Box.  The
            // pointee lives on the heap and stays put until the Coroutine is
            // dropped (which only happens after the thread has exited).
            let data_ptr = {
                let co_ref = schedule.co[id as usize].as_mut().unwrap();
                AnyMutPtr(&mut *co_ref.data as *mut (dyn Any + 'static))
            };

            thread::spawn(move || {
                run_coroutine_thread(func, data_ptr, ytx, rrx);
            });

            schedule.co[id as usize].as_mut().unwrap().status = COROUTINE_RUNNING;
            schedule.running = id;

            registry().lock().unwrap().insert(
                key,
                CoroChannels {
                    resume_tx: rtx.clone(),
                    yield_rx: None,
                },
            );

            (rtx, yrx)
        }
        COROUTINE_SUSPEND => {
            schedule.co[id as usize].as_mut().unwrap().status = COROUTINE_RUNNING;
            schedule.running = id;

            let (rtx, yrx) = {
                let mut reg = registry().lock().unwrap();
                let entry = reg
                    .get_mut(&key)
                    .expect("coroutine channels missing from registry");
                let yrx = entry
                    .yield_rx
                    .take()
                    .expect("yield_rx not parked in registry");
                (entry.resume_tx.clone(), yrx)
            };
            (rtx, yrx)
        }
        _ => panic!("coroutine_resume: invalid status {}", status),
    };

    // Hand control to the coroutine thread, then park until it yields/exits.
    resume_tx
        .send(sched_ptr)
        .expect("failed to send resume signal");
    let msg = yield_rx
        .recv()
        .expect("coroutine thread terminated unexpectedly");

    match msg {
        YieldMsg::Yielded => {
            // Coroutine is still alive; park yield_rx back in the registry
            // so the next resume can recover it.
            let mut reg = registry().lock().unwrap();
            if let Some(entry) = reg.get_mut(&key) {
                entry.yield_rx = Some(yield_rx);
            }
        }
        YieldMsg::Finished => {
            // Coroutine ran to completion.  Mirror the C cleanup:
            //   _co_delete(C); S->co[id] = NULL; --S->nco; S->running = -1;
            schedule.co[id as usize] = None;
            if schedule.nco > 0 {
                schedule.nco -= 1;
            }
            schedule.running = -1;
            registry().lock().unwrap().remove(&key);
        }
    }
}

pub fn coroutine_yield(schedule: &mut Schedule) {
    let id = schedule.running;
    assert!(id >= 0);

    if let Some(co) = schedule
        .co
        .get_mut(id as usize)
        .and_then(|c| c.as_mut())
    {
        co.status = COROUTINE_SUSPEND;
    }
    schedule.running = -1;

    COROUTINE_TLS.with(|tls| {
        let mut tls_mut = tls.borrow_mut();
        let tls_inner = tls_mut
            .as_mut()
            .expect("coroutine_yield called outside of a coroutine");
        tls_inner
            .yield_tx
            .send(YieldMsg::Yielded)
            .expect("failed to send yield signal");
        // Block until the scheduler resumes us.
        let _ = tls_inner
            .resume_rx
            .recv()
            .expect("resume channel closed while yielded");
    });
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
