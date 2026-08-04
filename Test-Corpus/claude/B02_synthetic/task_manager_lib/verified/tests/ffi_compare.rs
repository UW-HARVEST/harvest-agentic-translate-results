// Integration tests that load both the C and Rust shared libraries through
// `libloading` and compare exported function behavior byte-for-byte.

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, CString};
use std::fs;
use std::io::Read;
use std::os::raw::c_void;
use std::path::PathBuf;
use std::sync::Mutex;

// Tests touch global stdio state and environment variables, so they must
// run sequentially.
static TEST_LOCK: Mutex<()> = Mutex::new(());

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

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    project_root().join("c_src").join("build").join("libdriver.so")
}

fn rust_so_path() -> PathBuf {
    // Look for the Rust .so in target dirs.
    for profile in &["release", "debug"] {
        let p = project_root()
            .join("target")
            .join(profile)
            .join("libdriver.so");
        if p.exists() {
            return p;
        }
    }
    panic!("Could not find Rust libdriver.so. Run `cargo build` first.");
}

fn load_c() -> Library {
    unsafe {
        Library::new(c_so_path()).expect("failed to load C libdriver.so")
    }
}

fn load_rust() -> Library {
    unsafe {
        Library::new(rust_so_path()).expect("failed to load Rust libdriver.so")
    }
}

/// Redirect stdout to the given file path while running `f`. Returns the
/// captured bytes.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    extern "C" {
        fn dup(oldfd: c_int) -> c_int;
        fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
        fn close(fd: c_int) -> c_int;
        fn fflush(stream: *mut c_void) -> c_int;
    }
    // Resolve stdout via libc.
    let libc_handle = unsafe { Library::new("libc.so.6").unwrap() };
    let stdout_sym: Symbol<*mut *mut c_void> =
        unsafe { libc_handle.get(b"stdout").unwrap() };
    let stdout_ptr = unsafe { **stdout_sym };

    let tmp = tempfile_path("stdout_capture.txt");
    let _ = fs::remove_file(&tmp);

    unsafe { fflush(stdout_ptr) };
    let saved = unsafe { dup(1) };
    let cpath = CString::new(tmp.to_str().unwrap()).unwrap();
    let fd = unsafe {
        libc::open(
            cpath.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC,
            0o644,
        )
    };
    assert!(fd >= 0, "open() failed for {:?}", tmp);
    unsafe { dup2(fd, 1) };
    unsafe { close(fd) };

    f();

    unsafe { fflush(stdout_ptr) };
    unsafe { dup2(saved, 1) };
    unsafe { close(saved) };

    let mut data = Vec::new();
    let mut file = std::fs::File::open(&tmp).unwrap();
    file.read_to_end(&mut data).unwrap();
    let _ = fs::remove_file(&tmp);
    data
}

fn tempfile_path(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!(
        "ffi_compare_{}_{}_{}",
        std::process::id(),
        rand_suffix(),
        name
    ));
    p
}

fn rand_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:x}", n)
}

fn read_file(path: &std::path::Path) -> Vec<u8> {
    if path.exists() {
        fs::read(path).unwrap()
    } else {
        Vec::new()
    }
}

fn set_log_file(path: &std::path::Path) {
    std::env::set_var("LOG_FILE", path);
}

fn unset_max_tasks() {
    std::env::remove_var("MAX_TASKS");
}

// =========================================================================
// Logger functions
// =========================================================================

fn run_logger_sequence(lib: &Library, log_path: &std::path::Path) {
    set_log_file(log_path);
    unsafe {
        let init: Symbol<unsafe extern "C" fn() -> c_int> =
            lib.get(b"initialize_logger").unwrap();
        let log_info: Symbol<unsafe extern "C" fn(*const c_char)> =
            lib.get(b"log_info").unwrap();
        let log_warning: Symbol<unsafe extern "C" fn(*const c_char)> =
            lib.get(b"log_warning").unwrap();
        let log_error: Symbol<unsafe extern "C" fn(*const c_char)> =
            lib.get(b"log_error").unwrap();
        let finalize: Symbol<unsafe extern "C" fn()> =
            lib.get(b"finalize_logger").unwrap();

        let r = init();
        assert_eq!(r, 0);
        let s_info = CString::new("hello info").unwrap();
        let s_warn = CString::new("a warning here").unwrap();
        let s_err = CString::new("an error happened").unwrap();
        log_info(s_info.as_ptr());
        log_warning(s_warn.as_ptr());
        log_error(s_err.as_ptr());
        finalize();
    }
}

#[test]
fn test_logger_writes_match() {
    let _g = TEST_LOCK.lock().unwrap();
    let c = load_c();
    let r = load_rust();

    let c_log = tempfile_path("c.log");
    let r_log = tempfile_path("r.log");
    let _ = fs::remove_file(&c_log);
    let _ = fs::remove_file(&r_log);

    run_logger_sequence(&c, &c_log);
    run_logger_sequence(&r, &r_log);

    let c_data = read_file(&c_log);
    let r_data = read_file(&r_log);

    let _ = fs::remove_file(&c_log);
    let _ = fs::remove_file(&r_log);

    assert_eq!(c_data, r_data, "Logger output mismatch");
    assert!(!c_data.is_empty());
}

#[test]
fn test_initialize_logger_failure_path() {
    // Use a directory path as LOG_FILE — fopen will fail to open it.
    let _g = TEST_LOCK.lock().unwrap();
    let c = load_c();
    let r = load_rust();

    let dir = tempfile_path("dir_as_logfile");
    fs::create_dir_all(&dir).unwrap();
    set_log_file(&dir);

    unsafe {
        let init_c: Symbol<unsafe extern "C" fn() -> c_int> =
            c.get(b"initialize_logger").unwrap();
        let init_r: Symbol<unsafe extern "C" fn() -> c_int> =
            r.get(b"initialize_logger").unwrap();
        let cv = init_c();
        let rv = init_r();
        assert_eq!(cv, rv);
        assert_eq!(cv, -1);
    }
    let _ = fs::remove_dir_all(&dir);
}

// =========================================================================
// TaskManager functions
// =========================================================================

fn run_task_manager_sequence(
    lib: &Library,
    log_path: &std::path::Path,
    max_tasks: Option<&str>,
    descriptions: &[&str],
) -> Vec<u8> {
    let _ = fs::remove_file(log_path);
    set_log_file(log_path);
    if let Some(v) = max_tasks {
        std::env::set_var("MAX_TASKS", v);
    } else {
        unset_max_tasks();
    }

    let stdout = capture_stdout(|| unsafe {
        let init: Symbol<unsafe extern "C" fn() -> c_int> =
            lib.get(b"initialize_logger").unwrap();
        let create: Symbol<unsafe extern "C" fn() -> *mut TaskManager> =
            lib.get(b"create_task_manager").unwrap();
        let add: Symbol<
            unsafe extern "C" fn(*mut TaskManager, *const c_char, c_int),
        > = lib.get(b"add_task").unwrap();
        let print: Symbol<unsafe extern "C" fn(*const TaskManager)> =
            lib.get(b"print_tasks").unwrap();
        let destroy: Symbol<unsafe extern "C" fn(*mut TaskManager)> =
            lib.get(b"destroy_task_manager").unwrap();
        let finalize: Symbol<unsafe extern "C" fn()> =
            lib.get(b"finalize_logger").unwrap();

        assert_eq!(init(), 0);
        let mgr = create();
        assert!(!mgr.is_null());
        for (i, d) in descriptions.iter().enumerate() {
            let cd = CString::new(*d).unwrap();
            add(mgr, cd.as_ptr(), (i as c_int) + 1);
        }
        print(mgr);
        destroy(mgr);
        finalize();
    });
    stdout
}

#[test]
fn test_task_manager_basic() {
    let _g = TEST_LOCK.lock().unwrap();
    let c = load_c();
    let r = load_rust();

    let c_log = tempfile_path("c.log");
    let r_log = tempfile_path("r.log");

    let descs = ["alpha", "beta", "gamma"];
    let c_out = run_task_manager_sequence(&c, &c_log, Some("5"), &descs);
    let r_out = run_task_manager_sequence(&r, &r_log, Some("5"), &descs);

    assert_eq!(c_out, r_out, "stdout mismatch in basic case");

    let c_data = read_file(&c_log);
    let r_data = read_file(&r_log);
    let _ = fs::remove_file(&c_log);
    let _ = fs::remove_file(&r_log);
    assert_eq!(c_data, r_data, "log file mismatch in basic case");
}

#[test]
fn test_task_manager_default_max() {
    let _g = TEST_LOCK.lock().unwrap();
    let c = load_c();
    let r = load_rust();

    let c_log = tempfile_path("c.log");
    let r_log = tempfile_path("r.log");

    let descs = ["one", "two"];
    let c_out = run_task_manager_sequence(&c, &c_log, None, &descs);
    let r_out = run_task_manager_sequence(&r, &r_log, None, &descs);

    assert_eq!(c_out, r_out, "stdout mismatch (default MAX_TASKS)");

    let c_data = read_file(&c_log);
    let r_data = read_file(&r_log);
    let _ = fs::remove_file(&c_log);
    let _ = fs::remove_file(&r_log);
    assert_eq!(c_data, r_data, "log file mismatch (default MAX_TASKS)");
}

#[test]
fn test_task_manager_overflow_warning() {
    let _g = TEST_LOCK.lock().unwrap();
    let c = load_c();
    let r = load_rust();

    let c_log = tempfile_path("c.log");
    let r_log = tempfile_path("r.log");

    // MAX_TASKS = 2, but we add 4 tasks → 2 should be rejected.
    let descs = ["a", "b", "c", "d"];
    let c_out = run_task_manager_sequence(&c, &c_log, Some("2"), &descs);
    let r_out = run_task_manager_sequence(&r, &r_log, Some("2"), &descs);
    assert_eq!(c_out, r_out, "stdout mismatch (overflow)");

    let c_data = read_file(&c_log);
    let r_data = read_file(&r_log);
    let _ = fs::remove_file(&c_log);
    let _ = fs::remove_file(&r_log);
    assert_eq!(c_data, r_data, "log file mismatch (overflow)");
}

#[test]
fn test_task_manager_long_description_truncation() {
    let _g = TEST_LOCK.lock().unwrap();
    let c = load_c();
    let r = load_rust();

    let c_log = tempfile_path("c.log");
    let r_log = tempfile_path("r.log");

    // 300 chars — exceeds 255-byte description capacity.
    let long_desc: String = std::iter::repeat('x').take(300).collect();
    let descs = [long_desc.as_str(), "short"];
    let c_out = run_task_manager_sequence(&c, &c_log, Some("4"), &descs);
    let r_out = run_task_manager_sequence(&r, &r_log, Some("4"), &descs);
    assert_eq!(c_out, r_out, "stdout mismatch (truncation)");

    let c_data = read_file(&c_log);
    let r_data = read_file(&r_log);
    let _ = fs::remove_file(&c_log);
    let _ = fs::remove_file(&r_log);
    assert_eq!(c_data, r_data, "log file mismatch (truncation)");
}

#[test]
fn test_create_task_manager_struct_fields() {
    let _g = TEST_LOCK.lock().unwrap();
    let c = load_c();
    let r = load_rust();

    let c_log = tempfile_path("c.log");
    let r_log = tempfile_path("r.log");

    set_log_file(&c_log);
    std::env::set_var("MAX_TASKS", "7");
    unsafe {
        let init: Symbol<unsafe extern "C" fn() -> c_int> =
            c.get(b"initialize_logger").unwrap();
        let create: Symbol<unsafe extern "C" fn() -> *mut TaskManager> =
            c.get(b"create_task_manager").unwrap();
        let destroy: Symbol<unsafe extern "C" fn(*mut TaskManager)> =
            c.get(b"destroy_task_manager").unwrap();
        let finalize: Symbol<unsafe extern "C" fn()> =
            c.get(b"finalize_logger").unwrap();

        assert_eq!(init(), 0);
        let mgr = create();
        assert!(!mgr.is_null());
        assert_eq!((*mgr).max_tasks, 7);
        assert_eq!((*mgr).task_count, 0);
        assert!(!(*mgr).tasks.is_null());
        destroy(mgr);
        finalize();
    }

    set_log_file(&r_log);
    unsafe {
        let init: Symbol<unsafe extern "C" fn() -> c_int> =
            r.get(b"initialize_logger").unwrap();
        let create: Symbol<unsafe extern "C" fn() -> *mut TaskManager> =
            r.get(b"create_task_manager").unwrap();
        let destroy: Symbol<unsafe extern "C" fn(*mut TaskManager)> =
            r.get(b"destroy_task_manager").unwrap();
        let finalize: Symbol<unsafe extern "C" fn()> =
            r.get(b"finalize_logger").unwrap();

        assert_eq!(init(), 0);
        let mgr = create();
        assert!(!mgr.is_null());
        assert_eq!((*mgr).max_tasks, 7);
        assert_eq!((*mgr).task_count, 0);
        assert!(!(*mgr).tasks.is_null());
        destroy(mgr);
        finalize();
    }

    let _ = fs::remove_file(&c_log);
    let _ = fs::remove_file(&r_log);
}

// =========================================================================
// driver()
// =========================================================================

fn run_driver(lib: &Library, log_path: &std::path::Path, tasks: &str) -> (c_int, Vec<u8>) {
    let _ = fs::remove_file(log_path);
    set_log_file(log_path);

    let mut rv: c_int = 0;
    let stdout = capture_stdout(|| unsafe {
        let driver: Symbol<unsafe extern "C" fn(*const c_char) -> c_int> =
            lib.get(b"driver").unwrap();
        let cs = CString::new(tasks).unwrap();
        rv = driver(cs.as_ptr());
    });
    (rv, stdout)
}

#[test]
fn test_driver_simple_multiline() {
    let _g = TEST_LOCK.lock().unwrap();
    let c = load_c();
    let r = load_rust();

    std::env::set_var("MAX_TASKS", "10");
    let c_log = tempfile_path("c.log");
    let r_log = tempfile_path("r.log");

    let tasks = "task one\ntask two\ntask three";
    let (cv, c_out) = run_driver(&c, &c_log, tasks);
    let (rv, r_out) = run_driver(&r, &r_log, tasks);

    assert_eq!(cv, rv);
    assert_eq!(c_out, r_out, "driver stdout mismatch");

    let c_data = read_file(&c_log);
    let r_data = read_file(&r_log);
    let _ = fs::remove_file(&c_log);
    let _ = fs::remove_file(&r_log);
    assert_eq!(c_data, r_data, "driver log mismatch");
}

#[test]
fn test_driver_trailing_newline() {
    let _g = TEST_LOCK.lock().unwrap();
    let c = load_c();
    let r = load_rust();

    std::env::set_var("MAX_TASKS", "10");
    let c_log = tempfile_path("c.log");
    let r_log = tempfile_path("r.log");

    let tasks = "first\nsecond\n";
    let (cv, c_out) = run_driver(&c, &c_log, tasks);
    let (rv, r_out) = run_driver(&r, &r_log, tasks);

    assert_eq!(cv, rv);
    assert_eq!(c_out, r_out);

    let c_data = read_file(&c_log);
    let r_data = read_file(&r_log);
    let _ = fs::remove_file(&c_log);
    let _ = fs::remove_file(&r_log);
    assert_eq!(c_data, r_data);
}

#[test]
fn test_driver_empty_input() {
    let _g = TEST_LOCK.lock().unwrap();
    let c = load_c();
    let r = load_rust();

    std::env::set_var("MAX_TASKS", "10");
    let c_log = tempfile_path("c.log");
    let r_log = tempfile_path("r.log");

    let tasks = "";
    let (cv, c_out) = run_driver(&c, &c_log, tasks);
    let (rv, r_out) = run_driver(&r, &r_log, tasks);

    assert_eq!(cv, rv);
    assert_eq!(c_out, r_out);

    let c_data = read_file(&c_log);
    let r_data = read_file(&r_log);
    let _ = fs::remove_file(&c_log);
    let _ = fs::remove_file(&r_log);
    assert_eq!(c_data, r_data);
}

#[test]
fn test_driver_overflow_warns() {
    let _g = TEST_LOCK.lock().unwrap();
    let c = load_c();
    let r = load_rust();

    std::env::set_var("MAX_TASKS", "2");
    let c_log = tempfile_path("c.log");
    let r_log = tempfile_path("r.log");

    let tasks = "one\ntwo\nthree\nfour";
    let (cv, c_out) = run_driver(&c, &c_log, tasks);
    let (rv, r_out) = run_driver(&r, &r_log, tasks);

    assert_eq!(cv, rv);
    assert_eq!(c_out, r_out);

    let c_data = read_file(&c_log);
    let r_data = read_file(&r_log);
    let _ = fs::remove_file(&c_log);
    let _ = fs::remove_file(&r_log);
    assert_eq!(c_data, r_data);
}
