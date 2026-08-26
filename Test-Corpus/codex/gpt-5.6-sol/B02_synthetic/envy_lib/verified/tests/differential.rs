use libloading::Library;
use std::env;
use std::ffi::{CString, c_char, c_int, c_void};
use std::fs::{File, OpenOptions, remove_file};
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

type ParseEnvNumeric = unsafe extern "C" fn(*const c_char, c_int) -> c_int;
type InitConfig = unsafe extern "C" fn(*mut u32);
type PerformOperation = unsafe extern "C" fn(c_int, c_int, *mut u32) -> c_int;
type ApplyBitOperations = unsafe extern "C" fn(c_int, *mut u32) -> c_int;
type Envy = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

unsafe extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

const ENV_KEYS: &[&str] = &[
    "PROG_VERBOSE",
    "PROG_DEBUG",
    "PROG_OPTIMIZE",
    "PROG_BASE_OFFSET",
    "PROG_MULTIPLIER",
    "DIFF_PARSE_VALUE",
];

struct Libraries {
    c: Library,
    rust: Library,
}

impl Libraries {
    fn load() -> Self {
        let c_path = c_library_path();
        let rust_path = rust_library_path();
        assert!(c_path.is_file(), "missing C library: {}", c_path.display());
        assert!(
            rust_path.is_file(),
            "missing Rust library: {}",
            rust_path.display()
        );

        unsafe {
            Self {
                c: Library::new(c_path).expect("load C shared library"),
                rust: Library::new(rust_path).expect("load Rust shared library"),
            }
        }
    }

    unsafe fn symbol<T: Copy>(&self, rust: bool, name: &[u8]) -> T {
        let library = if rust { &self.rust } else { &self.c };
        *unsafe { library.get::<T>(name) }.expect("load exported symbol")
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Captured<T> {
    result: T,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value as u32
    }

    fn bool(&mut self) -> bool {
        self.next_u32() & 1 != 0
    }

    fn i32_in(&mut self, minimum: i32, maximum: i32) -> i32 {
        let width = i64::from(maximum) - i64::from(minimum) + 1;
        (i64::from(minimum) + i64::from(self.next_u32()) % width) as i32
    }
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    crate_root().join("c_src/build/libtranslated_rust.so")
}

fn rust_library_path() -> PathBuf {
    crate_root().join("target/debug/libenvy_lib.so")
}

fn clear_environment() {
    for key in ENV_KEYS {
        unsafe { env::remove_var(key) };
    }
}

fn set_environment(key: &str, value: Option<&str>) {
    unsafe {
        match value {
            Some(value) => env::set_var(key, value),
            None => env::remove_var(key),
        }
    }
}

fn temp_capture_file(label: &str) -> (PathBuf, File) {
    static NEXT_ID: AtomicU64 = AtomicU64::new(0);
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let path = env::temp_dir().join(format!(
        "envy-differential-{}-{id}-{label}",
        std::process::id()
    ));
    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(&path)
        .expect("open capture file");
    (path, file)
}

fn capture<T>(operation: impl FnOnce() -> T) -> Captured<T> {
    let (stdout_path, mut stdout_file) = temp_capture_file("stdout");
    let (stderr_path, mut stderr_file) = temp_capture_file("stderr");

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);
    }
    let saved_stdout = unsafe { dup(1) };
    let saved_stderr = unsafe { dup(2) };
    assert!(saved_stdout >= 0 && saved_stderr >= 0);
    assert_eq!(unsafe { dup2(stdout_file.as_raw_fd(), 1) }, 1);
    assert_eq!(unsafe { dup2(stderr_file.as_raw_fd(), 2) }, 2);

    let result = operation();

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);
    }
    assert_eq!(unsafe { dup2(saved_stdout, 1) }, 1);
    assert_eq!(unsafe { dup2(saved_stderr, 2) }, 2);
    assert_eq!(unsafe { close(saved_stdout) }, 0);
    assert_eq!(unsafe { close(saved_stderr) }, 0);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    stdout_file.seek(SeekFrom::Start(0)).unwrap();
    stderr_file.seek(SeekFrom::Start(0)).unwrap();
    stdout_file.read_to_end(&mut stdout).unwrap();
    stderr_file.read_to_end(&mut stderr).unwrap();
    drop(stdout_file);
    drop(stderr_file);
    remove_file(stdout_path).unwrap();
    remove_file(stderr_path).unwrap();

    Captured {
        result,
        stdout,
        stderr,
    }
}

fn compare_parse(
    libraries: &Libraries,
    env_value: Option<&str>,
    default_value: i32,
    context: &str,
) -> Captured<i32> {
    let name = CString::new("DIFF_PARSE_VALUE").unwrap();
    set_environment("DIFF_PARSE_VALUE", env_value);
    let c_function = unsafe { libraries.symbol::<ParseEnvNumeric>(false, b"parse_env_numeric\0") };
    let c = capture(|| unsafe { c_function(name.as_ptr(), default_value) });

    set_environment("DIFF_PARSE_VALUE", env_value);
    let rust_function =
        unsafe { libraries.symbol::<ParseEnvNumeric>(true, b"parse_env_numeric\0") };
    let rust = capture(|| unsafe { rust_function(name.as_ptr(), default_value) });
    assert_eq!(c, rust, "{context}");
    c
}

fn configure_flags(verbose: bool, debug: bool, optimize: bool, alternate_false: bool) {
    let false_value = alternate_false.then_some("off");
    set_environment("PROG_VERBOSE", verbose.then_some("value-1").or(false_value));
    set_environment("PROG_DEBUG", debug.then_some("x1x").or(false_value));
    set_environment("PROG_OPTIMIZE", optimize.then_some(""));
}

fn compare_init(
    libraries: &Libraries,
    verbose: bool,
    debug: bool,
    optimize: bool,
    initial: u32,
    alternate_false: bool,
    context: &str,
) -> u32 {
    configure_flags(verbose, debug, optimize, alternate_false);
    let c_function = unsafe { libraries.symbol::<InitConfig>(false, b"init_config_from_env\0") };
    let mut c_flags = initial;
    let c = capture(|| unsafe { c_function(&mut c_flags) });

    configure_flags(verbose, debug, optimize, alternate_false);
    let rust_function = unsafe { libraries.symbol::<InitConfig>(true, b"init_config_from_env\0") };
    let mut rust_flags = initial;
    let rust = capture(|| unsafe { rust_function(&mut rust_flags) });

    assert_eq!(c, rust, "{context}: output");
    assert_eq!(
        c_flags.to_ne_bytes(),
        rust_flags.to_ne_bytes(),
        "{context}: state bytes"
    );
    c_flags
}

fn compare_perform(
    libraries: &Libraries,
    val1: i32,
    val2: i32,
    flags: u32,
    context: &str,
) -> Captured<i32> {
    let c_function = unsafe { libraries.symbol::<PerformOperation>(false, b"perform_operation\0") };
    let mut c_flags = flags;
    let c = capture(|| unsafe { c_function(val1, val2, &mut c_flags) });

    let rust_function =
        unsafe { libraries.symbol::<PerformOperation>(true, b"perform_operation\0") };
    let mut rust_flags = flags;
    let rust = capture(|| unsafe { rust_function(val1, val2, &mut rust_flags) });
    assert_eq!(c, rust, "{context}");
    assert_eq!(c_flags, rust_flags, "{context}: flags changed");
    c
}

fn compare_apply(libraries: &Libraries, value: i32, flags: u32, context: &str) -> Captured<i32> {
    let c_function =
        unsafe { libraries.symbol::<ApplyBitOperations>(false, b"apply_bit_operations\0") };
    let mut c_flags = flags;
    let c = capture(|| unsafe { c_function(value, &mut c_flags) });

    let rust_function =
        unsafe { libraries.symbol::<ApplyBitOperations>(true, b"apply_bit_operations\0") };
    let mut rust_flags = flags;
    let rust = capture(|| unsafe { rust_function(value, &mut rust_flags) });
    assert_eq!(c, rust, "{context}");
    assert_eq!(c_flags, rust_flags, "{context}: flags changed");
    c
}

#[derive(Clone)]
struct EnvyEnvironment {
    verbose: bool,
    debug: bool,
    optimize: bool,
    base_offset: String,
    multiplier: Option<String>,
}

impl EnvyEnvironment {
    fn apply(&self) {
        configure_flags(self.verbose, self.debug, self.optimize, false);
        set_environment("PROG_BASE_OFFSET", Some(&self.base_offset));
        set_environment("PROG_MULTIPLIER", self.multiplier.as_deref());
    }
}

fn compare_envy(
    libraries: &Libraries,
    environment: &EnvyEnvironment,
    parameters: [i32; 4],
    context: &str,
) -> Captured<i32> {
    environment.apply();
    let c_function = unsafe { libraries.symbol::<Envy>(false, b"envy\0") };
    let c = capture(|| unsafe {
        c_function(parameters[0], parameters[1], parameters[2], parameters[3])
    });

    environment.apply();
    let rust_function = unsafe { libraries.symbol::<Envy>(true, b"envy\0") };
    let rust = capture(|| unsafe {
        rust_function(parameters[0], parameters[1], parameters[2], parameters[3])
    });
    assert_eq!(c, rust, "{context}");
    c
}

fn test_parser_rows(libraries: &Libraries, rng: &mut Rng) {
    for iteration in 0..64 {
        let default_value = rng.i32_in(-1_000_000, 1_000_000);
        let result = compare_parse(
            libraries,
            None,
            default_value,
            &format!("P01 iteration {iteration}"),
        );
        assert_eq!(result.result, default_value);
    }

    let nonnumeric = ["", "x", "  ", "+", "-", "words only"];
    for iteration in 0..64 {
        let value = nonnumeric[iteration % nonnumeric.len()];
        let result = compare_parse(
            libraries,
            Some(value),
            rng.i32_in(-1000, 1000),
            &format!("P02 iteration {iteration}"),
        );
        assert_eq!(result.result, 0);
    }

    for iteration in 0..64 {
        let value = rng.i32_in(-1_000_000, 1_000_000);
        let text = if iteration % 2 == 0 {
            format!("{value}")
        } else {
            format!(" \t{value}")
        };
        let result = compare_parse(
            libraries,
            Some(&text),
            77,
            &format!("P03 iteration {iteration}"),
        );
        assert_eq!(result.result, value);
    }

    for iteration in 0..64 {
        let value = rng.i32_in(-1_000_000, 1_000_000);
        let text = format!("{value}suffix");
        let result = compare_parse(
            libraries,
            Some(&text),
            77,
            &format!("P04 iteration {iteration}"),
        );
        assert_eq!(result.result, value);
    }

    for iteration in 0..64 {
        let value = match iteration {
            0 => 0,
            1 => i32::MIN,
            2 => i32::MAX,
            _ => rng.i32_in(-1_000_000, 1_000_000),
        };
        let text = value.to_string();
        let result = compare_parse(
            libraries,
            Some(&text),
            77,
            &format!("P05 iteration {iteration}"),
        );
        assert_eq!(result.result, value);
    }
}

fn test_init_rows(libraries: &Libraries, rng: &mut Rng) {
    let mut row = 0;
    for verbose in [false, true] {
        for debug in [false, true] {
            for optimize in [false, true] {
                row += 1;
                for iteration in 0..64 {
                    let initial = rng.next_u32();
                    let actual = compare_init(
                        libraries,
                        verbose,
                        debug,
                        optimize,
                        initial,
                        iteration % 2 == 1,
                        &format!("I{row:02} iteration {iteration}"),
                    );
                    let expected_low = u32::from(verbose)
                        | (u32::from(debug) << 1)
                        | (u32::from(optimize) << 2)
                        | (1 << 3)
                        | (3 << 4);
                    assert_eq!(actual, (initial & !0xff) | expected_low);
                }
            }
        }
    }
    assert_eq!(row, 8);
}

fn test_operation_rows(libraries: &Libraries, rng: &mut Rng) {
    let mut row = 0;
    for optimize in [false, true] {
        for debug in [false, true] {
            row += 1;
            for iteration in 0..128 {
                let val1 = match iteration {
                    0 => 0,
                    1 => -1,
                    _ => rng.i32_in(-1_000_000, 1_000_000),
                };
                let mut val2 = rng.i32_in(-1_000_000, 1_000_000);
                if iteration % 2 == 0 {
                    val2 &= !1;
                } else {
                    val2 |= 1;
                }
                let log_level = iteration as u32 % 8;
                let flags = (rng.next_u32() & !0xff)
                    | (u32::from(debug) << 1)
                    | (u32::from(optimize) << 2)
                    | (log_level << 4);
                compare_perform(
                    libraries,
                    val1,
                    val2,
                    flags,
                    &format!("O{row:02} iteration {iteration}"),
                );
            }
        }
    }
    assert_eq!(row, 4);
}

fn test_apply_rows(libraries: &Libraries, rng: &mut Rng) {
    let mut row = 0;
    for verbose in [false, true] {
        for cache in [false, true] {
            row += 1;
            for iteration in 0..128 {
                let value = if verbose {
                    match iteration {
                        0 => 0,
                        1 => i32::MAX / 2,
                        _ => rng.i32_in(0, i32::MAX / 2),
                    }
                } else {
                    match iteration {
                        0 => i32::MIN,
                        1 => i32::MAX,
                        _ => rng.i32_in(-1_000_000, 1_000_000),
                    }
                };
                let flags = (rng.next_u32() & !0xff) | u32::from(verbose) | (u32::from(cache) << 3);
                compare_apply(
                    libraries,
                    value,
                    flags,
                    &format!("A{row:02} iteration {iteration}"),
                );
            }
        }
    }
    assert_eq!(row, 4);
}

fn test_envy_rows(libraries: &Libraries, rng: &mut Rng) {
    let mut row = 0;
    for verbose in [false, true] {
        for debug in [false, true] {
            for optimize in [false, true] {
                for param3_nonzero in [false, true] {
                    for param4_nonzero in [false, true] {
                        for negative_result in [false, true] {
                            row += 1;
                            for iteration in 0..32 {
                                let param1 = rng.i32_in(1, 1000);
                                let param2 = rng.i32_in(0, 1000);
                                let param3 = if param3_nonzero {
                                    rng.i32_in(1, 100)
                                } else {
                                    0
                                };
                                let param4 = if param4_nonzero {
                                    rng.i32_in(1, 400)
                                } else {
                                    0
                                };
                                let multiplier = if iteration % 2 == 0 {
                                    None
                                } else {
                                    Some(rng.i32_in(1, 20).to_string())
                                };
                                let base_offset = if negative_result {
                                    "-200000".to_owned()
                                } else if rng.bool() {
                                    "64".to_owned()
                                } else {
                                    rng.i32_in(0, 100).to_string()
                                };
                                let environment = EnvyEnvironment {
                                    verbose,
                                    debug,
                                    optimize,
                                    base_offset,
                                    multiplier,
                                };
                                let result = compare_envy(
                                    libraries,
                                    &environment,
                                    [param1, param2, param3, param4],
                                    &format!("N{row:03} iteration {iteration}"),
                                );
                                if negative_result {
                                    assert_eq!(
                                        result.result, param1,
                                        "N{row:03}: restore branch did not run"
                                    );
                                } else {
                                    assert!(
                                        result.result >= 0,
                                        "N{row:03}: unexpected negative result"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(row, 64);
}

fn test_error_rows(libraries: &Libraries, rng: &mut Rng) {
    for iteration in 0..64 {
        let default_value = rng.i32_in(-1_000_000, 1_000_000);
        let value = format!("{}{},{}", rng.i32_in(-100, 100), iteration, "tail");
        let result = compare_parse(
            libraries,
            Some(&value),
            default_value,
            &format!("E01 iteration {iteration}"),
        );
        assert_eq!(result.result, default_value);
        assert_eq!(
            result.stderr,
            b"Warning: Invalid character in DIFF_PARSE_VALUE\n"
        );
    }

    for iteration in 0..64 {
        let default_value = rng.i32_in(-1_000_000, 1_000_000);
        let value = format!("{};tail", rng.i32_in(-100, 100));
        let result = compare_parse(
            libraries,
            Some(&value),
            default_value,
            &format!("E02 iteration {iteration}"),
        );
        assert_eq!(result.result, default_value);
        assert_eq!(
            result.stderr,
            b"Warning: Semicolon found in DIFF_PARSE_VALUE\n"
        );
    }

    let environment = EnvyEnvironment {
        verbose: false,
        debug: false,
        optimize: true,
        base_offset: "-10000".to_owned(),
        multiplier: Some("10".to_owned()),
    };
    for iteration in 0..64 {
        let param1 = rng.i32_in(1, 100);
        let result = compare_envy(
            libraries,
            &environment,
            [param1, rng.i32_in(0, 100), 0, 0],
            &format!("E03 iteration {iteration}"),
        );
        assert_eq!(result.result, param1);
    }
}

fn dynamic_exports(path: &Path) -> Vec<String> {
    let output = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(output.status.success());
    let mut symbols: Vec<_> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let _address = fields.next()?;
            let kind = fields.next()?;
            let name = fields.next()?;
            (kind == "T").then(|| name.to_owned())
        })
        .collect();
    symbols.sort();
    symbols
}

fn test_symbol_parity() {
    let c_symbols = dynamic_exports(&c_library_path());
    let rust_symbols = dynamic_exports(&rust_library_path());
    assert_eq!(
        c_symbols,
        [
            "apply_bit_operations",
            "envy",
            "init_config_from_env",
            "parse_env_numeric",
            "perform_operation"
        ]
    );
    assert_eq!(c_symbols, rust_symbols);
}

fn child_status(library: &Path, case: &str) -> std::process::ExitStatus {
    Command::new(env::current_exe().unwrap())
        .args(["--exact", "null_pointer_child", "--nocapture"])
        .env("DIFF_NULL_LIBRARY", library)
        .env("DIFF_NULL_CASE", case)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("run null-pointer child")
}

fn test_null_pointer_rows() {
    for (row, case) in [
        ("E04", "parse"),
        ("E05", "init"),
        ("E06", "perform"),
        ("E07", "apply"),
    ] {
        let c = child_status(&c_library_path(), case);
        let rust = child_status(&rust_library_path(), case);
        assert!(!c.success(), "{row}: C unexpectedly accepted null");
        assert!(!rust.success(), "{row}: Rust unexpectedly accepted null");
        assert_eq!(
            c.signal(),
            rust.signal(),
            "{row}: termination signal differs (C={c:?}, Rust={rust:?})"
        );
        assert_eq!(c.signal(), Some(11), "{row}: C did not raise SIGSEGV");
    }
}

#[test]
fn differential_surface() {
    clear_environment();
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x6a09_e667_f3bc_c909);

    test_symbol_parity();
    test_parser_rows(&libraries, &mut rng);
    test_init_rows(&libraries, &mut rng);
    test_operation_rows(&libraries, &mut rng);
    test_apply_rows(&libraries, &mut rng);
    test_envy_rows(&libraries, &mut rng);
    test_error_rows(&libraries, &mut rng);
    test_null_pointer_rows();
    clear_environment();
}

#[test]
fn null_pointer_child() {
    let Ok(path) = env::var("DIFF_NULL_LIBRARY") else {
        return;
    };
    let case = env::var("DIFF_NULL_CASE").unwrap();
    let library = unsafe { Library::new(path) }.unwrap();

    unsafe {
        match case.as_str() {
            "parse" => {
                let function = *library
                    .get::<ParseEnvNumeric>(b"parse_env_numeric\0")
                    .unwrap();
                function(std::ptr::null(), 17);
            }
            "init" => {
                let function = *library
                    .get::<InitConfig>(b"init_config_from_env\0")
                    .unwrap();
                function(std::ptr::null_mut());
            }
            "perform" => {
                let function = *library
                    .get::<PerformOperation>(b"perform_operation\0")
                    .unwrap();
                function(1, 2, std::ptr::null_mut());
            }
            "apply" => {
                let function = *library
                    .get::<ApplyBitOperations>(b"apply_bit_operations\0")
                    .unwrap();
                function(1, std::ptr::null_mut());
            }
            _ => panic!("unknown null case"),
        }
    }
}
