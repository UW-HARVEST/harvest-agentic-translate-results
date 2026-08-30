use libloading::Library;
use std::ffi::{c_int, c_void};
use std::path::PathBuf;
use std::sync::Mutex;

type Driver = unsafe extern "C" fn(c_int, c_int, c_int);

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipefd: *mut c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_i32(&mut self) -> i32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 32) as u32 as i32
    }

    fn next_except(&mut self, excluded: i32) -> i32 {
        loop {
            let value = self.next_i32();
            if value != excluded {
                return value;
            }
        }
    }
}

fn c_library_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libdriver.so")
}

fn rust_library_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/libdriver.so")
}

fn load_drivers() -> (Library, Driver, Library, Driver) {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(
        c_path.is_file(),
        "missing C shared object: {}",
        c_path.display()
    );
    assert!(
        rust_path.is_file(),
        "missing Rust shared object: {}",
        rust_path.display()
    );

    unsafe {
        let c_library = Library::new(&c_path).expect("load C shared object");
        let c_driver = *c_library
            .get::<Driver>(b"driver\0")
            .expect("load C driver symbol");
        let rust_library = Library::new(&rust_path).expect("load Rust shared object");
        let rust_driver = *rust_library
            .get::<Driver>(b"driver\0")
            .expect("load Rust driver symbol");
        (c_library, c_driver, rust_library, rust_driver)
    }
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    unsafe {
        assert_eq!(
            fflush(std::ptr::null_mut()),
            0,
            "flush stdout before capture"
        );

        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0, "duplicate stdout");

        let mut pipe_fds = [-1; 2];
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0, "create stdout pipe");
        assert_eq!(dup2(pipe_fds[1], 1), 1, "redirect stdout");
        assert_eq!(close(pipe_fds[1]), 0, "close extra pipe writer");

        call();

        assert_eq!(fflush(std::ptr::null_mut()), 0, "flush captured stdout");
        assert_eq!(dup2(saved_stdout, 1), 1, "restore stdout");
        assert_eq!(close(saved_stdout), 0, "close saved stdout");

        let mut output = Vec::new();
        let mut buffer = [0_u8; 256];
        loop {
            let count = read(
                pipe_fds[0],
                buffer.as_mut_ptr().cast::<c_void>(),
                buffer.len(),
            );
            assert!(count >= 0, "read captured stdout");
            if count == 0 {
                break;
            }
            output.extend_from_slice(&buffer[..count as usize]);
        }
        assert_eq!(close(pipe_fds[0]), 0, "close pipe reader");
        output
    }
}

fn assert_case(
    c_driver: Driver,
    rust_driver: Driver,
    input: (i32, i32, i32),
    expected: &[u8],
    row: &str,
) {
    let invoke = |driver: Driver| {
        capture_stdout(|| unsafe {
            driver(input.0, input.1, input.2);
        })
    };

    let c_output = invoke(c_driver);
    let rust_output = invoke(rust_driver);
    assert_eq!(
        c_output, expected,
        "{row}: unexpected C ground truth for {input:?}"
    );
    assert_eq!(
        rust_output, c_output,
        "{row}: Rust output differs for {input:?}"
    );
}

fn with_drivers(test: impl FnOnce(Driver, Driver)) {
    let _stdout_guard = STDOUT_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let (_c_library, c_driver, _rust_library, rust_driver) = load_drivers();
    test(c_driver, rust_driver);
}

const X_ERROR: &[u8] = b"Error: x != 1\nOperation failed\nResult: 1\n";
const Y_ERROR: &[u8] = b"Error: x == 1 but y != 2\nOperation failed\nResult: 2\n";
const Z_ERROR: &[u8] = b"Error: x == 1 and y == 2, but z != 3\nOperation failed\nResult: 3\n";
const SUCCESS: &[u8] = b"Ok!\nResult: 0\n";

#[test]
fn config_row_1_x_rejects_before_y_or_z_are_inspected() {
    with_drivers(|c_driver, rust_driver| {
        let boundaries = [
            (i32::MIN, i32::MIN, i32::MIN),
            (i32::MAX, i32::MAX, i32::MAX),
            (0, 2, 3),
            (2, 2, 3),
            (-1, 1, 4),
        ];
        for input in boundaries {
            assert_case(c_driver, rust_driver, input, X_ERROR, "CONFIGS.md row 1");
        }

        let mut rng = Rng::new(0xC0A1_F1A0_0000_0001);
        for _ in 0..256 {
            let input = (rng.next_except(1), rng.next_i32(), rng.next_i32());
            assert_case(c_driver, rust_driver, input, X_ERROR, "CONFIGS.md row 1");
        }
    });
}

#[test]
fn config_row_2_y_rejects_before_z_is_inspected() {
    with_drivers(|c_driver, rust_driver| {
        let boundaries = [
            (1, i32::MIN, i32::MIN),
            (1, i32::MAX, i32::MAX),
            (1, 1, 3),
            (1, 3, 3),
        ];
        for input in boundaries {
            assert_case(c_driver, rust_driver, input, Y_ERROR, "CONFIGS.md row 2");
        }

        let mut rng = Rng::new(0xC0A1_F1A0_0000_0002);
        for _ in 0..256 {
            let input = (1, rng.next_except(2), rng.next_i32());
            assert_case(c_driver, rust_driver, input, Y_ERROR, "CONFIGS.md row 2");
        }
    });
}

#[test]
fn config_row_3_z_rejection() {
    with_drivers(|c_driver, rust_driver| {
        let boundaries = [(1, 2, i32::MIN), (1, 2, i32::MAX), (1, 2, 2), (1, 2, 4)];
        for input in boundaries {
            assert_case(c_driver, rust_driver, input, Z_ERROR, "CONFIGS.md row 3");
        }

        let mut rng = Rng::new(0xC0A1_F1A0_0000_0003);
        for _ in 0..256 {
            let input = (1, 2, rng.next_except(3));
            assert_case(c_driver, rust_driver, input, Z_ERROR, "CONFIGS.md row 3");
        }
    });
}

#[test]
fn config_row_4_success_after_randomized_prior_state() {
    with_drivers(|c_driver, rust_driver| {
        let mut rng = Rng::new(0xC0A1_F1A0_0000_0004);
        for _ in 0..256 {
            let prior = (rng.next_except(1), rng.next_i32(), rng.next_i32());
            assert_case(c_driver, rust_driver, prior, X_ERROR, "row 4 prior state");
            assert_case(
                c_driver,
                rust_driver,
                (1, 2, 3),
                SUCCESS,
                "CONFIGS.md row 4",
            );
        }
    });
}

#[test]
fn error_row_1_exact_x_rejection() {
    with_drivers(|c_driver, rust_driver| {
        assert_case(
            c_driver,
            rust_driver,
            (0, i32::MIN, i32::MAX),
            X_ERROR,
            "ERRORS.md row 1",
        );
    });
}

#[test]
fn error_row_2_exact_y_rejection() {
    with_drivers(|c_driver, rust_driver| {
        assert_case(
            c_driver,
            rust_driver,
            (1, 1, i32::MAX),
            Y_ERROR,
            "ERRORS.md row 2",
        );
    });
}

#[test]
fn error_row_3_exact_z_rejection() {
    with_drivers(|c_driver, rust_driver| {
        assert_case(c_driver, rust_driver, (1, 2, 4), Z_ERROR, "ERRORS.md row 3");
    });
}
