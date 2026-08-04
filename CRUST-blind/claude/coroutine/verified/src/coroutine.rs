use std::any::Any;
use std::cell::RefCell;
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

// --- internal coordination types ---

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cmd {
    None,
    Run,
    Kill,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Resp {
    None,
    Yielded,
    Done,
}

struct CoroState {
    cmd: Mutex<Cmd>,
    cmd_cv: Condvar,
    resp: Mutex<Resp>,
    resp_cv: Condvar,
    handle: Mutex<Option<JoinHandle<()>>>,
}

fn states_map() -> &'static Mutex<HashMap<usize, Arc<CoroState>>> {
    static M: OnceLock<Mutex<HashMap<usize, Arc<CoroState>>>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(HashMap::new()))
}

thread_local! {
    static CURRENT: RefCell<Option<Arc<CoroState>>> = const { RefCell::new(None) };
}

// Wrapper to allow raw pointers to be sent across threads. Mutual exclusion
// is enforced by the mutex/condvar protocol so only one thread accesses the
// pointee at a time.
struct SendMutPtr<T: ?Sized>(*mut T);
unsafe impl<T: ?Sized> Send for SendMutPtr<T> {}
impl<T: ?Sized> SendMutPtr<T> {
    fn get(&self) -> *mut T {
        self.0
    }
}

impl Drop for Coroutine {
    fn drop(&mut self) {
        let key = self as *mut Coroutine as usize;
        let state_opt = {
            let mut map = states_map().lock().unwrap();
            map.remove(&key)
        };
        if let Some(state) = state_opt {
            // Tell the thread (if it is suspended in a yield) to terminate.
            {
                let mut cmd = state.cmd.lock().unwrap();
                *cmd = Cmd::Kill;
            }
            state.cmd_cv.notify_all();
            let handle = state.handle.lock().unwrap().take();
            if let Some(h) = handle {
                let _ = h.join();
            }
        }
    }
}

// --- public API ---

pub fn coroutine_open() -> Box<Schedule> {
    Box::new(Schedule {
        stack: vec![0u8; STACK_SIZE].into_boxed_slice(),
        nco: 0,
        cap: DEFAULT_COROUTINE,
        running: -1,
        co: (0..DEFAULT_COROUTINE).map(|_| None).collect(),
    })
}

pub fn coroutine_close(_schedule: Box<Schedule>) {
    // Dropping the Box drops the Schedule, which drops every remaining
    // Coroutine, which kills/joins their backing threads via Drop.
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
        unreachable!("coroutine_new: no slot found");
    }
}

pub fn coroutine_resume(schedule: &mut Schedule, id: i32) {
    assert!(schedule.running == -1);
    assert!(id >= 0 && (id as usize) < schedule.cap);
    let idx = id as usize;
    if schedule.co[idx].is_none() {
        return;
    }

    let sched_ptr: *mut Schedule = schedule as *mut Schedule;
    let status = schedule.co[idx].as_ref().unwrap().status;

    match status {
        COROUTINE_READY => {
            let (state, coro_key, func, co_ptr) = {
                let co_box = schedule.co[idx].as_mut().unwrap();
                let co_ref: &mut Coroutine = &mut **co_box;
                let co_ptr: *mut Coroutine = co_ref as *mut Coroutine;
                let coro_key = co_ptr as usize;
                let func = co_ref.func;
                let state = Arc::new(CoroState {
                    cmd: Mutex::new(Cmd::None),
                    cmd_cv: Condvar::new(),
                    resp: Mutex::new(Resp::None),
                    resp_cv: Condvar::new(),
                    handle: Mutex::new(None),
                });
                states_map()
                    .lock()
                    .unwrap()
                    .insert(coro_key, state.clone());
                (state, coro_key, func, co_ptr)
            };

            schedule.running = id;
            schedule.co[idx].as_mut().unwrap().status = COROUTINE_RUNNING;

            let thread_state = state.clone();
            let sched_ptr_s = SendMutPtr(sched_ptr);
            let co_ptr_s = SendMutPtr(co_ptr);

            let h = thread::spawn(move || {
                CURRENT.with(|c| *c.borrow_mut() = Some(thread_state.clone()));

                // Wait for first Run command.
                {
                    let mut cmd = thread_state.cmd.lock().unwrap();
                    while *cmd == Cmd::None {
                        cmd = thread_state.cmd_cv.wait(cmd).unwrap();
                    }
                    let received = *cmd;
                    *cmd = Cmd::None;
                    if received == Cmd::Kill {
                        return;
                    }
                }

                // Run user function. Catch any panic so the main thread
                // is not left waiting forever.
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    // SAFETY: while user code runs, the main thread is
                    // blocked waiting for a response, so it is the unique
                    // accessor of the Schedule and Coroutine.
                    unsafe {
                        let s: &mut Schedule = &mut *sched_ptr_s.get();
                        let co: &mut Coroutine = &mut *co_ptr_s.get();
                        let data: &mut dyn Any = &mut *co.data;
                        (func)(s, data);
                    }
                }));

                // Signal Done.
                {
                    let mut resp = thread_state.resp.lock().unwrap();
                    *resp = Resp::Done;
                }
                thread_state.resp_cv.notify_all();
            });

            *state.handle.lock().unwrap() = Some(h);

            // Send Run signal.
            {
                let mut cmd = state.cmd.lock().unwrap();
                *cmd = Cmd::Run;
            }
            state.cmd_cv.notify_all();

            wait_for_response(schedule, idx, coro_key, &state);
        }
        COROUTINE_SUSPEND => {
            let coro_key = {
                let co_box = schedule.co[idx].as_mut().unwrap();
                let co_ref: &mut Coroutine = &mut **co_box;
                co_ref as *mut Coroutine as usize
            };
            let state = states_map()
                .lock()
                .unwrap()
                .get(&coro_key)
                .cloned()
                .expect("missing coroutine state");

            schedule.running = id;
            schedule.co[idx].as_mut().unwrap().status = COROUTINE_RUNNING;

            {
                let mut cmd = state.cmd.lock().unwrap();
                *cmd = Cmd::Run;
            }
            state.cmd_cv.notify_all();

            wait_for_response(schedule, idx, coro_key, &state);
        }
        _ => unreachable!("coroutine_resume: invalid status"),
    }
}

fn wait_for_response(
    schedule: &mut Schedule,
    idx: usize,
    coro_key: usize,
    state: &Arc<CoroState>,
) {
    let r = {
        let mut resp = state.resp.lock().unwrap();
        while *resp == Resp::None {
            resp = state.resp_cv.wait(resp).unwrap();
        }
        let r = *resp;
        *resp = Resp::None;
        r
    };

    if r == Resp::Done {
        let handle = state.handle.lock().unwrap().take();
        if let Some(h) = handle {
            let _ = h.join();
        }
        states_map().lock().unwrap().remove(&coro_key);
        // Drop the coroutine; running is set to -1 here, mirroring the C
        // mainfunc cleanup.
        schedule.co[idx] = None;
        schedule.nco -= 1;
        schedule.running = -1;
    }
    // Yielded: state was already updated by the yield function on the
    // coroutine thread (status -> SUSPEND, running -> -1).
}

pub fn coroutine_yield(schedule: &mut Schedule) {
    let id = schedule.running;
    assert!(id >= 0);
    schedule.co[id as usize].as_mut().unwrap().status = COROUTINE_SUSPEND;
    schedule.running = -1;

    let state = CURRENT.with(|c| {
        c.borrow()
            .as_ref()
            .expect("coroutine_yield called outside of coroutine")
            .clone()
    });

    // Notify main thread that we yielded.
    {
        let mut resp = state.resp.lock().unwrap();
        *resp = Resp::Yielded;
    }
    state.resp_cv.notify_all();

    // Wait for the next Run (or Kill).
    let received = {
        let mut cmd = state.cmd.lock().unwrap();
        while *cmd == Cmd::None {
            cmd = state.cmd_cv.wait(cmd).unwrap();
        }
        let received = *cmd;
        *cmd = Cmd::None;
        received
    };

    if received == Cmd::Kill {
        // Unwind out of the user function so the thread can exit.
        std::panic::panic_any("coroutine killed");
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
