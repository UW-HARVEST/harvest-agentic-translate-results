mod logger;
mod task_manager;

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use task_manager::{create_task_manager_internal, destroy_task_manager_internal};

#[unsafe(no_mangle)]
pub extern "C" fn driver(tasks: *const c_char) -> c_int {
    let res = logger::initialize_logger_internal();
    if res != 0 {
        return 1;
    }

    if tasks.is_null() {
        destroy_and_finalize_on_error(None);
        return 1;
    }

    let manager = create_task_manager_internal();
    if manager.is_null() {
        logger::finalize_logger_internal();
        return 1;
    }

    let tasks_str = unsafe { CStr::from_ptr(tasks) }.to_string_lossy();
    let mut priority: c_int = 1;

    for task in tasks_str.split('\n') {
        if task.is_empty() {
            continue;
        }
        task_manager::add_task_internal(manager, task, priority);
        priority += 1;
    }

    task_manager::print_tasks_internal(manager);
    destroy_task_manager_internal(manager);
    logger::finalize_logger_internal();

    0
}

fn destroy_and_finalize_on_error(manager: Option<*mut task_manager::TaskManager>) {
    if let Some(manager) = manager {
        if !manager.is_null() {
            destroy_task_manager_internal(manager);
        }
    }
    logger::finalize_logger_internal();
}
