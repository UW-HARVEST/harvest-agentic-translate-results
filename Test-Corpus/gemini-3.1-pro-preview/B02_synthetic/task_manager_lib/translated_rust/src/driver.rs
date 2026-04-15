use std::ffi::{CStr, c_char, c_int};
use crate::logger::{initialize_logger, finalize_logger};
use crate::task_manager::{create_task_manager, add_task, print_tasks, destroy_task_manager};

#[unsafe(no_mangle)]
pub extern "C" fn driver(tasks: *const c_char) -> c_int {
    if tasks.is_null() {
        return 1;
    }

    let res = initialize_logger();
    if res != 0 {
        return 1;
    }

    let manager = create_task_manager();
    if manager.is_null() {
        return 1;
    }

    let tasks_c_str = unsafe { CStr::from_ptr(tasks) };
    let tasks_bytes = tasks_c_str.to_bytes();
    let mut priority = 1;

    let mut start = 0;
    while start < tasks_bytes.len() {
        let mut end = start;
        while end < tasks_bytes.len() && tasks_bytes[end] != b'\n' {
            end += 1;
        }

        let task_slice = &tasks_bytes[start..end];
        
        let c_task = std::ffi::CString::new(task_slice).unwrap_or_default();
        add_task(manager, c_task.as_ptr(), priority);
        priority += 1;

        if end < tasks_bytes.len() && tasks_bytes[end] == b'\n' {
            start = end + 1;
        } else {
            start = end;
        }
    }

    print_tasks(manager);
    destroy_task_manager(manager);
    finalize_logger();

    0
}
