use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

type NoArgFn = unsafe extern "C" fn();
type DriverFn = unsafe extern "C" fn(c_int);
type PrintLineFn = unsafe extern "C" fn(*const c_char);

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipefd: *mut c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
}

struct LoadedDrivers {
    c: Library,
    rust: Library,
}

impl LoadedDrivers {
    fn open() -> Self {
        let c_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libdriver.so");
        let rust_path = rust_library_path();

        assert!(
            c_path.is_file(),
            "C shared library missing: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "Rust shared library missing: {}",
            rust_path.display()
        );

        // SAFETY: Both paths are build artifacts controlled by this test.
        let c = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display()));
        // SAFETY: Both paths are build artifacts controlled by this test.
        let rust = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display()));

        Self { c, rust }
    }
}

fn rust_library_path() -> PathBuf {
    std::env::var_os("DRIVER_RUST_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libdriver.so")
        })
}

fn assert_success(result: c_int, operation: &str) {
    assert_ne!(result, -1, "{operation} failed");
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().expect("stdout capture lock poisoned");
    let mut pipe_fds = [-1; 2];

    // SAFETY: The descriptors and buffers are valid for each libc operation.
    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0, "pre-capture fflush failed");
        assert_success(pipe(pipe_fds.as_mut_ptr()), "pipe");

        let saved_stdout = dup(1);
        assert_success(saved_stdout, "dup");
        assert_success(dup2(pipe_fds[1], 1), "redirect stdout");
        assert_success(close(pipe_fds[1]), "close pipe writer");

        call();

        assert_eq!(fflush(std::ptr::null_mut()), 0, "post-call fflush failed");
        assert_success(dup2(saved_stdout, 1), "restore stdout");
        assert_success(close(saved_stdout), "close saved stdout");

        let mut output = Vec::new();
        let mut chunk = [0_u8; 4096];
        loop {
            let count = read(
                pipe_fds[0],
                chunk.as_mut_ptr().cast::<c_void>(),
                chunk.len(),
            );
            assert!(count >= 0, "read failed");
            if count == 0 {
                break;
            }
            output.extend_from_slice(&chunk[..count as usize]);
        }
        assert_success(close(pipe_fds[0]), "close pipe reader");
        output
    }
}

unsafe fn call_no_arg(library: &Library, symbol: &[u8]) -> Vec<u8> {
    // SAFETY: The symbol signatures come directly from the C definitions.
    let function: Symbol<NoArgFn> = unsafe { library.get(symbol) }
        .unwrap_or_else(|error| panic!("missing {}: {error}", String::from_utf8_lossy(symbol)));
    capture_stdout(|| {
        // SAFETY: This symbol accepts no arguments.
        unsafe { function() }
    })
}

unsafe fn call_driver(library: &Library, use_good: c_int) -> Vec<u8> {
    // SAFETY: The symbol signature comes directly from driver.c.
    let function: Symbol<DriverFn> =
        unsafe { library.get(b"driver") }.expect("missing driver symbol");
    capture_stdout(|| {
        // SAFETY: Every c_int value is valid for driver.
        unsafe { function(use_good) }
    })
}

unsafe fn call_print_line(library: &Library, line: *const c_char) -> Vec<u8> {
    // SAFETY: The symbol signature comes directly from driver.c.
    let function: Symbol<PrintLineFn> =
        unsafe { library.get(b"printLine") }.expect("missing printLine symbol");
    capture_stdout(|| {
        // SAFETY: Callers provide either null or a valid NUL-terminated string.
        unsafe { function(line) }
    })
}

fn next_random(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

fn random_c_string(state: &mut u64, length: usize) -> Vec<u8> {
    let mut value = Vec::with_capacity(length + 1);
    for _ in 0..length {
        value.push((next_random(state) % 255 + 1) as u8);
    }
    value.push(0);
    value
}

fn valid_configuration_surface_matches() {
    let libraries = LoadedDrivers::open();
    let mut state = 0x5eed_1234_cafe_babe_u64;

    let mut lengths = vec![0, 1, 2, 31, 255, 1024];
    lengths.extend((0..256).map(|_| (next_random(&mut state) % 2048) as usize));
    for length in lengths {
        let value = random_c_string(&mut state, length);
        // SAFETY: value is NUL-terminated and remains alive during both calls.
        let c_output = unsafe { call_print_line(&libraries.c, value.as_ptr().cast::<c_char>()) };
        // SAFETY: value is NUL-terminated and remains alive during both calls.
        let rust_output =
            unsafe { call_print_line(&libraries.rust, value.as_ptr().cast::<c_char>()) };
        assert_eq!(
            rust_output, c_output,
            "printLine diverged at length {length}"
        );
    }

    // SAFETY: These no-argument signatures come directly from driver.c.
    let c_bad = unsafe { call_no_arg(&libraries.c, b"bad") };
    // SAFETY: These no-argument signatures come directly from driver.c.
    let rust_bad = unsafe { call_no_arg(&libraries.rust, b"bad") };
    assert_eq!(rust_bad, c_bad, "bad diverged");

    // SAFETY: These no-argument signatures come directly from driver.c.
    let c_good = unsafe { call_no_arg(&libraries.c, b"good") };
    // SAFETY: These no-argument signatures come directly from driver.c.
    let rust_good = unsafe { call_no_arg(&libraries.rust, b"good") };
    assert_eq!(rust_good, c_good, "good diverged");

    // SAFETY: Zero is a valid c_int.
    let c_false = unsafe { call_driver(&libraries.c, 0) };
    // SAFETY: Zero is a valid c_int.
    let rust_false = unsafe { call_driver(&libraries.rust, 0) };
    assert_eq!(rust_false, c_false, "driver false branch diverged");

    let mut truthy_values = vec![c_int::MIN, -1, 1, c_int::MAX];
    for _ in 0..512 {
        let value = next_random(&mut state) as c_int;
        truthy_values.push(if value == 0 { 1 } else { value });
    }
    for use_good in truthy_values {
        // SAFETY: Every c_int value is valid for driver.
        let c_output = unsafe { call_driver(&libraries.c, use_good) };
        // SAFETY: Every c_int value is valid for driver.
        let rust_output = unsafe { call_driver(&libraries.rust, use_good) };
        assert_eq!(
            rust_output, c_output,
            "driver true branch diverged for {use_good}"
        );
    }
}

fn error_surface_matches() {
    let libraries = LoadedDrivers::open();

    // SAFETY: printLine explicitly accepts and rejects null without dereferencing it.
    let c_output = unsafe { call_print_line(&libraries.c, std::ptr::null()) };
    // SAFETY: The Rust export must implement the same null-pointer contract.
    let rust_output = unsafe { call_print_line(&libraries.rust, std::ptr::null()) };

    assert!(
        c_output.is_empty(),
        "C printLine(NULL) unexpectedly wrote output"
    );
    assert_eq!(rust_output, c_output, "printLine(NULL) diverged");
}

#[test]
fn complete_differential_surface_matches() {
    valid_configuration_surface_matches();
    error_surface_matches();
}
