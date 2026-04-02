use std::ffi::{c_char, c_int};
use std::ptr;

extern "C" {
    fn malloc(size: usize) -> *mut u8;
    fn free(ptr: *mut u8);
    fn getenv(name: *const c_char) -> *const c_char;
    fn atoi(s: *const c_char) -> c_int;
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut libc_FILE;
    fn fprintf(stream: *mut libc_FILE, fmt: *const c_char, ...) -> c_int;
    fn fclose(stream: *mut libc_FILE) -> c_int;
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn strncpy(dst: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
    fn strchr(s: *const c_char, c: c_int) -> *const c_char;
    fn strlen(s: *const c_char) -> usize;
    static stderr: *mut libc_FILE;
}

#[repr(C)]
struct libc_FILE {
    _opaque: [u8; 0],
}

// --- Task / TaskManager structs (repr(C), matching C layout) ---

#[repr(C)]
pub struct Task {
    description: [c_char; 256],
    priority: c_int,
}

#[repr(C)]
pub struct TaskManager {
    tasks: *mut Task,
    max_tasks: c_int,
    task_count: c_int,
}

// --- Logger (static FILE*, matching C static) ---

static mut LOG_FILE: *mut libc_FILE = ptr::null_mut();

#[unsafe(no_mangle)]
pub unsafe extern "C" fn initialize_logger() -> c_int {
    let env_name = b"LOG_FILE\0".as_ptr() as *const c_char;
    let log_file_env = getenv(env_name);
    let log_file_path = if log_file_env.is_null() {
        b"default.log\0".as_ptr() as *const c_char
    } else {
        log_file_env
    };

    let mode = b"a\0".as_ptr() as *const c_char;
    LOG_FILE = fopen(log_file_path, mode);
    if LOG_FILE.is_null() {
        fprintf(
            stderr,
            b"Failed to open log file: %s\n\0".as_ptr() as *const c_char,
            log_file_path,
        );
        return -1;
    }

    log_info(b"Logger initialized.\0".as_ptr() as *const c_char);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn log_info(message: *const c_char) {
    if !LOG_FILE.is_null() {
        fprintf(
            LOG_FILE,
            b"[INFO] %s\n\0".as_ptr() as *const c_char,
            message,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn log_warning(message: *const c_char) {
    if !LOG_FILE.is_null() {
        fprintf(
            LOG_FILE,
            b"[WARNING] %s\n\0".as_ptr() as *const c_char,
            message,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn log_error(message: *const c_char) {
    if !LOG_FILE.is_null() {
        fprintf(
            LOG_FILE,
            b"[ERROR] %s\n\0".as_ptr() as *const c_char,
            message,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn finalize_logger() {
    if !LOG_FILE.is_null() {
        log_info(b"Logger finalized.\0".as_ptr() as *const c_char);
        fclose(LOG_FILE);
    }
}

// --- TaskManager functions ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_task_manager() -> *mut TaskManager {
    let manager = malloc(std::mem::size_of::<TaskManager>()) as *mut TaskManager;
    if manager.is_null() {
        log_error(b"Failed to allocate memory for TaskManager.\0".as_ptr() as *const c_char);
        return ptr::null_mut();
    }

    let env_name = b"MAX_TASKS\0".as_ptr() as *const c_char;
    let max_tasks_env = getenv(env_name);
    let max_tasks = if max_tasks_env.is_null() {
        10
    } else {
        atoi(max_tasks_env)
    };

    (*manager).max_tasks = max_tasks;
    (*manager).task_count = 0;
    (*manager).tasks =
        malloc((max_tasks as usize) * std::mem::size_of::<Task>()) as *mut Task;
    if (*manager).tasks.is_null() {
        log_error(b"Failed to allocate memory for tasks.\0".as_ptr() as *const c_char);
        free(manager as *mut u8);
        return ptr::null_mut();
    }

    log_info(b"TaskManager created successfully.\0".as_ptr() as *const c_char);
    manager
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn add_task(
    manager: *mut TaskManager,
    description: *const c_char,
    priority: c_int,
) {
    if (*manager).task_count >= (*manager).max_tasks {
        log_warning(
            b"Cannot add task: Maximum task limit reached.\0".as_ptr() as *const c_char,
        );
        return;
    }

    let idx = (*manager).task_count;
    (*manager).task_count += 1;
    let task = &mut *(*manager).tasks.offset(idx as isize);
    strncpy(
        task.description.as_mut_ptr(),
        description,
        256 - 1,
    );
    task.description[255] = 0;
    task.priority = priority;

    log_info(b"Task added successfully.\0".as_ptr() as *const c_char);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn print_tasks(manager: *const TaskManager) {
    printf(b"Tasks:\n\0".as_ptr() as *const c_char);
    for i in 0..(*manager).task_count {
        let task = &*(*manager).tasks.offset(i as isize);
        printf(
            b"  [%d] %s (Priority: %d)\n\0".as_ptr() as *const c_char,
            i + 1,
            task.description.as_ptr(),
            task.priority,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destroy_task_manager(manager: *mut TaskManager) {
    free((*manager).tasks as *mut u8);
    free(manager as *mut u8);
    log_info(b"TaskManager destroyed successfully.\0".as_ptr() as *const c_char);
}

// --- Driver ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(tasks: *const c_char) -> c_int {
    let res = initialize_logger();
    if res != 0 {
        return 1; // EXIT_FAILURE
    }

    let manager = create_task_manager();
    if manager.is_null() {
        return 1;
    }

    let mut start = tasks;
    let mut priority: c_int = 1;
    while *start != 0 {
        let end = strchr(start, b'\n' as c_int);
        let end = if end.is_null() {
            start.offset(strlen(start) as isize)
        } else {
            end
        };

        let length = end.offset_from(start) as usize;
        let task_buf = malloc(length + 1) as *mut c_char;
        if task_buf.is_null() {
            fprintf(
                stderr,
                b"Error: Failed to allocate memory for task.\n\0".as_ptr() as *const c_char,
            );
            destroy_task_manager(manager);
            finalize_logger();
            return 1;
        }
        strncpy(task_buf, start, length);
        *task_buf.offset(length as isize) = 0;

        add_task(manager, task_buf, priority);
        priority += 1;
        free(task_buf as *mut u8);
        start = if *end == b'\n' as c_char {
            end.offset(1)
        } else {
            end
        };
    }

    print_tasks(manager);

    destroy_task_manager(manager);
    finalize_logger();

    0
}
