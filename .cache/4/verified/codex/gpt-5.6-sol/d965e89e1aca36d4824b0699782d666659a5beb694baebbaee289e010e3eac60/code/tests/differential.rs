use libloading::Library;
use std::env;
use std::ffi::{c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::ptr;

type StaticSum = unsafe extern "C" fn(c_int) -> c_int;
type Driver = unsafe extern "C" fn(c_int);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
}

struct Api {
    _library: Library,
    static_sum: StaticSum,
    driver: Driver,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let static_sum = unsafe {
            *library
                .get::<StaticSum>(b"static_sum\0")
                .unwrap_or_else(|error| panic!("missing static_sum in {}: {error}", path.display()))
        };
        let driver = unsafe {
            *library
                .get::<Driver>(b"driver\0")
                .unwrap_or_else(|error| panic!("missing driver in {}: {error}", path.display()))
        };

        Self {
            _library: library,
            static_sum,
            driver,
        }
    }
}

struct XorShift64(u64);

impl XorShift64 {
    fn next_i32(&mut self) -> i32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        (value >> 32) as i32
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libStaticLoop.so")
}

fn rust_library_path() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let target = env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest.join("target"));
    let target = if target.is_absolute() {
        target
    } else {
        manifest.join(target)
    };
    target.join("debug").join("libStaticLoop.so")
}

fn load_pair() -> (Api, Api) {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(
        c_path.is_file(),
        "C shared library is missing: {}",
        c_path.display()
    );
    assert!(
        rust_path.is_file(),
        "Rust shared library is missing: {}",
        rust_path.display()
    );

    unsafe { (Api::load(&c_path), Api::load(&rust_path)) }
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    const STDOUT_FILENO: c_int = 1;

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0, "failed to flush stdout");

        let mut pipe_fds = [-1; 2];
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0, "failed to create pipe");
        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0, "failed to duplicate stdout");
        assert_eq!(
            dup2(pipe_fds[1], STDOUT_FILENO),
            STDOUT_FILENO,
            "failed to redirect stdout"
        );
        assert_eq!(close(pipe_fds[1]), 0, "failed to close pipe writer");

        call();

        assert_eq!(fflush(ptr::null_mut()), 0, "failed to flush driver output");
        assert_eq!(
            dup2(saved_stdout, STDOUT_FILENO),
            STDOUT_FILENO,
            "failed to restore stdout"
        );
        assert_eq!(close(saved_stdout), 0, "failed to close saved stdout");

        let mut output = Vec::new();
        File::from_raw_fd(pipe_fds[0])
            .read_to_end(&mut output)
            .expect("failed to read captured stdout");
        output
    }
}

fn assert_sum_equal(c: &Api, rust: &Api, update: i32) {
    let c_result = unsafe { (c.static_sum)(update) };
    let rust_result = unsafe { (rust.static_sum)(update) };
    assert_eq!(
        c_result.to_ne_bytes(),
        rust_result.to_ne_bytes(),
        "static_sum diverged for update {update}"
    );
}

#[test]
fn all_public_entry_points_match_across_the_configuration_surface() {
    let boundaries = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    let mut rng = XorShift64(0x5a17_c9e3_d42b_806f);

    // CONFIGS.md row 1: each value is the first update from a freshly loaded library.
    let mut first_updates = boundaries.to_vec();
    first_updates.extend((0..64).map(|_| rng.next_i32()));
    for update in first_updates {
        let (c, rust) = load_pair();
        assert_sum_equal(&c, &rust, update);
    }

    // CONFIGS.md row 2: mixed updates against a persistent accumulator.
    let (c, rust) = load_pair();
    for update in boundaries
        .into_iter()
        .chain((0..4096).map(|_| rng.next_i32()))
    {
        assert_sum_equal(&c, &rust, update);
    }

    // CONFIGS.md row 3: driver composes ten static_sum calls and prints each result.
    let driver_boundaries = [i32::MIN, -1, 0, 1, i32::MAX];
    let strides = driver_boundaries
        .into_iter()
        .chain((0..128).map(|_| rng.next_i32()));
    for stride in strides {
        let c_output = capture_stdout(|| unsafe { (c.driver)(stride) });
        let rust_output = capture_stdout(|| unsafe { (rust.driver)(stride) });
        assert_eq!(
            c_output, rust_output,
            "driver output diverged for stride {stride}"
        );
        assert_sum_equal(&c, &rust, 0);
    }
}
