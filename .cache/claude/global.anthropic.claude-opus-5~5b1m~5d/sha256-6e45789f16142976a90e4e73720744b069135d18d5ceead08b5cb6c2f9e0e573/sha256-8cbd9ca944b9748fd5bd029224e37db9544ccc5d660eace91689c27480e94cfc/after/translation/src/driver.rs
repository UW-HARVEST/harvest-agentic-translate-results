//! Translation of `c_src/src/driver.c`.

use core::ffi::{c_char, c_int, c_void};

use crate::cstd;
use crate::logger::{finalize_logger, initialize_logger};
use crate::task_manager::{add_task, create_task_manager, destroy_task_manager, print_tasks};

/// `EXIT_FAILURE` from `<stdlib.h>`.
const EXIT_FAILURE: c_int = 1;

/// ```c
/// int driver(const char *tasks);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn driver(tasks: *const c_char) -> c_int {
    let res = initialize_logger();
    if res != 0 {
        return EXIT_FAILURE;
    }

    let manager = create_task_manager();
    if manager.is_null() {
        return EXIT_FAILURE;
    }

    unsafe {
        let mut start: *const c_char = tasks;
        let mut priority: c_int = 1;
        while *start != 0 {
            let mut end: *const c_char = cstd::strchr(start, b'\n' as c_int) as *const c_char;
            if end.is_null() {
                end = start.add(cstd::strlen(start));
            }

            // Extract the current task
            let length: usize = (end as usize).wrapping_sub(start as usize);
            let task = cstd::malloc(length.wrapping_add(1)) as *mut c_char;
            if task.is_null() {
                cstd::c_fprintf(
                    cstd::stderr,
                    c"Error: Failed to allocate memory for task.\n".as_ptr(),
                );
                destroy_task_manager(manager);
                finalize_logger();
                return EXIT_FAILURE;
            }
            cstd::strncpy(task, start, length);
            *task.add(length) = 0;

            add_task(manager, task as *const c_char, priority);
            priority = priority.wrapping_add(1);
            cstd::free(task as *mut c_void);
            start = if *end == b'\n' as c_char {
                end.add(1)
            } else {
                end
            };
        }
    }

    print_tasks(manager);

    destroy_task_manager(manager);
    finalize_logger();

    0
}
