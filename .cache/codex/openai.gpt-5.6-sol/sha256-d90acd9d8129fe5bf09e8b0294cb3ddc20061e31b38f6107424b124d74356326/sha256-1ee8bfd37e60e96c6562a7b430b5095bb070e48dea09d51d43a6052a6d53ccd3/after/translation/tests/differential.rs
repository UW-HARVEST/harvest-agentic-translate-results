use libloading::Library;
use std::ffi::{c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::PathBuf;
use std::ptr;
use std::sync::Mutex;

type StaticSum = unsafe extern "C" fn(c_int) -> c_int;
type Driver = unsafe extern "C" fn(c_int);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
}

static TEST_LOCK: Mutex<()> = Mutex::new(());
const STDOUT_FILENO: c_int = 1;

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value >> 12;
        value ^= value << 25;
        value ^= value >> 27;
        self.0 = value;
        (value.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 32) as u32
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
}

fn c_library_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation crate must have a parent")
        .join("c_src/build/libStaticLoop.so")
}

fn rust_library_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/libStaticLoop.so")
}

fn with_apis(test: impl FnOnce(StaticSum, Driver, StaticSum, Driver)) {
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
        let c_library = Library::new(c_path).expect("load C shared object");
        let rust_library = Library::new(rust_path).expect("load Rust shared object");
        let c_static_sum = *c_library
            .get::<StaticSum>(b"static_sum\0")
            .expect("load C static_sum");
        let c_driver = *c_library.get::<Driver>(b"driver\0").expect("load C driver");
        let rust_static_sum = *rust_library
            .get::<StaticSum>(b"static_sum\0")
            .expect("load Rust static_sum");
        let rust_driver = *rust_library
            .get::<Driver>(b"driver\0")
            .expect("load Rust driver");

        test(c_static_sum, c_driver, rust_static_sum, rust_driver);
    }
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0, "flush stdout before capture");

        let mut pipe_fds = [-1; 2];
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0, "create stdout pipe");
        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0, "duplicate stdout");
        assert_eq!(
            dup2(pipe_fds[1], STDOUT_FILENO),
            STDOUT_FILENO,
            "redirect stdout"
        );
        assert_eq!(close(pipe_fds[1]), 0, "close extra pipe writer");

        call();

        assert_eq!(fflush(ptr::null_mut()), 0, "flush captured stdout");
        assert_eq!(
            dup2(saved_stdout, STDOUT_FILENO),
            STDOUT_FILENO,
            "restore stdout"
        );
        assert_eq!(close(saved_stdout), 0, "close saved stdout");

        let mut output = Vec::new();
        File::from_raw_fd(pipe_fds[0])
            .read_to_end(&mut output)
            .expect("read captured stdout");
        output
    }
}

fn edge_values() -> [i32; 7] {
    [0, 1, -1, i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1]
}

#[test]
fn config_001_direct_static_sum_updates_match() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    with_apis(|c_sum, _, rust_sum, _| unsafe {
        for update in edge_values() {
            assert_eq!(c_sum(update), rust_sum(update), "update {update}");
        }

        let mut rng = Rng::new(0x4db6_d5aa_70c4_1337);
        for case in 0..4096 {
            let update = rng.next_i32();
            assert_eq!(
                c_sum(update),
                rust_sum(update),
                "random case {case}, update {update}"
            );
        }
    });
}

#[test]
fn config_002_driver_output_and_state_match() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    with_apis(|c_sum, c_driver, rust_sum, rust_driver| unsafe {
        let mut strides = edge_values().to_vec();
        let mut rng = Rng::new(0xe5c9_a1f0_34b7_284d);
        strides.extend((0..256).map(|_| rng.next_i32()));

        for (case, stride) in strides.into_iter().enumerate() {
            let c_output = capture_stdout(|| c_driver(stride));
            let rust_output = capture_stdout(|| rust_driver(stride));
            assert_eq!(
                c_output, rust_output,
                "driver output case {case}, stride {stride}"
            );
            assert_eq!(
                c_sum(0),
                rust_sum(0),
                "driver state case {case}, stride {stride}"
            );
        }
    });
}

#[test]
fn config_003_interleaved_entry_points_match() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    with_apis(|c_sum, c_driver, rust_sum, rust_driver| unsafe {
        let mut rng = Rng::new(0x8951_f2de_ca80_761b);

        for case in 0..1024 {
            let value = rng.next_i32();
            if rng.next_u32() % 4 == 0 {
                let c_output = capture_stdout(|| c_driver(value));
                let rust_output = capture_stdout(|| rust_driver(value));
                assert_eq!(
                    c_output, rust_output,
                    "interleaved driver case {case}, stride {value}"
                );
            } else {
                assert_eq!(
                    c_sum(value),
                    rust_sum(value),
                    "interleaved sum case {case}, update {value}"
                );
            }

            assert_eq!(c_sum(0), rust_sum(0), "interleaved state after case {case}");
        }
    });
}
