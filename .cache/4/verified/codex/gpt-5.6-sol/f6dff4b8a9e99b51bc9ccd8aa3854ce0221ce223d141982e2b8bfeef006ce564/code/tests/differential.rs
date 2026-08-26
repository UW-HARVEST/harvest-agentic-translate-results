use libloading::Library;
use std::env;
use std::ffi::{CString, c_char, c_int, c_void};
use std::fs::{self, File};
use std::os::fd::AsRawFd;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::sync::atomic::{AtomicU64, Ordering};

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
type Log = unsafe extern "C" fn(*const c_char);
type FinalizeLogger = unsafe extern "C" fn();
type CreateTaskManager = unsafe extern "C" fn() -> *mut TaskManager;
type AddTask = unsafe extern "C" fn(*mut TaskManager, *const c_char, c_int);
type PrintTasks = unsafe extern "C" fn(*const TaskManager);
type DestroyTaskManager = unsafe extern "C" fn(*mut TaskManager);
type Driver = unsafe extern "C" fn(*const c_char) -> c_int;

struct Api {
    _library: Library,
    initialize_logger: InitializeLogger,
    log_info: Log,
    log_warning: Log,
    log_error: Log,
    finalize_logger: FinalizeLogger,
    create_task_manager: CreateTaskManager,
    add_task: AddTask,
    print_tasks: PrintTasks,
    destroy_task_manager: DestroyTaskManager,
    driver: Driver,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }.unwrap_or_else(|error| {
            panic!("failed to load {}: {error}", path.display());
        });
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
}

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library() -> PathBuf {
    crate_root().join("c_src/build/libdriver.so")
}

fn rust_library() -> PathBuf {
    if let Some(path) = env::var_os("DIFF_RUST_SO") {
        return path.into();
    }
    let release = crate_root().join("target/release/libdriver.so");
    if release.exists() {
        release
    } else {
        crate_root().join("target/debug/libdriver.so")
    }
}

fn temp_dir(label: &str) -> PathBuf {
    let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
    let path = crate_root().join("target/differential").join(format!(
        "{}-{}-{id}",
        std::process::id(),
        label
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

fn set_env(name: &str, value: Option<&Path>) {
    unsafe {
        match value {
            Some(value) => env::set_var(name, value),
            None => env::remove_var(name),
        }
    }
}

fn set_env_text(name: &str, value: Option<&str>) {
    unsafe {
        match value {
            Some(value) => env::set_var(name, value),
            None => env::remove_var(name),
        }
    }
}

fn capture_fd<T>(fd: c_int, path: &Path, operation: impl FnOnce() -> T) -> (T, Vec<u8>) {
    let file = File::create(path).unwrap();
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(fd);
        assert!(saved >= 0);
        assert_eq!(dup2(file.as_raw_fd(), fd), fd);
        let result = operation();
        fflush(std::ptr::null_mut());
        assert_eq!(dup2(saved, fd), fd);
        close(saved);
        drop(file);
        (result, fs::read(path).unwrap())
    }
}

fn bytes(seed: &mut u64, length: usize) -> Vec<u8> {
    (0..length)
        .map(|_| {
            *seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            b'!' + ((*seed >> 33) % 90) as u8
        })
        .collect()
}

unsafe fn manager_snapshot(manager: *const TaskManager) -> (c_int, c_int, Vec<[u8; 260]>) {
    unsafe {
        let manager = &*manager;
        let mut tasks = Vec::new();
        for index in 0..manager.task_count {
            let task = &*manager.tasks.add(index as usize);
            let mut raw = [0_u8; 260];
            std::ptr::copy_nonoverlapping(
                task as *const Task as *const u8,
                raw.as_mut_ptr(),
                size_of::<Task>(),
            );
            tasks.push(raw);
        }
        (manager.max_tasks, manager.task_count, tasks)
    }
}

unsafe fn create_pair(
    c: &Api,
    rust: &Api,
    max_tasks: Option<&str>,
) -> (*mut TaskManager, *mut TaskManager) {
    set_env_text("MAX_TASKS", max_tasks);
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
        assert_eq!(manager_snapshot(managers.0), manager_snapshot(managers.1),);
    }
}

fn compare_task_manager_surface(c: &Api, rust: &Api) {
    unsafe {
        for max_tasks in [
            None,
            Some("1"),
            Some("7"),
            Some("31"),
            Some("0"),
            Some(""),
            Some("nope"),
        ] {
            let managers = create_pair(c, rust, max_tasks);
            if !managers.0.is_null() {
                destroy_pair(c, rust, managers);
            }
        }

        let managers = create_pair(c, rust, Some("96"));
        let mut seed = 0x4d59_5df4_d0f3_3173;
        for index in 0..32 {
            let length = (index * 37) % 255;
            let priority = ((seed >> 16) as i32).wrapping_sub(i32::MAX / 2);
            let description = bytes(&mut seed, length);
            add_pair(c, rust, managers, &description, priority);
        }
        add_pair(c, rust, managers, &bytes(&mut seed, 255), i32::MIN);
        for length in [256, 257, 511, 4096] {
            add_pair(c, rust, managers, &bytes(&mut seed, length), i32::MAX);
        }

        let c_output = capture_fd(1, &temp_dir("print-c").join("stdout"), || {
            (c.print_tasks)(managers.0)
        })
        .1;
        let rust_output = capture_fd(1, &temp_dir("print-rust").join("stdout"), || {
            (rust.print_tasks)(managers.1)
        })
        .1;
        assert_eq!(c_output, rust_output);
        destroy_pair(c, rust, managers);

        for count in [0, 1, 8] {
            let managers = create_pair(c, rust, Some("8"));
            for index in 0..count {
                add_pair(
                    c,
                    rust,
                    managers,
                    format!("task-{index}").as_bytes(),
                    index - 3,
                );
            }
            let c_output = capture_fd(1, &temp_dir("shape-c").join("stdout"), || {
                (c.print_tasks)(managers.0)
            })
            .1;
            let rust_output = capture_fd(1, &temp_dir("shape-rust").join("stdout"), || {
                (rust.print_tasks)(managers.1)
            })
            .1;
            assert_eq!(c_output, rust_output);
            destroy_pair(c, rust, managers);
        }

        for max_tasks in ["0", "3"] {
            let managers = create_pair(c, rust, Some(max_tasks));
            let capacity = max_tasks.parse::<usize>().unwrap();
            for index in 0..capacity {
                add_pair(
                    c,
                    rust,
                    managers,
                    format!("full-{index}").as_bytes(),
                    index as i32,
                );
            }
            let before = manager_snapshot(managers.0);
            for index in 0..12 {
                add_pair(
                    c,
                    rust,
                    managers,
                    format!("rejected-{index}").as_bytes(),
                    index,
                );
                assert_eq!(manager_snapshot(managers.0), before);
            }
            destroy_pair(c, rust, managers);
        }
    }
}

fn run_driver_case(c: &Api, rust: &Api, label: &str, input: &[u8], max_tasks: Option<&str>) {
    let dir = temp_dir(label);
    let c_log = dir.join("c.log");
    let rust_log = dir.join("rust.log");
    let input = CString::new(input).unwrap();
    set_env_text("MAX_TASKS", max_tasks);

    set_env("LOG_FILE", Some(&c_log));
    let (c_result, c_stdout) = capture_fd(1, &dir.join("c.stdout"), || unsafe {
        (c.driver)(input.as_ptr())
    });
    set_env("LOG_FILE", Some(&rust_log));
    let (rust_result, rust_stdout) = capture_fd(1, &dir.join("rust.stdout"), || unsafe {
        (rust.driver)(input.as_ptr())
    });

    assert_eq!(c_result, rust_result, "{label}: return value");
    assert_eq!(c_stdout, rust_stdout, "{label}: stdout");
    assert_eq!(
        fs::read(c_log).unwrap(),
        fs::read(rust_log).unwrap(),
        "{label}: log"
    );
}

fn compare_driver_surface(c: &Api, rust: &Api) {
    run_driver_case(c, rust, "driver-empty", b"", None);
    let mut seed = 0x9e37_79b9_7f4a_7c15;
    for index in 0..16 {
        let one = bytes(&mut seed, 1 + index * 13);
        run_driver_case(c, rust, &format!("driver-one-{index}"), &one, None);

        let lines = 2 + index % 7;
        let mut many = Vec::new();
        for line in 0..lines {
            if line > 0 {
                many.push(b'\n');
            }
            many.extend(bytes(&mut seed, 1 + (index + line) % 80));
        }
        run_driver_case(c, rust, &format!("driver-many-{index}"), &many, Some("20"));
    }

    for (index, shape) in [
        b"\n".as_slice(),
        b"\nalpha".as_slice(),
        b"alpha\n".as_slice(),
        b"alpha\n\nbeta".as_slice(),
        b"\n\n\n".as_slice(),
    ]
    .iter()
    .enumerate()
    {
        run_driver_case(
            c,
            rust,
            &format!("driver-newlines-{index}"),
            shape,
            Some("10"),
        );
    }

    for (index, length) in [255, 256, 257, 511, 4096].iter().enumerate() {
        let input = bytes(&mut seed, *length);
        run_driver_case(c, rust, &format!("driver-long-{index}"), &input, Some("3"));
    }

    for capacity in 0..8 {
        let input = (0..16)
            .map(|line| format!("task-{capacity}-{line}"))
            .collect::<Vec<_>>()
            .join("\n");
        run_driver_case(
            c,
            rust,
            &format!("driver-capacity-{capacity}"),
            input.as_bytes(),
            Some(&capacity.to_string()),
        );
    }
}

fn compare_logger_surface(c: &Api, rust: &Api) {
    let null = std::ptr::null();
    let ignored = CString::new("ignored while closed").unwrap();
    unsafe {
        for log in [c.log_info, c.log_warning, c.log_error] {
            log(null);
            log(ignored.as_ptr());
        }
        for log in [rust.log_info, rust.log_warning, rust.log_error] {
            log(null);
            log(ignored.as_ptr());
        }
        (c.finalize_logger)();
        (rust.finalize_logger)();
    }

    let dir = temp_dir("logger");
    let original_dir = env::current_dir().unwrap();
    let c_default_dir = dir.join("default-c");
    let rust_default_dir = dir.join("default-rust");
    fs::create_dir_all(&c_default_dir).unwrap();
    fs::create_dir_all(&rust_default_dir).unwrap();
    set_env("LOG_FILE", None);
    env::set_current_dir(&c_default_dir).unwrap();
    let c_result = unsafe { (c.initialize_logger)() };
    env::set_current_dir(&rust_default_dir).unwrap();
    let rust_result = unsafe { (rust.initialize_logger)() };
    env::set_current_dir(&original_dir).unwrap();
    unsafe { fflush(std::ptr::null_mut()) };
    assert_eq!(c_result, rust_result);
    assert_eq!(
        fs::read(c_default_dir.join("default.log")).unwrap(),
        fs::read(rust_default_dir.join("default.log")).unwrap()
    );

    let c_empty = dir.join("empty-c.log");
    let rust_empty = dir.join("empty-rust.log");
    File::create(&c_empty).unwrap();
    File::create(&rust_empty).unwrap();
    set_env("LOG_FILE", Some(&c_empty));
    let c_result = unsafe { (c.initialize_logger)() };
    set_env("LOG_FILE", Some(&rust_empty));
    let rust_result = unsafe { (rust.initialize_logger)() };
    unsafe { fflush(std::ptr::null_mut()) };
    assert_eq!(c_result, rust_result);
    assert_eq!(fs::read(&c_empty).unwrap(), fs::read(&rust_empty).unwrap());

    let prefix = b"existing-prefix\n";
    let c_append = dir.join("append-c.log");
    let rust_append = dir.join("append-rust.log");
    fs::write(&c_append, prefix).unwrap();
    fs::write(&rust_append, prefix).unwrap();
    set_env("LOG_FILE", Some(&c_append));
    assert_eq!(unsafe { (c.initialize_logger)() }, 0);
    set_env("LOG_FILE", Some(&rust_append));
    assert_eq!(unsafe { (rust.initialize_logger)() }, 0);

    unsafe {
        (c.log_info)(std::ptr::null());
        (rust.log_info)(std::ptr::null());
        (c.log_warning)(std::ptr::null());
        (rust.log_warning)(std::ptr::null());
        (c.log_error)(std::ptr::null());
        (rust.log_error)(std::ptr::null());
    }

    let mut seed = 0xd1b5_4a32_d192_ed03;
    for length in std::iter::once(0).chain((0..48).map(|index| 1 + index * 19)) {
        let message = CString::new(bytes(&mut seed, length)).unwrap();
        unsafe {
            (c.log_info)(message.as_ptr());
            (rust.log_info)(message.as_ptr());
            (c.log_warning)(message.as_ptr());
            (rust.log_warning)(message.as_ptr());
            (c.log_error)(message.as_ptr());
            (rust.log_error)(message.as_ptr());
        }
    }
    unsafe {
        (c.finalize_logger)();
        (rust.finalize_logger)();
    }
    assert_eq!(fs::read(c_append).unwrap(), fs::read(rust_append).unwrap());
}

fn compare_regular_error_surface(c: &Api, rust: &Api) {
    let dir = temp_dir("errors");
    let invalid = dir.join("missing/parent/log");
    set_env("LOG_FILE", Some(&invalid));
    let (c_result, c_stderr) = capture_fd(2, &dir.join("init-c.stderr"), || unsafe {
        (c.initialize_logger)()
    });
    let (rust_result, rust_stderr) = capture_fd(2, &dir.join("init-rust.stderr"), || unsafe {
        (rust.initialize_logger)()
    });
    assert_eq!(c_result, -1);
    assert_eq!(c_result, rust_result);
    assert_eq!(c_stderr, rust_stderr);

    set_env("LOG_FILE", Some(&invalid));
    let (c_result, c_stderr) = capture_fd(2, &dir.join("driver-init-c.stderr"), || unsafe {
        (c.driver)(c"x".as_ptr())
    });
    let (rust_result, rust_stderr) =
        capture_fd(2, &dir.join("driver-init-rust.stderr"), || unsafe {
            (rust.driver)(c"x".as_ptr())
        });
    assert_eq!(c_result, 1);
    assert_eq!(c_result, rust_result);
    assert_eq!(c_stderr, rust_stderr);

    set_env_text("MAX_TASKS", Some("-1"));
    let c_manager = unsafe { (c.create_task_manager)() };
    let rust_manager = unsafe { (rust.create_task_manager)() };
    assert!(c_manager.is_null());
    assert!(rust_manager.is_null());

    let c_log = dir.join("manager-c.log");
    let rust_log = dir.join("manager-rust.log");
    set_env("LOG_FILE", Some(&c_log));
    let c_result = unsafe { (c.driver)(c"x".as_ptr()) };
    set_env("LOG_FILE", Some(&rust_log));
    let rust_result = unsafe { (rust.driver)(c"x".as_ptr()) };
    assert_eq!(c_result, 1);
    assert_eq!(c_result, rust_result);
    unsafe { fflush(std::ptr::null_mut()) };
    assert_eq!(fs::read(c_log).unwrap(), fs::read(rust_log).unwrap());
}

#[test]
fn differential_surface() {
    assert!(c_library().exists(), "build the C shared object first");
    assert!(
        rust_library().exists(),
        "build the Rust shared object first"
    );
    let c = unsafe { Api::load(&c_library()) };
    let rust = unsafe { Api::load(&rust_library()) };

    compare_logger_surface(&c, &rust);
    compare_regular_error_surface(&c, &rust);
    compare_task_manager_surface(&c, &rust);
    compare_driver_surface(&c, &rust);
}

fn compile_malloc_shim() -> PathBuf {
    let output = crate_root().join("target/differential/libmalloc_fail.so");
    fs::create_dir_all(output.parent().unwrap()).unwrap();
    let status = Command::new("cc")
        .args(["-shared", "-fPIC", "-O2", "-o"])
        .arg(&output)
        .arg(crate_root().join("tests/support/malloc_fail.c"))
        .arg("-ldl")
        .status()
        .unwrap();
    assert!(status.success());
    output
}

fn run_allocation_child(kind: &str, library: &Path, output: &Path, shim: &Path) -> ExitStatus {
    fs::create_dir_all(output).unwrap();
    Command::new(env::current_exe().unwrap())
        .args(["--exact", "allocation_failure_child", "--nocapture"])
        .env("DIFF_CHILD_KIND", kind)
        .env("DIFF_CHILD_LIBRARY", library)
        .env("DIFF_CHILD_OUTPUT", output)
        .env("LD_PRELOAD", shim)
        .status()
        .unwrap()
}

fn compare_output_dirs(c_dir: &Path, rust_dir: &Path) {
    let mut names = fs::read_dir(c_dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect::<Vec<_>>();
    names.sort();
    let mut rust_names = fs::read_dir(rust_dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect::<Vec<_>>();
    rust_names.sort();
    assert_eq!(names, rust_names);
    for name in names {
        assert_eq!(
            fs::read(c_dir.join(&name)).unwrap(),
            fs::read(rust_dir.join(&name)).unwrap(),
            "allocation case output {}",
            name.to_string_lossy()
        );
    }
}

#[test]
fn allocation_failures_are_identical() {
    if env::var_os("DIFF_CHILD_KIND").is_some() {
        return;
    }
    let shim = compile_malloc_shim();
    for kind in ["manager", "line"] {
        let dir = temp_dir(&format!("allocation-{kind}"));
        let c_dir = dir.join("c");
        let rust_dir = dir.join("rust");
        assert!(run_allocation_child(kind, &c_library(), &c_dir, &shim).success());
        assert!(run_allocation_child(kind, &rust_library(), &rust_dir, &shim).success());
        compare_output_dirs(&c_dir, &rust_dir);
    }
}

#[test]
fn allocation_failure_child() {
    let Some(kind) = env::var_os("DIFF_CHILD_KIND") else {
        return;
    };
    let kind = kind.to_string_lossy();
    let output = PathBuf::from(env::var_os("DIFF_CHILD_OUTPUT").unwrap());
    let library = PathBuf::from(env::var_os("DIFF_CHILD_LIBRARY").unwrap());
    let api = unsafe { Api::load(&library) };
    let process = libloading::os::unix::Library::this();
    let arm = unsafe {
        *process
            .get::<unsafe extern "C" fn(usize, c_int)>(b"malloc_fail_arm\0")
            .unwrap()
    };
    let log = output.join("log");
    set_env("LOG_FILE", Some(&log));

    match kind.as_ref() {
        "manager" => unsafe {
            assert_eq!((api.initialize_logger)(), 0);
            arm(size_of::<TaskManager>(), 1);
            let manager = (api.create_task_manager)();
            let result: &[u8] = if manager.is_null() {
                b"null"
            } else {
                b"non-null"
            };
            fs::write(output.join("return"), result).unwrap();
            (api.finalize_logger)();
        },
        "line" => unsafe {
            set_env_text("MAX_TASKS", Some("2"));
            let input = CString::new(vec![b'x'; 4096]).unwrap();
            arm(4097, 1);
            let (result, stderr) = capture_fd(2, &output.join("captured.stderr"), || {
                (api.driver)(input.as_ptr())
            });
            fs::write(output.join("return"), result.to_string()).unwrap();
            fs::write(output.join("stderr"), stderr).unwrap();
        },
        _ => panic!("unknown allocation child kind"),
    }
    unsafe { fflush(std::ptr::null_mut()) };
}

fn run_null_child(case: &str, library: &Path, log: &Path) -> std::process::Output {
    Command::new(env::current_exe().unwrap())
        .args(["--exact", "null_boundary_child", "--nocapture"])
        .env("DIFF_NULL_CASE", case)
        .env("DIFF_NULL_LIBRARY", library)
        .env("LOG_FILE", log)
        .output()
        .unwrap()
}

#[test]
fn null_pointer_boundaries_match() {
    if env::var_os("DIFF_NULL_CASE").is_some() {
        return;
    }
    let dir = temp_dir("null-boundaries");
    for case in [
        "log-info-closed",
        "log-warning-closed",
        "log-error-closed",
        "add-manager",
        "add-description",
        "print-manager",
        "destroy-manager",
        "driver-tasks",
    ] {
        let c_output = run_null_child(case, &c_library(), &dir.join(format!("{case}-c.log")));
        let rust_output =
            run_null_child(case, &rust_library(), &dir.join(format!("{case}-rust.log")));
        assert_eq!(
            c_output.status.success(),
            rust_output.status.success(),
            "{case}"
        );
        assert_eq!(c_output.status.code(), rust_output.status.code(), "{case}");
        assert_eq!(
            c_output.status.signal(),
            rust_output.status.signal(),
            "{case}"
        );
        assert_eq!(c_output.stdout, rust_output.stdout, "{case}: stdout");
        assert_eq!(c_output.stderr, rust_output.stderr, "{case}: stderr");
    }
}

#[test]
fn null_boundary_child() {
    let Some(case) = env::var_os("DIFF_NULL_CASE") else {
        return;
    };
    set_env_text("MAX_TASKS", Some("2"));
    let library = PathBuf::from(env::var_os("DIFF_NULL_LIBRARY").unwrap());
    let api = unsafe { Api::load(&library) };
    let null_char: *const c_char = std::ptr::null();
    let null_manager: *const TaskManager = std::ptr::null();
    let null_manager_mut: *mut TaskManager = std::ptr::null_mut();
    unsafe {
        match case.to_string_lossy().as_ref() {
            "log-info-closed" => (api.log_info)(null_char),
            "log-warning-closed" => (api.log_warning)(null_char),
            "log-error-closed" => (api.log_error)(null_char),
            "add-manager" => (api.add_task)(null_manager_mut, c"x".as_ptr(), 1),
            "add-description" => {
                let manager = (api.create_task_manager)();
                (api.add_task)(manager, null_char, 1);
            }
            "print-manager" => (api.print_tasks)(null_manager),
            "destroy-manager" => (api.destroy_task_manager)(null_manager_mut),
            "driver-tasks" => {
                (api.driver)(null_char);
            }
            _ => panic!("unknown null boundary case"),
        }
    }
}
