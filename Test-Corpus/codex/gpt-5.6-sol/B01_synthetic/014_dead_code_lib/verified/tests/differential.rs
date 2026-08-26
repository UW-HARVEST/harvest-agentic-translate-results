use libloading::{Library, Symbol};
use std::ffi::{CString, c_char, c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::Mutex;

type VoidFn = unsafe extern "C" fn();
type PrintLineFn = unsafe extern "C" fn(*const c_char);

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipefd: *mut c_int) -> c_int;
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libdriver.so")
}

fn rust_library_path() -> PathBuf {
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("target"));
    target.join("release").join("libdriver.so")
}

fn load_libraries() -> (Library, Library) {
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
            Library::new(c_path).expect("load C shared library"),
            Library::new(rust_path).expect("load Rust shared library"),
        )
    }
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().expect("stdout lock poisoned");
    let mut pipe_fds = [-1; 2];

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0, "flush stdout before capture");
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0, "create stdout pipe");

        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0, "duplicate stdout");
        assert_eq!(dup2(pipe_fds[1], 1), 1, "redirect stdout");
        assert_eq!(close(pipe_fds[1]), 0, "close duplicated pipe writer");

        call();

        assert_eq!(fflush(ptr::null_mut()), 0, "flush captured stdout");
        assert_eq!(dup2(saved_stdout, 1), 1, "restore stdout");
        assert_eq!(close(saved_stdout), 0, "close saved stdout");
    }

    let mut output = Vec::new();
    unsafe {
        File::from_raw_fd(pipe_fds[0])
            .read_to_end(&mut output)
            .expect("read captured stdout");
    }
    output
}

fn call_void(library: &Library, symbol: &[u8], repetitions: usize) -> Vec<u8> {
    capture_stdout(|| unsafe {
        let function: Symbol<VoidFn> = library.get(symbol).expect("load void symbol");
        for _ in 0..repetitions {
            function();
        }
    })
}

fn call_print_line(library: &Library, line: *const c_char) -> Vec<u8> {
    capture_stdout(|| unsafe {
        let function: Symbol<PrintLineFn> = library.get(b"printLine").expect("load printLine");
        function(line);
    })
}

fn next_random(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state
}

#[test]
fn config_1_print_line_matches_for_randomized_strings() {
    let (c, rust) = load_libraries();
    let empty = CString::new(Vec::<u8>::new()).unwrap();
    assert_eq!(
        call_print_line(&c, empty.as_ptr()),
        call_print_line(&rust, empty.as_ptr())
    );

    let mut state = 0x5eed_c0de_d15c_a11u64;
    for case in 0..256 {
        let len = (next_random(&mut state) % 512) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| (next_random(&mut state) % 255 + 1) as u8)
            .collect();
        let line = CString::new(bytes).unwrap();
        assert_eq!(
            call_print_line(&c, line.as_ptr()),
            call_print_line(&rust, line.as_ptr()),
            "output mismatch for randomized case {case}"
        );
    }
}

#[test]
fn config_2_bad_matches() {
    let (c, rust) = load_libraries();
    assert_eq!(call_void(&c, b"bad", 64), call_void(&rust, b"bad", 64));
}

#[test]
fn config_3_good_matches() {
    let (c, rust) = load_libraries();
    assert_eq!(call_void(&c, b"good", 64), call_void(&rust, b"good", 64));
}

#[test]
fn config_4_driver_matches() {
    let (c, rust) = load_libraries();
    assert_eq!(
        call_void(&c, b"driver", 64),
        call_void(&rust, b"driver", 64)
    );
}

#[test]
fn error_1_print_line_null_matches() {
    let (c, rust) = load_libraries();
    assert_eq!(
        call_print_line(&c, ptr::null()),
        call_print_line(&rust, ptr::null())
    );
    assert!(call_print_line(&c, ptr::null()).is_empty());
    assert!(call_print_line(&rust, ptr::null()).is_empty());
}

#[test]
fn all_c_symbols_are_loadable_from_rust_library() {
    let (_, rust) = load_libraries();
    unsafe {
        let _: Symbol<VoidFn> = rust.get(b"bad").expect("Rust export bad");
        let _: Symbol<VoidFn> = rust.get(b"driver").expect("Rust export driver");
        let _: Symbol<VoidFn> = rust.get(b"good").expect("Rust export good");
        let _: Symbol<PrintLineFn> = rust.get(b"printLine").expect("Rust export printLine");
    }
}
