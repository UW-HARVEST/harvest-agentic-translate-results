use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::ptr;

use corosensei::{
    Coroutine as StackCoroutine,
    CoroutineResult,
    Yielder as StackYielder,
    stack::DefaultStack,
};

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

type RuntimeCoroutine = StackCoroutine<(), (), ()>;

thread_local! {
    static RUNTIMES: RefCell<HashMap<usize, RuntimeCoroutine>> = RefCell::new(HashMap::new());
    static ACTIVE_YIELDER: Cell<*const StackYielder<(), ()>> = const { Cell::new(ptr::null()) };
}

fn coroutine_key(coroutine: &Coroutine) -> usize {
    coroutine as *const Coroutine as usize
}

fn remove_runtime(key: usize) {
    RUNTIMES.with(|runtimes| {
        runtimes.borrow_mut().remove(&key);
    });
}

struct ActiveYielderGuard {
    previous: *const StackYielder<(), ()>,
}

impl ActiveYielderGuard {
    fn install(yielder: *const StackYielder<(), ()>) -> Self {
        let previous = ACTIVE_YIELDER.with(|slot| slot.replace(yielder));
        Self { previous }
    }
}

impl Drop for ActiveYielderGuard {
    fn drop(&mut self) {
        ACTIVE_YIELDER.with(|slot| slot.set(self.previous));
    }
}

pub fn coroutine_open() -> Box<Schedule> {
    let mut co = Vec::with_capacity(DEFAULT_COROUTINE);
    co.resize_with(DEFAULT_COROUTINE, || None);

    Box::new(Schedule {
        stack: vec![0; STACK_SIZE].into_boxed_slice(),
        nco: 0,
        cap: DEFAULT_COROUTINE,
        running: -1,
        co,
    })
}
pub fn coroutine_close(schedule: Box<Schedule>) {
    let keys: Vec<usize> = schedule
        .co
        .iter()
        .filter_map(|co| co.as_deref().map(coroutine_key))
        .collect();

    RUNTIMES.with(|runtimes| {
        let mut runtimes = runtimes.borrow_mut();
        for key in keys {
            runtimes.remove(&key);
        }
    });

    drop(schedule);
}
pub fn coroutine_new(schedule: &mut Schedule, func: CoroutineFunc, data: Box<dyn Any>) -> i32 {
    let coroutine = Box::new(Coroutine {
        func,
        data,
        cap: 0,
        size: 0,
        status: COROUTINE_READY,
        stack: None,
    });

    if schedule.nco >= schedule.cap {
        let id = schedule.cap;
        schedule.co.resize_with(schedule.cap * 2, || None);
        schedule.co[id] = Some(coroutine);
        schedule.cap *= 2;
        schedule.nco += 1;
        return id as i32;
    }

    for i in 0..schedule.cap {
        let id = (i + schedule.nco) % schedule.cap;
        if schedule.co[id].is_none() {
            schedule.co[id] = Some(coroutine);
            schedule.nco += 1;
            return id as i32;
        }
    }

    unreachable!("coroutine table has no free slot")
}
pub fn coroutine_resume(schedule: &mut Schedule, id: i32) {
    assert_eq!(schedule.running, -1);
    assert!(id >= 0 && (id as usize) < schedule.cap);

    let index = id as usize;
    let (status, key, coroutine_ptr) = match schedule.co[index].as_deref_mut() {
        Some(coroutine) => (
            coroutine.status,
            coroutine_key(coroutine),
            coroutine as *mut Coroutine,
        ),
        None => return,
    };

    if status == COROUTINE_READY {
        let schedule_ptr = schedule as *mut Schedule;
        let runtime = StackCoroutine::with_stack(
            DefaultStack::new(STACK_SIZE).expect("failed to allocate coroutine stack"),
            move |yielder, ()| {
                let _guard = ActiveYielderGuard::install(yielder as *const StackYielder<(), ()>);
                let schedule = unsafe { &mut *schedule_ptr };
                let coroutine = unsafe { &mut *coroutine_ptr };
                (coroutine.func)(schedule, coroutine.data.as_mut());
            },
        );

        RUNTIMES.with(|runtimes| {
            runtimes.borrow_mut().insert(key, runtime);
        });
    } else {
        assert_eq!(status, COROUTINE_SUSPEND);
    }

    schedule.running = id;
    if let Some(coroutine) = schedule.co[index].as_deref_mut() {
        coroutine.status = COROUTINE_RUNNING;
    }

    let result = RUNTIMES.with(|runtimes| {
        let mut runtimes = runtimes.borrow_mut();
        let runtime = runtimes
            .get_mut(&key)
            .expect("missing coroutine runtime for active coroutine");
        catch_unwind(AssertUnwindSafe(|| runtime.resume(())))
    });

    match result {
        Ok(CoroutineResult::Yield(())) => {}
        Ok(CoroutineResult::Return(())) => {
            remove_runtime(key);
            schedule.co[index] = None;
            schedule.nco -= 1;
            schedule.running = -1;
        }
        Err(payload) => {
            remove_runtime(key);
            schedule.co[index] = None;
            schedule.nco -= 1;
            schedule.running = -1;
            resume_unwind(payload);
        }
    }
}
pub fn coroutine_yield(schedule: &mut Schedule) {
    let id = schedule.running;
    assert!(id >= 0);

    let coroutine = schedule.co[id as usize]
        .as_deref_mut()
        .expect("running coroutine slot must exist");
    coroutine.status = COROUTINE_SUSPEND;
    schedule.running = -1;

    let yielder = ACTIVE_YIELDER.with(|slot| slot.get());
    assert!(!yielder.is_null(), "coroutine_yield called outside a running coroutine");
    unsafe {
        (&*yielder).suspend(());
    }
}
pub fn coroutine_status(schedule: &Schedule, id: i32) -> i32 {
    assert!(id >= 0 && (id as usize) < schedule.cap);
    schedule.co[id as usize]
        .as_deref()
        .map_or(COROUTINE_DEAD, |coroutine| coroutine.status)
}
pub fn coroutine_running(schedule: &Schedule) -> i32 {
    schedule.running
}
