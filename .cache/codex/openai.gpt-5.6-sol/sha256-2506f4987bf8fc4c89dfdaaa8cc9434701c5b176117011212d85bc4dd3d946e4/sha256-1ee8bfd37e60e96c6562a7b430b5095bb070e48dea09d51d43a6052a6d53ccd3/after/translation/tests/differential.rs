use libloading::Library;
use std::env;
use std::ffi::{c_int, c_void};
use std::fs;
use std::os::fd::RawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

type StaticAlias = unsafe extern "C" fn(*mut c_int) -> *mut c_int;
type Driver = unsafe extern "C" fn(c_int, c_int);

const RANDOM_CASES: usize = 32;
static NEXT_CASE: AtomicU64 = AtomicU64::new(0);
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: RawFd) -> c_int;
    fn dup(fd: RawFd) -> RawFd;
    fn dup2(old_fd: RawFd, new_fd: RawFd) -> RawFd;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(fds: *mut RawFd) -> c_int;
    fn read(fd: RawFd, buffer: *mut c_void, count: usize) -> isize;
}

#[derive(Debug, Eq, PartialEq)]
struct AliasObservation {
    caller_bytes: [u8; 4],
    returned_bytes: [u8; 4],
    returned_caller: bool,
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

    fn range(&mut self, start: c_int, end: c_int) -> c_int {
        assert!(start < end);
        start + (self.next_u32() % (end - start) as u32) as c_int
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("../c_src/build/libStaticAlias.so")
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/release/libStaticAlias.so")
}

fn with_fresh_pair<T>(run: impl FnOnce(&Library, &Library) -> T) -> T {
    let case = NEXT_CASE.fetch_add(1, Ordering::Relaxed);
    let directory = env::temp_dir().join(format!(
        "staticalias-differential-{}-{case}",
        std::process::id()
    ));
    fs::create_dir(&directory).expect("create per-case library directory");

    let c_copy = directory.join("libStaticAlias_c.so");
    let rust_copy = directory.join("libStaticAlias_rust.so");
    fs::copy(c_library_path(), &c_copy).expect("copy C shared library");
    fs::copy(rust_library_path(), &rust_copy).expect("copy Rust shared library");

    let c_library = unsafe { Library::new(&c_copy) }.expect("load C shared library");
    let rust_library = unsafe { Library::new(&rust_copy) }.expect("load Rust shared library");
    let result = run(&c_library, &rust_library);

    drop(rust_library);
    drop(c_library);
    fs::remove_dir_all(directory).expect("remove per-case library directory");
    result
}

fn observe_alias(library: &Library, caller: &mut c_int) -> AliasObservation {
    let function = unsafe {
        library
            .get::<StaticAlias>(b"static_alias\0")
            .expect("resolve static_alias")
    };
    let caller_pointer = ptr::from_mut(caller);
    let returned = unsafe { function(caller_pointer) };
    assert!(!returned.is_null(), "valid static_alias call returned null");

    AliasObservation {
        caller_bytes: caller.to_ne_bytes(),
        returned_bytes: unsafe { returned.read() }.to_ne_bytes(),
        returned_caller: returned == caller_pointer,
    }
}

fn compare_alias_once(c_library: &Library, rust_library: &Library, value: c_int) {
    let mut c_value = value;
    let mut rust_value = value;
    let c_observation = observe_alias(c_library, &mut c_value);
    let rust_observation = observe_alias(rust_library, &mut rust_value);
    assert_eq!(c_observation, rust_observation);
}

fn capture_driver(library: &Library, initial_value: c_int, iterations: c_int) -> Vec<u8> {
    let function = unsafe { library.get::<Driver>(b"driver\0").expect("resolve driver") };
    let _guard = STDOUT_LOCK.lock().expect("lock stdout redirection");
    let mut pipe_fds = [-1; 2];

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0);
    }
    let saved_stdout = unsafe { dup(1) };
    assert!(saved_stdout >= 0);
    assert_eq!(unsafe { dup2(pipe_fds[1], 1) }, 1);
    assert_eq!(unsafe { close(pipe_fds[1]) }, 0);

    unsafe {
        function(initial_value, iterations);
        assert_eq!(fflush(ptr::null_mut()), 0);
    }

    assert_eq!(unsafe { dup2(saved_stdout, 1) }, 1);
    assert_eq!(unsafe { close(saved_stdout) }, 0);

    let mut output = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        let count = unsafe { read(pipe_fds[0], buffer.as_mut_ptr().cast(), buffer.len()) };
        assert!(count >= 0);
        if count == 0 {
            break;
        }
        output.extend_from_slice(&buffer[..count as usize]);
    }
    assert_eq!(unsafe { close(pipe_fds[0]) }, 0);
    output
}

fn compare_driver(
    c_library: &Library,
    rust_library: &Library,
    initial_value: c_int,
    iterations: c_int,
) {
    let c_output = capture_driver(c_library, initial_value, iterations);
    let rust_output = capture_driver(rust_library, initial_value, iterations);
    assert_eq!(c_output, rust_output);

    // The output is public behavior, while this mirrored call also checks the
    // otherwise-hidden static state left behind by the full driver pipeline.
    compare_alias_once(c_library, rust_library, -10_000);
}

fn prepare_inner(c_library: &Library, rust_library: &Library, value: c_int) -> c_int {
    assert!(value >= 1);
    compare_alias_once(c_library, rust_library, value);
    1 + value
}

#[test]
fn config_01_caller_value_less_than_inner() {
    let mut rng = Rng::new(0x0101_5eed);
    for _ in 0..RANDOM_CASES {
        let value = rng.range(-1_000_000, 1);
        with_fresh_pair(|c, rust| compare_alias_once(c, rust, value));
    }
}

#[test]
fn config_02_caller_value_equal_to_inner() {
    let mut rng = Rng::new(0x0202_5eed);
    for _ in 0..RANDOM_CASES {
        let preparation = rng.range(1, 100_000);
        with_fresh_pair(|c, rust| {
            let inner = prepare_inner(c, rust, preparation);
            compare_alias_once(c, rust, inner);
        });
    }
}

#[test]
fn config_03_caller_value_greater_than_inner() {
    let mut rng = Rng::new(0x0303_5eed);
    for _ in 0..RANDOM_CASES {
        let preparation = rng.range(1, 100_000);
        let delta = rng.range(1, 100_000);
        with_fresh_pair(|c, rust| {
            let inner = prepare_inner(c, rust, preparation);
            compare_alias_once(c, rust, inner + delta);
        });
    }
}

#[test]
fn config_04_reuse_returned_caller_pointer() {
    let mut rng = Rng::new(0x0404_5eed);
    for _ in 0..RANDOM_CASES {
        let value = rng.range(-100_000, 1);
        with_fresh_pair(|c, rust| {
            let c_function = unsafe { c.get::<StaticAlias>(b"static_alias\0").unwrap() };
            let rust_function = unsafe { rust.get::<StaticAlias>(b"static_alias\0").unwrap() };
            let mut c_value = value;
            let mut rust_value = value;
            let c_returned = unsafe { c_function(ptr::from_mut(&mut c_value)) };
            let rust_returned = unsafe { rust_function(ptr::from_mut(&mut rust_value)) };
            assert_eq!(c_returned, ptr::from_mut(&mut c_value));
            assert_eq!(rust_returned, ptr::from_mut(&mut rust_value));
            assert_eq!(c_value.to_ne_bytes(), rust_value.to_ne_bytes());

            let c_second = unsafe { c_function(c_returned) };
            let rust_second = unsafe { rust_function(rust_returned) };
            assert_eq!(c_value.to_ne_bytes(), rust_value.to_ne_bytes());
            assert_eq!(
                unsafe { c_second.read() }.to_ne_bytes(),
                unsafe { rust_second.read() }.to_ne_bytes()
            );
            assert_eq!(c_second == c_returned, rust_second == rust_returned);
        });
    }
}

#[test]
fn config_06_negative_iterations() {
    let mut rng = Rng::new(0x0606_5eed);
    for _ in 0..RANDOM_CASES {
        let initial = rng.next_u32() as c_int;
        let iterations = -rng.range(1, 1_000);
        with_fresh_pair(|c, rust| compare_driver(c, rust, initial, iterations));
    }
}

#[test]
fn config_07_zero_iterations() {
    let mut rng = Rng::new(0x0707_5eed);
    for _ in 0..RANDOM_CASES {
        let initial = rng.next_u32() as c_int;
        with_fresh_pair(|c, rust| compare_driver(c, rust, initial, 0));
    }
}

#[test]
fn config_08_one_iteration_initial_less_than_inner() {
    let mut rng = Rng::new(0x0808_5eed);
    for _ in 0..RANDOM_CASES {
        let initial = rng.range(-100_000, 1);
        with_fresh_pair(|c, rust| compare_driver(c, rust, initial, 1));
    }
}

#[test]
fn config_09_one_iteration_initial_equal_to_inner() {
    let mut rng = Rng::new(0x0909_5eed);
    for _ in 0..RANDOM_CASES {
        let preparation = rng.range(1, 100_000);
        with_fresh_pair(|c, rust| {
            let inner = prepare_inner(c, rust, preparation);
            compare_driver(c, rust, inner, 1);
        });
    }
}

#[test]
fn config_10_one_iteration_initial_greater_than_inner() {
    let mut rng = Rng::new(0x1010_5eed);
    for _ in 0..RANDOM_CASES {
        let initial = rng.range(2, 100_000);
        with_fresh_pair(|c, rust| compare_driver(c, rust, initial, 1));
    }
}

#[test]
fn config_11_many_iterations_initial_less_than_inner() {
    let mut rng = Rng::new(0x1111_5eed);
    for _ in 0..RANDOM_CASES {
        let initial = rng.range(-1_000, 1);
        let iterations = rng.range(2, 17);
        with_fresh_pair(|c, rust| compare_driver(c, rust, initial, iterations));
    }
}

#[test]
fn config_12_many_iterations_initial_equal_to_inner() {
    let mut rng = Rng::new(0x1212_5eed);
    for _ in 0..RANDOM_CASES {
        let preparation = rng.range(1, 10_000);
        let iterations = rng.range(2, 13);
        with_fresh_pair(|c, rust| {
            let inner = prepare_inner(c, rust, preparation);
            compare_driver(c, rust, inner, iterations);
        });
    }
}

#[test]
fn config_13_many_iterations_initial_greater_than_inner() {
    let mut rng = Rng::new(0x1313_5eed);
    for _ in 0..RANDOM_CASES {
        let initial = rng.range(2, 10_000);
        let iterations = rng.range(2, 13);
        with_fresh_pair(|c, rust| compare_driver(c, rust, initial, iterations));
    }
}

#[test]
fn error_01_null_outer_matches_process_rejection() {
    let executable = env::current_exe().expect("locate integration test executable");
    let run_child = |library: &Path| {
        Command::new(&executable)
            .arg("--exact")
            .arg("null_pointer_child")
            .arg("--nocapture")
            .env("STATIC_ALIAS_NULL_LIBRARY", library)
            .status()
            .expect("run null-pointer child")
    };

    let c_status = run_child(&c_library_path());
    let rust_status = run_child(&rust_library_path());
    assert_eq!(c_status.signal(), Some(11), "C null call did not SIGSEGV");
    assert_eq!(
        rust_status.signal(),
        c_status.signal(),
        "Rust null call must terminate with the same signal as C"
    );
}

#[test]
fn null_pointer_child() {
    let Some(path) = env::var_os("STATIC_ALIAS_NULL_LIBRARY") else {
        return;
    };
    let library = unsafe { Library::new(path) }.expect("load null-test library");
    let function = unsafe {
        library
            .get::<StaticAlias>(b"static_alias\0")
            .expect("resolve static_alias")
    };
    unsafe {
        function(ptr::null_mut());
    }
}

#[test]
fn config_05_feed_returned_inner_pointer_back() {
    let mut rng = Rng::new(0x0505_5eed);
    for _ in 0..RANDOM_CASES {
        let value = rng.range(1, 100_000);
        with_fresh_pair(|c, rust| {
            let c_function = unsafe { c.get::<StaticAlias>(b"static_alias\0").unwrap() };
            let rust_function = unsafe { rust.get::<StaticAlias>(b"static_alias\0").unwrap() };
            let mut c_value = value;
            let mut rust_value = value;
            let c_inner = unsafe { c_function(ptr::from_mut(&mut c_value)) };
            let rust_inner = unsafe { rust_function(ptr::from_mut(&mut rust_value)) };
            assert_ne!(c_inner, ptr::from_mut(&mut c_value));
            assert_ne!(rust_inner, ptr::from_mut(&mut rust_value));

            let c_returned = unsafe { c_function(c_inner) };
            let rust_returned = unsafe { rust_function(rust_inner) };
            assert_eq!(c_returned, c_inner);
            assert_eq!(rust_returned, rust_inner);
            assert_eq!(
                unsafe { c_returned.read() }.to_ne_bytes(),
                unsafe { rust_returned.read() }.to_ne_bytes()
            );
        });
    }
}
