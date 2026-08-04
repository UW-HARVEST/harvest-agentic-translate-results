use std::ffi::{CStr, c_char, c_int};
use std::env;
use crate::logger::{log_info_internal, log_warning_internal, log_error_internal};

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
}

#[unsafe(no_mangle)]
pub extern "C" fn create_task_manager() -> *mut TaskManager {
    let max_tasks = env::var("MAX_TASKS")
        .ok()
        .and_then(|s| s.parse::<c_int>().ok())
        .unwrap_or(10)
        .max(1);

    let tasks_layout = std::alloc::Layout::array::<Task>(max_tasks as usize).unwrap();
    let tasks_ptr = unsafe { std::alloc::alloc_zeroed(tasks_layout) } as *mut Task;

    if tasks_ptr.is_null() {
        log_error_internal("Failed to allocate memory for tasks.");
        return std::ptr::null_mut();
    }

    let manager = Box::new(TaskManager {
        tasks: tasks_ptr,
        max_tasks,
        task_count: 0,
    });

    log_info_internal("TaskManager created successfully.");
    Box::into_raw(manager)
}

#[unsafe(no_mangle)]
pub extern "C" fn add_task(manager: *mut TaskManager, description: *const c_char, priority: c_int) {
    if manager.is_null() { return; }
    let mgr = unsafe { &mut *manager };

    if mgr.task_count >= mgr.max_tasks {
        log_warning_internal("Cannot add task: Maximum task limit reached.");
        return;
    }

    let task_ptr = unsafe { mgr.tasks.add(mgr.task_count as usize) };
    let task = unsafe { &mut *task_ptr };

    if !description.is_null() {
        let c_str = unsafe { CStr::from_ptr(description) };
        let bytes = c_str.to_bytes();
        let len = bytes.len().min(255);
        for i in 0..len {
            task.description[i] = bytes[i] as c_char;
        }
        task.description[len] = 0;
    } else {
        task.description[0] = 0;
    }
    task.priority = priority;
    mgr.task_count += 1;

    log_info_internal("Task added successfully.");
}

#[unsafe(no_mangle)]
pub extern "C" fn print_tasks(manager: *const TaskManager) {
    if manager.is_null() { return; }
    let mgr = unsafe { &*manager };
    println!("Tasks:");
    for i in 0..mgr.task_count {
        let task = unsafe { &*mgr.tasks.add(i as usize) };
        let desc = unsafe { CStr::from_ptr(task.description.as_ptr()) }.to_string_lossy();
        println!("  [{}] {} (Priority: {})", i + 1, desc, task.priority);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn destroy_task_manager(manager: *mut TaskManager) {
    if manager.is_null() { return; }
    let mgr = unsafe { Box::from_raw(manager) };
    if !mgr.tasks.is_null() {
        let tasks_layout = std::alloc::Layout::array::<Task>(mgr.max_tasks as usize).unwrap();
        unsafe { std::alloc::dealloc(mgr.tasks as *mut u8, tasks_layout) };
    }
    log_info_internal("TaskManager destroyed successfully.");
}
