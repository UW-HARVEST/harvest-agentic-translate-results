use libloading::{Library, Symbol};
use std::ffi::{CString, c_char, c_int, c_void};
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

type NoArgFn = unsafe extern "C" fn();
type PrintLineFn = unsafe extern "C" fn(*const c_char);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

const STDOUT_FILENO: c_int = 1;
static CAPTURE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static CAPTURE_ID: AtomicU64 = AtomicU64::new(0);

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libdriver.so")
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libdriver.so")
}

fn open_libraries() -> (Library, Library) {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
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

    unsafe {
        (
            Library::new(&c_path)
                .unwrap_or_else(|error| panic!("load {}: {error}", c_path.display())),
            Library::new(&rust_path)
                .unwrap_or_else(|error| panic!("load {}: {error}", rust_path.display())),
        )
    }
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let _guard = CAPTURE_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let id = CAPTURE_ID.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "driver-differential-{}-{id}.out",
        std::process::id()
    ));
    let mut output = OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(&path)
        .unwrap_or_else(|error| panic!("create {}: {error}", path.display()));

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0, "flush before capture");
    }
    let saved_stdout = unsafe { dup(STDOUT_FILENO) };
    assert!(saved_stdout >= 0, "dup(stdout) failed");
    assert_eq!(
        unsafe { dup2(output.as_raw_fd(), STDOUT_FILENO) },
        STDOUT_FILENO,
        "redirect stdout failed"
    );

    call();

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0, "flush captured output");
    }
    assert_eq!(
        unsafe { dup2(saved_stdout, STDOUT_FILENO) },
        STDOUT_FILENO,
        "restore stdout failed"
    );
    assert_eq!(unsafe { close(saved_stdout) }, 0, "close saved stdout");

    output.seek(SeekFrom::Start(0)).expect("rewind capture");
    let mut bytes = Vec::new();
    output.read_to_end(&mut bytes).expect("read capture");
    drop(output);
    std::fs::remove_file(&path)
        .unwrap_or_else(|error| panic!("remove {}: {error}", path.display()));
    bytes
}

fn compare_no_arg_symbol(symbol: &[u8], repetitions: usize) {
    let (c_library, rust_library) = open_libraries();
    let c_function: Symbol<NoArgFn> = unsafe {
        c_library
            .get(symbol)
            .unwrap_or_else(|error| panic!("C symbol {:?}: {error}", symbol))
    };
    let rust_function: Symbol<NoArgFn> = unsafe {
        rust_library
            .get(symbol)
            .unwrap_or_else(|error| panic!("Rust symbol {:?}: {error}", symbol))
    };

    for iteration in 0..repetitions {
        let c_output = capture_stdout(|| unsafe { c_function() });
        let rust_output = capture_stdout(|| unsafe { rust_function() });
        assert_eq!(
            rust_output, c_output,
            "symbol {:?}, iteration {iteration}",
            symbol
        );
    }
}

fn next_random(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

#[test]
fn all_c_api_symbols_are_loadable_from_both_libraries() {
    let (c_library, rust_library) = open_libraries();
    for symbol in [b"bad\0".as_slice(), b"driver\0", b"good\0", b"printLine\0"] {
        unsafe {
            c_library
                .get::<*const c_void>(symbol)
                .unwrap_or_else(|error| panic!("C symbol {:?}: {error}", symbol));
            rust_library
                .get::<*const c_void>(symbol)
                .unwrap_or_else(|error| panic!("Rust symbol {:?}: {error}", symbol));
        }
    }
}

#[test]
fn config_1_print_line_non_null_randomized() {
    let (c_library, rust_library) = open_libraries();
    let c_function: Symbol<PrintLineFn> =
        unsafe { c_library.get(b"printLine\0").expect("C printLine") };
    let rust_function: Symbol<PrintLineFn> =
        unsafe { rust_library.get(b"printLine\0").expect("Rust printLine") };
    let mut random_state = 0x4d59_5df4_d0f3_3173_u64;

    for case in 0..256 {
        let length = match case {
            0 => 0,
            255 => 1024 * 1024,
            _ => (next_random(&mut random_state) % 513) as usize,
        };
        let bytes: Vec<u8> = (0..length)
            .map(|_| ((next_random(&mut random_state) % 255) + 1) as u8)
            .collect();
        let line = CString::new(bytes).expect("generated string has no NUL");

        let c_output = capture_stdout(|| unsafe { c_function(line.as_ptr()) });
        let rust_output = capture_stdout(|| unsafe { rust_function(line.as_ptr()) });
        assert_eq!(rust_output, c_output, "randomized printLine case {case}");
    }
}

#[test]
fn config_2_bad_direct() {
    compare_no_arg_symbol(b"bad\0", 64);
}

#[test]
fn config_3_good_composed() {
    compare_no_arg_symbol(b"good\0", 64);
}

#[test]
fn config_4_driver_full_pipeline() {
    compare_no_arg_symbol(b"driver\0", 64);
}

#[test]
fn error_1_print_line_null_is_ignored() {
    let (c_library, rust_library) = open_libraries();
    let c_function: Symbol<PrintLineFn> =
        unsafe { c_library.get(b"printLine\0").expect("C printLine") };
    let rust_function: Symbol<PrintLineFn> =
        unsafe { rust_library.get(b"printLine\0").expect("Rust printLine") };

    let c_output = capture_stdout(|| unsafe { c_function(std::ptr::null()) });
    let rust_output = capture_stdout(|| unsafe { rust_function(std::ptr::null()) });
    assert_eq!(rust_output, c_output);
    assert!(c_output.is_empty(), "C null call unexpectedly wrote output");
}
