use std::ffi::c_char;
use std::ffi::c_int;
use std::mem;
use std::ptr;

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
        let manager = libc::malloc(mem::size_of::<TaskManager>()) as *mut TaskManager;
        if manager.is_null() {
            log_error(b"Failed to allocate memory for TaskManager.\0".as_ptr() as *const c_char);
            return ptr::null_mut();
        }

        let max_tasks_env = libc::getenv(b"MAX_TASKS\0".as_ptr() as *const c_char);
        let max_tasks = if max_tasks_env.is_null() {
            10
        } else {
            libc::atoi(max_tasks_env as *const c_char)
        };

        (*manager).max_tasks = max_tasks;
        (*manager).task_count = 0;

        let task_size = mem::size_of::<Task>();
        let total = (max_tasks as usize).wrapping_mul(task_size);
        let tasks_ptr = libc::malloc(total) as *mut Task;
        (*manager).tasks = tasks_ptr;

        if tasks_ptr.is_null() {
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
            log_warning(b"Cannot add task: Maximum task limit reached.\0".as_ptr() as *const c_char);
            return;
        }

        let idx = (*manager).task_count;
        (*manager).task_count += 1;
        let task = (*manager).tasks.add(idx as usize);

        // strncpy(task->description, description, sizeof(task->description) - 1);
        let dest = (*task).description.as_mut_ptr();
        libc::strncpy(dest, description, 256 - 1);
        // task->description[sizeof(task->description) - 1] = '\0';
        *dest.add(256 - 1) = 0;
        (*task).priority = priority;

        log_info(b"Task added successfully.\0".as_ptr() as *const c_char);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn print_tasks(manager: *const TaskManager) {
    unsafe {
        libc::printf(b"Tasks:\n\0".as_ptr() as *const c_char);
        let count = (*manager).task_count;
        for i in 0..count {
            let task = (*manager).tasks.add(i as usize);
            libc::printf(
                b"  [%d] %s (Priority: %d)\n\0".as_ptr() as *const c_char,
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
        libc::free((*manager).tasks as *mut libc::c_void);
        libc::free(manager as *mut libc::c_void);
        log_info(b"TaskManager destroyed successfully.\0".as_ptr() as *const c_char);
    }
}
