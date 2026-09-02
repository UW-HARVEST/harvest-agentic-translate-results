/*
 * Copyright 2025 MIT Lincoln Laboratory
 * Permission is hereby granted, free of charge,
 * to any person obtaining a copy of this software
 * and associated documentation files (the "Software"),
 * to deal in the Software without restriction,
 * including without limitation the rights to use, copy,
 * modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software,
 * and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice
 * shall be included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
 * EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
 * THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
 * IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
 * FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
 * TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
 * OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

//! Translation of `c_src/src/driver.c`.

use core::ffi::{c_char, c_int, c_void};

use crate::cbind::{fprintf, free, malloc, strchr, strlen, strncpy, EXIT_FAILURE};
use crate::logger::{finalize_logger, initialize_logger};
use crate::task_manager::{add_task, create_task_manager, destroy_task_manager, print_tasks};

/// `int driver(const char *tasks);`
///
/// Splits `tasks` on `'\n'` and registers every line - including empty ones -
/// with an incrementing priority starting at 1, then prints the task list.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(tasks: *const c_char) -> c_int {
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
        let mut end: *const c_char = strchr(start, b'\n' as c_int);
        if end.is_null() {
            end = start.add(strlen(start));
        }

        // Extract the current task
        let length = end.offset_from(start) as usize;
        let task = malloc(length + 1) as *mut c_char;
        if task.is_null() {
            fprintf(
                crate::cbind::stderr,
                c"Error: Failed to allocate memory for task.\n".as_ptr(),
            );
            destroy_task_manager(manager);
            finalize_logger();
            return EXIT_FAILURE;
        }
        strncpy(task, start, length);
        *task.add(length) = 0;

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
