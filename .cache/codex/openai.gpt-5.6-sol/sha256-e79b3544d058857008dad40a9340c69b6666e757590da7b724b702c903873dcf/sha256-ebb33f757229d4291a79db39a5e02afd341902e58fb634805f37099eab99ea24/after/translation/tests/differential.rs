use libloading::{Library, Symbol};
use std::env;
use std::ffi::{CString, c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::Mutex;

type SliceFn = unsafe extern "C" fn(*mut c_char, *mut c_int, *mut c_int) -> c_int;

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
}

const STDOUT_FD: c_int = 1;
const START_ERROR: &[u8] = b"Error: start is off the end of the string!\n";
const STOP_ERROR: &[u8] = b"Error: stop is off the end of the string!\n";
const ORDER_ERROR: &[u8] = b"Error: stop must come after start!\n";
const RANDOM_CASES: usize = 128;
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    return_code: c_int,
    stdout: Vec<u8>,
}

#[derive(Clone)]
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

    fn usize_inclusive(&mut self, low: usize, high: usize) -> usize {
        assert!(low <= high);
        low + (self.next_u64() as usize % (high - low + 1))
    }

    fn nonzero_byte(&mut self) -> u8 {
        (self.next_u64() % 255 + 1) as u8
    }

    fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.nonzero_byte()).collect()
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libString_Slice.so")
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libString_Slice.so")
}

fn assert_libraries_exist() {
    assert!(
        c_library_path().is_file(),
        "build the C library before running tests: {}",
        c_library_path().display()
    );
    assert!(
        rust_library_path().is_file(),
        "build the Rust release library before running tests: {}",
        rust_library_path().display()
    );
}

unsafe fn capture_stdout(call: impl FnOnce() -> c_int) -> Outcome {
    let _guard = STDOUT_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut pipe_fds = [-1, -1];

    assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0);
    assert_eq!(unsafe { pipe(pipe_fds.as_mut_ptr()) }, 0);

    let saved_stdout = unsafe { dup(STDOUT_FD) };
    assert!(saved_stdout >= 0);
    assert_eq!(unsafe { dup2(pipe_fds[1], STDOUT_FD) }, STDOUT_FD);
    assert_eq!(unsafe { close(pipe_fds[1]) }, 0);

    let return_code = call();

    assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0);
    assert_eq!(unsafe { dup2(saved_stdout, STDOUT_FD) }, STDOUT_FD);
    assert_eq!(unsafe { close(saved_stdout) }, 0);

    let mut stdout = Vec::new();
    let mut chunk = [0_u8; 256];
    loop {
        let count = unsafe {
            read(
                pipe_fds[0],
                chunk.as_mut_ptr().cast::<c_void>(),
                chunk.len(),
            )
        };
        assert!(count >= 0);
        if count == 0 {
            break;
        }
        stdout.extend_from_slice(&chunk[..count as usize]);
    }
    assert_eq!(unsafe { close(pipe_fds[0]) }, 0);

    Outcome {
        return_code,
        stdout,
    }
}

fn call_library(
    library_path: &Path,
    bytes: &[u8],
    start: Option<c_int>,
    stop: Option<c_int>,
) -> Outcome {
    let string = CString::new(bytes).expect("random test data contains no NUL");
    let mut start_value = start.unwrap_or_default();
    let mut stop_value = stop.unwrap_or_default();
    let start_ptr = if start.is_some() {
        &mut start_value
    } else {
        ptr::null_mut()
    };
    let stop_ptr = if stop.is_some() {
        &mut stop_value
    } else {
        ptr::null_mut()
    };

    unsafe {
        let library = Library::new(library_path)
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", library_path.display()));
        let slice: Symbol<'_, SliceFn> = library
            .get(b"slice\0")
            .unwrap_or_else(|error| panic!("missing slice export: {error}"));
        capture_stdout(|| slice(string.as_ptr().cast_mut(), start_ptr, stop_ptr))
    }
}

fn compare(bytes: &[u8], start: Option<c_int>, stop: Option<c_int>) -> Outcome {
    assert_libraries_exist();
    let c = call_library(&c_library_path(), bytes, start, stop);
    let rust = call_library(&rust_library_path(), bytes, start, stop);
    assert_eq!(
        rust, c,
        "C/Rust divergence for bytes={bytes:?}, start={start:?}, stop={stop:?}"
    );
    c
}

fn expected_success(bytes: &[u8], start: usize, stop: usize) -> Outcome {
    let mut stdout = bytes[start..stop].to_vec();
    stdout.push(b'\n');
    Outcome {
        return_code: 0,
        stdout,
    }
}

#[test]
fn config_01_null_start_null_stop_empty() {
    for _ in 0..RANDOM_CASES {
        assert_eq!(compare(b"", None, None), expected_success(b"", 0, 0));
    }
}

#[test]
fn config_02_null_start_null_stop_one_byte() {
    let mut rng = Rng::new(0x0202_0202_0202_0202);
    for _ in 0..RANDOM_CASES {
        let bytes = rng.bytes(1);
        assert_eq!(compare(&bytes, None, None), expected_success(&bytes, 0, 1));
    }
}

#[test]
fn config_03_null_start_null_stop_many_bytes() {
    let mut rng = Rng::new(0x0303_0303_0303_0303);
    for _ in 0..RANDOM_CASES {
        let len = rng.usize_inclusive(2, 128);
        let bytes = rng.bytes(len);
        assert_eq!(
            compare(&bytes, None, None),
            expected_success(&bytes, 0, len)
        );
    }
}

#[test]
fn config_04_present_start_null_stop_empty() {
    for _ in 0..RANDOM_CASES {
        assert_eq!(compare(b"", Some(0), None), expected_success(b"", 0, 0));
    }
}

#[test]
fn config_05_present_zero_start_null_stop_nonempty() {
    let mut rng = Rng::new(0x0505_0505_0505_0505);
    for _ in 0..RANDOM_CASES {
        let len = rng.usize_inclusive(1, 128);
        let bytes = rng.bytes(len);
        assert_eq!(
            compare(&bytes, Some(0), None),
            expected_success(&bytes, 0, len)
        );
    }
}

#[test]
fn config_06_present_interior_start_null_stop_many() {
    let mut rng = Rng::new(0x0606_0606_0606_0606);
    for _ in 0..RANDOM_CASES {
        let len = rng.usize_inclusive(2, 128);
        let start = rng.usize_inclusive(1, len - 1);
        let bytes = rng.bytes(len);
        assert_eq!(
            compare(&bytes, Some(start as c_int), None),
            expected_success(&bytes, start, len)
        );
    }
}

#[test]
fn config_07_present_end_start_null_stop_nonempty() {
    let mut rng = Rng::new(0x0707_0707_0707_0707);
    for _ in 0..RANDOM_CASES {
        let len = rng.usize_inclusive(1, 128);
        let bytes = rng.bytes(len);
        assert_eq!(
            compare(&bytes, Some(len as c_int), None),
            expected_success(&bytes, len, len)
        );
    }
}

#[test]
fn config_08_null_start_present_valid_stop() {
    let mut rng = Rng::new(0x0808_0808_0808_0808);
    for case in 0..RANDOM_CASES {
        let len = rng.usize_inclusive(1, 128);
        let stop = match case % 3 {
            0 => 1,
            1 => len,
            _ => rng.usize_inclusive(1, len),
        };
        let bytes = rng.bytes(len);
        assert_eq!(
            compare(&bytes, None, Some(stop as c_int)),
            expected_success(&bytes, 0, stop)
        );
    }
}

#[test]
fn config_09_present_start_stop_one_byte() {
    let mut rng = Rng::new(0x0909_0909_0909_0909);
    for _ in 0..RANDOM_CASES {
        let bytes = rng.bytes(1);
        assert_eq!(
            compare(&bytes, Some(0), Some(1)),
            expected_success(&bytes, 0, 1)
        );
    }
}

#[test]
fn config_10_present_start_stop_many_bytes() {
    let mut rng = Rng::new(0x1010_1010_1010_1010);
    for case in 0..RANDOM_CASES {
        let len = rng.usize_inclusive(2, 128);
        let (start, stop) = match case % 4 {
            0 => (0, len),
            1 => (0, rng.usize_inclusive(1, len)),
            2 => {
                let start = rng.usize_inclusive(0, len - 1);
                (start, len)
            }
            _ => {
                let start = rng.usize_inclusive(0, len - 1);
                (start, rng.usize_inclusive(start + 1, len))
            }
        };
        let bytes = rng.bytes(len);
        assert_eq!(
            compare(&bytes, Some(start as c_int), Some(stop as c_int),),
            expected_success(&bytes, start, stop)
        );
    }
}

#[test]
fn error_01_start_beyond_end_or_negative() {
    let mut rng = Rng::new(0xe101_e101_e101_e101);
    for case in 0..RANDOM_CASES {
        let len = rng.usize_inclusive(0, 128);
        let bytes = rng.bytes(len);
        let start = match case % 4 {
            0 => len as c_int + 1,
            1 => c_int::MAX,
            2 => c_int::MIN,
            _ => -(rng.usize_inclusive(1, i16::MAX as usize) as c_int),
        };
        let outcome = compare(&bytes, Some(start), None);
        assert_eq!(outcome.return_code, 1);
        assert_eq!(outcome.stdout, START_ERROR);
    }
}

#[test]
fn error_02_stop_beyond_end_or_negative() {
    let mut rng = Rng::new(0xe202_e202_e202_e202);
    for case in 0..RANDOM_CASES {
        let len = rng.usize_inclusive(0, 128);
        let bytes = rng.bytes(len);
        let start = rng.usize_inclusive(0, len) as c_int;
        let stop = match case % 4 {
            0 => len as c_int + 1,
            1 => c_int::MAX,
            2 => c_int::MIN,
            _ => -(rng.usize_inclusive(1, i16::MAX as usize) as c_int),
        };
        let outcome = compare(&bytes, Some(start), Some(stop));
        assert_eq!(outcome.return_code, 1);
        assert_eq!(outcome.stdout, STOP_ERROR);
    }
}

#[test]
fn error_03_stop_not_after_start() {
    let mut rng = Rng::new(0xe303_e303_e303_e303);
    for case in 0..RANDOM_CASES {
        let len = rng.usize_inclusive(0, 128);
        let bytes = rng.bytes(len);
        let start = rng.usize_inclusive(0, len);
        let stop = match case % 3 {
            0 => start,
            1 if start > 0 => 0,
            _ => rng.usize_inclusive(0, start),
        };
        let outcome = compare(&bytes, Some(start as c_int), Some(stop as c_int));
        assert_eq!(outcome.return_code, 1);
        assert_eq!(outcome.stdout, ORDER_ERROR);
    }
}

#[test]
fn generic_null_string_matches_in_isolated_processes() {
    assert_libraries_exist();
    let test_binary = env::current_exe().expect("test binary path");

    let run = |library: &Path| {
        Command::new(&test_binary)
            .arg("--exact")
            .arg("null_string_child")
            .arg("--nocapture")
            .env("SLICE_NULL_CHILD_LIBRARY", library)
            .output()
            .unwrap_or_else(|error| panic!("failed to run null-string child: {error}"))
    };

    let c = run(&c_library_path());
    let rust = run(&rust_library_path());

    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(rust.status.signal(), c.status.signal());
        assert!(
            c.status.signal().is_some(),
            "C unexpectedly survived strlen(NULL): {c:?}"
        );
    }

    #[cfg(not(unix))]
    {
        assert_eq!(rust.status.code(), c.status.code());
        assert!(!c.status.success());
    }
}

#[test]
fn null_string_child() {
    let Ok(library_path) = env::var("SLICE_NULL_CHILD_LIBRARY") else {
        return;
    };

    unsafe {
        let library = Library::new(&library_path)
            .unwrap_or_else(|error| panic!("failed to load {library_path}: {error}"));
        let slice: Symbol<'_, SliceFn> = library
            .get(b"slice\0")
            .unwrap_or_else(|error| panic!("missing slice export: {error}"));
        let _ = slice(ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
    }
}
