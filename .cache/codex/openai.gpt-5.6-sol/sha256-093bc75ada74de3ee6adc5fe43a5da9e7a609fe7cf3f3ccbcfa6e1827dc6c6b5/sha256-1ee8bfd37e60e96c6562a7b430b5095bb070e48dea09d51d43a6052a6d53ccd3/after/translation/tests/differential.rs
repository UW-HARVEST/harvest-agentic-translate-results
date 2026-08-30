use libloading::Library;
use std::ffi::{CString, c_char, c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::PathBuf;
use std::ptr;
use std::sync::{Mutex, MutexGuard};

type PrintLine = unsafe extern "C" fn(*const c_char);
type PrintHexCharLine = unsafe extern "C" fn(c_char);
type NoArgs = unsafe extern "C" fn();
type Driver = unsafe extern "C" fn(c_int);

unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipefd: *mut c_int) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

struct Api {
    _library: Library,
    print_line: PrintLine,
    print_hex_char_line: PrintHexCharLine,
    bad: NoArgs,
    good: NoArgs,
    driver: Driver,
}

impl Api {
    unsafe fn load(path: PathBuf) -> Self {
        let library = unsafe {
            Library::new(&path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()))
        };
        let print_line = unsafe { *library.get(b"printLine\0").unwrap() };
        let print_hex_char_line = unsafe { *library.get(b"printHexCharLine\0").unwrap() };
        let bad = unsafe { *library.get(b"bad\0").unwrap() };
        let good = unsafe { *library.get(b"good\0").unwrap() };
        let driver = unsafe { *library.get(b"driver\0").unwrap() };

        Self {
            _library: library,
            print_line,
            print_hex_char_line,
            bad,
            good,
            driver,
        }
    }
}

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

fn load_apis() -> (MutexGuard<'static, ()>, Api, Api) {
    let guard = STDOUT_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_path = manifest.join("../c_src/build/libdriver.so");
    let rust_path = manifest.join("target/release/libdriver.so");
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

    let c = unsafe { Api::load(c_path) };
    let rust = unsafe { Api::load(rust_path) };
    (guard, c, rust)
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);

        let mut pipe_fds = [-1; 2];
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0);
        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0);
        assert_eq!(dup2(pipe_fds[1], 1), 1);
        assert_eq!(close(pipe_fds[1]), 0);

        call();

        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(close(saved_stdout), 0);

        let mut output = Vec::new();
        let mut reader = File::from_raw_fd(pipe_fds[0]);
        reader.read_to_end(&mut output).unwrap();
        output
    }
}

fn assert_same(c_call: impl FnOnce(), rust_call: impl FnOnce(), context: &str) {
    let c_output = capture_stdout(c_call);
    let rust_output = capture_stdout(rust_call);
    assert_eq!(rust_output, c_output, "{context}");
}

struct XorShift64(u64);

impl XorShift64 {
    fn next(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }
}

#[test]
fn config_1_print_line_non_null_strings() {
    let (_guard, c, rust) = load_apis();
    let mut rng = XorShift64(0x54a3_22d1_2f87_96c5);

    for case in 0..256 {
        let length = if case == 0 {
            0
        } else {
            (rng.next() % 129) as usize
        };
        let bytes: Vec<u8> = (0..length)
            .map(|_| ((rng.next() % 255) + 1) as u8)
            .collect();
        let line = CString::new(bytes).unwrap();
        assert_same(
            || unsafe { (c.print_line)(line.as_ptr()) },
            || unsafe { (rust.print_line)(line.as_ptr()) },
            &format!("printLine randomized case {case}"),
        );
    }
}

#[test]
fn config_2_print_hex_negative_char() {
    let (_guard, c, rust) = load_apis();
    let mut rng = XorShift64(0xc191_8e37_62bf_a451);

    for case in 0..256 {
        let value = if case == 0 {
            c_char::MIN
        } else if case == 1 {
            -1
        } else {
            -((rng.next() % 128) as c_char) - 1
        };
        assert_same(
            || unsafe { (c.print_hex_char_line)(value) },
            || unsafe { (rust.print_hex_char_line)(value) },
            &format!("printHexCharLine({value})"),
        );
    }
}

#[test]
fn config_3_print_hex_nonnegative_char() {
    let (_guard, c, rust) = load_apis();
    let mut rng = XorShift64(0x072d_91a4_b3c8_ef65);

    for case in 0..256 {
        let value = match case {
            0 => 0,
            1 => 1,
            2 => c_char::MAX,
            _ => (rng.next() % 128) as c_char,
        };
        assert_same(
            || unsafe { (c.print_hex_char_line)(value) },
            || unsafe { (rust.print_hex_char_line)(value) },
            &format!("printHexCharLine({value})"),
        );
    }
}

#[test]
fn config_4_bad_pipeline() {
    let (_guard, c, rust) = load_apis();
    assert_same(|| unsafe { (c.bad)() }, || unsafe { (rust.bad)() }, "bad()");
}

#[test]
fn config_5_good_pipeline() {
    let (_guard, c, rust) = load_apis();
    assert_same(
        || unsafe { (c.good)() },
        || unsafe { (rust.good)() },
        "good()",
    );
}

#[test]
fn config_6_driver_zero() {
    let (_guard, c, rust) = load_apis();
    assert_same(
        || unsafe { (c.driver)(0) },
        || unsafe { (rust.driver)(0) },
        "driver(0)",
    );
}

#[test]
fn config_7_driver_nonzero() {
    let (_guard, c, rust) = load_apis();
    let mut rng = XorShift64(0x968f_235c_4ba1_d7e2);

    for case in 0..256 {
        let mut value = rng.next() as c_int;
        if value == 0 {
            value = if case % 2 == 0 {
                c_int::MIN
            } else {
                c_int::MAX
            };
        }
        assert_same(
            || unsafe { (c.driver)(value) },
            || unsafe { (rust.driver)(value) },
            &format!("driver({value})"),
        );
    }
}

#[test]
fn error_1_print_line_null() {
    let (_guard, c, rust) = load_apis();
    assert_same(
        || unsafe { (c.print_line)(ptr::null()) },
        || unsafe { (rust.print_line)(ptr::null()) },
        "printLine(NULL)",
    );
}

#[test]
fn error_2_good_b2g_rejects_oversized_data() {
    let (_guard, c, rust) = load_apis();
    let c_output = capture_stdout(|| unsafe { (c.good)() });
    let rust_output = capture_stdout(|| unsafe { (rust.good)() });
    let expected = b"04\ndata value is too large to perform arithmetic safely.\n";

    assert_eq!(c_output, expected);
    assert_eq!(rust_output, c_output);
}
