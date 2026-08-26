use libloading::{Library, Symbol};
use std::env;
use std::ffi::{CString, c_char, c_int, c_void};
use std::fmt::Write as _;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{Command, ExitStatus};

type Driver = unsafe extern "C" fn(*const c_char);
type Run = unsafe extern "C" fn(c_int);

const ERROR: &[u8] = b"An error occurred\n";
const FRESH_RUN_ZERO: &[u8] = concat!(
    "The house has 2 floors, 5 bedrooms, and 2.5 bathrooms\n",
    "The house has 3 floors, 5 bedrooms, and 2.5 bathrooms\n",
    "The house has 3 floors, 5 bedrooms, and 3.5 bathrooms\n",
    "The house has 3 floors, 5 bedrooms, and 3.5 bathrooms\n",
)
.as_bytes();

#[derive(Debug)]
struct Invocation {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u64() as i32
    }

    fn below(&mut self, upper: u64) -> u64 {
        self.next_u64() % upper
    }
}

fn rust_library_path() -> PathBuf {
    env::current_exe()
        .expect("current test executable")
        .parent()
        .expect("test executable directory")
        .parent()
        .expect("Cargo profile directory")
        .join("libdriver.so")
}

fn c_library_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libdriver.so")
}

fn ensure_rust_library() {
    let output = Command::new(env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(["build", "--lib", "--no-default-features"])
        .output()
        .expect("build Rust shared library");
    assert!(
        output.status.success(),
        "Rust shared-library build failed:\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        rust_library_path().is_file(),
        "Rust shared library was not produced at {}",
        rust_library_path().display()
    );
}

fn worker_main(arguments: &[String]) {
    assert!(arguments.len() >= 4, "worker arguments: {arguments:?}");
    let library_path = match arguments[2].as_str() {
        "c" => c_library_path(),
        "rust" => rust_library_path(),
        other => panic!("unknown library kind: {other}"),
    };

    let library = unsafe { Library::new(&library_path) }
        .unwrap_or_else(|error| panic!("load {}: {error}", library_path.display()));

    match arguments[3].as_str() {
        "run" => {
            let value = arguments[4].parse::<i32>().expect("run integer");
            let run: Symbol<Run> = unsafe { library.get(b"run\0") }.expect("load run");
            unsafe { run(value) };
        }
        "driver" => {
            let input = CString::new(arguments[4].as_bytes()).expect("driver input");
            let driver: Symbol<Driver> = unsafe { library.get(b"driver\0") }.expect("load driver");
            unsafe { driver(input.as_ptr()) };
        }
        "driver_then_run" => {
            let input = CString::new(arguments[4].as_bytes()).expect("driver input");
            let driver: Symbol<Driver> = unsafe { library.get(b"driver\0") }.expect("load driver");
            let run: Symbol<Run> = unsafe { library.get(b"run\0") }.expect("load run");
            unsafe {
                driver(input.as_ptr());
                run(0);
            }
        }
        "driver_null" => {
            let driver: Symbol<Driver> = unsafe { library.get(b"driver\0") }.expect("load driver");
            unsafe { driver(std::ptr::null()) };
        }
        other => panic!("unknown operation: {other}"),
    }

    unsafe extern "C" {
        fn fflush(stream: *mut c_void) -> c_int;
    }
    unsafe {
        fflush(std::ptr::null_mut());
    }
}

fn invoke(kind: &str, operation: &str, argument: Option<&str>) -> Invocation {
    let mut command = Command::new(env::current_exe().expect("current test executable"));
    command.arg("worker").arg(kind).arg(operation);
    if let Some(argument) = argument {
        command.arg(argument);
    }
    let output = command.output().expect("run differential worker");
    Invocation {
        status: output.status,
        stdout: output.stdout,
        stderr: output.stderr,
    }
}

fn assert_success(invocation: &Invocation, context: &str) {
    assert!(
        invocation.status.success(),
        "{context} failed with {:?}; stderr={}",
        invocation.status,
        String::from_utf8_lossy(&invocation.stderr)
    );
}

fn compare(operation: &str, argument: &str, context: &str) -> Vec<u8> {
    let c = invoke("c", operation, Some(argument));
    let rust = invoke("rust", operation, Some(argument));
    assert_success(&c, &format!("{context}: C"));
    assert_success(&rust, &format!("{context}: Rust"));
    assert_eq!(
        rust.stdout, c.stdout,
        "{context}: stdout differs for input {argument:?}"
    );
    c.stdout
}

fn valid_run_row() {
    let mut rng = Rng::new(0x51a7_7e11_0000_0001);
    let mut values = vec![-100, -1, 0, 1, 100];
    values.extend((0..32).map(|_| rng.below(20_001) as i32 - 10_000));

    for value in values {
        let output = compare("run", &value.to_string(), "CONFIGS row 1");
        assert_eq!(
            output.iter().filter(|byte| **byte == b'\n').count(),
            4,
            "run must emit four lines"
        );
    }
}

fn valid_plain_decimal_row() {
    let mut rng = Rng::new(0x51a7_7e11_0000_0002);
    let mut values = vec![i32::MIN + 1, -1, 0, 1, i32::MAX - 5];
    values.extend((0..32).map(|_| rng.next_i32()));

    for value in values {
        let output = compare("driver", &value.to_string(), "CONFIGS row 2");
        assert_eq!(
            output.iter().filter(|byte| **byte == b'\n').count(),
            8,
            "successful driver must emit eight lines"
        );
    }
}

fn valid_whitespace_and_sign_row() {
    let mut rng = Rng::new(0x51a7_7e11_0000_0003);
    let whitespace = [" ", "\t", "\n", "\r", " \t"];

    for case in 0..40 {
        let magnitude = rng.below(1_000_001) as i32;
        let negative = rng.below(2) == 0;
        let sign = if negative {
            "-"
        } else if rng.below(2) == 0 {
            "+"
        } else {
            ""
        };
        let input = format!(
            "{}{}{}",
            whitespace[rng.below(whitespace.len() as u64) as usize],
            sign,
            magnitude
        );
        let output = compare("driver", &input, &format!("CONFIGS row 3 case {case}"));
        assert_eq!(output.iter().filter(|byte| **byte == b'\n').count(), 8);
    }
}

fn valid_numeric_prefix_row() {
    let mut rng = Rng::new(0x51a7_7e11_0000_0004);
    let suffixes = ["x", " trailing", "_suffix", ".5", "e9", "xyz123"];

    for case in 0..40 {
        let value = rng.next_i32();
        let input = format!(
            "{}{}",
            value,
            suffixes[rng.below(suffixes.len() as u64) as usize]
        );
        let output = compare("driver", &input, &format!("CONFIGS row 4 case {case}"));
        assert_eq!(output.iter().filter(|byte| **byte == b'\n').count(), 8);
    }
}

fn valid_int_boundaries_row() {
    let mut rng = Rng::new(0x51a7_7e11_0000_0005);
    let whitespace = ["", " ", "\t", "\n", " \t"];

    for case in 0..40 {
        let minimum = rng.below(2) == 0;
        let zeros = "0".repeat(rng.below(12) as usize);
        let input = if minimum {
            format!(
                "{}-{}2147483648",
                whitespace[rng.below(whitespace.len() as u64) as usize],
                zeros
            )
        } else {
            format!(
                "{}+{}2147483647",
                whitespace[rng.below(whitespace.len() as u64) as usize],
                zeros
            )
        };
        let output = compare("driver", &input, &format!("CONFIGS row 5 case {case}"));
        assert_eq!(output.iter().filter(|byte| **byte == b'\n').count(), 8);
    }
}

fn assert_rejection(input: &str, row: usize, case: usize) {
    let expected = [ERROR, FRESH_RUN_ZERO].concat();
    let output = compare(
        "driver_then_run",
        input,
        &format!("ERRORS row {row} case {case}"),
    );
    assert_eq!(
        output, expected,
        "ERRORS row {row}: rejection bytes or state mutation differ for {input:?}"
    );
}

fn error_no_conversion_row() {
    let fixed = ["", " ", "\t\n", "+", "-", "x", " xyz", "--1", "+-2"];
    for (case, input) in fixed.into_iter().enumerate() {
        assert_rejection(input, 1, case);
    }

    let mut rng = Rng::new(0xe220_0000_0000_0001);
    for case in 9..41 {
        let first = (b'a' + rng.below(26) as u8) as char;
        let input = format!("{first}{}", rng.next_u64());
        assert_rejection(&input, 1, case);
    }
}

fn oversized_decimal(rng: &mut Rng, negative: bool) -> String {
    let digits = 30 + rng.below(50) as usize;
    let mut value = String::with_capacity(digits + usize::from(negative));
    if negative {
        value.push('-');
    }
    value.push((b'1' + rng.below(9) as u8) as char);
    for _ in 1..digits {
        value.push((b'0' + rng.below(10) as u8) as char);
    }
    value
}

fn error_long_overflow_row() {
    let mut rng = Rng::new(0xe220_0000_0000_0002);
    for case in 0..40 {
        let input = oversized_decimal(&mut rng, false);
        assert_rejection(&input, 2, case);
    }
}

fn error_long_underflow_row() {
    let mut rng = Rng::new(0xe220_0000_0000_0003);
    for case in 0..40 {
        let input = oversized_decimal(&mut rng, true);
        assert_rejection(&input, 3, case);
    }
}

fn error_below_int_min_row() {
    let mut rng = Rng::new(0xe220_0000_0000_0004);
    for case in 0..40 {
        let value = i64::from(i32::MIN) - 1 - rng.below(1_000_000) as i64;
        assert_rejection(&value.to_string(), 4, case);
    }
}

fn error_above_int_max_row() {
    let mut rng = Rng::new(0xe220_0000_0000_0005);
    for case in 0..40 {
        let value = i64::from(i32::MAX) + 1 + rng.below(1_000_000) as i64;
        assert_rejection(&value.to_string(), 5, case);
    }
}

fn generic_null_boundary() {
    let c = invoke("c", "driver_null", None);
    let rust = invoke("rust", "driver_null", None);
    assert!(!c.status.success(), "C driver(NULL) unexpectedly succeeded");
    assert!(
        !rust.status.success(),
        "Rust driver(NULL) unexpectedly succeeded"
    );
    assert_eq!(
        rust.status.signal(),
        c.status.signal(),
        "driver(NULL) termination signal differs; C stderr={}, Rust stderr={}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&rust.stderr)
    );
}

fn main() {
    let arguments: Vec<String> = env::args().collect();
    if arguments.get(1).map(String::as_str) == Some("worker") {
        worker_main(&arguments);
        return;
    }

    ensure_rust_library();

    let mut failures = String::new();
    let checks: &[(&str, fn())] = &[
        ("CONFIGS row 1", valid_run_row),
        ("CONFIGS row 2", valid_plain_decimal_row),
        ("CONFIGS row 3", valid_whitespace_and_sign_row),
        ("CONFIGS row 4", valid_numeric_prefix_row),
        ("CONFIGS row 5", valid_int_boundaries_row),
        ("ERRORS row 1", error_no_conversion_row),
        ("ERRORS row 2", error_long_overflow_row),
        ("ERRORS row 3", error_long_underflow_row),
        ("ERRORS row 4", error_below_int_min_row),
        ("ERRORS row 5", error_above_int_max_row),
        ("generic NULL boundary", generic_null_boundary),
    ];

    for (name, check) in checks {
        if std::panic::catch_unwind(check).is_err() {
            let _ = writeln!(failures, "{name} failed");
        }
    }

    assert!(failures.is_empty(), "{failures}");
    println!("differential verification passed: 5 CONFIGS rows, 5 ERRORS rows, and NULL");
}
