use libloading::Library;
use std::ffi::{CString, c_char, c_int, c_void};
use std::fs::{OpenOptions, remove_file};
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};

type PrintLine = unsafe extern "C" fn(*const c_char);
type PrintIntLine = unsafe extern "C" fn(c_int);
type Unary = unsafe extern "C" fn(c_int);
type Driver = unsafe extern "C" fn(c_int, c_int);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

struct Api {
    _library: Library,
    print_line: PrintLine,
    print_int_line: PrintIntLine,
    bad: Unary,
    good: Unary,
    driver: Driver,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let print_line = unsafe { *library.get(b"printLine\0").unwrap() };
        let print_int_line = unsafe { *library.get(b"printIntLine\0").unwrap() };
        let bad = unsafe { *library.get(b"bad\0").unwrap() };
        let good = unsafe { *library.get(b"good\0").unwrap() };
        let driver = unsafe { *library.get(b"driver\0").unwrap() };

        Self {
            _library: library,
            print_line,
            print_int_line,
            bad,
            good,
            driver,
        }
    }
}

struct Libraries {
    c: Api,
    rust: Api,
}

impl Libraries {
    fn load() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("c_src/build/libdriver.so");
        let rust_path = root.join("target/release/libdriver.so");
        assert!(c_path.is_file(), "missing C library: {}", c_path.display());
        assert!(
            rust_path.is_file(),
            "missing Rust library: {}; run a release build first",
            rust_path.display()
        );

        unsafe {
            Self {
                c: Api::load(&c_path),
                rust: Api::load(&rust_path),
            }
        }
    }
}

struct FixedRng(u64);

impl FixedRng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u64() as i32
    }

    fn index(&mut self) -> i32 {
        (self.next_u64() % 10) as i32
    }
}

static STDOUT_LOCK: Mutex<()> = Mutex::new(());
static CAPTURE_ID: AtomicU64 = AtomicU64::new(0);

fn stdout_lock() -> MutexGuard<'static, ()> {
    STDOUT_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn capture_stdout(action: impl FnOnce()) -> Vec<u8> {
    let _lock = stdout_lock();
    let path = std::env::temp_dir().join(format!(
        "driver-differential-{}-{}.out",
        std::process::id(),
        CAPTURE_ID.fetch_add(1, Ordering::Relaxed)
    ));
    let mut output = OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);
        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0);
        assert_eq!(dup2(output.as_raw_fd(), 1), 1);

        action();

        assert_eq!(fflush(std::ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(close(saved_stdout), 0);
    }

    output.seek(SeekFrom::Start(0)).unwrap();
    let mut bytes = Vec::new();
    output.read_to_end(&mut bytes).unwrap();
    drop(output);
    remove_file(path).unwrap();
    bytes
}

fn compare(c_call: impl FnOnce(), rust_call: impl FnOnce()) -> Vec<u8> {
    let c_output = capture_stdout(c_call);
    let rust_output = capture_stdout(rust_call);
    assert_eq!(rust_output, c_output);
    c_output
}

#[test]
fn config_1_print_line_randomized() {
    let libraries = Libraries::load();
    let mut rng = FixedRng::new(0x3a5f_91c2_74e8_d601);

    for case in 0..256 {
        let length = if case == 0 {
            0
        } else {
            (rng.next_u64() % 129) as usize
        };
        let bytes: Vec<u8> = (0..length)
            .map(|_| ((rng.next_u64() % 255) + 1) as u8)
            .collect();
        let line = CString::new(bytes).unwrap();
        compare(
            || unsafe { (libraries.c.print_line)(line.as_ptr()) },
            || unsafe { (libraries.rust.print_line)(line.as_ptr()) },
        );
    }
}

#[test]
fn config_2_print_int_line_randomized() {
    let libraries = Libraries::load();
    let mut values = vec![i32::MIN, -1, 0, 1, i32::MAX];
    let mut rng = FixedRng::new(0x624d_b951_0e73_a8cf);
    values.extend((0..256).map(|_| rng.next_i32()));

    for value in values {
        compare(
            || unsafe { (libraries.c.print_int_line)(value) },
            || unsafe { (libraries.rust.print_int_line)(value) },
        );
    }
}

#[test]
fn config_3_bad_valid_indices_randomized() {
    let libraries = Libraries::load();
    let mut rng = FixedRng::new(0x92de_5f30_187a_c46b);
    let mut values = vec![0, 9];
    values.extend((0..256).map(|_| rng.index()));

    for value in values {
        compare(
            || unsafe { (libraries.c.bad)(value) },
            || unsafe { (libraries.rust.bad)(value) },
        );
    }
}

#[test]
fn config_4_good_valid_indices_randomized() {
    let libraries = Libraries::load();
    let mut rng = FixedRng::new(0xa57c_813e_69d2_04bf);
    let mut values = vec![0, 9];
    values.extend((0..256).map(|_| rng.index()));

    for value in values {
        compare(
            || unsafe { (libraries.c.good)(value) },
            || unsafe { (libraries.rust.good)(value) },
        );
    }
}

#[test]
fn config_5_driver_valid_cross_product_randomized() {
    let libraries = Libraries::load();

    for good_data in 0..10 {
        for bad_data in 0..10 {
            compare(
                || unsafe { (libraries.c.driver)(good_data, bad_data) },
                || unsafe { (libraries.rust.driver)(good_data, bad_data) },
            );
        }
    }

    let mut rng = FixedRng::new(0x47b9_e012_d635_ac8f);
    for _ in 0..256 {
        let good_data = rng.index();
        let bad_data = rng.index();
        compare(
            || unsafe { (libraries.c.driver)(good_data, bad_data) },
            || unsafe { (libraries.rust.driver)(good_data, bad_data) },
        );
    }
}

#[test]
fn error_1_print_line_null() {
    let libraries = Libraries::load();
    let output = compare(
        || unsafe { (libraries.c.print_line)(std::ptr::null()) },
        || unsafe { (libraries.rust.print_line)(std::ptr::null()) },
    );
    assert!(output.is_empty());
}

#[test]
fn error_2_bad_negative_randomized_and_driver_path() {
    let libraries = Libraries::load();
    let mut rng = FixedRng::new(0xb42e_7ca1_590d_836f);
    let mut values = vec![i32::MIN, -1];
    values.extend((0..128).map(|_| rng.next_i32() | i32::MIN));

    for value in values {
        let direct = compare(
            || unsafe { (libraries.c.bad)(value) },
            || unsafe { (libraries.rust.bad)(value) },
        );
        assert_eq!(direct, b"ERROR: Array index is negative.\n");

        let good_data = rng.index();
        let composed = compare(
            || unsafe { (libraries.c.driver)(good_data, value) },
            || unsafe { (libraries.rust.driver)(good_data, value) },
        );
        assert!(
            composed
                .windows(b"ERROR: Array index is negative.\n".len())
                .any(|window| window == b"ERROR: Array index is negative.\n")
        );
    }
}

#[test]
fn error_3_good_negative_randomized_and_driver_path() {
    let libraries = Libraries::load();
    let mut rng = FixedRng::new(0x16f3_89ca_b750_4d2e);
    let mut values = vec![i32::MIN, -1];
    values.extend((0..128).map(|_| rng.next_i32() | i32::MIN));

    for value in values {
        let direct = compare(
            || unsafe { (libraries.c.good)(value) },
            || unsafe { (libraries.rust.good)(value) },
        );
        assert!(direct.ends_with(b"ERROR: Array index is out-of-bounds\n"));

        let bad_data = rng.index();
        let composed = compare(
            || unsafe { (libraries.c.driver)(value, bad_data) },
            || unsafe { (libraries.rust.driver)(value, bad_data) },
        );
        assert!(
            composed
                .windows(b"ERROR: Array index is out-of-bounds\n".len())
                .any(|window| window == b"ERROR: Array index is out-of-bounds\n")
        );
    }
}

#[test]
fn error_4_good_upper_bound_randomized_and_driver_path() {
    let libraries = Libraries::load();
    let mut rng = FixedRng::new(0xc581_4ad7_32e9_06bf);
    let mut values = vec![10, 11, i32::MAX];
    values.extend((0..128).map(|_| 10 + (rng.next_u64() % (i32::MAX as u64 - 9)) as i32));

    for value in values {
        let direct = compare(
            || unsafe { (libraries.c.good)(value) },
            || unsafe { (libraries.rust.good)(value) },
        );
        assert!(direct.ends_with(b"ERROR: Array index is out-of-bounds\n"));

        let bad_data = rng.index();
        let composed = compare(
            || unsafe { (libraries.c.driver)(value, bad_data) },
            || unsafe { (libraries.rust.driver)(value, bad_data) },
        );
        assert!(
            composed
                .windows(b"ERROR: Array index is out-of-bounds\n".len())
                .any(|window| window == b"ERROR: Array index is out-of-bounds\n")
        );
    }
}

#[test]
fn error_5_bad_one_past_is_not_rejected() {
    let libraries = Libraries::load();
    let direct = compare(
        || unsafe { (libraries.c.bad)(10) },
        || unsafe { (libraries.rust.bad)(10) },
    );
    assert!(!direct.starts_with(b"ERROR:"));

    let composed = compare(
        || unsafe { (libraries.c.driver)(0, 10) },
        || unsafe { (libraries.rust.driver)(0, 10) },
    );
    assert!(
        !composed
            .windows(b"ERROR:".len())
            .any(|window| window == b"ERROR:")
    );
}
