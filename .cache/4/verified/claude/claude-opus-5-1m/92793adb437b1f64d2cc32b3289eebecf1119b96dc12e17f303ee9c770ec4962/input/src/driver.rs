//! Translation of `c_src/src/driver.c`.
//!
//! `driver` has no declaration in a public header but is a non-static
//! definition, so it is part of the shared library's exported ABI:
//! `int driver(const char *tasks);`

use std::ffi::{c_char, c_int, c_void};

use crate::cffi::{EXIT_FAILURE, fprintf, free, malloc, stderr, strchr, strlen, strncpy};
use crate::logger::{finalize_logger, initialize_logger};
use crate::task_manager::{add_task, create_task_manager, destroy_task_manager, print_tasks};

/// ```c
/// int driver(const char *tasks) {
///     int res = initialize_logger();
///     if (res != 0) return EXIT_FAILURE;
///
///     TaskManager *manager = create_task_manager();
///     if (!manager) return EXIT_FAILURE;
///
///     const char *start = tasks;
///     int priority = 1;
///     while (*start != '\0') {
///         const char *end = strchr(start, '\n');
///         if (end == NULL) end = start + strlen(start);
///
///         size_t length = end - start;
///         char *task = (char *)malloc(length + 1);
///         if (!task) {
///             fprintf(stderr, "Error: Failed to allocate memory for task.\n");
///             destroy_task_manager(manager);
///             finalize_logger();
///             return EXIT_FAILURE;
///         }
///         strncpy(task, start, length);
///         task[length] = '\0';
///
///         add_task(manager, task, priority++);
///         free(task);
///         start = (*end == '\n') ? end + 1 : end;
///     }
///
///     print_tasks(manager);
///
///     destroy_task_manager(manager);
///     finalize_logger();
///
///     return 0;
/// }
/// ```
///
/// Note: on the early-return paths the C code leaks (`initialize_logger`
/// succeeded but the logger is never finalised when `create_task_manager`
/// fails). That behaviour, and the exact ordering of every call, is preserved
/// verbatim.
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
            let mut end: *const c_char = strchr(start, b'\n' as c_int) as *const c_char;
            if end.is_null() {
                end = start.add(strlen(start));
            }

            // Extract the current task
            let length: usize = end.offset_from(start) as usize;
            let task = malloc(length + 1) as *mut c_char;
            if task.is_null() {
                fprintf(
                    stderr,
                    c"Error: Failed to allocate memory for task.\n".as_ptr(),
                );
                destroy_task_manager(manager);
                finalize_logger();
                return EXIT_FAILURE;
            }
            strncpy(task, start, length);
            *task.add(length) = 0;

            // `priority++`: pass the old value, then increment.
            add_task(manager, task, priority);
            priority = priority.wrapping_add(1);
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
}
