use libloading::{Library, Symbol};
use std::env;
use std::ffi::{CString, c_char, c_int, c_void};
use std::fs::{OpenOptions, remove_file};
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

type VoidFn = unsafe extern "C" fn();
type PrintLineFn = unsafe extern "C" fn(*const c_char);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
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

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let _guard = CAPTURE_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("stdout capture lock");
    let path = env::temp_dir().join(format!(
        "driver-differential-{}-{}",
        std::process::id(),
        CAPTURE_ID.fetch_add(1, Ordering::Relaxed)
    ));
    let mut capture = OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(&path)
        .expect("create stdout capture");

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0, "flush stdout");
    }
    let saved_stdout = unsafe { dup(STDOUT_FILENO) };
    assert!(saved_stdout >= 0, "duplicate stdout");
    assert_eq!(
        unsafe { dup2(capture.as_raw_fd(), STDOUT_FILENO) },
        STDOUT_FILENO,
        "redirect stdout"
    );

    call();

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0, "flush captured stdout");
        assert_eq!(
            dup2(saved_stdout, STDOUT_FILENO),
            STDOUT_FILENO,
            "restore stdout"
        );
        assert_eq!(close(saved_stdout), 0, "close saved stdout");
    }

    capture.seek(SeekFrom::Start(0)).expect("rewind capture");
    let mut output = Vec::new();
    capture.read_to_end(&mut output).expect("read capture");
    drop(capture);
    remove_file(path).expect("remove stdout capture");
    output
}

fn call_void(function: &Symbol<VoidFn>) -> Vec<u8> {
    capture_stdout(|| unsafe { function() })
}

fn call_print_line(function: &Symbol<PrintLineFn>, line: *const c_char) -> Vec<u8> {
    capture_stdout(|| unsafe { function(line) })
}

fn next_random(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

fn randomized_strings() -> Vec<CString> {
    let mut strings = vec![
        CString::new(Vec::<u8>::new()).unwrap(),
        CString::new(b"x".as_slice()).unwrap(),
        CString::new(vec![b'z'; 65_536]).unwrap(),
    ];
    let mut state = 0x4d59_5df4_d0f3_3173_u64;

    for iteration in 0..256 {
        let length = match iteration {
            0 => 0,
            1 => 1,
            _ => (next_random(&mut state) % 4097) as usize,
        };
        let bytes = (0..length)
            .map(|_| {
                let value = next_random(&mut state) as u8;
                (value % 255) + 1
            })
            .collect::<Vec<_>>();
        strings.push(CString::new(bytes).unwrap());
    }
    strings
}

fn load_libraries() -> (Library, Library) {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(c_path.is_file(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.is_file(),
        "missing Rust library: {}",
        rust_path.display()
    );

    let c_library = unsafe { Library::new(&c_path) }.expect("load C library");
    let rust_library = unsafe { Library::new(&rust_path) }.expect("load Rust library");
    (c_library, rust_library)
}

fn assert_void_entry_point_matches(name: &[u8]) {
    let (c_library, rust_library) = load_libraries();
    unsafe {
        let c_function: Symbol<VoidFn> = c_library.get(name).expect("C void entry point");
        let rust_function: Symbol<VoidFn> = rust_library.get(name).expect("Rust void entry point");
        assert_eq!(
            call_void(&c_function),
            call_void(&rust_function),
            "{} output diverged",
            String::from_utf8_lossy(&name[..name.len() - 1])
        );
    }
}

#[test]
fn config_1_print_line_matches_for_randomized_valid_inputs() {
    let (c_library, rust_library) = load_libraries();
    unsafe {
        let c_print_line: Symbol<PrintLineFn> = c_library.get(b"printLine\0").expect("C printLine");
        let rust_print_line: Symbol<PrintLineFn> =
            rust_library.get(b"printLine\0").expect("Rust printLine");

        for input in randomized_strings() {
            assert_eq!(
                call_print_line(&c_print_line, input.as_ptr()),
                call_print_line(&rust_print_line, input.as_ptr()),
                "printLine diverged for {} input bytes",
                input.as_bytes().len()
            );
        }
    }
}

#[test]
fn error_1_print_line_null_guard_matches() {
    let (c_library, rust_library) = load_libraries();
    unsafe {
        let c_print_line: Symbol<PrintLineFn> = c_library.get(b"printLine\0").expect("C printLine");
        let rust_print_line: Symbol<PrintLineFn> =
            rust_library.get(b"printLine\0").expect("Rust printLine");
        assert_eq!(
            call_print_line(&c_print_line, std::ptr::null()),
            call_print_line(&rust_print_line, std::ptr::null()),
            "printLine NULL handling diverged"
        );
    }
}

#[test]
fn config_2_bad_matches() {
    assert_void_entry_point_matches(b"bad\0");
}

#[test]
fn config_3_good_matches() {
    assert_void_entry_point_matches(b"good\0");
}

#[test]
fn config_4_driver_matches() {
    assert_void_entry_point_matches(b"driver\0");
}
