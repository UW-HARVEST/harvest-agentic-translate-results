// Translation of C library to Rust producing byte-identical output.

use std::ffi::{c_char, c_int, c_void};
use std::sync::atomic::{AtomicPtr, Ordering};

// libc bindings we use directly to preserve C stdio formatting exactly.
extern "C" {
    fn getenv(name: *const c_char) -> *const c_char;
    fn atoi(s: *const c_char) -> c_int;
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strchr(s: *const c_char, c: c_int) -> *const c_char;
    fn strlen(s: *const c_char) -> usize;
    fn strncpy(dst: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fclose(stream: *mut c_void) -> c_int;
    fn fprintf(stream: *mut c_void, format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;

    static stderr: *mut c_void;
}

// Use a thread-unsafe-like global FILE pointer to mirror the C `static FILE *log_file`.
// We store as AtomicPtr<c_void> for interior mutability without unsafe statics.
static LOG_FILE: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

// ---- Task / TaskManager structs (must match C layout) ----

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

// ---- Logger ----

#[unsafe(no_mangle)]
pub extern "C" fn initialize_logger() -> c_int {
    unsafe {
        let log_file_env_name = b"LOG_FILE\0".as_ptr() as *const c_char;
        let log_file_env = getenv(log_file_env_name);

        let default_path = b"default.log\0".as_ptr() as *const c_char;
        let log_file_path = if !log_file_env.is_null() {
            log_file_env
        } else {
            default_path
        };

        let mode = b"a\0".as_ptr() as *const c_char;
        let f = fopen(log_file_path, mode);
        if f.is_null() {
            let fmt = b"Failed to open log file: %s\n\0".as_ptr() as *const c_char;
            fprintf(stderr, fmt, log_file_path);
            return -1;
        }

        LOG_FILE.store(f, Ordering::SeqCst);

        let msg = b"Logger initialized.\0".as_ptr() as *const c_char;
        log_info(msg);
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn log_info(message: *const c_char) {
    unsafe {
        let f = LOG_FILE.load(Ordering::SeqCst);
        if !f.is_null() {
            let fmt = b"[INFO] %s\n\0".as_ptr() as *const c_char;
            fprintf(f, fmt, message);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn log_warning(message: *const c_char) {
    unsafe {
        let f = LOG_FILE.load(Ordering::SeqCst);
        if !f.is_null() {
            let fmt = b"[WARNING] %s\n\0".as_ptr() as *const c_char;
            fprintf(f, fmt, message);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn log_error(message: *const c_char) {
    unsafe {
        let f = LOG_FILE.load(Ordering::SeqCst);
        if !f.is_null() {
            let fmt = b"[ERROR] %s\n\0".as_ptr() as *const c_char;
            fprintf(f, fmt, message);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn finalize_logger() {
    unsafe {
        let f = LOG_FILE.load(Ordering::SeqCst);
        if !f.is_null() {
            let msg = b"Logger finalized.\0".as_ptr() as *const c_char;
            log_info(msg);
            fclose(f);
            // Note: original C did not reset the static pointer, but to avoid
            // double-free if called twice we mirror C exactly (don't reset).
        }
    }
}

// ---- Task Manager ----

#[unsafe(no_mangle)]
pub extern "C" fn create_task_manager() -> *mut TaskManager {
    unsafe {
        let manager = malloc(std::mem::size_of::<TaskManager>()) as *mut TaskManager;
        if manager.is_null() {
            let msg = b"Failed to allocate memory for TaskManager.\0".as_ptr() as *const c_char;
            log_error(msg);
            return std::ptr::null_mut();
        }

        let max_tasks_env_name = b"MAX_TASKS\0".as_ptr() as *const c_char;
        let max_tasks_env = getenv(max_tasks_env_name);
        let max_tasks: c_int = if !max_tasks_env.is_null() {
            atoi(max_tasks_env)
        } else {
            10
        };

        (*manager).max_tasks = max_tasks;
        (*manager).task_count = 0;

        let tasks_size = (max_tasks as usize).wrapping_mul(std::mem::size_of::<Task>());
        let tasks_ptr = malloc(tasks_size) as *mut Task;
        (*manager).tasks = tasks_ptr;
        if tasks_ptr.is_null() {
            let msg = b"Failed to allocate memory for tasks.\0".as_ptr() as *const c_char;
            log_error(msg);
            free(manager as *mut c_void);
            return std::ptr::null_mut();
        }

        let msg = b"TaskManager created successfully.\0".as_ptr() as *const c_char;
        log_info(msg);
        manager
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn add_task(manager: *mut TaskManager, description: *const c_char, priority: c_int) {
    unsafe {
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
        // strncpy(task->description, description, sizeof(task->description) - 1);
        strncpy(desc_ptr, description, desc_size - 1);
        // task->description[sizeof(task->description) - 1] = '\0';
        *desc_ptr.add(desc_size - 1) = 0;
        (*task).priority = priority;

        let msg = b"Task added successfully.\0".as_ptr() as *const c_char;
        log_info(msg);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn print_tasks(manager: *const TaskManager) {
    unsafe {
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
}

#[unsafe(no_mangle)]
pub extern "C" fn destroy_task_manager(manager: *mut TaskManager) {
    unsafe {
        free((*manager).tasks as *mut c_void);
        free(manager as *mut c_void);
        let msg = b"TaskManager destroyed successfully.\0".as_ptr() as *const c_char;
        log_info(msg);
    }
}

// ---- Driver ----

const EXIT_FAILURE: c_int = 1;

#[unsafe(no_mangle)]
pub extern "C" fn driver(tasks: *const c_char) -> c_int {
    unsafe {
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
            let mut end = strchr(start, '\n' as c_int);
            if end.is_null() {
                end = start.add(strlen(start));
            }

            // Extract the current task
            let length = end.offset_from(start) as usize;
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
            start = if *end == '\n' as c_char { end.add(1) } else { end };
        }

        print_tasks(manager);

        destroy_task_manager(manager);
        finalize_logger();

        0
    }
}
