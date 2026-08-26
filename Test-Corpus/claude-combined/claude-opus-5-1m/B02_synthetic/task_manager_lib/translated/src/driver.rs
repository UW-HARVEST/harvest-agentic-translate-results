// Copyright 2025 MIT Lincoln Laboratory
// SPDX-License-Identifier: MIT

use core::ffi::{c_char, c_int};

use crate::logger::{finalize_logger, initialize_logger};
use crate::task_manager::{add_task, create_task_manager, destroy_task_manager, print_tasks};

// EXIT_FAILURE per stdlib.h
const EXIT_FAILURE: c_int = 1;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(tasks: *const c_char) -> c_int {
    let res = unsafe { initialize_logger() };
    if res != 0 {
        return EXIT_FAILURE;
    }

    let manager = unsafe { create_task_manager() };
    if manager.is_null() {
        return EXIT_FAILURE;
    }

    let mut start: *const c_char = tasks;
    let mut priority: c_int = 1;
    while unsafe { *start } != 0 {
        let mut end: *const c_char = unsafe { libc::strchr(start, b'\n' as c_int) };
        if end.is_null() {
            end = unsafe { start.add(libc::strlen(start)) };
        }

        // Extract the current task
        let length: usize = unsafe { end.offset_from(start) as usize };
        let task = unsafe { libc::malloc(length + 1) as *mut c_char };
        if task.is_null() {
            // fprintf(stderr, "Error: Failed to allocate memory for task.\n");
            unsafe {
                let stderr = stderr_handle();
                let msg = b"Error: Failed to allocate memory for task.\n\0".as_ptr()
                    as *const c_char;
                libc::fprintf(stderr, msg);
                destroy_task_manager(manager);
                finalize_logger();
            }
            return EXIT_FAILURE;
        }
        unsafe {
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
    }

    unsafe {
        print_tasks(manager);
        destroy_task_manager(manager);
        finalize_logger();
    }

    0
}

unsafe fn stderr_handle() -> *mut libc::FILE {
    extern "C" {
        static stderr: *mut libc::FILE;
    }
    unsafe { stderr }
}
