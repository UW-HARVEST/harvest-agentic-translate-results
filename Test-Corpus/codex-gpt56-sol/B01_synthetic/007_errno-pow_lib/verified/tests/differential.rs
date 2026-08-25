use libloading::Library;
use std::ffi::{c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::PathBuf;
use std::ptr;
use std::sync::Mutex;

type PowFn = unsafe extern "C" fn(f64, f64) -> f64;

const EDOM: c_int = 33;
const ERANGE: c_int = 34;
const RANDOM_CASES: usize = 256;

static STDERR_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn __errno_location() -> *mut c_int;
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
}

struct Apis {
    _c_library: Library,
    _rust_library: Library,
    c_pow: PowFn,
    rust_pow: PowFn,
}

#[derive(Debug)]
struct Call {
    bits: u64,
    errno: c_int,
}

impl Apis {
    fn load() -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest.join("c_src/build/libpow.so");
        let test_binary = std::env::current_exe().expect("current test executable");
        let profile_dir = test_binary
            .parent()
            .and_then(|deps| deps.parent())
            .expect("target profile directory");
        let rust_path = profile_dir.join("libpow.so");

        assert!(
            c_path.is_file(),
            "missing C shared library: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "missing Rust shared library: {}",
            rust_path.display()
        );

        unsafe {
            let c_library = Library::new(&c_path)
                .unwrap_or_else(|error| panic!("load {}: {error}", c_path.display()));
            let rust_library = Library::new(&rust_path)
                .unwrap_or_else(|error| panic!("load {}: {error}", rust_path.display()));
            let c_pow = *c_library.get::<PowFn>(b"my_pow\0").expect("load C my_pow");
            let rust_pow = *rust_library
                .get::<PowFn>(b"my_pow\0")
                .expect("load Rust my_pow");

            Self {
                _c_library: c_library,
                _rust_library: rust_library,
                c_pow,
                rust_pow,
            }
        }
    }

    fn compare_valid(&self, base: f64, exponent: f64) {
        let c = call(self.c_pow, base, exponent);
        let rust = call(self.rust_pow, base, exponent);

        assert_eq!(
            c.errno, 0,
            "C unexpectedly rejected base={base:?}, exponent={exponent:?}"
        );
        assert_eq!(
            rust.errno, c.errno,
            "errno mismatch for base={base:?}, exponent={exponent:?}"
        );
        assert_eq!(
            rust.bits,
            c.bits,
            "result mismatch for base={base:?}, exponent={exponent:?}: \
             C={:?}, Rust={:?}",
            f64::from_bits(c.bits),
            f64::from_bits(rust.bits)
        );
    }
}

fn call(function: PowFn, base: f64, exponent: f64) -> Call {
    unsafe {
        let result = function(base, exponent);
        Call {
            bits: result.to_bits(),
            errno: *__errno_location(),
        }
    }
}

fn capture_stderr(function: impl FnOnce() -> Call) -> (Call, Vec<u8>) {
    let _guard = STDERR_LOCK.lock().expect("stderr lock");

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0, "flush stderr before capture");

        let saved_stderr = dup(2);
        assert!(saved_stderr >= 0, "duplicate stderr");

        let mut pipe_fds = [-1, -1];
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0, "create stderr pipe");
        assert_eq!(dup2(pipe_fds[1], 2), 2, "redirect stderr");
        assert_eq!(close(pipe_fds[1]), 0, "close duplicate pipe writer");

        let result = function();

        assert_eq!(fflush(ptr::null_mut()), 0, "flush captured stderr");
        assert_eq!(dup2(saved_stderr, 2), 2, "restore stderr");
        assert_eq!(close(saved_stderr), 0, "close saved stderr");

        let mut bytes = Vec::new();
        let mut reader = File::from_raw_fd(pipe_fds[0]);
        reader
            .read_to_end(&mut bytes)
            .expect("read captured stderr");
        (result, bytes)
    }
}

fn compare_error(
    apis: &Apis,
    base: f64,
    exponent: f64,
    expected_errno: c_int,
    message_prefix: &[u8],
) {
    let (c, c_stderr) = capture_stderr(|| call(apis.c_pow, base, exponent));
    let (rust, rust_stderr) = capture_stderr(|| call(apis.rust_pow, base, exponent));

    assert_eq!(c.bits, (-1.0f64).to_bits(), "C rejection sentinel");
    assert_eq!(rust.bits, c.bits, "Rust rejection sentinel");
    assert_eq!(c.errno, expected_errno, "C rejection errno");
    assert_eq!(rust.errno, c.errno, "Rust rejection errno");
    assert!(
        c_stderr.starts_with(message_prefix),
        "unexpected C stderr: {:?}",
        String::from_utf8_lossy(&c_stderr)
    );
    assert_eq!(rust_stderr, c_stderr, "stderr mismatch");
}

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
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

    fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / ((1u64 << 53) as f64))
    }

    fn between(&mut self, low: f64, high: f64) -> f64 {
        low + (high - low) * self.unit()
    }
}

#[test]
fn config_1_positive_finite_ordinary_results() {
    let apis = Apis::load();
    let mut rng = Rng::new(0x81c2_9847_2d3a_4f51);

    for _ in 0..RANDOM_CASES {
        let base = rng.between(0.25, 4.0);
        let exponent = rng.between(-8.0, 8.0);
        apis.compare_valid(base, exponent);
    }
}

#[test]
fn config_2_identity_boundaries() {
    let apis = Apis::load();
    let mut rng = Rng::new(0x5f04_b4d1_9a6e_72c3);

    for index in 0..RANDOM_CASES {
        let base = rng.between(-1.0e100, 1.0e100);
        let zero = if index % 2 == 0 { 0.0 } else { -0.0 };
        apis.compare_valid(base, zero);

        let exponent = rng.between(-1.0e6, 1.0e6);
        apis.compare_valid(1.0, exponent);
    }
}

#[test]
fn config_3_signed_zero_positive_exponents() {
    let apis = Apis::load();
    let mut rng = Rng::new(0x3a17_d695_eb20_c84f);

    for index in 0..RANDOM_CASES {
        let base = if index % 2 == 0 { 0.0 } else { -0.0 };
        let exponent = if index % 3 == 0 {
            (rng.next_u64() % 31 + 1) as f64
        } else {
            rng.between(0.125, 32.0)
        };
        apis.compare_valid(base, exponent);
    }
}

#[test]
fn config_4_negative_base_integral_exponents() {
    let apis = Apis::load();
    let mut rng = Rng::new(0xd603_712a_49bc_e85f);

    for _ in 0..RANDOM_CASES {
        let base = -rng.between(0.25, 4.0);
        let mut exponent = (rng.next_u64() % 17) as i32 - 8;
        if exponent == 0 {
            exponent = 1;
        }
        apis.compare_valid(base, f64::from(exponent));
    }
}

#[test]
fn config_5_representable_boundary_results() {
    let apis = Apis::load();
    let mut rng = Rng::new(0xa42b_c813_769e_05df);
    let boundaries = [
        (f64::MAX, 1.0),
        (f64::MIN, 1.0),
        (f64::MIN_POSITIVE, 1.0),
        (f64::from_bits(1), 1.0),
        (-0.0, 3.0),
        (-1.0, 3.0),
        (-1.0, 4.0),
    ];

    for &(base, exponent) in &boundaries {
        apis.compare_valid(base, exponent);
    }

    for _ in 0..RANDOM_CASES {
        let base_power = (rng.next_u64() % 17) as i32 - 8;
        let exponent = (rng.next_u64() % 7 + 1) as i32;
        let sign = if rng.next_u64() & 1 == 0 { 1.0 } else { -1.0 };
        let base = sign * 2.0f64.powi(base_power);
        apis.compare_valid(base, f64::from(exponent));
    }
}

#[test]
fn config_6_nan_and_infinity_inputs() {
    let apis = Apis::load();
    let fixtures = [
        (f64::NAN, 0.0),
        (1.0, f64::NAN),
        (f64::NAN, 2.0),
        (f64::INFINITY, 2.0),
        (f64::INFINITY, -2.0),
        (f64::NEG_INFINITY, 3.0),
        (f64::NEG_INFINITY, 4.0),
        (2.0, f64::INFINITY),
        (2.0, f64::NEG_INFINITY),
        (0.5, f64::INFINITY),
        (0.5, f64::NEG_INFINITY),
    ];

    for &(base, exponent) in &fixtures {
        apis.compare_valid(base, exponent);
    }

    let mut rng = Rng::new(0x72e9_146b_a503_dc8f);
    for index in 0..RANDOM_CASES {
        let payload = rng.next_u64() & 0x0007_ffff_ffff_ffff;
        let nan = f64::from_bits(0x7ff8_0000_0000_0000 | payload);
        if index % 2 == 0 {
            apis.compare_valid(nan, 2.0);
        } else {
            apis.compare_valid(2.0, nan);
        }
    }
}

#[test]
fn error_1_domain_error() {
    let apis = Apis::load();
    compare_error(&apis, -2.0, 0.5, EDOM, b"Domain error:");
}

#[test]
fn error_2_range_error_overflow_and_underflow() {
    let apis = Apis::load();
    compare_error(&apis, 1.0e308, 2.0, ERANGE, b"Range error:");
    compare_error(&apis, 1.0e-308, 2.0, ERANGE, b"Range error:");
}
