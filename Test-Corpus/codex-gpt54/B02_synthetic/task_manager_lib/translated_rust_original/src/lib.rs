use libc::{self, FILE};
use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr;

unsafe extern "C" {
    static mut stderr: *mut FILE;
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

static mut LOG_FILE: *mut FILE = ptr::null_mut();

const DEFAULT_LOG_PATH: &[u8] = b"default.log\0";
const APPEND_MODE: &[u8] = b"a\0";
const FAILED_OPEN_LOG_FMT: &[u8] = b"Failed to open log file: %s\n\0";
const INFO_FMT: &[u8] = b"[INFO] %s\n\0";
const WARNING_FMT: &[u8] = b"[WARNING] %s\n\0";
const ERROR_FMT: &[u8] = b"[ERROR] %s\n\0";
const PRINT_TASKS_HEADER: &[u8] = b"Tasks:\n\0";
const PRINT_TASK_FMT: &[u8] = b"  [%d] %s (Priority: %d)\n\0";
const ALLOC_TASK_ERROR: &[u8] = b"Error: Failed to allocate memory for task.\n\0";
const LOGGER_INITIALIZED: &[u8] = b"Logger initialized.\0";
const LOGGER_FINALIZED: &[u8] = b"Logger finalized.\0";
const TASK_MANAGER_ALLOC_ERROR: &[u8] = b"Failed to allocate memory for TaskManager.\0";
const TASKS_ALLOC_ERROR: &[u8] = b"Failed to allocate memory for tasks.\0";
const TASK_MANAGER_CREATED: &[u8] = b"TaskManager created successfully.\0";
const MAX_TASKS_WARNING: &[u8] = b"Cannot add task: Maximum task limit reached.\0";
const TASK_ADDED: &[u8] = b"Task added successfully.\0";
const TASK_MANAGER_DESTROYED: &[u8] = b"TaskManager destroyed successfully.\0";
const LOG_FILE_ENV: &[u8] = b"LOG_FILE\0";
const MAX_TASKS_ENV: &[u8] = b"MAX_TASKS\0";

#[inline]
fn c_ptr(bytes: &'static [u8]) -> *const c_char {
    bytes.as_ptr().cast()
}

#[unsafe(no_mangle)]
pub extern "C" fn initialize_logger() -> c_int {
    let log_file_env = unsafe { libc::getenv(c_ptr(LOG_FILE_ENV)) };
    let log_file_path = if log_file_env.is_null() {
        c_ptr(DEFAULT_LOG_PATH)
    } else {
        log_file_env
    };

    unsafe {
        LOG_FILE = libc::fopen(log_file_path, c_ptr(APPEND_MODE));
        if LOG_FILE.is_null() {
            libc::fprintf(stderr, c_ptr(FAILED_OPEN_LOG_FMT), log_file_path);
            return -1;
        }
    }

    log_info(c_ptr(LOGGER_INITIALIZED));
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn log_info(message: *const c_char) {
    unsafe {
        if !LOG_FILE.is_null() {
            libc::fprintf(LOG_FILE, c_ptr(INFO_FMT), message);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn log_warning(message: *const c_char) {
    unsafe {
        if !LOG_FILE.is_null() {
            libc::fprintf(LOG_FILE, c_ptr(WARNING_FMT), message);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn log_error(message: *const c_char) {
    unsafe {
        if !LOG_FILE.is_null() {
            libc::fprintf(LOG_FILE, c_ptr(ERROR_FMT), message);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn finalize_logger() {
    unsafe {
        if !LOG_FILE.is_null() {
            log_info(c_ptr(LOGGER_FINALIZED));
            libc::fclose(LOG_FILE);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn create_task_manager() -> *mut TaskManager {
    let manager = unsafe { libc::malloc(size_of::<TaskManager>()) as *mut TaskManager };
    if manager.is_null() {
        log_error(c_ptr(TASK_MANAGER_ALLOC_ERROR));
        return ptr::null_mut();
    }

    let max_tasks_env = unsafe { libc::getenv(c_ptr(MAX_TASKS_ENV)) };
    unsafe {
        (*manager).max_tasks = if max_tasks_env.is_null() {
            10
        } else {
            libc::atoi(max_tasks_env)
        };
        (*manager).task_count = 0;

        let task_bytes = ((*manager).max_tasks as usize).wrapping_mul(size_of::<Task>());
        (*manager).tasks = libc::malloc(task_bytes) as *mut Task;
        if (*manager).tasks.is_null() {
            log_error(c_ptr(TASKS_ALLOC_ERROR));
            libc::free(manager.cast::<c_void>());
            return ptr::null_mut();
        }
    }

    log_info(c_ptr(TASK_MANAGER_CREATED));
    manager
}

#[unsafe(no_mangle)]
pub extern "C" fn add_task(manager: *mut TaskManager, description: *const c_char, priority: c_int) {
    unsafe {
        if (*manager).task_count >= (*manager).max_tasks {
            log_warning(c_ptr(MAX_TASKS_WARNING));
            return;
        }

        let task = (*manager).tasks.add((*manager).task_count as usize);
        (*manager).task_count += 1;
        libc::strncpy(
            (*task).description.as_mut_ptr(),
            description,
            (*task).description.len() - 1,
        );
        (*task).description[(*task).description.len() - 1] = 0;
        (*task).priority = priority;
    }

    log_info(c_ptr(TASK_ADDED));
}

#[unsafe(no_mangle)]
pub extern "C" fn print_tasks(manager: *const TaskManager) {
    unsafe {
        libc::printf(c_ptr(PRINT_TASKS_HEADER));
        let mut i: c_int = 0;
        while i < (*manager).task_count {
            let task = (*manager).tasks.add(i as usize);
            libc::printf(
                c_ptr(PRINT_TASK_FMT),
                i + 1,
                (*task).description.as_ptr(),
                (*task).priority,
            );
            i += 1;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn destroy_task_manager(manager: *mut TaskManager) {
    unsafe {
        libc::free((*manager).tasks.cast::<c_void>());
        libc::free(manager.cast::<c_void>());
    }
    log_info(c_ptr(TASK_MANAGER_DESTROYED));
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(tasks: *const c_char) -> c_int {
    let res = initialize_logger();
    if res != 0 {
        return libc::EXIT_FAILURE;
    }

    let manager = create_task_manager();
    if manager.is_null() {
        return libc::EXIT_FAILURE;
    }

    let mut start = tasks;
    let mut priority: c_int = 1;

    unsafe {
        while *start != 0 {
            let mut end = libc::strchr(start, '\n' as c_int) as *const c_char;
            if end.is_null() {
                end = start.add(libc::strlen(start));
            }

            let length = end.offset_from(start) as usize;
            let task = libc::malloc(length + 1) as *mut c_char;
            if task.is_null() {
                libc::fprintf(stderr, c_ptr(ALLOC_TASK_ERROR));
                destroy_task_manager(manager);
                finalize_logger();
                return libc::EXIT_FAILURE;
            }

            libc::strncpy(task, start, length);
            *task.add(length) = 0;

            add_task(manager, task, priority);
            priority += 1;
            libc::free(task.cast::<c_void>());

            start = if *end == '\n' as c_char {
                end.add(1)
            } else {
                end
            };
        }
    }

    print_tasks(manager);
    destroy_task_manager(manager);
    finalize_logger();

    0
}
