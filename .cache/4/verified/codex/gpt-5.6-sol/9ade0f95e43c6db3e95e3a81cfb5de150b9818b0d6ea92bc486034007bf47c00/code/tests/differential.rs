use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

type DriverFn = unsafe extern "C" fn(c_char);
type MainFn = unsafe extern "C" fn() -> c_int;

static STDIO_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    static mut stdin: *mut c_void;

    fn clearerr(stream: *mut c_void);
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn setvbuf(stream: *mut c_void, buffer: *mut c_char, mode: c_int, size: usize) -> c_int;
}

struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 32) as u8
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn ensure_c_library() -> PathBuf {
    let source = manifest_dir().join("c_src/src/main.c");
    let library = manifest_dir().join("c_src/build/libdriver_c.so");
    let stale = match (source.metadata(), library.metadata()) {
        (Ok(source_meta), Ok(library_meta)) => {
            source_meta.modified().unwrap() > library_meta.modified().unwrap()
        }
        _ => true,
    };

    if stale {
        std::fs::create_dir_all(library.parent().unwrap()).unwrap();
        let status = Command::new("cc")
            .args(["-shared", "-fPIC", "-o"])
            .arg(&library)
            .arg(&source)
            .status()
            .expect("failed to invoke the C compiler");
        assert!(status.success(), "failed to build {}", library.display());
    }

    library
}

fn rust_library() -> PathBuf {
    let test_executable = std::env::current_exe().unwrap();
    test_executable
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .join("libdriver.so")
}

fn load_libraries() -> (Library, Library) {
    let c_path = ensure_c_library();
    let rust_path = rust_library();
    assert!(rust_path.is_file(), "missing {}", rust_path.display());

    unsafe {
        (
            Library::new(&c_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display())),
            Library::new(&rust_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display())),
        )
    }
}

fn capture_stdout<R>(call: impl FnOnce() -> R) -> (R, Vec<u8>) {
    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);

        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0);

        let mut fds = [-1; 2];
        assert_eq!(pipe(fds.as_mut_ptr()), 0);
        assert_eq!(dup2(fds[1], 1), 1);
        assert_eq!(close(fds[1]), 0);

        let result = call();

        assert_eq!(fflush(std::ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(close(saved_stdout), 0);

        let mut output = Vec::new();
        File::from_raw_fd(fds[0]).read_to_end(&mut output).unwrap();
        (result, output)
    }
}

fn with_stdin<R>(input: &[u8], call: impl FnOnce() -> R) -> R {
    unsafe {
        let saved_stdin = dup(0);
        assert!(saved_stdin >= 0);

        let mut fds = [-1; 2];
        assert_eq!(pipe(fds.as_mut_ptr()), 0);
        let mut writer = File::from_raw_fd(fds[1]);
        writer.write_all(input).unwrap();
        drop(writer);

        assert_eq!(dup2(fds[0], 0), 0);
        assert_eq!(close(fds[0]), 0);
        clearerr(stdin);

        let result = call();

        clearerr(stdin);
        assert_eq!(dup2(saved_stdin, 0), 0);
        assert_eq!(close(saved_stdin), 0);
        result
    }
}

fn compare_driver_pool(row: &str, pool: &[u8], seed: u64) {
    assert!(!pool.is_empty());
    let _guard = STDIO_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let (c_library, rust_library) = load_libraries();
    let c_driver: Symbol<DriverFn> = unsafe { c_library.get(b"driver\0").unwrap() };
    let rust_driver: Symbol<DriverFn> = unsafe { rust_library.get(b"driver\0").unwrap() };
    let mut rng = Lcg::new(seed);

    for iteration in 0..128 {
        let byte = pool[(rng.next_u64() as usize) % pool.len()];
        let (_, c_output) = capture_stdout(|| unsafe { c_driver(byte as c_char) });
        let (_, rust_output) = capture_stdout(|| unsafe { rust_driver(byte as c_char) });
        assert_eq!(
            rust_output, c_output,
            "{row}, iteration {iteration}, byte 0x{byte:02x}"
        );
    }
}

fn bytes_in(ranges: &[std::ops::RangeInclusive<u8>]) -> Vec<u8> {
    ranges.iter().flat_map(Clone::clone).collect()
}

#[test]
fn c01_non_whitespace_controls() {
    compare_driver_pool(
        "C1",
        &bytes_in(&[0x00..=0x08, 0x0e..=0x1f]),
        0xc01c_01c0_1c01_c01c,
    );
}

#[test]
fn c02_horizontal_tab() {
    compare_driver_pool("C2", &[b'\t'], 0xc02c_02c0_2c02_c02c);
}

#[test]
fn c03_non_blank_whitespace_controls() {
    compare_driver_pool("C3", &bytes_in(&[0x0a..=0x0d]), 0xc03c_03c0_3c03_c03c);
}

#[test]
fn c04_space() {
    compare_driver_pool("C4", &[b' '], 0xc04c_04c0_4c04_c04c);
}

#[test]
fn c05_ascii_punctuation() {
    let punctuation: Vec<u8> = (0u8..=0x7f)
        .filter(|byte| byte.is_ascii_punctuation())
        .collect();
    compare_driver_pool("C5", &punctuation, 0xc05c_05c0_5c05_c05c);
}

#[test]
fn c06_decimal_digits() {
    compare_driver_pool("C6", &bytes_in(&[b'0'..=b'9']), 0xc06c_06c0_6c06_c06c);
}

#[test]
fn c07_uppercase_hexadecimal_letters() {
    compare_driver_pool("C7", &bytes_in(&[b'A'..=b'F']), 0xc07c_07c0_7c07_c07c);
}

#[test]
fn c08_uppercase_non_hexadecimal_letters() {
    compare_driver_pool("C8", &bytes_in(&[b'G'..=b'Z']), 0xc08c_08c0_8c08_c08c);
}

#[test]
fn c09_lowercase_hexadecimal_letters() {
    compare_driver_pool("C9", &bytes_in(&[b'a'..=b'f']), 0xc09c_09c0_9c09_c09c);
}

#[test]
fn c10_lowercase_non_hexadecimal_letters() {
    compare_driver_pool("C10", &bytes_in(&[b'g'..=b'z']), 0xc10c_10c0_0c10_c10c);
}

#[test]
fn c11_del_control() {
    compare_driver_pool("C11", &[0x7f], 0xc11c_11c0_1c11_c11c);
}

#[test]
fn c12_negative_signed_chars() {
    compare_driver_pool("C12", &bytes_in(&[0x80..=0xfe]), 0xc12c_12c0_2c12_c12c);
}

#[test]
fn c13_eof_char_value() {
    compare_driver_pool("C13", &[0xff], 0xc13c_13c0_3c13_c13c);
}

fn compare_main_input(
    row: &str,
    iteration: usize,
    input: &[u8],
    c_main: MainFn,
    rust_main: MainFn,
) {
    let (c_result, c_output) = capture_stdout(|| with_stdin(input, || unsafe { c_main() }));
    let (rust_result, rust_output) =
        capture_stdout(|| with_stdin(input, || unsafe { rust_main() }));
    assert_eq!(
        rust_result, c_result,
        "{row}, iteration {iteration}, result"
    );
    assert_eq!(
        rust_output, c_output,
        "{row}, iteration {iteration}, input {input:02x?}"
    );
}

fn with_main_functions(test: impl FnOnce(MainFn, MainFn)) {
    let _guard = STDIO_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let (c_library, rust_library) = load_libraries();
    let c_main: Symbol<MainFn> = unsafe { c_library.get(b"main\0").unwrap() };
    let rust_main: Symbol<MainFn> = unsafe { rust_library.get(b"main\0").unwrap() };

    unsafe {
        const _IONBF: c_int = 2;
        assert_eq!(setvbuf(stdin, std::ptr::null_mut(), _IONBF, 0), 0);
    }
    test(*c_main, *rust_main);
}

#[test]
fn c14_main_empty_input() {
    with_main_functions(|c_main, rust_main| {
        for iteration in 0..32 {
            compare_main_input("C14", iteration, &[], c_main, rust_main);
        }
    });
}

#[test]
fn c15_main_single_byte() {
    with_main_functions(|c_main, rust_main| {
        let mut rng = Lcg::new(0xc15c_15c0_5c15_c15c);
        for iteration in 0..512 {
            compare_main_input("C15", iteration, &[rng.next_u8()], c_main, rust_main);
        }
    });
}

#[test]
fn c16_main_multiple_bytes() {
    with_main_functions(|c_main, rust_main| {
        let mut rng = Lcg::new(0xc16c_16c0_6c16_c16c);
        for iteration in 0..128 {
            let length = 2 + (rng.next_u64() as usize % 63);
            let input: Vec<u8> = (0..length).map(|_| rng.next_u8()).collect();
            compare_main_input("C16", iteration, &input, c_main, rust_main);
        }
    });
}
