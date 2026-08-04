use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{mpsc, Mutex, OnceLock};
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

// ---- Internal synchronization helpers ----

#[derive(Debug)]
enum Notification {
    Yielded,
    Done,
}

struct MainSyncState {
    resume_tx: mpsc::Sender<()>,
    notify_rx: mpsc::Receiver<Notification>,
    handle: Option<JoinHandle<()>>,
}

struct ThreadSyncState {
    resume_rx: mpsc::Receiver<()>,
    notify_tx: mpsc::Sender<Notification>,
}

thread_local! {
    static THREAD_SYNC: RefCell<Option<ThreadSyncState>> = RefCell::new(None);
}

static REGISTRY: OnceLock<Mutex<HashMap<(usize, i32), MainSyncState>>> = OnceLock::new();

fn registry() -> &'static Mutex<HashMap<(usize, i32), MainSyncState>> {
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn schedule_id(s: &Schedule) -> usize {
    s.stack.as_ptr() as usize
}

#[derive(Clone, Copy)]
struct SchedulePtr(usize);
unsafe impl Send for SchedulePtr {}
impl SchedulePtr {
    fn new(s: &mut Schedule) -> Self {
        SchedulePtr(s as *mut Schedule as usize)
    }
    unsafe fn get(self) -> *mut Schedule {
        self.0 as *mut Schedule
    }
}

fn handle_notification(
    schedule: &mut Schedule,
    sid: usize,
    id: i32,
    notif: Notification,
    resume_tx: mpsc::Sender<()>,
    notify_rx: mpsc::Receiver<Notification>,
    handle: JoinHandle<()>,
) {
    match notif {
        Notification::Yielded => {
            // Status was already updated by yield()
            registry().lock().unwrap().insert(
                (sid, id),
                MainSyncState {
                    resume_tx,
                    notify_rx,
                    handle: Some(handle),
                },
            );
        }
        Notification::Done => {
            schedule.co[id as usize] = None;
            schedule.nco -= 1;
            schedule.running = -1;
            drop(resume_tx);
            drop(notify_rx);
            let _ = handle.join();
        }
    }
}

// ---- Public API ----

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
    let sid = schedule_id(&schedule);

    // Remove and collect all registry entries for this schedule
    let mut to_join: Vec<MainSyncState> = Vec::new();
    {
        let mut reg = registry().lock().unwrap();
        let keys: Vec<(usize, i32)> = reg
            .keys()
            .filter(|(s, _)| *s == sid)
            .copied()
            .collect();
        for k in keys {
            if let Some(state) = reg.remove(&k) {
                to_join.push(state);
            }
        }
    }

    // Drop sender to break receiver, then join threads
    for state in to_join {
        drop(state.resume_tx);
        if let Some(h) = state.handle {
            let _ = h.join();
        }
    }

    // Clear all coroutine slots
    for i in 0..schedule.cap {
        schedule.co[i] = None;
    }
    // Box drops here, freeing the schedule.
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
        let old_cap = schedule.cap;
        let new_cap = schedule.cap * 2;
        schedule.co.resize_with(new_cap, || None);
        schedule.co[old_cap] = Some(co);
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
        unreachable!()
    }
}

pub fn coroutine_resume(schedule: &mut Schedule, id: i32) {
    assert_eq!(schedule.running, -1);
    assert!(id >= 0 && (id as usize) < schedule.cap);

    let status = match &schedule.co[id as usize] {
        Some(co) => co.status,
        None => return,
    };

    let sid = schedule_id(schedule);

    match status {
        COROUTINE_READY => {
            let (resume_tx, resume_rx) = mpsc::channel::<()>();
            let (notify_tx, notify_rx) = mpsc::channel::<Notification>();

            schedule.running = id;
            let co = schedule.co[id as usize].as_mut().unwrap();
            co.status = COROUTINE_RUNNING;
            let func = co.func;

            let schedule_ptr = SchedulePtr::new(schedule);
            let notify_tx_for_thread = notify_tx.clone();
            let notify_tx_done = notify_tx;

            let handle = thread::spawn(move || {
                THREAD_SYNC.with(|ts| {
                    *ts.borrow_mut() = Some(ThreadSyncState {
                        resume_rx,
                        notify_tx: notify_tx_for_thread,
                    });
                });

                {
                    let sched: &mut Schedule = unsafe { &mut *schedule_ptr.get() };
                    let id_local = sched.running as usize;
                    // Take ownership of data so we don't hold overlapping &mut.
                    let mut data_box: Box<dyn Any> = std::mem::replace(
                        &mut sched.co[id_local].as_mut().unwrap().data,
                        Box::new(()) as Box<dyn Any>,
                    );
                    func(sched, data_box.as_mut());
                    // data_box drops here; the coroutine is dying anyway.
                }

                // Clean thread-local before exit
                THREAD_SYNC.with(|ts| {
                    let _ = ts.borrow_mut().take();
                });

                let _ = notify_tx_done.send(Notification::Done);
            });

            let notif = notify_rx.recv().unwrap();
            handle_notification(schedule, sid, id, notif, resume_tx, notify_rx, handle);
        }
        COROUTINE_SUSPEND => {
            schedule.running = id;
            let co = schedule.co[id as usize].as_mut().unwrap();
            co.status = COROUTINE_RUNNING;

            let state = registry().lock().unwrap().remove(&(sid, id)).unwrap();
            let MainSyncState {
                resume_tx,
                notify_rx,
                handle,
            } = state;
            let handle = handle.unwrap();

            resume_tx.send(()).unwrap();
            let notif = notify_rx.recv().unwrap();
            handle_notification(schedule, sid, id, notif, resume_tx, notify_rx, handle);
        }
        _ => panic!("coroutine_resume: invalid status {}", status),
    }
}

pub fn coroutine_yield(schedule: &mut Schedule) {
    let id = schedule.running;
    assert!(id >= 0);

    let co = schedule.co[id as usize].as_mut().unwrap();
    co.status = COROUTINE_SUSPEND;
    schedule.running = -1;

    // Take sync state out, signal yield, wait for resume, put back.
    let sync = THREAD_SYNC.with(|ts| ts.borrow_mut().take().unwrap());
    sync.notify_tx.send(Notification::Yielded).unwrap();
    sync.resume_rx.recv().unwrap();
    THREAD_SYNC.with(|ts| *ts.borrow_mut() = Some(sync));
}

pub fn coroutine_status(schedule: &Schedule, id: i32) -> i32 {
    assert!(id >= 0 && (id as usize) < schedule.cap);
    match &schedule.co[id as usize] {
        Some(co) => co.status,
        None => COROUTINE_DEAD,
    }
}

pub fn coroutine_running(schedule: &Schedule) -> i32 {
    schedule.running
}
