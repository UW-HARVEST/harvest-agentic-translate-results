use libloading::Library;
use std::ffi::{CString, c_char, c_int, c_void};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

type VoidFn = unsafe extern "C" fn();
type PrintLineFn = unsafe extern "C" fn(*const c_char);
type PrintIntLineFn = unsafe extern "C" fn(c_int);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

struct Api {
    _library: Library,
    bad: VoidFn,
    driver: VoidFn,
    good: VoidFn,
    print_int_line: PrintIntLineFn,
    print_line: PrintLineFn,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let bad = unsafe { *library.get(b"bad\0").expect("missing symbol bad") };
        let driver = unsafe { *library.get(b"driver\0").expect("missing symbol driver") };
        let good = unsafe { *library.get(b"good\0").expect("missing symbol good") };
        let print_int_line = unsafe {
            *library
                .get(b"printIntLine\0")
                .expect("missing symbol printIntLine")
        };
        let print_line = unsafe {
            *library
                .get(b"printLine\0")
                .expect("missing symbol printLine")
        };

        Self {
            _library: library,
            bad,
            driver,
            good,
            print_int_line,
            print_line,
        }
    }
}

static C_API: OnceLock<Api> = OnceLock::new();
static RUST_API: OnceLock<Api> = OnceLock::new();
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_api() -> &'static Api {
    C_API.get_or_init(|| {
        let path = manifest_dir().join("../c_src/build/libdriver.so");
        assert!(
            path.is_file(),
            "C shared library not found at {}",
            path.display()
        );
        unsafe { Api::load(&path) }
    })
}

fn rust_api() -> &'static Api {
    RUST_API.get_or_init(|| {
        let profile = if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        };
        let path = manifest_dir()
            .join("target")
            .join(profile)
            .join("libdriver.so");
        assert!(
            path.is_file(),
            "Rust shared library not found at {}",
            path.display()
        );
        unsafe { Api::load(&path) }
    })
}

struct StdoutRestore {
    saved_fd: c_int,
}

impl Drop for StdoutRestore {
    fn drop(&mut self) {
        unsafe {
            fflush(std::ptr::null_mut());
            assert_eq!(dup2(self.saved_fd, 1), 1, "failed to restore stdout");
            assert_eq!(close(self.saved_fd), 0, "failed to close saved stdout");
        }
    }
}

fn capture_stdout(action: impl FnOnce()) -> Vec<u8> {
    std::io::stdout()
        .flush()
        .expect("failed to flush Rust stdout");
    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0, "failed to flush C stdout");
    }

    let mut capture = temporary_capture_file();
    let saved_fd = unsafe { dup(1) };
    assert!(saved_fd >= 0, "failed to duplicate stdout");
    let restore = StdoutRestore { saved_fd };
    assert_eq!(
        unsafe { dup2(capture.as_raw_fd(), 1) },
        1,
        "failed to redirect stdout"
    );

    action();

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0, "failed to flush C stdout");
    }
    std::io::stdout()
        .flush()
        .expect("failed to flush Rust stdout");
    drop(restore);

    capture
        .seek(SeekFrom::Start(0))
        .expect("failed to rewind captured stdout");
    let mut bytes = Vec::new();
    capture
        .read_to_end(&mut bytes)
        .expect("failed to read captured stdout");
    bytes
}

fn temporary_capture_file() -> File {
    let path = std::env::temp_dir().join(format!(
        "driver-differential-{}-{:?}.out",
        std::process::id(),
        std::thread::current().id()
    ));
    OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(path)
        .expect("failed to create stdout capture file")
}

fn locked() -> MutexGuard<'static, ()> {
    STDOUT_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn next_random(state: &mut u64) -> u64 {
    let mut value = *state;
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    *state = value;
    value
}

#[test]
fn phase_b_config_01_print_line_valid_strings() {
    let _lock = locked();
    let mut state = 0x4d59_5df4_d0f3_3173;
    let mut inputs = Vec::new();

    for length in [0, 1, 2, 15, 255, 4096] {
        let bytes = (0..length)
            .map(|_| {
                let value = next_random(&mut state);
                (1 + (value % 255)) as u8
            })
            .collect::<Vec<_>>();
        inputs.push(CString::new(bytes).unwrap());
    }
    for _ in 0..256 {
        let length = (next_random(&mut state) % 513) as usize;
        let bytes = (0..length)
            .map(|_| (1 + (next_random(&mut state) % 255)) as u8)
            .collect::<Vec<_>>();
        inputs.push(CString::new(bytes).unwrap());
    }

    let c_output = capture_stdout(|| {
        for input in &inputs {
            unsafe { (c_api().print_line)(input.as_ptr()) };
        }
    });
    let rust_output = capture_stdout(|| {
        for input in &inputs {
            unsafe { (rust_api().print_line)(input.as_ptr()) };
        }
    });

    assert_eq!(rust_output, c_output);
    let expected = inputs
        .iter()
        .flat_map(|input| {
            input
                .as_bytes()
                .iter()
                .copied()
                .chain(std::iter::once(b'\n'))
        })
        .collect::<Vec<_>>();
    assert_eq!(c_output, expected);
}

#[test]
fn phase_b_config_02_print_int_line_full_range() {
    let _lock = locked();
    let mut state = 0x8a5c_d789_635d_2dff;
    let mut inputs = vec![c_int::MIN, -1, 0, 1, c_int::MAX];
    inputs.extend((0..1024).map(|_| next_random(&mut state) as c_int));

    let c_output = capture_stdout(|| {
        for input in &inputs {
            unsafe { (c_api().print_int_line)(*input) };
        }
    });
    let rust_output = capture_stdout(|| {
        for input in &inputs {
            unsafe { (rust_api().print_int_line)(*input) };
        }
    });

    assert_eq!(rust_output, c_output);
    let expected = inputs
        .iter()
        .map(|input| format!("{input}\n"))
        .collect::<String>();
    assert_eq!(c_output, expected.as_bytes());
}

#[test]
fn phase_b_config_03_bad() {
    let _lock = locked();
    let c_output = capture_stdout(|| unsafe { (c_api().bad)() });
    let rust_output = capture_stdout(|| unsafe { (rust_api().bad)() });

    assert_eq!(rust_output, c_output);
    assert_eq!(c_output, b"0\n0\n");
}

#[test]
fn phase_b_config_04_good() {
    let _lock = locked();
    let c_output = capture_stdout(|| unsafe { (c_api().good)() });
    let rust_output = capture_stdout(|| unsafe { (rust_api().good)() });

    assert_eq!(rust_output, c_output);
    assert_eq!(c_output, b"0\n2\n");
}

#[test]
fn phase_b_config_05_driver_end_to_end() {
    let _lock = locked();
    let c_output = capture_stdout(|| unsafe { (c_api().driver)() });
    let rust_output = capture_stdout(|| unsafe { (rust_api().driver)() });

    assert_eq!(rust_output, c_output);
    assert_eq!(
        c_output,
        b"Calling good()...\n0\n2\nFinished good()\nCalling bad()...\n0\n0\nFinished bad()\n"
    );
}

#[test]
fn phase_c_error_01_print_line_null_is_silent() {
    let _lock = locked();
    let c_output = capture_stdout(|| unsafe { (c_api().print_line)(std::ptr::null()) });
    let rust_output = capture_stdout(|| unsafe { (rust_api().print_line)(std::ptr::null()) });

    assert_eq!(rust_output, c_output);
    assert!(c_output.is_empty());
}
