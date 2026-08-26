use std::ffi::c_char;
use std::ffi::c_int;
use std::ptr;

use crate::logger::{finalize_logger, initialize_logger};
use crate::task_manager::{add_task, create_task_manager, destroy_task_manager, print_tasks};

extern "C" {
    static stderr: *mut libc::FILE;
}

// Mirrors EXIT_FAILURE from <stdlib.h>
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

        let mut start: *const c_char = tasks;
        let mut priority: c_int = 1;
        while *start != 0 {
            let mut end: *const c_char =
                libc::strchr(start, b'\n' as c_int) as *const c_char;
            if end.is_null() {
                end = start.add(libc::strlen(start));
            }

            // Extract the current task
            let length: usize = (end as usize) - (start as usize);
            let task = libc::malloc(length + 1) as *mut c_char;
            if task.is_null() {
                libc::fprintf(
                    stderr,
                    b"Error: Failed to allocate memory for task.\n\0".as_ptr() as *const c_char,
                );
                destroy_task_manager(manager);
                finalize_logger();
                return EXIT_FAILURE;
            }
            libc::strncpy(task, start, length);
            *task.add(length) = 0;

            add_task(manager, task, priority);
            priority += 1;
            libc::free(task as *mut libc::c_void);
            start = if *end == b'\n' as c_char {
                end.add(1)
            } else {
                end
            };
        }

        print_tasks(manager);

        destroy_task_manager(manager);
        finalize_logger();

        // Suppress unused warning for ptr import
        let _ = ptr::null::<c_char>();

        0
    }
}
