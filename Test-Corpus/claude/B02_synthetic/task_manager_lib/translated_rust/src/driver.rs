use core::ffi::{c_char, c_int};

use crate::logger::{finalize_logger, initialize_logger};
use crate::task_manager::{add_task, create_task_manager, destroy_task_manager, print_tasks};

// Mirror EXIT_FAILURE from <stdlib.h>. On all common Unix and Windows targets
// EXIT_FAILURE is 1.
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
            // const char *end = strchr(start, '\n');
            let mut end: *const c_char = libc::strchr(start, b'\n' as c_int);
            if end.is_null() {
                end = start.add(libc::strlen(start));
            }

            // size_t length = end - start;
            let length = (end as usize) - (start as usize);

            // char *task = (char *)malloc(length + 1);
            let task = libc::malloc(length + 1) as *mut c_char;
            if task.is_null() {
                let fmt = b"Error: Failed to allocate memory for task.\n\0".as_ptr() as *const c_char;
                libc::fprintf(stderr_ptr(), fmt);
                destroy_task_manager(manager);
                finalize_logger();
                return EXIT_FAILURE;
            }

            // strncpy(task, start, length);
            libc::strncpy(task, start, length);
            // task[length] = '\0';
            *task.add(length) = 0;

            add_task(manager, task, priority);
            priority += 1;

            libc::free(task as *mut libc::c_void);

            // start = (*end == '\n') ? end + 1 : end;
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

fn stderr_ptr() -> *mut libc::FILE {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    unsafe {
        extern "C" {
            static mut stderr: *mut libc::FILE;
        }
        return stderr;
    }
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    unsafe {
        extern "C" {
            static mut __stderrp: *mut libc::FILE;
        }
        return __stderrp;
    }
    #[cfg(not(any(
        target_os = "linux",
        target_os = "android",
        target_os = "macos",
        target_os = "ios"
    )))]
    unsafe {
        let mode = b"w\0".as_ptr() as *const core::ffi::c_char;
        libc::fdopen(2, mode)
    }
}
