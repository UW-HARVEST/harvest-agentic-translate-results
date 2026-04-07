use std::ffi::{c_char, c_int};
use std::ptr;

// ---------------------------------------------------------------------------
// Logger
// ---------------------------------------------------------------------------

static mut LOG_FILE: *mut libc::FILE = ptr::null_mut();

#[unsafe(no_mangle)]
pub extern "C" fn initialize_logger() -> c_int {
    unsafe {
        let env = libc::getenv(b"LOG_FILE\0".as_ptr() as *const c_char);
        let path = if env.is_null() {
            b"default.log\0".as_ptr() as *const c_char
        } else {
            env
        };
        LOG_FILE = libc::fopen(path, b"a\0".as_ptr() as *const c_char);
        if LOG_FILE.is_null() {
            libc::fprintf(
                libc_stderr(),
                b"Failed to open log file: %s\n\0".as_ptr() as *const c_char,
                path,
            );
            return -1;
        }
        log_info(b"Logger initialized.\0".as_ptr() as *const c_char);
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn log_info(message: *const c_char) {
    unsafe {
        if !LOG_FILE.is_null() {
            libc::fprintf(LOG_FILE, b"[INFO] %s\n\0".as_ptr() as *const c_char, message);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn log_warning(message: *const c_char) {
    unsafe {
        if !LOG_FILE.is_null() {
            libc::fprintf(LOG_FILE, b"[WARNING] %s\n\0".as_ptr() as *const c_char, message);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn log_error(message: *const c_char) {
    unsafe {
        if !LOG_FILE.is_null() {
            libc::fprintf(LOG_FILE, b"[ERROR] %s\n\0".as_ptr() as *const c_char, message);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn finalize_logger() {
    unsafe {
        if !LOG_FILE.is_null() {
            log_info(b"Logger finalized.\0".as_ptr() as *const c_char);
            libc::fclose(LOG_FILE);
        }
    }
}

unsafe fn libc_stderr() -> *mut libc::FILE {
    unsafe { libc::fdopen(2, b"w\0".as_ptr() as *const c_char) }
}

// ---------------------------------------------------------------------------
// Task / TaskManager
// ---------------------------------------------------------------------------

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

#[unsafe(no_mangle)]
pub extern "C" fn create_task_manager() -> *mut TaskManager {
    unsafe {
        let manager = libc::malloc(std::mem::size_of::<TaskManager>()) as *mut TaskManager;
        if manager.is_null() {
            log_error(b"Failed to allocate memory for TaskManager.\0".as_ptr() as *const c_char);
            return ptr::null_mut();
        }

        let env = libc::getenv(b"MAX_TASKS\0".as_ptr() as *const c_char);
        let max_tasks = if env.is_null() { 10 } else { libc::atoi(env) };
        (*manager).max_tasks = max_tasks;
        (*manager).task_count = 0;
        (*manager).tasks =
            libc::malloc((max_tasks as usize) * std::mem::size_of::<Task>()) as *mut Task;
        if (*manager).tasks.is_null() {
            log_error(b"Failed to allocate memory for tasks.\0".as_ptr() as *const c_char);
            libc::free(manager as *mut libc::c_void);
            return ptr::null_mut();
        }

        log_info(b"TaskManager created successfully.\0".as_ptr() as *const c_char);
        manager
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn add_task(manager: *mut TaskManager, description: *const c_char, priority: c_int) {
    unsafe {
        if (*manager).task_count >= (*manager).max_tasks {
            log_warning(
                b"Cannot add task: Maximum task limit reached.\0".as_ptr() as *const c_char,
            );
            return;
        }

        let idx = (*manager).task_count;
        (*manager).task_count += 1;
        let task = &mut *(*manager).tasks.offset(idx as isize);
        libc::strncpy(
            task.description.as_mut_ptr(),
            description,
            std::mem::size_of_val(&task.description) - 1,
        );
        task.description[255] = 0;
        task.priority = priority;

        log_info(b"Task added successfully.\0".as_ptr() as *const c_char);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn print_tasks(manager: *const TaskManager) {
    unsafe {
        libc::printf(b"Tasks:\n\0".as_ptr() as *const c_char);
        for i in 0..(*manager).task_count {
            let task = &*(*manager).tasks.offset(i as isize);
            libc::printf(
                b"  [%d] %s (Priority: %d)\n\0".as_ptr() as *const c_char,
                i + 1,
                task.description.as_ptr(),
                task.priority,
            );
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn destroy_task_manager(manager: *mut TaskManager) {
    unsafe {
        libc::free((*manager).tasks as *mut libc::c_void);
        libc::free(manager as *mut libc::c_void);
        log_info(b"TaskManager destroyed successfully.\0".as_ptr() as *const c_char);
    }
}

// ---------------------------------------------------------------------------
// Driver
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn driver(tasks: *const c_char) -> c_int {
    unsafe {
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
        while *start != 0 {
            let end = libc::strchr(start, b'\n' as c_int);
            let end = if end.is_null() {
                start.offset(libc::strlen(start) as isize)
            } else {
                end
            };

            let length = end.offset_from(start) as usize;
            let task_buf = libc::malloc(length + 1) as *mut c_char;
            if task_buf.is_null() {
                libc::fprintf(
                    libc_stderr(),
                    b"Error: Failed to allocate memory for task.\n\0".as_ptr() as *const c_char,
                );
                destroy_task_manager(manager);
                finalize_logger();
                return libc::EXIT_FAILURE;
            }
            libc::strncpy(task_buf, start, length);
            *task_buf.offset(length as isize) = 0;

            add_task(manager, task_buf, priority);
            priority += 1;
            libc::free(task_buf as *mut libc::c_void);
            start = if *end == b'\n' as c_char { end.offset(1) } else { end };
        }

        print_tasks(manager);
        destroy_task_manager(manager);
        finalize_logger();
        0
    }
}
