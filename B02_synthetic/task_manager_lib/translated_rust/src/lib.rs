use std::ffi::{c_char, c_int, CStr};
use std::io::Write;
use std::ptr;
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// Logger (mirrors logger.c)
// ---------------------------------------------------------------------------

static LOG_FILE: Mutex<Option<std::fs::File>> = Mutex::new(None);

#[unsafe(no_mangle)]
pub extern "C" fn initialize_logger() -> c_int {
    let path = std::env::var("LOG_FILE").unwrap_or_else(|_| "default.log".to_string());

    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path);

    match file {
        Ok(f) => {
            *LOG_FILE.lock().unwrap() = Some(f);
            log_info(b"Logger initialized.\0".as_ptr() as *const c_char);
            0
        }
        Err(_) => {
            eprintln!("Failed to open log file: {}", path);
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn log_info(message: *const c_char) {
    log_write("INFO", message);
}

#[unsafe(no_mangle)]
pub extern "C" fn log_warning(message: *const c_char) {
    log_write("WARNING", message);
}

#[unsafe(no_mangle)]
pub extern "C" fn log_error(message: *const c_char) {
    log_write("ERROR", message);
}

fn log_write(level: &str, message: *const c_char) {
    let mut guard = LOG_FILE.lock().unwrap();
    if let Some(ref mut f) = *guard {
        let msg = unsafe { CStr::from_ptr(message) }.to_str().unwrap_or("");
        let _ = writeln!(f, "[{}] {}", level, msg);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn finalize_logger() {
    let mut guard = LOG_FILE.lock().unwrap();
    if guard.is_some() {
        // Write final message before closing
        if let Some(ref mut f) = *guard {
            let _ = writeln!(f, "[INFO] Logger finalized.");
        }
        *guard = None; // drops the File, closing it
    }
}

// ---------------------------------------------------------------------------
// Task / TaskManager (mirrors task_manager.c)
// ---------------------------------------------------------------------------

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
pub extern "C" fn create_task_manager() -> *mut TaskManager {
    let max_tasks: c_int = std::env::var("MAX_TASKS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);

    let layout_mgr = std::alloc::Layout::new::<TaskManager>();
    let mgr_ptr = unsafe { std::alloc::alloc_zeroed(layout_mgr) } as *mut TaskManager;
    if mgr_ptr.is_null() {
        log_error(b"Failed to allocate memory for TaskManager.\0".as_ptr() as *const c_char);
        return ptr::null_mut();
    }

    let task_layout = std::alloc::Layout::array::<Task>(max_tasks as usize);
    let tasks_ptr = match task_layout {
        Ok(layout) if layout.size() > 0 => (unsafe { std::alloc::alloc_zeroed(layout) }) as *mut Task,
        _ => ptr::null_mut(),
    };

    if tasks_ptr.is_null() && max_tasks > 0 {
        log_error(b"Failed to allocate memory for tasks.\0".as_ptr() as *const c_char);
        unsafe { std::alloc::dealloc(mgr_ptr as *mut u8, layout_mgr) };
        return ptr::null_mut();
    }

    unsafe {
        (*mgr_ptr).tasks = tasks_ptr;
        (*mgr_ptr).max_tasks = max_tasks;
        (*mgr_ptr).task_count = 0;
    }

    log_info(b"TaskManager created successfully.\0".as_ptr() as *const c_char);
    mgr_ptr
}

#[unsafe(no_mangle)]
pub extern "C" fn add_task(manager: *mut TaskManager, description: *const c_char, priority: c_int) {
    let mgr = unsafe { &mut *manager };

    if mgr.task_count >= mgr.max_tasks {
        log_warning(b"Cannot add task: Maximum task limit reached.\0".as_ptr() as *const c_char);
        return;
    }

    let task = unsafe { &mut *mgr.tasks.add(mgr.task_count as usize) };
    mgr.task_count += 1;

    // Replicate strncpy behaviour: copy up to 255 bytes, then null-terminate at [255]
    let src = unsafe { CStr::from_ptr(description) }.to_bytes();
    let copy_len = src.len().min(255);
    // zero the whole description first (strncpy pads with zeros)
    task.description = [0; 256];
    unsafe {
        ptr::copy_nonoverlapping(src.as_ptr(), task.description.as_mut_ptr() as *mut u8, copy_len);
    }
    task.description[255] = 0;
    task.priority = priority;

    log_info(b"Task added successfully.\0".as_ptr() as *const c_char);
}

#[unsafe(no_mangle)]
pub extern "C" fn print_tasks(manager: *const TaskManager) {
    let mgr = unsafe { &*manager };
    print!("Tasks:\n");
    for i in 0..mgr.task_count as usize {
        let task = unsafe { &*mgr.tasks.add(i) };
        let desc = unsafe { CStr::from_ptr(task.description.as_ptr()) }
            .to_str()
            .unwrap_or("");
        print!("  [{}] {} (Priority: {})\n", i + 1, desc, task.priority);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn destroy_task_manager(manager: *mut TaskManager) {
    let mgr = unsafe { &*manager };
    if !mgr.tasks.is_null() {
        let layout = std::alloc::Layout::array::<Task>(mgr.max_tasks as usize).unwrap();
        if layout.size() > 0 {
            unsafe { std::alloc::dealloc(mgr.tasks as *mut u8, layout) };
        }
    }
    unsafe { std::alloc::dealloc(manager as *mut u8, std::alloc::Layout::new::<TaskManager>()) };
    log_info(b"TaskManager destroyed successfully.\0".as_ptr() as *const c_char);
}

// ---------------------------------------------------------------------------
// Driver (mirrors driver.c)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn driver(tasks: *const c_char) -> c_int {
    let res = initialize_logger();
    if res != 0 {
        return 1; // EXIT_FAILURE
    }

    let manager = create_task_manager();
    if manager.is_null() {
        return 1;
    }

    let tasks_str = unsafe { CStr::from_ptr(tasks) }.to_bytes();
    let mut start = 0usize;
    let mut priority: c_int = 1;

    while start < tasks_str.len() {
        let end = tasks_str[start..]
            .iter()
            .position(|&b| b == b'\n')
            .map(|p| start + p)
            .unwrap_or(tasks_str.len());

        let length = end - start;
        let mut task_buf = vec![0u8; length + 1];
        task_buf[..length].copy_from_slice(&tasks_str[start..end]);
        task_buf[length] = 0;

        add_task(manager, task_buf.as_ptr() as *const c_char, priority);
        priority += 1;

        start = if end < tasks_str.len() && tasks_str[end] == b'\n' {
            end + 1
        } else {
            end
        };
    }

    print_tasks(manager);
    destroy_task_manager(manager);
    finalize_logger();

    0
}
