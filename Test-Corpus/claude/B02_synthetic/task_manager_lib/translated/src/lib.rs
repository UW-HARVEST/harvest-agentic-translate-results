// Translated Rust library replicating the C driver.

use std::ffi::c_char;
use std::ffi::c_int;
use std::ffi::CStr;
use std::os::raw::c_void;
use std::ptr;
use std::sync::Mutex;

// libc bindings used to mimic exact stdio behavior of the C code.
extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fclose(stream: *mut c_void) -> c_int;
    fn fprintf(stream: *mut c_void, format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
    fn getenv(name: *const c_char) -> *const c_char;
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strncpy(dest: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
    fn strchr(s: *const c_char, c: c_int) -> *const c_char;
    fn strlen(s: *const c_char) -> usize;
    fn atoi(s: *const c_char) -> c_int;

    // stderr is a `FILE*` symbol exposed by glibc as a variable.
    static stderr: *mut c_void;
}

const EXIT_FAILURE: c_int = 1;

// ======================================================================
// Task / TaskManager structs matching the C definitions byte-for-byte.
// ======================================================================

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

// ======================================================================
// Logger state.
// The C version uses a `static FILE *log_file = NULL;` variable.
// We protect Rust access using a Mutex; the underlying pointer is opaque.
// ======================================================================

struct LoggerState {
    file: *mut c_void,
}

unsafe impl Send for LoggerState {}

static LOGGER: Mutex<LoggerState> = Mutex::new(LoggerState {
    file: ptr::null_mut(),
});

fn log_file_ptr() -> *mut c_void {
    let guard = LOGGER.lock().unwrap();
    guard.file
}

fn set_log_file(p: *mut c_void) {
    let mut guard = LOGGER.lock().unwrap();
    guard.file = p;
}

// ======================================================================
// Logger functions (mirrors logger.c)
// ======================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn initialize_logger() -> c_int {
    let key = b"LOG_FILE\0".as_ptr() as *const c_char;
    let log_file_env = getenv(key);
    let default = b"default.log\0".as_ptr() as *const c_char;
    let log_file_path = if log_file_env.is_null() {
        default
    } else {
        log_file_env
    };

    let mode = b"a\0".as_ptr() as *const c_char;
    let f = fopen(log_file_path, mode);
    if f.is_null() {
        let fmt = b"Failed to open log file: %s\n\0".as_ptr() as *const c_char;
        fprintf(stderr, fmt, log_file_path);
        return -1;
    }
    set_log_file(f);

    let msg = b"Logger initialized.\0".as_ptr() as *const c_char;
    log_info(msg);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn log_info(message: *const c_char) {
    let f = log_file_ptr();
    if !f.is_null() {
        let fmt = b"[INFO] %s\n\0".as_ptr() as *const c_char;
        fprintf(f, fmt, message);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn log_warning(message: *const c_char) {
    let f = log_file_ptr();
    if !f.is_null() {
        let fmt = b"[WARNING] %s\n\0".as_ptr() as *const c_char;
        fprintf(f, fmt, message);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn log_error(message: *const c_char) {
    let f = log_file_ptr();
    if !f.is_null() {
        let fmt = b"[ERROR] %s\n\0".as_ptr() as *const c_char;
        fprintf(f, fmt, message);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn finalize_logger() {
    let f = log_file_ptr();
    if !f.is_null() {
        let msg = b"Logger finalized.\0".as_ptr() as *const c_char;
        log_info(msg);
        fclose(f);
        set_log_file(ptr::null_mut());
    }
}

// ======================================================================
// TaskManager functions (mirrors task_manager.c)
// ======================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_task_manager() -> *mut TaskManager {
    let manager = malloc(std::mem::size_of::<TaskManager>()) as *mut TaskManager;
    if manager.is_null() {
        let msg = b"Failed to allocate memory for TaskManager.\0".as_ptr() as *const c_char;
        log_error(msg);
        return ptr::null_mut();
    }

    let key = b"MAX_TASKS\0".as_ptr() as *const c_char;
    let max_tasks_env = getenv(key);
    let max_tasks = if max_tasks_env.is_null() {
        10
    } else {
        atoi(max_tasks_env)
    };

    (*manager).max_tasks = max_tasks;
    (*manager).task_count = 0;

    let bytes = (max_tasks as usize).wrapping_mul(std::mem::size_of::<Task>());
    let tasks_ptr = malloc(bytes) as *mut Task;
    (*manager).tasks = tasks_ptr;
    if tasks_ptr.is_null() {
        let msg = b"Failed to allocate memory for tasks.\0".as_ptr() as *const c_char;
        log_error(msg);
        free(manager as *mut c_void);
        return ptr::null_mut();
    }

    let msg = b"TaskManager created successfully.\0".as_ptr() as *const c_char;
    log_info(msg);
    manager
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn add_task(
    manager: *mut TaskManager,
    description: *const c_char,
    priority: c_int,
) {
    if (*manager).task_count >= (*manager).max_tasks {
        let msg = b"Cannot add task: Maximum task limit reached.\0".as_ptr() as *const c_char;
        log_warning(msg);
        return;
    }

    let idx = (*manager).task_count as isize;
    (*manager).task_count += 1;
    let task = (*manager).tasks.offset(idx);
    let desc_ptr = (*task).description.as_mut_ptr();
    let desc_size = (*task).description.len();
    strncpy(desc_ptr, description, desc_size - 1);
    *desc_ptr.add(desc_size - 1) = 0;
    (*task).priority = priority;

    let msg = b"Task added successfully.\0".as_ptr() as *const c_char;
    log_info(msg);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn print_tasks(manager: *const TaskManager) {
    let header = b"Tasks:\n\0".as_ptr() as *const c_char;
    printf(header);
    let count = (*manager).task_count;
    for i in 0..count {
        let task = (*manager).tasks.offset(i as isize);
        let fmt = b"  [%d] %s (Priority: %d)\n\0".as_ptr() as *const c_char;
        printf(
            fmt,
            i + 1,
            (*task).description.as_ptr(),
            (*task).priority,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destroy_task_manager(manager: *mut TaskManager) {
    free((*manager).tasks as *mut c_void);
    free(manager as *mut c_void);
    let msg = b"TaskManager destroyed successfully.\0".as_ptr() as *const c_char;
    log_info(msg);
}

// ======================================================================
// driver function (mirrors driver.c)
// ======================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(tasks: *const c_char) -> c_int {
    let res = initialize_logger();
    if res != 0 {
        return EXIT_FAILURE;
    }

    let manager = create_task_manager();
    if manager.is_null() {
        return EXIT_FAILURE;
    }

    let mut start = tasks;
    let mut priority: c_int = 1;
    while *start != 0 {
        let mut end = strchr(start, b'\n' as c_int);
        if end.is_null() {
            end = start.add(strlen(start));
        }

        let length = (end as usize) - (start as usize);
        let task = malloc(length + 1) as *mut c_char;
        if task.is_null() {
            let fmt = b"Error: Failed to allocate memory for task.\n\0".as_ptr() as *const c_char;
            fprintf(stderr, fmt);
            destroy_task_manager(manager);
            finalize_logger();
            return EXIT_FAILURE;
        }
        strncpy(task, start, length);
        *task.add(length) = 0;

        add_task(manager, task, priority);
        priority += 1;
        free(task as *mut c_void);
        start = if *end == b'\n' as c_char {
            end.add(1)
        } else {
            end
        };
    }

    print_tasks(manager);

    destroy_task_manager(manager);
    finalize_logger();

    0
}

// Suppress unused-import warnings.
#[allow(dead_code)]
fn _suppress_unused() {
    let _ = CStr::from_bytes_with_nul(b"\0");
}
