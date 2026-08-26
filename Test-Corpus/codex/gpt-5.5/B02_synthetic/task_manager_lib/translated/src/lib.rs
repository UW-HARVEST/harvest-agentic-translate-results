use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicPtr, Ordering};

type CFile = c_void;

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

static LOG_FILE: AtomicPtr<CFile> = AtomicPtr::new(null_mut());

unsafe extern "C" {
    static mut stderr: *mut CFile;

    fn getenv(name: *const c_char) -> *mut c_char;
    fn fopen(filename: *const c_char, mode: *const c_char) -> *mut CFile;
    fn fclose(stream: *mut CFile) -> c_int;
    fn fprintf(stream: *mut CFile, format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn atoi(nptr: *const c_char) -> c_int;
    fn strncpy(dest: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
    fn strlen(s: *const c_char) -> usize;
    fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
}

#[inline]
fn cstr(bytes: &'static [u8]) -> *const c_char {
    bytes.as_ptr().cast()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn initialize_logger() -> c_int {
    let log_file_env = unsafe { getenv(cstr(b"LOG_FILE\0")) };
    let log_file_path = if !log_file_env.is_null() {
        log_file_env.cast_const()
    } else {
        cstr(b"default.log\0")
    };

    let file = unsafe { fopen(log_file_path, cstr(b"a\0")) };
    LOG_FILE.store(file, Ordering::SeqCst);
    if file.is_null() {
        unsafe {
            fprintf(
                stderr,
                cstr(b"Failed to open log file: %s\n\0"),
                log_file_path,
            );
        }
        return -1;
    }

    unsafe { log_info(cstr(b"Logger initialized.\0")) };
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn log_info(message: *const c_char) {
    let file = LOG_FILE.load(Ordering::SeqCst);
    if !file.is_null() {
        unsafe {
            fprintf(file, cstr(b"[INFO] %s\n\0"), message);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn log_warning(message: *const c_char) {
    let file = LOG_FILE.load(Ordering::SeqCst);
    if !file.is_null() {
        unsafe {
            fprintf(file, cstr(b"[WARNING] %s\n\0"), message);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn log_error(message: *const c_char) {
    let file = LOG_FILE.load(Ordering::SeqCst);
    if !file.is_null() {
        unsafe {
            fprintf(file, cstr(b"[ERROR] %s\n\0"), message);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn finalize_logger() {
    let file = LOG_FILE.load(Ordering::SeqCst);
    if !file.is_null() {
        unsafe {
            log_info(cstr(b"Logger finalized.\0"));
            fclose(file);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_task_manager() -> *mut TaskManager {
    let manager = unsafe { malloc(size_of::<TaskManager>()).cast::<TaskManager>() };
    if manager.is_null() {
        unsafe { log_error(cstr(b"Failed to allocate memory for TaskManager.\0")) };
        return null_mut();
    }

    let max_tasks_env = unsafe { getenv(cstr(b"MAX_TASKS\0")) };
    unsafe {
        (*manager).max_tasks = if !max_tasks_env.is_null() {
            atoi(max_tasks_env.cast_const())
        } else {
            10
        };
        (*manager).task_count = 0;

        let task_bytes = ((*manager).max_tasks as usize).wrapping_mul(size_of::<Task>());
        (*manager).tasks = malloc(task_bytes).cast::<Task>();
        if (*manager).tasks.is_null() {
            log_error(cstr(b"Failed to allocate memory for tasks.\0"));
            free(manager.cast::<c_void>());
            return null_mut();
        }

        log_info(cstr(b"TaskManager created successfully.\0"));
    }
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
            log_warning(cstr(b"Cannot add task: Maximum task limit reached.\0"));
            return;
        }

        let index = (*manager).task_count as usize;
        (*manager).task_count = (*manager).task_count.wrapping_add(1);
        let task = (*manager).tasks.add(index);
        strncpy(
            (*task).description.as_mut_ptr(),
            description,
            size_of::<[c_char; 256]>() - 1,
        );
        (*task).description[size_of::<[c_char; 256]>() - 1] = 0;
        (*task).priority = priority;

        log_info(cstr(b"Task added successfully.\0"));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn print_tasks(manager: *const TaskManager) {
    unsafe {
        printf(cstr(b"Tasks:\n\0"));
        let mut i: c_int = 0;
        while i < (*manager).task_count {
            let task = (*manager).tasks.add(i as usize);
            printf(
                cstr(b"  [%d] %s (Priority: %d)\n\0"),
                i + 1,
                (*task).description.as_ptr(),
                (*task).priority,
            );
            i += 1;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destroy_task_manager(manager: *mut TaskManager) {
    unsafe {
        free((*manager).tasks.cast::<c_void>());
        free(manager.cast::<c_void>());
        log_info(cstr(b"TaskManager destroyed successfully.\0"));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(tasks: *const c_char) -> c_int {
    const EXIT_FAILURE: c_int = 1;

    let res = unsafe { initialize_logger() };
    if res != 0 {
        return EXIT_FAILURE;
    }

    let manager = unsafe { create_task_manager() };
    if manager.is_null() {
        return EXIT_FAILURE;
    }

    let mut start = tasks;
    let mut priority: c_int = 1;
    unsafe {
        while *start != 0 {
            let mut end = strchr(start, '\n' as c_int);
            if end.is_null() {
                end = start.add(strlen(start)).cast_mut();
            }

            let length = end.offset_from(start) as usize;
            let task = malloc(length + 1).cast::<c_char>();
            if task.is_null() {
                fprintf(
                    stderr,
                    cstr(b"Error: Failed to allocate memory for task.\n\0"),
                );
                destroy_task_manager(manager);
                finalize_logger();
                return EXIT_FAILURE;
            }
            strncpy(task, start, length);
            *task.add(length) = 0;

            add_task(manager, task.cast_const(), priority);
            priority = priority.wrapping_add(1);
            free(task.cast::<c_void>());
            start = if *end == '\n' as c_char {
                end.add(1).cast_const()
            } else {
                end.cast_const()
            };
        }
    }

    unsafe {
        print_tasks(manager);
        destroy_task_manager(manager);
        finalize_logger();
    }

    0
}
