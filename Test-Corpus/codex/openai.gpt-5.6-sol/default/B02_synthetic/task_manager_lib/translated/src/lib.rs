use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr;

type CFile = c_void;

unsafe extern "C" {
    static mut stderr: *mut CFile;

    fn atoi(value: *const c_char) -> c_int;
    fn fclose(stream: *mut CFile) -> c_int;
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut CFile;
    fn fprintf(stream: *mut CFile, format: *const c_char, ...) -> c_int;
    fn free(pointer: *mut c_void);
    fn getenv(name: *const c_char) -> *mut c_char;
    fn malloc(size: usize) -> *mut c_void;
    fn printf(format: *const c_char, ...) -> c_int;
    fn strchr(value: *const c_char, character: c_int) -> *mut c_char;
    fn strlen(value: *const c_char) -> usize;
    fn strncpy(destination: *mut c_char, source: *const c_char, count: usize) -> *mut c_char;
}

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

static mut LOG_FILE: *mut CFile = ptr::null_mut();

const LOG_FILE_ENV: &[u8] = b"LOG_FILE\0";
const DEFAULT_LOG_FILE: &[u8] = b"default.log\0";
const APPEND_MODE: &[u8] = b"a\0";
const LOGGER_INITIALIZED: &[u8] = b"Logger initialized.\0";
const LOGGER_FINALIZED: &[u8] = b"Logger finalized.\0";
const TASK_MANAGER_CREATED: &[u8] = b"TaskManager created successfully.\0";
const TASK_MANAGER_ALLOC_FAILED: &[u8] =
    b"Failed to allocate memory for TaskManager.\0";
const TASKS_ALLOC_FAILED: &[u8] = b"Failed to allocate memory for tasks.\0";
const TASK_ADDED: &[u8] = b"Task added successfully.\0";
const TASK_LIMIT_REACHED: &[u8] = b"Cannot add task: Maximum task limit reached.\0";
const TASK_MANAGER_DESTROYED: &[u8] = b"TaskManager destroyed successfully.\0";
const MAX_TASKS_ENV: &[u8] = b"MAX_TASKS\0";

#[inline]
fn c_ptr(value: &'static [u8]) -> *const c_char {
    value.as_ptr().cast()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn initialize_logger() -> c_int {
    let log_file_env = unsafe { getenv(c_ptr(LOG_FILE_ENV)) };
    let log_file_path = if log_file_env.is_null() {
        c_ptr(DEFAULT_LOG_FILE)
    } else {
        log_file_env.cast_const()
    };

    unsafe {
        LOG_FILE = fopen(log_file_path, c_ptr(APPEND_MODE));
        if LOG_FILE.is_null() {
            fprintf(
                stderr,
                c_ptr(b"Failed to open log file: %s\n\0"),
                log_file_path,
            );
            return -1;
        }
    }

    unsafe { log_info(c_ptr(LOGGER_INITIALIZED)) };
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn log_info(message: *const c_char) {
    unsafe {
        if !LOG_FILE.is_null() {
            fprintf(LOG_FILE, c_ptr(b"[INFO] %s\n\0"), message);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn log_warning(message: *const c_char) {
    unsafe {
        if !LOG_FILE.is_null() {
            fprintf(LOG_FILE, c_ptr(b"[WARNING] %s\n\0"), message);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn log_error(message: *const c_char) {
    unsafe {
        if !LOG_FILE.is_null() {
            fprintf(LOG_FILE, c_ptr(b"[ERROR] %s\n\0"), message);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn finalize_logger() {
    unsafe {
        if !LOG_FILE.is_null() {
            log_info(c_ptr(LOGGER_FINALIZED));
            fclose(LOG_FILE);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_task_manager() -> *mut TaskManager {
    let manager = unsafe { malloc(size_of::<TaskManager>()) }.cast::<TaskManager>();
    if manager.is_null() {
        unsafe { log_error(c_ptr(TASK_MANAGER_ALLOC_FAILED)) };
        return ptr::null_mut();
    }

    let max_tasks_env = unsafe { getenv(c_ptr(MAX_TASKS_ENV)) };
    unsafe {
        (*manager).max_tasks = if max_tasks_env.is_null() {
            10
        } else {
            atoi(max_tasks_env)
        };
        (*manager).task_count = 0;
        let allocation_size = ((*manager).max_tasks as usize).wrapping_mul(size_of::<Task>());
        (*manager).tasks = malloc(allocation_size).cast::<Task>();
        if (*manager).tasks.is_null() {
            log_error(c_ptr(TASKS_ALLOC_FAILED));
            free(manager.cast());
            return ptr::null_mut();
        }
    }

    unsafe { log_info(c_ptr(TASK_MANAGER_CREATED)) };
    manager
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn add_task(
    manager: *mut TaskManager,
    description: *const c_char,
    priority: c_int,
) {
    unsafe {
        if (*manager).task_count >= (*manager).max_tasks {
            log_warning(c_ptr(TASK_LIMIT_REACHED));
            return;
        }

        let index = (*manager).task_count;
        (*manager).task_count = (*manager).task_count.wrapping_add(1);
        let task = (*manager).tasks.offset(index as isize);
        strncpy(
            (*task).description.as_mut_ptr(),
            description,
            (*task).description.len() - 1,
        );
        (*task).description[(*task).description.len() - 1] = 0;
        (*task).priority = priority;
        log_info(c_ptr(TASK_ADDED));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn print_tasks(manager: *const TaskManager) {
    unsafe {
        printf(c_ptr(b"Tasks:\n\0"));
        let mut index: c_int = 0;
        while index < (*manager).task_count {
            let task = (*manager).tasks.offset(index as isize);
            printf(
                c_ptr(b"  [%d] %s (Priority: %d)\n\0"),
                index.wrapping_add(1),
                (*task).description.as_ptr(),
                (*task).priority,
            );
            index = index.wrapping_add(1);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destroy_task_manager(manager: *mut TaskManager) {
    unsafe {
        free((*manager).tasks.cast());
        free(manager.cast());
        log_info(c_ptr(TASK_MANAGER_DESTROYED));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(tasks: *const c_char) -> c_int {
    let result = unsafe { initialize_logger() };
    if result != 0 {
        return 1;
    }

    let manager = unsafe { create_task_manager() };
    if manager.is_null() {
        return 1;
    }

    let mut start = tasks;
    let mut priority: c_int = 1;
    unsafe {
        while *start != 0 {
            let mut end = strchr(start, b'\n' as c_int).cast_const();
            if end.is_null() {
                end = start.add(strlen(start));
            }

            let length = end.offset_from(start) as usize;
            let task = malloc(length.wrapping_add(1)).cast::<c_char>();
            if task.is_null() {
                fprintf(
                    stderr,
                    c_ptr(b"Error: Failed to allocate memory for task.\n\0"),
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
