use libloading::Library;
use std::ffi::{CString, c_char, c_int, c_void};
use std::fmt::Debug;
use std::fs::{File, OpenOptions};
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

type Forward = unsafe extern "C" fn(c_int) -> c_int;
type OpenWithCleanup = unsafe extern "C" fn(*const c_char) -> *mut c_void;
type Driver = unsafe extern "C" fn(c_int, *const c_char) -> c_int;

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fclose(stream: *mut c_void) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

struct Api {
    _library: Library,
    forward: Forward,
    open: OpenWithCleanup,
    driver: Driver,
}

impl Api {
    fn load(path: &Path) -> Self {
        // SAFETY: the library paths and symbol signatures come from the C ABI.
        unsafe {
            let library = Library::new(path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
            let forward = *library
                .get::<Forward>(b"forward_goto_example\0")
                .expect("missing forward_goto_example");
            let open = *library
                .get::<OpenWithCleanup>(b"open_with_cleanup\0")
                .expect("missing open_with_cleanup");
            let driver = *library.get::<Driver>(b"driver\0").expect("missing driver");
            Self {
                _library: library,
                forward,
                open,
                driver,
            }
        }
    }
}

struct Apis {
    c: Api,
    rust: Api,
}

impl Apis {
    fn load() -> Self {
        Self {
            c: Api::load(&c_library_path()),
            rust: Api::load(&rust_library_path()),
        }
    }
}

fn c_library_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate must have workspace parent")
        .join("c_src/build/libdriver.so")
}

fn rust_library_path() -> PathBuf {
    if let Some(path) = std::env::var_os("RUST_DRIVER_SO") {
        let path = PathBuf::from(path);
        assert!(path.is_file(), "RUST_DRIVER_SO does not name a file");
        return path;
    }

    let executable = std::env::current_exe().expect("test executable path");
    let deps = executable.parent().expect("test executable parent");
    let profile = deps.parent().expect("Cargo profile directory");
    let manifest_target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    for candidate in [
        profile.join("libdriver.so"),
        deps.join("libdriver.so"),
        manifest_target.join("release/libdriver.so"),
    ] {
        if candidate.is_file() {
            return candidate;
        }
    }
    panic!(
        "Rust cdylib not found under {}, {}, or target/release",
        profile.display(),
        deps.display()
    );
}

#[derive(Debug, PartialEq, Eq)]
struct Captured<T> {
    value: T,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

static CAPTURE_LOCK: Mutex<()> = Mutex::new(());
static NEXT_PATH: AtomicU64 = AtomicU64::new(0);

struct Redirect {
    saved_stdout: c_int,
    saved_stderr: c_int,
}

impl Redirect {
    fn install(stdout: &File, stderr: &File) -> Self {
        // SAFETY: these calls operate on valid process file descriptors.
        unsafe {
            fflush(ptr::null_mut());
            let saved_stdout = dup(1);
            let saved_stderr = dup(2);
            assert!(saved_stdout >= 0 && saved_stderr >= 0, "dup failed");
            assert_eq!(dup2(stdout.as_raw_fd(), 1), 1, "stdout dup2 failed");
            assert_eq!(dup2(stderr.as_raw_fd(), 2), 2, "stderr dup2 failed");
            Self {
                saved_stdout,
                saved_stderr,
            }
        }
    }
}

impl Drop for Redirect {
    fn drop(&mut self) {
        // SAFETY: saved descriptors were created by dup and are owned here.
        unsafe {
            fflush(ptr::null_mut());
            assert_eq!(dup2(self.saved_stdout, 1), 1, "restore stdout failed");
            assert_eq!(dup2(self.saved_stderr, 2), 2, "restore stderr failed");
            close(self.saved_stdout);
            close(self.saved_stderr);
        }
    }
}

fn capture<T>(operation: impl FnOnce() -> T) -> Captured<T> {
    let _lock = CAPTURE_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let stdout_path = unique_path("stdout");
    let stderr_path = unique_path("stderr");
    let stdout_file = capture_file(&stdout_path);
    let stderr_file = capture_file(&stderr_path);

    let redirect = Redirect::install(&stdout_file, &stderr_file);
    let value = operation();
    drop(redirect);
    drop(stdout_file);
    drop(stderr_file);

    let stdout = std::fs::read(&stdout_path).expect("read captured stdout");
    let stderr = std::fs::read(&stderr_path).expect("read captured stderr");
    std::fs::remove_file(stdout_path).expect("remove captured stdout");
    std::fs::remove_file(stderr_path).expect("remove captured stderr");
    Captured {
        value,
        stdout,
        stderr,
    }
}

fn capture_file(path: &Path) -> File {
    OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(path)
        .expect("create capture file")
}

fn unique_path(kind: &str) -> PathBuf {
    let id = NEXT_PATH.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "goto-differential-{}-{}-{kind}",
        std::process::id(),
        id
    ))
}

struct InputPath {
    path: PathBuf,
    directory: bool,
}

impl InputPath {
    fn file(bytes: &[u8]) -> Self {
        let path = unique_path("input");
        std::fs::write(&path, bytes).expect("write input file");
        Self {
            path,
            directory: false,
        }
    }

    fn directory() -> Self {
        let path = unique_path("directory");
        std::fs::create_dir(&path).expect("create input directory");
        Self {
            path,
            directory: true,
        }
    }

    fn missing() -> Self {
        let path = unique_path("missing");
        assert!(!path.exists());
        Self {
            path,
            directory: false,
        }
    }

    fn c_string(&self) -> CString {
        CString::new(self.path.as_os_str().as_bytes()).expect("path contains NUL")
    }
}

impl Drop for InputPath {
    fn drop(&mut self) {
        if self.directory {
            let _ = std::fs::remove_dir(&self.path);
        } else {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct OpenResult {
    is_null: bool,
    close_result: Option<c_int>,
}

fn call_forward(function: Forward, x: c_int) -> Captured<c_int> {
    capture(|| {
        // SAFETY: the function pointer was loaded with the exact C signature.
        unsafe { function(x) }
    })
}

fn call_open(function: OpenWithCleanup, filename: *const c_char) -> Captured<OpenResult> {
    capture(|| {
        // SAFETY: the function pointer and filename follow the exported C ABI.
        let stream = unsafe { function(filename) };
        if stream.is_null() {
            OpenResult {
                is_null: true,
                close_result: None,
            }
        } else {
            // SAFETY: a non-null success return is an open FILE* owned by the caller.
            let result = unsafe { fclose(stream) };
            OpenResult {
                is_null: false,
                close_result: Some(result),
            }
        }
    })
}

fn call_driver(function: Driver, num: c_int, filename: *const c_char) -> Captured<c_int> {
    capture(|| {
        // SAFETY: the function pointer and arguments follow the exported C ABI.
        unsafe { function(num, filename) }
    })
}

fn assert_differential<T: Debug + PartialEq>(c: Captured<T>, rust: Captured<T>) -> Captured<T> {
    assert_eq!(rust, c);
    c
}

fn compare_forward(apis: &Apis, x: c_int) -> Captured<c_int> {
    assert_differential(
        call_forward(apis.c.forward, x),
        call_forward(apis.rust.forward, x),
    )
}

fn compare_open(apis: &Apis, filename: *const c_char) -> Captured<OpenResult> {
    assert_differential(
        call_open(apis.c.open, filename),
        call_open(apis.rust.open, filename),
    )
}

fn compare_driver(apis: &Apis, num: c_int, filename: *const c_char) -> Captured<c_int> {
    assert_differential(
        call_driver(apis.c.driver, num, filename),
        call_driver(apis.rust.driver, num, filename),
    )
}

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x as u32
    }

    fn range(&mut self, start: usize, end: usize) -> usize {
        start + (self.next_u32() as usize % (end - start))
    }

    fn printable(&mut self) -> u8 {
        b'!' + (self.next_u32() % 90) as u8
    }
}

fn random_bytes(rng: &mut Rng, length: usize) -> Vec<u8> {
    (0..length).map(|_| rng.printable()).collect()
}

fn valid_num(rng: &mut Rng, iteration: usize) -> c_int {
    const BOUNDARIES: [c_int; 6] = [0, 1, 2, (c_int::MAX / 2), (c_int::MAX / 2) + 1, c_int::MAX];
    BOUNDARIES
        .get(iteration)
        .copied()
        .unwrap_or_else(|| (rng.next_u32() & 0x7fff_ffff) as c_int)
}

fn verify_open_shape(seed: u64, mut make_bytes: impl FnMut(&mut Rng) -> Vec<u8>) {
    let apis = Apis::load();
    let mut rng = Rng::new(seed);
    for _ in 0..32 {
        let input = InputPath::file(&make_bytes(&mut rng));
        let filename = input.c_string();
        let result = compare_open(&apis, filename.as_ptr());
        assert_eq!(
            result.value,
            OpenResult {
                is_null: false,
                close_result: Some(0)
            }
        );
        assert!(result.stderr.is_empty());
    }
}

fn verify_driver_shape(seed: u64, mut make_bytes: impl FnMut(&mut Rng) -> Vec<u8>) {
    let apis = Apis::load();
    let mut rng = Rng::new(seed);
    for iteration in 0..32 {
        let input = InputPath::file(&make_bytes(&mut rng));
        let filename = input.c_string();
        let result = compare_driver(&apis, valid_num(&mut rng, iteration), filename.as_ptr());
        assert_eq!(result.value, 0);
        assert!(result.stderr.is_empty());
    }
}

fn short_newline(rng: &mut Rng) -> Vec<u8> {
    let length = rng.range(0, 98);
    let mut bytes = random_bytes(rng, length);
    bytes.push(b'\n');
    bytes
}

fn short_final(rng: &mut Rng) -> Vec<u8> {
    let length = rng.range(1, 99);
    random_bytes(rng, length)
}

fn multiple_lines(rng: &mut Rng) -> Vec<u8> {
    let count = rng.range(2, 7);
    let mut bytes = Vec::new();
    for _ in 0..count {
        let length = rng.range(0, 24);
        bytes.extend(random_bytes(rng, length));
        bytes.push(b'\n');
    }
    bytes
}

fn exactly_99(rng: &mut Rng) -> Vec<u8> {
    random_bytes(rng, 99)
}

fn long_line(rng: &mut Rng) -> Vec<u8> {
    let length = rng.range(100, 401);
    random_bytes(rng, length)
}

fn embedded_nul(rng: &mut Rng) -> Vec<u8> {
    let prefix = rng.range(1, 24);
    let suffix = rng.range(1, 24);
    let mut bytes = random_bytes(rng, prefix);
    bytes.push(0);
    bytes.extend(random_bytes(rng, suffix));
    bytes.push(b'\n');
    bytes
}

#[test]
fn config_01_forward_valid_integer_branch() {
    let apis = Apis::load();
    let mut rng = Rng::new(0x3a18_462e_72c9_01d5);
    for iteration in 0..256 {
        let x = valid_num(&mut rng, iteration);
        let result = compare_forward(&apis, x);
        assert_eq!(result.value, x.wrapping_mul(2));
        assert!(result.stderr.is_empty());
    }
}

#[test]
fn config_02_open_empty_file() {
    verify_open_shape(0x02, |_| Vec::new());
}

#[test]
fn config_03_open_short_newline_line() {
    verify_open_shape(0x03, short_newline);
}

#[test]
fn config_04_open_short_final_line() {
    verify_open_shape(0x04, short_final);
}

#[test]
fn config_05_open_multiple_lines() {
    verify_open_shape(0x05, multiple_lines);
}

#[test]
fn config_06_open_exactly_99_bytes() {
    verify_open_shape(0x06, exactly_99);
}

#[test]
fn config_07_open_long_line() {
    verify_open_shape(0x07, long_line);
}

#[test]
fn config_08_open_embedded_nul() {
    verify_open_shape(0x08, embedded_nul);
}

#[test]
fn config_09_driver_empty_file() {
    verify_driver_shape(0x09, |_| Vec::new());
}

#[test]
fn config_10_driver_short_newline_line() {
    verify_driver_shape(0x10, short_newline);
}

#[test]
fn config_11_driver_short_final_line() {
    verify_driver_shape(0x11, short_final);
}

#[test]
fn config_12_driver_multiple_lines() {
    verify_driver_shape(0x12, multiple_lines);
}

#[test]
fn config_13_driver_exactly_99_bytes() {
    verify_driver_shape(0x13, exactly_99);
}

#[test]
fn config_14_driver_long_line() {
    verify_driver_shape(0x14, long_line);
}

#[test]
fn config_15_driver_embedded_nul() {
    verify_driver_shape(0x15, embedded_nul);
}

#[test]
fn error_01_forward_negative() {
    let apis = Apis::load();
    let mut rng = Rng::new(0xe1);
    let mut values = vec![-1, c_int::MIN];
    values.extend((0..64).map(|_| -1 - (rng.next_u32() & 0x7fff_fffe) as c_int));
    for x in values {
        let result = compare_forward(&apis, x);
        assert_eq!(result.value, -1);
        assert!(result.stdout.is_empty());
        assert_eq!(result.stderr, b"Error: negative input\n");
    }
}

#[test]
fn error_02_open_fopen_failure() {
    let apis = Apis::load();
    for _ in 0..32 {
        let input = InputPath::missing();
        let filename = input.c_string();
        let result = compare_open(&apis, filename.as_ptr());
        assert!(result.value.is_null);
        assert!(result.stdout.is_empty());
        assert!(
            result
                .stderr
                .starts_with(b"Error: opening or processing file ")
        );
    }
}

#[test]
fn error_03_open_ferror_cleanup() {
    let apis = Apis::load();
    for _ in 0..16 {
        let input = InputPath::directory();
        let filename = input.c_string();
        let result = compare_open(&apis, filename.as_ptr());
        assert!(result.value.is_null);
        assert!(result.stdout.is_empty());
        assert!(
            result
                .stderr
                .starts_with(b"Error: opening or processing file ")
        );
    }
}

#[test]
fn error_04_driver_propagates_negative() {
    let apis = Apis::load();
    let mut rng = Rng::new(0xe4);
    for _ in 0..64 {
        let num = -1 - (rng.next_u32() & 0x7fff_fffe) as c_int;
        let result = compare_driver(&apis, num, ptr::null());
        assert_eq!(result.value, -1);
        assert!(result.stdout.is_empty());
        assert_eq!(result.stderr, b"Error: negative input\n");
    }
}

#[test]
fn error_05_driver_fopen_failure() {
    let apis = Apis::load();
    let mut rng = Rng::new(0xe5);
    for iteration in 0..32 {
        let input = InputPath::missing();
        let filename = input.c_string();
        let result = compare_driver(&apis, valid_num(&mut rng, iteration), filename.as_ptr());
        assert_eq!(result.value, -2);
        assert!(
            result
                .stderr
                .starts_with(b"Error: opening or processing file ")
        );
    }
}

#[test]
fn error_06_driver_ferror_cleanup() {
    let apis = Apis::load();
    let mut rng = Rng::new(0xe6);
    for iteration in 0..16 {
        let input = InputPath::directory();
        let filename = input.c_string();
        let result = compare_driver(&apis, valid_num(&mut rng, iteration), filename.as_ptr());
        assert_eq!(result.value, -2);
        assert!(
            result
                .stderr
                .starts_with(b"Error: opening or processing file ")
        );
    }
}

#[test]
fn error_07_open_null_filename() {
    let apis = Apis::load();
    let result = compare_open(&apis, ptr::null());
    assert!(result.value.is_null);
    assert!(result.stdout.is_empty());
    assert_eq!(result.stderr, b"Error: opening or processing file (null)\n");
}

#[test]
fn error_08_driver_null_filename() {
    let apis = Apis::load();
    let result = compare_driver(&apis, 0, ptr::null());
    assert_eq!(result.value, -2);
    assert_eq!(result.stdout, b"Processing: 0\nGoto output: 0\n");
    assert_eq!(result.stderr, b"Error: opening or processing file (null)\n");
}
