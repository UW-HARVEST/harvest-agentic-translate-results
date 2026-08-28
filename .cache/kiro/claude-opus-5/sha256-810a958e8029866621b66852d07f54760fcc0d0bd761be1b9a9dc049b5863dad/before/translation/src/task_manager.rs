//! Translation of `c_src/src/task_manager.c` / `c_src/include/task_manager.h`.

use crate::cstdio::print_stdout;
use crate::cutil::{c_atoi, c_str_bytes, getenv_bytes, strncpy};
use crate::logger::{log_error_str, log_info_str, log_warning_str};
use std::alloc::{alloc, dealloc, Layout};
use std::ffi::{c_char, c_int};

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

/// `malloc(max_tasks * sizeof(Task))`.
///
/// The C expression converts the `int` count to `size_t` (sign extension) and
/// multiplies modulo 2^64, so a negative or absurdly large `MAX_TASKS`
/// produces a request that `malloc` refuses.  `malloc(0)` returns a valid,
/// non-NULL pointer under glibc.
fn tasks_layout(max_tasks: c_int) -> Option<Layout> {
    let size = (max_tasks as usize).wrapping_mul(std::mem::size_of::<Task>());
    if size == 0 {
        // Stand-in for glibc's minimum allocation for malloc(0).
        let align = std::mem::align_of::<Task>();
        Some(Layout::from_size_align(align, align).unwrap())
    } else {
        Layout::from_size_align(size, std::mem::align_of::<Task>()).ok()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn create_task_manager() -> *mut TaskManager {
    let manager_layout = Layout::new::<TaskManager>();
    let manager = unsafe { alloc(manager_layout) } as *mut TaskManager;
    if manager.is_null() {
        log_error_str(c"Failed to allocate memory for TaskManager.");
        return std::ptr::null_mut();
    }

    unsafe {
        let max_tasks_env = getenv_bytes("MAX_TASKS");
        (*manager).max_tasks = match max_tasks_env {
            Some(v) => c_atoi(&v),
            None => 10,
        };
        (*manager).task_count = 0;

        let tasks = match tasks_layout((*manager).max_tasks) {
            Some(layout) => alloc(layout) as *mut Task,
            None => std::ptr::null_mut(),
        };
        (*manager).tasks = tasks;

        if tasks.is_null() {
            log_error_str(c"Failed to allocate memory for tasks.");
            dealloc(manager as *mut u8, manager_layout);
            return std::ptr::null_mut();
        }
    }

    log_info_str(c"TaskManager created successfully.");
    manager
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn add_task(
    manager: *mut TaskManager,
    description: *const c_char,
    priority: c_int,
) {
    if (*manager).task_count >= (*manager).max_tasks {
        log_warning_str(c"Cannot add task: Maximum task limit reached.");
        return;
    }

    let index = (*manager).task_count;
    (*manager).task_count = index.wrapping_add(1);

    let task = (*manager).tasks.add(index as usize);
    let desc = std::ptr::addr_of_mut!((*task).description) as *mut u8;
    // strncpy(task->description, description, sizeof(task->description) - 1);
    strncpy(desc, description, 256 - 1);
    // task->description[sizeof(task->description) - 1] = '\0';
    *desc.add(255) = 0;
    (*task).priority = priority;

    log_info_str(c"Task added successfully.");
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn print_tasks(manager: *const TaskManager) {
    // printf("Tasks:\n");
    print_stdout(b"Tasks:\n");

    let count = (*manager).task_count;
    let mut i: c_int = 0;
    while i < count {
        let task = (*manager).tasks.add(i as usize);
        // printf("  [%d] %s (Priority: %d)\n", i + 1, description, priority);
        let mut line: Vec<u8> = Vec::new();
        line.extend_from_slice(b"  [");
        line.extend_from_slice(i.wrapping_add(1).to_string().as_bytes());
        line.extend_from_slice(b"] ");
        line.extend_from_slice(&c_str_bytes(
            std::ptr::addr_of!((*task).description) as *const c_char
        ));
        line.extend_from_slice(b" (Priority: ");
        line.extend_from_slice((*task).priority.to_string().as_bytes());
        line.extend_from_slice(b")\n");
        print_stdout(&line);
        i = i.wrapping_add(1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destroy_task_manager(manager: *mut TaskManager) {
    // free(manager->tasks);
    if !(*manager).tasks.is_null() {
        if let Some(layout) = tasks_layout((*manager).max_tasks) {
            dealloc((*manager).tasks as *mut u8, layout);
        }
    }
    // free(manager);
    dealloc(manager as *mut u8, Layout::new::<TaskManager>());

    log_info_str(c"TaskManager destroyed successfully.");
}
