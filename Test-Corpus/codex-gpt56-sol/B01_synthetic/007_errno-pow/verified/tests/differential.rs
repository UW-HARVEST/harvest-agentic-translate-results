use libloading::Library;
use std::env;
use std::ffi::{c_char, c_int, c_void, CString};
use std::fs::{self, File};
use std::os::fd::AsRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};

type DriverMain = unsafe extern "C" fn(c_int, *mut *mut c_char) -> c_int;

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

static IO_LOCK: Mutex<()> = Mutex::new(());
static CAPTURE_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Eq, PartialEq)]
struct Outcome {
    status: c_int,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

struct Harness {
    c: Library,
    rust: Library,
}

impl Harness {
    fn new() -> Self {
        let c_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libdriver_c.so");
        let rust_path = rust_library_path();
        assert!(
            c_path.is_file(),
            "missing C shared object: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "missing Rust shared object: {}",
            rust_path.display()
        );

        // SAFETY: both paths are build artifacts held open for the Harness lifetime.
        unsafe {
            Self {
                c: Library::new(c_path).expect("load C shared object"),
                rust: Library::new(rust_path).expect("load Rust shared object"),
            }
        }
    }

    fn compare(&self, args: &[String]) -> Outcome {
        self.compare_with_argc(args, args.len() as c_int)
    }

    fn compare_with_argc(&self, args: &[String], argc: c_int) -> Outcome {
        let c = invoke(&self.c, args, argc);
        let rust = invoke(&self.rust, args, argc);
        assert_eq!(
            c, rust,
            "ABI result mismatch for argc={argc}, argv={args:?}"
        );
        c
    }

    fn compare_nullable(&self, args: &[Option<&str>], argc: c_int) -> Outcome {
        let c = invoke_nullable(&self.c, args, argc);
        let rust = invoke_nullable(&self.rust, args, argc);
        assert_eq!(
            c, rust,
            "ABI result mismatch for nullable argc={argc}, argv={args:?}"
        );
        c
    }
}

fn rust_library_path() -> PathBuf {
    let executable = env::current_exe().expect("current test executable");
    executable
        .parent()
        .expect("target dependency directory")
        .join("libdriver_ffi.so")
}

fn lock_io() -> MutexGuard<'static, ()> {
    IO_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn invoke(library: &Library, args: &[String], argc: c_int) -> Outcome {
    let nullable: Vec<Option<&str>> = args.iter().map(|arg| Some(arg.as_str())).collect();
    invoke_nullable(library, &nullable, argc)
}

fn invoke_nullable(library: &Library, args: &[Option<&str>], argc: c_int) -> Outcome {
    let strings: Vec<Option<CString>> = args
        .iter()
        .map(|arg| {
            arg.map(|value| CString::new(value.as_bytes()).expect("argument contains no NUL"))
        })
        .collect();
    let mut pointers: Vec<*mut c_char> = strings
        .iter()
        .map(|arg| {
            arg.as_ref()
                .map_or(ptr::null_mut(), |value| value.as_ptr().cast_mut())
        })
        .collect();

    // SAFETY: the symbol signature and argv storage match C main for this call.
    unsafe {
        let function = library
            .get::<DriverMain>(b"main\0")
            .expect("load main symbol");
        capture(|| function(argc, pointers.as_mut_ptr()))
    }
}

unsafe fn capture(call: impl FnOnce() -> c_int) -> Outcome {
    let _guard = lock_io();
    let id = CAPTURE_ID.fetch_add(1, Ordering::Relaxed);
    let prefix = env::temp_dir().join(format!("driver-differential-{}-{id}", std::process::id()));
    let stdout_path = prefix.with_extension("stdout");
    let stderr_path = prefix.with_extension("stderr");
    let stdout_file = File::create(&stdout_path).expect("create stdout capture");
    let stderr_file = File::create(&stderr_path).expect("create stderr capture");

    // SAFETY: null fflush flushes every open C stream before descriptor changes.
    assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0);
    let saved_stdout = unsafe { dup(1) };
    let saved_stderr = unsafe { dup(2) };
    assert!(saved_stdout >= 0 && saved_stderr >= 0);
    assert_eq!(unsafe { dup2(stdout_file.as_raw_fd(), 1) }, 1);
    assert_eq!(unsafe { dup2(stderr_file.as_raw_fd(), 2) }, 2);

    let status = call();

    assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0);
    assert_eq!(unsafe { dup2(saved_stdout, 1) }, 1);
    assert_eq!(unsafe { dup2(saved_stderr, 2) }, 2);
    assert_eq!(unsafe { close(saved_stdout) }, 0);
    assert_eq!(unsafe { close(saved_stderr) }, 0);
    drop(stdout_file);
    drop(stderr_file);

    let stdout = fs::read(&stdout_path).expect("read stdout capture");
    let stderr = fs::read(&stderr_path).expect("read stderr capture");
    fs::remove_file(stdout_path).expect("remove stdout capture");
    fs::remove_file(stderr_path).expect("remove stderr capture");

    Outcome {
        status,
        stdout,
        stderr,
    }
}

fn arguments(base: impl Into<String>, exponent: impl Into<String>) -> Vec<String> {
    vec!["driver".into(), base.into(), exponent.into()]
}

fn assert_valid(harness: &Harness, args: Vec<String>) {
    let outcome = harness.compare(&args);
    assert_eq!(outcome.status, 0, "expected success for {args:?}");
    assert!(outcome.stderr.is_empty(), "unexpected stderr for {args:?}");
    assert!(outcome.stdout.starts_with(b"Result: "));
    assert!(outcome.stdout.ends_with(b"\n"));
}

fn assert_error(harness: &Harness, args: Vec<String>) {
    let outcome = harness.compare(&args);
    assert_eq!(outcome.status, 1, "expected rejection for {args:?}");
    assert!(outcome.stdout.is_empty(), "unexpected stdout for {args:?}");
    assert!(!outcome.stderr.is_empty(), "missing stderr for {args:?}");
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn range(&mut self, upper: u64) -> u64 {
        self.next() % upper
    }
}

#[test]
fn differential_completion_gate() {
    if env::var_os("DRIVER_PROBE_LIBRARY").is_some() {
        crash_probe_case();
        return;
    }

    configs_row_01_finite_decimal_integral_exponent();
    configs_row_02_finite_decimal_fractional_exponent();
    configs_row_03_leading_whitespace();
    configs_row_04_explicit_signs();
    configs_row_05_empty_string_is_accepted();
    configs_row_06_hexadecimal_floats();
    configs_row_07_infinity_inputs();
    configs_row_08_nan_inputs();
    configs_row_09_signed_zero();
    configs_row_10_printf_rounding();
    configs_row_11_successful_infinite_results();
    errors_row_01_wrong_argument_count();
    errors_row_02_base_conversion_range();
    errors_row_03_base_trailing_input();
    errors_row_04_exponent_conversion_range();
    errors_row_05_exponent_trailing_input();
    errors_row_06_pow_domain();
    errors_row_07_pow_range();
    generic_null_pointer_boundaries_match_termination_signal();
}

fn configs_row_01_finite_decimal_integral_exponent() {
    let harness = Harness::new();
    let mut rng = Rng::new(0x101);
    for _ in 0..64 {
        let base = 1.0 + rng.range(20_000) as f64 / 1000.0;
        let exponent = rng.range(9) as i32 - 4;
        assert_valid(
            &harness,
            arguments(format!("{base:.3}"), exponent.to_string()),
        );
    }
}

fn configs_row_02_finite_decimal_fractional_exponent() {
    let harness = Harness::new();
    let mut rng = Rng::new(0x202);
    for _ in 0..64 {
        let base = 0.25 + rng.range(40_000) as f64 / 2000.0;
        let numerator = (rng.range(15) * 2 + 1) as f64;
        let exponent = numerator / 4.0 - 2.0;
        assert_valid(
            &harness,
            arguments(format!("{base:.4}"), format!("{exponent:.2}")),
        );
    }
}

fn configs_row_03_leading_whitespace() {
    let harness = Harness::new();
    let mut rng = Rng::new(0x303);
    for index in 0..48 {
        let base = 1 + rng.range(40);
        let exponent = 1 + rng.range(4);
        let base = if index % 2 == 0 {
            format!(" \t {base}")
        } else {
            base.to_string()
        };
        let exponent = if index % 3 == 0 {
            format!("\n {exponent}")
        } else {
            exponent.to_string()
        };
        assert_valid(&harness, arguments(base, exponent));
    }
}

fn configs_row_04_explicit_signs() {
    let harness = Harness::new();
    let mut rng = Rng::new(0x404);
    for index in 0..48 {
        let magnitude = 1 + rng.range(30);
        let exponent = 1 + rng.range(5);
        let base = if index % 2 == 0 {
            format!("+{magnitude}")
        } else {
            format!("-{magnitude}")
        };
        assert_valid(&harness, arguments(base, format!("+{exponent}")));
    }
}

fn configs_row_05_empty_string_is_accepted() {
    let harness = Harness::new();
    let mut rng = Rng::new(0x505);
    for index in 0..48 {
        let value = (1 + rng.range(50)).to_string();
        let args = match index % 3 {
            0 => arguments("", value),
            1 => arguments(value, ""),
            _ => arguments("", ""),
        };
        assert_valid(&harness, args);
    }
}

fn configs_row_06_hexadecimal_floats() {
    let harness = Harness::new();
    let mut rng = Rng::new(0x606);
    for index in 0..48 {
        let digit = rng.range(16);
        let power = rng.range(7) as i32 - 3;
        let base = format!("0x1.{digit:x}p{power:+}");
        let exponent = if index % 2 == 0 { "0x1p+1" } else { "2" };
        assert_valid(&harness, arguments(base, exponent));
    }
}

fn configs_row_07_infinity_inputs() {
    let harness = Harness::new();
    let spellings = ["inf", "+inf", "INFINITY", "+InFiNiTy"];
    let mut rng = Rng::new(0x707);
    for index in 0..48 {
        let spelling = spellings[rng.range(spellings.len() as u64) as usize];
        let args = if index % 2 == 0 {
            arguments(spelling, (1 + rng.range(7)).to_string())
        } else {
            arguments((2 + rng.range(20)).to_string(), spelling)
        };
        assert_valid(&harness, args);
    }
}

fn configs_row_08_nan_inputs() {
    let harness = Harness::new();
    let mut rng = Rng::new(0x808);
    for index in 0..48 {
        let nan = match index % 3 {
            0 => "nan".to_string(),
            1 => "NAN".to_string(),
            _ => format!("nan(payload_{:x})", rng.next()),
        };
        let args = if index % 2 == 0 {
            arguments(nan, "2")
        } else {
            arguments("2", nan)
        };
        assert_valid(&harness, args);
    }
}

fn configs_row_09_signed_zero() {
    let harness = Harness::new();
    let mut rng = Rng::new(0x909);
    for index in 0..48 {
        let base = if index % 2 == 0 { "-0" } else { "+0" };
        let exponent = (rng.range(8) * 2 + 1).to_string();
        assert_valid(&harness, arguments(base, exponent));
    }
}

fn configs_row_10_printf_rounding() {
    let harness = Harness::new();
    let mut rng = Rng::new(0xa10);
    for _ in 0..64 {
        let integer = rng.range(1000);
        let thousandths = rng.range(1000);
        let base = format!("{integer}.{thousandths:03}");
        assert_valid(&harness, arguments(base, "1"));
    }
}

fn configs_row_11_successful_infinite_results() {
    let harness = Harness::new();
    let mut rng = Rng::new(0xb11);
    for index in 0..48 {
        let base = if index % 2 == 0 { "-inf" } else { "inf" };
        let exponent = if index % 2 == 0 {
            (rng.range(8) * 2 + 1).to_string()
        } else {
            (1 + rng.range(16)).to_string()
        };
        assert_valid(&harness, arguments(base, exponent));
    }
}

fn errors_row_01_wrong_argument_count() {
    let harness = Harness::new();
    let cases = [
        (vec!["driver".into()], 0),
        (vec!["driver".into()], -1),
        (vec!["driver".into()], 1),
        (vec!["driver".into(), "2".into()], 2),
        (vec!["driver".into(), "2".into(), "3".into(), "4".into()], 4),
        (vec!["driver".into()], c_int::MAX),
    ];
    for (args, argc) in cases {
        let outcome = harness.compare_with_argc(&args, argc);
        assert_eq!(outcome.status, 1);
        assert!(outcome.stdout.is_empty());
        assert_eq!(outcome.stderr, b"Usage: driver base exponent\n");
    }
}

fn errors_row_02_base_conversion_range() {
    let harness = Harness::new();
    let mut rng = Rng::new(0xc02);
    for index in 0..48 {
        let input = match index % 3 {
            0 => format!("1e{}", 309 + rng.range(500)),
            1 => format!("-1e-{}", 400 + rng.range(500)),
            _ => format!("1e{}junk", 309 + rng.range(500)),
        };
        assert_error(&harness, arguments(input, "2"));
    }
}

fn errors_row_03_base_trailing_input() {
    let harness = Harness::new();
    let mut rng = Rng::new(0xd03);
    for index in 0..48 {
        let value = rng.range(10_000);
        let input = match index % 3 {
            0 => format!("{value}junk"),
            1 => format!("{value} "),
            _ => format!("not-a-number-{value}"),
        };
        assert_error(&harness, arguments(input, "2"));
    }
}

fn errors_row_04_exponent_conversion_range() {
    let harness = Harness::new();
    let mut rng = Rng::new(0xe04);
    for index in 0..48 {
        let input = match index % 3 {
            0 => format!("1e{}", 309 + rng.range(500)),
            1 => format!("-1e-{}", 400 + rng.range(500)),
            _ => format!("1e{}junk", 309 + rng.range(500)),
        };
        assert_error(&harness, arguments("2", input));
    }
}

fn errors_row_05_exponent_trailing_input() {
    let harness = Harness::new();
    let mut rng = Rng::new(0xf05);
    for index in 0..48 {
        let value = rng.range(10_000);
        let input = match index % 3 {
            0 => format!("{value}junk"),
            1 => format!("{value} "),
            _ => format!("not-a-number-{value}"),
        };
        assert_error(&harness, arguments("2", input));
    }
}

fn errors_row_06_pow_domain() {
    let harness = Harness::new();
    let mut rng = Rng::new(0x106);
    for _ in 0..48 {
        let base = format!("-{}", 1 + rng.range(1000));
        let exponent = format!("{}.5", rng.range(10));
        assert_error(&harness, arguments(base, exponent));
    }
}

fn errors_row_07_pow_range() {
    let harness = Harness::new();
    let mut rng = Rng::new(0x207);
    for index in 0..48 {
        let args = if index % 2 == 0 {
            arguments(format!("{}e200", 1 + rng.range(9)), "2")
        } else {
            arguments(format!("{}e-200", 1 + rng.range(9)), "2")
        };
        assert_error(&harness, args);
    }
}

fn generic_null_pointer_boundaries_match_termination_signal() {
    let harness = Harness::new();
    let null_program = harness.compare_nullable(&[None, Some("2"), Some("3")], 2);
    assert_eq!(null_program.status, 1);
    assert!(null_program.stdout.is_empty());
    assert_eq!(
        null_program.stderr, b"Usage: (null) base exponent\n",
        "glibc formats a null %s argument as (null)"
    );

    let _guard = lock_io();
    for case in [
        "null_argv_wrong_argc",
        "null_argv_three",
        "null_base",
        "null_exponent",
    ] {
        let c = run_crash_probe(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libdriver_c.so"),
            case,
        );
        let rust = run_crash_probe(&rust_library_path(), case);
        assert!(!c.success(), "C unexpectedly survived {case}");
        assert!(!rust.success(), "Rust unexpectedly survived {case}");
        assert_eq!(
            c.signal(),
            rust.signal(),
            "different terminating signals for {case}: C={c:?}, Rust={rust:?}"
        );
    }
}

fn run_crash_probe(library: &Path, case: &str) -> ExitStatus {
    Command::new(env::current_exe().expect("current test executable"))
        .args(["--exact", "differential_completion_gate", "--nocapture"])
        .env("DRIVER_PROBE_LIBRARY", library)
        .env("DRIVER_PROBE_CASE", case)
        .status()
        .expect("run crash probe")
}

fn crash_probe_case() {
    let Ok(library_path) = env::var("DRIVER_PROBE_LIBRARY") else {
        return;
    };
    let case = env::var("DRIVER_PROBE_CASE").expect("probe case");
    // SAFETY: this process exists solely to observe behavior of intentionally
    // invalid C ABI pointers and is expected to terminate by signal.
    unsafe {
        let library = Library::new(library_path).expect("load probe library");
        let function = library
            .get::<DriverMain>(b"main\0")
            .expect("load probe main");
        match case.as_str() {
            "null_argv_wrong_argc" => {
                function(0, ptr::null_mut());
            }
            "null_argv_three" => {
                function(3, ptr::null_mut());
            }
            "null_base" => {
                let program = CString::new("driver").unwrap();
                let exponent = CString::new("2").unwrap();
                let mut argv = [
                    program.as_ptr().cast_mut(),
                    ptr::null_mut(),
                    exponent.as_ptr().cast_mut(),
                ];
                function(3, argv.as_mut_ptr());
            }
            "null_exponent" => {
                let program = CString::new("driver").unwrap();
                let base = CString::new("2").unwrap();
                let mut argv = [
                    program.as_ptr().cast_mut(),
                    base.as_ptr().cast_mut(),
                    ptr::null_mut(),
                ];
                function(3, argv.as_mut_ptr());
            }
            _ => panic!("unknown probe case: {case}"),
        }
    }
}
