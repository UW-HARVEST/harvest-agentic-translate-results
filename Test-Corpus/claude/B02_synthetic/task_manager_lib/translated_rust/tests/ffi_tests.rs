// FFI integration tests that load both the C and Rust shared libraries
// and compare their outputs byte-for-byte.

use libloading::{Library, Symbol};
use std::ffi::CString;
use std::fs;
use std::io::Read;
use std::os::raw::{c_char, c_int};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

// Serialize all tests because they manipulate process-wide state
// (env vars, fd 1/fd 2, log files).
static TEST_LOCK: Mutex<()> = Mutex::new(());

#[repr(C)]
#[derive(Debug)]
pub struct Task {
    pub description: [c_char; 256],
    pub priority: c_int,
}

#[repr(C)]
#[derive(Debug)]
pub struct TaskManager {
    pub tasks: *mut Task,
    pub max_tasks: c_int,
    pub task_count: c_int,
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    workspace_root().join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    // Default debug build path. Tests should run after `cargo build`.
    workspace_root().join("target/debug/libdriver.so")
}

unsafe fn load_lib(p: &Path) -> Library {
    Library::new(p).unwrap_or_else(|e| panic!("failed to load {:?}: {}", p, e))
}

fn unique_path(prefix: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    p.push(format!("{}-{}-{}.log", prefix, std::process::id(), nanos));
    p
}

// Redirect stdout (fd 1) to a temp file. Returns saved old fd and the new path.
fn redirect_stdout() -> (c_int, PathBuf) {
    unsafe {
        // Flush all C stdio output first.
        libc::fflush(std::ptr::null_mut());
        let path = unique_path("stdout");
        let cpath = CString::new(path.to_str().unwrap()).unwrap();
        let saved = libc::dup(1);
        let new_fd = libc::open(
            cpath.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC,
            0o644,
        );
        assert!(new_fd >= 0, "open failed: {}", path.display());
        let r = libc::dup2(new_fd, 1);
        assert!(r >= 0);
        libc::close(new_fd);
        (saved, path)
    }
}

fn restore_stdout(saved: c_int) {
    unsafe {
        libc::fflush(std::ptr::null_mut());
        libc::dup2(saved, 1);
        libc::close(saved);
    }
}

fn read_file(p: &Path) -> Vec<u8> {
    let mut f = fs::File::open(p).unwrap_or_else(|e| panic!("open {:?}: {}", p, e));
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).unwrap();
    buf
}

// Run a closure with `LOG_FILE` set to a fresh temp file. Returns the log
// file path so the caller can read its contents.
fn with_log_file<F: FnOnce() -> R, R>(f: F) -> (R, PathBuf) {
    let path = unique_path("rustcttest");
    // Make sure file doesn't already exist
    let _ = fs::remove_file(&path);
    std::env::set_var("LOG_FILE", &path);
    let r = f();
    std::env::remove_var("LOG_FILE");
    (r, path)
}

// Helper - load a function pointer from a library.
unsafe fn sym<'lib, T>(lib: &'lib Library, name: &[u8]) -> Symbol<'lib, T> {
    lib.get(name).unwrap()
}

// ----------------------------------------------------------------------------
// Test: initialize_logger / log_* / finalize_logger byte-for-byte equality.
// ----------------------------------------------------------------------------
#[test]
fn test_logger_messages() {
    let _g = TEST_LOCK.lock().unwrap();
    unsafe {
        let c_lib = load_lib(&c_lib_path());
        let r_lib = load_lib(&rust_lib_path());

        // Bind to symbols.
        type Init = unsafe extern "C" fn() -> c_int;
        type Log = unsafe extern "C" fn(*const c_char);
        type Fini = unsafe extern "C" fn();

        let c_init: Symbol<Init> = sym(&c_lib, b"initialize_logger");
        let c_info: Symbol<Log> = sym(&c_lib, b"log_info");
        let c_warn: Symbol<Log> = sym(&c_lib, b"log_warning");
        let c_err: Symbol<Log> = sym(&c_lib, b"log_error");
        let c_fini: Symbol<Fini> = sym(&c_lib, b"finalize_logger");

        let r_init: Symbol<Init> = sym(&r_lib, b"initialize_logger");
        let r_info: Symbol<Log> = sym(&r_lib, b"log_info");
        let r_warn: Symbol<Log> = sym(&r_lib, b"log_warning");
        let r_err: Symbol<Log> = sym(&r_lib, b"log_error");
        let r_fini: Symbol<Fini> = sym(&r_lib, b"finalize_logger");

        // Run C version.
        let (c_rc, c_path) = with_log_file(|| {
            let rc = c_init();
            let m1 = CString::new("hello info").unwrap();
            let m2 = CString::new("hello warn").unwrap();
            let m3 = CString::new("hello error").unwrap();
            c_info(m1.as_ptr());
            c_warn(m2.as_ptr());
            c_err(m3.as_ptr());
            c_fini();
            rc
        });

        // Run Rust version.
        let (r_rc, r_path) = with_log_file(|| {
            let rc = r_init();
            let m1 = CString::new("hello info").unwrap();
            let m2 = CString::new("hello warn").unwrap();
            let m3 = CString::new("hello error").unwrap();
            r_info(m1.as_ptr());
            r_warn(m2.as_ptr());
            r_err(m3.as_ptr());
            r_fini();
            rc
        });

        assert_eq!(c_rc, r_rc, "init_logger return codes differ");

        let c_bytes = read_file(&c_path);
        let r_bytes = read_file(&r_path);
        assert_eq!(
            c_bytes, r_bytes,
            "logger output differs:\nC:    {:?}\nRust: {:?}",
            String::from_utf8_lossy(&c_bytes),
            String::from_utf8_lossy(&r_bytes)
        );

        let _ = fs::remove_file(&c_path);
        let _ = fs::remove_file(&r_path);
    }
}

// ----------------------------------------------------------------------------
// Test: log_* without initialize_logger — should be no-ops.
// ----------------------------------------------------------------------------
#[test]
fn test_log_without_init() {
    let _g = TEST_LOCK.lock().unwrap();
    unsafe {
        let c_lib = load_lib(&c_lib_path());
        let r_lib = load_lib(&rust_lib_path());

        type Log = unsafe extern "C" fn(*const c_char);
        let c_info: Symbol<Log> = sym(&c_lib, b"log_info");
        let r_info: Symbol<Log> = sym(&r_lib, b"log_info");

        let m = CString::new("nope").unwrap();
        c_info(m.as_ptr()); // should not crash, no log file
        r_info(m.as_ptr());
    }
}

// ----------------------------------------------------------------------------
// Test: create_task_manager / destroy_task_manager — struct contents match.
// ----------------------------------------------------------------------------
#[test]
fn test_task_manager_create_destroy_default() {
    let _g = TEST_LOCK.lock().unwrap();
    unsafe {
        let c_lib = load_lib(&c_lib_path());
        let r_lib = load_lib(&rust_lib_path());

        type Init = unsafe extern "C" fn() -> c_int;
        type Fini = unsafe extern "C" fn();
        type Create = unsafe extern "C" fn() -> *mut TaskManager;
        type Destroy = unsafe extern "C" fn(*mut TaskManager);

        // initialize so log_info inside create_task_manager has somewhere to go.
        let c_init: Symbol<Init> = sym(&c_lib, b"initialize_logger");
        let c_fini: Symbol<Fini> = sym(&c_lib, b"finalize_logger");
        let r_init: Symbol<Init> = sym(&r_lib, b"initialize_logger");
        let r_fini: Symbol<Fini> = sym(&r_lib, b"finalize_logger");

        let c_create: Symbol<Create> = sym(&c_lib, b"create_task_manager");
        let c_destroy: Symbol<Destroy> = sym(&c_lib, b"destroy_task_manager");
        let r_create: Symbol<Create> = sym(&r_lib, b"create_task_manager");
        let r_destroy: Symbol<Destroy> = sym(&r_lib, b"destroy_task_manager");

        std::env::remove_var("MAX_TASKS");

        // C side
        let (_, c_log) = with_log_file(|| {
            assert_eq!(c_init(), 0);
            let m = c_create();
            assert!(!m.is_null());
            assert_eq!((*m).max_tasks, 10);
            assert_eq!((*m).task_count, 0);
            assert!(!(*m).tasks.is_null());
            c_destroy(m);
            c_fini();
        });

        // Rust side
        let (_, r_log) = with_log_file(|| {
            assert_eq!(r_init(), 0);
            let m = r_create();
            assert!(!m.is_null());
            assert_eq!((*m).max_tasks, 10);
            assert_eq!((*m).task_count, 0);
            assert!(!(*m).tasks.is_null());
            r_destroy(m);
            r_fini();
        });

        let c_bytes = read_file(&c_log);
        let r_bytes = read_file(&r_log);
        assert_eq!(c_bytes, r_bytes, "logger output differs");

        let _ = fs::remove_file(&c_log);
        let _ = fs::remove_file(&r_log);
    }
}

// ----------------------------------------------------------------------------
// Test: create_task_manager picks up MAX_TASKS env.
// ----------------------------------------------------------------------------
#[test]
fn test_task_manager_max_tasks_env() {
    let _g = TEST_LOCK.lock().unwrap();
    unsafe {
        let c_lib = load_lib(&c_lib_path());
        let r_lib = load_lib(&rust_lib_path());

        type Create = unsafe extern "C" fn() -> *mut TaskManager;
        type Destroy = unsafe extern "C" fn(*mut TaskManager);
        type Init = unsafe extern "C" fn() -> c_int;
        type Fini = unsafe extern "C" fn();

        let c_create: Symbol<Create> = sym(&c_lib, b"create_task_manager");
        let c_destroy: Symbol<Destroy> = sym(&c_lib, b"destroy_task_manager");
        let r_create: Symbol<Create> = sym(&r_lib, b"create_task_manager");
        let r_destroy: Symbol<Destroy> = sym(&r_lib, b"destroy_task_manager");

        let c_init: Symbol<Init> = sym(&c_lib, b"initialize_logger");
        let c_fini: Symbol<Fini> = sym(&c_lib, b"finalize_logger");
        let r_init: Symbol<Init> = sym(&r_lib, b"initialize_logger");
        let r_fini: Symbol<Fini> = sym(&r_lib, b"finalize_logger");

        std::env::set_var("MAX_TASKS", "25");

        let (_, c_log) = with_log_file(|| {
            assert_eq!(c_init(), 0);
            let m = c_create();
            assert!(!m.is_null());
            assert_eq!((*m).max_tasks, 25);
            c_destroy(m);
            c_fini();
        });

        let (_, r_log) = with_log_file(|| {
            assert_eq!(r_init(), 0);
            let m = r_create();
            assert!(!m.is_null());
            assert_eq!((*m).max_tasks, 25);
            r_destroy(m);
            r_fini();
        });

        std::env::remove_var("MAX_TASKS");

        let c_bytes = read_file(&c_log);
        let r_bytes = read_file(&r_log);
        assert_eq!(c_bytes, r_bytes, "logger output differs (MAX_TASKS)");
        let _ = fs::remove_file(&c_log);
        let _ = fs::remove_file(&r_log);
    }
}

// ----------------------------------------------------------------------------
// Test: add_task — task buffer bytes and counters match between C and Rust.
// ----------------------------------------------------------------------------
#[test]
fn test_add_task() {
    let _g = TEST_LOCK.lock().unwrap();
    unsafe {
        let c_lib = load_lib(&c_lib_path());
        let r_lib = load_lib(&rust_lib_path());

        type Init = unsafe extern "C" fn() -> c_int;
        type Fini = unsafe extern "C" fn();
        type Create = unsafe extern "C" fn() -> *mut TaskManager;
        type Destroy = unsafe extern "C" fn(*mut TaskManager);
        type Add = unsafe extern "C" fn(*mut TaskManager, *const c_char, c_int);

        let c_init: Symbol<Init> = sym(&c_lib, b"initialize_logger");
        let c_fini: Symbol<Fini> = sym(&c_lib, b"finalize_logger");
        let c_create: Symbol<Create> = sym(&c_lib, b"create_task_manager");
        let c_destroy: Symbol<Destroy> = sym(&c_lib, b"destroy_task_manager");
        let c_add: Symbol<Add> = sym(&c_lib, b"add_task");

        let r_init: Symbol<Init> = sym(&r_lib, b"initialize_logger");
        let r_fini: Symbol<Fini> = sym(&r_lib, b"finalize_logger");
        let r_create: Symbol<Create> = sym(&r_lib, b"create_task_manager");
        let r_destroy: Symbol<Destroy> = sym(&r_lib, b"destroy_task_manager");
        let r_add: Symbol<Add> = sym(&r_lib, b"add_task");

        std::env::set_var("MAX_TASKS", "3"); // limit to 3, try to add 4

        let descs = [
            CString::new("alpha").unwrap(),
            CString::new("beta task").unwrap(),
            CString::new("gamma task with more characters").unwrap(),
            CString::new("this is too many").unwrap(),
        ];

        // We capture each task buffer's full 256 bytes for comparison.
        unsafe fn snapshot(m: *mut TaskManager) -> (c_int, c_int, Vec<Vec<u8>>, Vec<c_int>) {
            let count = (*m).task_count;
            let mut bufs: Vec<Vec<u8>> = Vec::new();
            let mut prios: Vec<c_int> = Vec::new();
            for i in 0..count {
                let t = (*m).tasks.add(i as usize);
                let buf: Vec<u8> = (*t)
                    .description
                    .iter()
                    .map(|&c| c as u8)
                    .collect();
                bufs.push(buf);
                prios.push((*t).priority);
            }
            ((*m).max_tasks, count, bufs, prios)
        }

        let (_, c_log) = with_log_file(|| {
            assert_eq!(c_init(), 0);
            let m = c_create();
            for (i, d) in descs.iter().enumerate() {
                c_add(m, d.as_ptr(), (i + 1) as c_int);
            }
            let snap = snapshot(m);
            // store snap in a global side-channel via leaking
            STORE.with(|s| *s.borrow_mut() = Some(snap));
            c_destroy(m);
            c_fini();
        });
        let c_snap = STORE.with(|s| s.borrow_mut().take().unwrap());

        let (_, r_log) = with_log_file(|| {
            assert_eq!(r_init(), 0);
            let m = r_create();
            for (i, d) in descs.iter().enumerate() {
                r_add(m, d.as_ptr(), (i + 1) as c_int);
            }
            let snap = snapshot(m);
            STORE.with(|s| *s.borrow_mut() = Some(snap));
            r_destroy(m);
            r_fini();
        });
        let r_snap = STORE.with(|s| s.borrow_mut().take().unwrap());

        std::env::remove_var("MAX_TASKS");

        assert_eq!(c_snap.0, r_snap.0, "max_tasks differ");
        assert_eq!(c_snap.1, r_snap.1, "task_count differ");
        assert_eq!(c_snap.2, r_snap.2, "task description bytes differ");
        assert_eq!(c_snap.3, r_snap.3, "task priorities differ");

        let c_bytes = read_file(&c_log);
        let r_bytes = read_file(&r_log);
        assert_eq!(c_bytes, r_bytes, "logger output differs (add_task)");
        let _ = fs::remove_file(&c_log);
        let _ = fs::remove_file(&r_log);
    }
}

// thread-local snapshot store so we can return an opaque value from a closure.
thread_local! {
    static STORE: std::cell::RefCell<Option<(c_int, c_int, Vec<Vec<u8>>, Vec<c_int>)>> =
        std::cell::RefCell::new(None);
}

// ----------------------------------------------------------------------------
// Test: add_task with truncation (description longer than 255 bytes).
// ----------------------------------------------------------------------------
#[test]
fn test_add_task_truncation() {
    let _g = TEST_LOCK.lock().unwrap();
    unsafe {
        let c_lib = load_lib(&c_lib_path());
        let r_lib = load_lib(&rust_lib_path());

        type Init = unsafe extern "C" fn() -> c_int;
        type Fini = unsafe extern "C" fn();
        type Create = unsafe extern "C" fn() -> *mut TaskManager;
        type Destroy = unsafe extern "C" fn(*mut TaskManager);
        type Add = unsafe extern "C" fn(*mut TaskManager, *const c_char, c_int);

        let c_init: Symbol<Init> = sym(&c_lib, b"initialize_logger");
        let c_fini: Symbol<Fini> = sym(&c_lib, b"finalize_logger");
        let c_create: Symbol<Create> = sym(&c_lib, b"create_task_manager");
        let c_destroy: Symbol<Destroy> = sym(&c_lib, b"destroy_task_manager");
        let c_add: Symbol<Add> = sym(&c_lib, b"add_task");

        let r_init: Symbol<Init> = sym(&r_lib, b"initialize_logger");
        let r_fini: Symbol<Fini> = sym(&r_lib, b"finalize_logger");
        let r_create: Symbol<Create> = sym(&r_lib, b"create_task_manager");
        let r_destroy: Symbol<Destroy> = sym(&r_lib, b"destroy_task_manager");
        let r_add: Symbol<Add> = sym(&r_lib, b"add_task");

        std::env::remove_var("MAX_TASKS");

        // Build a description of length 300 (longer than 255).
        let big = "x".repeat(300);
        let bigc = CString::new(big.clone()).unwrap();

        unsafe fn buf_of(m: *mut TaskManager, idx: c_int) -> Vec<u8> {
            let t = (*m).tasks.add(idx as usize);
            (*t).description.iter().map(|&c| c as u8).collect()
        }

        let (_, c_log) = with_log_file(|| {
            assert_eq!(c_init(), 0);
            let m = c_create();
            c_add(m, bigc.as_ptr(), 7);
            let buf = buf_of(m, 0);
            STORE2.with(|s| *s.borrow_mut() = Some(buf));
            c_destroy(m);
            c_fini();
        });
        let c_buf = STORE2.with(|s| s.borrow_mut().take().unwrap());

        let (_, r_log) = with_log_file(|| {
            assert_eq!(r_init(), 0);
            let m = r_create();
            r_add(m, bigc.as_ptr(), 7);
            let buf = buf_of(m, 0);
            STORE2.with(|s| *s.borrow_mut() = Some(buf));
            r_destroy(m);
            r_fini();
        });
        let r_buf = STORE2.with(|s| s.borrow_mut().take().unwrap());

        assert_eq!(c_buf, r_buf, "truncated description buffers differ");
        // Sanity: terminator is at position 255.
        assert_eq!(c_buf[255], 0u8);
        // First 255 bytes should be 'x'
        for i in 0..255 {
            assert_eq!(c_buf[i], b'x', "byte {} not 'x'", i);
        }

        let c_bytes = read_file(&c_log);
        let r_bytes = read_file(&r_log);
        assert_eq!(c_bytes, r_bytes, "logger output differs (truncation)");
        let _ = fs::remove_file(&c_log);
        let _ = fs::remove_file(&r_log);
    }
}

thread_local! {
    static STORE2: std::cell::RefCell<Option<Vec<u8>>> = std::cell::RefCell::new(None);
}

// ----------------------------------------------------------------------------
// Test: print_tasks — captured stdout must match.
// ----------------------------------------------------------------------------
#[test]
fn test_print_tasks() {
    let _g = TEST_LOCK.lock().unwrap();
    unsafe {
        let c_lib = load_lib(&c_lib_path());
        let r_lib = load_lib(&rust_lib_path());

        type Init = unsafe extern "C" fn() -> c_int;
        type Fini = unsafe extern "C" fn();
        type Create = unsafe extern "C" fn() -> *mut TaskManager;
        type Destroy = unsafe extern "C" fn(*mut TaskManager);
        type Add = unsafe extern "C" fn(*mut TaskManager, *const c_char, c_int);
        type Print = unsafe extern "C" fn(*const TaskManager);

        let c_init: Symbol<Init> = sym(&c_lib, b"initialize_logger");
        let c_fini: Symbol<Fini> = sym(&c_lib, b"finalize_logger");
        let c_create: Symbol<Create> = sym(&c_lib, b"create_task_manager");
        let c_destroy: Symbol<Destroy> = sym(&c_lib, b"destroy_task_manager");
        let c_add: Symbol<Add> = sym(&c_lib, b"add_task");
        let c_print: Symbol<Print> = sym(&c_lib, b"print_tasks");

        let r_init: Symbol<Init> = sym(&r_lib, b"initialize_logger");
        let r_fini: Symbol<Fini> = sym(&r_lib, b"finalize_logger");
        let r_create: Symbol<Create> = sym(&r_lib, b"create_task_manager");
        let r_destroy: Symbol<Destroy> = sym(&r_lib, b"destroy_task_manager");
        let r_add: Symbol<Add> = sym(&r_lib, b"add_task");
        let r_print: Symbol<Print> = sym(&r_lib, b"print_tasks");

        std::env::remove_var("MAX_TASKS");

        let descs = [
            CString::new("first task").unwrap(),
            CString::new("second").unwrap(),
            CString::new("third!!").unwrap(),
        ];

        // C side
        let (_, c_log) = with_log_file(|| {
            assert_eq!(c_init(), 0);
            let m = c_create();
            for (i, d) in descs.iter().enumerate() {
                c_add(m, d.as_ptr(), (i + 1) as c_int);
            }
            let (saved, path) = redirect_stdout();
            c_print(m);
            libc::fflush(std::ptr::null_mut());
            restore_stdout(saved);
            STORE_PATH.with(|s| *s.borrow_mut() = Some(path));
            c_destroy(m);
            c_fini();
        });
        let c_stdout_path = STORE_PATH.with(|s| s.borrow_mut().take().unwrap());

        let (_, r_log) = with_log_file(|| {
            assert_eq!(r_init(), 0);
            let m = r_create();
            for (i, d) in descs.iter().enumerate() {
                r_add(m, d.as_ptr(), (i + 1) as c_int);
            }
            let (saved, path) = redirect_stdout();
            r_print(m);
            libc::fflush(std::ptr::null_mut());
            restore_stdout(saved);
            STORE_PATH.with(|s| *s.borrow_mut() = Some(path));
            r_destroy(m);
            r_fini();
        });
        let r_stdout_path = STORE_PATH.with(|s| s.borrow_mut().take().unwrap());

        let c_out = read_file(&c_stdout_path);
        let r_out = read_file(&r_stdout_path);
        assert_eq!(
            c_out,
            r_out,
            "print_tasks output differs:\nC:    {:?}\nRust: {:?}",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );

        let c_log_bytes = read_file(&c_log);
        let r_log_bytes = read_file(&r_log);
        assert_eq!(c_log_bytes, r_log_bytes, "log files differ for print_tasks test");

        let _ = fs::remove_file(&c_stdout_path);
        let _ = fs::remove_file(&r_stdout_path);
        let _ = fs::remove_file(&c_log);
        let _ = fs::remove_file(&r_log);
    }
}

thread_local! {
    static STORE_PATH: std::cell::RefCell<Option<PathBuf>> = std::cell::RefCell::new(None);
}

// ----------------------------------------------------------------------------
// Test: driver — full pipeline. Captured stdout AND log file must match.
// ----------------------------------------------------------------------------
fn run_driver_case(input: &str, max_tasks: Option<&str>) {
    let _g = TEST_LOCK.lock().unwrap();
    unsafe {
        let c_lib = load_lib(&c_lib_path());
        let r_lib = load_lib(&rust_lib_path());

        type Driver = unsafe extern "C" fn(*const c_char) -> c_int;
        let c_driver: Symbol<Driver> = sym(&c_lib, b"driver");
        let r_driver: Symbol<Driver> = sym(&r_lib, b"driver");

        if let Some(v) = max_tasks {
            std::env::set_var("MAX_TASKS", v);
        } else {
            std::env::remove_var("MAX_TASKS");
        }

        let inputc = CString::new(input).unwrap();

        let ((c_rc, c_stdout_path), c_log) = with_log_file(|| {
            let (saved, path) = redirect_stdout();
            let rc = c_driver(inputc.as_ptr());
            libc::fflush(std::ptr::null_mut());
            restore_stdout(saved);
            (rc, path)
        });

        let ((r_rc, r_stdout_path), r_log) = with_log_file(|| {
            let (saved, path) = redirect_stdout();
            let rc = r_driver(inputc.as_ptr());
            libc::fflush(std::ptr::null_mut());
            restore_stdout(saved);
            (rc, path)
        });

        std::env::remove_var("MAX_TASKS");

        assert_eq!(c_rc, r_rc, "driver return values differ");

        let c_out = read_file(&c_stdout_path);
        let r_out = read_file(&r_stdout_path);
        assert_eq!(
            c_out,
            r_out,
            "driver stdout differs (input={:?}):\nC:    {:?}\nRust: {:?}",
            input,
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );

        let c_log_bytes = read_file(&c_log);
        let r_log_bytes = read_file(&r_log);
        assert_eq!(
            c_log_bytes,
            r_log_bytes,
            "driver log differs (input={:?})",
            input
        );

        let _ = fs::remove_file(&c_stdout_path);
        let _ = fs::remove_file(&r_stdout_path);
        let _ = fs::remove_file(&c_log);
        let _ = fs::remove_file(&r_log);
    }
}

#[test]
fn test_driver_empty() {
    run_driver_case("", None);
}

#[test]
fn test_driver_single_no_newline() {
    run_driver_case("a single task", None);
}

#[test]
fn test_driver_multi_newline_separated() {
    run_driver_case("first\nsecond\nthird", None);
}

#[test]
fn test_driver_trailing_newline() {
    run_driver_case("first\nsecond\n", None);
}

#[test]
fn test_driver_blank_lines() {
    run_driver_case("\n\n\n", None);
}

#[test]
fn test_driver_max_tasks_exceeded() {
    run_driver_case("a\nb\nc\nd\ne\nf\ng\nh\ni\nj\nk\nl", Some("3"));
}
