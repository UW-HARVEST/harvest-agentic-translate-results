use libloading::{Library, Symbol};
use std::env;
use std::ffi::{c_char, c_int, c_void};
use std::fmt::Write as _;
use std::io::Write as _;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

const CHILD_ENV: &str = "DRIVER_DIFF_CHILD";
const LIBRARY_ENV: &str = "DRIVER_DIFF_LIBRARY";
const OPERATION_ENV: &str = "DRIVER_DIFF_OPERATION";
const ARGUMENT_ENV: &str = "DRIVER_DIFF_ARGUMENT";
const ITERATIONS: usize = 24;

unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
}

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    code: Option<i32>,
    signal: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl Outcome {
    fn from_status(status: ExitStatus, stdout: Vec<u8>, stderr: Vec<u8>) -> Self {
        Self {
            code: status.code(),
            signal: status.signal(),
            stdout,
            stderr,
        }
    }
}

#[derive(Clone, Copy)]
enum InputShape {
    Newline,
    Eof,
    Truncated,
}

#[derive(Clone, Copy)]
enum DataClass {
    Zero,
    Mid,
    Max,
    Rejected,
}

struct Lcg(u64);

impl Lcg {
    fn new() -> Self {
        Self(0xd1ff_e2e5_5eed_cafe)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 32) as u32
    }

    fn range(&mut self, low: i32, high_inclusive: i32) -> i32 {
        low + (self.next_u32() % (high_inclusive - low + 1) as u32) as i32
    }
}

fn main() {
    if env::var_os(CHILD_ENV).is_some() {
        child_main();
    }

    let c_library = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libdriver_c.so");
    let rust_library = rust_library_path();
    assert!(c_library.is_file(), "missing {}", c_library.display());
    assert!(rust_library.is_file(), "missing {}", rust_library.display());

    run_configuration_rows(&c_library, &rust_library);
    run_error_rows(&c_library, &rust_library);
    println!("differential matrix passed: 14 configuration rows, 4 error rows");
}

fn child_main() -> ! {
    let library_path = env::var_os(LIBRARY_ENV).expect("missing library path");
    let operation = env::var(OPERATION_ENV).expect("missing operation");
    let argument = env::var(ARGUMENT_ENV).unwrap_or_default();
    let library = unsafe { Library::new(library_path) }.expect("load shared library");

    let return_code = unsafe {
        match operation.as_str() {
            "print-null" => {
                let function: Symbol<unsafe extern "C" fn(*const c_char)> =
                    library.get(b"printLine\0").expect("load printLine");
                function(std::ptr::null());
                0
            }
            "print-bytes" => {
                let function: Symbol<unsafe extern "C" fn(*const c_char)> =
                    library.get(b"printLine\0").expect("load printLine");
                let mut bytes = decode_hex(&argument);
                bytes.push(0);
                function(bytes.as_ptr().cast());
                0
            }
            "main" => {
                let function: Symbol<unsafe extern "C" fn() -> c_int> =
                    library.get(b"main\0").expect("load main");
                function()
            }
            other => panic!("unknown operation {other}"),
        }
    };

    unsafe {
        fflush(std::ptr::null_mut());
    }
    drop(library);
    std::process::exit(return_code);
}

fn rust_library_path() -> PathBuf {
    let executable = env::current_exe().expect("current test executable");
    let deps = executable.parent().expect("test deps directory");
    let profile = deps.parent().expect("Cargo profile directory");
    for candidate in [profile.join("libdriver.so"), deps.join("libdriver.so")] {
        if candidate.is_file() {
            return candidate;
        }
    }
    panic!(
        "could not find Rust cdylib beside test executable {}",
        executable.display()
    );
}

fn invoke(library: &Path, operation: &str, argument: &[u8], stdin: &[u8]) -> Outcome {
    let mut child = Command::new(env::current_exe().expect("current test executable"))
        .env(CHILD_ENV, "1")
        .env(LIBRARY_ENV, library)
        .env(OPERATION_ENV, operation)
        .env(ARGUMENT_ENV, encode_hex(argument))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn differential worker");

    child
        .stdin
        .take()
        .expect("worker stdin")
        .write_all(stdin)
        .expect("write worker stdin");
    let output = child.wait_with_output().expect("wait for worker");
    Outcome::from_status(output.status, output.stdout, output.stderr)
}

fn compare(
    label: &str,
    c_library: &Path,
    rust_library: &Path,
    operation: &str,
    argument: &[u8],
    stdin: &[u8],
) -> Outcome {
    let c = invoke(c_library, operation, argument, stdin);
    let rust = invoke(rust_library, operation, argument, stdin);
    assert_eq!(
        (rust.code, rust.signal, &rust.stdout),
        (c.code, c.signal, &c.stdout),
        "{label} diverged\nC: {c:?}\nRust: {rust:?}\nstdin: {stdin:?}"
    );
    c
}

fn assert_success(label: &str, outcome: &Outcome, expected_stdout: &[u8]) {
    assert_eq!(
        (outcome.code, outcome.signal),
        (Some(0), None),
        "{label} had unexpected status: {outcome:?}"
    );
    assert_eq!(
        outcome.stdout, expected_stdout,
        "{label} had unexpected stdout"
    );
    assert!(
        outcome.stderr.is_empty(),
        "{label} had unexpected stderr: {:?}",
        outcome.stderr
    );
}

fn run_configuration_rows(c_library: &Path, rust_library: &Path) {
    for iteration in 0..ITERATIONS {
        let outcome = compare(
            &format!("CONFIGS row 1 iteration {iteration}"),
            c_library,
            rust_library,
            "print-bytes",
            b"",
            b"",
        );
        assert_success("CONFIGS row 1", &outcome, b"\n");
    }

    let mut rng = Lcg::new();
    for iteration in 0..ITERATIONS {
        let length = rng.range(1, 256) as usize;
        let bytes: Vec<u8> = (0..length)
            .map(|_| loop {
                let byte = rng.next_u32() as u8;
                if byte != 0 {
                    break byte;
                }
            })
            .collect();
        let outcome = compare(
            &format!("CONFIGS row 2 iteration {iteration}"),
            c_library,
            rust_library,
            "print-bytes",
            &bytes,
            b"",
        );
        let mut expected = bytes;
        expected.push(b'\n');
        assert_success("CONFIGS row 2", &outcome, &expected);
    }

    let shapes = [InputShape::Newline, InputShape::Eof, InputShape::Truncated];
    let classes = [
        DataClass::Zero,
        DataClass::Mid,
        DataClass::Max,
        DataClass::Rejected,
    ];

    for (shape_index, shape) in shapes.into_iter().enumerate() {
        for (class_index, class) in classes.into_iter().enumerate() {
            let row = 3 + shape_index * 4 + class_index;
            for iteration in 0..ITERATIONS {
                let value = value_for(class, iteration, &mut rng);
                let input = encode_input(value, class, shape, iteration, &mut rng);
                let outcome = compare(
                    &format!("CONFIGS row {row} iteration {iteration}"),
                    c_library,
                    rust_library,
                    "main",
                    b"",
                    &input,
                );
                let expected = expected_main_stdout(value);
                assert_success(&format!("CONFIGS row {row}"), &outcome, &expected);
            }
        }
    }
}

fn run_error_rows(c_library: &Path, rust_library: &Path) {
    for iteration in 0..ITERATIONS {
        let outcome = compare(
            &format!("ERRORS row 1 iteration {iteration}"),
            c_library,
            rust_library,
            "print-null",
            b"",
            b"",
        );
        assert_success("ERRORS row 1", &outcome, b"");
    }

    for iteration in 0..ITERATIONS {
        let outcome = compare(
            &format!("ERRORS row 2 iteration {iteration}"),
            c_library,
            rust_library,
            "main",
            b"",
            b"",
        );
        assert_eq!(
            (outcome.code, outcome.signal, outcome.stdout.as_slice()),
            (None, Some(11), b"".as_slice()),
            "ERRORS row 2 did not match measured C rejection"
        );
    }

    let mut rng = Lcg::new();
    for iteration in 0..ITERATIONS {
        let magnitude = rng.range(1, 10_000);
        let input = encode_negative(magnitude, iteration);
        let outcome = compare(
            &format!("ERRORS row 3 iteration {iteration}"),
            c_library,
            rust_library,
            "main",
            b"",
            &input,
        );
        assert_eq!(
            (outcome.code, outcome.signal, outcome.stdout.as_slice()),
            (None, Some(11), b"".as_slice()),
            "ERRORS row 3 did not match measured C rejection"
        );
    }

    for iteration in 0..ITERATIONS {
        let value = value_for(DataClass::Rejected, iteration, &mut rng);
        let shape = match iteration % 3 {
            0 => InputShape::Newline,
            1 => InputShape::Eof,
            _ => InputShape::Truncated,
        };
        let input = encode_input(value, DataClass::Rejected, shape, iteration, &mut rng);
        let outcome = compare(
            &format!("ERRORS row 4 iteration {iteration}"),
            c_library,
            rust_library,
            "main",
            b"",
            &input,
        );
        assert_success("ERRORS row 4", &outcome, b"\n");
    }
}

fn value_for(class: DataClass, iteration: usize, rng: &mut Lcg) -> i32 {
    match class {
        DataClass::Zero => 0,
        DataClass::Mid => rng.range(1, 98),
        DataClass::Max => 99,
        DataClass::Rejected => match iteration % 6 {
            0 => 100,
            1 => 101,
            2 => i32::MAX,
            _ => rng.range(102, 1_000_000),
        },
    }
}

fn encode_input(
    value: i32,
    class: DataClass,
    shape: InputShape,
    iteration: usize,
    rng: &mut Lcg,
) -> Vec<u8> {
    if matches!(shape, InputShape::Truncated) {
        let mut prefix = if iteration % 2 == 0 {
            format!("{value:013}").into_bytes()
        } else {
            let mut bytes = value.to_string().into_bytes();
            bytes.push(b'x');
            bytes.resize(13, b'Q');
            bytes
        };
        assert_eq!(prefix.len(), 13);
        let suffix_len = rng.range(1, 16) as usize;
        prefix.extend((0..suffix_len).map(|_| b'a' + (rng.next_u32() % 26) as u8));
        prefix.push(b'\n');
        return prefix;
    }

    let mut bytes = short_spelling(value, class, iteration);
    assert!(!bytes.is_empty());
    assert!(bytes.len() <= 12);
    if matches!(shape, InputShape::Newline) {
        bytes.push(b'\n');
    }
    bytes
}

fn short_spelling(value: i32, class: DataClass, iteration: usize) -> Vec<u8> {
    if matches!(class, DataClass::Zero) {
        return match iteration % 6 {
            0 => b"0".to_vec(),
            1 => b"not-a-number".to_vec(),
            2 => b" +0".to_vec(),
            3 => b"000000".to_vec(),
            4 => b"0trailing".to_vec(),
            _ => b"0\0ignored".to_vec(),
        };
    }

    match iteration % 6 {
        0 => value.to_string().into_bytes(),
        1 => format!("+{value}").into_bytes(),
        2 => format!(" {value}").into_bytes(),
        3 => format!("{value:010}").into_bytes(),
        4 => format!("{value}x").into_bytes(),
        _ => {
            let mut bytes = value.to_string().into_bytes();
            bytes.extend_from_slice(b"\0x");
            bytes
        }
    }
}

fn encode_negative(magnitude: i32, iteration: usize) -> Vec<u8> {
    match iteration % 3 {
        0 => format!("-{magnitude}\n").into_bytes(),
        1 => format!(" -{magnitude}").into_bytes(),
        _ => {
            let mut bytes = format!("-{magnitude:012}").into_bytes();
            assert_eq!(bytes.len(), 13);
            bytes.extend_from_slice(b"truncated\n");
            bytes
        }
    }
}

fn expected_main_stdout(value: i32) -> Vec<u8> {
    let mut expected = if value < 100 {
        vec![b'A'; value as usize]
    } else {
        Vec::new()
    };
    expected.push(b'\n');
    expected
}

fn encode_hex(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut encoded, "{byte:02x}").expect("write hex");
    }
    encoded
}

fn decode_hex(encoded: &str) -> Vec<u8> {
    assert_eq!(encoded.len() % 2, 0, "odd hex string");
    encoded
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair).expect("hex utf8");
            u8::from_str_radix(text, 16).expect("hex byte")
        })
        .collect()
}
