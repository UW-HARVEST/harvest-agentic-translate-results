use std::any::Any;
use std::ptr;
use nix::libc;

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
    pub ctx: libc::ucontext_t,
}
pub struct Schedule {
    pub stack: Box<[u8]>,
    pub nco: usize,
    pub cap: usize,
    pub running: i32,
    pub co: Vec<Option<Box<Coroutine>>>,
    pub main: libc::ucontext_t,
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
        main: unsafe { std::mem::zeroed() },
    })
}

pub fn coroutine_close(_schedule: Box<Schedule>) {
    // Box<Schedule>'s Drop impl will recursively drop the co vec, the stack,
    // and each Coroutine (with its saved stack and data).
}

pub fn coroutine_new(schedule: &mut Schedule, func: CoroutineFunc, data: Box<dyn Any>) -> i32 {
    let co = Box::new(Coroutine {
        func,
        data,
        cap: 0,
        size: 0,
        status: COROUTINE_READY,
        stack: None,
        ctx: unsafe { std::mem::zeroed() },
    });

    if schedule.nco >= schedule.cap {
        let id = schedule.cap;
        let new_cap = schedule.cap * 2;
        // Grow the vector, filling with None
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
        unreachable!("coroutine_new: no free slot but nco < cap");
    }
}

extern "C" fn mainfunc(low32: u32, hi32: u32) {
    let ptr_val = (low32 as usize) | ((hi32 as usize) << 32);
    let schedule_ptr = ptr_val as *mut Schedule;
    // SAFETY: the pointer originates from a live Box<Schedule>, and we
    // are running on a stack created from that schedule. There are no
    // simultaneous &mut Schedule references active while this code runs;
    // the original &mut Schedule has been turned into a raw pointer in
    // coroutine_resume.
    let schedule: &mut Schedule = unsafe { &mut *schedule_ptr };
    let id = schedule.running as usize;

    // Pull func and a raw pointer to the dyn Any data out of the
    // coroutine slot. We don't keep a borrow into `schedule.co` while
    // calling the user function, since that function calls
    // coroutine_yield which mutates the schedule.
    let (func, data_ptr): (CoroutineFunc, *mut dyn Any) = {
        let co = schedule.co[id].as_mut().expect("running coroutine missing");
        let data_ptr: *mut dyn Any = &mut *co.data;
        (co.func, data_ptr)
    };

    // SAFETY: the data lives inside the coroutine struct stored in
    // schedule.co[id], which is not moved out or dropped while this
    // call is on the call stack.
    func(schedule, unsafe { &mut *data_ptr });

    // Coroutine's func has returned -- drop the coroutine and update
    // bookkeeping. After this returns, uc_link makes the kernel switch
    // back to S->main automatically.
    schedule.co[id] = None;
    schedule.nco -= 1;
    schedule.running = -1;
}

pub fn coroutine_resume(schedule: &mut Schedule, id: i32) {
    assert_eq!(schedule.running, -1);
    assert!(id >= 0 && (id as usize) < schedule.cap);
    let id_usize = id as usize;
    if schedule.co[id_usize].is_none() {
        return;
    }
    let status = schedule.co[id_usize].as_ref().unwrap().status;

    let schedule_ptr: *mut Schedule = schedule;

    match status {
        COROUTINE_READY => {
            let stack_ptr = schedule.stack.as_mut_ptr() as *mut libc::c_void;
            let main_ptr: *mut libc::ucontext_t = &mut schedule.main;
            unsafe {
                {
                    let co = schedule.co[id_usize].as_mut().unwrap();
                    libc::getcontext(&mut co.ctx);
                    co.ctx.uc_stack.ss_sp = stack_ptr;
                    co.ctx.uc_stack.ss_size = STACK_SIZE;
                    co.ctx.uc_stack.ss_flags = 0;
                    co.ctx.uc_link = main_ptr;
                    co.status = COROUTINE_RUNNING;
                }
                schedule.running = id;

                let s_ptr_val = schedule_ptr as usize;
                let low32 = s_ptr_val as u32;
                let hi32 = (s_ptr_val >> 32) as u32;

                let co_ctx: *mut libc::ucontext_t =
                    &mut schedule.co[id_usize].as_mut().unwrap().ctx;
                let main_func: extern "C" fn(u32, u32) = mainfunc;
                let main_func_erased: extern "C" fn() =
                    std::mem::transmute::<extern "C" fn(u32, u32), extern "C" fn()>(main_func);
                libc::makecontext(co_ctx, main_func_erased, 2, low32, hi32);
                libc::swapcontext(main_ptr, co_ctx);
            }
        }
        COROUTINE_SUSPEND => {
            let s_stack_ptr = schedule.stack.as_mut_ptr();
            let main_ptr: *mut libc::ucontext_t = &mut schedule.main;
            unsafe {
                {
                    let co = schedule.co[id_usize].as_mut().unwrap();
                    let size = co.size as usize;
                    let saved_stack_ptr = co.stack.as_ref().unwrap().as_ptr();
                    let dest = s_stack_ptr.add(STACK_SIZE - size);
                    ptr::copy_nonoverlapping(saved_stack_ptr, dest, size);
                    co.status = COROUTINE_RUNNING;
                }
                schedule.running = id;
                let co_ctx: *mut libc::ucontext_t =
                    &mut schedule.co[id_usize].as_mut().unwrap().ctx;
                libc::swapcontext(main_ptr, co_ctx);
            }
        }
        _ => panic!("coroutine_resume: invalid status {}", status),
    }
}

pub fn coroutine_yield(schedule: &mut Schedule) {
    let id = schedule.running;
    assert!(id >= 0);
    let id_usize = id as usize;

    // Compute the size of the live portion of the shared stack: the shared
    // stack grows downward from `schedule.stack + STACK_SIZE`. `dummy` is a
    // local on that stack near its current top.
    let dummy: u8 = 0;
    let dummy_ptr: *const u8 = &dummy;
    let stack_top: *const u8 = unsafe { schedule.stack.as_ptr().add(STACK_SIZE) };
    let used_size = (stack_top as usize).wrapping_sub(dummy_ptr as usize);
    assert!(used_size <= STACK_SIZE);

    let main_ptr: *mut libc::ucontext_t = &mut schedule.main;
    unsafe {
        {
            let co = schedule.co[id_usize].as_mut().unwrap();
            // Grow the saved-stack buffer if needed.
            if (co.cap as usize) < used_size {
                co.stack = Some(vec![0u8; used_size].into_boxed_slice());
                co.cap = used_size as isize;
            }
            co.size = used_size as isize;
            let dst = co.stack.as_mut().unwrap().as_mut_ptr();
            ptr::copy_nonoverlapping(dummy_ptr, dst, used_size);
            co.status = COROUTINE_SUSPEND;
        }
        schedule.running = -1;
        let co_ctx: *mut libc::ucontext_t = &mut schedule.co[id_usize].as_mut().unwrap().ctx;
        libc::swapcontext(co_ctx, main_ptr);
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
