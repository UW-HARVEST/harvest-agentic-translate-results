use core::ffi::{c_char, c_int};
use core::ptr;

use crate::logger::{log_error, log_info, log_warning};

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

#[unsafe(no_mangle)]
pub extern "C" fn create_task_manager() -> *mut TaskManager {
    unsafe {
        let manager = libc::malloc(core::mem::size_of::<TaskManager>()) as *mut TaskManager;
        if manager.is_null() {
            let msg = b"Failed to allocate memory for TaskManager.\0".as_ptr() as *const c_char;
            log_error(msg);
            return ptr::null_mut();
        }

        let max_tasks_env_name = b"MAX_TASKS\0".as_ptr() as *const c_char;
        let max_tasks_env = libc::getenv(max_tasks_env_name);
        let max_tasks: c_int = if max_tasks_env.is_null() {
            10
        } else {
            libc::atoi(max_tasks_env)
        };

        (*manager).max_tasks = max_tasks;
        (*manager).task_count = 0;

        let bytes = (max_tasks as usize).wrapping_mul(core::mem::size_of::<Task>());
        let tasks_ptr = libc::malloc(bytes) as *mut Task;
        (*manager).tasks = tasks_ptr;
        if tasks_ptr.is_null() {
            let msg = b"Failed to allocate memory for tasks.\0".as_ptr() as *const c_char;
            log_error(msg);
            libc::free(manager as *mut libc::c_void);
            return ptr::null_mut();
        }

        let msg = b"TaskManager created successfully.\0".as_ptr() as *const c_char;
        log_info(msg);
        manager
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn add_task(
    manager: *mut TaskManager,
    description: *const c_char,
    priority: c_int,
) {
    unsafe {
        if (*manager).task_count >= (*manager).max_tasks {
            let msg = b"Cannot add task: Maximum task limit reached.\0".as_ptr() as *const c_char;
            log_warning(msg);
            return;
        }

        let idx = (*manager).task_count;
        (*manager).task_count += 1;

        let task: *mut Task = (*manager).tasks.add(idx as usize);
        let desc_buf = (*task).description.as_mut_ptr();
        let cap = core::mem::size_of_val(&(*task).description); // 256
        // strncpy(task->description, description, sizeof(task->description) - 1);
        libc::strncpy(desc_buf, description, cap - 1);
        // task->description[sizeof(task->description) - 1] = '\0';
        *desc_buf.add(cap - 1) = 0;
        (*task).priority = priority;

        let msg = b"Task added successfully.\0".as_ptr() as *const c_char;
        log_info(msg);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn print_tasks(manager: *const TaskManager) {
    unsafe {
        let header = b"Tasks:\n\0".as_ptr() as *const c_char;
        libc::printf(header);
        let count = (*manager).task_count;
        let mut i: c_int = 0;
        while i < count {
            let task: *const Task = (*manager).tasks.add(i as usize);
            let fmt = b"  [%d] %s (Priority: %d)\n\0".as_ptr() as *const c_char;
            libc::printf(
                fmt,
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
        libc::free((*manager).tasks as *mut libc::c_void);
        libc::free(manager as *mut libc::c_void);
        let msg = b"TaskManager destroyed successfully.\0".as_ptr() as *const c_char;
        log_info(msg);
    }
}
