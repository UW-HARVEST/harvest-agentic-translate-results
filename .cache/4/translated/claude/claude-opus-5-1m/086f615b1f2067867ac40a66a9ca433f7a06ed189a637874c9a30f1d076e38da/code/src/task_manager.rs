//! Translation of `c_src/src/task_manager.c`
//! (public API: `c_src/include/task_manager.h`).

use std::ffi::{c_char, c_int, c_void};
use std::ptr;

use crate::cffi::{atoi, free, getenv, malloc, printf, strncpy};
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

// Layout must match the C structs exactly (x86-64 / LP64): 256 + 4 = 260,
// and 8 + 4 + 4 = 16.
const _: () = assert!(size_of::<Task>() == 260);
const _: () = assert!(align_of::<Task>() == 4);
const _: () = assert!(size_of::<TaskManager>() == 16);
const _: () = assert!(align_of::<TaskManager>() == 8);

/// ```c
/// TaskManager *create_task_manager() {
///     TaskManager *manager = (TaskManager *)malloc(sizeof(TaskManager));
///     if (!manager) {
///         log_error("Failed to allocate memory for TaskManager.");
///         return NULL;
///     }
///
///     const char *max_tasks_env = getenv("MAX_TASKS");
///     manager->max_tasks = max_tasks_env ? atoi(max_tasks_env) : 10;
///     manager->task_count = 0;
///     manager->tasks = (Task *)malloc(manager->max_tasks * sizeof(Task));
///     if (!manager->tasks) {
///         log_error("Failed to allocate memory for tasks.");
///         free(manager);
///         return NULL;
///     }
///
///     log_info("TaskManager created successfully.");
///     return manager;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn create_task_manager() -> *mut TaskManager {
    unsafe {
        let manager = malloc(size_of::<TaskManager>()) as *mut TaskManager;
        if manager.is_null() {
            log_error(c"Failed to allocate memory for TaskManager.".as_ptr());
            return ptr::null_mut();
        }

        let max_tasks_env: *const c_char = getenv(c"MAX_TASKS".as_ptr());
        let max_tasks: c_int = if !max_tasks_env.is_null() {
            atoi(max_tasks_env)
        } else {
            10
        };
        (*manager).max_tasks = max_tasks;
        (*manager).task_count = 0;

        // `manager->max_tasks * sizeof(Task)`: in C the `int` operand is
        // converted to `size_t`, so a negative MAX_TASKS sign-extends and the
        // multiplication wraps modulo 2^64. Reproduce that precisely.
        let bytes = (max_tasks as isize as usize).wrapping_mul(size_of::<Task>());
        (*manager).tasks = malloc(bytes) as *mut Task;
        if (*manager).tasks.is_null() {
            log_error(c"Failed to allocate memory for tasks.".as_ptr());
            free(manager as *mut c_void);
            return ptr::null_mut();
        }

        log_info(c"TaskManager created successfully.".as_ptr());
        manager
    }
}

/// ```c
/// void add_task(TaskManager *manager, const char *description, int priority) {
///     if (manager->task_count >= manager->max_tasks) {
///         log_warning("Cannot add task: Maximum task limit reached.");
///         return;
///     }
///
///     Task *task = &manager->tasks[manager->task_count++];
///     strncpy(task->description, description, sizeof(task->description) - 1);
///     task->description[sizeof(task->description) - 1] = '\0';
///     task->priority = priority;
///
///     log_info("Task added successfully.");
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn add_task(manager: *mut TaskManager, description: *const c_char, priority: c_int) {
    unsafe {
        if (*manager).task_count >= (*manager).max_tasks {
            log_warning(c"Cannot add task: Maximum task limit reached.".as_ptr());
            return;
        }

        // post-increment: index is the old value of task_count
        let index = (*manager).task_count;
        (*manager).task_count = index.wrapping_add(1);

        let task: *mut Task = (*manager).tasks.offset(index as isize);
        let desc: *mut c_char = (&raw mut (*task).description).cast::<c_char>();
        strncpy(desc, description, 256 - 1);
        *desc.add(256 - 1) = 0;
        (*task).priority = priority;

        log_info(c"Task added successfully.".as_ptr());
    }
}

/// ```c
/// void print_tasks(const TaskManager *manager) {
///     printf("Tasks:\n");
///     for (int i = 0; i < manager->task_count; i++) {
///         printf("  [%d] %s (Priority: %d)\n", i + 1,
///                manager->tasks[i].description, manager->tasks[i].priority);
///     }
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn print_tasks(manager: *const TaskManager) {
    unsafe {
        printf(c"Tasks:\n".as_ptr());

        let mut i: c_int = 0;
        while i < (*manager).task_count {
            let task: *const Task = (*manager).tasks.offset(i as isize);
            let desc: *const c_char = (&raw const (*task).description).cast::<c_char>();
            printf(
                c"  [%d] %s (Priority: %d)\n".as_ptr(),
                i.wrapping_add(1),
                desc,
                (*task).priority,
            );
            i = i.wrapping_add(1);
        }
    }
}

/// ```c
/// void destroy_task_manager(TaskManager *manager) {
///     free(manager->tasks);
///     free(manager);
///     log_info("TaskManager destroyed successfully.");
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn destroy_task_manager(manager: *mut TaskManager) {
    unsafe {
        free((*manager).tasks as *mut c_void);
        free(manager as *mut c_void);
        log_info(c"TaskManager destroyed successfully.".as_ptr());
    }
}
