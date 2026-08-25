use libloading::Library;
use std::env;
use std::ffi::{c_void, CString};
use std::fs::File;
use std::io::Read;
use std::mem::offset_of;
use std::os::fd::FromRawFd;
use std::os::raw::{c_char, c_int};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[repr(C)]
struct CTest {
    a: c_int,
    b: c_int,
}

type FindContainer = unsafe extern "C" fn(*mut c_int) -> *mut CTest;
type DriverMain = unsafe extern "C" fn(c_int, *mut *mut c_char) -> c_int;

struct Api {
    _c_library: Library,
    _rust_library: Library,
    c_find_a: FindContainer,
    rust_find_a: FindContainer,
    c_find_b: FindContainer,
    rust_find_b: FindContainer,
    c_main: DriverMain,
    rust_main: DriverMain,
}

impl Api {
    unsafe fn load() -> Self {
        let c_library = Library::new(c_library_path()).expect("load C reference shared object");
        let rust_library = Library::new(rust_library_path()).expect("load Rust shared object");

        let c_find_a = *c_library
            .get::<FindContainer>(b"find_container_of_a\0")
            .expect("load C find_container_of_a");
        let rust_find_a = *rust_library
            .get::<FindContainer>(b"find_container_of_a\0")
            .expect("load Rust find_container_of_a");
        let c_find_b = *c_library
            .get::<FindContainer>(b"find_container_of_b\0")
            .expect("load C find_container_of_b");
        let rust_find_b = *rust_library
            .get::<FindContainer>(b"find_container_of_b\0")
            .expect("load Rust find_container_of_b");
        let c_main = *c_library.get::<DriverMain>(b"main\0").expect("load C main");
        let rust_main = *rust_library
            .get::<DriverMain>(b"main\0")
            .expect("load Rust main");

        Self {
            _c_library: c_library,
            _rust_library: rust_library,
            c_find_a,
            rust_find_a,
            c_find_b,
            rust_find_b,
            c_main,
            rust_main,
        }
    }
}

struct Rng(u64);

impl Rng {
    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 32) as u32
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libdriver.so")
}

fn rust_library_path() -> PathBuf {
    env::current_exe()
        .expect("resolve current test executable")
        .parent()
        .and_then(Path::parent)
        .expect("test executable should be under target/<profile>/deps")
        .join("libdriver.so")
}

extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
}

unsafe fn call_main(function: DriverMain, argc: c_int, arguments: &[String]) -> (c_int, Vec<u8>) {
    let strings: Vec<CString> = arguments
        .iter()
        .map(|argument| CString::new(argument.as_bytes()).expect("argument contains NUL"))
        .collect();
    let mut argv: Vec<*mut c_char> = strings
        .iter()
        .map(|argument| argument.as_ptr().cast_mut())
        .collect();
    argv.push(std::ptr::null_mut());

    assert_eq!(fflush(std::ptr::null_mut()), 0);
    let mut pipe_fds = [-1; 2];
    assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0);
    let saved_stdout = dup(1);
    assert!(saved_stdout >= 0);
    assert_eq!(dup2(pipe_fds[1], 1), 1);
    assert_eq!(close(pipe_fds[1]), 0);

    let result = function(argc, argv.as_mut_ptr());

    assert_eq!(fflush(std::ptr::null_mut()), 0);
    assert_eq!(dup2(saved_stdout, 1), 1);
    assert_eq!(close(saved_stdout), 0);

    let mut output = Vec::new();
    File::from_raw_fd(pipe_fds[0])
        .read_to_end(&mut output)
        .expect("read captured stdout");
    (result, output)
}

unsafe fn assert_main_equal(api: &Api, argc: c_int, arguments: &[String], context: &str) {
    let c_result = call_main(api.c_main, argc, arguments);
    let rust_result = call_main(api.rust_main, argc, arguments);
    assert_eq!(rust_result, c_result, "{context}: {arguments:?}");
}

#[test]
fn valid_configuration_matrix_matches() {
    let api = unsafe { Api::load() };
    let mut rng = Rng(0x5eed_c0de_d15c_a11e);

    // CONFIGS C1-C2: both container offsets over the full int value range.
    for case in 0..512 {
        let mut value = CTest {
            a: rng.next_i32(),
            b: rng.next_i32(),
        };
        let base = std::ptr::addr_of_mut!(value);

        let c_a = unsafe { (api.c_find_a)(std::ptr::addr_of_mut!(value.a)) };
        let rust_a = unsafe { (api.rust_find_a)(std::ptr::addr_of_mut!(value.a)) };
        assert_eq!(rust_a, c_a, "C1 case {case}");
        assert_eq!(c_a, base, "C1 enclosing pointer case {case}");

        let c_b = unsafe { (api.c_find_b)(std::ptr::addr_of_mut!(value.b)) };
        let rust_b = unsafe { (api.rust_find_b)(std::ptr::addr_of_mut!(value.b)) };
        assert_eq!(rust_b, c_b, "C2 case {case}");
        assert_eq!(c_b, base, "C2 enclosing pointer case {case}");
    }

    // ERRORS G1-G2: null pointers are transformed without dereferencing.
    let c_null_a = unsafe { (api.c_find_a)(std::ptr::null_mut()) };
    let rust_null_a = unsafe { (api.rust_find_a)(std::ptr::null_mut()) };
    assert_eq!(rust_null_a, c_null_a);
    assert!(c_null_a.is_null());

    let c_null_b = unsafe { (api.c_find_b)(std::ptr::null_mut()) };
    let rust_null_b = unsafe { (api.rust_find_b)(std::ptr::null_mut()) };
    assert_eq!(rust_null_b, c_null_b);
    assert_eq!(c_null_b as usize, 0usize.wrapping_sub(offset_of!(CTest, b)));

    // CONFIGS C3: randomized decimal operands with in-range sums.
    for case in 0..256 {
        let a = (rng.next_u32() % 2_000_001) as i32 - 1_000_000;
        let b = (rng.next_u32() % 2_000_001) as i32 - 1_000_000;
        let arguments = vec!["driver".into(), a.to_string(), b.to_string()];
        unsafe { assert_main_equal(&api, 3, &arguments, &format!("C3 case {case}")) };
    }

    // CONFIGS C4: all atoi lexical shapes used through the public entry point.
    for case in 0..256 {
        let a = (rng.next_u32() % 2_000_001) as i32 - 1_000_000;
        let b = (rng.next_u32() % 2_000_001) as i32 - 1_000_000;
        let first = match case % 6 {
            0 => format!("  {a}tail"),
            1 => format!("\t+{}xyz", a.unsigned_abs()),
            2 => format!("{a} trailing"),
            3 => String::new(),
            4 => format!("nondigit{a}"),
            _ => a.to_string(),
        };
        let second = match case % 5 {
            0 => format!("\n{b}suffix"),
            1 => format!("+{}", b.unsigned_abs()),
            2 => String::from("-0"),
            3 => String::from("words"),
            _ => b.to_string(),
        };
        let arguments = vec!["driver".into(), first, second];
        unsafe { assert_main_equal(&api, 3, &arguments, &format!("C4 case {case}")) };
    }
    for (a, b) in [
        (i32::MIN.to_string(), "0".to_string()),
        (i32::MAX.to_string(), "0".to_string()),
        ("0".to_string(), i32::MIN.to_string()),
        ("0".to_string(), i32::MAX.to_string()),
    ] {
        unsafe { assert_main_equal(&api, 3, &["driver".into(), a, b], "C4 int boundary") };
    }

    // CONFIGS C5: machine-int addition overflow in the compiled C reference.
    for case in 0..256 {
        let (a, b) = if case % 2 == 0 {
            (
                i32::MAX - (rng.next_u32() % 10_000) as i32,
                10_001 + (rng.next_u32() % 10_000) as i32,
            )
        } else {
            (
                i32::MIN + (rng.next_u32() % 10_000) as i32,
                -10_001 - (rng.next_u32() % 10_000) as i32,
            )
        };
        let arguments = vec!["driver".into(), a.to_string(), b.to_string()];
        unsafe { assert_main_equal(&api, 3, &arguments, &format!("C5 case {case}")) };
    }

    // CONFIGS C6: argc and trailing arguments are ignored.
    let argc_values = [i32::MIN, -1, 0, 2, 3, i32::MAX];
    for case in 0..256 {
        let a = (rng.next_u32() % 2_000_001) as i32 - 1_000_000;
        let b = (rng.next_u32() % 2_000_001) as i32 - 1_000_000;
        let mut arguments = vec!["driver".into(), a.to_string(), b.to_string()];
        arguments.extend((0..case % 5).map(|index| format!("ignored-{case}-{index}")));
        unsafe {
            assert_main_equal(
                &api,
                argc_values[case % argc_values.len()],
                &arguments,
                &format!("C6 case {case}"),
            )
        };
    }
}

fn crash_status(library: &Path, scenario: &str) -> std::process::ExitStatus {
    Command::new(env::current_exe().expect("resolve current test executable"))
        .args(["--ignored", "--exact", "ffi_crash_child", "--nocapture"])
        .env("DIFF_CHILD_LIBRARY", library)
        .env("DIFF_CHILD_SCENARIO", scenario)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("run crash-isolation child")
}

#[test]
fn generic_error_boundaries_match() {
    for scenario in ["argv_null", "argv1_null", "argv2_null"] {
        let c_status = crash_status(&c_library_path(), scenario);
        let rust_status = crash_status(&rust_library_path(), scenario);
        assert_eq!(
            rust_status.signal(),
            c_status.signal(),
            "{scenario}: C={c_status:?}, Rust={rust_status:?}"
        );
        assert_eq!(c_status.signal(), Some(11), "{scenario}: {c_status:?}");
    }
}

#[test]
#[ignore = "crash-isolation helper; invoked by generic_error_boundaries_match"]
fn ffi_crash_child() {
    let library_path = env::var_os("DIFF_CHILD_LIBRARY").expect("child library path");
    let scenario = env::var("DIFF_CHILD_SCENARIO").expect("child scenario");
    let library = unsafe { Library::new(library_path).expect("load child library") };
    let function = unsafe {
        *library
            .get::<DriverMain>(b"main\0")
            .expect("load child main")
    };

    let program = CString::new("driver").unwrap();
    let first = CString::new("1").unwrap();
    let second = CString::new("2").unwrap();
    let mut argv = match scenario.as_str() {
        "argv_null" => Vec::new(),
        "argv1_null" => vec![
            program.as_ptr().cast_mut(),
            std::ptr::null_mut(),
            second.as_ptr().cast_mut(),
            std::ptr::null_mut(),
        ],
        "argv2_null" => vec![
            program.as_ptr().cast_mut(),
            first.as_ptr().cast_mut(),
            std::ptr::null_mut(),
        ],
        _ => panic!("unknown child scenario: {scenario}"),
    };
    let argv_pointer = if scenario == "argv_null" {
        std::ptr::null_mut()
    } else {
        argv.as_mut_ptr()
    };

    unsafe {
        function(3, argv_pointer);
    }
    panic!("child call unexpectedly returned");
}
