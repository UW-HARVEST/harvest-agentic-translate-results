use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, CString};
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn unique_id() -> usize {
    TEST_COUNTER.fetch_add(1, Ordering::SeqCst)
}

fn c_lib_path() -> String {
    std::env::current_dir()
        .unwrap()
        .join("c_src/build/libdriver.so")
        .to_str()
        .unwrap()
        .to_string()
}

fn rust_lib_path() -> String {
    // Find the built Rust cdylib
    let target_dir = std::env::current_dir().unwrap().join("target");
    for profile in &["debug", "release"] {
        let p = target_dir.join(profile).join("libdriver.so");
        if p.exists() {
            return p.to_str().unwrap().to_string();
        }
    }
    panic!("Rust libdriver.so not found in target/debug or target/release");
}

// ---- Logger tests ----

#[test]
fn test_logger_initialize_and_finalize() {
    let id = unique_id();
    let c_log = format!("/tmp/test_c_logger_{}.log", id);
    let r_log = format!("/tmp/test_r_logger_{}.log", id);

    // C version
    unsafe {
        let lib = Library::new(c_lib_path()).unwrap();
        std::env::set_var("LOG_FILE", &c_log);
        let init: Symbol<unsafe extern "C" fn() -> c_int> = lib.get(b"initialize_logger").unwrap();
        let finalize: Symbol<unsafe extern "C" fn()> = lib.get(b"finalize_logger").unwrap();
        let ret = init();
        assert_eq!(ret, 0, "C initialize_logger should return 0");
        finalize();
    }

    // Rust version
    unsafe {
        let lib = Library::new(rust_lib_path()).unwrap();
        std::env::set_var("LOG_FILE", &r_log);
        let init: Symbol<unsafe extern "C" fn() -> c_int> = lib.get(b"initialize_logger").unwrap();
        let finalize: Symbol<unsafe extern "C" fn()> = lib.get(b"finalize_logger").unwrap();
        let ret = init();
        assert_eq!(ret, 0, "Rust initialize_logger should return 0");
        finalize();
    }

    let c_content = fs::read(&c_log).unwrap_or_default();
    let r_content = fs::read(&r_log).unwrap_or_default();
    assert_eq!(c_content, r_content, "Logger init/finalize output mismatch.\nC:    {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c_content), String::from_utf8_lossy(&r_content));

    let _ = fs::remove_file(&c_log);
    let _ = fs::remove_file(&r_log);
}

#[test]
fn test_log_info_warning_error() {
    let id = unique_id();
    let c_log = format!("/tmp/test_c_logmsg_{}.log", id);
    let r_log = format!("/tmp/test_r_logmsg_{}.log", id);

    let msg = CString::new("Test message 123").unwrap();

    // C version
    unsafe {
        let lib = Library::new(c_lib_path()).unwrap();
        std::env::set_var("LOG_FILE", &c_log);
        let init: Symbol<unsafe extern "C" fn() -> c_int> = lib.get(b"initialize_logger").unwrap();
        let log_info: Symbol<unsafe extern "C" fn(*const c_char)> = lib.get(b"log_info").unwrap();
        let log_warning: Symbol<unsafe extern "C" fn(*const c_char)> = lib.get(b"log_warning").unwrap();
        let log_error: Symbol<unsafe extern "C" fn(*const c_char)> = lib.get(b"log_error").unwrap();
        let finalize: Symbol<unsafe extern "C" fn()> = lib.get(b"finalize_logger").unwrap();
        init();
        log_info(msg.as_ptr());
        log_warning(msg.as_ptr());
        log_error(msg.as_ptr());
        finalize();
    }

    // Rust version
    unsafe {
        let lib = Library::new(rust_lib_path()).unwrap();
        std::env::set_var("LOG_FILE", &r_log);
        let init: Symbol<unsafe extern "C" fn() -> c_int> = lib.get(b"initialize_logger").unwrap();
        let log_info: Symbol<unsafe extern "C" fn(*const c_char)> = lib.get(b"log_info").unwrap();
        let log_warning: Symbol<unsafe extern "C" fn(*const c_char)> = lib.get(b"log_warning").unwrap();
        let log_error: Symbol<unsafe extern "C" fn(*const c_char)> = lib.get(b"log_error").unwrap();
        let finalize: Symbol<unsafe extern "C" fn()> = lib.get(b"finalize_logger").unwrap();
        init();
        log_info(msg.as_ptr());
        log_warning(msg.as_ptr());
        log_error(msg.as_ptr());
        finalize();
    }

    let c_content = fs::read(&c_log).unwrap_or_default();
    let r_content = fs::read(&r_log).unwrap_or_default();
    assert_eq!(c_content, r_content, "Log messages mismatch.\nC:    {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c_content), String::from_utf8_lossy(&r_content));

    let _ = fs::remove_file(&c_log);
    let _ = fs::remove_file(&r_log);
}

// ---- TaskManager struct tests ----

#[repr(C)]
struct Task {
    description: [c_char; 256],
    priority: c_int,
}

#[repr(C)]
struct TaskManager {
    tasks: *mut Task,
    max_tasks: c_int,
    task_count: c_int,
}

#[test]
fn test_create_and_destroy_task_manager() {
    let id = unique_id();
    let c_log = format!("/tmp/test_c_tm_create_{}.log", id);
    let r_log = format!("/tmp/test_r_tm_create_{}.log", id);

    // C version
    let (c_max, c_count) = unsafe {
        let lib = Library::new(c_lib_path()).unwrap();
        std::env::set_var("LOG_FILE", &c_log);
        std::env::remove_var("MAX_TASKS");
        let init: Symbol<unsafe extern "C" fn() -> c_int> = lib.get(b"initialize_logger").unwrap();
        let create: Symbol<unsafe extern "C" fn() -> *mut TaskManager> = lib.get(b"create_task_manager").unwrap();
        let destroy: Symbol<unsafe extern "C" fn(*mut TaskManager)> = lib.get(b"destroy_task_manager").unwrap();
        let finalize: Symbol<unsafe extern "C" fn()> = lib.get(b"finalize_logger").unwrap();
        init();
        let mgr = create();
        assert!(!mgr.is_null());
        let max = (*mgr).max_tasks;
        let count = (*mgr).task_count;
        destroy(mgr);
        finalize();
        (max, count)
    };

    // Rust version
    let (r_max, r_count) = unsafe {
        let lib = Library::new(rust_lib_path()).unwrap();
        std::env::set_var("LOG_FILE", &r_log);
        std::env::remove_var("MAX_TASKS");
        let init: Symbol<unsafe extern "C" fn() -> c_int> = lib.get(b"initialize_logger").unwrap();
        let create: Symbol<unsafe extern "C" fn() -> *mut TaskManager> = lib.get(b"create_task_manager").unwrap();
        let destroy: Symbol<unsafe extern "C" fn(*mut TaskManager)> = lib.get(b"destroy_task_manager").unwrap();
        let finalize: Symbol<unsafe extern "C" fn()> = lib.get(b"finalize_logger").unwrap();
        init();
        let mgr = create();
        assert!(!mgr.is_null());
        let max = (*mgr).max_tasks;
        let count = (*mgr).task_count;
        destroy(mgr);
        finalize();
        (max, count)
    };

    assert_eq!(c_max, r_max, "max_tasks mismatch: C={} Rust={}", c_max, r_max);
    assert_eq!(c_count, r_count, "task_count mismatch: C={} Rust={}", c_count, r_count);

    // Compare log files
    let c_content = fs::read(&c_log).unwrap_or_default();
    let r_content = fs::read(&r_log).unwrap_or_default();
    assert_eq!(c_content, r_content, "create/destroy log mismatch.\nC:    {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c_content), String::from_utf8_lossy(&r_content));

    let _ = fs::remove_file(&c_log);
    let _ = fs::remove_file(&r_log);
}

#[test]
fn test_add_task_struct_contents() {
    let id = unique_id();
    let c_log = format!("/tmp/test_c_addtask_{}.log", id);
    let r_log = format!("/tmp/test_r_addtask_{}.log", id);
    let desc = CString::new("Buy groceries").unwrap();

    unsafe fn run_add_task(lib_path: &str, log_path: &str, desc: &CString) -> ([c_char; 256], c_int, c_int) {
        let lib = Library::new(lib_path).unwrap();
        std::env::set_var("LOG_FILE", log_path);
        std::env::remove_var("MAX_TASKS");
        let init: Symbol<unsafe extern "C" fn() -> c_int> = lib.get(b"initialize_logger").unwrap();
        let create: Symbol<unsafe extern "C" fn() -> *mut TaskManager> = lib.get(b"create_task_manager").unwrap();
        let add: Symbol<unsafe extern "C" fn(*mut TaskManager, *const c_char, c_int)> = lib.get(b"add_task").unwrap();
        let destroy: Symbol<unsafe extern "C" fn(*mut TaskManager)> = lib.get(b"destroy_task_manager").unwrap();
        let finalize: Symbol<unsafe extern "C" fn()> = lib.get(b"finalize_logger").unwrap();
        init();
        let mgr = create();
        add(mgr, desc.as_ptr(), 5);
        let task = &*(*mgr).tasks;
        let description = task.description;
        let priority = task.priority;
        let count = (*mgr).task_count;
        destroy(mgr);
        finalize();
        (description, priority, count)
    }

    let (c_desc, c_pri, c_count) = unsafe { run_add_task(&c_lib_path(), &c_log, &desc) };
    let (r_desc, r_pri, r_count) = unsafe { run_add_task(&rust_lib_path(), &r_log, &desc) };

    // Compare description bytes
    assert_eq!(&c_desc[..], &r_desc[..], "Task description bytes mismatch");
    assert_eq!(c_pri, r_pri, "Task priority mismatch: C={} Rust={}", c_pri, r_pri);
    assert_eq!(c_count, r_count, "task_count mismatch: C={} Rust={}", c_count, r_count);

    let c_content = fs::read(&c_log).unwrap_or_default();
    let r_content = fs::read(&r_log).unwrap_or_default();
    assert_eq!(c_content, r_content, "add_task log mismatch.\nC:    {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c_content), String::from_utf8_lossy(&r_content));

    let _ = fs::remove_file(&c_log);
    let _ = fs::remove_file(&r_log);
}

// ---- print_tasks stdout comparison ----

#[test]
fn test_print_tasks_stdout() {
    let id = unique_id();
    let c_log = format!("/tmp/test_c_print_{}.log", id);
    let r_log = format!("/tmp/test_r_print_{}.log", id);

    // We'll use the driver function to test stdout since print_tasks writes to stdout.
    // Capture via a helper that redirects stdout to a pipe.
    // Simpler: use driver() which calls print_tasks, and capture via process.

    // Instead, test print_tasks by calling driver with known input and comparing stdout.
    // We'll do that in the driver test. Here, test the struct-level behavior.

    let desc1 = CString::new("Task A").unwrap();
    let desc2 = CString::new("Task B").unwrap();

    unsafe fn run_print(lib_path: &str, log_path: &str, d1: &CString, d2: &CString) -> Vec<u8> {
        let lib = Library::new(lib_path).unwrap();
        std::env::set_var("LOG_FILE", log_path);
        std::env::remove_var("MAX_TASKS");
        let init: Symbol<unsafe extern "C" fn() -> c_int> = lib.get(b"initialize_logger").unwrap();
        let create: Symbol<unsafe extern "C" fn() -> *mut TaskManager> = lib.get(b"create_task_manager").unwrap();
        let add: Symbol<unsafe extern "C" fn(*mut TaskManager, *const c_char, c_int)> = lib.get(b"add_task").unwrap();
        let destroy: Symbol<unsafe extern "C" fn(*mut TaskManager)> = lib.get(b"destroy_task_manager").unwrap();
        let finalize: Symbol<unsafe extern "C" fn()> = lib.get(b"finalize_logger").unwrap();
        init();
        let mgr = create();
        add(mgr, d1.as_ptr(), 1);
        add(mgr, d2.as_ptr(), 2);
        // Read struct to verify
        let count = (*mgr).task_count;
        let mut tasks_data = Vec::new();
        for i in 0..count {
            let t = &*(*mgr).tasks.offset(i as isize);
            tasks_data.extend_from_slice(&std::mem::transmute::<[c_char; 256], [u8; 256]>(t.description));
            tasks_data.extend_from_slice(&t.priority.to_ne_bytes());
        }
        destroy(mgr);
        finalize();
        tasks_data
    }

    let c_data = unsafe { run_print(&c_lib_path(), &c_log, &desc1, &desc2) };
    let r_data = unsafe { run_print(&rust_lib_path(), &r_log, &desc1, &desc2) };

    assert_eq!(c_data, r_data, "Task struct byte contents mismatch after adding multiple tasks");

    let _ = fs::remove_file(&c_log);
    let _ = fs::remove_file(&r_log);
}

// ---- Driver function test (full integration with stdout capture) ----

#[test]
fn test_driver_stdout_and_log() {
    let id = unique_id();
    let c_log = format!("/tmp/test_c_driver_{}.log", id);
    let r_log = format!("/tmp/test_r_driver_{}.log", id);

    let tasks_input = CString::new("Buy milk\nWalk dog\nWrite code").unwrap();

    // For stdout capture, we need to redirect fd 1. Use a pipe via libc.
    unsafe fn capture_driver(lib_path: &str, log_path: &str, input: &CString) -> (Vec<u8>, Vec<u8>, c_int) {
        let lib = Library::new(lib_path).unwrap();
        std::env::set_var("LOG_FILE", log_path);
        std::env::remove_var("MAX_TASKS");

        // Create a pipe to capture stdout
        let mut pipe_fds = [0i32; 2];
        libc::pipe(pipe_fds.as_mut_ptr());
        let old_stdout = libc::dup(1);
        libc::dup2(pipe_fds[1], 1);

        let driver: Symbol<unsafe extern "C" fn(*const c_char) -> c_int> = lib.get(b"driver").unwrap();
        let ret = driver(input.as_ptr());

        // Flush stdout
        libc::fflush(std::ptr::null_mut());
        libc::dup2(old_stdout, 1);
        libc::close(old_stdout);
        libc::close(pipe_fds[1]);

        // Read captured stdout
        let mut stdout_buf = Vec::new();
        let mut read_buf = [0u8; 4096];
        loop {
            let n = libc::read(pipe_fds[0], read_buf.as_mut_ptr() as *mut libc::c_void, read_buf.len());
            if n <= 0 { break; }
            stdout_buf.extend_from_slice(&read_buf[..n as usize]);
        }
        libc::close(pipe_fds[0]);

        let log_content = std::fs::read(log_path).unwrap_or_default();
        (stdout_buf, log_content, ret)
    }

    let (c_stdout, c_logdata, c_ret) = unsafe { capture_driver(&c_lib_path(), &c_log, &tasks_input) };
    let (r_stdout, r_logdata, r_ret) = unsafe { capture_driver(&rust_lib_path(), &r_log, &tasks_input) };

    assert_eq!(c_ret, r_ret, "driver return value mismatch: C={} Rust={}", c_ret, r_ret);
    assert_eq!(c_stdout, r_stdout, "driver stdout mismatch.\nC:    {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c_stdout), String::from_utf8_lossy(&r_stdout));
    assert_eq!(c_logdata, r_logdata, "driver log file mismatch.\nC:    {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c_logdata), String::from_utf8_lossy(&r_logdata));

    let _ = fs::remove_file(&c_log);
    let _ = fs::remove_file(&r_log);
}
