use libloading::{Library, Symbol};
use std::env;
use std::ffi::{CString, c_char, c_int, c_void};
use std::fs::{self, File, OpenOptions};
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};

type Forward = unsafe extern "C" fn(c_int) -> c_int;
type OpenWithCleanup = unsafe extern "C" fn(*const c_char) -> *mut c_void;
type Driver = unsafe extern "C" fn(c_int, *const c_char) -> c_int;

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fclose(stream: *mut c_void) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn write(fd: c_int, buffer: *const c_void, count: usize) -> isize;
}

const ITERATIONS: usize = 12;
const STDOUT_FILENO: c_int = 1;
const STDERR_FILENO: c_int = 2;
static NEXT_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug)]
enum Call {
    Forward,
    Open,
    Driver,
}

impl Call {
    fn as_str(self) -> &'static str {
        match self {
            Self::Forward => "forward",
            Self::Open => "open",
            Self::Driver => "driver",
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum FileShape {
    Empty,
    OneChunk,
    ManyChunks,
}

#[derive(Debug)]
struct Observation {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

struct TestDir(PathBuf);

impl TestDir {
    fn new(label: &str) -> Self {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let path = env::temp_dir().join(format!(
            "driver-differential-{label}-{}-{id}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create differential-test directory");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[derive(Clone, Copy)]
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

    fn inclusive_i32(&mut self, low: i32, high: i32) -> i32 {
        let width = (i64::from(high) - i64::from(low) + 1) as u64;
        (i64::from(low) + (self.next_u64() % width) as i64) as i32
    }

    fn inclusive_usize(&mut self, low: usize, high: usize) -> usize {
        low + self.next_u64() as usize % (high - low + 1)
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library() -> PathBuf {
    manifest_dir().join("c_src/build/libdriver.so")
}

fn rust_library() -> PathBuf {
    manifest_dir().join("target/release/libdriver.so")
}

fn marker(call: Call, value: &str) -> Vec<u8> {
    format!("\n__DIFF_RESULT__:{}:{value}\n", call.as_str()).into_bytes()
}

fn write_all_fd(fd: c_int, mut bytes: &[u8]) {
    while !bytes.is_empty() {
        let written = unsafe { write(fd, bytes.as_ptr().cast(), bytes.len()) };
        assert!(
            written > 0,
            "write capture marker: {}",
            io::Error::last_os_error()
        );
        bytes = &bytes[written as usize..];
    }
}

fn filename_from_environment() -> (Option<CString>, *const c_char) {
    if env::var_os("DIFF_NULL_FILENAME").is_some() {
        return (None, ptr::null());
    }

    let path = env::var_os("DIFF_FILENAME").expect("DIFF_FILENAME");
    let filename = CString::new(path.as_os_str().as_bytes()).expect("filename without NUL");
    let pointer = filename.as_ptr();
    (Some(filename), pointer)
}

fn invoke_child() {
    let library_path = env::var_os("DIFF_LIBRARY").expect("DIFF_LIBRARY");
    let call = env::var("DIFF_CALL").expect("DIFF_CALL");
    let number = env::var("DIFF_NUMBER")
        .unwrap_or_else(|_| "0".to_owned())
        .parse::<c_int>()
        .expect("DIFF_NUMBER integer");
    let (_filename, filename) = filename_from_environment();
    let library = unsafe { Library::new(library_path) }.expect("load differential library");

    let result_marker = match call.as_str() {
        "forward" => {
            let function: Symbol<'_, Forward> =
                unsafe { library.get(b"forward_goto_example\0") }.expect("forward symbol");
            let result = unsafe { function(number) };
            marker(Call::Forward, &result.to_string())
        }
        "open" => {
            let function: Symbol<'_, OpenWithCleanup> =
                unsafe { library.get(b"open_with_cleanup\0") }.expect("open symbol");
            let result = unsafe { function(filename) };
            if result.is_null() {
                marker(Call::Open, "NULL")
            } else {
                assert_eq!(unsafe { fclose(result) }, 0, "close returned FILE");
                marker(Call::Open, "NONNULL")
            }
        }
        "driver" => {
            let function: Symbol<'_, Driver> =
                unsafe { library.get(b"driver\0") }.expect("driver symbol");
            let result = unsafe { function(number, filename) };
            marker(Call::Driver, &result.to_string())
        }
        other => panic!("unknown DIFF_CALL {other}"),
    };

    unsafe {
        fflush(ptr::null_mut());
    }
    write_all_fd(STDOUT_FILENO, &result_marker);
}

fn redirect_and_invoke_child() {
    let stdout_path = env::var_os("DIFF_STDOUT").expect("DIFF_STDOUT");
    let stderr_path = env::var_os("DIFF_STDERR").expect("DIFF_STDERR");
    let stdout_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(stdout_path)
        .expect("open stdout capture");
    let stderr_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(stderr_path)
        .expect("open stderr capture");

    let saved_stdout = unsafe { dup(STDOUT_FILENO) };
    let saved_stderr = unsafe { dup(STDERR_FILENO) };
    assert!(
        saved_stdout >= 0 && saved_stderr >= 0,
        "duplicate standard descriptors"
    );
    assert_eq!(
        unsafe { dup2(stdout_file.as_raw_fd(), STDOUT_FILENO) },
        STDOUT_FILENO
    );
    assert_eq!(
        unsafe { dup2(stderr_file.as_raw_fd(), STDERR_FILENO) },
        STDERR_FILENO
    );

    invoke_child();

    unsafe {
        fflush(ptr::null_mut());
    }
    assert_eq!(unsafe { dup2(saved_stdout, STDOUT_FILENO) }, STDOUT_FILENO);
    assert_eq!(unsafe { dup2(saved_stderr, STDERR_FILENO) }, STDERR_FILENO);
    unsafe {
        close(saved_stdout);
        close(saved_stderr);
    }
}

#[test]
fn ffi_child() {
    if env::var_os("DIFF_CHILD").is_some() {
        redirect_and_invoke_child();
    }
}

fn run_library(
    captures: &Path,
    library: &Path,
    label: &str,
    call: Call,
    number: i32,
    filename: Option<&Path>,
) -> Observation {
    let stdout_path = captures.join(format!("{label}.stdout"));
    let stderr_path = captures.join(format!("{label}.stderr"));
    File::create(&stdout_path).expect("create stdout capture");
    File::create(&stderr_path).expect("create stderr capture");

    let mut command = Command::new(env::current_exe().expect("current integration-test binary"));
    command
        .arg("--exact")
        .arg("ffi_child")
        .arg("--nocapture")
        .env("DIFF_CHILD", "1")
        .env("DIFF_LIBRARY", library)
        .env("DIFF_CALL", call.as_str())
        .env("DIFF_NUMBER", number.to_string())
        .env("DIFF_STDOUT", &stdout_path)
        .env("DIFF_STDERR", &stderr_path);
    match filename {
        Some(path) => {
            command.env("DIFF_FILENAME", path);
        }
        None => {
            command.env("DIFF_NULL_FILENAME", "1");
        }
    }

    let output = command.output().expect("run isolated FFI child");
    Observation {
        status: output.status,
        stdout: fs::read(stdout_path).expect("read stdout capture"),
        stderr: fs::read(stderr_path).expect("read stderr capture"),
    }
}

fn run_pair(
    captures: &Path,
    label: &str,
    call: Call,
    number: i32,
    filename: Option<&Path>,
) -> (Observation, Observation) {
    let c = run_library(
        captures,
        &c_library(),
        &format!("{label}-c"),
        call,
        number,
        filename,
    );
    let rust = run_library(
        captures,
        &rust_library(),
        &format!("{label}-rust"),
        call,
        number,
        filename,
    );

    assert_eq!(
        (c.status.code(), c.status.signal()),
        (rust.status.code(), rust.status.signal()),
        "{label}: process result differs"
    );
    assert_eq!(c.stdout, rust.stdout, "{label}: stdout differs");
    assert_eq!(c.stderr, rust.stderr, "{label}: stderr differs");
    (c, rust)
}

fn assert_normal_result(observation: &Observation, expected_marker: &[u8], label: &str) {
    assert!(
        observation.status.success(),
        "{label}: child failed with {:?}",
        observation.status
    );
    assert!(
        observation.stdout.ends_with(expected_marker),
        "{label}: missing result marker; stdout={:?}",
        String::from_utf8_lossy(&observation.stdout)
    );
}

fn integer_samples(rng: &mut Rng, wrapping: bool) -> Vec<i32> {
    let midpoint = i32::MAX / 2;
    let mut values = if wrapping {
        vec![midpoint + 1, i32::MAX]
    } else {
        vec![1, midpoint]
    };
    while values.len() < ITERATIONS {
        values.push(if wrapping {
            rng.inclusive_i32(midpoint + 1, i32::MAX)
        } else {
            rng.inclusive_i32(1, midpoint)
        });
    }
    values
}

fn file_content(shape: FileShape, iteration: usize, rng: &mut Rng) -> Vec<u8> {
    let length = match shape {
        FileShape::Empty => 0,
        FileShape::OneChunk if iteration == 0 => 1,
        FileShape::OneChunk if iteration == 1 => 99,
        FileShape::OneChunk => rng.inclusive_usize(1, 99),
        FileShape::ManyChunks if iteration == 0 => 100,
        FileShape::ManyChunks if iteration == 1 => 199,
        FileShape::ManyChunks => rng.inclusive_usize(100, 350),
    };
    let mut bytes = Vec::with_capacity(length);
    for _ in 0..length {
        let mut byte = rng.next_u64() as u8;
        if matches!(shape, FileShape::OneChunk) && byte == b'\n' {
            byte = b'X';
        }
        bytes.push(byte);
    }
    bytes
}

fn create_input_file(
    directory: &Path,
    shape: FileShape,
    iteration: usize,
    rng: &mut Rng,
) -> PathBuf {
    let path = directory.join(format!("{shape:?}-{iteration}-{:016x}.dat", rng.next_u64()));
    fs::write(&path, file_content(shape, iteration, rng)).expect("write randomized input");
    path
}

fn check_forward_row(captures: &Path, values: &[i32], row: usize) {
    for (iteration, &value) in values.iter().enumerate() {
        let label = format!("config-{row}-forward-{iteration}");
        let (c, _) = run_pair(captures, &label, Call::Forward, value, None);
        assert_normal_result(
            &c,
            &marker(Call::Forward, &value.wrapping_mul(2).to_string()),
            &label,
        );
    }
}

fn check_open_row(directory: &Path, captures: &Path, shape: FileShape, row: usize, rng: &mut Rng) {
    for iteration in 0..ITERATIONS {
        let path = create_input_file(directory, shape, iteration, rng);
        let label = format!("config-{row}-open-{iteration}");
        let (c, _) = run_pair(captures, &label, Call::Open, 0, Some(&path));
        assert_normal_result(&c, &marker(Call::Open, "NONNULL"), &label);
    }
}

fn check_driver_row(
    directory: &Path,
    captures: &Path,
    shape: FileShape,
    values: &[i32],
    row: usize,
    rng: &mut Rng,
) {
    for (iteration, &value) in values.iter().enumerate() {
        let path = create_input_file(directory, shape, iteration, rng);
        let label = format!("config-{row}-driver-{iteration}");
        let (c, _) = run_pair(captures, &label, Call::Driver, value, Some(&path));
        assert_normal_result(&c, &marker(Call::Driver, "0"), &label);
    }
}

#[test]
fn phase_a_and_d_all_exported_symbols_load() {
    for library_path in [c_library(), rust_library()] {
        let library = unsafe { Library::new(&library_path) }.expect("load shared library");
        unsafe {
            let _: Symbol<'_, Forward> = library
                .get(b"forward_goto_example\0")
                .expect("forward_goto_example export");
            let _: Symbol<'_, OpenWithCleanup> = library
                .get(b"open_with_cleanup\0")
                .expect("open_with_cleanup export");
            let _: Symbol<'_, Driver> = library.get(b"driver\0").expect("driver export");
        }
    }
}

#[test]
fn phase_b_every_configuration_row() {
    let temp = TestDir::new("valid");
    let captures = temp.path().join("captures");
    let inputs = temp.path().join("inputs");
    fs::create_dir_all(&captures).expect("create capture directory");
    fs::create_dir_all(&inputs).expect("create input directory");
    let mut rng = Rng::new(0x4d59_5df4_d0f3_3173);

    // CONFIGS.md rows 1-3.
    check_forward_row(&captures, &[0; ITERATIONS], 1);
    let representable = integer_samples(&mut rng, false);
    check_forward_row(&captures, &representable, 2);
    let wrapping = integer_samples(&mut rng, true);
    check_forward_row(&captures, &wrapping, 3);

    // CONFIGS.md rows 4-6.
    check_open_row(&inputs, &captures, FileShape::Empty, 4, &mut rng);
    check_open_row(&inputs, &captures, FileShape::OneChunk, 5, &mut rng);
    check_open_row(&inputs, &captures, FileShape::ManyChunks, 6, &mut rng);

    // CONFIGS.md rows 7-15: integer class crossed with file shape.
    let zeros = [0; ITERATIONS];
    check_driver_row(&inputs, &captures, FileShape::Empty, &zeros, 7, &mut rng);
    check_driver_row(
        &inputs,
        &captures,
        FileShape::Empty,
        &representable,
        8,
        &mut rng,
    );
    check_driver_row(&inputs, &captures, FileShape::Empty, &wrapping, 9, &mut rng);
    check_driver_row(
        &inputs,
        &captures,
        FileShape::OneChunk,
        &zeros,
        10,
        &mut rng,
    );
    check_driver_row(
        &inputs,
        &captures,
        FileShape::OneChunk,
        &representable,
        11,
        &mut rng,
    );
    check_driver_row(
        &inputs,
        &captures,
        FileShape::OneChunk,
        &wrapping,
        12,
        &mut rng,
    );
    check_driver_row(
        &inputs,
        &captures,
        FileShape::ManyChunks,
        &zeros,
        13,
        &mut rng,
    );
    check_driver_row(
        &inputs,
        &captures,
        FileShape::ManyChunks,
        &representable,
        14,
        &mut rng,
    );
    check_driver_row(
        &inputs,
        &captures,
        FileShape::ManyChunks,
        &wrapping,
        15,
        &mut rng,
    );
}

#[test]
fn phase_c_every_error_row_and_generic_boundary() {
    let temp = TestDir::new("errors");
    let captures = temp.path().join("captures");
    fs::create_dir_all(&captures).expect("create capture directory");
    let missing = temp.path().join("missing-file");
    let directory = temp.path().join("read-error-directory");
    fs::create_dir(&directory).expect("create read-error directory");

    // ERRORS.md row 1.
    let (c, _) = run_pair(&captures, "error-1", Call::Forward, -1, None);
    assert_normal_result(&c, &marker(Call::Forward, "-1"), "error-1");
    assert_eq!(c.stderr, b"Error: negative input\n");

    // ERRORS.md row 2.
    let (c, _) = run_pair(&captures, "error-2", Call::Open, 0, Some(&missing));
    assert_normal_result(&c, &marker(Call::Open, "NULL"), "error-2");
    assert_eq!(
        c.stderr,
        format!("Error: opening or processing file {}\n", missing.display()).as_bytes()
    );

    // ERRORS.md row 3.
    let (c, _) = run_pair(&captures, "error-3", Call::Open, 0, Some(&directory));
    assert_normal_result(&c, &marker(Call::Open, "NULL"), "error-3");
    assert_eq!(
        c.stderr,
        format!(
            "Error: opening or processing file {}\n",
            directory.display()
        )
        .as_bytes()
    );

    // ERRORS.md row 4.
    let (c, _) = run_pair(&captures, "error-4", Call::Driver, -1, Some(&missing));
    assert_normal_result(&c, &marker(Call::Driver, "-1"), "error-4");
    assert_eq!(c.stderr, b"Error: negative input\n");

    // ERRORS.md row 5.
    let (c, _) = run_pair(&captures, "error-5", Call::Driver, 7, Some(&missing));
    assert_normal_result(&c, &marker(Call::Driver, "-2"), "error-5");
    assert_eq!(
        c.stderr,
        format!("Error: opening or processing file {}\n", missing.display()).as_bytes()
    );

    // ERRORS.md row 6.
    let (c, _) = run_pair(&captures, "error-6", Call::Driver, 7, Some(&directory));
    assert_normal_result(&c, &marker(Call::Driver, "-2"), "error-6");
    assert_eq!(
        c.stderr,
        format!(
            "Error: opening or processing file {}\n",
            directory.display()
        )
        .as_bytes()
    );

    // Generic boundaries G1-G3.
    let (c, _) = run_pair(&captures, "generic-g1", Call::Open, 0, None);
    assert_normal_result(&c, &marker(Call::Open, "NULL"), "generic-g1");
    assert_eq!(c.stderr, b"Error: opening or processing file (null)\n");

    let (c, _) = run_pair(&captures, "generic-g2", Call::Driver, -1, None);
    assert_normal_result(&c, &marker(Call::Driver, "-1"), "generic-g2");

    let (c, _) = run_pair(&captures, "generic-g3", Call::Driver, 0, None);
    assert_normal_result(&c, &marker(Call::Driver, "-2"), "generic-g3");
    assert_eq!(c.stderr, b"Error: opening or processing file (null)\n");
}
