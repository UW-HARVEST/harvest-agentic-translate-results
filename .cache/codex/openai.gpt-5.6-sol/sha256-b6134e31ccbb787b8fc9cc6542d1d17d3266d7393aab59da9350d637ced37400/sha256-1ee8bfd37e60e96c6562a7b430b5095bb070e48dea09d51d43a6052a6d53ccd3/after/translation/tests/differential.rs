use libloading::Library;
use std::env;
use std::ffi::{CString, c_char, c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::Mutex;

type Foo = unsafe extern "C" fn(*const c_char, c_char) -> c_int;
type Driver = unsafe extern "C" fn(*const c_char);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipefd: *mut c_int) -> c_int;
}

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

fn c_library_path() -> PathBuf {
    env::var_os("C_DRIVER_SO").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libdriver.so"),
        PathBuf::from,
    )
}

fn rust_library_path() -> PathBuf {
    env::var_os("RUST_DRIVER_SO").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libdriver.so"),
        PathBuf::from,
    )
}

fn load_library(path: &Path) -> Library {
    assert!(
        path.is_file(),
        "shared library does not exist: {}",
        path.display()
    );
    unsafe { Library::new(path) }
        .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()))
}

fn next_u64(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

fn random_nonzero_byte(state: &mut u64) -> u8 {
    ((next_u64(state) % 255) + 1) as u8
}

fn shuffle(bytes: &mut [u8], state: &mut u64) {
    for index in (1..bytes.len()).rev() {
        let other = (next_u64(state) as usize) % (index + 1);
        bytes.swap(index, other);
    }
}

fn foo_input(state: &mut u64, target: u8, count: usize) -> CString {
    let filler_len = (next_u64(state) % 96) as usize;
    let mut bytes = Vec::with_capacity(filler_len + count);

    for _ in 0..filler_len {
        let mut byte = random_nonzero_byte(state);
        while byte == target {
            byte = random_nonzero_byte(state);
        }
        bytes.push(byte);
    }
    bytes.extend(std::iter::repeat_n(target, count));
    shuffle(&mut bytes, state);
    CString::new(bytes).unwrap()
}

fn count_for_class(class: usize, state: &mut u64) -> usize {
    match class {
        0 => 0,
        1 => 1,
        2 => 2 + (next_u64(state) % 7) as usize,
        _ => unreachable!(),
    }
}

fn driver_input(state: &mut u64, a_class: usize, x_class: usize) -> CString {
    let a_count = count_for_class(a_class, state);
    let x_count = count_for_class(x_class, state);
    let filler_len = (next_u64(state) % 96) as usize;
    let mut bytes = Vec::with_capacity(a_count + x_count + filler_len);

    bytes.extend(std::iter::repeat_n(b'A', a_count));
    bytes.extend(std::iter::repeat_n(b'x', x_count));
    for _ in 0..filler_len {
        let mut byte = random_nonzero_byte(state);
        while byte == b'A' || byte == b'x' {
            byte = random_nonzero_byte(state);
        }
        bytes.push(byte);
    }
    shuffle(&mut bytes, state);
    CString::new(bytes).unwrap()
}

unsafe fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().unwrap();
    let mut pipe_fds = [-1; 2];

    assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0);
    let saved_stdout = unsafe { dup(1) };
    assert!(saved_stdout >= 0);
    assert_eq!(unsafe { pipe(pipe_fds.as_mut_ptr()) }, 0);
    assert_eq!(unsafe { dup2(pipe_fds[1], 1) }, 1);
    assert_eq!(unsafe { close(pipe_fds[1]) }, 0);

    call();

    assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0);
    assert_eq!(unsafe { dup2(saved_stdout, 1) }, 1);
    assert_eq!(unsafe { close(saved_stdout) }, 0);

    let mut output = Vec::new();
    let mut reader = unsafe { File::from_raw_fd(pipe_fds[0]) };
    reader.read_to_end(&mut output).unwrap();
    output
}

#[test]
fn foo_matches_for_every_match_count_shape() {
    let c_lib = load_library(&c_library_path());
    let rust_lib = load_library(&rust_library_path());
    let c_foo = unsafe { c_lib.get::<Foo>(b"foo\0") }.unwrap();
    let rust_foo = unsafe { rust_lib.get::<Foo>(b"foo\0") }.unwrap();
    let mut state = 0x8d26_7a4b_19c3_e501;

    for count_class in 0..3 {
        for case in 0..512 {
            let target = random_nonzero_byte(&mut state);
            let count = count_for_class(count_class, &mut state);
            let input = foo_input(&mut state, target, count);
            let c_result = unsafe { c_foo(input.as_ptr(), target as c_char) };
            let rust_result = unsafe { rust_foo(input.as_ptr(), target as c_char) };
            assert_eq!(
                rust_result,
                c_result,
                "foo mismatch for class={count_class}, case={case}, target={target:#04x}, input={:?}",
                input.as_bytes()
            );
            assert_eq!(c_result, count as c_int);
        }
    }
}

#[test]
fn driver_matches_for_every_a_x_count_cross_product() {
    let c_lib = load_library(&c_library_path());
    let rust_lib = load_library(&rust_library_path());
    let c_driver = unsafe { c_lib.get::<Driver>(b"driver\0") }.unwrap();
    let rust_driver = unsafe { rust_lib.get::<Driver>(b"driver\0") }.unwrap();
    let mut state = 0xb4e1_0f92_c675_3ad8;

    for a_class in 0..3 {
        for x_class in 0..3 {
            for case in 0..256 {
                let input = driver_input(&mut state, a_class, x_class);
                let c_output = unsafe { capture_stdout(|| c_driver(input.as_ptr())) };
                let rust_output = unsafe { capture_stdout(|| rust_driver(input.as_ptr())) };
                assert_eq!(
                    rust_output,
                    c_output,
                    "driver mismatch for A class={a_class}, x class={x_class}, case={case}, input={:?}",
                    input.as_bytes()
                );
            }
        }
    }
}

#[test]
fn empty_input_boundaries_match() {
    let c_lib = load_library(&c_library_path());
    let rust_lib = load_library(&rust_library_path());
    let c_foo = unsafe { c_lib.get::<Foo>(b"foo\0") }.unwrap();
    let rust_foo = unsafe { rust_lib.get::<Foo>(b"foo\0") }.unwrap();
    let c_driver = unsafe { c_lib.get::<Driver>(b"driver\0") }.unwrap();
    let rust_driver = unsafe { rust_lib.get::<Driver>(b"driver\0") }.unwrap();
    let input = CString::new(Vec::<u8>::new()).unwrap();

    assert_eq!(unsafe { c_foo(input.as_ptr(), b'Q' as c_char) }, 0);
    assert_eq!(
        unsafe { rust_foo(input.as_ptr(), b'Q' as c_char) },
        unsafe { c_foo(input.as_ptr(), b'Q' as c_char) }
    );

    let c_output = unsafe { capture_stdout(|| c_driver(input.as_ptr())) };
    let rust_output = unsafe { capture_stdout(|| rust_driver(input.as_ptr())) };
    assert_eq!(rust_output, c_output);
    assert_eq!(c_output, b"A: 0\nx: 0\n");
}

fn run_null_probe(library_kind: &str, symbol: &str) -> std::process::ExitStatus {
    Command::new(env::current_exe().unwrap())
        .args(["--ignored", "--exact", "null_pointer_probe"])
        .env("NULL_PROBE_LIBRARY", library_kind)
        .env("NULL_PROBE_SYMBOL", symbol)
        .status()
        .unwrap()
}

#[test]
fn null_pointer_process_failures_match() {
    for symbol in ["foo", "driver"] {
        let c_status = run_null_probe("c", symbol);
        let rust_status = run_null_probe("rust", symbol);
        assert!(
            !c_status.success(),
            "C {symbol}(NULL) unexpectedly succeeded"
        );
        assert!(
            !rust_status.success(),
            "Rust {symbol}(NULL) unexpectedly succeeded"
        );
        assert_eq!(
            rust_status.signal(),
            c_status.signal(),
            "different terminating signals for {symbol}(NULL): C={c_status:?}, Rust={rust_status:?}"
        );
    }
}

#[test]
#[ignore = "invoked in an isolated subprocess by null_pointer_process_failures"]
fn null_pointer_probe() {
    let library_kind = env::var("NULL_PROBE_LIBRARY").unwrap();
    let symbol = env::var("NULL_PROBE_SYMBOL").unwrap();
    let path = match library_kind.as_str() {
        "c" => c_library_path(),
        "rust" => rust_library_path(),
        _ => panic!("unknown library kind: {library_kind}"),
    };
    let library = load_library(&path);

    match symbol.as_str() {
        "foo" => {
            let foo = unsafe { library.get::<Foo>(b"foo\0") }.unwrap();
            let _ = unsafe { foo(ptr::null(), b'A' as c_char) };
        }
        "driver" => {
            let driver = unsafe { library.get::<Driver>(b"driver\0") }.unwrap();
            unsafe { driver(ptr::null()) };
        }
        _ => panic!("unknown symbol: {symbol}"),
    }
}

#[test]
fn both_libraries_export_the_complete_c_symbol_surface() {
    for path in [c_library_path(), rust_library_path()] {
        let library = load_library(&path);
        unsafe {
            library.get::<Foo>(b"foo\0").unwrap();
            library.get::<Driver>(b"driver\0").unwrap();
        }
    }
}
