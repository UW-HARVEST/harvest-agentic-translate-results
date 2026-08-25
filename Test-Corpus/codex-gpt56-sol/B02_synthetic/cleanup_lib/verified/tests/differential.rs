use libloading::Library;
use std::env;
use std::ffi::{CString, c_char, c_int, c_void};
use std::fs::{self, File};
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::Mutex;

type Cleanup = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
type PrintResult = unsafe extern "C" fn(*const c_char, c_int);
type CleanupResources = unsafe extern "C" fn(*mut c_char);
type SetMode = unsafe extern "C" fn(c_int);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn malloc(size: usize) -> *mut c_void;
    fn pipe(fds: *mut c_int) -> c_int;
}

const STDOUT_FILENO: c_int = 1;
const NORMAL_OUTPUT: &[u8] = b"Processed numbers: numbers\n";
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

struct Api {
    _library: Library,
    cleanup: Cleanup,
    print_result: PrintResult,
    cleanup_resources: CleanupResources,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        // SAFETY: The test validates the exact C ABI signatures from lib.c.
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let cleanup = unsafe { *library.get::<Cleanup>(b"cleanup\0").unwrap() };
        let print_result = unsafe { *library.get::<PrintResult>(b"print_result\0").unwrap() };
        let cleanup_resources = unsafe {
            *library
                .get::<CleanupResources>(b"cleanup_resources\0")
                .unwrap()
        };
        Self {
            _library: library,
            cleanup,
            print_result,
            cleanup_resources,
        }
    }
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value as u32
    }

    fn index(&mut self, upper: usize) -> usize {
        self.next_u32() as usize % upper
    }

    fn bounded_i32(&mut self, magnitude: i32) -> i32 {
        (self.next_u32() % (2 * magnitude as u32 + 1)) as i32 - magnitude
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir()
        .join("c_src")
        .join("build")
        .join("libtranslated_rust.so")
}

fn target_dir() -> PathBuf {
    env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir().join("target"))
}

fn rust_library_path() -> PathBuf {
    let path = target_dir().join("debug").join("libcleanup_lib.so");
    assert!(
        path.is_file(),
        "Rust cdylib is missing at {}; build it before testing",
        path.display()
    );
    path
}

fn load_apis() -> (Api, Api) {
    let c_path = c_library_path();
    assert!(
        c_path.is_file(),
        "C shared library is missing at {}; build it before testing",
        c_path.display()
    );
    // SAFETY: Paths refer to the two libraries under differential test.
    unsafe { (Api::load(&c_path), Api::load(&rust_library_path())) }
}

fn capture_stdout<T>(operation: impl FnOnce() -> T) -> (T, Vec<u8>) {
    let _guard = STDOUT_LOCK.lock().unwrap();
    let mut fds = [-1, -1];

    // SAFETY: The file-descriptor calls are checked, and stdout is restored
    // before the read end is consumed.
    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(pipe(fds.as_mut_ptr()), 0);
        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0);
        assert_eq!(dup2(fds[1], STDOUT_FILENO), STDOUT_FILENO);
        assert_eq!(close(fds[1]), 0);

        let result = operation();

        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, STDOUT_FILENO), STDOUT_FILENO);
        assert_eq!(close(saved_stdout), 0);

        let mut output = Vec::new();
        File::from_raw_fd(fds[0]).read_to_end(&mut output).unwrap();
        (result, output)
    }
}

fn observe_cleanup(api: &Api, values: [i32; 4]) -> (i32, Vec<u8>) {
    capture_stdout(|| unsafe { (api.cleanup)(values[0], values[1], values[2], values[3]) })
}

fn compare_cleanup(c_api: &Api, rust_api: &Api, values: [i32; 4]) {
    let c_observed = observe_cleanup(c_api, values);
    let rust_observed = observe_cleanup(rust_api, values);
    assert_eq!(rust_observed, c_observed, "cleanup inputs: {values:?}");
}

fn default_value(rng: &mut Rng) -> i32 {
    loop {
        let value = rng.bounded_i32(1_000_000);
        if ![10, 20, 30, 40].contains(&value) {
            return value;
        }
    }
}

fn run_normal_surface() {
    let (c_api, rust_api) = load_apis();
    let mut rng = Rng::new(0x8f31_42d9_a75c_10e3);

    // CONFIGS #1 and ERRORS #3: null cleanup is a no-op.
    let c_null = capture_stdout(|| unsafe { (c_api.cleanup_resources)(ptr::null_mut()) });
    let rust_null = capture_stdout(|| unsafe { (rust_api.cleanup_resources)(ptr::null_mut()) });
    assert_eq!(rust_null.1, c_null.1);

    // CONFIGS #2: independently allocated non-null pointers are freed.
    for _ in 0..128 {
        let c_pointer = unsafe { malloc(1).cast::<c_char>() };
        let rust_pointer = unsafe { malloc(1).cast::<c_char>() };
        assert!(!c_pointer.is_null() && !rust_pointer.is_null());
        let c_output = capture_stdout(|| unsafe { (c_api.cleanup_resources)(c_pointer) }).1;
        let rust_output =
            capture_stdout(|| unsafe { (rust_api.cleanup_resources)(rust_pointer) }).1;
        assert_eq!(rust_output, c_output);
    }

    // CONFIGS #3: empty label.
    let empty = CString::new("").unwrap();
    for _ in 0..256 {
        let result = rng.next_u32() as i32;
        let c_output = capture_stdout(|| unsafe { (c_api.print_result)(empty.as_ptr(), result) }).1;
        let rust_output =
            capture_stdout(|| unsafe { (rust_api.print_result)(empty.as_ptr(), result) }).1;
        assert_eq!(
            rust_output, c_output,
            "print_result empty label, result={result}"
        );
    }

    // CONFIGS #4: one/many-byte labels.
    for iteration in 0..256 {
        let length = if iteration == 0 {
            1
        } else if iteration == 1 {
            255
        } else {
            1 + rng.index(255)
        };
        let label: Vec<u8> = (0..length).map(|_| b'a' + rng.index(26) as u8).collect();
        let label = CString::new(label).unwrap();
        let result = rng.next_u32() as i32;
        let c_output = capture_stdout(|| unsafe { (c_api.print_result)(label.as_ptr(), result) }).1;
        let rust_output =
            capture_stdout(|| unsafe { (rust_api.print_result)(label.as_ptr(), result) }).1;
        assert_eq!(rust_output, c_output, "print_result iteration={iteration}");
    }

    // Generic pointer boundary: glibc renders a null %s argument as "(null)".
    for result in [i32::MIN, -1, 0, 1, i32::MAX] {
        let c_output = capture_stdout(|| unsafe { (c_api.print_result)(ptr::null(), result) }).1;
        let rust_output =
            capture_stdout(|| unsafe { (rust_api.print_result)(ptr::null(), result) }).1;
        assert_eq!(
            rust_output, c_output,
            "print_result null label, result={result}"
        );
    }

    // CONFIGS #5-#8: one recognized value, randomized position.
    for special in [10, 20, 30, 40] {
        for _ in 0..256 {
            let mut values = [
                default_value(&mut rng),
                default_value(&mut rng),
                default_value(&mut rng),
                default_value(&mut rng),
            ];
            values[rng.index(4)] = special;
            let observed = observe_cleanup(&c_api, values);
            assert_eq!(observed.1, NORMAL_OUTPUT);
            compare_cleanup(&c_api, &rust_api, values);
        }
    }

    // CONFIGS #9: all-default switch arms.
    for _ in 0..512 {
        let values = [
            default_value(&mut rng),
            default_value(&mut rng),
            default_value(&mut rng),
            default_value(&mut rng),
        ];
        compare_cleanup(&c_api, &rust_api, values);
    }

    // CONFIGS #10: multiple recognized values and permutations.
    let recognized = [10, 20, 30, 40];
    for _ in 0..512 {
        let values = [
            recognized[rng.index(4)],
            recognized[rng.index(4)],
            recognized[rng.index(4)],
            recognized[rng.index(4)],
        ];
        compare_cleanup(&c_api, &rust_api, values);
    }

    // CONFIGS #11: integer boundaries without signed overflow in C.
    for _ in 0..256 {
        let boundary = if rng.next_u32() & 1 == 0 {
            i32::MIN
        } else {
            i32::MAX
        };
        let mut values = if boundary == i32::MIN {
            [0, rng.index(1000) as i32, rng.index(1000) as i32, 0]
        } else {
            [0, -(rng.index(1000) as i32), -(rng.index(1000) as i32), 0]
        };
        values[rng.index(4)] = boundary;
        compare_cleanup(&c_api, &rust_api, values);
    }
}

fn compile_interposer() -> PathBuf {
    let output_dir = target_dir().join("differential");
    fs::create_dir_all(&output_dir).unwrap();
    let output = output_dir.join("libcleanup_interpose.so");
    let status = Command::new("cc")
        .args(["-std=c11", "-shared", "-fPIC"])
        .arg(manifest_dir().join("tests").join("interpose.c"))
        .arg("-o")
        .arg(&output)
        .status()
        .expect("failed to execute cc for the test interposer");
    assert!(status.success(), "failed to compile test interposer");
    assert!(output.is_file());
    output
}

fn run_interposed_child(interposer: &Path) {
    let existing = env::var_os("LD_PRELOAD").unwrap_or_default();
    let mut preload = interposer.as_os_str().to_owned();
    if !existing.is_empty() {
        preload.push(":");
        preload.push(existing);
    }

    let output = Command::new(env::current_exe().unwrap())
        .args([
            "--exact",
            "interposed_child",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("DIFFERENTIAL_INTERPOSE_CHILD", "1")
        .env("DIFFERENTIAL_INTERPOSER_PATH", interposer)
        .env("LD_PRELOAD", preload)
        .output()
        .expect("failed to run interposed differential child");
    assert!(
        output.status.success(),
        "interposed child failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn interposed_assertions() {
    let (c_api, rust_api) = load_apis();
    // SAFETY: The preloaded test shim exports this exact function.
    let shim_path = env::var_os("DIFFERENTIAL_INTERPOSER_PATH").unwrap();
    let shim = unsafe { Library::new(shim_path) }.unwrap();
    let set_mode = unsafe { *shim.get::<SetMode>(b"differential_set_mode\0").unwrap() };

    // ERRORS #1: force the compiled-in validation comparison to disagree.
    unsafe { set_mode(1) };
    let c_validation = observe_cleanup(&c_api, [1, 2, 3, 4]);
    unsafe { set_mode(1) };
    let rust_validation = observe_cleanup(&rust_api, [1, 2, 3, 4]);
    assert_eq!(
        c_validation,
        (0, b"Input string validation failed.\n".to_vec())
    );
    assert_eq!(rust_validation, c_validation);

    // ERRORS #2: fail exactly the operation's next 50-byte allocation.
    let values = [10, 20, 30, 40];
    unsafe { set_mode(2) };
    let c_allocation = observe_cleanup(&c_api, values);
    unsafe { set_mode(2) };
    let rust_allocation = observe_cleanup(&rust_api, values);
    assert_eq!(c_allocation, (160, b"Memory allocation failed.\n".to_vec()));
    assert_eq!(rust_allocation, c_allocation);
}

#[test]
fn differential_surface() {
    run_normal_surface();
    run_interposed_child(&compile_interposer());
}

#[test]
fn interposed_child() {
    if env::var_os("DIFFERENTIAL_INTERPOSE_CHILD").is_some() {
        interposed_assertions();
    }
}
