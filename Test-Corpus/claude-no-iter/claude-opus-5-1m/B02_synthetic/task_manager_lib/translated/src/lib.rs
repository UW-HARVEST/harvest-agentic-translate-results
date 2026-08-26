// Rust translation of the C library in c_src/.
// Goal: produce byte-identical output for the same inputs.

use libc::{
    FILE, atoi, c_char, c_int, c_void, fclose, fopen, fprintf, free, getenv, malloc, printf,
    strchr, strlen, strncpy,
};
use std::sync::atomic::{AtomicPtr, Ordering};

unsafe extern "C" {
    // Standard C stderr stream — declared here so we can pass it to `fprintf`
    // and mirror the C source verbatim.
    static stderr: *mut FILE;
}

// ---------------------------------------------------------------------------
// Public C-compatible types (must match the layouts in c_src/include).
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Logger
// ---------------------------------------------------------------------------

// Equivalent of C's `static FILE *log_file = NULL;`.
static LOG_FILE: AtomicPtr<FILE> = AtomicPtr::new(std::ptr::null_mut());

#[inline]
fn current_log_file() -> *mut FILE {
    LOG_FILE.load(Ordering::SeqCst)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn initialize_logger() -> c_int {
    unsafe {
        let log_file_env = getenv(c"LOG_FILE".as_ptr());
        let log_file_path = if log_file_env.is_null() {
            c"default.log".as_ptr()
        } else {
            log_file_env as *const c_char
        };

        let f = fopen(log_file_path, c"a".as_ptr());
        if f.is_null() {
            fprintf(stderr, c"Failed to open log file: %s\n".as_ptr(), log_file_path);
            return -1;
        }
        LOG_FILE.store(f, Ordering::SeqCst);

        log_info(c"Logger initialized.".as_ptr());
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn log_info(message: *const c_char) {
    let f = current_log_file();
    if !f.is_null() {
        unsafe {
            fprintf(f, c"[INFO] %s\n".as_ptr(), message);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn log_warning(message: *const c_char) {
    let f = current_log_file();
    if !f.is_null() {
        unsafe {
            fprintf(f, c"[WARNING] %s\n".as_ptr(), message);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn log_error(message: *const c_char) {
    let f = current_log_file();
    if !f.is_null() {
        unsafe {
            fprintf(f, c"[ERROR] %s\n".as_ptr(), message);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn finalize_logger() {
    let f = current_log_file();
    if !f.is_null() {
        unsafe {
            // log_info("Logger finalized.");
            fprintf(f, c"[INFO] %s\n".as_ptr(), c"Logger finalized.".as_ptr());
            fclose(f);
        }
    }
}

// ---------------------------------------------------------------------------
// Task manager
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_task_manager() -> *mut TaskManager {
    unsafe {
        let manager = malloc(std::mem::size_of::<TaskManager>()) as *mut TaskManager;
        if manager.is_null() {
            log_error(c"Failed to allocate memory for TaskManager.".as_ptr());
            return std::ptr::null_mut();
        }

        let max_tasks_env = getenv(c"MAX_TASKS".as_ptr());
        let max_tasks: c_int = if max_tasks_env.is_null() {
            10
        } else {
            atoi(max_tasks_env)
        };

        (*manager).max_tasks = max_tasks;
        (*manager).task_count = 0;
        (*manager).tasks =
            malloc((max_tasks as usize).wrapping_mul(std::mem::size_of::<Task>())) as *mut Task;
        if (*manager).tasks.is_null() {
            log_error(c"Failed to allocate memory for tasks.".as_ptr());
            free(manager as *mut c_void);
            return std::ptr::null_mut();
        }

        log_info(c"TaskManager created successfully.".as_ptr());
        manager
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn add_task(
    manager: *mut TaskManager,
    description: *const c_char,
    priority: c_int,
) {
    unsafe {
        if (*manager).task_count >= (*manager).max_tasks {
            log_warning(c"Cannot add task: Maximum task limit reached.".as_ptr());
            return;
        }

        // Task *task = &manager->tasks[manager->task_count++];
        let idx = (*manager).task_count as isize;
        (*manager).task_count += 1;
        let task = (*manager).tasks.offset(idx);

        // strncpy(task->description, description, sizeof(task->description) - 1);
        // task->description[sizeof(task->description) - 1] = '\0';
        strncpy((*task).description.as_mut_ptr(), description, 256 - 1);
        (*task).description[255] = 0;
        (*task).priority = priority;

        log_info(c"Task added successfully.".as_ptr());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn print_tasks(manager: *const TaskManager) {
    unsafe {
        printf(c"Tasks:\n".as_ptr());
        let count = (*manager).task_count;
        let mut i: c_int = 0;
        while i < count {
            let task = (*manager).tasks.offset(i as isize);
            printf(
                c"  [%d] %s (Priority: %d)\n".as_ptr(),
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
        free((*manager).tasks as *mut c_void);
        free(manager as *mut c_void);
        log_info(c"TaskManager destroyed successfully.".as_ptr());
    }
}

// ---------------------------------------------------------------------------
// Driver
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(tasks: *const c_char) -> c_int {
    unsafe {
        let res = initialize_logger();
        if res != 0 {
            return libc::EXIT_FAILURE;
        }

        let manager = create_task_manager();
        if manager.is_null() {
            return libc::EXIT_FAILURE;
        }

        let mut start: *const c_char = tasks;
        let mut priority: c_int = 1;
        while *start != 0 {
            let mut end: *const c_char = strchr(start, b'\n' as c_int);
            if end.is_null() {
                end = start.add(strlen(start));
            }

            // Extract the current task
            let length: usize = (end as usize) - (start as usize);
            let task = malloc(length + 1) as *mut c_char;
            if task.is_null() {
                fprintf(stderr, c"Error: Failed to allocate memory for task.\n".as_ptr());
                destroy_task_manager(manager);
                finalize_logger();
                return libc::EXIT_FAILURE;
            }
            strncpy(task, start, length);
            *task.add(length) = 0;

            // add_task(manager, task, priority++);
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
}
