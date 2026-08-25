use libloading::Library;
use std::ffi::{CString, c_char, c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::Mutex;

type DriverFn = unsafe extern "C" fn(*const c_char, *const c_char);

const STDOUT_FILENO: c_int = 1;
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
}

struct LoadedDriver {
    _library: Library,
    driver: DriverFn,
}

impl LoadedDriver {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let driver = unsafe {
            *library
                .get::<DriverFn>(b"driver\0")
                .unwrap_or_else(|error| panic!("missing driver in {}: {error}", path.display()))
        };
        Self {
            _library: library,
            driver,
        }
    }

    fn call(&self, s1: *const c_char, s2: *const c_char) -> Vec<u8> {
        capture_stdout(|| unsafe { (self.driver)(s1, s2) })
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("c_src/build/libdriver.so")
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/debug/libdriver.so")
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().expect("stdout capture lock poisoned");
    let mut fds = [-1; 2];

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(pipe(fds.as_mut_ptr()), 0);
        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0);
        assert_eq!(dup2(fds[1], STDOUT_FILENO), STDOUT_FILENO);
        assert_eq!(close(fds[1]), 0);

        call();

        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, STDOUT_FILENO), STDOUT_FILENO);
        assert_eq!(close(saved_stdout), 0);
    }

    let mut output = Vec::new();
    unsafe { File::from_raw_fd(fds[0]) }
        .read_to_end(&mut output)
        .expect("failed to read captured stdout");
    output
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    fn len(&mut self, minimum: usize, maximum: usize) -> usize {
        minimum + (self.next() as usize % (maximum - minimum + 1))
    }

    fn nonzero_byte(&mut self) -> u8 {
        ((self.next() % 255) + 1) as u8
    }

    fn bytes(&mut self, length: usize) -> Vec<u8> {
        (0..length).map(|_| self.nonzero_byte()).collect()
    }
}

fn assert_case(
    c: &LoadedDriver,
    rust: &LoadedDriver,
    s1: Vec<u8>,
    s2: Vec<u8>,
    expected: usize,
    row: usize,
    iteration: usize,
) {
    let s1 = CString::new(s1).expect("generated s1 contained a null byte");
    let s2 = CString::new(s2).expect("generated s2 contained a null byte");
    let c_output = c.call(s1.as_ptr(), s2.as_ptr());
    let rust_output = rust.call(s1.as_ptr(), s2.as_ptr());
    let expected = format!("{expected}\n").into_bytes();

    assert_eq!(
        c_output, expected,
        "C baseline mismatch, row {row}, case {iteration}"
    );
    assert_eq!(
        rust_output, c_output,
        "Rust/C mismatch, row {row}, case {iteration}"
    );
}

#[test]
fn all_valid_configuration_rows_match() {
    assert!(
        c_library_path().is_file(),
        "build the C shared library first"
    );
    assert!(
        rust_library_path().is_file(),
        "build the Rust shared library first"
    );

    let c = unsafe { LoadedDriver::load(&c_library_path()) };
    let rust = unsafe { LoadedDriver::load(&rust_library_path()) };
    let mut rng = Rng(0x5eed_c0de_d15c_a11);

    for iteration in 0..64 {
        assert_case(&c, &rust, vec![], vec![], 0, 1, iteration);
    }

    for iteration in 0..128 {
        let reject_length = rng.len(1, 64);
        let reject = rng.bytes(reject_length);
        assert_case(&c, &rust, vec![], reject, 0, 2, iteration);
    }

    for iteration in 0..128 {
        let source_length = rng.len(1, 128);
        let source = rng.bytes(source_length);
        assert_case(&c, &rust, source, vec![], source_length, 3, iteration);
    }

    for iteration in 0..128 {
        let source_length = rng.len(1, 128);
        let source = rng.bytes(source_length);
        let reject_length = rng.len(1, 32);
        let mut reject = rng.bytes(reject_length);
        let replacement_index = rng.next() as usize % reject.len();
        reject[replacement_index] = source[0];
        assert_case(&c, &rust, source, reject, 0, 4, iteration);
    }

    for iteration in 0..128 {
        let source_length = rng.len(2, 128);
        let match_index = rng.len(1, source_length - 1);
        let mut source: Vec<u8> = (0..source_length)
            .map(|_| (rng.next() % 127 + 1) as u8)
            .collect();
        let rejected = (rng.next() % 128 + 128) as u8;
        source[match_index] = rejected;
        let mut reject = vec![rejected];
        reject.extend((0..rng.len(0, 31)).map(|_| (rng.next() % 128 + 128) as u8));
        assert_case(&c, &rust, source, reject, match_index, 5, iteration);
    }

    for iteration in 0..128 {
        let source_length = rng.len(1, 128);
        let reject_length = rng.len(1, 64);
        let source = (0..source_length)
            .map(|_| (rng.next() % 127 + 1) as u8)
            .collect();
        let reject = (0..reject_length)
            .map(|_| (rng.next() % 128 + 128) as u8)
            .collect();
        assert_case(&c, &rust, source, reject, source_length, 6, iteration);
    }
}

#[test]
fn null_pointer_probe_child() {
    let Ok(library_kind) = std::env::var("DRIVER_NULL_PROBE_LIBRARY") else {
        return;
    };
    let null_argument = std::env::var("DRIVER_NULL_PROBE_ARGUMENT").unwrap();
    let path = match library_kind.as_str() {
        "c" => c_library_path(),
        "rust" => rust_library_path(),
        _ => panic!("unknown library kind {library_kind}"),
    };
    let library = unsafe { LoadedDriver::load(&path) };
    let valid = c"abc";

    match null_argument.as_str() {
        "s1" => {
            library.call(ptr::null(), valid.as_ptr());
        }
        "s2" => {
            library.call(valid.as_ptr(), ptr::null());
        }
        _ => panic!("unknown null argument {null_argument}"),
    }
}

#[test]
fn null_pointer_process_behavior_matches() {
    let current_test = std::env::current_exe().expect("test executable path unavailable");

    for argument in ["s1", "s2"] {
        let run = |library: &str| {
            Command::new(&current_test)
                .args(["--exact", "null_pointer_probe_child", "--nocapture"])
                .env("DRIVER_NULL_PROBE_LIBRARY", library)
                .env("DRIVER_NULL_PROBE_ARGUMENT", argument)
                .status()
                .unwrap_or_else(|error| panic!("failed to run {library} null probe: {error}"))
        };

        let c_status = run("c");
        let rust_status = run("rust");
        assert!(
            !c_status.success(),
            "C unexpectedly accepted null {argument}"
        );
        assert_eq!(
            rust_status.signal(),
            c_status.signal(),
            "Rust/C termination mismatch for null {argument}"
        );
        assert_eq!(
            rust_status.code(),
            c_status.code(),
            "Rust/C exit-code mismatch for null {argument}"
        );
    }
}
