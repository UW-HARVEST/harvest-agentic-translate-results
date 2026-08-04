use std::any::Any;
use std::sync::mpsc;
use std::thread;

pub const COROUTINE_DEAD: i32 = 0;
pub const COROUTINE_READY: i32 = 1;
pub const COROUTINE_RUNNING: i32 = 2;
pub const COROUTINE_SUSPEND: i32 = 3;
pub const STACK_SIZE: usize = 1024 * 1024;
pub const DEFAULT_COROUTINE: usize = 16;
pub type CoroutineFunc = fn(schedule: &mut Schedule, data: &mut dyn Any);

enum ToCoroutine {
    Resume,
    Shutdown,
}

enum FromCoroutine {
    Yielded,
    Finished,
}

struct SendPtr(*mut dyn Any);
unsafe impl Send for SendPtr {}

pub struct Coroutine {
    pub func: CoroutineFunc,
    pub data: Box<dyn Any>,
    pub cap: isize,
    pub size: isize,
    pub status: i32,
    pub stack: Option<Box<[u8]>>,
    tx_to_co: Option<mpsc::Sender<ToCoroutine>>,
    rx_from_co: Option<mpsc::Receiver<FromCoroutine>>,
    handle: Option<thread::JoinHandle<()>>,
}

pub struct Schedule {
    pub stack: Box<[u8]>,
    pub nco: usize,
    pub cap: usize,
    pub running: i32,
    pub co: Vec<Option<Box<Coroutine>>>,
    yield_tx: Option<mpsc::Sender<FromCoroutine>>,
    yield_rx: Option<mpsc::Receiver<ToCoroutine>>,
}

fn wait_for_coroutine(schedule: &mut Schedule, idx: usize) {
    let msg = schedule.co[idx]
        .as_ref()
        .unwrap()
        .rx_from_co
        .as_ref()
        .unwrap()
        .recv();
    match msg {
        Ok(FromCoroutine::Yielded) => {
            schedule.co[idx].as_mut().unwrap().status = COROUTINE_SUSPEND;
            schedule.running = -1;
        }
        Ok(FromCoroutine::Finished) | Err(_) => {
            let mut co = schedule.co[idx].take().unwrap();
            if let Some(handle) = co.handle.take() {
                let _ = handle.join();
            }
            schedule.nco -= 1;
            schedule.running = -1;
        }
    }
}

pub fn coroutine_open() -> Box<Schedule> {
    let mut co = Vec::with_capacity(DEFAULT_COROUTINE);
    co.resize_with(DEFAULT_COROUTINE, || None);
    Box::new(Schedule {
        stack: vec![0u8; STACK_SIZE].into_boxed_slice(),
        nco: 0,
        cap: DEFAULT_COROUTINE,
        running: -1,
        co,
        yield_tx: None,
        yield_rx: None,
    })
}

pub fn coroutine_close(mut schedule: Box<Schedule>) {
    for i in 0..schedule.cap {
        if let Some(mut co) = schedule.co[i].take() {
            if let Some(tx) = co.tx_to_co.take() {
                let _ = tx.send(ToCoroutine::Shutdown);
            }
            if let Some(handle) = co.handle.take() {
                let _ = handle.join();
            }
        }
    }
}

pub fn coroutine_new(schedule: &mut Schedule, func: CoroutineFunc, data: Box<dyn Any>) -> i32 {
    let co = Box::new(Coroutine {
        func,
        data,
        cap: 0,
        size: 0,
        status: COROUTINE_READY,
        stack: None,
        tx_to_co: None,
        rx_from_co: None,
        handle: None,
    });

    if schedule.nco >= schedule.cap {
        let id = schedule.cap;
        let old_cap = schedule.cap;
        schedule.co.resize_with(old_cap * 2, || None);
        schedule.co[old_cap] = Some(co);
        schedule.cap *= 2;
        schedule.nco += 1;
        return id as i32;
    }
    for i in 0..schedule.cap {
        let id = (i + schedule.nco) % schedule.cap;
        if schedule.co[id].is_none() {
            schedule.co[id] = Some(co);
            schedule.nco += 1;
            return id as i32;
        }
    }
    panic!("unreachable");
}

pub fn coroutine_resume(schedule: &mut Schedule, id: i32) {
    assert!(schedule.running == -1);
    assert!(id >= 0 && (id as usize) < schedule.cap);
    let idx = id as usize;

    if schedule.co[idx].is_none() {
        return;
    }

    let status = schedule.co[idx].as_ref().unwrap().status;
    match status {
        COROUTINE_READY => {
            let (tx_to_co, rx_in_co) = mpsc::channel::<ToCoroutine>();
            let (tx_from_co, rx_from_co) = mpsc::channel::<FromCoroutine>();

            let co = schedule.co[idx].as_mut().unwrap();
            co.tx_to_co = Some(tx_to_co);
            co.rx_from_co = Some(rx_from_co);
            co.status = COROUTINE_RUNNING;
            schedule.running = id;

            let func = co.func;
            let data_ptr = SendPtr(&mut *co.data as *mut dyn Any);
            let tx_clone = tx_from_co;

            let handle = thread::spawn(move || {
                let data_ptr = data_ptr; // move SendPtr into closure
                let mut proxy = Schedule {
                    stack: vec![0u8; 0].into_boxed_slice(),
                    nco: 0,
                    cap: 0,
                    running: id,
                    co: Vec::new(),
                    yield_tx: Some(tx_clone),
                    yield_rx: Some(rx_in_co),
                };
                // Safety: data is only accessed from this thread while running.
                // Main thread blocks on channel until coroutine yields/finishes.
                let data_ref = unsafe { &mut *data_ptr.0 };
                func(&mut proxy, data_ref);
                let _ = proxy.yield_tx.as_ref().unwrap().send(FromCoroutine::Finished);
            });

            schedule.co[idx].as_mut().unwrap().handle = Some(handle);
            wait_for_coroutine(schedule, idx);
        }
        COROUTINE_SUSPEND => {
            let co = schedule.co[idx].as_mut().unwrap();
            co.status = COROUTINE_RUNNING;
            schedule.running = id;
            co.tx_to_co.as_ref().unwrap().send(ToCoroutine::Resume).unwrap();
            wait_for_coroutine(schedule, idx);
        }
        _ => panic!("invalid coroutine status"),
    }
}

pub fn coroutine_yield(schedule: &mut Schedule) {
    assert!(schedule.running >= 0);
    schedule
        .yield_tx
        .as_ref()
        .unwrap()
        .send(FromCoroutine::Yielded)
        .unwrap();
    match schedule.yield_rx.as_ref().unwrap().recv() {
        Ok(ToCoroutine::Resume) => {}
        _ => {
            panic!("coroutine shutdown");
        }
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
