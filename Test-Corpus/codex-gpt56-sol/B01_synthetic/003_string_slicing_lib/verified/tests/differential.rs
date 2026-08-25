use libloading::{Library, Symbol};
use std::env;
use std::ffi::{c_char, c_int, c_void};
use std::os::fd::RawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::Mutex;

type Slice = unsafe extern "C" fn(*mut c_char, *mut c_int, *mut c_int) -> c_int;

unsafe extern "C" {
    fn close(fd: RawFd) -> c_int;
    fn dup(fd: RawFd) -> RawFd;
    fn dup2(old_fd: RawFd, new_fd: RawFd) -> RawFd;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(fds: *mut RawFd) -> c_int;
    fn read(fd: RawFd, buffer: *mut c_void, count: usize) -> isize;
}

const STDOUT_FILENO: RawFd = 1;
const RANDOM_CASES: usize = 128;
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Eq, PartialEq)]
struct Outcome {
    return_code: c_int,
    stdout: Vec<u8>,
    buffer: Vec<u8>,
}

#[derive(Clone, Copy)]
struct Bounds {
    start: Option<c_int>,
    stop: Option<c_int>,
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn usize_in(&mut self, start: usize, end_exclusive: usize) -> usize {
        start + self.next_u64() as usize % (end_exclusive - start)
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libString_Slice.so")
}

fn rust_library_path() -> PathBuf {
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_owned());
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(profile)
        .join("libString_Slice.so")
}

fn random_buffer(rng: &mut Rng, logical_len: usize) -> Vec<u8> {
    let trailing_len = rng.usize_in(0, 17);
    let mut bytes = Vec::with_capacity(logical_len + 1 + trailing_len);
    for _ in 0..logical_len {
        bytes.push(rng.usize_in(1, 256) as u8);
    }
    bytes.push(0);
    for _ in 0..trailing_len {
        bytes.push(rng.usize_in(0, 256) as u8);
    }
    bytes
}

unsafe fn read_all(fd: RawFd) -> Vec<u8> {
    let mut output = Vec::new();
    loop {
        let mut chunk = [0_u8; 4096];
        let count = unsafe { read(fd, chunk.as_mut_ptr().cast(), chunk.len()) };
        assert!(count >= 0, "read from stdout pipe failed");
        if count == 0 {
            break;
        }
        output.extend_from_slice(&chunk[..count as usize]);
    }
    output
}

unsafe fn invoke(path: &Path, mut buffer: Vec<u8>, bounds: Bounds) -> Outcome {
    let _guard = STDOUT_LOCK.lock().expect("stdout lock poisoned");
    let library = unsafe { Library::new(path) }
        .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
    let slice: Symbol<Slice> = unsafe { library.get(b"slice\0") }
        .unwrap_or_else(|error| panic!("failed to load slice from {}: {error}", path.display()));

    let mut start = bounds.start.unwrap_or_default();
    let mut stop = bounds.stop.unwrap_or_default();
    let start_ptr = if bounds.start.is_some() {
        &mut start
    } else {
        ptr::null_mut()
    };
    let stop_ptr = if bounds.stop.is_some() {
        &mut stop
    } else {
        ptr::null_mut()
    };

    let mut pipe_fds = [-1; 2];
    assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0);
    assert_eq!(unsafe { pipe(pipe_fds.as_mut_ptr()) }, 0);
    let saved_stdout = unsafe { dup(STDOUT_FILENO) };
    assert!(saved_stdout >= 0);
    assert_eq!(unsafe { dup2(pipe_fds[1], STDOUT_FILENO) }, STDOUT_FILENO);
    assert_eq!(unsafe { close(pipe_fds[1]) }, 0);

    let return_code = unsafe { slice(buffer.as_mut_ptr().cast(), start_ptr, stop_ptr) };

    assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0);
    assert_eq!(unsafe { dup2(saved_stdout, STDOUT_FILENO) }, STDOUT_FILENO);
    assert_eq!(unsafe { close(saved_stdout) }, 0);
    let stdout = unsafe { read_all(pipe_fds[0]) };
    assert_eq!(unsafe { close(pipe_fds[0]) }, 0);

    Outcome {
        return_code,
        stdout,
        buffer,
    }
}

fn compare_case(row: usize, iteration: usize, buffer: Vec<u8>, bounds: Bounds) -> Outcome {
    let c = unsafe { invoke(&c_library_path(), buffer.clone(), bounds) };
    let rust = unsafe { invoke(&rust_library_path(), buffer, bounds) };
    assert_eq!(
        rust, c,
        "differential mismatch in row {row}, randomized iteration {iteration}"
    );
    c
}

fn one_or_many_len(rng: &mut Rng, iteration: usize) -> usize {
    if iteration % 2 == 0 {
        1
    } else if iteration == RANDOM_CASES - 1 {
        4096
    } else {
        rng.usize_in(2, 129)
    }
}

fn valid_configuration_surface_matches() {
    let mut rng = Rng::new(0xd1ff_e2e0_5eed_1234);

    for iteration in 0..RANDOM_CASES {
        let cases = [
            (
                1,
                0,
                Bounds {
                    start: None,
                    stop: None,
                },
            ),
            (
                2,
                one_or_many_len(&mut rng, iteration),
                Bounds {
                    start: None,
                    stop: None,
                },
            ),
            (
                3,
                0,
                Bounds {
                    start: Some(0),
                    stop: None,
                },
            ),
            (
                4,
                one_or_many_len(&mut rng, iteration),
                Bounds {
                    start: Some(0),
                    stop: None,
                },
            ),
        ];

        for (row, len, bounds) in cases {
            let outcome = compare_case(row, iteration, random_buffer(&mut rng, len), bounds);
            assert_eq!(outcome.return_code, 0, "CONFIGS.md row {row}");
        }

        let len = rng.usize_in(2, 129);
        let start = rng.usize_in(1, len) as c_int;
        let outcome = compare_case(
            5,
            iteration,
            random_buffer(&mut rng, len),
            Bounds {
                start: Some(start),
                stop: None,
            },
        );
        assert_eq!(outcome.return_code, 0, "CONFIGS.md row 5");

        let len = one_or_many_len(&mut rng, iteration);
        let outcome = compare_case(
            6,
            iteration,
            random_buffer(&mut rng, len),
            Bounds {
                start: Some(len as c_int),
                stop: None,
            },
        );
        assert_eq!(outcome.return_code, 0, "CONFIGS.md row 6");

        let len = one_or_many_len(&mut rng, iteration);
        let outcome = compare_case(
            7,
            iteration,
            random_buffer(&mut rng, len),
            Bounds {
                start: None,
                stop: Some(len as c_int),
            },
        );
        assert_eq!(outcome.return_code, 0, "CONFIGS.md row 7");

        let len = rng.usize_in(2, 129);
        let stop = rng.usize_in(1, len) as c_int;
        let outcome = compare_case(
            8,
            iteration,
            random_buffer(&mut rng, len),
            Bounds {
                start: None,
                stop: Some(stop),
            },
        );
        assert_eq!(outcome.return_code, 0, "CONFIGS.md row 8");

        let len = one_or_many_len(&mut rng, iteration);
        let outcome = compare_case(
            9,
            iteration,
            random_buffer(&mut rng, len),
            Bounds {
                start: Some(0),
                stop: Some(len as c_int),
            },
        );
        assert_eq!(outcome.return_code, 0, "CONFIGS.md row 9");

        let len = rng.usize_in(2, 129);
        let stop = rng.usize_in(1, len) as c_int;
        let outcome = compare_case(
            10,
            iteration,
            random_buffer(&mut rng, len),
            Bounds {
                start: Some(0),
                stop: Some(stop),
            },
        );
        assert_eq!(outcome.return_code, 0, "CONFIGS.md row 10");

        let len = rng.usize_in(2, 129);
        let start = rng.usize_in(1, len) as c_int;
        let outcome = compare_case(
            11,
            iteration,
            random_buffer(&mut rng, len),
            Bounds {
                start: Some(start),
                stop: Some(len as c_int),
            },
        );
        assert_eq!(outcome.return_code, 0, "CONFIGS.md row 11");

        let len = rng.usize_in(3, 129);
        let start = rng.usize_in(1, len - 1);
        let stop = rng.usize_in(start + 1, len);
        let outcome = compare_case(
            12,
            iteration,
            random_buffer(&mut rng, len),
            Bounds {
                start: Some(start as c_int),
                stop: Some(stop as c_int),
            },
        );
        assert_eq!(outcome.return_code, 0, "CONFIGS.md row 12");
    }
}

fn explicit_error_surface_matches() {
    const START_ERROR: &[u8] = b"Error: start is off the end of the string!\n";
    const STOP_ERROR: &[u8] = b"Error: stop is off the end of the string!\n";
    const ORDER_ERROR: &[u8] = b"Error: stop must come after start!\n";

    let mut rng = Rng::new(0xe220_a770_5eed_5678);
    for iteration in 0..RANDOM_CASES {
        let len = rng.usize_in(0, 129);

        let invalid_start = match iteration % 4 {
            0 => -1,
            1 => c_int::MIN,
            2 => len as c_int + 1,
            _ => c_int::MAX,
        };
        let outcome = compare_case(
            1,
            iteration,
            random_buffer(&mut rng, len),
            Bounds {
                start: Some(invalid_start),
                stop: if iteration % 2 == 0 {
                    None
                } else {
                    Some(len as c_int)
                },
            },
        );
        assert_eq!(
            (outcome.return_code, outcome.stdout.as_slice()),
            (1, START_ERROR)
        );

        let invalid_stop = match iteration % 4 {
            0 => -1,
            1 => c_int::MIN,
            2 => len as c_int + 1,
            _ => c_int::MAX,
        };
        let outcome = compare_case(
            2,
            iteration,
            random_buffer(&mut rng, len),
            Bounds {
                start: if iteration % 2 == 0 { None } else { Some(0) },
                stop: Some(invalid_stop),
            },
        );
        assert_eq!(
            (outcome.return_code, outcome.stdout.as_slice()),
            (1, STOP_ERROR)
        );

        let start = rng.usize_in(0, len + 1) as c_int;
        let stop = rng.usize_in(0, start as usize + 1) as c_int;
        let outcome = compare_case(
            3,
            iteration,
            random_buffer(&mut rng, len),
            Bounds {
                start: if iteration % 4 == 0 {
                    None
                } else {
                    Some(start)
                },
                stop: Some(if iteration % 4 == 0 { 0 } else { stop }),
            },
        );
        assert_eq!(
            (outcome.return_code, outcome.stdout.as_slice()),
            (1, ORDER_ERROR)
        );
    }
}

fn exported_symbol_is_loadable_from_both_libraries() {
    for path in [c_library_path(), rust_library_path()] {
        let library = unsafe { Library::new(&path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let _: Symbol<Slice> = unsafe { library.get(b"slice\0") }
            .unwrap_or_else(|error| panic!("missing slice in {}: {error}", path.display()));
    }
}

fn null_string_process_behavior_matches() {
    let test_binary = env::current_exe().expect("current test binary");
    let mut statuses = Vec::new();

    for path in [c_library_path(), rust_library_path()] {
        let status = Command::new(&test_binary)
            .args([
                "--ignored",
                "--exact",
                "ffi_null_mystr_child",
                "--nocapture",
            ])
            .env("SLICE_NULL_LIBRARY", path)
            .status()
            .expect("run null-string child");
        assert!(
            !status.success(),
            "slice(NULL, NULL, NULL) unexpectedly succeeded"
        );
        statuses.push((status.code(), status.signal()));
    }

    assert_eq!(
        statuses[1], statuses[0],
        "null-string process behavior differs"
    );
}

#[test]
fn complete_differential_surface_matches() {
    exported_symbol_is_loadable_from_both_libraries();
    valid_configuration_surface_matches();
    explicit_error_surface_matches();
    null_string_process_behavior_matches();
}

#[test]
#[ignore = "process-isolated helper for null_string_process_behavior_matches"]
fn ffi_null_mystr_child() {
    let path = env::var_os("SLICE_NULL_LIBRARY").expect("SLICE_NULL_LIBRARY");
    let library = unsafe { Library::new(&path) }.expect("load child library");
    let slice: Symbol<Slice> = unsafe { library.get(b"slice\0") }.expect("load slice");
    unsafe {
        slice(ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
    }
}
