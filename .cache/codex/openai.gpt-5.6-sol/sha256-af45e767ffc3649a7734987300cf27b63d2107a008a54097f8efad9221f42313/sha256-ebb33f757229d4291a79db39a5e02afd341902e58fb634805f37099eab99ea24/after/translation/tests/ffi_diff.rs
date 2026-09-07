use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

type Driver = unsafe extern "C" fn(c_int, c_int, c_int);

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipefd: *mut c_int) -> c_int;
}

fn library_paths() -> (PathBuf, PathBuf) {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_library = crate_dir.join("../c_src/build/libdriver.so");
    let rust_library = crate_dir.join("target/release/libdriver.so");
    (c_library, rust_library)
}

fn load_library(path: &Path) -> Library {
    assert!(
        path.is_file(),
        "shared library does not exist: {}",
        path.display()
    );
    unsafe { Library::new(path) }.unwrap_or_else(|error| {
        panic!("failed to load {}: {error}", path.display());
    })
}

fn capture_stdout(function: &Symbol<'_, Driver>, x: i32, y: i32, z: i32) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().unwrap();

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);

        let mut pipe_fds = [-1; 2];
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0);

        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0);
        assert_eq!(dup2(pipe_fds[1], 1), 1);
        assert_eq!(close(pipe_fds[1]), 0);

        function(x, y, z);
        assert_eq!(fflush(std::ptr::null_mut()), 0);

        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(close(saved_stdout), 0);

        let mut output = Vec::new();
        let mut reader = File::from_raw_fd(pipe_fds[0]);
        reader.read_to_end(&mut output).unwrap();
        output
    }
}

fn with_drivers(test: impl FnOnce(&Symbol<'_, Driver>, &Symbol<'_, Driver>)) {
    let (c_path, rust_path) = library_paths();
    let c_library = load_library(&c_path);
    let rust_library = load_library(&rust_path);

    unsafe {
        let c_driver: Symbol<'_, Driver> = c_library.get(b"driver\0").unwrap();
        let rust_driver: Symbol<'_, Driver> = rust_library.get(b"driver\0").unwrap();
        test(&c_driver, &rust_driver);
    }
}

fn compare_case(
    row: &str,
    c_driver: &Symbol<'_, Driver>,
    rust_driver: &Symbol<'_, Driver>,
    x: i32,
    y: i32,
    z: i32,
) -> Vec<u8> {
    let c_output = capture_stdout(c_driver, x, y, z);
    let rust_output = capture_stdout(rust_driver, x, y, z);
    assert_eq!(
        rust_output, c_output,
        "{row} diverged for driver({x}, {y}, {z})"
    );
    c_output
}

#[derive(Clone, Copy)]
struct FixedRng(u64);

impl FixedRng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_i32(&mut self) -> i32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 32) as i32
    }
}

#[test]
fn config_1_x_not_one_randomized() {
    with_drivers(|c_driver, rust_driver| {
        let mut rng = FixedRng::new(0x19c5_6d74_8a21_f003);
        let edge_x = [i32::MIN, -1, 0, 2, i32::MAX];

        for index in 0..256 {
            let mut x = if index < edge_x.len() {
                edge_x[index]
            } else {
                rng.next_i32()
            };
            if x == 1 {
                x = 0;
            }
            compare_case(
                "CONFIGS.md row 1",
                c_driver,
                rust_driver,
                x,
                rng.next_i32(),
                rng.next_i32(),
            );
        }
    });
}

#[test]
fn config_2_x_one_y_not_two_randomized() {
    with_drivers(|c_driver, rust_driver| {
        let mut rng = FixedRng::new(0xc1a0_77ed_52b4_9106);
        let edge_y = [i32::MIN, -1, 0, 1, 3, i32::MAX];

        for index in 0..256 {
            let mut y = if index < edge_y.len() {
                edge_y[index]
            } else {
                rng.next_i32()
            };
            if y == 2 {
                y = 0;
            }
            compare_case(
                "CONFIGS.md row 2",
                c_driver,
                rust_driver,
                1,
                y,
                rng.next_i32(),
            );
        }
    });
}

#[test]
fn config_3_x_one_y_two_z_not_three_randomized() {
    with_drivers(|c_driver, rust_driver| {
        let mut rng = FixedRng::new(0xa4ef_38d2_77c9_0b15);
        let edge_z = [i32::MIN, -1, 0, 1, 2, 4, i32::MAX];

        for index in 0..256 {
            let mut z = if index < edge_z.len() {
                edge_z[index]
            } else {
                rng.next_i32()
            };
            if z == 3 {
                z = 0;
            }
            compare_case("CONFIGS.md row 3", c_driver, rust_driver, 1, 2, z);
        }
    });
}

#[test]
fn config_4_success_singleton() {
    with_drivers(|c_driver, rust_driver| {
        let output = compare_case("CONFIGS.md row 4", c_driver, rust_driver, 1, 2, 3);
        assert_eq!(output, b"Ok!\nResult: 0\n");
    });
}

#[test]
fn error_1_x_not_one_exact_result_and_boundaries() {
    with_drivers(|c_driver, rust_driver| {
        for x in [i32::MIN, -1, 0, 2, i32::MAX] {
            let output = compare_case("ERRORS.md row 1", c_driver, rust_driver, x, 2, 3);
            assert_eq!(output, b"Error: x != 1\nOperation failed\nResult: 1\n");
        }
    });
}

#[test]
fn error_2_y_not_two_exact_result_and_boundaries() {
    with_drivers(|c_driver, rust_driver| {
        for y in [i32::MIN, -1, 0, 1, 3, i32::MAX] {
            let output = compare_case("ERRORS.md row 2", c_driver, rust_driver, 1, y, 3);
            assert_eq!(
                output,
                b"Error: x == 1 but y != 2\nOperation failed\nResult: 2\n"
            );
        }
    });
}

#[test]
fn error_3_z_not_three_exact_result_and_boundaries() {
    with_drivers(|c_driver, rust_driver| {
        for z in [i32::MIN, -1, 0, 1, 2, 4, i32::MAX] {
            let output = compare_case("ERRORS.md row 3", c_driver, rust_driver, 1, 2, z);
            assert_eq!(
                output,
                b"Error: x == 1 and y == 2, but z != 3\nOperation failed\nResult: 3\n"
            );
        }
    });
}
