//! Translation of `c_src/src/driver.c`.

use crate::cstdio::print_stderr;
use crate::logger::{finalize_logger, initialize_logger};
use crate::task_manager::{add_task, create_task_manager, destroy_task_manager, print_tasks};
use std::alloc::{alloc, dealloc, Layout};
use std::ffi::{c_char, c_int};

/// `EXIT_FAILURE`
const EXIT_FAILURE: c_int = 1;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(tasks: *const c_char) -> c_int {
    let res = initialize_logger();
    if res != 0 {
        return EXIT_FAILURE;
    }

    let manager = create_task_manager();
    if manager.is_null() {
        // NOTE: the original returns here without calling finalize_logger();
        // the missing finalisation is preserved on purpose.
        return EXIT_FAILURE;
    }

    let mut start = tasks;
    let mut priority: c_int = 1;
    while *start != 0 {
        // const char *end = strchr(start, '\n');
        // if (end == NULL) end = start + strlen(start);
        let mut end = start;
        while *end != 0 && *end != b'\n' as c_char {
            end = end.add(1);
        }

        // Extract the current task
        let length = end.offset_from(start) as usize;
        let layout = Layout::from_size_align(length + 1, 1).unwrap();
        let task = alloc(layout);
        if task.is_null() {
            print_stderr(b"Error: Failed to allocate memory for task.\n");
            destroy_task_manager(manager);
            finalize_logger();
            return EXIT_FAILURE;
        }
        // strncpy(task, start, length); task[length] = '\0';
        std::ptr::copy_nonoverlapping(start as *const u8, task, length);
        *task.add(length) = 0;

        add_task(manager, task as *const c_char, priority);
        priority = priority.wrapping_add(1);
        dealloc(task, layout);

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
