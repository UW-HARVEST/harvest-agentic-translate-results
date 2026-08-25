use std::ffi::{c_char, c_int, c_void};
use std::ptr;

#[repr(C)]
pub struct Task {
    pub description: [c_char; 256],
    pub priority: c_int,
}

#[repr(C)]
pub struct TaskManager {
    pub tasks: *mut Task,
    pub max_tasks: c_int,
    pub task_count: c_int,
}

unsafe extern "C" {
    static mut stderr: *mut c_void;

    fn atoi(value: *const c_char) -> c_int;
    fn fclose(stream: *mut c_void) -> c_int;
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fprintf(stream: *mut c_void, format: *const c_char, ...) -> c_int;
    fn free(memory: *mut c_void);
    fn getenv(name: *const c_char) -> *mut c_char;
    fn malloc(size: usize) -> *mut c_void;
    fn printf(format: *const c_char, ...) -> c_int;
    fn strchr(value: *const c_char, character: c_int) -> *mut c_char;
    fn strlen(value: *const c_char) -> usize;
    fn strncpy(destination: *mut c_char, source: *const c_char, count: usize) -> *mut c_char;
}

static mut LOG_FILE: *mut c_void = ptr::null_mut();

const LOG_FILE_ENV: &[u8] = b"LOG_FILE\0";
const DEFAULT_LOG_FILE: &[u8] = b"default.log\0";
const APPEND_MODE: &[u8] = b"a\0";
const MAX_TASKS_ENV: &[u8] = b"MAX_TASKS\0";

const LOGGER_INITIALIZED: &[u8] = b"Logger initialized.\0";
const LOGGER_FINALIZED: &[u8] = b"Logger finalized.\0";
const MANAGER_ALLOCATION_FAILED: &[u8] = b"Failed to allocate memory for TaskManager.\0";
const TASKS_ALLOCATION_FAILED: &[u8] = b"Failed to allocate memory for tasks.\0";
const MANAGER_CREATED: &[u8] = b"TaskManager created successfully.\0";
const TASK_LIMIT_REACHED: &[u8] = b"Cannot add task: Maximum task limit reached.\0";
const TASK_ADDED: &[u8] = b"Task added successfully.\0";
const MANAGER_DESTROYED: &[u8] = b"TaskManager destroyed successfully.\0";

/// # Safety
///
/// Calls that access the global logger must be externally synchronized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn initialize_logger() -> c_int {
    let log_file_env = unsafe { getenv(LOG_FILE_ENV.as_ptr().cast()) };
    let log_file_path = if log_file_env.is_null() {
        DEFAULT_LOG_FILE.as_ptr().cast()
    } else {
        log_file_env.cast_const()
    };

    unsafe {
        LOG_FILE = fopen(log_file_path, APPEND_MODE.as_ptr().cast());
        if LOG_FILE.is_null() {
            fprintf(
                stderr,
                c"Failed to open log file: %s\n".as_ptr(),
                log_file_path,
            );
            return -1;
        }
        log_info(LOGGER_INITIALIZED.as_ptr().cast());
    }
    0
}

/// # Safety
///
/// `message` must point to a valid NUL-terminated string. Logger access must be synchronized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn log_info(message: *const c_char) {
    unsafe {
        if !LOG_FILE.is_null() {
            fprintf(LOG_FILE, c"[INFO] %s\n".as_ptr(), message);
        }
    }
}

/// # Safety
///
/// `message` must point to a valid NUL-terminated string. Logger access must be synchronized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn log_warning(message: *const c_char) {
    unsafe {
        if !LOG_FILE.is_null() {
            fprintf(LOG_FILE, c"[WARNING] %s\n".as_ptr(), message);
        }
    }
}

/// # Safety
///
/// `message` must point to a valid NUL-terminated string. Logger access must be synchronized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn log_error(message: *const c_char) {
    unsafe {
        if !LOG_FILE.is_null() {
            fprintf(LOG_FILE, c"[ERROR] %s\n".as_ptr(), message);
        }
    }
}

/// # Safety
///
/// Calls that access the global logger must be externally synchronized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn finalize_logger() {
    unsafe {
        if !LOG_FILE.is_null() {
            log_info(LOGGER_FINALIZED.as_ptr().cast());
            fclose(LOG_FILE);
        }
    }
}

/// # Safety
///
/// Calls that access the global logger must be externally synchronized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_task_manager() -> *mut TaskManager {
    let manager = unsafe { malloc(size_of::<TaskManager>()).cast::<TaskManager>() };
    if manager.is_null() {
        unsafe { log_error(MANAGER_ALLOCATION_FAILED.as_ptr().cast()) };
        return ptr::null_mut();
    }

    let max_tasks_env = unsafe { getenv(MAX_TASKS_ENV.as_ptr().cast()) };
    let max_tasks = if max_tasks_env.is_null() {
        10
    } else {
        unsafe { atoi(max_tasks_env) }
    };
    unsafe {
        (*manager).max_tasks = max_tasks;
        (*manager).task_count = 0;
        let allocation_size = (max_tasks as usize).wrapping_mul(size_of::<Task>());
        (*manager).tasks = malloc(allocation_size).cast::<Task>();
        if (*manager).tasks.is_null() {
            log_error(TASKS_ALLOCATION_FAILED.as_ptr().cast());
            free(manager.cast());
            return ptr::null_mut();
        }
        log_info(MANAGER_CREATED.as_ptr().cast());
    }
    manager
}

/// # Safety
///
/// `manager` must point to a live manager and `description` to a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn add_task(
    manager: *mut TaskManager,
    description: *const c_char,
    priority: c_int,
) {
    unsafe {
        if (*manager).task_count >= (*manager).max_tasks {
            log_warning(TASK_LIMIT_REACHED.as_ptr().cast());
            return;
        }

        let task = (*manager).tasks.add((*manager).task_count as usize);
        (*manager).task_count = (*manager).task_count.wrapping_add(1);
        strncpy(
            (*task).description.as_mut_ptr(),
            description,
            (*task).description.len() - 1,
        );
        (*task).description[(*task).description.len() - 1] = 0;
        (*task).priority = priority;
        log_info(TASK_ADDED.as_ptr().cast());
    }
}

/// # Safety
///
/// `manager` and its task storage must be valid for reads for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn print_tasks(manager: *const TaskManager) {
    unsafe {
        printf(c"Tasks:\n".as_ptr());
        let mut index = 0;
        while index < (*manager).task_count {
            let task = (*manager).tasks.add(index as usize);
            printf(
                c"  [%d] %s (Priority: %d)\n".as_ptr(),
                index + 1,
                (*task).description.as_ptr(),
                (*task).priority,
            );
            index += 1;
        }
    }
}

/// # Safety
///
/// `manager` must be a live pointer returned by `create_task_manager` and not yet destroyed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destroy_task_manager(manager: *mut TaskManager) {
    unsafe {
        free((*manager).tasks.cast());
        free(manager.cast());
        log_info(MANAGER_DESTROYED.as_ptr().cast());
    }
}

/// # Safety
///
/// `tasks` must point to a valid NUL-terminated string. Logger access must be synchronized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(tasks: *const c_char) -> c_int {
    if unsafe { initialize_logger() } != 0 {
        return 1;
    }

    let manager = unsafe { create_task_manager() };
    if manager.is_null() {
        return 1;
    }

    let mut start = tasks;
    let mut priority = 1;
    unsafe {
        while *start != 0 {
            let mut end = strchr(start, b'\n' as c_int).cast_const();
            if end.is_null() {
                end = start.add(strlen(start));
            }

            let length = end.offset_from(start) as usize;
            let task = malloc(length + 1).cast::<c_char>();
            if task.is_null() {
                fprintf(
                    stderr,
                    c"Error: Failed to allocate memory for task.\n".as_ptr(),
                );
                destroy_task_manager(manager);
                finalize_logger();
                return 1;
            }
            strncpy(task, start, length);
            *task.add(length) = 0;

            add_task(manager, task, priority);
            priority = priority.wrapping_add(1);
            free(task.cast());
            start = if *end == b'\n' as c_char {
                end.add(1)
            } else {
                end
            };
        }

        print_tasks(manager);
        destroy_task_manager(manager);
        finalize_logger();
    }
    0
}
