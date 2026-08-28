use libloading::{Library, Symbol};
use std::env;
use std::ffi::{CString, OsString, c_char, c_int, c_void};
use std::fmt::Debug;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::Mutex;

const ENV_NAMES: [&str; 5] = [
    "PROG_VERBOSE",
    "PROG_DEBUG",
    "PROG_OPTIMIZE",
    "PROG_BASE_OFFSET",
    "PROG_MULTIPLIER",
];
const RANDOM_CASES: usize = 32;
static ENV_LOCK: Mutex<()> = Mutex::new(());

type ParseEnvNumeric = unsafe extern "C" fn(*const c_char, c_int) -> c_int;
type InitConfig = unsafe extern "C" fn(*mut u32);
type PerformOperation = unsafe extern "C" fn(c_int, c_int, *mut u32) -> c_int;
type ApplyBitOperations = unsafe extern "C" fn(c_int, *mut u32) -> c_int;
type Envy = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

unsafe extern "C" {
    fn pipe(fds: *mut c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
    fn fflush(stream: *mut c_void) -> c_int;
}

struct Api {
    library: Library,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        Self {
            library: unsafe { Library::new(path) }
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display())),
        }
    }

    unsafe fn parse_env_numeric(&self, name: *const c_char, default: c_int) -> c_int {
        let function: Symbol<ParseEnvNumeric> =
            unsafe { self.library.get(b"parse_env_numeric\0") }.unwrap();
        unsafe { function(name, default) }
    }

    unsafe fn init_config_from_env(&self, flags: *mut u32) {
        let function: Symbol<InitConfig> =
            unsafe { self.library.get(b"init_config_from_env\0") }.unwrap();
        unsafe { function(flags) };
    }

    unsafe fn perform_operation(&self, first: c_int, second: c_int, flags: *mut u32) -> c_int {
        let function: Symbol<PerformOperation> =
            unsafe { self.library.get(b"perform_operation\0") }.unwrap();
        unsafe { function(first, second, flags) }
    }

    unsafe fn apply_bit_operations(&self, value: c_int, flags: *mut u32) -> c_int {
        let function: Symbol<ApplyBitOperations> =
            unsafe { self.library.get(b"apply_bit_operations\0") }.unwrap();
        unsafe { function(value, flags) }
    }

    unsafe fn envy(&self, first: c_int, second: c_int, third: c_int, fourth: c_int) -> c_int {
        let function: Symbol<Envy> = unsafe { self.library.get(b"envy\0") }.unwrap();
        unsafe { function(first, second, third, fourth) }
    }
}

struct EnvironmentSnapshot(Vec<(&'static str, Option<OsString>)>);

impl EnvironmentSnapshot {
    fn clear() -> Self {
        let values = ENV_NAMES
            .iter()
            .map(|name| (*name, env::var_os(name)))
            .collect();
        for name in ENV_NAMES {
            unsafe { env::remove_var(name) };
        }
        Self(values)
    }
}

impl Drop for EnvironmentSnapshot {
    fn drop(&mut self) {
        for (name, value) in &self.0 {
            match value {
                Some(value) => unsafe { env::set_var(name, value) },
                None => unsafe { env::remove_var(name) },
            }
        }
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

    fn range(&mut self, minimum: i32, maximum: i32) -> i32 {
        let width = (maximum as i64 - minimum as i64 + 1) as u32;
        minimum + (self.next_u32() % width) as i32
    }
}

fn library_paths() -> (PathBuf, PathBuf) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_path = manifest
        .parent()
        .unwrap()
        .join("c_src/build/libharvest-work-GjQeyl.so");
    let test_executable = env::current_exe().unwrap();
    let profile_dir = test_executable.parent().unwrap().parent().unwrap();
    let profile_rust_path = profile_dir.join("libenvy_lib.so");
    let rust_path = if profile_rust_path.is_file() {
        profile_rust_path
    } else {
        manifest.join("target/release/libenvy_lib.so")
    };

    assert!(c_path.is_file(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.is_file(),
        "missing Rust cdylib: {}",
        rust_path.display()
    );
    (c_path, rust_path)
}

fn set_env(name: &str, value: Option<&str>) {
    match value {
        Some(value) => unsafe { env::set_var(name, value) },
        None => unsafe { env::remove_var(name) },
    }
}

fn set_boolean_env(name: &str, enabled: bool, iteration: usize) {
    if enabled {
        set_env(name, Some(if iteration % 2 == 0 { "1" } else { "x1x" }));
    } else if iteration % 2 == 0 {
        set_env(name, None);
    } else {
        set_env(name, Some(if iteration % 4 == 1 { "" } else { "off" }));
    }
}

fn read_pipe(fd: c_int) -> Vec<u8> {
    let mut output = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let count = unsafe { read(fd, buffer.as_mut_ptr().cast(), buffer.len()) };
        assert!(count >= 0, "read from capture pipe failed");
        if count == 0 {
            break;
        }
        output.extend_from_slice(&buffer[..count as usize]);
    }
    assert_eq!(unsafe { close(fd) }, 0);
    output
}

fn capture_output<T>(call: impl FnOnce() -> T) -> (T, Vec<u8>, Vec<u8>) {
    let mut stdout_pipe = [-1; 2];
    let mut stderr_pipe = [-1; 2];
    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(pipe(stdout_pipe.as_mut_ptr()), 0);
        assert_eq!(pipe(stderr_pipe.as_mut_ptr()), 0);
    }

    let saved_stdout = unsafe { dup(1) };
    let saved_stderr = unsafe { dup(2) };
    assert!(saved_stdout >= 0 && saved_stderr >= 0);
    unsafe {
        assert_eq!(dup2(stdout_pipe[1], 1), 1);
        assert_eq!(dup2(stderr_pipe[1], 2), 2);
    }

    let result = call();

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(dup2(saved_stderr, 2), 2);
        assert_eq!(close(saved_stdout), 0);
        assert_eq!(close(saved_stderr), 0);
        assert_eq!(close(stdout_pipe[1]), 0);
        assert_eq!(close(stderr_pipe[1]), 0);
    }

    (result, read_pipe(stdout_pipe[0]), read_pipe(stderr_pipe[0]))
}

fn compare_calls<T: Debug + Eq>(
    label: &str,
    c_call: impl FnOnce() -> T,
    rust_call: impl FnOnce() -> T,
) -> T {
    let c_output = capture_output(c_call);
    let rust_output = capture_output(rust_call);
    assert_eq!(c_output, rust_output, "{label}");
    c_output.0
}

fn raw_flags(
    verbose: bool,
    debug: bool,
    optimize: bool,
    cache: bool,
    log_level: u32,
    high_bits: u32,
) -> u32 {
    u32::from(verbose)
        | (u32::from(debug) << 1)
        | (u32::from(optimize) << 2)
        | (u32::from(cache) << 3)
        | ((log_level & 7) << 4)
        | (high_bits & !0xff)
}

#[test]
fn phase_b_valid_paths() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _environment = EnvironmentSnapshot::clear();
    let (c_path, rust_path) = library_paths();
    let c = unsafe { Api::load(&c_path) };
    let rust = unsafe { Api::load(&rust_path) };
    let mut rng = Rng::new(0x7cb2_91e4_f035_a86d);

    let parse_name = CString::new("DIFF_PARSE_VALUE").unwrap();
    unsafe { env::remove_var("DIFF_PARSE_VALUE") };
    for iteration in 0..RANDOM_CASES {
        let default = rng.next_u32() as i32;
        compare_calls(
            &format!("CONFIGS row 1, case {iteration}"),
            || unsafe { c.parse_env_numeric(parse_name.as_ptr(), default) },
            || unsafe { rust.parse_env_numeric(parse_name.as_ptr(), default) },
        );
    }

    for iteration in 0..RANDOM_CASES {
        let number = rng.range(-1_000_000, 1_000_000);
        let value = match iteration % 6 {
            0 => number.to_string(),
            1 => format!("  {number}tail"),
            2 => String::new(),
            3 => "not-a-number".to_owned(),
            4 => format!("+{}", number.unsigned_abs()),
            _ => format!("\t{number} "),
        };
        unsafe { env::set_var("DIFF_PARSE_VALUE", value) };
        compare_calls(
            &format!("CONFIGS row 2, case {iteration}"),
            || unsafe { c.parse_env_numeric(parse_name.as_ptr(), number) },
            || unsafe { rust.parse_env_numeric(parse_name.as_ptr(), number) },
        );
    }
    unsafe { env::remove_var("DIFF_PARSE_VALUE") };

    let mut row = 3;
    for verbose in [false, true] {
        for debug in [false, true] {
            for optimize in [false, true] {
                for iteration in 0..RANDOM_CASES {
                    set_boolean_env("PROG_VERBOSE", verbose, iteration);
                    set_boolean_env("PROG_DEBUG", debug, iteration + 1);
                    set_env(
                        "PROG_OPTIMIZE",
                        optimize.then_some(if iteration % 2 == 0 { "" } else { "enabled" }),
                    );
                    let initial = rng.next_u32();
                    let mut c_flags = initial;
                    let mut rust_flags = initial;
                    compare_calls(
                        &format!("CONFIGS row {row}, case {iteration}"),
                        || unsafe { c.init_config_from_env(&mut c_flags) },
                        || unsafe { rust.init_config_from_env(&mut rust_flags) },
                    );
                    assert_eq!(c_flags, rust_flags, "CONFIGS row {row}, case {iteration}");
                    assert_eq!(c_flags & 1 != 0, verbose);
                    assert_eq!(c_flags & 2 != 0, debug);
                    assert_eq!(c_flags & 4 != 0, optimize);
                    assert_eq!(c_flags & 8, 8);
                    assert_eq!((c_flags >> 4) & 7, 3);
                    assert_eq!(c_flags & 0xffff_ff00, initial & 0xffff_ff00);
                }
                row += 1;
            }
        }
    }

    for optimize in [false, true] {
        for debug in [false, true] {
            for iteration in 0..RANDOM_CASES {
                let first = rng.range(-1_000_000, 1_000_000);
                let second = rng.range(-1_000_000, 1_000_000);
                let log_level = if iteration == 0 {
                    8
                } else {
                    rng.next_u32() % 8
                };
                let flags = if log_level == 8 {
                    raw_flags(false, debug, optimize, false, 0, rng.next_u32()) | (8 << 4)
                } else {
                    raw_flags(false, debug, optimize, false, log_level, rng.next_u32())
                };
                let mut c_flags = flags;
                let mut rust_flags = flags;
                compare_calls(
                    &format!("CONFIGS row {row}, case {iteration}"),
                    || unsafe { c.perform_operation(first, second, &mut c_flags) },
                    || unsafe { rust.perform_operation(first, second, &mut rust_flags) },
                );
            }
            row += 1;
        }
    }

    for verbose in [false, true] {
        for cache in [false, true] {
            for iteration in 0..RANDOM_CASES {
                let value = rng.range(-1_000_000, 1_000_000);
                let flags = raw_flags(
                    verbose,
                    rng.next_u32() & 1 != 0,
                    rng.next_u32() & 1 != 0,
                    cache,
                    rng.next_u32() % 8,
                    rng.next_u32(),
                );
                let mut c_flags = flags;
                let mut rust_flags = flags;
                compare_calls(
                    &format!("CONFIGS row {row}, case {iteration}"),
                    || unsafe { c.apply_bit_operations(value, &mut c_flags) },
                    || unsafe { rust.apply_bit_operations(value, &mut rust_flags) },
                );
            }
            row += 1;
        }
    }

    for verbose in [false, true] {
        for debug in [false, true] {
            for optimize in [false, true] {
                for explicit_base in [false, true] {
                    for explicit_multiplier in [false, true] {
                        for nonzero_third in [false, true] {
                            for nonzero_fourth in [false, true] {
                                for negative in [false, true] {
                                    for iteration in 0..RANDOM_CASES {
                                        set_boolean_env("PROG_VERBOSE", verbose, iteration);
                                        set_boolean_env("PROG_DEBUG", debug, iteration + 1);
                                        set_env(
                                            "PROG_OPTIMIZE",
                                            optimize.then_some(if iteration % 2 == 0 {
                                                ""
                                            } else {
                                                "0"
                                            }),
                                        );

                                        let sign = if negative { -1 } else { 1 };
                                        let first = sign * rng.range(1_000, 10_000);
                                        let second = sign * rng.range(1_000, 10_000);
                                        let third = if nonzero_third {
                                            sign * rng.range(1, 1_000)
                                        } else {
                                            0
                                        };
                                        let fourth = if nonzero_fourth {
                                            sign * rng.range(1, 1_000)
                                        } else {
                                            0
                                        };
                                        let base = if explicit_base {
                                            sign * rng.range(1, 100)
                                        } else {
                                            0o100
                                        };
                                        let multiplier = if explicit_multiplier {
                                            rng.range(1, 20)
                                        } else {
                                            0o12
                                        };
                                        let base_text = base.to_string();
                                        let multiplier_text = multiplier.to_string();
                                        set_env(
                                            "PROG_BASE_OFFSET",
                                            explicit_base.then_some(base_text.as_str()),
                                        );
                                        set_env(
                                            "PROG_MULTIPLIER",
                                            explicit_multiplier.then_some(multiplier_text.as_str()),
                                        );

                                        let mut before_fallback = if optimize {
                                            first + second
                                        } else {
                                            first * 3 + second / 2
                                        };
                                        before_fallback += third * multiplier;
                                        before_fallback += fourth >> 2;
                                        if verbose {
                                            before_fallback <<= 1;
                                        }
                                        before_fallback |= 0x0f;
                                        before_fallback += base;
                                        assert_eq!(
                                            before_fallback < 0,
                                            negative,
                                            "generator failed for CONFIGS row {row}"
                                        );

                                        let result = compare_calls(
                                            &format!("CONFIGS row {row}, case {iteration}"),
                                            || unsafe { c.envy(first, second, third, fourth) },
                                            || unsafe { rust.envy(first, second, third, fourth) },
                                        );
                                        assert_eq!(
                                            result,
                                            if negative { first } else { before_fallback },
                                            "C result did not exercise CONFIGS row {row}"
                                        );
                                    }
                                    row += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    assert_eq!(row, 275, "not every CONFIGS.md row was executed");
}

#[test]
fn phase_c_error_paths() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _environment = EnvironmentSnapshot::clear();
    let (c_path, rust_path) = library_paths();
    let c = unsafe { Api::load(&c_path) };
    let rust = unsafe { Api::load(&rust_path) };
    let mut rng = Rng::new(0x53ec_d118_54af_09b3);
    let name = CString::new("DIFF_INVALID_VALUE").unwrap();

    for iteration in 0..RANDOM_CASES {
        unsafe { env::remove_var("DIFF_INVALID_VALUE") };
        let default = rng.next_u32() as i32;
        let result = compare_calls(
            &format!("ERRORS row 1, case {iteration}"),
            || unsafe { c.parse_env_numeric(name.as_ptr(), default) },
            || unsafe { rust.parse_env_numeric(name.as_ptr(), default) },
        );
        assert_eq!(result, default);
    }

    for iteration in 0..RANDOM_CASES {
        let default = rng.next_u32() as i32;
        let value = if iteration % 2 == 0 {
            format!("{},{}", rng.range(-1000, 1000), rng.range(-1000, 1000))
        } else {
            "12,34;56".to_owned()
        };
        unsafe { env::set_var("DIFF_INVALID_VALUE", value) };
        let result = compare_calls(
            &format!("ERRORS row 2, case {iteration}"),
            || unsafe { c.parse_env_numeric(name.as_ptr(), default) },
            || unsafe { rust.parse_env_numeric(name.as_ptr(), default) },
        );
        assert_eq!(result, default);
    }

    for iteration in 0..RANDOM_CASES {
        let default = rng.next_u32() as i32;
        unsafe {
            env::set_var(
                "DIFF_INVALID_VALUE",
                format!("{};{}", rng.range(-1000, 1000), iteration),
            )
        };
        let result = compare_calls(
            &format!("ERRORS row 3, case {iteration}"),
            || unsafe { c.parse_env_numeric(name.as_ptr(), default) },
            || unsafe { rust.parse_env_numeric(name.as_ptr(), default) },
        );
        assert_eq!(result, default);
    }
    unsafe { env::remove_var("DIFF_INVALID_VALUE") };

    set_env("PROG_VERBOSE", None);
    set_env("PROG_DEBUG", None);
    set_env("PROG_OPTIMIZE", None);
    set_env("PROG_BASE_OFFSET", None);
    set_env("PROG_MULTIPLIER", None);
    for iteration in 0..RANDOM_CASES {
        let first = -rng.range(1_000, 10_000);
        let second = -rng.range(1_000, 10_000);
        let third = -rng.range(1, 1_000);
        let fourth = -rng.range(1, 1_000);
        let result = compare_calls(
            &format!("ERRORS row 4, case {iteration}"),
            || unsafe { c.envy(first, second, third, fourth) },
            || unsafe { rust.envy(first, second, third, fourth) },
        );
        assert_eq!(result, first);
    }

    let test_executable = env::current_exe().unwrap();
    for (row, scenario) in [
        (5, "parse-null"),
        (6, "init-null"),
        (7, "perform-null"),
        (8, "apply-null"),
    ] {
        let run = |library: &Path| {
            Command::new(&test_executable)
                .args(["--exact", "ffi_crash_worker", "--nocapture"])
                .env("DIFF_CRASH_LIBRARY", library)
                .env("DIFF_CRASH_SCENARIO", scenario)
                .env("RUST_BACKTRACE", "0")
                .output()
                .unwrap()
        };
        let c_status = run(&c_path).status;
        let rust_status = run(&rust_path).status;
        assert!(
            !c_status.success(),
            "ERRORS row {row}: C unexpectedly returned"
        );
        assert!(
            !rust_status.success(),
            "ERRORS row {row}: Rust unexpectedly returned"
        );
        assert_eq!(
            c_status.signal(),
            rust_status.signal(),
            "ERRORS row {row}: termination signals differ"
        );
    }
}

#[test]
fn ffi_crash_worker() {
    let Some(path) = env::var_os("DIFF_CRASH_LIBRARY") else {
        return;
    };
    let scenario = env::var("DIFF_CRASH_SCENARIO").unwrap();
    let api = unsafe { Api::load(Path::new(&path)) };
    match scenario.as_str() {
        "parse-null" => {
            unsafe { api.parse_env_numeric(ptr::null(), 0) };
        }
        "init-null" => unsafe { api.init_config_from_env(ptr::null_mut()) },
        "perform-null" => {
            unsafe { api.perform_operation(1, 2, ptr::null_mut()) };
        }
        "apply-null" => {
            unsafe { api.apply_bit_operations(1, ptr::null_mut()) };
        }
        other => panic!("unknown crash scenario: {other}"),
    }
}
