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

// =========================================================================
// Internal coroutine runtime
//
// The C implementation uses `ucontext_t` (makecontext/swapcontext) to switch
// between coroutine stacks within a single OS thread. There is no portable,
// safe-Rust equivalent for arbitrary stack switching of synchronous code, so
// instead we run each coroutine in its own OS thread and use rendezvous
// (zero-capacity) channels to give the impression of cooperative,
// single-threaded execution: at any point exactly one of the main thread or
// a coroutine thread is running, while the others are blocked on a channel.
//
// Sharing `&mut Schedule` between the main thread and the coroutine thread
// requires a small amount of `unsafe` (raw pointer dereference). This is
// safe in practice because:
//   - The `Schedule` lives in a `Box`, so its address is stable.
//   - When a coroutine is running, the main thread is blocked waiting on
//     a channel and does not touch the `Schedule`.
//   - When the main thread is running, the coroutine thread is blocked.
// =========================================================================

#[derive(Copy, Clone)]
struct SendUsize(usize);
unsafe impl Send for SendUsize {}

struct SendBoxAny(Box<dyn Any>);
// SAFETY: We require that user data is logically Send. Since `Box<dyn Any>`
// is the only way to pass user data into a coroutine and the data is owned
// solely by the coroutine while it runs, we mark this Send so it can cross
// the thread boundary. Users that store !Send types (e.g. `Rc`) in
// coroutine data would violate this contract; the existing C-style API
// has no equivalent restriction so this matches expected usage.
unsafe impl Send for SendBoxAny {}

enum CoSignal {
    Yielded,
    Done(SendBoxAny),
}

struct CoThreadInfo {
    handle: Option<JoinHandle<()>>,
    tx_resume: SyncSender<SendUsize>,
    rx_signal: Receiver<CoSignal>,
}

thread_local! {
    static CO_CHANS: RefCell<Option<(SyncSender<CoSignal>, Receiver<SendUsize>)>>
        = const { RefCell::new(None) };
}

fn co_thread_map() -> &'static Mutex<HashMap<usize, CoThreadInfo>> {
    static MAP: OnceLock<Mutex<HashMap<usize, CoThreadInfo>>> = OnceLock::new();
    MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

fn coroutine_addr(co: &Coroutine) -> usize {
    co as *const Coroutine as usize
}

// =========================================================================
// Public API
// =========================================================================

pub fn coroutine_open() -> Box<Schedule> {
    let mut co_vec: Vec<Option<Box<Coroutine>>> = Vec::with_capacity(DEFAULT_COROUTINE);
    for _ in 0..DEFAULT_COROUTINE {
        co_vec.push(None);
    }
    Box::new(Schedule {
        stack: vec![0u8; STACK_SIZE].into_boxed_slice(),
        nco: 0,
        cap: DEFAULT_COROUTINE,
        running: -1,
        co: co_vec,
    })
}

pub fn coroutine_close(mut schedule: Box<Schedule>) {
    // Drop every coroutine; for any threads still alive, dropping the
    // resume sender will cause the coroutine thread's `recv` to fail and
    // the thread to terminate, which we then join.
    for i in 0..schedule.co.len() {
        if let Some(co) = schedule.co[i].take() {
            let addr = coroutine_addr(&*co);
            let info_opt = co_thread_map().lock().unwrap().remove(&addr);
            if let Some(mut info) = info_opt {
                drop(info.tx_resume);
                drop(info.rx_signal);
                if let Some(h) = info.handle.take() {
                    let _ = h.join();
                }
            }
            // `co` (Box<Coroutine>) is dropped here.
            drop(co);
        }
    }
    // `schedule` (Box<Schedule>) is dropped here.
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
        unreachable!("coroutine_new: no free slot found despite nco < cap");
    }
}

pub fn coroutine_resume(schedule: &mut Schedule, id: i32) {
    assert_eq!(schedule.running, -1);
    assert!(id >= 0 && (id as usize) < schedule.cap);
    let id_us = id as usize;

    if schedule.co[id_us].is_none() {
        return;
    }

    let status = schedule.co[id_us].as_ref().unwrap().status;
    let sched_ptr = schedule as *mut Schedule as usize;
    let co_address = coroutine_addr(schedule.co[id_us].as_deref().unwrap());

    match status {
        COROUTINE_READY => {
            schedule.running = id;
            schedule.co[id_us].as_deref_mut().unwrap().status = COROUTINE_RUNNING;

            // Take ownership of the user data so the coroutine thread can
            // own it for the duration of `func`. Replace it with a
            // placeholder so the field type is preserved while the
            // coroutine runs; the data is moved back when the coroutine
            // completes.
            let data_taken = std::mem::replace(
                &mut schedule.co[id_us].as_deref_mut().unwrap().data,
                Box::new(()) as Box<dyn Any>,
            );
            let data_send = SendBoxAny(data_taken);
            let func = schedule.co[id_us].as_ref().unwrap().func;

            let (tx_resume, rx_resume) = sync_channel::<SendUsize>(0);
            let (tx_signal, rx_signal) = sync_channel::<CoSignal>(0);

            let handle = thread::spawn(move || {
                run_coroutine_thread(func, data_send, tx_signal, rx_resume);
            });

            // Register thread info so future resumes / close can find it.
            co_thread_map().lock().unwrap().insert(
                co_address,
                CoThreadInfo {
                    handle: Some(handle),
                    tx_resume: tx_resume.clone(),
                    rx_signal,
                },
            );

            // Send first resume signal (rendezvous: blocks until thread recvs).
            if tx_resume.send(SendUsize(sched_ptr)).is_err() {
                // Thread died early; clean up.
                let info_opt = co_thread_map().lock().unwrap().remove(&co_address);
                if let Some(mut info) = info_opt {
                    if let Some(h) = info.handle.take() {
                        let _ = h.join();
                    }
                }
                schedule.co[id_us] = None;
                schedule.nco -= 1;
                schedule.running = -1;
                return;
            }

            // Wait for yield/done signal.
            let signal = recv_signal(co_address);
            handle_signal(schedule, id_us, co_address, signal);
        }
        COROUTINE_SUSPEND => {
            schedule.running = id;
            schedule.co[id_us].as_deref_mut().unwrap().status = COROUTINE_RUNNING;

            // Send resume signal.
            let tx_resume = {
                let map = co_thread_map().lock().unwrap();
                let info = map
                    .get(&co_address)
                    .expect("coroutine_resume: missing thread info");
                info.tx_resume.clone()
            };
            tx_resume
                .send(SendUsize(sched_ptr))
                .expect("coroutine_resume: failed to send resume signal");

            let signal = recv_signal(co_address);
            handle_signal(schedule, id_us, co_address, signal);
        }
        _ => panic!("coroutine_resume: invalid status {}", status),
    }
}

fn run_coroutine_thread(
    func: CoroutineFunc,
    data_send: SendBoxAny,
    tx_signal: SyncSender<CoSignal>,
    rx_resume: Receiver<SendUsize>,
) {
    let SendBoxAny(mut data) = data_send;

    // Install thread-local channels so `coroutine_yield` (called from
    // within `func`) can communicate with the main thread.
    CO_CHANS.with(|c| {
        *c.borrow_mut() = Some((tx_signal, rx_resume));
    });

    // Wait for the first resume signal from the main thread.
    let first_addr = CO_CHANS.with(|c| {
        c.borrow()
            .as_ref()
            .expect("coroutine thread: CO_CHANS not set")
            .1
            .recv()
    });

    let sched_addr = match first_addr {
        Ok(SendUsize(addr)) => addr,
        Err(_) => {
            // Schedule was closed before we ever ran; just exit.
            return;
        }
    };

    // SAFETY: The main thread is currently blocked waiting on `rx_signal`,
    // so we have exclusive access to the Schedule for the duration of
    // `func` (until we yield or return). The Schedule lives in a Box and
    // its address is stable.
    let schedule_ref: &mut Schedule = unsafe { &mut *(sched_addr as *mut Schedule) };
    let data_ref: &mut dyn Any = &mut *data;
    func(schedule_ref, data_ref);

    // Function completed; signal Done and return ownership of `data` to
    // the main thread so it can be put back into the Coroutine struct
    // (if the struct is still alive, i.e. for cleanup).
    let chans = CO_CHANS.with(|c| c.borrow_mut().take());
    if let Some((tx, _rx)) = chans {
        let _ = tx.send(CoSignal::Done(SendBoxAny(data)));
    }
}

fn recv_signal(co_address: usize) -> CoSignal {
    // Hold the global map lock briefly while receiving. Because only one
    // coroutine is active at a time and the coroutine thread does not
    // touch the map, contention is minimal.
    let map = co_thread_map().lock().unwrap();
    let info = map
        .get(&co_address)
        .expect("recv_signal: missing thread info");
    info.rx_signal
        .recv()
        .expect("recv_signal: coroutine thread terminated unexpectedly")
}

fn handle_signal(schedule: &mut Schedule, id_us: usize, co_address: usize, signal: CoSignal) {
    match signal {
        CoSignal::Yielded => {
            if let Some(co) = schedule.co[id_us].as_deref_mut() {
                co.status = COROUTINE_SUSPEND;
            }
            schedule.running = -1;
        }
        CoSignal::Done(data) => {
            // Restore data into the coroutine slot just so it has the
            // expected type, then drop the whole coroutine.
            if let Some(co) = schedule.co[id_us].as_deref_mut() {
                co.data = data.0;
            }
            schedule.co[id_us] = None;
            schedule.nco -= 1;
            schedule.running = -1;

            // Remove the thread info from the global map and join the
            // coroutine thread.
            let info_opt = co_thread_map().lock().unwrap().remove(&co_address);
            if let Some(mut info) = info_opt {
                drop(info.tx_resume);
                if let Some(h) = info.handle.take() {
                    let _ = h.join();
                }
            }
        }
    }
}

pub fn coroutine_yield(schedule: &mut Schedule) {
    let id = schedule.running;
    assert!(id >= 0, "coroutine_yield called outside of a coroutine");

    // The user data placeholder etc. stay valid; the schedule address
    // (Box<Schedule>) is stable, so the user's `schedule: &mut Schedule`
    // local in `func` is still valid after the resume.
    let _ = schedule;

    // Send Yielded signal to main and wait for next resume.
    let result: Result<usize, ()> = CO_CHANS.with(|c| {
        let chans = c.borrow();
        let (tx, rx) = chans
            .as_ref()
            .expect("coroutine_yield: CO_CHANS not set");
        if tx.send(CoSignal::Yielded).is_err() {
            return Err(());
        }
        match rx.recv() {
            Ok(SendUsize(addr)) => Ok(addr),
            Err(_) => Err(()),
        }
    });

    if result.is_err() {
        // Main has closed the schedule while we were yielded.
        // Panicking here unwinds the coroutine's stack so destructors run.
        panic!("coroutine_yield: schedule was closed while suspended");
    }
}

pub fn coroutine_status(schedule: &Schedule, id: i32) -> i32 {
    assert!(id >= 0 && (id as usize) < schedule.cap);
    let id_us = id as usize;
    match &schedule.co[id_us] {
        None => COROUTINE_DEAD,
        Some(co) => co.status,
    }
}

pub fn coroutine_running(schedule: &Schedule) -> i32 {
    schedule.running
}
