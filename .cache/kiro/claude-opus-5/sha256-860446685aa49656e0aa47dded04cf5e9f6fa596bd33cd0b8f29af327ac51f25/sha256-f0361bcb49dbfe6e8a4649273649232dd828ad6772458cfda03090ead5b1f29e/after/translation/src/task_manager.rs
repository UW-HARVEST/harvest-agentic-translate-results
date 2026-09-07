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

//! Translation of `c_src/src/task_manager.c` and `c_src/include/task_manager.h`.

use core::ffi::{c_char, c_int, c_void};
use core::mem::size_of;
use core::ptr::{self, addr_of, addr_of_mut};

use crate::cbind::{atoi, free, getenv, malloc, printf, strncpy};
use crate::logger::{log_error, log_info, log_warning};

/// ```c
/// typedef struct {
///     char description[256];
///     int priority;
/// } Task;
/// ```
#[repr(C)]
pub struct Task {
    pub description: [c_char; 256],
    pub priority: c_int,
}

/// ```c
/// typedef struct {
///     Task *tasks;
///     int max_tasks;
///     int task_count;
/// } TaskManager;
/// ```
#[repr(C)]
pub struct TaskManager {
    pub tasks: *mut Task,
    pub max_tasks: c_int,
    pub task_count: c_int,
}

// Layout must match the C structs exactly; the ABI depends on it.
const _: () = assert!(size_of::<Task>() == 260);
const _: () = assert!(size_of::<TaskManager>() == 16);

/// `TaskManager *create_task_manager();`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_task_manager() -> *mut TaskManager {
    let manager = malloc(size_of::<TaskManager>()) as *mut TaskManager;
    if manager.is_null() {
        log_error(c"Failed to allocate memory for TaskManager.".as_ptr());
        return ptr::null_mut();
    }

    let max_tasks_env: *const c_char = getenv(c"MAX_TASKS".as_ptr());
    let max_tasks: c_int = if !max_tasks_env.is_null() {
        atoi(max_tasks_env)
    } else {
        10
    };
    addr_of_mut!((*manager).max_tasks).write(max_tasks);
    addr_of_mut!((*manager).task_count).write(0);

    // `manager->max_tasks * sizeof(Task)` in C promotes the (possibly negative)
    // int to size_t and then multiplies with wrap-around; reproduce that.
    let bytes = (max_tasks as isize as usize).wrapping_mul(size_of::<Task>());
    let tasks = malloc(bytes) as *mut Task;
    addr_of_mut!((*manager).tasks).write(tasks);
    if tasks.is_null() {
        log_error(c"Failed to allocate memory for tasks.".as_ptr());
        free(manager as *mut c_void);
        return ptr::null_mut();
    }

    log_info(c"TaskManager created successfully.".as_ptr());
    manager
}

/// `void add_task(TaskManager *manager, const char *description, int priority);`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn add_task(
    manager: *mut TaskManager,
    description: *const c_char,
    priority: c_int,
) {
    if (*manager).task_count >= (*manager).max_tasks {
        log_warning(c"Cannot add task: Maximum task limit reached.".as_ptr());
        return;
    }

    let index = (*manager).task_count;
    (*manager).task_count = index + 1;
    let task: *mut Task = (*manager).tasks.offset(index as isize);

    let desc = addr_of_mut!((*task).description) as *mut c_char;
    strncpy(desc, description, 256 - 1);
    *desc.add(256 - 1) = 0;
    addr_of_mut!((*task).priority).write(priority);

    log_info(c"Task added successfully.".as_ptr());
}

/// `void print_tasks(const TaskManager *manager);`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn print_tasks(manager: *const TaskManager) {
    printf(c"Tasks:\n".as_ptr());
    let mut i: c_int = 0;
    while i < (*manager).task_count {
        let task: *const Task = (*manager).tasks.offset(i as isize);
        printf(
            c"  [%d] %s (Priority: %d)\n".as_ptr(),
            i + 1,
            addr_of!((*task).description) as *const c_char,
            (*task).priority,
        );
        i += 1;
    }
}

/// `void destroy_task_manager(TaskManager *manager);`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destroy_task_manager(manager: *mut TaskManager) {
    free((*manager).tasks as *mut c_void);
    free(manager as *mut c_void);
    log_info(c"TaskManager destroyed successfully.".as_ptr());
}
