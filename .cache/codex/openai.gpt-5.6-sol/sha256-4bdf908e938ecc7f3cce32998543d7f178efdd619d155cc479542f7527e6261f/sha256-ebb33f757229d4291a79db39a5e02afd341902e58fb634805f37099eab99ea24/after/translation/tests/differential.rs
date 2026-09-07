use libloading::{Library, Symbol};
use std::env;
use std::ffi::{CString, c_char, c_int};
use std::fs;
use std::mem::size_of;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

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

type InitializeLogger = unsafe extern "C" fn() -> c_int;
type Log = unsafe extern "C" fn(*const c_char);
type FinalizeLogger = unsafe extern "C" fn();
type CreateTaskManager = unsafe extern "C" fn() -> *mut TaskManager;
type AddTask = unsafe extern "C" fn(*mut TaskManager, *const c_char, c_int);
type PrintTasks = unsafe extern "C" fn(*const TaskManager);
type DestroyTaskManager = unsafe extern "C" fn(*mut TaskManager);
type Driver = unsafe extern "C" fn(*const c_char) -> c_int;
type FailAfter = unsafe extern "C" fn(i64);
type FailSize = unsafe extern "C" fn(usize);

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
struct Observation {
    status_code: Option<i32>,
    signal: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    result: Option<Vec<u8>>,
    log: Option<Vec<u8>>,
}

fn temp_dir(label: &str) -> PathBuf {
    let sequence = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
    let path = env::temp_dir().join(format!(
        "driver-diff-{}-{}-{}",
        std::process::id(),
        sequence,
        label
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

fn compile_failmalloc(crate_dir: &Path) -> PathBuf {
    let output = crate_dir.join("target/failmalloc.so");
    let status = Command::new("cc")
        .args(["-shared", "-fPIC", "-std=c11", "-O2"])
        .arg(crate_dir.join("tests/failmalloc.c"))
        .args(["-o"])
        .arg(&output)
        .status()
        .expect("run cc for failmalloc");
    assert!(status.success(), "failed to compile failmalloc.so");
    output
}

fn observe(
    library: &Path,
    scenario: &str,
    seed: u64,
    max_tasks: Option<&str>,
    log_mode: &str,
    preload: Option<&Path>,
) -> Observation {
    let directory = temp_dir(if library.to_string_lossy().contains("c_src") {
        "c"
    } else {
        "rust"
    });
    let result_path = directory.join("result.bin");
    let explicit_log = directory.join("explicit.log");
    let executable = env::current_exe().unwrap();

    let mut command = Command::new(executable);
    command
        .current_dir(&directory)
        .args(["--exact", "ffi_worker", "--nocapture", "--test-threads=1"])
        .env("DIFF_WORKER", "1")
        .env("DRIVER_LIB", library)
        .env("SCENARIO", scenario)
        .env("SEED", seed.to_string())
        .env("RESULT_PATH", &result_path)
        .env_remove("MAX_TASKS")
        .env_remove("LOG_FILE");

    if let Some(value) = max_tasks {
        command.env("MAX_TASKS", value);
    }
    match log_mode {
        "default" => {}
        "explicit" => {
            command.env("LOG_FILE", &explicit_log);
        }
        "invalid" => {
            command.env(
                "LOG_FILE",
                "/proc/driver-differential-missing-parent/log.txt",
            );
        }
        other => panic!("unknown log mode {other}"),
    }
    if let Some(shim) = preload {
        command.env("LD_PRELOAD", shim);
    }

    let Output {
        status,
        stdout,
        stderr,
    } = command.output().expect("run worker process");
    let log_path = if log_mode == "default" {
        directory.join("default.log")
    } else {
        explicit_log
    };
    let observation = Observation {
        status_code: status.code(),
        signal: status.signal(),
        stdout,
        stderr,
        result: fs::read(&result_path).ok(),
        log: fs::read(&log_path).ok(),
    };
    fs::remove_dir_all(directory).unwrap();
    observation
}

fn assert_pair(
    c_library: &Path,
    rust_library: &Path,
    scenario: &str,
    seed: u64,
    max_tasks: Option<&str>,
    log_mode: &str,
    preload: Option<&Path>,
) -> Observation {
    let c = observe(c_library, scenario, seed, max_tasks, log_mode, preload);
    let rust = observe(rust_library, scenario, seed, max_tasks, log_mode, preload);
    assert_eq!(c.status_code, rust.status_code, "{scenario} status code");
    assert_eq!(c.signal, rust.signal, "{scenario} signal");
    assert_eq!(c.stdout, rust.stdout, "{scenario} stdout");
    assert_eq!(c.stderr, rust.stderr, "{scenario} stderr");
    assert_eq!(c.result, rust.result, "{scenario} result bytes");
    assert_eq!(c.log, rust.log, "{scenario} log bytes");
    c
}

fn next_random(state: &mut u64) -> u64 {
    let mut value = *state;
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    *state = value;
    value
}

fn bytes_for(seed: &mut u64, length: usize) -> Vec<u8> {
    (0..length)
        .map(|_| b'a' + (next_random(seed) % 26) as u8)
        .collect()
}

fn random_length(seed: &mut u64, minimum: usize, span: usize) -> usize {
    minimum + next_random(seed) as usize % span
}

fn c_string(bytes: Vec<u8>) -> CString {
    CString::new(bytes).unwrap()
}

unsafe fn symbol<'a, T>(library: &'a Library, name: &[u8]) -> Symbol<'a, T> {
    unsafe { library.get(name).unwrap() }
}

fn write_result(bytes: &[u8]) {
    fs::write(env::var_os("RESULT_PATH").unwrap(), bytes).unwrap();
}

unsafe fn dump_manager(manager: *const TaskManager) -> Vec<u8> {
    let manager = unsafe { &*manager };
    let mut output = Vec::new();
    output.extend_from_slice(&manager.max_tasks.to_ne_bytes());
    output.extend_from_slice(&manager.task_count.to_ne_bytes());
    for index in 0..manager.task_count {
        let task = unsafe { &*manager.tasks.offset(index as isize) };
        let description = unsafe {
            std::slice::from_raw_parts(
                task.description.as_ptr().cast::<u8>(),
                task.description.len(),
            )
        };
        output.extend_from_slice(description);
        output.extend_from_slice(&task.priority.to_ne_bytes());
    }
    output
}

unsafe fn fail_after(allocation_number: i64) {
    let shim_path = env::var_os("LD_PRELOAD").expect("allocator shim is preloaded");
    let shim = unsafe { Library::new(shim_path).unwrap() };
    let configure: Symbol<FailAfter> = unsafe { symbol(&shim, b"failmalloc_after\0") };
    unsafe { configure(allocation_number) };
}

unsafe fn fail_size(size: usize) {
    let shim_path = env::var_os("LD_PRELOAD").expect("allocator shim is preloaded");
    let shim = unsafe { Library::new(shim_path).unwrap() };
    let configure: Symbol<FailSize> = unsafe { symbol(&shim, b"failmalloc_size\0") };
    unsafe { configure(size) };
}

fn driver_input(scenario: &str, seed: &mut u64) -> Vec<u8> {
    let max_tasks = env::var("MAX_TASKS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(10);
    match scenario {
        "driver_empty" => Vec::new(),
        "driver_single" => {
            let length = random_length(seed, 1, 220);
            bytes_for(seed, length)
        }
        "driver_leading" => {
            let mut input = vec![b'\n'];
            let length = random_length(seed, 1, 80);
            input.extend(bytes_for(seed, length));
            input
        }
        "driver_consecutive" => {
            let first_length = random_length(seed, 1, 40);
            let mut input = bytes_for(seed, first_length);
            input.extend_from_slice(b"\n\n");
            let second_length = random_length(seed, 1, 40);
            input.extend(bytes_for(seed, second_length));
            input
        }
        "driver_trailing" => {
            let first_length = random_length(seed, 1, 40);
            let mut input = bytes_for(seed, first_length);
            input.push(b'\n');
            let second_length = random_length(seed, 1, 40);
            input.extend(bytes_for(seed, second_length));
            input.push(b'\n');
            input
        }
        "driver_under" => {
            let count = random_length(seed, 2, 7);
            joined_lines(seed, count)
        }
        "driver_exact" => joined_lines(seed, max_tasks),
        "driver_over" => joined_lines(seed, max_tasks.saturating_add(5)),
        "driver_long" => {
            let mut lines = Vec::new();
            for length in [255, 256, 257, 300, 511] {
                lines.push(bytes_for(seed, length));
            }
            for _ in 0..8 {
                let length = random_length(seed, 256, 1000);
                lines.push(bytes_for(seed, length));
            }
            lines.join(&b'\n')
        }
        "driver_zero" => joined_lines(seed, 5),
        "driver_fail_task" => bytes_for(seed, 1234),
        other => panic!("not a driver input scenario: {other}"),
    }
}

fn joined_lines(seed: &mut u64, count: usize) -> Vec<u8> {
    let mut lines = Vec::new();
    for _ in 0..count {
        let length = random_length(seed, 1, 90);
        lines.push(bytes_for(seed, length));
    }
    lines.join(&b'\n')
}

#[test]
fn ffi_worker() {
    if env::var_os("DIFF_WORKER").is_none() {
        return;
    }

    let library_path = env::var_os("DRIVER_LIB").unwrap();
    let scenario = env::var("SCENARIO").unwrap();
    let mut seed = env::var("SEED").unwrap().parse::<u64>().unwrap();
    let library = unsafe { Library::new(library_path).unwrap() };

    unsafe {
        match scenario.as_str() {
            "logger_closed" => {
                let info: Symbol<Log> = symbol(&library, b"log_info\0");
                let warning: Symbol<Log> = symbol(&library, b"log_warning\0");
                let error: Symbol<Log> = symbol(&library, b"log_error\0");
                for index in 0..64 {
                    let message = c_string(bytes_for(&mut seed, index % 33));
                    match index % 3 {
                        0 => info(message.as_ptr()),
                        1 => warning(message.as_ptr()),
                        _ => error(message.as_ptr()),
                    }
                }
                write_result(b"closed logger calls completed");
            }
            "finalize_closed" => {
                let finalize: Symbol<FinalizeLogger> = symbol(&library, b"finalize_logger\0");
                finalize();
                write_result(b"closed finalize completed");
            }
            "logger_active" => {
                let initialize: Symbol<InitializeLogger> = symbol(&library, b"initialize_logger\0");
                let info: Symbol<Log> = symbol(&library, b"log_info\0");
                let warning: Symbol<Log> = symbol(&library, b"log_warning\0");
                let error: Symbol<Log> = symbol(&library, b"log_error\0");
                let finalize: Symbol<FinalizeLogger> = symbol(&library, b"finalize_logger\0");
                let result = initialize();
                for index in 0..64 {
                    let message = c_string(bytes_for(&mut seed, index % 41));
                    match index % 3 {
                        0 => info(message.as_ptr()),
                        1 => warning(message.as_ptr()),
                        _ => error(message.as_ptr()),
                    }
                }
                finalize();
                write_result(&result.to_ne_bytes());
            }
            "logger_null_closed" => {
                let info: Symbol<Log> = symbol(&library, b"log_info\0");
                let warning: Symbol<Log> = symbol(&library, b"log_warning\0");
                let error: Symbol<Log> = symbol(&library, b"log_error\0");
                info(std::ptr::null());
                warning(std::ptr::null());
                error(std::ptr::null());
                write_result(b"null closed completed");
            }
            "logger_null_open" => {
                let initialize: Symbol<InitializeLogger> = symbol(&library, b"initialize_logger\0");
                let info: Symbol<Log> = symbol(&library, b"log_info\0");
                let warning: Symbol<Log> = symbol(&library, b"log_warning\0");
                let error: Symbol<Log> = symbol(&library, b"log_error\0");
                let finalize: Symbol<FinalizeLogger> = symbol(&library, b"finalize_logger\0");
                let result = initialize();
                info(std::ptr::null());
                warning(std::ptr::null());
                error(std::ptr::null());
                finalize();
                write_result(&result.to_ne_bytes());
            }
            "manager_basic" => {
                let create: Symbol<CreateTaskManager> = symbol(&library, b"create_task_manager\0");
                let print: Symbol<PrintTasks> = symbol(&library, b"print_tasks\0");
                let destroy: Symbol<DestroyTaskManager> =
                    symbol(&library, b"destroy_task_manager\0");
                let manager = create();
                if manager.is_null() {
                    write_result(b"NULL");
                } else {
                    print(manager);
                    write_result(&dump_manager(manager));
                    destroy(manager);
                }
            }
            "manager_shapes" => {
                let create: Symbol<CreateTaskManager> = symbol(&library, b"create_task_manager\0");
                let add: Symbol<AddTask> = symbol(&library, b"add_task\0");
                let print: Symbol<PrintTasks> = symbol(&library, b"print_tasks\0");
                let destroy: Symbol<DestroyTaskManager> =
                    symbol(&library, b"destroy_task_manager\0");
                let manager = create();
                assert!(!manager.is_null());
                let mut lengths = vec![0, 1, 2, 10, 254, 255, 256, 257, 300, 511];
                for _ in 0..48 {
                    lengths.push(next_random(&mut seed) as usize % 600);
                }
                let priorities = [c_int::MIN, -1000, -1, 0, 1, 1000, c_int::MAX];
                for (index, length) in lengths.into_iter().enumerate() {
                    let description = c_string(bytes_for(&mut seed, length));
                    add(
                        manager,
                        description.as_ptr(),
                        priorities[index % priorities.len()],
                    );
                }
                print(manager);
                write_result(&dump_manager(manager));
                destroy(manager);
            }
            "manager_one" => {
                let create: Symbol<CreateTaskManager> = symbol(&library, b"create_task_manager\0");
                let add: Symbol<AddTask> = symbol(&library, b"add_task\0");
                let print: Symbol<PrintTasks> = symbol(&library, b"print_tasks\0");
                let destroy: Symbol<DestroyTaskManager> =
                    symbol(&library, b"destroy_task_manager\0");
                let manager = create();
                assert!(!manager.is_null());
                let priorities = [c_int::MIN, -1, 0, 1, c_int::MAX];
                let priority = priorities[next_random(&mut seed) as usize % priorities.len()];
                let length = random_length(&mut seed, 0, 600);
                let description = c_string(bytes_for(&mut seed, length));
                add(manager, description.as_ptr(), priority);
                print(manager);
                write_result(&dump_manager(manager));
                destroy(manager);
            }
            "manager_full" => {
                let create: Symbol<CreateTaskManager> = symbol(&library, b"create_task_manager\0");
                let add: Symbol<AddTask> = symbol(&library, b"add_task\0");
                let print: Symbol<PrintTasks> = symbol(&library, b"print_tasks\0");
                let destroy: Symbol<DestroyTaskManager> =
                    symbol(&library, b"destroy_task_manager\0");
                let manager = create();
                assert!(!manager.is_null());
                let capacity = (*manager).max_tasks;
                for priority in 0..capacity {
                    let length = random_length(&mut seed, 0, 400);
                    let description = c_string(bytes_for(&mut seed, length));
                    add(manager, description.as_ptr(), priority);
                }
                let before = dump_manager(manager);
                let rejected = c_string(bytes_for(&mut seed, 333));
                add(manager, rejected.as_ptr(), c_int::MAX);
                let after = dump_manager(manager);
                print(manager);
                let mut result = before;
                result.extend_from_slice(b"AFTER");
                result.extend_from_slice(&after);
                write_result(&result);
                destroy(manager);
            }
            "create_fail_first" | "create_fail_second" => {
                let initialize: Symbol<InitializeLogger> = symbol(&library, b"initialize_logger\0");
                let create: Symbol<CreateTaskManager> = symbol(&library, b"create_task_manager\0");
                let finalize: Symbol<FinalizeLogger> = symbol(&library, b"finalize_logger\0");
                assert_eq!(initialize(), 0);
                fail_after(if scenario == "create_fail_first" {
                    1
                } else {
                    2
                });
                let manager = create();
                write_result(&[manager.is_null() as u8]);
                finalize();
            }
            "initialize_fail" => {
                let initialize: Symbol<InitializeLogger> = symbol(&library, b"initialize_logger\0");
                write_result(&initialize().to_ne_bytes());
            }
            "driver_manager_fail" => {
                let driver: Symbol<Driver> = symbol(&library, b"driver\0");
                let input = c_string(bytes_for(&mut seed, 20));
                write_result(&driver(input.as_ptr()).to_ne_bytes());
            }
            "driver_manager_alloc_fail" => {
                let driver: Symbol<Driver> = symbol(&library, b"driver\0");
                let input = c_string(bytes_for(&mut seed, 20));
                fail_size(size_of::<TaskManager>());
                write_result(&driver(input.as_ptr()).to_ne_bytes());
            }
            "driver_fail_task" => {
                let driver: Symbol<Driver> = symbol(&library, b"driver\0");
                let input = c_string(driver_input(&scenario, &mut seed));
                fail_size(1235);
                write_result(&driver(input.as_ptr()).to_ne_bytes());
            }
            "null_add_manager" => {
                let add: Symbol<AddTask> = symbol(&library, b"add_task\0");
                let description = c_string(b"task".to_vec());
                add(std::ptr::null_mut(), description.as_ptr(), 1);
            }
            "null_add_description" => {
                let create: Symbol<CreateTaskManager> = symbol(&library, b"create_task_manager\0");
                let add: Symbol<AddTask> = symbol(&library, b"add_task\0");
                let manager = create();
                add(manager, std::ptr::null(), 1);
            }
            "null_print_manager" => {
                let print: Symbol<PrintTasks> = symbol(&library, b"print_tasks\0");
                print(std::ptr::null());
            }
            "null_destroy_manager" => {
                let destroy: Symbol<DestroyTaskManager> =
                    symbol(&library, b"destroy_task_manager\0");
                destroy(std::ptr::null_mut());
            }
            "null_driver_tasks" => {
                let driver: Symbol<Driver> = symbol(&library, b"driver\0");
                driver(std::ptr::null());
            }
            scenario if scenario.starts_with("driver_") => {
                let driver: Symbol<Driver> = symbol(&library, b"driver\0");
                let input = c_string(driver_input(scenario, &mut seed));
                write_result(&driver(input.as_ptr()).to_ne_bytes());
            }
            other => panic!("unknown worker scenario {other}"),
        }
    }
}

#[test]
fn differential_surface() {
    if env::var_os("DIFF_WORKER").is_some() {
        return;
    }

    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_library = crate_dir
        .join("../c_src/build/libdriver.so")
        .canonicalize()
        .expect("build the C shared library first");
    let rust_library = crate_dir
        .join("target/release/libdriver.so")
        .canonicalize()
        .expect("build the Rust release cdylib first");
    let failmalloc = compile_failmalloc(&crate_dir)
        .canonicalize()
        .expect("canonical failmalloc path");

    // Logger configurations, including many fixed-seed randomized messages.
    assert_pair(
        &c_library,
        &rust_library,
        "logger_closed",
        0x1001,
        None,
        "explicit",
        None,
    );
    assert_pair(
        &c_library,
        &rust_library,
        "finalize_closed",
        0x1002,
        None,
        "explicit",
        None,
    );
    assert_pair(
        &c_library,
        &rust_library,
        "logger_active",
        0x1003,
        None,
        "default",
        None,
    );
    for seed in 0x1010..0x1018 {
        assert_pair(
            &c_library,
            &rust_library,
            "logger_active",
            seed,
            None,
            "explicit",
            None,
        );
    }

    // Direct low-level manager API: capacities, shapes, boundaries, printing,
    // rejection at capacity, and destruction.
    for capacity in [
        None,
        Some("1"),
        Some("2"),
        Some("3"),
        Some("5"),
        Some("7"),
        Some("11"),
        Some("17"),
        Some("31"),
        Some("63"),
        Some("0"),
        Some(""),
        Some("words"),
    ] {
        assert_pair(
            &c_library,
            &rust_library,
            "manager_basic",
            0x2001,
            capacity,
            "explicit",
            None,
        );
    }
    for seed in 0x2010..0x2018 {
        assert_pair(
            &c_library,
            &rust_library,
            "manager_one",
            seed,
            Some("1"),
            "explicit",
            None,
        );
        assert_pair(
            &c_library,
            &rust_library,
            "manager_shapes",
            seed,
            Some("80"),
            "explicit",
            None,
        );
        assert_pair(
            &c_library,
            &rust_library,
            "manager_full",
            seed,
            Some("17"),
            "explicit",
            None,
        );
    }

    // End-to-end driver configurations. Each shape is sampled repeatedly.
    for scenario in [
        "driver_empty",
        "driver_single",
        "driver_leading",
        "driver_consecutive",
        "driver_trailing",
        "driver_under",
        "driver_exact",
        "driver_over",
        "driver_long",
    ] {
        for seed in 0x3010..0x3018 {
            let capacity = match scenario {
                "driver_exact" | "driver_over" => Some("9"),
                _ => Some("10"),
            };
            assert_pair(
                &c_library,
                &rust_library,
                scenario,
                seed,
                capacity,
                "explicit",
                None,
            );
        }
    }
    for max_tasks in ["0", "nonnumeric"] {
        for seed in 0x3020..0x3028 {
            assert_pair(
                &c_library,
                &rust_library,
                "driver_zero",
                seed,
                Some(max_tasks),
                "explicit",
                None,
            );
        }
    }
    assert_pair(
        &c_library,
        &rust_library,
        "driver_single",
        0x3030,
        None,
        "default",
        None,
    );

    // Explicit error branches. Validate the C result as well as parity.
    let observation = assert_pair(
        &c_library,
        &rust_library,
        "initialize_fail",
        0x4001,
        None,
        "invalid",
        None,
    );
    assert_eq!(observation.result, Some((-1_i32).to_ne_bytes().to_vec()));

    for (scenario, expected_log) in [
        (
            "create_fail_first",
            b"[ERROR] Failed to allocate memory for TaskManager.\n".as_slice(),
        ),
        (
            "create_fail_second",
            b"[ERROR] Failed to allocate memory for tasks.\n".as_slice(),
        ),
    ] {
        let observation = assert_pair(
            &c_library,
            &rust_library,
            scenario,
            0x4010,
            None,
            "explicit",
            Some(&failmalloc),
        );
        assert_eq!(observation.result, Some(vec![1]));
        assert!(
            observation
                .log
                .as_deref()
                .unwrap()
                .windows(expected_log.len())
                .any(|window| window == expected_log),
            "{scenario} did not reach expected C log branch"
        );
    }

    let observation = assert_pair(
        &c_library,
        &rust_library,
        "driver_manager_fail",
        0x4020,
        None,
        "invalid",
        None,
    );
    assert_eq!(observation.result, Some(1_i32.to_ne_bytes().to_vec()));

    let observation = assert_pair(
        &c_library,
        &rust_library,
        "driver_manager_fail",
        0x4021,
        Some("-1"),
        "explicit",
        None,
    );
    assert_eq!(observation.result, Some(1_i32.to_ne_bytes().to_vec()));

    let observation = assert_pair(
        &c_library,
        &rust_library,
        "driver_manager_alloc_fail",
        0x4022,
        None,
        "explicit",
        Some(&failmalloc),
    );
    assert_eq!(observation.result, Some(1_i32.to_ne_bytes().to_vec()));

    let observation = assert_pair(
        &c_library,
        &rust_library,
        "driver_fail_task",
        0x4023,
        None,
        "explicit",
        Some(&failmalloc),
    );
    assert_eq!(observation.result, Some(1_i32.to_ne_bytes().to_vec()));
    assert!(
        observation
            .stderr
            .windows(b"Error: Failed to allocate memory for task.\n".len())
            .any(|window| window == b"Error: Failed to allocate memory for task.\n")
    );

    // Generic FFI boundaries are isolated because the C contract has no guard.
    for scenario in [
        "null_add_manager",
        "null_add_description",
        "null_print_manager",
        "null_destroy_manager",
        "null_driver_tasks",
    ] {
        let observation = assert_pair(
            &c_library,
            &rust_library,
            scenario,
            0x4030,
            None,
            "explicit",
            None,
        );
        assert!(
            observation.signal.is_some() || observation.status_code != Some(0),
            "{scenario} unexpectedly succeeded"
        );
    }
    assert_pair(
        &c_library,
        &rust_library,
        "logger_null_closed",
        0x4040,
        None,
        "explicit",
        None,
    );
    assert_pair(
        &c_library,
        &rust_library,
        "logger_null_open",
        0x4041,
        None,
        "explicit",
        None,
    );
}
