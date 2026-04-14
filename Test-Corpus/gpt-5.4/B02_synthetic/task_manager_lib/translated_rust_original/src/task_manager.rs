use crate::logger::{log_error_internal, log_info_internal, log_warning_internal};
use std::env;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::ptr;

#[repr(C)]
pub struct Task {
    pub description: [c_char; 256],
    pub priority: c_int,
}

#[repr(C)]
pub struct TaskManager {
    pub tasks: *mut Task,
    pub max_tasks: c_int,
    pub task_count: c_int,
    storage: Vec<Task>,
}

pub fn create_task_manager_internal() -> *mut TaskManager {
    let max_tasks = env::var("MAX_TASKS")
        .ok()
        .and_then(|s| s.parse::<c_int>().ok())
        .unwrap_or(10);

    let capacity = if max_tasks > 0 { max_tasks as usize } else { 0 };
    let mut storage = Vec::with_capacity(capacity);
    let tasks_ptr = if capacity > 0 {
        storage.as_mut_ptr()
    } else {
        ptr::null_mut()
    };

    let manager = Box::new(TaskManager {
        tasks: tasks_ptr,
        max_tasks,
        task_count: 0,
        storage,
    });

    log_info_internal("TaskManager created successfully.");
    Box::into_raw(manager)
}

pub fn add_task_internal(manager: *mut TaskManager, description: &str, priority: c_int) {
    if manager.is_null() {
        return;
    }

    let manager = unsafe { &mut *manager };
    if manager.task_count >= manager.max_tasks {
        log_warning_internal("Cannot add task: Maximum task limit reached.");
        return;
    }

    let mut task = Task {
        description: [0; 256],
        priority,
    };

    let bytes = description.as_bytes();
    let len = bytes.len().min(255);
    for (i, b) in bytes.iter().take(len).enumerate() {
        task.description[i] = *b as c_char;
    }
    task.description[len] = 0;

    manager.storage.push(task);
    manager.tasks = if manager.storage.is_empty() {
        ptr::null_mut()
    } else {
        manager.storage.as_mut_ptr()
    };
    manager.task_count += 1;

    log_info_internal("Task added successfully.");
}

pub fn print_tasks_internal(manager: *const TaskManager) {
    if manager.is_null() {
        return;
    }

    let manager = unsafe { &*manager };
    println!("Tasks:");
    for (i, task) in manager.storage.iter().enumerate() {
        let desc_ptr = task.description.as_ptr();
        let description = unsafe { CStr::from_ptr(desc_ptr) }.to_string_lossy();
        println!("  [{}] {} (Priority: {})", i + 1, description, task.priority);
    }
}

pub fn destroy_task_manager_internal(manager: *mut TaskManager) {
    if manager.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(manager));
    }
    log_info_internal("TaskManager destroyed successfully.");
}

#[unsafe(no_mangle)]
pub extern "C" fn create_task_manager() -> *mut TaskManager {
    create_task_manager_internal()
}

#[unsafe(no_mangle)]
pub extern "C" fn add_task(manager: *mut TaskManager, description: *const c_char, priority: c_int) {
    if manager.is_null() || description.is_null() {
        return;
    }
    let description = unsafe { CStr::from_ptr(description) }.to_string_lossy();
    add_task_internal(manager, &description, priority);
}

#[unsafe(no_mangle)]
pub extern "C" fn print_tasks(manager: *const TaskManager) {
    print_tasks_internal(manager);
}

#[unsafe(no_mangle)]
pub extern "C" fn destroy_task_manager(manager: *mut TaskManager) {
    destroy_task_manager_internal(manager);
}
