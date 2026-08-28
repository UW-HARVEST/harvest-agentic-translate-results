use libloading::Library;
use std::env;
use std::ffi::{c_double, c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::Mutex;

type MyPow = unsafe extern "C" fn(c_double, c_double) -> c_double;

struct PowLibrary {
    function: MyPow,
    _library: Library,
}

impl PowLibrary {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let function = unsafe {
            *library.get::<MyPow>(b"my_pow\0").unwrap_or_else(|error| {
                panic!("failed to load my_pow from {}: {error}", path.display())
            })
        };
        Self {
            function,
            _library: library,
        }
    }

    unsafe fn call(&self, base: f64, exponent: f64) -> f64 {
        unsafe { (self.function)(base, exponent) }
    }
}

struct Libraries {
    c: PowLibrary,
    rust: PowLibrary,
}

impl Libraries {
    unsafe fn load() -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = env::var_os("POW_C_LIB")
            .map(PathBuf::from)
            .unwrap_or_else(|| manifest.join("../c_src/build/libpow.so"));
        let rust_path = env::var_os("POW_RUST_LIB")
            .map(PathBuf::from)
            .unwrap_or_else(|| manifest.join("target/release/libpow.so"));

        Self {
            c: unsafe { PowLibrary::load(&c_path) },
            rust: unsafe { PowLibrary::load(&rust_path) },
        }
    }
}

fn assert_same_bits(c_result: f64, rust_result: f64, base: f64, exponent: f64) {
    assert_eq!(
        c_result.to_bits(),
        rust_result.to_bits(),
        "my_pow differs for base={base:?} ({:#018x}), exponent={exponent:?} ({:#018x}): \
         C={c_result:?} ({:#018x}), Rust={rust_result:?} ({:#018x})",
        base.to_bits(),
        exponent.to_bits(),
        c_result.to_bits(),
        rust_result.to_bits(),
    );
}

fn next_random(state: &mut u64) -> u64 {
    let mut value = *state;
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    *state = value;
    value
}

#[test]
fn valid_configuration_matches_byte_for_byte() {
    let libraries = unsafe { Libraries::load() };

    let fixed_cases = [
        (2.0, 3.0),
        (0.5, -12.0),
        (-2.0, 3.0),
        (-2.0, 4.0),
        (0.0, 3.0),
        (-0.0, 3.0),
        (-0.0, 4.0),
        (f64::INFINITY, 2.0),
        (f64::NEG_INFINITY, 3.0),
        (2.0, f64::INFINITY),
        (0.5, f64::INFINITY),
        (f64::NAN, 2.0),
        (2.0, f64::NAN),
        (f64::NAN, 0.0),
        (1.0, f64::NAN),
    ];

    for (base, exponent) in fixed_cases {
        let c_result = unsafe { libraries.c.call(base, exponent) };
        let rust_result = unsafe { libraries.rust.call(base, exponent) };
        assert_same_bits(c_result, rust_result, base, exponent);
    }

    let mut state = 0x4d59_5df4_d0f3_3173;
    for _ in 0..10_000 {
        let base_fraction = next_random(&mut state) >> 11;
        let exponent_fraction = next_random(&mut state) >> 11;
        let base = 0.5 + (base_fraction as f64) * f64::EPSILON;
        let exponent = -128.0 + (exponent_fraction as f64) * f64::EPSILON * 256.0;

        let c_result = unsafe { libraries.c.call(base, exponent) };
        let rust_result = unsafe { libraries.rust.call(base, exponent) };
        assert_same_bits(c_result, rust_result, base, exponent);
    }

    for _ in 0..2_000 {
        let arbitrary_bits = next_random(&mut state);
        for (base, exponent) in [
            (f64::from_bits(arbitrary_bits), 0.0),
            (1.0, f64::from_bits(arbitrary_bits)),
        ] {
            let c_result = unsafe { libraries.c.call(base, exponent) };
            let rust_result = unsafe { libraries.rust.call(base, exponent) };
            assert_same_bits(c_result, rust_result, base, exponent);
        }

        let negative_base = -(0.5 + ((arbitrary_bits >> 11) as f64) * f64::EPSILON);
        let integer_exponent = ((next_random(&mut state) % 33) as i64 - 16) as f64;
        let c_result = unsafe { libraries.c.call(negative_base, integer_exponent) };
        let rust_result = unsafe { libraries.rust.call(negative_base, integer_exponent) };
        assert_same_bits(c_result, rust_result, negative_base, integer_exponent);
    }
}

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
}

static STDERR_REDIRECT: Mutex<()> = Mutex::new(());

fn capture_stderr<T>(operation: impl FnOnce() -> T) -> (T, Vec<u8>) {
    let _guard = STDERR_REDIRECT
        .lock()
        .expect("stderr redirect lock poisoned");
    let mut pipe_fds = [-1; 2];

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0);
        let saved_stderr = dup(2);
        assert!(saved_stderr >= 0);
        assert_eq!(dup2(pipe_fds[1], 2), 2);
        assert_eq!(close(pipe_fds[1]), 0);

        let result = operation();

        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stderr, 2), 2);
        assert_eq!(close(saved_stderr), 0);

        let mut output = Vec::new();
        File::from_raw_fd(pipe_fds[0])
            .read_to_end(&mut output)
            .expect("failed to read captured stderr");
        (result, output)
    }
}

fn assert_same_error(libraries: &Libraries, base: f64, exponent: f64) {
    let (c_result, c_stderr) = capture_stderr(|| unsafe { libraries.c.call(base, exponent) });
    let (rust_result, rust_stderr) =
        capture_stderr(|| unsafe { libraries.rust.call(base, exponent) });

    assert_eq!(
        c_result.to_bits(),
        (-1.0_f64).to_bits(),
        "C did not reject base={base:?}, exponent={exponent:?}"
    );
    assert_same_bits(c_result, rust_result, base, exponent);
    assert_eq!(
        c_stderr, rust_stderr,
        "stderr differs for base={base:?}, exponent={exponent:?}"
    );
}

#[test]
fn domain_errors_match() {
    let libraries = unsafe { Libraries::load() };
    let mut state = 0x8f68_3a84_5f31_2e91;

    for _ in 0..256 {
        let magnitude = 0.5 + ((next_random(&mut state) >> 11) as f64) * f64::EPSILON * 8.0;
        let integer = (next_random(&mut state) % 21) as f64 - 10.0;
        assert_same_error(&libraries, -magnitude, integer + 0.5);
    }
}

#[test]
fn range_errors_match() {
    let libraries = unsafe { Libraries::load() };

    for (base, exponent) in [
        (f64::MAX, 2.0),
        (10.0, 400.0),
        (f64::MIN_POSITIVE, 2.0),
        (0.5, 1075.0),
        (0.0, -1.0),
        (-0.0, -3.0),
    ] {
        assert_same_error(&libraries, base, exponent);
    }
}
