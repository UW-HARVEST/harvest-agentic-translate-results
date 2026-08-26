use libloading::Library;
use std::ffi::c_void;
use std::fs::{remove_file, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::fd::AsRawFd;
use std::os::raw::c_int;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

type MainFn = unsafe extern "C" fn() -> c_int;

unsafe extern "C" {
    fn clearerr(stream: *mut c_void);
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    static mut stdin: *mut c_void;
}

struct LoadedMain {
    _library: Library,
    entry: MainFn,
}

impl LoadedMain {
    unsafe fn load(path: &Path) -> Self {
        let library = Library::new(path)
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let entry = *library
            .get::<MainFn>(b"main\0")
            .unwrap_or_else(|error| panic!("failed to load main from {}: {error}", path.display()));
        Self {
            _library: library,
            entry,
        }
    }
}

fn rust_library_path() -> PathBuf {
    let executable = std::env::current_exe().expect("current test executable");
    let deps_dir = executable.parent().expect("test deps directory");
    let profile_dir = deps_dir.parent().expect("Cargo profile directory");
    let candidates = [
        profile_dir.join("libdriver.so"),
        deps_dir.join("libdriver.so"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/libdriver.so"),
    ];

    candidates
        .into_iter()
        .find(|path| path.is_file())
        .unwrap_or_else(|| panic!("Rust cdylib not found in Cargo target directory"))
}

fn c_library_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libdriver_c.so")
}

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn invoke(entry: MainFn, input: &[u8]) -> (c_int, Vec<u8>) {
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let stem = format!("driver-differential-{}-{sequence}", std::process::id());
    let input_path = std::env::temp_dir().join(format!("{stem}.in"));
    let output_path = std::env::temp_dir().join(format!("{stem}.out"));

    let mut input_file = OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(&input_path)
        .expect("create redirected stdin");
    input_file.write_all(input).expect("write redirected stdin");
    input_file.flush().expect("flush redirected stdin");
    input_file
        .seek(SeekFrom::Start(0))
        .expect("rewind redirected stdin");

    let mut output_file = OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(&output_path)
        .expect("create redirected stdout");

    let (saved_stdin, saved_stdout) = unsafe {
        fflush(std::ptr::null_mut());
        let saved_stdin = dup(0);
        let saved_stdout = dup(1);
        assert!(saved_stdin >= 0 && saved_stdout >= 0, "dup failed");
        assert_eq!(dup2(input_file.as_raw_fd(), 0), 0, "redirect stdin");
        assert_eq!(dup2(output_file.as_raw_fd(), 1), 1, "redirect stdout");
        clearerr(stdin);
        (saved_stdin, saved_stdout)
    };

    let result = unsafe { entry() };

    unsafe {
        fflush(std::ptr::null_mut());
        assert_eq!(dup2(saved_stdin, 0), 0, "restore stdin");
        assert_eq!(dup2(saved_stdout, 1), 1, "restore stdout");
        assert_eq!(close(saved_stdin), 0, "close saved stdin");
        assert_eq!(close(saved_stdout), 0, "close saved stdout");
        clearerr(stdin);
    }

    output_file
        .seek(SeekFrom::Start(0))
        .expect("rewind redirected stdout");
    let mut output = Vec::new();
    output_file
        .read_to_end(&mut output)
        .expect("read redirected stdout");

    drop(input_file);
    drop(output_file);
    remove_file(input_path).expect("remove redirected stdin");
    remove_file(output_path).expect("remove redirected stdout");
    (result, output)
}

fn assert_case(
    c_main: MainFn,
    rust_main: MainFn,
    input: &[u8],
    expected_output: &[u8],
    label: &str,
) {
    let c_result = invoke(c_main, input);
    let rust_result = invoke(rust_main, input);

    assert_eq!(c_result.0, 0, "{label}: unexpected C return");
    assert_eq!(rust_result.0, c_result.0, "{label}: return mismatch");
    assert_eq!(c_result.1, expected_output, "{label}: unexpected C output");
    assert_eq!(
        rust_result.1,
        c_result.1,
        "{label}: output mismatch for input {:?}",
        String::from_utf8_lossy(input)
    );
}

struct XorShift64(u64);

impl XorShift64 {
    fn next_u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value as u32
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    fn i32_except(&mut self, excluded: i32) -> i32 {
        loop {
            let value = self.next_i32();
            if value != excluded {
                return value;
            }
        }
    }
}

const OK: &[u8] = b"Ok!\nResult: 0\n";
const ERROR_1: &[u8] = b"Error: x != 1\nOperation failed\nResult: 1\n";
const ERROR_2: &[u8] = b"Error: x == 1 but y != 2\nOperation failed\nResult: 2\n";
const ERROR_3: &[u8] = b"Error: x == 1 and y == 2, but z != 3\nOperation failed\nResult: 3\n";

#[test]
fn all_configuration_and_error_rows_match_through_shared_objects() {
    let c_path = c_library_path();
    assert!(
        c_path.is_file(),
        "build the C shared object first: {}",
        c_path.display()
    );
    let rust_path = rust_library_path();
    let c = unsafe { LoadedMain::load(&c_path) };
    let rust = unsafe { LoadedMain::load(&rust_path) };
    let mut random = XorShift64(0x5eed_c0de_d15c_a11e);

    let prefixes = ["", " ", "\n", "\t \n"];
    let ones = ["1", "+1", "01", "+0001"];
    let twos = ["2", "+2", "02", "+0002"];
    let threes = ["3", "+3", "03", "+0003"];
    let separators = [" ", "  ", "\n", "\t", " \n\t "];
    let suffixes = ["", "\n", " \t\n"];
    for case in 0..128 {
        let input = format!(
            "{}{}{}{}{}{}{}",
            prefixes[random.next_u32() as usize % prefixes.len()],
            ones[random.next_u32() as usize % ones.len()],
            separators[random.next_u32() as usize % separators.len()],
            twos[random.next_u32() as usize % twos.len()],
            separators[random.next_u32() as usize % separators.len()],
            threes[random.next_u32() as usize % threes.len()],
            suffixes[random.next_u32() as usize % suffixes.len()],
        );
        assert_case(
            c.entry,
            rust.entry,
            input.as_bytes(),
            OK,
            &format!("valid-{case}"),
        );
    }

    for case in 0..128 {
        let x = random.i32_except(1);
        let input = format!("{x} {} {}\n", random.next_i32(), random.next_i32());
        assert_case(
            c.entry,
            rust.entry,
            input.as_bytes(),
            ERROR_1,
            &format!("error-1-{case}"),
        );
    }

    for case in 0..128 {
        let y = random.i32_except(2);
        let input = format!("1 {y} {}\n", random.next_i32());
        assert_case(
            c.entry,
            rust.entry,
            input.as_bytes(),
            ERROR_2,
            &format!("error-2-{case}"),
        );
    }

    for case in 0..128 {
        let z = random.i32_except(3);
        let input = format!("1 2 {z}\n");
        assert_case(
            c.entry,
            rust.entry,
            input.as_bytes(),
            ERROR_3,
            &format!("error-3-{case}"),
        );
    }

    let boundary_and_shape_cases: &[(&[u8], &[u8], &str)] = &[
        (b"", ERROR_1, "empty-input"),
        (b"not-an-integer", ERROR_1, "malformed-first"),
        (b"1 not-an-integer", ERROR_3, "malformed-second-retains-y"),
        (
            b"1 9 not-an-integer",
            ERROR_2,
            "malformed-third-after-bad-y",
        ),
        (b"1 2 not-an-integer", ERROR_3, "malformed-third"),
        (b"-2147483648 2 3", ERROR_1, "minimum-int-x"),
        (b"1 -2147483648 3", ERROR_2, "minimum-int-y"),
        (b"1 2 2147483647", ERROR_3, "maximum-int-z"),
        (b"1 2 3 4 trailing input", OK, "ignored-trailing-input"),
        (b"1\0 2 3", ERROR_3, "embedded-nul"),
    ];
    for (input, expected, label) in boundary_and_shape_cases {
        assert_case(c.entry, rust.entry, input, expected, label);
    }

    assert_case(c.entry, rust.entry, b"0 77 0", ERROR_1, "set-y-to-77");
    assert_case(
        c.entry,
        rust.entry,
        b"1",
        ERROR_2,
        "partial-input-retains-y-77",
    );
    assert_case(c.entry, rust.entry, b"0 2 0", ERROR_1, "set-y-to-2");
    assert_case(
        c.entry,
        rust.entry,
        b"1",
        ERROR_3,
        "partial-input-retains-y-2",
    );
    assert_case(c.entry, rust.entry, b"1 2 3", OK, "final-success");
}
