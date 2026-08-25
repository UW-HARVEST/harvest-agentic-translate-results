use libloading::Library;
use std::ffi::{c_char, c_int, c_void, CString};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};

type Driver = unsafe extern "C" fn(*const c_char, *const c_char);
type Main = unsafe extern "C" fn() -> c_int;

unsafe extern "C" {
    static mut stdin: *mut c_void;

    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn freopen(path: *const c_char, mode: *const c_char, stream: *mut c_void) -> *mut c_void;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(status: c_int) -> !;
}

struct Rng(u64);

impl Rng {
    fn new() -> Self {
        Self(0x4d59_5df4_d0f3_3173)
    }

    fn next(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn len(&mut self, minimum: usize, maximum: usize) -> usize {
        minimum + self.next() as usize % (maximum - minimum + 1)
    }

    fn bytes(&mut self, length: usize, first: u8, count: u8) -> Vec<u8> {
        (0..length)
            .map(|_| first + (self.next() % u64::from(count)) as u8)
            .collect()
    }

    fn variable_bytes(&mut self, minimum: usize, maximum: usize, first: u8, count: u8) -> Vec<u8> {
        let length = self.len(minimum, maximum);
        self.bytes(length, first, count)
    }
}

fn manifest_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn capture_stdout<T>(call: impl FnOnce() -> T) -> (T, Vec<u8>) {
    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);

        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0);

        let mut pipe_fds = [-1, -1];
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0);
        assert_eq!(dup2(pipe_fds[1], 1), 1);
        assert_eq!(close(pipe_fds[1]), 0);

        let result = call();

        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(close(saved_stdout), 0);

        let mut output = Vec::new();
        File::from_raw_fd(pipe_fds[0])
            .read_to_end(&mut output)
            .expect("read captured stdout");
        (result, output)
    }
}

fn with_stdin<T>(input: &[u8], call: impl FnOnce() -> T) -> T {
    static NEXT_FILE: AtomicU64 = AtomicU64::new(0);

    let sequence = NEXT_FILE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "driver-differential-{}-{sequence}.input",
        std::process::id()
    ));
    std::fs::write(&path, input).expect("write stdin fixture");
    let c_path = CString::new(path.as_os_str().as_encoded_bytes()).expect("fixture path");

    let result = unsafe {
        let saved_stdin = dup(0);
        assert!(saved_stdin >= 0);
        assert!(!freopen(c_path.as_ptr(), c"rb".as_ptr(), stdin).is_null());

        let result = call();

        assert_eq!(dup2(saved_stdin, 0), 0);
        assert_eq!(close(saved_stdin), 0);
        result
    };

    std::fs::remove_file(path).expect("remove stdin fixture");
    result
}

fn invoke_driver(function: Driver, s1: &[u8], s2: &[u8]) -> Vec<u8> {
    let s1 = CString::new(s1).expect("s1 is a C string");
    let s2 = CString::new(s2).expect("s2 is a C string");
    capture_stdout(|| unsafe { function(s1.as_ptr(), s2.as_ptr()) }).1
}

fn invoke_main(function: Main, input: &[u8]) -> (c_int, Vec<u8>) {
    with_stdin(input, || capture_stdout(|| unsafe { function() }))
}

fn compare_driver(c: Driver, rust: Driver, row: &str, case: usize, s1: &[u8], s2: &[u8]) {
    let c_output = invoke_driver(c, s1, s2);
    let rust_output = invoke_driver(rust, s1, s2);
    assert_eq!(rust_output, c_output, "{row}, randomized case {case}");
}

fn compare_main(c: Main, rust: Main, row: &str, case: usize, input: &[u8]) {
    let c_result = invoke_main(c, input);
    let rust_result = invoke_main(rust, input);
    assert_eq!(rust_result, c_result, "{row}, randomized case {case}");
}

fn child_status(call: impl FnOnce()) -> c_int {
    unsafe {
        let pid = fork();
        assert!(pid >= 0);
        if pid == 0 {
            call();
            _exit(0);
        }

        let mut status = 0;
        assert_eq!(waitpid(pid, &mut status, 0), pid);
        status
    }
}

fn valid_path_cases(c_driver: Driver, rust_driver: Driver, c_main: Main, rust_main: Main) {
    let mut rng = Rng::new();

    for case in 0..64 {
        compare_driver(c_driver, rust_driver, "D1", case, b"", b"");
        compare_driver(
            c_driver,
            rust_driver,
            "D2",
            case,
            b"",
            &rng.variable_bytes(1, 32, b'a', 26),
        );
        compare_driver(
            c_driver,
            rust_driver,
            "D3",
            case,
            &rng.variable_bytes(1, 256, 1, 254),
            b"",
        );

        let value = rng.variable_bytes(1, 128, b'a', 20);
        let mut rejected = rng.variable_bytes(1, 16, b'u', 6);
        rejected.push(value[0]);
        compare_driver(c_driver, rust_driver, "D4", case, &value, &rejected);

        let mut value = rng.variable_bytes(3, 128, b'a', 20);
        let index = rng.len(1, value.len() - 2);
        value[index] = b'z';
        compare_driver(c_driver, rust_driver, "D5", case, &value, b"z");

        let mut value = rng.variable_bytes(2, 128, b'a', 20);
        *value.last_mut().unwrap() = b'z';
        compare_driver(c_driver, rust_driver, "D6", case, &value, b"z");

        compare_driver(
            c_driver,
            rust_driver,
            "D7",
            case,
            &rng.variable_bytes(1, 256, b'a', 13),
            &rng.variable_bytes(1, 32, b'n', 13),
        );
    }

    for case in 0..48 {
        compare_main(c_main, rust_main, "M1", case, b"\n\n");

        let first = rng.variable_bytes(1, 80, b'a', 13);
        let second = rng.variable_bytes(1, 18, b'n', 13);
        let input = [first.as_slice(), b"\n", second.as_slice(), b"\n"].concat();
        compare_main(c_main, rust_main, "M2", case, &input);

        let first = rng.variable_bytes(1, 80, b'a', 26);
        let second = [rng.variable_bytes(0, 12, b'a', 26), vec![first[0]]].concat();
        let input = [first.as_slice(), b"\n", second.as_slice(), b"\n"].concat();
        compare_main(c_main, rust_main, "M3", case, &input);

        let mut first = rng.variable_bytes(2, 80, b'a', 20);
        let index = if case % 2 == 0 {
            rng.len(1, first.len() - 1)
        } else {
            first.len() - 1
        };
        first[index] = b'z';
        let input = [first.as_slice(), b"\nz\n"].concat();
        compare_main(c_main, rust_main, "M4", case, &input);

        let first = rng.bytes(98, b'a', 26);
        let input = [first.as_slice(), b"\n", b"q\n"].concat();
        compare_main(c_main, rust_main, "M5", case, &input);

        let first = rng.bytes(99, b'a', 26);
        let input = [first.as_slice(), b"\nignored\n"].concat();
        compare_main(c_main, rust_main, "M6", case, &input);

        let first = rng.variable_bytes(198, 240, b'a', 26);
        let input = [first.as_slice(), b"\nignored\n"].concat();
        compare_main(c_main, rust_main, "M7", case, &input);

        let first = rng.variable_bytes(1, 80, b'a', 26);
        let second = rng.variable_bytes(1, 98, b'a', 26);
        let input = [first.as_slice(), b"\n", second.as_slice()].concat();
        compare_main(c_main, rust_main, "M8", case, &input);

        let prefix = rng.variable_bytes(1, 20, b'a', 26);
        let suffix = rng.variable_bytes(0, 20, b'a', 26);
        let input = [prefix.as_slice(), b"\0", suffix.as_slice(), b"\n", b"z\n"].concat();
        compare_main(c_main, rust_main, "M9", case, &input);
    }
}

fn generic_boundary_cases(c_driver: Driver, rust_driver: Driver, c_main: Main, rust_main: Main) {
    let mut rng = Rng::new();

    for (case, length) in [0, 1, 98, 99, 100, 101, 4096, 1_048_576]
        .into_iter()
        .enumerate()
    {
        let value = rng.bytes(length, b'a', 20);
        compare_driver(
            c_driver,
            rust_driver,
            "generic length boundary",
            case,
            &value,
            b"z",
        );
    }

    let valid = CString::new("abc").unwrap();
    let c_null_s1 = child_status(|| unsafe { c_driver(ptr::null(), valid.as_ptr()) });
    let rust_null_s1 = child_status(|| unsafe { rust_driver(ptr::null(), valid.as_ptr()) });
    assert_eq!(rust_null_s1, c_null_s1, "null s1 process status");

    let c_null_s2 = child_status(|| unsafe { c_driver(valid.as_ptr(), ptr::null()) });
    let rust_null_s2 = child_status(|| unsafe { rust_driver(valid.as_ptr(), ptr::null()) });
    assert_eq!(rust_null_s2, c_null_s2, "null s2 process status");

    let c_empty_eof = child_status(|| {
        with_stdin(b"", || unsafe {
            c_main();
        });
    });
    let rust_empty_eof = child_status(|| {
        with_stdin(b"", || unsafe {
            rust_main();
        });
    });
    assert_eq!(rust_empty_eof, c_empty_eof, "empty EOF process status");
}

#[test]
fn c_and_rust_shared_libraries_are_byte_identical() {
    let c_path = manifest_path("c_src/build/libdriver_c.so");
    let rust_path = manifest_path("target/debug/libdriver.so");
    assert!(c_path.is_file(), "missing C shared library: {c_path:?}");
    assert!(
        rust_path.is_file(),
        "missing Rust shared library: {rust_path:?}"
    );

    unsafe {
        let c_library = Library::new(&c_path).expect("load C shared library");
        let rust_library = Library::new(&rust_path).expect("load Rust shared library");

        let c_driver = *c_library.get::<Driver>(b"driver\0").expect("load C driver");
        let rust_driver = *rust_library
            .get::<Driver>(b"driver\0")
            .expect("load Rust driver");
        let c_main = *c_library.get::<Main>(b"main\0").expect("load C main");
        let rust_main = *rust_library.get::<Main>(b"main\0").expect("load Rust main");

        valid_path_cases(c_driver, rust_driver, c_main, rust_main);
        generic_boundary_cases(c_driver, rust_driver, c_main, rust_main);
    }
}
