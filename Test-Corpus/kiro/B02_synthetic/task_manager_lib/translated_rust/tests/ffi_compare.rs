use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, CString};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn unique_id() -> usize {
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/debug/libdriver.so")
}

/// Capture stdout from a closure by redirecting fd 1 to a pipe.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::io::Read;
    use std::os::unix::io::FromRawFd;

    unsafe {
        libc::fflush(std::ptr::null_mut()); // flush all
        let mut pipe_fds = [0i32; 2];
        assert_eq!(libc::pipe(pipe_fds.as_mut_ptr()), 0);
        let saved = libc::dup(1);
        libc::dup2(pipe_fds[1], 1);
        libc::close(pipe_fds[1]);

        f();

        libc::fflush(std::ptr::null_mut());
        libc::dup2(saved, 1);
        libc::close(saved);

        let mut buf = Vec::new();
        let mut reader = std::fs::File::from_raw_fd(pipe_fds[0]);
        // Set non-blocking to avoid hanging
        libc::fcntl(pipe_fds[0], libc::F_SETFL, libc::O_NONBLOCK);
        let _ = reader.read_to_end(&mut buf);
        buf
    }
}

// =========================================================================
// Logger tests
// =========================================================================

#[test]
fn test_initialize_and_finalize_logger() {
    let id = unique_id();
    let c_log = format!("/tmp/test_c_logger_{}.log", id);
    let r_log = format!("/tmp/test_r_logger_{}.log", id);

    // C
    unsafe {
        let lib = Library::new(c_lib_path()).unwrap();
        let init: Symbol<unsafe extern "C" fn() -> c_int> = lib.get(b"initialize_logger").unwrap();
        let fin: Symbol<unsafe extern "C" fn()> = lib.get(b"finalize_logger").unwrap();

        std::env::set_var("LOG_FILE", &c_log);
        let ret = init();
        assert_eq!(ret, 0);
        fin();
        std::env::remove_var("LOG_FILE");
    }

    // Rust
    unsafe {
        let lib = Library::new(rust_lib_path()).unwrap();
        let init: Symbol<unsafe extern "C" fn() -> c_int> = lib.get(b"initialize_logger").unwrap();
        let fin: Symbol<unsafe extern "C" fn()> = lib.get(b"finalize_logger").unwrap();

        std::env::set_var("LOG_FILE", &r_log);
        let ret = init();
        assert_eq!(ret, 0);
        fin();
        std::env::remove_var("LOG_FILE");
    }

    let c_content = fs::read(&c_log).unwrap();
    let r_content = fs::read(&r_log).unwrap();
    assert_eq!(c_content, r_content, "Logger init/finalize output mismatch.\nC:    {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c_content), String::from_utf8_lossy(&r_content));

    let _ = fs::remove_file(&c_log);
    let _ = fs::remove_file(&r_log);
}

#[test]
fn test_log_info_warning_error() {
    let id = unique_id();
    let c_log = format!("/tmp/test_c_levels_{}.log", id);
    let r_log = format!("/tmp/test_r_levels_{}.log", id);

    let msg = CString::new("test message 123").unwrap();

    // C
    unsafe {
        let lib = Library::new(c_lib_path()).unwrap();
        let init: Symbol<unsafe extern "C" fn() -> c_int> = lib.get(b"initialize_logger").unwrap();
        let fin: Symbol<unsafe extern "C" fn()> = lib.get(b"finalize_logger").unwrap();
        let info: Symbol<unsafe extern "C" fn(*const c_char)> = lib.get(b"log_info").unwrap();
        let warn: Symbol<unsafe extern "C" fn(*const c_char)> = lib.get(b"log_warning").unwrap();
        let err: Symbol<unsafe extern "C" fn(*const c_char)> = lib.get(b"log_error").unwrap();

        std::env::set_var("LOG_FILE", &c_log);
        init();
        info(msg.as_ptr());
        warn(msg.as_ptr());
        err(msg.as_ptr());
        fin();
        std::env::remove_var("LOG_FILE");
    }

    // Rust
    unsafe {
        let lib = Library::new(rust_lib_path()).unwrap();
        let init: Symbol<unsafe extern "C" fn() -> c_int> = lib.get(b"initialize_logger").unwrap();
        let fin: Symbol<unsafe extern "C" fn()> = lib.get(b"finalize_logger").unwrap();
        let info: Symbol<unsafe extern "C" fn(*const c_char)> = lib.get(b"log_info").unwrap();
        let warn: Symbol<unsafe extern "C" fn(*const c_char)> = lib.get(b"log_warning").unwrap();
        let err: Symbol<unsafe extern "C" fn(*const c_char)> = lib.get(b"log_error").unwrap();

        std::env::set_var("LOG_FILE", &r_log);
        init();
        info(msg.as_ptr());
        warn(msg.as_ptr());
        err(msg.as_ptr());
        fin();
        std::env::remove_var("LOG_FILE");
    }

    let c_content = fs::read(&c_log).unwrap();
    let r_content = fs::read(&r_log).unwrap();
    assert_eq!(c_content, r_content, "Log levels output mismatch.\nC:    {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c_content), String::from_utf8_lossy(&r_content));

    let _ = fs::remove_file(&c_log);
    let _ = fs::remove_file(&r_log);
}

// =========================================================================
// TaskManager tests
// =========================================================================

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
fn test_create_add_print_destroy() {
    let id = unique_id();
    let c_log = format!("/tmp/test_c_task_{}.log", id);
    let r_log = format!("/tmp/test_r_task_{}.log", id);

    let desc = CString::new("Buy groceries").unwrap();

    // C
    let c_stdout = unsafe {
        let lib = Library::new(c_lib_path()).unwrap();
        let init: Symbol<unsafe extern "C" fn() -> c_int> = lib.get(b"initialize_logger").unwrap();
        let fin: Symbol<unsafe extern "C" fn()> = lib.get(b"finalize_logger").unwrap();
        let create: Symbol<unsafe extern "C" fn() -> *mut TaskManager> = lib.get(b"create_task_manager").unwrap();
        let add: Symbol<unsafe extern "C" fn(*mut TaskManager, *const c_char, c_int)> = lib.get(b"add_task").unwrap();
        let print: Symbol<unsafe extern "C" fn(*const TaskManager)> = lib.get(b"print_tasks").unwrap();
        let destroy: Symbol<unsafe extern "C" fn(*mut TaskManager)> = lib.get(b"destroy_task_manager").unwrap();

        std::env::set_var("LOG_FILE", &c_log);
        std::env::remove_var("MAX_TASKS");
        init();
        let mgr = create();
        assert!(!mgr.is_null());
        add(mgr, desc.as_ptr(), 5);
        let out = capture_stdout(|| print(mgr));
        destroy(mgr);
        fin();
        std::env::remove_var("LOG_FILE");
        out
    };

    // Rust
    let r_stdout = unsafe {
        let lib = Library::new(rust_lib_path()).unwrap();
        let init: Symbol<unsafe extern "C" fn() -> c_int> = lib.get(b"initialize_logger").unwrap();
        let fin: Symbol<unsafe extern "C" fn()> = lib.get(b"finalize_logger").unwrap();
        let create: Symbol<unsafe extern "C" fn() -> *mut TaskManager> = lib.get(b"create_task_manager").unwrap();
        let add: Symbol<unsafe extern "C" fn(*mut TaskManager, *const c_char, c_int)> = lib.get(b"add_task").unwrap();
        let print: Symbol<unsafe extern "C" fn(*const TaskManager)> = lib.get(b"print_tasks").unwrap();
        let destroy: Symbol<unsafe extern "C" fn(*mut TaskManager)> = lib.get(b"destroy_task_manager").unwrap();

        std::env::set_var("LOG_FILE", &r_log);
        std::env::remove_var("MAX_TASKS");
        init();
        let mgr = create();
        assert!(!mgr.is_null());
        add(mgr, desc.as_ptr(), 5);
        let out = capture_stdout(|| print(mgr));
        destroy(mgr);
        fin();
        std::env::remove_var("LOG_FILE");
        out
    };

    assert_eq!(c_stdout, r_stdout, "print_tasks stdout mismatch.\nC:    {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c_stdout), String::from_utf8_lossy(&r_stdout));

    // Also compare log files
    let c_content = fs::read(&c_log).unwrap();
    let r_content = fs::read(&r_log).unwrap();
    assert_eq!(c_content, r_content, "TaskManager log output mismatch.\nC:    {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c_content), String::from_utf8_lossy(&r_content));

    let _ = fs::remove_file(&c_log);
    let _ = fs::remove_file(&r_log);
}

#[test]
fn test_add_task_overflow() {
    let id = unique_id();
    let c_log = format!("/tmp/test_c_overflow_{}.log", id);
    let r_log = format!("/tmp/test_r_overflow_{}.log", id);

    let desc = CString::new("task").unwrap();

    // Test with MAX_TASKS=2, add 3 tasks
    for (log_path, lib_path) in [(&c_log, c_lib_path()), (&r_log, rust_lib_path())] {
        unsafe {
            let lib = Library::new(lib_path).unwrap();
            let init: Symbol<unsafe extern "C" fn() -> c_int> = lib.get(b"initialize_logger").unwrap();
            let fin: Symbol<unsafe extern "C" fn()> = lib.get(b"finalize_logger").unwrap();
            let create: Symbol<unsafe extern "C" fn() -> *mut TaskManager> = lib.get(b"create_task_manager").unwrap();
            let add: Symbol<unsafe extern "C" fn(*mut TaskManager, *const c_char, c_int)> = lib.get(b"add_task").unwrap();
            let destroy: Symbol<unsafe extern "C" fn(*mut TaskManager)> = lib.get(b"destroy_task_manager").unwrap();

            std::env::set_var("LOG_FILE", log_path);
            std::env::set_var("MAX_TASKS", "2");
            init();
            let mgr = create();
            add(mgr, desc.as_ptr(), 1);
            add(mgr, desc.as_ptr(), 2);
            add(mgr, desc.as_ptr(), 3); // should trigger warning
            destroy(mgr);
            fin();
            std::env::remove_var("LOG_FILE");
            std::env::remove_var("MAX_TASKS");
        }
    }

    let c_content = fs::read(&c_log).unwrap();
    let r_content = fs::read(&r_log).unwrap();
    assert_eq!(c_content, r_content, "Overflow log mismatch.\nC:    {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c_content), String::from_utf8_lossy(&r_content));

    let _ = fs::remove_file(&c_log);
    let _ = fs::remove_file(&r_log);
}

// =========================================================================
// Driver test
// =========================================================================

#[test]
fn test_driver() {
    let id = unique_id();
    let c_log = format!("/tmp/test_c_driver_{}.log", id);
    let r_log = format!("/tmp/test_r_driver_{}.log", id);

    let tasks = CString::new("Task A\nTask B\nTask C").unwrap();

    // C
    let c_stdout = unsafe {
        let lib = Library::new(c_lib_path()).unwrap();
        let drv: Symbol<unsafe extern "C" fn(*const c_char) -> c_int> = lib.get(b"driver").unwrap();

        std::env::set_var("LOG_FILE", &c_log);
        std::env::remove_var("MAX_TASKS");
        let out = capture_stdout(|| { drv(tasks.as_ptr()); });
        std::env::remove_var("LOG_FILE");
        out
    };

    // Rust
    let r_stdout = unsafe {
        let lib = Library::new(rust_lib_path()).unwrap();
        let drv: Symbol<unsafe extern "C" fn(*const c_char) -> c_int> = lib.get(b"driver").unwrap();

        std::env::set_var("LOG_FILE", &r_log);
        std::env::remove_var("MAX_TASKS");
        let out = capture_stdout(|| { drv(tasks.as_ptr()); });
        std::env::remove_var("LOG_FILE");
        out
    };

    assert_eq!(c_stdout, r_stdout, "driver stdout mismatch.\nC:    {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c_stdout), String::from_utf8_lossy(&r_stdout));

    let c_content = fs::read(&c_log).unwrap();
    let r_content = fs::read(&r_log).unwrap();
    assert_eq!(c_content, r_content, "driver log mismatch.\nC:    {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c_content), String::from_utf8_lossy(&r_content));

    let _ = fs::remove_file(&c_log);
    let _ = fs::remove_file(&r_log);
}

#[test]
fn test_driver_empty_input() {
    let id = unique_id();
    let c_log = format!("/tmp/test_c_drvempty_{}.log", id);
    let r_log = format!("/tmp/test_r_drvempty_{}.log", id);

    let tasks = CString::new("").unwrap();

    // C
    let c_stdout = unsafe {
        let lib = Library::new(c_lib_path()).unwrap();
        let drv: Symbol<unsafe extern "C" fn(*const c_char) -> c_int> = lib.get(b"driver").unwrap();
        std::env::set_var("LOG_FILE", &c_log);
        std::env::remove_var("MAX_TASKS");
        let out = capture_stdout(|| { drv(tasks.as_ptr()); });
        std::env::remove_var("LOG_FILE");
        out
    };

    // Rust
    let r_stdout = unsafe {
        let lib = Library::new(rust_lib_path()).unwrap();
        let drv: Symbol<unsafe extern "C" fn(*const c_char) -> c_int> = lib.get(b"driver").unwrap();
        std::env::set_var("LOG_FILE", &r_log);
        std::env::remove_var("MAX_TASKS");
        let out = capture_stdout(|| { drv(tasks.as_ptr()); });
        std::env::remove_var("LOG_FILE");
        out
    };

    assert_eq!(c_stdout, r_stdout, "driver empty stdout mismatch.\nC:    {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c_stdout), String::from_utf8_lossy(&r_stdout));

    let c_content = fs::read(&c_log).unwrap();
    let r_content = fs::read(&r_log).unwrap();
    assert_eq!(c_content, r_content, "driver empty log mismatch.\nC:    {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c_content), String::from_utf8_lossy(&r_content));

    let _ = fs::remove_file(&c_log);
    let _ = fs::remove_file(&r_log);
}

#[test]
fn test_driver_single_task_no_newline() {
    let id = unique_id();
    let c_log = format!("/tmp/test_c_drvsingle_{}.log", id);
    let r_log = format!("/tmp/test_r_drvsingle_{}.log", id);

    let tasks = CString::new("Only one task").unwrap();

    let c_stdout = unsafe {
        let lib = Library::new(c_lib_path()).unwrap();
        let drv: Symbol<unsafe extern "C" fn(*const c_char) -> c_int> = lib.get(b"driver").unwrap();
        std::env::set_var("LOG_FILE", &c_log);
        std::env::remove_var("MAX_TASKS");
        let out = capture_stdout(|| { drv(tasks.as_ptr()); });
        std::env::remove_var("LOG_FILE");
        out
    };

    let r_stdout = unsafe {
        let lib = Library::new(rust_lib_path()).unwrap();
        let drv: Symbol<unsafe extern "C" fn(*const c_char) -> c_int> = lib.get(b"driver").unwrap();
        std::env::set_var("LOG_FILE", &r_log);
        std::env::remove_var("MAX_TASKS");
        let out = capture_stdout(|| { drv(tasks.as_ptr()); });
        std::env::remove_var("LOG_FILE");
        out
    };

    assert_eq!(c_stdout, r_stdout, "driver single task stdout mismatch.\nC:    {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c_stdout), String::from_utf8_lossy(&r_stdout));

    let _ = fs::remove_file(&c_log);
    let _ = fs::remove_file(&r_log);
}
