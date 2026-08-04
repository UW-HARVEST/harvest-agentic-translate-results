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

// Wrapper to send non-Send types across threads.
// SAFETY: The data is moved entirely to the thread, not shared.
struct SendPtr(*mut dyn Any);
unsafe impl Send for SendPtr {}
unsafe impl Sync for SendPtr {}

struct SendFn(Box<dyn FnOnce()>);
unsafe impl Send for SendFn {}
impl SendFn {
    fn run(self) { (self.0)() }
}

// Wrapper to allow Schedule proxy to be constructed in thread context.
// SAFETY: Proxy schedules have empty co vecs and are only used within their thread.
unsafe impl Send for Schedule {}
unsafe impl Sync for Schedule {}

// Synchronization state for a single coroutine thread
struct CoSync {
    // true when the coroutine thread should run, false when main should run
    coroutine_turn: bool,
    finished: bool,
}

struct CoThread {
    sync: Arc<(Mutex<CoSync>, Condvar)>,
    handle: Option<thread::JoinHandle<()>>,
}

pub struct Coroutine {
    pub func: CoroutineFunc,
    pub data: Box<dyn Any>,
    pub cap: isize,
    pub size: isize,
    pub status: i32,
    pub stack: Option<Box<[u8]>>,
    thread: Option<CoThread>,
}

pub struct Schedule {
    pub stack: Box<[u8]>,
    pub nco: usize,
    pub cap: usize,
    pub running: i32,
    pub co: Vec<Option<Box<Coroutine>>>,
    // When this Schedule is a "proxy" inside a coroutine thread, this holds the sync handle
    proxy_sync: Option<Arc<(Mutex<CoSync>, Condvar)>>,
    // Shared state for proxy schedules to read/write running and coroutine statuses
    shared: Arc<Mutex<SharedState>>,
}

struct SharedState {
    running: i32,
    // Maps coroutine id -> status
    statuses: Vec<i32>,
    nco: usize,
}

pub fn coroutine_open() -> Box<Schedule> {
    let shared = Arc::new(Mutex::new(SharedState {
        running: -1,
        statuses: vec![COROUTINE_DEAD; DEFAULT_COROUTINE],
        nco: 0,
    }));
    Box::new(Schedule {
        stack: vec![0u8; STACK_SIZE].into_boxed_slice(),
        nco: 0,
        cap: DEFAULT_COROUTINE,
        running: -1,
        co: (0..DEFAULT_COROUTINE).map(|_| None).collect(),
        proxy_sync: None,
        shared,
    })
}

pub fn coroutine_close(schedule: Box<Schedule>) {
    // Drop the schedule. All coroutine threads that are suspended need to be cleaned up.
    // When Schedule drops, the CoThread handles drop, which drops the sync Arcs.
    // The coroutine threads will eventually unblock and finish.
    // We need to signal all suspended coroutines to wake up and finish.
    for slot in &schedule.co {
        if let Some(co) = slot {
            if let Some(ct) = &co.thread {
                let (lock, cvar) = &*ct.sync;
                let mut sync = lock.lock().unwrap();
                sync.finished = true;
                sync.coroutine_turn = true;
                cvar.notify_all();
            }
        }
    }
    // Now join all threads
    let mut schedule = schedule;
    for slot in &mut schedule.co {
        if let Some(co) = slot.as_mut() {
            if let Some(ct) = co.thread.as_mut() {
                if let Some(handle) = ct.handle.take() {
                    let _ = handle.join();
                }
            }
        }
    }
    // schedule drops here
}

pub fn coroutine_new(schedule: &mut Schedule, func: CoroutineFunc, data: Box<dyn Any>) -> i32 {
    let co = Box::new(Coroutine {
        func,
        data,
        cap: 0,
        size: 0,
        status: COROUTINE_READY,
        stack: None,
        thread: None,
    });

    if schedule.nco >= schedule.cap {
        let id = schedule.cap;
        let old_cap = schedule.cap;
        schedule.co.resize_with(old_cap * 2, || None);
        schedule.co[old_cap] = Some(co);
        schedule.cap *= 2;
        schedule.nco += 1;
        // Update shared state
        {
            let mut shared = schedule.shared.lock().unwrap();
            shared.statuses.resize(schedule.cap, COROUTINE_DEAD);
            shared.statuses[id] = COROUTINE_READY;
            shared.nco = schedule.nco;
        }
        return id as i32;
    } else {
        for i in 0..schedule.cap {
            let id = (i + schedule.nco) % schedule.cap;
            if schedule.co[id].is_none() {
                schedule.co[id] = Some(co);
                schedule.nco += 1;
                {
                    let mut shared = schedule.shared.lock().unwrap();
                    shared.statuses[id] = COROUTINE_READY;
                    shared.nco = schedule.nco;
                }
                return id as i32;
            }
        }
    }
    panic!("coroutine_new: no free slot");
}

pub fn coroutine_resume(schedule: &mut Schedule, id: i32) {
    assert!(schedule.running == -1);
    assert!(id >= 0 && (id as usize) < schedule.cap);
    let id_usize = id as usize;

    let status = match &schedule.co[id_usize] {
        Some(co) => co.status,
        None => return,
    };

    match status {
        COROUTINE_READY => {
            // Create a new thread for this coroutine
            let sync = Arc::new((
                Mutex::new(CoSync {
                    coroutine_turn: false,
                    finished: false,
                }),
                Condvar::new(),
            ));
            let sync_clone = Arc::clone(&sync);
            let shared_clone = Arc::clone(&schedule.shared);

            // Take func and data out of the coroutine to send to the thread
            let co = schedule.co[id_usize].as_mut().unwrap();
            let func = co.func;
            let data = std::mem::replace(&mut co.data, Box::new(()));
            // SAFETY: data is moved entirely to the thread, not shared
            let data_ptr = SendPtr(Box::into_raw(data));

            co.status = COROUTINE_RUNNING;
            co.thread = Some(CoThread {
                sync: Arc::clone(&sync),
                handle: None,
            });

            schedule.running = id;
            {
                let mut shared = schedule.shared.lock().unwrap();
                shared.running = id;
                shared.statuses[id_usize] = COROUTINE_RUNNING;
            }

            let handle = {
                let closure = move || {
                    // Wait for our turn
                    {
                        let (lock, cvar) = &*sync_clone;
                        let mut sync = lock.lock().unwrap();
                        while !sync.coroutine_turn {
                            sync = cvar.wait(sync).unwrap();
                        }
                        if sync.finished {
                            return;
                        }
                    }

                    // Create a proxy schedule for the coroutine to use
                    let mut proxy = Schedule {
                        stack: vec![0u8; 0].into_boxed_slice(),
                        nco: 0,
                        cap: 0,
                        running: id,
                        co: Vec::new(),
                        proxy_sync: Some(Arc::clone(&sync_clone)),
                        shared: shared_clone.clone(),
                    };

                    // SAFETY: reconstructing the Box from the raw pointer we created
                    let mut data: Box<dyn Any> = unsafe { Box::from_raw(data_ptr.0) };
                    func(&mut proxy, data.as_mut());

                    // Coroutine function returned — signal main thread
                    {
                        let (lock, cvar) = &*sync_clone;
                        let mut sync = lock.lock().unwrap();
                        sync.coroutine_turn = false;
                        sync.finished = true;
                        cvar.notify_all();
                    }
                };
                let send_fn = SendFn(Box::new(closure));
                thread::spawn(move || send_fn.run())
            };

            // Store the handle
            let co = schedule.co[id_usize].as_mut().unwrap();
            if let Some(ct) = co.thread.as_mut() {
                ct.handle = Some(handle);
            }

            // Signal the coroutine thread to start
            {
                let (lock, cvar) = &*sync;
                let mut s = lock.lock().unwrap();
                s.coroutine_turn = true;
                cvar.notify_all();
            }

            // Wait for coroutine to yield or finish
            {
                let (lock, cvar) = &*sync;
                let mut s = lock.lock().unwrap();
                while s.coroutine_turn && !s.finished {
                    s = cvar.wait(s).unwrap();
                }
                if s.finished {
                    // Coroutine completed
                    let co = schedule.co[id_usize].take();
                    if let Some(mut co) = co {
                        if let Some(ct) = co.thread.as_mut() {
                            if let Some(handle) = ct.handle.take() {
                                let _ = handle.join();
                            }
                        }
                    }
                    schedule.nco -= 1;
                    schedule.running = -1;
                    {
                        let mut shared = schedule.shared.lock().unwrap();
                        shared.running = -1;
                        shared.statuses[id_usize] = COROUTINE_DEAD;
                        shared.nco = schedule.nco;
                    }
                } else {
                    // Coroutine yielded
                    let co = schedule.co[id_usize].as_mut().unwrap();
                    co.status = COROUTINE_SUSPEND;
                    schedule.running = -1;
                    {
                        let mut shared = schedule.shared.lock().unwrap();
                        shared.running = -1;
                        shared.statuses[id_usize] = COROUTINE_SUSPEND;
                    }
                }
            }
        }
        COROUTINE_SUSPEND => {
            let co = schedule.co[id_usize].as_mut().unwrap();
            co.status = COROUTINE_RUNNING;
            schedule.running = id;
            {
                let mut shared = schedule.shared.lock().unwrap();
                shared.running = id;
                shared.statuses[id_usize] = COROUTINE_RUNNING;
            }

            let sync = co.thread.as_ref().unwrap().sync.clone();

            // Signal coroutine to continue
            {
                let (lock, cvar) = &*sync;
                let mut s = lock.lock().unwrap();
                s.coroutine_turn = true;
                cvar.notify_all();
            }

            // Wait for yield or finish
            {
                let (lock, cvar) = &*sync;
                let mut s = lock.lock().unwrap();
                while s.coroutine_turn && !s.finished {
                    s = cvar.wait(s).unwrap();
                }
                if s.finished {
                    let co = schedule.co[id_usize].take();
                    if let Some(mut co) = co {
                        if let Some(ct) = co.thread.as_mut() {
                            if let Some(handle) = ct.handle.take() {
                                let _ = handle.join();
                            }
                        }
                    }
                    schedule.nco -= 1;
                    schedule.running = -1;
                    {
                        let mut shared = schedule.shared.lock().unwrap();
                        shared.running = -1;
                        shared.statuses[id_usize] = COROUTINE_DEAD;
                        shared.nco = schedule.nco;
                    }
                } else {
                    let co = schedule.co[id_usize].as_mut().unwrap();
                    co.status = COROUTINE_SUSPEND;
                    schedule.running = -1;
                    {
                        let mut shared = schedule.shared.lock().unwrap();
                        shared.running = -1;
                        shared.statuses[id_usize] = COROUTINE_SUSPEND;
                    }
                }
            }
        }
        _ => panic!("coroutine_resume: unexpected status"),
    }
}

pub fn coroutine_yield(schedule: &mut Schedule) {
    // This is called from within a coroutine thread via the proxy schedule
    let sync = schedule
        .proxy_sync
        .as_ref()
        .expect("coroutine_yield called outside coroutine")
        .clone();

    let (lock, cvar) = &*sync;
    let mut s = lock.lock().unwrap();
    s.coroutine_turn = false;
    cvar.notify_all();

    // Wait until it's our turn again
    while !s.coroutine_turn {
        s = cvar.wait(s).unwrap();
    }
    if s.finished {
        // Schedule is being closed, just return (thread will exit)
        return;
    }
}

pub fn coroutine_status(schedule: &Schedule, id: i32) -> i32 {
    assert!(id >= 0 && (id as usize) < schedule.cap);
    let id_usize = id as usize;
    // If this is a proxy schedule, read from shared state
    if schedule.proxy_sync.is_some() {
        let shared = schedule.shared.lock().unwrap();
        return shared.statuses[id_usize];
    }
    match &schedule.co[id_usize] {
        None => COROUTINE_DEAD,
        Some(co) => co.status,
    }
}

pub fn coroutine_running(schedule: &Schedule) -> i32 {
    if schedule.proxy_sync.is_some() {
        let shared = schedule.shared.lock().unwrap();
        return shared.running;
    }
    schedule.running
}
