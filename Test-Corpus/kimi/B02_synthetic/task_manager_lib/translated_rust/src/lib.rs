use std::ffi::{CStr, c_char, c_int};
use std::fs::OpenOptions;
use std::io::Write;
use std::os::raw::c_void;
use std::ptr;
use std::sync::Mutex;

static LOG_FILE: Mutex<Option<std::fs::File>> = Mutex::new(None);

#[repr(C)]
pub struct Task {
    description: [c_char; 256],
    priority: c_int,
}

#[repr(C)]
pub struct TaskManager {
    tasks: *mut Task,
    max_tasks: c_int,
    task_count: c_int,
}

#[unsafe(no_mangle)]
pub extern "C" fn initialize_logger() -> c_int {
    let log_file_path = std::env::var("LOG_FILE").unwrap_or_else(|_| "default.log".to_string());
    
    match OpenOptions::new().append(true).create(true).open(&log_file_path) {
        Ok(file) => {
            let mut guard = LOG_FILE.lock().unwrap();
            *guard = Some(file);
            drop(guard);
            log_info(b"Logger initialized.\0".as_ptr() as *const c_char);
            0
        }
        Err(_) => -1,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn log_info(message: *const c_char) {
    let c_str = unsafe { CStr::from_ptr(message) };
    if let Ok(s) = c_str.to_str() {
        let mut guard = LOG_FILE.lock().unwrap();
        if let Some(ref mut file) = *guard {
            let _ = writeln!(file, "[INFO] {}", s);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn log_warning(message: *const c_char) {
    let c_str = unsafe { CStr::from_ptr(message) };
    if let Ok(s) = c_str.to_str() {
        let mut guard = LOG_FILE.lock().unwrap();
        if let Some(ref mut file) = *guard {
            let _ = writeln!(file, "[WARNING] {}", s);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn log_error(message: *const c_char) {
    let c_str = unsafe { CStr::from_ptr(message) };
    if let Ok(s) = c_str.to_str() {
        let mut guard = LOG_FILE.lock().unwrap();
        if let Some(ref mut file) = *guard {
            let _ = writeln!(file, "[ERROR] {}", s);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn finalize_logger() {
    log_info(b"Logger finalized.\0".as_ptr() as *const c_char);
    let mut guard = LOG_FILE.lock().unwrap();
    *guard = None;
}

#[unsafe(no_mangle)]
pub extern "C" fn create_task_manager() -> *mut TaskManager {
    let max_tasks = std::env::var("MAX_TASKS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);
    
    let layout = std::alloc::Layout::new::<TaskManager>();
    let manager = unsafe { std::alloc::alloc(layout) as *mut TaskManager };
    
    if manager.is_null() {
        log_error(b"Failed to allocate memory for TaskManager.\0".as_ptr() as *const c_char);
        return ptr::null_mut();
    }
    
    let tasks_layout = std::alloc::Layout::array::<Task>(max_tasks).unwrap();
    let tasks = unsafe { std::alloc::alloc(tasks_layout) as *mut Task };
    
    if tasks.is_null() {
        log_error(b"Failed to allocate memory for tasks.\0".as_ptr() as *const c_char);
        unsafe { std::alloc::dealloc(manager as *mut u8, layout) };
        return ptr::null_mut();
    }
    
    unsafe {
        (*manager).max_tasks = max_tasks as c_int;
        (*manager).task_count = 0;
        (*manager).tasks = tasks;
    }
    
    log_info(b"TaskManager created successfully.\0".as_ptr() as *const c_char);
    manager
}

#[unsafe(no_mangle)]
pub extern "C" fn add_task(manager: *mut TaskManager, description: *const c_char, priority: c_int) {
    if manager.is_null() {
        return;
    }
    
    unsafe {
        if (*manager).task_count >= (*manager).max_tasks {
            log_warning(b"Cannot add task: Maximum task limit reached.\0".as_ptr() as *const c_char);
            return;
        }
        
        let task = &mut *((*manager).tasks.offset((*manager).task_count as isize));
        (*manager).task_count += 1;
        
        let c_str = CStr::from_ptr(description);
        let bytes = c_str.to_bytes();
        let len = bytes.len().min(255);
        
        ptr::copy_nonoverlapping(bytes.as_ptr() as *const c_char, task.description.as_mut_ptr(), len);
        task.description[len] = 0;
        task.priority = priority;
    }
    
    log_info(b"Task added successfully.\0".as_ptr() as *const c_char);
}

#[unsafe(no_mangle)]
pub extern "C" fn print_tasks(manager: *const TaskManager) {
    if manager.is_null() {
        return;
    }
    
    unsafe {
        println!("Tasks:");
        for i in 0..(*manager).task_count {
            let task = &*((*manager).tasks.offset(i as isize));
            let desc = CStr::from_ptr(task.description.as_ptr());
            println!("  [{}] {} (Priority: {})", i + 1, desc.to_string_lossy(), task.priority);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn destroy_task_manager(manager: *mut TaskManager) {
    if manager.is_null() {
        return;
    }
    
    unsafe {
        let max_tasks = (*manager).max_tasks as usize;
        let tasks_layout = std::alloc::Layout::array::<Task>(max_tasks).unwrap();
        std::alloc::dealloc((*manager).tasks as *mut u8, tasks_layout);
        
        let layout = std::alloc::Layout::new::<TaskManager>();
        std::alloc::dealloc(manager as *mut u8, layout);
    }
    
    log_info(b"TaskManager destroyed successfully.\0".as_ptr() as *const c_char);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(tasks: *const c_char) -> c_int {
    if initialize_logger() != 0 {
        return 1;
    }
    
    let manager = create_task_manager();
    if manager.is_null() {
        finalize_logger();
        return 1;
    }
    
    unsafe {
        let tasks_str = CStr::from_ptr(tasks);
        let tasks_bytes = tasks_str.to_bytes();
        let mut start = 0;
        let mut priority = 1;
        
        while start < tasks_bytes.len() {
            let end = tasks_bytes[start..]
                .iter()
                .position(|&b| b == b'\n')
                .map(|i| start + i)
                .unwrap_or(tasks_bytes.len());
            
            let task_slice = &tasks_bytes[start..end];
            let task_cstring = std::ffi::CString::new(task_slice).unwrap();
            add_task(manager, task_cstring.as_ptr(), priority);
            priority += 1;
            
            start = if end < tasks_bytes.len() && tasks_bytes[end] == b'\n' {
                end + 1
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
