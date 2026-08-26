use libloading::Library;
use libloading::os::unix::{Library as UnixLibrary, RTLD_LOCAL, RTLD_NOW};
use std::arch::asm;
use std::ffi::{CString, c_char, c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::{FromRawFd, RawFd};
use std::path::{Path, PathBuf};

type PrintLine = unsafe extern "C" fn(*const c_char);
type NoArgs = unsafe extern "C" fn();
type Driver = unsafe extern "C" fn(c_int);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
}

struct Api {
    _library: Library,
    print_line: PrintLine,
    bad: NoArgs,
    good: NoArgs,
    driver: Driver,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { UnixLibrary::open(Some(path), RTLD_NOW | RTLD_LOCAL) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let library: Library = library.into();
        let print_line = unsafe { *library.get::<PrintLine>(b"printLine\0").unwrap() };
        let bad = unsafe { *library.get::<NoArgs>(b"bad\0").unwrap() };
        let good = unsafe { *library.get::<NoArgs>(b"good\0").unwrap() };
        let driver = unsafe { *library.get::<Driver>(b"driver\0").unwrap() };

        Self {
            _library: library,
            print_line,
            bad,
            good,
            driver,
        }
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libdriver.so")
}

fn rust_library_path() -> PathBuf {
    let test_binary = std::env::current_exe().expect("test executable path");
    test_binary
        .parent()
        .and_then(Path::parent)
        .expect("target profile directory")
        .join("libdriver.so")
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let mut pipe_fds: [RawFd; 2] = [-1, -1];

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0, "flush before capture");
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0, "create capture pipe");
    }

    let saved_stdout = unsafe { dup(1) };
    assert!(saved_stdout >= 0, "duplicate stdout");
    assert_eq!(unsafe { dup2(pipe_fds[1], 1) }, 1, "redirect stdout");
    assert_eq!(unsafe { close(pipe_fds[1]) }, 0, "close duplicate writer");

    call();

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0, "flush captured output");
        assert_eq!(dup2(saved_stdout, 1), 1, "restore stdout");
        assert_eq!(close(saved_stdout), 0, "close saved stdout");
    }

    let mut output = Vec::new();
    let mut reader = unsafe { File::from_raw_fd(pipe_fds[0]) };
    reader
        .read_to_end(&mut output)
        .expect("read captured output");
    output
}

fn compare_output(context: &str, c_call: impl FnOnce(), rust_call: impl FnOnce()) {
    let c_output = capture_stdout(c_call);
    let rust_output = capture_stdout(rust_call);
    assert_eq!(rust_output, c_output, "{context}");
}

fn compare_no_output(context: &str, c_call: impl FnOnce(), rust_call: impl FnOnce()) {
    let c_output = capture_stdout(c_call);
    let rust_output = capture_stdout(rust_call);
    assert_eq!(rust_output, c_output, "{context}");
    assert!(c_output.is_empty(), "{context}: expected no C output");
}

fn next_random(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

unsafe fn call_bad_with_stack_value(function: NoArgs, value: *const c_char) {
    unsafe {
        asm!(
            "mov qword ptr [rsp - 24], {value}",
            "call {function}",
            value = in(reg) value,
            function = in(reg) function,
            clobber_abi("C"),
        );
    }
}

unsafe fn call_driver_zero_with_stack_value(function: Driver, value: *const c_char) {
    unsafe {
        asm!(
            "mov qword ptr [rsp - 56], {value}",
            "mov rax, {function}",
            "xor edi, edi",
            "call rax",
            value = in(reg) value,
            function = in(reg) function,
            clobber_abi("C"),
        );
    }
}

#[test]
fn complete_ffi_surface_matches_c() {
    let c = unsafe { Api::load(&c_library_path()) };
    let rust = unsafe { Api::load(&rust_library_path()) };

    // CONFIGS #1: non-null strings, including empty, one-byte, and many-byte.
    let mut seed = 0x4d59_5df4_d0f3_3173;
    for case in 0..256 {
        let length = match case {
            0 => 0,
            1 => 1,
            _ => (next_random(&mut seed) % 512) as usize,
        };
        let bytes: Vec<u8> = (0..length)
            .map(|_| (next_random(&mut seed) % 255 + 1) as u8)
            .collect();
        let value = CString::new(bytes).unwrap();
        compare_output(
            "CONFIGS #1",
            || unsafe { (c.print_line)(value.as_ptr()) },
            || unsafe { (rust.print_line)(value.as_ptr()) },
        );
    }

    // CONFIGS #2 and ERRORS #1: null is accepted and produces no output.
    for _ in 0..64 {
        compare_no_output(
            "CONFIGS #2 / ERRORS #1",
            || unsafe { (c.print_line)(std::ptr::null()) },
            || unsafe { (rust.print_line)(std::ptr::null()) },
        );
    }

    // CONFIGS #3: the fixed good path.
    for _ in 0..64 {
        compare_output(
            "CONFIGS #3",
            || unsafe { (c.good)() },
            || unsafe { (rust.good)() },
        );
    }

    // CONFIGS #4: seed the otherwise uninitialized local stack slot.
    for _ in 0..64 {
        let length = (next_random(&mut seed) % 128) as usize;
        let bytes: Vec<u8> = (0..length)
            .map(|_| (next_random(&mut seed) % 255 + 1) as u8)
            .collect();
        let value = CString::new(bytes).unwrap();
        compare_output(
            "CONFIGS #4",
            || unsafe { call_bad_with_stack_value(c.bad, value.as_ptr()) },
            || unsafe { call_bad_with_stack_value(rust.bad, value.as_ptr()) },
        );
    }

    // CONFIGS #5: zero dispatches through bad with the same seeded stack state.
    for _ in 0..64 {
        let length = (next_random(&mut seed) % 128) as usize;
        let bytes: Vec<u8> = (0..length)
            .map(|_| (next_random(&mut seed) % 255 + 1) as u8)
            .collect();
        let value = CString::new(bytes).unwrap();
        compare_output(
            "CONFIGS #5",
            || unsafe { call_driver_zero_with_stack_value(c.driver, value.as_ptr()) },
            || unsafe { call_driver_zero_with_stack_value(rust.driver, value.as_ptr()) },
        );
    }

    // CONFIGS #6: every nonzero integer dispatches through good.
    let mut values = vec![c_int::MIN, -1, 1, c_int::MAX];
    while values.len() < 256 {
        let value = next_random(&mut seed) as c_int;
        if value != 0 {
            values.push(value);
        }
    }
    for value in values {
        compare_output(
            "CONFIGS #6",
            || unsafe { (c.driver)(value) },
            || unsafe { (rust.driver)(value) },
        );
    }
}
