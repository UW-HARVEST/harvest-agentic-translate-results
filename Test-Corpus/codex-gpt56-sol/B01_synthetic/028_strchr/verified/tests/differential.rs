use libloading::Library;
use std::ffi::{c_char, c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::{FromRawFd, RawFd};
use std::path::PathBuf;
use std::process::Command;
use std::ptr;
use std::sync::{Mutex, Once};

type Foo = unsafe extern "C" fn(*const c_char, c_char) -> c_int;
type Driver = unsafe extern "C" fn(*const c_char);
type Main = unsafe extern "C" fn() -> c_int;

unsafe extern "C" {
    fn clearerr(stream: *mut c_void);
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
    fn setvbuf(stream: *mut c_void, buffer: *mut c_char, mode: c_int, size: usize) -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn write(fd: c_int, buffer: *const c_void, count: usize) -> isize;
    fn _exit(status: c_int) -> !;

    static mut stdin: *mut c_void;
}

const STDIN_FILENO: RawFd = 0;
const STDOUT_FILENO: RawFd = 1;
const SIGSEGV: c_int = 11;
const _IONBF: c_int = 2;
const CASES: usize = 64;

static STDIO_LOCK: Mutex<()> = Mutex::new(());
static BUILD_RUST_SO: Once = Once::new();
static UNBUFFER_STDIN: Once = Once::new();

struct Api {
    _library: Library,
    foo: Foo,
    driver: Driver,
    main: Main,
}

impl Api {
    unsafe fn load(path: PathBuf) -> Self {
        let library = unsafe { Library::new(&path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let foo = unsafe { *library.get::<Foo>(b"foo\0").expect("missing foo") };
        let driver = unsafe { *library.get::<Driver>(b"driver\0").expect("missing driver") };
        let main = unsafe { *library.get::<Main>(b"main\0").expect("missing main") };

        Self {
            _library: library,
            foo,
            driver,
            main,
        }
    }
}

fn load_apis() -> (Api, Api) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_path = root.join("c_src/build/libdriver_c.so");
    let rust_path = root.join("target/debug/libdriver.so");
    assert!(
        c_path.is_file(),
        "missing C shared object: {}",
        c_path.display()
    );

    BUILD_RUST_SO.call_once(|| {
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        let status = Command::new(cargo)
            .args(["build", "--no-default-features", "--lib"])
            .current_dir(&root)
            .status()
            .expect("failed to start cargo build for Rust cdylib");
        assert!(status.success(), "failed to build Rust cdylib");
    });
    assert!(
        rust_path.is_file(),
        "missing Rust shared object: {}",
        rust_path.display()
    );

    unsafe { (Api::load(c_path), Api::load(rust_path)) }
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(row: u64) -> Self {
        Self(0x4d59_5df4_d0f3_3173 ^ row.wrapping_mul(0x9e37_79b9_7f4a_7c15))
    }

    fn next(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn usize(&mut self, start: usize, end: usize) -> usize {
        start + self.next() as usize % (end - start)
    }

    fn nonzero_byte(&mut self) -> u8 {
        self.usize(1, 256) as u8
    }

    fn byte_except(&mut self, excluded: &[u8]) -> u8 {
        loop {
            let byte = self.nonzero_byte();
            if !excluded.contains(&byte) {
                return byte;
            }
        }
    }
}

fn compare_foo(cases: impl IntoIterator<Item = (Vec<u8>, u8)>) {
    let (c, rust) = load_apis();
    for (index, (input, needle)) in cases.into_iter().enumerate() {
        assert_eq!(input.last(), Some(&0), "case {index} lacks a terminator");
        let c_result = unsafe { (c.foo)(input.as_ptr().cast(), needle as c_char) };
        let rust_result = unsafe { (rust.foo)(input.as_ptr().cast(), needle as c_char) };
        assert_eq!(
            rust_result, c_result,
            "foo diverged in randomized case {index}, needle={needle:#04x}"
        );
    }
}

unsafe fn capture_stdout<T>(call: impl FnOnce() -> T) -> (T, Vec<u8>) {
    let mut pipe_fds = [-1; 2];
    assert_eq!(unsafe { pipe(pipe_fds.as_mut_ptr()) }, 0);
    assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0);

    let saved_stdout = unsafe { dup(STDOUT_FILENO) };
    assert!(saved_stdout >= 0);
    assert_eq!(unsafe { dup2(pipe_fds[1], STDOUT_FILENO) }, STDOUT_FILENO);
    assert_eq!(unsafe { close(pipe_fds[1]) }, 0);

    let result = call();
    assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0);
    assert_eq!(unsafe { dup2(saved_stdout, STDOUT_FILENO) }, STDOUT_FILENO);
    assert_eq!(unsafe { close(saved_stdout) }, 0);

    let mut output = Vec::new();
    let mut reader = unsafe { File::from_raw_fd(pipe_fds[0]) };
    reader
        .read_to_end(&mut output)
        .expect("read captured stdout");
    (result, output)
}

fn compare_driver(cases: impl IntoIterator<Item = Vec<u8>>) {
    let _guard = STDIO_LOCK.lock().expect("stdio lock poisoned");
    let (c, rust) = load_apis();

    for (index, input) in cases.into_iter().enumerate() {
        assert_eq!(input.last(), Some(&0), "case {index} lacks a terminator");
        let (_, c_output) = unsafe { capture_stdout(|| (c.driver)(input.as_ptr().cast())) };
        let (_, rust_output) = unsafe { capture_stdout(|| (rust.driver)(input.as_ptr().cast())) };
        assert_eq!(
            rust_output, c_output,
            "driver diverged in randomized case {index}"
        );
    }
}

unsafe fn write_all_fd(fd: RawFd, mut bytes: &[u8]) {
    while !bytes.is_empty() {
        let written = unsafe { write(fd, bytes.as_ptr().cast(), bytes.len()) };
        assert!(written > 0, "write to stdin pipe failed");
        bytes = &bytes[written as usize..];
    }
}

unsafe fn call_main_with_stdin(function: Main, input: &[u8]) -> (c_int, Vec<u8>) {
    UNBUFFER_STDIN.call_once(|| {
        assert_eq!(
            unsafe { setvbuf(stdin, ptr::null_mut(), _IONBF, 0) },
            0,
            "failed to make C stdin unbuffered"
        );
    });

    let mut pipe_fds = [-1; 2];
    assert_eq!(unsafe { pipe(pipe_fds.as_mut_ptr()) }, 0);
    unsafe { write_all_fd(pipe_fds[1], input) };
    assert_eq!(unsafe { close(pipe_fds[1]) }, 0);

    let saved_stdin = unsafe { dup(STDIN_FILENO) };
    assert!(saved_stdin >= 0);
    assert_eq!(unsafe { dup2(pipe_fds[0], STDIN_FILENO) }, STDIN_FILENO);
    assert_eq!(unsafe { close(pipe_fds[0]) }, 0);
    unsafe { clearerr(stdin) };

    let captured = unsafe { capture_stdout(|| function()) };

    assert_eq!(unsafe { dup2(saved_stdin, STDIN_FILENO) }, STDIN_FILENO);
    assert_eq!(unsafe { close(saved_stdin) }, 0);
    unsafe { clearerr(stdin) };
    captured
}

fn compare_main(cases: impl IntoIterator<Item = Vec<u8>>) {
    let _guard = STDIO_LOCK.lock().expect("stdio lock poisoned");
    let (c, rust) = load_apis();

    for (index, input) in cases.into_iter().enumerate() {
        let c_result = unsafe { call_main_with_stdin(c.main, &input) };
        let rust_result = unsafe { call_main_with_stdin(rust.main, &input) };
        assert_eq!(
            rust_result,
            c_result,
            "main diverged in randomized case {index}, input length={}",
            input.len()
        );
    }
}

fn random_bytes(rng: &mut Rng, len: usize, excluded: &[u8]) -> Vec<u8> {
    (0..len).map(|_| rng.byte_except(excluded)).collect()
}

#[test]
fn config_01_foo_empty() {
    let mut rng = Rng::new(1);
    compare_foo((0..CASES).map(|_| (vec![0], rng.nonzero_byte())));
}

#[test]
fn config_02_foo_one_byte_needle_absent() {
    let mut rng = Rng::new(2);
    compare_foo((0..CASES).map(|_| {
        let input = rng.nonzero_byte();
        let needle = rng.byte_except(&[input]);
        (vec![input, 0], needle)
    }));
}

#[test]
fn config_03_foo_one_byte_needle_present() {
    let mut rng = Rng::new(3);
    compare_foo((0..CASES).map(|_| {
        let needle = rng.nonzero_byte();
        (vec![needle, 0], needle)
    }));
}

#[test]
fn config_04_foo_many_bytes_needle_absent() {
    let mut rng = Rng::new(4);
    compare_foo((0..CASES).map(|_| {
        let needle = rng.nonzero_byte();
        let len = rng.usize(2, 129);
        let mut input = random_bytes(&mut rng, len, &[needle]);
        input.push(0);
        (input, needle)
    }));
}

#[test]
fn config_05_foo_many_bytes_one_match() {
    let mut rng = Rng::new(5);
    compare_foo((0..CASES).map(|_| {
        let needle = rng.nonzero_byte();
        let len = rng.usize(2, 129);
        let mut input = random_bytes(&mut rng, len, &[needle]);
        let index = rng.usize(0, len);
        input[index] = needle;
        input.push(0);
        (input, needle)
    }));
}

#[test]
fn config_06_foo_many_bytes_repeated_matches() {
    let mut rng = Rng::new(6);
    compare_foo((0..CASES).map(|_| {
        let needle = rng.nonzero_byte();
        let len = rng.usize(3, 129);
        let mut input = random_bytes(&mut rng, len, &[needle]);
        let first = rng.usize(0, len);
        let mut second = rng.usize(0, len - 1);
        if second >= first {
            second += 1;
        }
        input[first] = needle;
        input[second] = needle;
        input.push(0);
        (input, needle)
    }));
}

#[test]
fn config_07_foo_embedded_nul() {
    let mut rng = Rng::new(7);
    compare_foo((0..CASES).map(|_| {
        let needle = rng.nonzero_byte();
        let prefix_len = rng.usize(0, 65);
        let suffix_len = rng.usize(1, 65);
        let mut input = random_bytes(&mut rng, prefix_len, &[]);
        input.push(0);
        input.extend(std::iter::repeat(needle).take(suffix_len));
        input.push(0);
        (input, needle)
    }));
}

#[test]
fn config_08_foo_high_bit_bytes() {
    let mut rng = Rng::new(8);
    compare_foo((0..CASES).map(|_| {
        let needle = rng.usize(128, 256) as u8;
        let len = rng.usize(2, 129);
        let mut input: Vec<u8> = (0..len).map(|_| rng.usize(128, 256) as u8).collect();
        input[rng.usize(0, len)] = needle;
        input.push(0);
        (input, needle)
    }));
}

#[test]
fn config_09_driver_empty() {
    compare_driver((0..CASES).map(|_| vec![0]));
}

#[test]
fn config_10_driver_a_without_x() {
    let mut rng = Rng::new(10);
    compare_driver((0..CASES).map(|_| {
        let len = rng.usize(1, 129);
        let mut input = random_bytes(&mut rng, len, &[b'x', 0]);
        input[rng.usize(0, len)] = b'A';
        input.push(0);
        input
    }));
}

#[test]
fn config_11_driver_x_without_a() {
    let mut rng = Rng::new(11);
    compare_driver((0..CASES).map(|_| {
        let len = rng.usize(1, 129);
        let mut input = random_bytes(&mut rng, len, &[b'A', 0]);
        input[rng.usize(0, len)] = b'x';
        input.push(0);
        input
    }));
}

#[test]
fn config_12_driver_neither_fixed_needle() {
    let mut rng = Rng::new(12);
    compare_driver((0..CASES).map(|_| {
        let len = rng.usize(1, 129);
        let mut input = random_bytes(&mut rng, len, &[b'A', b'x', 0]);
        input.push(0);
        input
    }));
}

#[test]
fn config_13_driver_mixed_repeated_needles() {
    let mut rng = Rng::new(13);
    compare_driver((0..CASES).map(|_| {
        let len = rng.usize(4, 129);
        let mut input = random_bytes(&mut rng, len, &[b'A', b'x', 0]);
        input[0] = b'A';
        input[1] = b'x';
        input[len - 2] = b'A';
        input[len - 1] = b'x';
        input.push(0);
        input
    }));
}

#[test]
fn config_14_driver_embedded_nul() {
    let mut rng = Rng::new(14);
    compare_driver((0..CASES).map(|_| {
        let prefix_len = rng.usize(0, 65);
        let suffix_len = rng.usize(2, 65);
        let mut input = random_bytes(&mut rng, prefix_len, &[]);
        input.push(0);
        input.extend(random_bytes(&mut rng, suffix_len, &[]));
        input.push(b'A');
        input.push(b'x');
        input.push(0);
        input
    }));
}

#[test]
fn config_15_main_immediate_eof() {
    compare_main((0..CASES).map(|_| Vec::new()));
}

#[test]
fn config_16_main_short_without_nul() {
    let mut rng = Rng::new(16);
    compare_main((0..CASES).map(|_| {
        let len = rng.usize(1, 1000);
        random_bytes(&mut rng, len, &[])
    }));
}

#[test]
fn config_17_main_short_with_embedded_nul() {
    let mut rng = Rng::new(17);
    compare_main((0..CASES).map(|_| {
        let len = rng.usize(2, 1000);
        let mut input = random_bytes(&mut rng, len, &[]);
        let index = rng.usize(0, len);
        input[index] = 0;
        input
    }));
}

#[test]
fn config_18_main_exactly_1000_with_nul() {
    let mut rng = Rng::new(18);
    compare_main((0..CASES).map(|_| {
        let mut input = random_bytes(&mut rng, 1000, &[]);
        input[rng.usize(0, 1000)] = 0;
        input
    }));
}

#[test]
fn config_19_main_more_than_1000() {
    let mut rng = Rng::new(19);
    compare_main((0..CASES).map(|_| {
        let len = rng.usize(1001, 1401);
        let mut input = random_bytes(&mut rng, len, &[]);
        input[rng.usize(0, 1000)] = 0;
        input
    }));
}

unsafe fn crash_status(call: impl FnOnce()) -> c_int {
    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        call();
        unsafe { _exit(0) };
    }

    let mut status = 0;
    assert_eq!(unsafe { waitpid(pid, &mut status, 0) }, pid);
    status
}

fn terminating_signal(status: c_int) -> Option<c_int> {
    let signal = status & 0x7f;
    (signal != 0 && signal != 0x7f).then_some(signal)
}

#[test]
fn error_g1_foo_null_pointer() {
    let _guard = STDIO_LOCK.lock().expect("stdio lock poisoned");
    let (c, rust) = load_apis();
    let c_status = unsafe {
        crash_status(|| {
            (c.foo)(ptr::null(), b'A' as c_char);
        })
    };
    let rust_status = unsafe {
        crash_status(|| {
            (rust.foo)(ptr::null(), b'A' as c_char);
        })
    };
    assert_eq!(rust_status, c_status);
    assert_eq!(terminating_signal(c_status), Some(SIGSEGV));
}

#[test]
fn error_g2_driver_null_pointer() {
    let _guard = STDIO_LOCK.lock().expect("stdio lock poisoned");
    let (c, rust) = load_apis();
    let c_status = unsafe { crash_status(|| (c.driver)(ptr::null())) };
    let rust_status = unsafe { crash_status(|| (rust.driver)(ptr::null())) };
    assert_eq!(rust_status, c_status);
    assert_eq!(terminating_signal(c_status), Some(SIGSEGV));
}
