use libloading::Library;
use std::env;
use std::ffi::{CString, OsStr, c_char, c_int, c_void};
use std::fs::{self, File};
use std::io::Read;
use std::mem::size_of;
use std::os::fd::{FromRawFd, RawFd};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::ptr;

#[repr(C)]
#[derive(Clone, Copy)]
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

type InitializeLogger = unsafe extern "C" fn() -> c_int;
type LogMessage = unsafe extern "C" fn(*const c_char);
type FinalizeLogger = unsafe extern "C" fn();
type CreateTaskManager = unsafe extern "C" fn() -> *mut TaskManager;
type AddTask = unsafe extern "C" fn(*mut TaskManager, *const c_char, c_int);
type PrintTasks = unsafe extern "C" fn(*const TaskManager);
type DestroyTaskManager = unsafe extern "C" fn(*mut TaskManager);
type Driver = unsafe extern "C" fn(*const c_char) -> c_int;

struct Api {
    _library: Library,
    initialize_logger: InitializeLogger,
    log_info: LogMessage,
    log_warning: LogMessage,
    log_error: LogMessage,
    finalize_logger: FinalizeLogger,
    create_task_manager: CreateTaskManager,
    add_task: AddTask,
    print_tasks: PrintTasks,
    destroy_task_manager: DestroyTaskManager,
    driver: Driver,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        unsafe {
            Self {
                initialize_logger: *library.get(b"initialize_logger\0").unwrap(),
                log_info: *library.get(b"log_info\0").unwrap(),
                log_warning: *library.get(b"log_warning\0").unwrap(),
                log_error: *library.get(b"log_error\0").unwrap(),
                finalize_logger: *library.get(b"finalize_logger\0").unwrap(),
                create_task_manager: *library.get(b"create_task_manager\0").unwrap(),
                add_task: *library.get(b"add_task\0").unwrap(),
                print_tasks: *library.get(b"print_tasks\0").unwrap(),
                destroy_task_manager: *library.get(b"destroy_task_manager\0").unwrap(),
                driver: *library.get(b"driver\0").unwrap(),
                _library: library,
            }
        }
    }
}

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
}

const STDOUT_FILENO: RawFd = 1;
const STDERR_FILENO: RawFd = 2;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("../c_src/build/libdriver.so")
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/release/libdriver.so")
}

fn test_root(name: &str) -> PathBuf {
    let path = env::temp_dir().join(format!("driver-differential-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

unsafe fn set_env(name: &str, value: Option<&OsStr>) {
    match value {
        Some(value) => unsafe { env::set_var(name, value) },
        None => unsafe { env::remove_var(name) },
    }
}

fn read_or_empty(path: &Path) -> Vec<u8> {
    fs::read(path).unwrap_or_default()
}

fn capture_fd<T>(fd: RawFd, operation: impl FnOnce() -> T) -> (T, Vec<u8>) {
    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        let mut pipe_fds = [-1; 2];
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0);
        let saved = dup(fd);
        assert!(saved >= 0);
        assert_eq!(dup2(pipe_fds[1], fd), fd);
        assert_eq!(close(pipe_fds[1]), 0);

        let result = operation();

        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(dup2(saved, fd), fd);
        assert_eq!(close(saved), 0);

        let mut output = Vec::new();
        File::from_raw_fd(pipe_fds[0])
            .read_to_end(&mut output)
            .unwrap();
        (result, output)
    }
}

fn capture_stdout<T>(operation: impl FnOnce() -> T) -> (T, Vec<u8>) {
    capture_fd(STDOUT_FILENO, operation)
}

fn capture_stderr<T>(operation: impl FnOnce() -> T) -> (T, Vec<u8>) {
    capture_fd(STDERR_FILENO, operation)
}

#[derive(Debug, PartialEq, Eq)]
struct ManagerSnapshot {
    max_tasks: c_int,
    task_count: c_int,
    tasks: Vec<(Vec<u8>, c_int)>,
}

unsafe fn manager_snapshot(manager: *const TaskManager) -> ManagerSnapshot {
    let manager = unsafe { &*manager };
    let mut tasks = Vec::new();
    for index in 0..manager.task_count.max(0) as usize {
        let task = unsafe { &*manager.tasks.add(index) };
        let description = task
            .description
            .iter()
            .map(|value| *value as u8)
            .take_while(|value| *value != 0)
            .collect();
        tasks.push((description, task.priority));
    }
    ManagerSnapshot {
        max_tasks: manager.max_tasks,
        task_count: manager.task_count,
        tasks,
    }
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 as u32
    }

    fn usize(&mut self, upper_exclusive: usize) -> usize {
        self.next_u32() as usize % upper_exclusive
    }

    fn priority(&mut self) -> c_int {
        self.next_u32() as c_int
    }

    fn bytes(&mut self, length: usize) -> Vec<u8> {
        (0..length)
            .map(|_| b'!' + self.usize((b'~' - b'!') as usize + 1) as u8)
            .collect()
    }
}

unsafe fn create_pair(
    c: &Api,
    rust: &Api,
    max_tasks: Option<&str>,
) -> (*mut TaskManager, *mut TaskManager) {
    unsafe { set_env("MAX_TASKS", max_tasks.map(OsStr::new)) };
    let c_manager = unsafe { (c.create_task_manager)() };
    let rust_manager = unsafe { (rust.create_task_manager)() };
    assert_eq!(c_manager.is_null(), rust_manager.is_null());
    if !c_manager.is_null() {
        assert_eq!(unsafe { manager_snapshot(c_manager) }, unsafe {
            manager_snapshot(rust_manager)
        });
    }
    (c_manager, rust_manager)
}

unsafe fn destroy_pair(c: &Api, rust: &Api, managers: (*mut TaskManager, *mut TaskManager)) {
    unsafe {
        (c.destroy_task_manager)(managers.0);
        (rust.destroy_task_manager)(managers.1);
    }
}

unsafe fn add_pair(
    c: &Api,
    rust: &Api,
    managers: (*mut TaskManager, *mut TaskManager),
    description: &[u8],
    priority: c_int,
) {
    let description = CString::new(description).unwrap();
    unsafe {
        (c.add_task)(managers.0, description.as_ptr(), priority);
        (rust.add_task)(managers.1, description.as_ptr(), priority);
        assert_eq!(manager_snapshot(managers.0), manager_snapshot(managers.1));
    }
}

unsafe fn exercise_task_manager(c: &Api, rust: &Api) {
    unsafe {
        set_env("LOG_FILE", None);

        let default_pair = create_pair(c, rust, None);
        assert_eq!(manager_snapshot(default_pair.0).max_tasks, 10);
        destroy_pair(c, rust, default_pair);

        for max_tasks in ["1", "2", "7", "31", " 5suffix"] {
            let pair = create_pair(c, rust, Some(max_tasks));
            assert_eq!(
                manager_snapshot(pair.0).max_tasks,
                max_tasks
                    .trim_start()
                    .trim_end_matches("suffix")
                    .parse()
                    .unwrap()
            );
            destroy_pair(c, rust, pair);
        }
        let mut rng = Rng::new(0x7adf_39e1_5120_884b);
        for _ in 0..64 {
            let max_tasks = (1 + rng.usize(128)).to_string();
            let pair = create_pair(c, rust, Some(&max_tasks));
            assert_eq!(
                manager_snapshot(pair.0).max_tasks,
                max_tasks.parse().unwrap()
            );
            destroy_pair(c, rust, pair);
        }

        for zero in ["0", "not-a-number", ""] {
            let pair = create_pair(c, rust, Some(zero));
            assert!(
                !pair.0.is_null(),
                "malloc(0) must match the successful C build"
            );
            assert_eq!(manager_snapshot(pair.0).max_tasks, 0);
            destroy_pair(c, rust, pair);
        }

        for _ in 0..64 {
            let lengths = [0, 1 + rng.usize(254), 255, 256 + rng.usize(768)];
            for length in lengths {
                let pair = create_pair(c, rust, Some("3"));
                let description = rng.bytes(length);
                add_pair(c, rust, pair, &description, rng.priority());
                let snapshot = manager_snapshot(pair.0);
                assert_eq!(snapshot.tasks[0].0, description[..length.min(255)]);
                destroy_pair(c, rust, pair);
            }
        }

        let full = create_pair(c, rust, Some("2"));
        add_pair(c, rust, full, b"first", c_int::MIN);
        add_pair(c, rust, full, b"second", c_int::MAX);
        let before = manager_snapshot(full.0);
        add_pair(c, rust, full, b"rejected-at-capacity", 42);
        assert_eq!(manager_snapshot(full.0), before);
        (*full.0).task_count = 3;
        (*full.1).task_count = 3;
        let rejected = CString::new("rejected-over-capacity").unwrap();
        (c.add_task)(full.0, rejected.as_ptr(), -42);
        (rust.add_task)(full.1, rejected.as_ptr(), -42);
        assert_eq!((*full.0).task_count, 3);
        assert_eq!((*full.1).task_count, 3);
        (*full.0).task_count = 2;
        (*full.1).task_count = 2;
        assert_eq!(manager_snapshot(full.0), before);
        assert_eq!(manager_snapshot(full.0), manager_snapshot(full.1));

        let ((), c_output) = capture_stdout(|| (c.print_tasks)(full.0));
        let ((), rust_output) = capture_stdout(|| (rust.print_tasks)(full.1));
        assert_eq!(c_output, rust_output);
        assert!(c_output.starts_with(b"Tasks:\n  [1] first"));
        destroy_pair(c, rust, full);

        for count in [0, 1, 2, 9] {
            let max = (count + 1).to_string();
            let pair = create_pair(c, rust, Some(&max));
            for index in 0..count {
                let length = match index {
                    0 => 0,
                    1 => 255,
                    _ => 1 + rng.usize(400),
                };
                add_pair(c, rust, pair, &rng.bytes(length), rng.priority());
            }
            let ((), c_output) = capture_stdout(|| (c.print_tasks)(pair.0));
            let ((), rust_output) = capture_stdout(|| (rust.print_tasks)(pair.1));
            assert_eq!(c_output, rust_output);
            destroy_pair(c, rust, pair);
        }
    }
}

unsafe fn exercise_logger(c: &Api, rust: &Api, root: &Path) {
    unsafe {
        let ignored = CString::new("ignored-before-initialize").unwrap();
        for logger in [c.log_info, c.log_warning, c.log_error] {
            logger(ignored.as_ptr());
        }
        for logger in [rust.log_info, rust.log_warning, rust.log_error] {
            logger(ignored.as_ptr());
        }
        (c.finalize_logger)();
        (rust.finalize_logger)();

        let original_dir = env::current_dir().unwrap();
        let c_default_dir = root.join("c-default");
        let rust_default_dir = root.join("rust-default");
        fs::create_dir_all(&c_default_dir).unwrap();
        fs::create_dir_all(&rust_default_dir).unwrap();
        set_env("LOG_FILE", None);
        env::set_current_dir(&c_default_dir).unwrap();
        let c_result = (c.initialize_logger)();
        (c.finalize_logger)();
        env::set_current_dir(&rust_default_dir).unwrap();
        let rust_result = (rust.initialize_logger)();
        (rust.finalize_logger)();
        env::set_current_dir(&original_dir).unwrap();
        assert_eq!(c_result, rust_result);
        assert_eq!(
            fs::read(c_default_dir.join("default.log")).unwrap(),
            fs::read(rust_default_dir.join("default.log")).unwrap()
        );

        let c_log = root.join("c-custom.log");
        let rust_log = root.join("rust-custom.log");
        set_env("LOG_FILE", Some(c_log.as_os_str()));
        assert_eq!((c.initialize_logger)(), 0);
        set_env("LOG_FILE", Some(rust_log.as_os_str()));
        assert_eq!((rust.initialize_logger)(), 0);

        let mut rng = Rng::new(0x1298_abcd_7711_04ef);
        let empty = CString::new("").unwrap();
        (c.log_info)(empty.as_ptr());
        (rust.log_info)(empty.as_ptr());
        (c.log_warning)(empty.as_ptr());
        (rust.log_warning)(empty.as_ptr());
        (c.log_error)(empty.as_ptr());
        (rust.log_error)(empty.as_ptr());
        for _ in 0..64 {
            let length = rng.usize(300);
            let message = CString::new(rng.bytes(length)).unwrap();
            (c.log_info)(message.as_ptr());
            (rust.log_info)(message.as_ptr());
            (c.log_warning)(message.as_ptr());
            (rust.log_warning)(message.as_ptr());
            (c.log_error)(message.as_ptr());
            (rust.log_error)(message.as_ptr());
        }
        (c.log_info)(ptr::null());
        (rust.log_info)(ptr::null());

        let populated = create_pair(c, rust, Some("2"));
        add_pair(c, rust, populated, b"logged task", 9);
        destroy_pair(c, rust, populated);

        (c.finalize_logger)();
        (rust.finalize_logger)();
        assert_eq!(fs::read(c_log).unwrap(), fs::read(rust_log).unwrap());
    }
}

struct DriverResult {
    return_value: c_int,
    stdout: Vec<u8>,
    log: Vec<u8>,
}

unsafe fn run_one_driver(
    api: &Api,
    input: &CString,
    max_tasks: Option<&str>,
    case_dir: &Path,
    default_log: bool,
) -> DriverResult {
    fs::create_dir_all(case_dir).unwrap();
    unsafe { set_env("MAX_TASKS", max_tasks.map(OsStr::new)) };
    let original_dir = env::current_dir().unwrap();
    let log_path = if default_log {
        unsafe { set_env("LOG_FILE", None) };
        env::set_current_dir(case_dir).unwrap();
        case_dir.join("default.log")
    } else {
        let path = case_dir.join("driver.log");
        unsafe { set_env("LOG_FILE", Some(path.as_os_str())) };
        path
    };
    let (return_value, stdout) = capture_stdout(|| unsafe { (api.driver)(input.as_ptr()) });
    env::set_current_dir(original_dir).unwrap();
    DriverResult {
        return_value,
        stdout,
        log: read_or_empty(&log_path),
    }
}

unsafe fn run_driver_pair(
    c: &Api,
    rust: &Api,
    input: Vec<u8>,
    max_tasks: Option<&str>,
    root: &Path,
    case: &str,
    default_log: bool,
) -> DriverResult {
    let input = CString::new(input).unwrap();
    let c_result = unsafe {
        run_one_driver(
            c,
            &input,
            max_tasks,
            &root.join(format!("{case}-c")),
            default_log,
        )
    };
    let rust_result = unsafe {
        run_one_driver(
            rust,
            &input,
            max_tasks,
            &root.join(format!("{case}-rust")),
            default_log,
        )
    };
    assert_eq!(c_result.return_value, rust_result.return_value, "{case}");
    assert_eq!(c_result.stdout, rust_result.stdout, "{case}");
    assert_eq!(c_result.log, rust_result.log, "{case}");
    c_result
}

unsafe fn exercise_driver(c: &Api, rust: &Api, root: &Path) {
    unsafe {
        let result = run_driver_pair(c, rust, Vec::new(), None, root, "empty-default", true);
        assert_eq!(result.stdout, b"Tasks:\n");

        let mut rng = Rng::new(0xd341_972c_a811_5e09);
        for iteration in 0..48 {
            let line_length = 1 + rng.usize(200);
            let line = rng.bytes(line_length);
            let result = run_driver_pair(
                c,
                rust,
                line.clone(),
                None,
                root,
                &format!("single-{iteration}"),
                false,
            );
            assert!(
                result
                    .stdout
                    .windows(line.len())
                    .any(|window| window == line)
            );

            let mut trailing = line.clone();
            trailing.push(b'\n');
            let trailing_result = run_driver_pair(
                c,
                rust,
                trailing,
                None,
                root,
                &format!("trailing-{iteration}"),
                false,
            );
            assert_eq!(result.stdout, trailing_result.stdout);

            let line_count = 2 + rng.usize(8);
            let lines: Vec<Vec<u8>> = (0..line_count)
                .map(|_| {
                    let length = 1 + rng.usize(80);
                    rng.bytes(length)
                })
                .collect();
            let joined = lines
                .iter()
                .enumerate()
                .flat_map(|(index, line)| {
                    let mut bytes = line.clone();
                    if index + 1 != lines.len() {
                        bytes.push(b'\n');
                    }
                    bytes
                })
                .collect();
            let many = run_driver_pair(
                c,
                rust,
                joined,
                Some(&(line_count + 2).to_string()),
                root,
                &format!("many-{iteration}"),
                false,
            );
            assert_eq!(
                many.stdout
                    .split(|byte| *byte == b'\n')
                    .filter(|line| line.starts_with(b"  ["))
                    .count(),
                line_count
            );
        }

        for iteration in 0..48 {
            let left_length = rng.usize(60);
            let left = rng.bytes(left_length);
            let right_length = rng.usize(60);
            let right = rng.bytes(right_length);
            let mut consecutive = left;
            consecutive.extend_from_slice(b"\n\n");
            consecutive.extend_from_slice(&right);
            let result = run_driver_pair(
                c,
                rust,
                consecutive,
                Some("8"),
                root,
                &format!("consecutive-{iteration}"),
                false,
            );
            assert!(
                result
                    .stdout
                    .windows(b"  [2]  (Priority: 2)".len())
                    .any(|window| window == b"  [2]  (Priority: 2)")
            );
        }

        for (iteration, length) in
            (0..48).flat_map(|iteration| [(iteration, 255), (iteration, 256 + iteration)])
        {
            let line = rng.bytes(length);
            let result = run_driver_pair(
                c,
                rust,
                line.clone(),
                Some("2"),
                root,
                &format!("long-{iteration}-{length}"),
                false,
            );
            assert!(
                result
                    .stdout
                    .windows(255)
                    .any(|window| window == &line[..255])
            );
            if length > 255 {
                assert!(!result.stdout.windows(length).any(|window| window == line));
            }
        }

        for iteration in 0..48 {
            let line_count = 2 + rng.usize(12);
            let input = (0..line_count)
                .flat_map(|index| {
                    let length = 1 + rng.usize(20);
                    let mut line = rng.bytes(length);
                    if index + 1 != line_count {
                        line.push(b'\n');
                    }
                    line
                })
                .collect::<Vec<_>>();

            let zero = run_driver_pair(
                c,
                rust,
                input.clone(),
                Some("0"),
                root,
                &format!("zero-cap-{iteration}"),
                false,
            );
            assert_eq!(zero.stdout, b"Tasks:\n");

            let smaller = 1 + rng.usize(line_count - 1);
            let limited = run_driver_pair(
                c,
                rust,
                input.clone(),
                Some(&smaller.to_string()),
                root,
                &format!("limited-{iteration}"),
                false,
            );
            assert_eq!(
                limited
                    .stdout
                    .split(|byte| *byte == b'\n')
                    .filter(|line| line.starts_with(b"  ["))
                    .count(),
                smaller
            );

            run_driver_pair(
                c,
                rust,
                input.clone(),
                Some(&line_count.to_string()),
                root,
                &format!("equal-{iteration}"),
                false,
            );
            run_driver_pair(
                c,
                rust,
                input,
                Some(&(line_count + 1 + rng.usize(20)).to_string()),
                root,
                &format!("greater-{iteration}"),
                false,
            );
        }
    }
}

unsafe fn exercise_nonallocation_errors(c: &Api, rust: &Api, root: &Path) {
    unsafe {
        let invalid = root.join("missing-parent").join("cannot-open.log");
        set_env("LOG_FILE", Some(invalid.as_os_str()));
        let (c_result, c_stderr) = capture_stderr(|| (c.initialize_logger)());
        let (rust_result, rust_stderr) = capture_stderr(|| (rust.initialize_logger)());
        assert_eq!(c_result, -1);
        assert_eq!(c_result, rust_result);
        assert_eq!(c_stderr, rust_stderr);

        let input = CString::new("task").unwrap();
        let (c_result, c_stderr) = capture_stderr(|| (c.driver)(input.as_ptr()));
        let (rust_result, rust_stderr) = capture_stderr(|| (rust.driver)(input.as_ptr()));
        assert_eq!(c_result, 1);
        assert_eq!(c_result, rust_result);
        assert_eq!(c_stderr, rust_stderr);

        set_env("LOG_FILE", None);
        let pair = create_pair(c, rust, Some("1"));
        add_pair(c, rust, pair, b"accepted", 1);
        let before = manager_snapshot(pair.0);
        add_pair(c, rust, pair, b"rejected", 2);
        assert_eq!(manager_snapshot(pair.0), before);
        destroy_pair(c, rust, pair);

        for oversized in ["-1", "-2147483648"] {
            let pair = create_pair(c, rust, Some(oversized));
            assert!(pair.0.is_null());
        }
    }
}

fn compile_malloc_shim() -> PathBuf {
    let output = manifest_dir().join("target/failmalloc.so");
    let status = Command::new("cc")
        .args(["-shared", "-fPIC", "-std=c11", "-O2"])
        .arg(manifest_dir().join("tests/support/failmalloc.c"))
        .arg("-o")
        .arg(&output)
        .status()
        .unwrap();
    assert!(status.success());
    output
}

fn run_inner_with_preload(shim: &Path) {
    let status = Command::new(env::current_exe().unwrap())
        .args(["--exact", "allocation_fault_inner", "--nocapture"])
        .env("LD_PRELOAD", shim)
        .env("DRIVER_ALLOCATION_INNER", "1")
        .status()
        .unwrap();
    assert!(status.success(), "allocation fault child failed: {status}");
}

#[test]
fn allocation_fault_inner() {
    if env::var_os("DRIVER_ALLOCATION_INNER").is_none() {
        return;
    }

    unsafe {
        let root = test_root("allocation-inner");
        let c = Api::load(&c_library_path());
        let rust = Api::load(&rust_library_path());
        let shim = Library::new(
            env::var_os("LD_PRELOAD").expect("LD_PRELOAD missing in allocation child"),
        )
        .unwrap();
        let fail: unsafe extern "C" fn(usize) = *shim.get(b"fail_one_malloc_of_size\0").unwrap();

        let c_log = root.join("manager-c.log");
        let rust_log = root.join("manager-rust.log");
        set_env("LOG_FILE", Some(c_log.as_os_str()));
        assert_eq!((c.initialize_logger)(), 0);
        set_env("LOG_FILE", Some(rust_log.as_os_str()));
        assert_eq!((rust.initialize_logger)(), 0);
        fail(size_of::<TaskManager>());
        assert!((c.create_task_manager)().is_null());
        fail(size_of::<TaskManager>());
        assert!((rust.create_task_manager)().is_null());
        (c.finalize_logger)();
        (rust.finalize_logger)();
        assert_eq!(fs::read(c_log).unwrap(), fs::read(rust_log).unwrap());

        set_env("MAX_TASKS", Some(OsStr::new("10")));
        let c_log = root.join("array-c.log");
        let rust_log = root.join("array-rust.log");
        set_env("LOG_FILE", Some(c_log.as_os_str()));
        assert_eq!((c.initialize_logger)(), 0);
        set_env("LOG_FILE", Some(rust_log.as_os_str()));
        assert_eq!((rust.initialize_logger)(), 0);
        fail(10 * size_of::<Task>());
        assert!((c.create_task_manager)().is_null());
        fail(10 * size_of::<Task>());
        assert!((rust.create_task_manager)().is_null());
        (c.finalize_logger)();
        (rust.finalize_logger)();
        assert_eq!(fs::read(c_log).unwrap(), fs::read(rust_log).unwrap());

        for (label, failed_size) in [
            ("driver-manager", size_of::<TaskManager>()),
            ("driver-array", 10 * size_of::<Task>()),
        ] {
            let c_path = root.join(format!("{label}-c.log"));
            let rust_path = root.join(format!("{label}-rust.log"));
            let input = CString::new("fault target").unwrap();
            set_env("LOG_FILE", Some(c_path.as_os_str()));
            fail(failed_size);
            let c_result = (c.driver)(input.as_ptr());
            set_env("LOG_FILE", Some(rust_path.as_os_str()));
            fail(failed_size);
            let rust_result = (rust.driver)(input.as_ptr());
            assert_eq!(c_result, rust_result);
            assert_eq!(c_result, 1);
            assert_eq!(read_or_empty(&c_path), read_or_empty(&rust_path));
        }

        let line = vec![b'x'; 136];
        let input = CString::new(line).unwrap();
        let c_path = root.join("driver-line-c.log");
        let rust_path = root.join("driver-line-rust.log");
        set_env("LOG_FILE", Some(c_path.as_os_str()));
        let (c_result, c_stderr) = capture_stderr(|| {
            fail(137);
            (c.driver)(input.as_ptr())
        });
        set_env("LOG_FILE", Some(rust_path.as_os_str()));
        let (rust_result, rust_stderr) = capture_stderr(|| {
            fail(137);
            (rust.driver)(input.as_ptr())
        });
        assert_eq!(c_result, rust_result);
        assert_eq!(c_result, 1);
        assert_eq!(c_stderr, rust_stderr);
        assert_eq!(fs::read(c_path).unwrap(), fs::read(rust_path).unwrap());
    }
}

fn probe_status(library: &Path, action: &str) -> ExitStatus {
    Command::new(env::current_exe().unwrap())
        .args(["--exact", "null_probe_inner", "--nocapture"])
        .env("DRIVER_NULL_PROBE_LIB", library)
        .env("DRIVER_NULL_PROBE_ACTION", action)
        .status()
        .unwrap()
}

#[test]
fn null_probe_inner() {
    let Some(library) = env::var_os("DRIVER_NULL_PROBE_LIB") else {
        return;
    };
    let action = env::var("DRIVER_NULL_PROBE_ACTION").unwrap();
    unsafe {
        let api = Api::load(Path::new(&library));
        match action.as_str() {
            "add-null-manager" => (api.add_task)(ptr::null_mut(), ptr::null(), 0),
            "print-null-manager" => (api.print_tasks)(ptr::null()),
            "destroy-null-manager" => (api.destroy_task_manager)(ptr::null_mut()),
            "driver-null-input" => {
                set_env("LOG_FILE", Some(OsStr::new("/dev/null")));
                set_env("MAX_TASKS", Some(OsStr::new("10")));
                (api.driver)(ptr::null());
            }
            "add-null-description" => {
                set_env("MAX_TASKS", Some(OsStr::new("1")));
                let manager = (api.create_task_manager)();
                (api.add_task)(manager, ptr::null(), 0);
            }
            _ => panic!("unknown null probe action: {action}"),
        }
    }
}

fn exercise_null_boundaries() {
    for action in [
        "add-null-manager",
        "print-null-manager",
        "destroy-null-manager",
        "driver-null-input",
        "add-null-description",
    ] {
        let c_status = probe_status(&c_library_path(), action);
        let rust_status = probe_status(&rust_library_path(), action);
        assert!(!c_status.success(), "C unexpectedly accepted {action}");
        assert!(
            !rust_status.success(),
            "Rust unexpectedly accepted {action}"
        );
        assert_eq!(
            c_status.signal(),
            rust_status.signal(),
            "different process-level rejection for {action}: C={c_status}, Rust={rust_status}"
        );
    }
}

#[test]
fn differential_suite() {
    assert!(
        c_library_path().is_file(),
        "build the C shared library first"
    );
    assert!(
        rust_library_path().is_file(),
        "build the Rust release shared library first"
    );

    let root = test_root("suite");
    unsafe {
        let c = Api::load(&c_library_path());
        let rust = Api::load(&rust_library_path());
        exercise_task_manager(&c, &rust);
        exercise_logger(&c, &rust, &root.join("logger"));
        exercise_driver(&c, &rust, &root.join("driver"));
        exercise_nonallocation_errors(&c, &rust, &root.join("errors"));
    }
    run_inner_with_preload(&compile_malloc_shim());
    exercise_null_boundaries();
}
