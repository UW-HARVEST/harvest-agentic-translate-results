use libloading::Library;
use std::ffi::{c_char, c_int, c_void};
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::FromRawFd;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

type PrintHexCharLine = unsafe extern "C" fn(c_char);
type Main = unsafe extern "C" fn() -> c_int;

static STDIO_LOCK: Mutex<()> = Mutex::new(());

extern "C" {
    static mut stdin: *mut c_void;

    fn __fpurge(stream: *mut c_void);
    fn clearerr(stream: *mut c_void);
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
}

struct Libraries {
    c: Library,
    rust: Library,
}

impl Libraries {
    unsafe fn load() -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest.join("c_src/build/libdriver_c.so");
        let profile_dir = std::env::current_exe()
            .expect("test executable path")
            .parent()
            .expect("deps directory")
            .parent()
            .expect("target profile directory")
            .to_owned();
        let rust_path = profile_dir.join("libdriver.so");

        assert!(
            c_path.is_file(),
            "missing C shared library {}; build it before testing",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "missing Rust shared library {}",
            rust_path.display()
        );

        Self {
            c: Library::new(c_path).expect("load C shared library"),
            rust: Library::new(rust_path).expect("load Rust shared library"),
        }
    }

    unsafe fn print_functions(&self) -> (PrintHexCharLine, PrintHexCharLine) {
        (
            *self
                .c
                .get::<PrintHexCharLine>(b"printHexCharLine\0")
                .expect("C printHexCharLine export"),
            *self
                .rust
                .get::<PrintHexCharLine>(b"printHexCharLine\0")
                .expect("Rust printHexCharLine export"),
        )
    }

    unsafe fn main_functions(&self) -> (Main, Main) {
        (
            *self.c.get::<Main>(b"main\0").expect("C main export"),
            *self.rust.get::<Main>(b"main\0").expect("Rust main export"),
        )
    }
}

struct Generator(u64);

impl Generator {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn byte(&mut self) -> u8 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value as u8
    }

    fn in_range(&mut self, start: u8, end: u8) -> u8 {
        let width = u16::from(end) - u16::from(start) + 1;
        (u16::from(start) + u16::from(self.byte()) % width) as u8
    }

    fn increment_result_nonnegative_input(&mut self) -> u8 {
        if self.byte() & 7 == 0 {
            u8::MAX
        } else {
            self.in_range(0, 0x7e)
        }
    }
}

fn stdio_guard() -> MutexGuard<'static, ()> {
    STDIO_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

unsafe fn make_pipe() -> [c_int; 2] {
    let mut fds = [-1, -1];
    assert_eq!(pipe(fds.as_mut_ptr()), 0, "pipe failed");
    fds
}

unsafe fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    assert_eq!(fflush(std::ptr::null_mut()), 0, "pre-call fflush failed");
    let fds = make_pipe();
    let saved_stdout = dup(1);
    assert!(saved_stdout >= 0, "dup stdout failed");
    assert_eq!(dup2(fds[1], 1), 1, "redirect stdout failed");
    assert_eq!(close(fds[1]), 0, "close pipe writer failed");

    call();

    assert_eq!(fflush(std::ptr::null_mut()), 0, "post-call fflush failed");
    assert_eq!(dup2(saved_stdout, 1), 1, "restore stdout failed");
    assert_eq!(close(saved_stdout), 0, "close saved stdout failed");

    let mut output = Vec::new();
    File::from_raw_fd(fds[0])
        .read_to_end(&mut output)
        .expect("read captured stdout");
    output
}

unsafe fn invoke_main(function: Main, input: &[u8]) -> (c_int, Vec<u8>) {
    let input_pipe = make_pipe();
    {
        let mut writer = File::from_raw_fd(input_pipe[1]);
        writer.write_all(input).expect("write redirected stdin");
    }

    let saved_stdin = dup(0);
    assert!(saved_stdin >= 0, "dup stdin failed");
    __fpurge(stdin);
    clearerr(stdin);
    assert_eq!(dup2(input_pipe[0], 0), 0, "redirect stdin failed");
    assert_eq!(close(input_pipe[0]), 0, "close pipe reader failed");

    let mut result = -1;
    let output = capture_stdout(|| result = function());

    __fpurge(stdin);
    clearerr(stdin);
    assert_eq!(dup2(saved_stdin, 0), 0, "restore stdin failed");
    assert_eq!(close(saved_stdin), 0, "close saved stdin failed");
    (result, output)
}

fn compare_print_range(start: u8, end: u8, seed: u64) {
    let _guard = stdio_guard();
    let mut generator = Generator::new(seed);

    unsafe {
        let libraries = Libraries::load();
        let (c_print, rust_print) = libraries.print_functions();
        for _ in 0..512 {
            let value = generator.in_range(start, end);
            let c_output = capture_stdout(|| c_print(value as c_char));
            let rust_output = capture_stdout(|| rust_print(value as c_char));
            assert_eq!(rust_output, c_output, "input byte 0x{value:02x}");
        }
    }
}

fn compare_main_inputs(mut input: impl FnMut(&mut Generator) -> Vec<u8>, seed: u64) {
    let _guard = stdio_guard();
    let mut generator = Generator::new(seed);

    unsafe {
        let libraries = Libraries::load();
        let (c_main, rust_main) = libraries.main_functions();
        for _ in 0..512 {
            let bytes = input(&mut generator);
            let (c_result, c_output) = invoke_main(c_main, &bytes);
            let (rust_result, rust_output) = invoke_main(rust_main, &bytes);
            assert_eq!(rust_result, c_result, "return code for input {bytes:02x?}");
            assert_eq!(rust_output, c_output, "stdout for input {bytes:02x?}");
        }
    }
}

#[test]
fn config_01_print_zero_padded_nonnegative() {
    compare_print_range(0x00, 0x0f, 0x4d59_5df4_d0f3_3173);
}

#[test]
fn config_02_print_unpadded_nonnegative() {
    compare_print_range(0x10, 0x7f, 0x94d0_49bb_1331_11eb);
}

#[test]
fn config_03_print_negative_signed_char() {
    compare_print_range(0x80, 0xff, 0x853c_49e6_748f_ea9b);
}

#[test]
fn config_04_main_empty_input() {
    compare_main_inputs(|_| Vec::new(), 0xda3e_39cb_94b9_5bdb);
}

#[test]
fn config_05_main_one_byte_nonnegative_result() {
    compare_main_inputs(
        |generator| vec![generator.increment_result_nonnegative_input()],
        0x7d89_4842_753b_534d,
    );
}

#[test]
fn config_06_main_one_byte_negative_result() {
    compare_main_inputs(
        |generator| vec![generator.in_range(0x7f, 0xfe)],
        0x2e2a_c13a_30a9_9d65,
    );
}

#[test]
fn config_07_main_many_bytes_nonnegative_result() {
    compare_main_inputs(
        |generator| {
            let length = usize::from(generator.in_range(2, 65));
            let mut input = vec![generator.increment_result_nonnegative_input()];
            input.extend((1..length).map(|_| generator.byte()));
            input
        },
        0xb4c8_7612_9f9d_6d67,
    );
}

#[test]
fn config_08_main_many_bytes_negative_result() {
    compare_main_inputs(
        |generator| {
            let length = usize::from(generator.in_range(2, 65));
            let mut input = vec![generator.in_range(0x7f, 0xfe)];
            input.extend((1..length).map(|_| generator.byte()));
            input
        },
        0x8e5a_4e2f_153c_bdfd,
    );
}
