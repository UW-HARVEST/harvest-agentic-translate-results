use nix::libc::{self, getcontext, makecontext, swapcontext, ucontext_t};
use std::any::Any;
use std::mem::MaybeUninit;
use std::ptr;

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
    pub ctx: ucontext_t,
}

pub struct Schedule {
    pub stack: Box<[u8]>,
    pub nco: usize,
    pub cap: usize,
    pub running: i32,
    pub co: Vec<Option<Box<Coroutine>>>,
    pub main: ucontext_t,
}

fn new_coroutine(func: CoroutineFunc, data: Box<dyn Any>) -> Box<Coroutine> {
    Box::new(Coroutine {
        func,
        data,
        cap: 0,
        size: 0,
        status: COROUTINE_READY,
        stack: None,
        ctx: unsafe { MaybeUninit::<ucontext_t>::zeroed().assume_init() },
    })
}

pub fn coroutine_open() -> Box<Schedule> {
    let stack = vec![0u8; STACK_SIZE].into_boxed_slice();
    let mut co: Vec<Option<Box<Coroutine>>> = Vec::with_capacity(DEFAULT_COROUTINE);
    for _ in 0..DEFAULT_COROUTINE {
        co.push(None);
    }
    Box::new(Schedule {
        stack,
        nco: 0,
        cap: DEFAULT_COROUTINE,
        running: -1,
        co,
        main: unsafe { MaybeUninit::<ucontext_t>::zeroed().assume_init() },
    })
}

pub fn coroutine_close(mut schedule: Box<Schedule>) {
    for i in 0..schedule.cap {
        // Replace with None to drop the coroutine (which frees its stack via Box).
        schedule.co[i] = None;
    }
    schedule.co.clear();
    // Schedule is dropped at end of scope.
    drop(schedule);
}

pub fn coroutine_new(schedule: &mut Schedule, func: CoroutineFunc, data: Box<dyn Any>) -> i32 {
    let co = new_coroutine(func, data);
    if schedule.nco >= schedule.cap {
        let id = schedule.cap as i32;
        let new_cap = schedule.cap * 2;
        // Grow the vector.
        schedule.co.reserve(schedule.cap);
        for _ in schedule.cap..new_cap {
            schedule.co.push(None);
        }
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
        unreachable!("coroutine_new: no free slot found");
    }
}

extern "C" fn mainfunc(low32: u32, hi32: u32) {
    let ptr: usize = (low32 as usize) | ((hi32 as usize) << 32);
    let s_ptr = ptr as *mut Schedule;
    unsafe {
        let s: &mut Schedule = &mut *s_ptr;
        let id = s.running as usize;
        // Take out func and a raw pointer to data so we can call without
        // holding a borrow on the slot for the duration of the call.
        let (func, data_ptr): (CoroutineFunc, *mut dyn Any) = {
            let co = s.co[id].as_mut().expect("running coroutine missing");
            let data_ptr: *mut dyn Any = &mut *co.data;
            (co.func, data_ptr)
        };
        func(&mut *s_ptr, &mut *data_ptr);
        // Coroutine has finished; clean up.
        let s: &mut Schedule = &mut *s_ptr;
        s.co[id] = None;
        s.nco -= 1;
        s.running = -1;
    }
}

pub fn coroutine_resume(schedule: &mut Schedule, id: i32) {
    assert!(schedule.running == -1);
    assert!(id >= 0 && (id as usize) < schedule.cap);
    let idx = id as usize;
    if schedule.co[idx].is_none() {
        return;
    }
    let status = schedule.co[idx].as_ref().unwrap().status;
    let s_ptr: *mut Schedule = schedule as *mut Schedule;
    match status {
        COROUTINE_READY => unsafe {
            let stack_ptr = schedule.stack.as_mut_ptr();
            {
                let co = schedule.co[idx].as_mut().unwrap();
                let ctx_ptr: *mut ucontext_t = &mut co.ctx;
                getcontext(ctx_ptr);
                (*ctx_ptr).uc_stack.ss_sp = stack_ptr as *mut libc::c_void;
                (*ctx_ptr).uc_stack.ss_size = STACK_SIZE;
                (*ctx_ptr).uc_link = &mut (*s_ptr).main as *mut ucontext_t;
                co.status = COROUTINE_RUNNING;
            }
            schedule.running = id;
            let ptr_val = s_ptr as usize;
            let low32 = (ptr_val & 0xFFFFFFFF) as u32;
            let hi32 = (ptr_val >> 32) as u32;
            let ctx_ptr: *mut ucontext_t = &mut schedule.co[idx].as_mut().unwrap().ctx;
            let mainfunc_cast: extern "C" fn() = std::mem::transmute(mainfunc as extern "C" fn(u32, u32));
            makecontext(ctx_ptr, mainfunc_cast, 2, low32, hi32);
            swapcontext(&mut schedule.main as *mut ucontext_t, ctx_ptr);
        },
        COROUTINE_SUSPEND => unsafe {
            let stack_ptr = schedule.stack.as_mut_ptr();
            {
                let co = schedule.co[idx].as_mut().unwrap();
                let size = co.size as usize;
                if let Some(ref saved) = co.stack {
                    let dst = stack_ptr.add(STACK_SIZE - size);
                    ptr::copy_nonoverlapping(saved.as_ptr(), dst, size);
                }
                co.status = COROUTINE_RUNNING;
            }
            schedule.running = id;
            let ctx_ptr: *mut ucontext_t = &mut schedule.co[idx].as_mut().unwrap().ctx;
            swapcontext(&mut schedule.main as *mut ucontext_t, ctx_ptr);
        },
        _ => panic!("coroutine_resume: invalid status"),
    }
}

fn save_stack(co: &mut Coroutine, top: *const u8) {
    let dummy: u8 = 0;
    let dummy_ptr: *const u8 = &dummy;
    let used = (top as isize) - (dummy_ptr as isize);
    assert!(used <= STACK_SIZE as isize);
    let used_usize = used as usize;
    if (co.cap as usize) < used_usize {
        co.cap = used as isize;
        co.stack = Some(vec![0u8; used_usize].into_boxed_slice());
    }
    co.size = used;
    if let Some(ref mut buf) = co.stack {
        unsafe {
            ptr::copy_nonoverlapping(dummy_ptr, buf.as_mut_ptr(), used_usize);
        }
    }
}

pub fn coroutine_yield(schedule: &mut Schedule) {
    let id = schedule.running;
    assert!(id >= 0);
    let idx = id as usize;
    let stack_top: *const u8 = unsafe { schedule.stack.as_ptr().add(STACK_SIZE) };
    let stack_base: *const u8 = schedule.stack.as_ptr();
    let main_ptr: *mut ucontext_t = &mut schedule.main;
    let ctx_ptr: *mut ucontext_t = {
        let co = schedule.co[idx].as_mut().expect("running coroutine missing");
        // Sanity check that we are on the shared stack.
        let dummy: u8 = 0;
        let dummy_ptr: *const u8 = &dummy;
        assert!((dummy_ptr as usize) > (stack_base as usize));
        save_stack(co, stack_top);
        co.status = COROUTINE_SUSPEND;
        &mut co.ctx as *mut ucontext_t
    };
    schedule.running = -1;
    unsafe {
        swapcontext(ctx_ptr, main_ptr);
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
