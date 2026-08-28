use libloading::Library;
use std::env;
use std::ffi::{CString, c_char, c_int, c_void};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::ptr;

type Forward = unsafe extern "C" fn(c_int) -> c_int;
type OpenWithCleanup = unsafe extern "C" fn(*const c_char) -> *mut c_void;
type Driver = unsafe extern "C" fn(c_int, *const c_char) -> c_int;

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
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
    unsafe fn load(path: &Path) -> Self {
        // SAFETY: the test controls both shared libraries and validates all symbol types.
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let forward = unsafe { *library.get::<Forward>(b"forward_goto_example\0").unwrap() };
        let open = unsafe {
            *library
                .get::<OpenWithCleanup>(b"open_with_cleanup\0")
                .unwrap()
        };
        let driver = unsafe { *library.get::<Driver>(b"driver\0").unwrap() };
        Self {
            _library: library,
            forward,
            open,
            driver,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct Observation<T> {
    result: T,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

struct FixtureDir(PathBuf);

impl FixtureDir {
    fn new() -> Self {
        let path = env::temp_dir().join(format!(
            "goto-differential-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, bytes).unwrap();
        path
    }

    fn directory(&self, name: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::create_dir(&path).unwrap();
        path
    }
}

impl Drop for FixtureDir {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

struct Rng(u64);

impl Rng {
    fn new() -> Self {
        Self(0x4d59_5df4_d0f3_3173)
    }

    fn next_u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value as u32
    }

    fn range(&mut self, low: usize, high_inclusive: usize) -> usize {
        low + (self.next_u32() as usize % (high_inclusive - low + 1))
    }
}

fn shared_objects() -> (PathBuf, PathBuf) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_library = manifest.join("../c_src/build/libdriver.so");
    let rust_library = manifest.join("target/release/libdriver.so");
    assert!(
        c_library.is_file(),
        "C shared library is missing: {}",
        c_library.display()
    );
    assert!(
        rust_library.is_file(),
        "Rust shared library is missing (run `cargo build --release`): {}",
        rust_library.display()
    );
    (c_library, rust_library)
}

fn c_path(path: &Path) -> CString {
    CString::new(path.as_os_str().as_encoded_bytes()).unwrap()
}

fn temporary_capture_file(label: &str) -> File {
    let path = env::temp_dir().join(format!(
        "goto-capture-{}-{}-{}",
        std::process::id(),
        label,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    fs::remove_file(path).unwrap();
    file
}

fn capture<T>(operation: impl FnOnce() -> T) -> Observation<T> {
    let mut stdout_file = temporary_capture_file("stdout");
    let mut stderr_file = temporary_capture_file("stderr");

    // SAFETY: all descriptors are checked and restored before this function returns.
    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        let saved_stdout = dup(1);
        let saved_stderr = dup(2);
        assert!(saved_stdout >= 0);
        assert!(saved_stderr >= 0);
        assert_eq!(dup2(stdout_file.as_raw_fd(), 1), 1);
        assert_eq!(dup2(stderr_file.as_raw_fd(), 2), 2);

        let result = operation();

        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(dup2(saved_stderr, 2), 2);
        assert_eq!(close(saved_stdout), 0);
        assert_eq!(close(saved_stderr), 0);

        stdout_file.seek(SeekFrom::Start(0)).unwrap();
        stderr_file.seek(SeekFrom::Start(0)).unwrap();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        stdout_file.read_to_end(&mut stdout).unwrap();
        stderr_file.read_to_end(&mut stderr).unwrap();
        Observation {
            result,
            stdout,
            stderr,
        }
    }
}

fn observe_forward(api: &Api, value: i32) -> Observation<i32> {
    capture(|| {
        // SAFETY: the symbol type matches the C declaration.
        unsafe { (api.forward)(value) }
    })
}

fn observe_open(api: &Api, path: *const c_char) -> Observation<bool> {
    capture(|| {
        // SAFETY: each test supplies either a live C string or the explicit null probe.
        let file = unsafe { (api.open)(path) };
        if file.is_null() {
            false
        } else {
            // SAFETY: successful open_with_cleanup calls return a live FILE pointer.
            assert_eq!(unsafe { fclose(file) }, 0);
            true
        }
    })
}

fn observe_driver(api: &Api, value: i32, path: *const c_char) -> Observation<i32> {
    capture(|| {
        // SAFETY: each test supplies either a live C string or a short-circuited null.
        unsafe { (api.driver)(value, path) }
    })
}

fn assert_forward_matches(c_api: &Api, rust_api: &Api, value: i32, row: usize) {
    assert_eq!(
        observe_forward(c_api, value),
        observe_forward(rust_api, value),
        "CONFIGS/ERRORS row {row}, x={value}"
    );
}

fn assert_open_matches(c_api: &Api, rust_api: &Api, path: &Path, row: usize) {
    let path = c_path(path);
    assert_eq!(
        observe_open(c_api, path.as_ptr()),
        observe_open(rust_api, path.as_ptr()),
        "CONFIGS/ERRORS row {row}, path={path:?}"
    );
}

fn assert_driver_matches(
    c_api: &Api,
    rust_api: &Api,
    value: i32,
    path: *const c_char,
    context: &str,
) {
    assert_eq!(
        observe_driver(c_api, value, path),
        observe_driver(rust_api, value, path),
        "{context}, x={value}"
    );
}

fn random_text(rng: &mut Rng, length: usize) -> Vec<u8> {
    (0..length)
        .map(|_| b' ' + (rng.next_u32() % 95) as u8)
        .collect()
}

fn file_shape(rng: &mut Rng, shape: usize) -> Vec<u8> {
    match shape {
        0 => Vec::new(),
        1 => {
            let length = rng.range(1, 99);
            let mut bytes = random_text(rng, length);
            if rng.next_u32() & 1 == 0 {
                *bytes.last_mut().unwrap() = b'\n';
            }
            bytes
        }
        2 => {
            let length = rng.range(100, 400);
            let mut bytes = random_text(rng, length);
            let newline_count = rng.range(0, 5);
            for _ in 0..newline_count {
                let index = rng.range(0, bytes.len() - 1);
                bytes[index] = b'\n';
            }
            bytes
        }
        3 => {
            let length = rng.range(3, 99);
            let mut bytes = random_text(rng, length);
            let index = rng.range(1, bytes.len() - 1);
            bytes[index] = 0;
            bytes
        }
        _ => unreachable!(),
    }
}

fn positive_value(rng: &mut Rng, class: usize) -> i32 {
    match class {
        0 => 0,
        1 => 1 + (rng.next_u32() % (i32::MAX as u32 / 2)) as i32,
        2 => {
            let offset = rng.next_u32() % (i32::MAX as u32 / 2 + 1);
            i32::MAX / 2 + 1 + offset as i32
        }
        _ => unreachable!(),
    }
}

fn check_valid_rows(c_api: &Api, rust_api: &Api, fixture: &FixtureDir, rng: &mut Rng) {
    const CASES: usize = 32;

    for _ in 0..CASES {
        assert_forward_matches(c_api, rust_api, 0, 1);
        assert_forward_matches(c_api, rust_api, positive_value(rng, 1), 2);
        assert_forward_matches(c_api, rust_api, positive_value(rng, 2), 3);
    }

    for shape in 0..4 {
        let row = 4 + shape;
        for case in 0..CASES {
            let path = fixture.file(
                &format!("open-row-{row}-case-{case}"),
                &file_shape(rng, shape),
            );
            assert_open_matches(c_api, rust_api, &path, row);
        }
    }

    for class in 0..3 {
        for shape in 0..4 {
            let row = 8 + class * 4 + shape;
            for case in 0..CASES {
                let value = positive_value(rng, class);
                let path = fixture.file(
                    &format!("driver-row-{row}-case-{case}"),
                    &file_shape(rng, shape),
                );
                let path = c_path(&path);
                assert_driver_matches(
                    c_api,
                    rust_api,
                    value,
                    path.as_ptr(),
                    &format!("CONFIGS row {row}"),
                );
            }
        }
    }
}

fn check_error_rows(c_api: &Api, rust_api: &Api, fixture: &FixtureDir, rng: &mut Rng) {
    const CASES: usize = 32;

    assert_forward_matches(c_api, rust_api, i32::MIN, 1);
    assert_forward_matches(c_api, rust_api, -1, 1);
    for _ in 0..CASES {
        let value = -1 - (rng.next_u32() % i32::MAX as u32) as i32;
        assert_forward_matches(c_api, rust_api, value, 1);
    }

    for case in 0..CASES {
        let missing = fixture.0.join(format!("missing-{case}-{}", rng.next_u32()));
        assert_open_matches(c_api, rust_api, &missing, 2);
    }

    for case in 0..CASES {
        let directory = fixture.directory(&format!("unreadable-stream-{case}"));
        assert_open_matches(c_api, rust_api, &directory, 3);
    }

    for _ in 0..CASES {
        let value = -1 - (rng.next_u32() % i32::MAX as u32) as i32;
        assert_driver_matches(
            c_api,
            rust_api,
            value,
            ptr::null(),
            "ERRORS row 4 and null short circuit",
        );
    }

    for case in 0..CASES {
        let class = rng.range(0, 2);
        let value = positive_value(rng, class);
        let missing = c_path(
            &fixture
                .0
                .join(format!("driver-missing-{case}-{}", rng.next_u32())),
        );
        assert_driver_matches(c_api, rust_api, value, missing.as_ptr(), "ERRORS row 5");
    }

    for case in 0..CASES {
        let class = rng.range(0, 2);
        let value = positive_value(rng, class);
        let directory = fixture.directory(&format!("driver-unreadable-stream-{case}"));
        let directory = c_path(&directory);
        assert_driver_matches(c_api, rust_api, value, directory.as_ptr(), "ERRORS row 6");
    }

    let empty = CString::new("").unwrap();
    assert_eq!(
        observe_open(c_api, empty.as_ptr()),
        observe_open(rust_api, empty.as_ptr()),
        "generic empty filename boundary"
    );
    assert_driver_matches(
        c_api,
        rust_api,
        i32::MAX,
        empty.as_ptr(),
        "generic INT_MAX and empty filename boundaries",
    );
}

fn null_probe_output(library: &Path, function: &str) -> Output {
    Command::new(env::current_exe().unwrap())
        .arg("--exact")
        .arg("differential_surface")
        .arg("--nocapture")
        .env("GOTO_NULL_PROBE_LIBRARY", library)
        .env("GOTO_NULL_PROBE_FUNCTION", function)
        .output()
        .unwrap()
}

fn run_null_probe() -> ! {
    let library = PathBuf::from(env::var_os("GOTO_NULL_PROBE_LIBRARY").unwrap());
    let function = env::var("GOTO_NULL_PROBE_FUNCTION").unwrap();
    // SAFETY: this process exists solely to isolate the native null-pointer behavior.
    let api = unsafe { Api::load(&library) };
    let result = match function.as_str() {
        "open" => {
            let file = unsafe { (api.open)(ptr::null()) };
            if file.is_null() {
                80
            } else {
                unsafe {
                    fclose(file);
                }
                81
            }
        }
        "driver" => {
            let value = unsafe { (api.driver)(0, ptr::null()) };
            128 + value
        }
        _ => unreachable!(),
    };
    unsafe {
        fflush(ptr::null_mut());
    }
    std::process::exit(result);
}

fn check_null_pointer_boundaries(c_library: &Path, rust_library: &Path) {
    for function in ["open", "driver"] {
        let c_output = null_probe_output(c_library, function);
        let rust_output = null_probe_output(rust_library, function);
        assert_eq!(
            c_output.status, rust_output.status,
            "null {function} process status differs"
        );
        assert_eq!(
            c_output.stdout, rust_output.stdout,
            "null {function} stdout differs"
        );
        assert_eq!(
            c_output.stderr, rust_output.stderr,
            "null {function} stderr differs"
        );
    }
}

#[test]
fn differential_surface() {
    if env::var_os("GOTO_NULL_PROBE_LIBRARY").is_some() {
        run_null_probe();
    }

    let (c_library, rust_library) = shared_objects();
    // SAFETY: paths identify the two libraries under differential test.
    let c_api = unsafe { Api::load(&c_library) };
    let rust_api = unsafe { Api::load(&rust_library) };
    let fixture = FixtureDir::new();
    let mut rng = Rng::new();

    check_valid_rows(&c_api, &rust_api, &fixture, &mut rng);
    check_error_rows(&c_api, &rust_api, &fixture, &mut rng);
    check_null_pointer_boundaries(&c_library, &rust_library);
}
