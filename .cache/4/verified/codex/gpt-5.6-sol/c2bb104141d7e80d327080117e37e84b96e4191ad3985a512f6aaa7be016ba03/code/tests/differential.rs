use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void, CString};
use std::path::PathBuf;
use std::ptr;
use std::sync::Mutex;

type PrintLine = unsafe extern "C" fn(*const c_char);
type VoidFunction = unsafe extern "C" fn();
type MainFunction = unsafe extern "C" fn(c_int, *mut *mut c_char) -> c_int;

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
}

fn library_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn load_libraries() -> (Library, Library) {
    let c_path = library_path("c_src/build/libdriver_c.so");
    let rust_path = library_path("target/debug/libdriver.so");
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
            Library::new(c_path).expect("load C shared library"),
            Library::new(rust_path).expect("load Rust shared library"),
        )
    }
}

fn capture_stdout<T>(invoke: impl FnOnce() -> T) -> (T, Vec<u8>) {
    let _guard = STDOUT_LOCK.lock().expect("lock stdout capture");

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0, "flush stdout before capture");

        let mut pipe_fds = [-1; 2];
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0, "create stdout pipe");
        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0, "duplicate stdout");
        assert_eq!(dup2(pipe_fds[1], 1), 1, "redirect stdout");
        assert_eq!(close(pipe_fds[1]), 0, "close duplicate pipe writer");

        let result = invoke();

        assert_eq!(fflush(ptr::null_mut()), 0, "flush captured stdout");
        assert_eq!(dup2(saved_stdout, 1), 1, "restore stdout");
        assert_eq!(close(saved_stdout), 0, "close saved stdout");

        let mut output = Vec::new();
        let mut buffer = [0_u8; 4096];
        loop {
            let count = read(pipe_fds[0], buffer.as_mut_ptr().cast(), buffer.len());
            assert!(count >= 0, "read captured stdout");
            if count == 0 {
                break;
            }
            output.extend_from_slice(&buffer[..count as usize]);
        }
        assert_eq!(close(pipe_fds[0]), 0, "close pipe reader");

        (result, output)
    }
}

unsafe fn call_print_line(library: &Library, line: *const c_char) -> Vec<u8> {
    let function: Symbol<PrintLine> = library.get(b"printLine").expect("load printLine");
    capture_stdout(|| function(line)).1
}

unsafe fn call_void(library: &Library, symbol: &[u8]) -> Vec<u8> {
    let function: Symbol<VoidFunction> = library.get(symbol).expect("load void function");
    capture_stdout(|| function()).1
}

unsafe fn call_main(library: &Library, argc: c_int, argv: *mut *mut c_char) -> (c_int, Vec<u8>) {
    let function: Symbol<MainFunction> = library.get(b"main").expect("load main");
    capture_stdout(|| function(argc, argv))
}

fn next_random(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state
}

#[test]
fn symbols_are_loadable_from_both_shared_libraries() {
    let (c_library, rust_library) = load_libraries();

    unsafe {
        for symbol in [b"bad".as_slice(), b"good", b"main", b"printLine"] {
            c_library
                .get::<*const c_void>(symbol)
                .unwrap_or_else(|error| panic!("C symbol {symbol:?}: {error}"));
            rust_library
                .get::<*const c_void>(symbol)
                .unwrap_or_else(|error| panic!("Rust symbol {symbol:?}: {error}"));
        }
    }
}

#[test]
fn config_1_print_line_matches_for_randomized_byte_strings() {
    let (c_library, rust_library) = load_libraries();
    let mut random = 0x5eed_c0de_1234_5678;

    for case in 0..256 {
        let length = match case {
            0 => 0,
            1 => 1,
            _ => (next_random(&mut random) % 257) as usize,
        };
        let bytes: Vec<u8> = (0..length)
            .map(|_| ((next_random(&mut random) % 255) + 1) as u8)
            .collect();
        let line = CString::new(bytes).expect("random bytes contain no NUL");

        unsafe {
            let c_output = call_print_line(&c_library, line.as_ptr());
            let rust_output = call_print_line(&rust_library, line.as_ptr());
            assert_eq!(rust_output, c_output, "random case {case}");
        }
    }
}

#[test]
fn config_2_bad_matches_across_repeated_calls() {
    let (c_library, rust_library) = load_libraries();

    for case in 0..128 {
        unsafe {
            assert_eq!(
                call_void(&rust_library, b"bad"),
                call_void(&c_library, b"bad"),
                "repetition {case}"
            );
        }
    }
}

#[test]
fn config_3_good_composition_matches_across_repeated_calls() {
    let (c_library, rust_library) = load_libraries();

    for case in 0..128 {
        unsafe {
            assert_eq!(
                call_void(&rust_library, b"good"),
                call_void(&c_library, b"good"),
                "repetition {case}"
            );
        }
    }
}

#[test]
fn config_4_main_matches_for_zero_argc_and_null_argv() {
    let (c_library, rust_library) = load_libraries();

    for case in 0..64 {
        unsafe {
            assert_eq!(
                call_main(&rust_library, 0, ptr::null_mut()),
                call_main(&c_library, 0, ptr::null_mut()),
                "repetition {case}"
            );
        }
    }
}

#[test]
fn config_5_main_matches_for_randomized_ignored_arguments() {
    let (c_library, rust_library) = load_libraries();
    let argument = CString::new("driver").expect("static argument");
    let mut argv = [argument.as_ptr().cast_mut(), ptr::null_mut()];
    let mut random = 0xa11c_e5ed_f00d_beef;

    for case in 0..256 {
        let argc = match case {
            0 => -1,
            1 => 1,
            2 => c_int::MAX,
            _ => next_random(&mut random) as c_int | 1,
        };
        let argv_pointer = if case % 2 == 0 {
            ptr::null_mut()
        } else {
            argv.as_mut_ptr()
        };

        unsafe {
            assert_eq!(
                call_main(&rust_library, argc, argv_pointer),
                call_main(&c_library, argc, argv_pointer),
                "random case {case}, argc {argc}"
            );
        }
    }
}

#[test]
fn error_1_print_line_null_matches_and_writes_nothing() {
    let (c_library, rust_library) = load_libraries();

    for case in 0..128 {
        unsafe {
            let c_output = call_print_line(&c_library, ptr::null());
            let rust_output = call_print_line(&rust_library, ptr::null());
            assert_eq!(rust_output, c_output, "repetition {case}");
            assert!(c_output.is_empty(), "C wrote output for NULL");
        }
    }
}
