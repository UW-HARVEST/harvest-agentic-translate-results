//! Translation of `c_src/src/task_manager.c` / `c_src/include/task_manager.h`.

use core::ffi::{c_char, c_int, c_void};
use core::mem::size_of;
use core::ptr;

use crate::cstd;
use crate::logger::{log_error, log_info, log_warning};

/// ```c
/// typedef struct {
///     char description[256];
///     int priority;
/// } Task;
/// ```
#[repr(C)]
pub struct Task {
    pub description: [c_char; 256],
    pub priority: c_int,
}

/// ```c
/// typedef struct {
///     Task *tasks;
///     int max_tasks;
///     int task_count;
/// } TaskManager;
/// ```
#[repr(C)]
pub struct TaskManager {
    pub tasks: *mut Task,
    pub max_tasks: c_int,
    pub task_count: c_int,
}

/// ```c
/// TaskManager *create_task_manager();
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn create_task_manager() -> *mut TaskManager {
    unsafe {
        let manager = cstd::malloc(size_of::<TaskManager>()) as *mut TaskManager;
        if manager.is_null() {
            log_error(c"Failed to allocate memory for TaskManager.".as_ptr());
            return ptr::null_mut();
        }

        let max_tasks_env: *mut c_char = cstd::getenv(c"MAX_TASKS".as_ptr());
        (*manager).max_tasks = if !max_tasks_env.is_null() {
            cstd::atoi(max_tasks_env as *const c_char)
        } else {
            10
        };
        (*manager).task_count = 0;

        // `manager->max_tasks * sizeof(Task)`: the `int` operand is converted
        // to `size_t` (sign extension followed by reinterpretation) and the
        // product wraps modulo 2^64, exactly as in C.
        let bytes = ((*manager).max_tasks as isize as usize).wrapping_mul(size_of::<Task>());
        (*manager).tasks = cstd::malloc(bytes) as *mut Task;
        if (*manager).tasks.is_null() {
            log_error(c"Failed to allocate memory for tasks.".as_ptr());
            cstd::free(manager as *mut c_void);
            return ptr::null_mut();
        }

        log_info(c"TaskManager created successfully.".as_ptr());
        manager
    }
}

/// ```c
/// void add_task(TaskManager *manager, const char *description, int priority);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn add_task(manager: *mut TaskManager, description: *const c_char, priority: c_int) {
    unsafe {
        if (*manager).task_count >= (*manager).max_tasks {
            log_warning(c"Cannot add task: Maximum task limit reached.".as_ptr());
            return;
        }

        let index = (*manager).task_count;
        (*manager).task_count = index.wrapping_add(1);
        let task: *mut Task = (*manager).tasks.offset(index as isize);

        let desc: *mut c_char = (&raw mut (*task).description) as *mut c_char;
        cstd::strncpy(desc, description, 256 - 1);
        *desc.add(256 - 1) = 0;
        (*task).priority = priority;

        log_info(c"Task added successfully.".as_ptr());
    }
}

/// ```c
/// void print_tasks(const TaskManager *manager);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn print_tasks(manager: *const TaskManager) {
    unsafe {
        cstd::c_printf(c"Tasks:\n".as_ptr());
        let mut i: c_int = 0;
        while i < (*manager).task_count {
            let task: *const Task = (*manager).tasks.offset(i as isize);
            cstd::c_printf(
                c"  [%d] %s (Priority: %d)\n".as_ptr(),
                i.wrapping_add(1),
                (&raw const (*task).description) as *const c_char,
                (*task).priority,
            );
            i = i.wrapping_add(1);
        }
    }
}

/// ```c
/// void destroy_task_manager(TaskManager *manager);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn destroy_task_manager(manager: *mut TaskManager) {
    unsafe {
        cstd::free((*manager).tasks as *mut c_void);
        cstd::free(manager as *mut c_void);
    }
    log_info(c"TaskManager destroyed successfully.".as_ptr());
}
