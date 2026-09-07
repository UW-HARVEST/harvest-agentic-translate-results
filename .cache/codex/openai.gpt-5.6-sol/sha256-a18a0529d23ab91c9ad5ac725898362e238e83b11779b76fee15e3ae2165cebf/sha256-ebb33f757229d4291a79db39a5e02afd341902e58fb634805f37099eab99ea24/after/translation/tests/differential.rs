use libloading::Library;
use std::ffi::{CString, c_char, c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::Mutex;

type PrintLine = unsafe extern "C" fn(*const c_char);
type PrintIntLine = unsafe extern "C" fn(c_int);
type NoArgs = unsafe extern "C" fn();

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
}

const STDOUT_FILENO: c_int = 1;
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

struct Api {
    _library: Library,
    print_line: PrintLine,
    print_int_line: PrintIntLine,
    bad: NoArgs,
    good: NoArgs,
    driver: NoArgs,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));

        let print_line = unsafe { *library.get::<PrintLine>(b"printLine\0").unwrap() };
        let print_int_line = unsafe { *library.get::<PrintIntLine>(b"printIntLine\0").unwrap() };
        let bad = unsafe { *library.get::<NoArgs>(b"bad\0").unwrap() };
        let good = unsafe { *library.get::<NoArgs>(b"good\0").unwrap() };
        let driver = unsafe { *library.get::<NoArgs>(b"driver\0").unwrap() };

        Self {
            _library: library,
            print_line,
            print_int_line,
            bad,
            good,
            driver,
        }
    }
}

fn library_paths() -> (PathBuf, PathBuf) {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    (
        crate_root.join("../c_src/build/libdriver.so"),
        crate_root.join("target/release/libdriver.so"),
    )
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let _lock = STDOUT_LOCK.lock().unwrap();
    let mut pipe_fds = [-1; 2];

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0, "fflush before capture failed");
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0, "pipe failed");

        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0, "dup failed");
        assert_eq!(
            dup2(pipe_fds[1], STDOUT_FILENO),
            STDOUT_FILENO,
            "dup2 to capture stdout failed"
        );
        assert_eq!(close(pipe_fds[1]), 0, "close pipe writer failed");

        call();

        assert_eq!(fflush(ptr::null_mut()), 0, "fflush after call failed");
        assert_eq!(
            dup2(saved_stdout, STDOUT_FILENO),
            STDOUT_FILENO,
            "dup2 to restore stdout failed"
        );
        assert_eq!(close(saved_stdout), 0, "close saved stdout failed");

        let mut output = Vec::new();
        let mut reader = File::from_raw_fd(pipe_fds[0]);
        reader.read_to_end(&mut output).unwrap();
        output
    }
}

fn next_random(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

#[test]
fn all_configuration_and_error_rows_match() {
    let (c_path, rust_path) = library_paths();
    assert!(
        c_path.is_file(),
        "missing C shared library: {}",
        c_path.display()
    );
    assert!(
        rust_path.is_file(),
        "missing Rust shared library: {}",
        rust_path.display()
    );

    let c_api = unsafe { Api::load(&c_path) };
    let rust_api = unsafe { Api::load(&rust_path) };

    // CONFIGS.md row 1: valid non-null C strings, including empty strings.
    let mut strings = vec![Vec::new()];
    let mut state = 0x4d59_5df4_d0f3_3173_u64;
    for index in 0..512 {
        let length = if index < 256 {
            index
        } else {
            (next_random(&mut state) as usize) % 1025
        };
        let mut bytes = Vec::with_capacity(length);
        for _ in 0..length {
            let byte = ((next_random(&mut state) % 255) + 1) as u8;
            bytes.push(byte);
        }
        strings.push(bytes);
    }

    for bytes in strings {
        let string = CString::new(bytes).unwrap();
        let c_output = capture_stdout(|| unsafe { (c_api.print_line)(string.as_ptr()) });
        let rust_output = capture_stdout(|| unsafe { (rust_api.print_line)(string.as_ptr()) });
        assert_eq!(rust_output, c_output, "printLine output differed");
    }

    // CONFIGS.md row 2: randomized C ints plus exact boundaries.
    let mut integers = vec![c_int::MIN, -1, 0, 1, c_int::MAX];
    for _ in 0..4096 {
        integers.push(next_random(&mut state) as u32 as c_int);
    }

    for value in integers {
        let c_output = capture_stdout(|| unsafe { (c_api.print_int_line)(value) });
        let rust_output = capture_stdout(|| unsafe { (rust_api.print_int_line)(value) });
        assert_eq!(
            rust_output, c_output,
            "printIntLine output differed for {value}"
        );
    }

    // CONFIGS.md rows 3-5: complete low-level and composed operations.
    for (name, c_function, rust_function) in [
        ("bad", c_api.bad, rust_api.bad),
        ("good", c_api.good, rust_api.good),
        ("driver", c_api.driver, rust_api.driver),
    ] {
        let c_output = capture_stdout(|| unsafe { c_function() });
        let rust_output = capture_stdout(|| unsafe { rust_function() });
        assert_eq!(rust_output, c_output, "{name} output differed");
    }

    // ERRORS.md row 1 and the API's only pointer boundary: NULL is ignored.
    let c_output = capture_stdout(|| unsafe { (c_api.print_line)(ptr::null()) });
    let rust_output = capture_stdout(|| unsafe { (rust_api.print_line)(ptr::null()) });
    assert_eq!(rust_output, c_output, "printLine(NULL) differed");
    assert!(
        c_output.is_empty(),
        "C printLine(NULL) unexpectedly wrote output"
    );
}
