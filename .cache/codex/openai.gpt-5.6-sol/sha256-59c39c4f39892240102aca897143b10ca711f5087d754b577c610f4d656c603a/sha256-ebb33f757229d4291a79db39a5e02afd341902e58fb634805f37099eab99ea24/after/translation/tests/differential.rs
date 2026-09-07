use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

type MyPow = unsafe extern "C" fn(f64, f64) -> f64;

static STDERR_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipefd: *mut c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
}

struct LoadedLibraries {
    _c: Library,
    _rust: Library,
    c_pow: MyPow,
    rust_pow: MyPow,
}

impl LoadedLibraries {
    fn load() -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest.join("../c_src/build/libpow.so");
        let rust_path = manifest.join("target/release/libpow.so");

        assert_shared_library(&c_path);
        assert_shared_library(&rust_path);

        unsafe {
            let c = Library::new(&c_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display()));
            let rust = Library::new(&rust_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display()));

            let c_pow: Symbol<MyPow> = c
                .get(b"my_pow\0")
                .unwrap_or_else(|error| panic!("C my_pow export missing: {error}"));
            let rust_pow: Symbol<MyPow> = rust
                .get(b"my_pow\0")
                .unwrap_or_else(|error| panic!("Rust my_pow export missing: {error}"));
            let c_pow = *c_pow;
            let rust_pow = *rust_pow;

            Self {
                _c: c,
                _rust: rust,
                c_pow,
                rust_pow,
            }
        }
    }
}

fn assert_shared_library(path: &Path) {
    assert!(
        path.is_file(),
        "shared library does not exist: {}; build it before running tests",
        path.display()
    );
}

fn call_with_stderr(function: MyPow, base: f64, exponent: f64) -> (u64, Vec<u8>) {
    let _guard = STDERR_LOCK.lock().expect("stderr capture lock poisoned");

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0, "fflush failed");

        let mut pipe_fds = [-1; 2];
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0, "pipe failed");
        let saved_stderr = dup(2);
        assert!(saved_stderr >= 0, "dup(stderr) failed");
        assert_eq!(dup2(pipe_fds[1], 2), 2, "redirecting stderr failed");
        assert_eq!(close(pipe_fds[1]), 0, "closing pipe writer failed");

        let result = function(base, exponent);
        assert_eq!(fflush(std::ptr::null_mut()), 0, "fflush after call failed");

        assert_eq!(dup2(saved_stderr, 2), 2, "restoring stderr failed");
        assert_eq!(close(saved_stderr), 0, "closing saved stderr failed");

        let mut output = Vec::new();
        let mut buffer = [0_u8; 512];
        loop {
            let count = read(pipe_fds[0], buffer.as_mut_ptr().cast(), buffer.len());
            assert!(count >= 0, "reading captured stderr failed");
            if count == 0 {
                break;
            }
            output.extend_from_slice(&buffer[..count as usize]);
        }
        assert_eq!(close(pipe_fds[0]), 0, "closing pipe reader failed");

        (result.to_bits(), output)
    }
}

fn assert_same_call(libraries: &LoadedLibraries, base: f64, exponent: f64) {
    let c = call_with_stderr(libraries.c_pow, base, exponent);
    let rust = call_with_stderr(libraries.rust_pow, base, exponent);
    assert_eq!(
        c,
        rust,
        "my_pow diverged for base={base:?} ({:#018x}), exponent={exponent:?} ({:#018x})",
        base.to_bits(),
        exponent.to_bits()
    );
}

#[derive(Clone, Copy)]
struct XorShift64(u64);

impl XorShift64 {
    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn unit_f64(&mut self) -> f64 {
        let mantissa = self.next_u64() >> 11;
        mantissa as f64 / (1_u64 << 53) as f64
    }
}

#[test]
fn valid_configuration_matches_byte_for_byte() {
    let libraries = LoadedLibraries::load();

    let special_cases = [
        (2.0, 3.0),
        (2.0, -3.0),
        (-2.0, 4.0),
        (-2.0, 5.0),
        (0.0, 0.0),
        (0.0, 2.0),
        (-0.0, 2.0),
        (-0.0, 3.0),
        (1.0, f64::INFINITY),
        (-1.0, f64::INFINITY),
        (f64::INFINITY, 2.0),
        (f64::INFINITY, -2.0),
        (f64::NEG_INFINITY, 2.0),
        (f64::NEG_INFINITY, 3.0),
        (f64::NAN, 0.0),
        (1.0, f64::NAN),
        (f64::NAN, 2.0),
        (f64::MIN_POSITIVE, 1.0),
        (f64::MAX, 1.0),
    ];
    for (base, exponent) in special_cases {
        assert_same_call(&libraries, base, exponent);
    }

    let mut rng = XorShift64(0x4d59_5df4_d0f3_3173);
    for _ in 0..10_000 {
        let base = 0.25 + rng.unit_f64() * 3.75;
        let exponent = -8.0 + rng.unit_f64() * 16.0;
        assert_same_call(&libraries, base, exponent);

        let negative_base = -(0.25 + rng.unit_f64() * 3.75);
        let integer_exponent = (rng.next_u64() % 17) as f64 - 8.0;
        assert_same_call(&libraries, negative_base, integer_exponent);
    }
}

#[test]
fn domain_error_matches_result_and_diagnostic() {
    let libraries = LoadedLibraries::load();
    let cases = [(-1.0, 0.5), (-2.0, 1.5), (-10.0, -0.5), (-f64::MAX, 0.25)];

    for (base, exponent) in cases {
        let c = call_with_stderr(libraries.c_pow, base, exponent);
        let rust = call_with_stderr(libraries.rust_pow, base, exponent);
        assert_eq!(c.0, (-1.0_f64).to_bits());
        assert_eq!(rust.0, c.0);
        assert_eq!(rust.1, c.1);
        assert!(
            c.1.starts_with(b"Domain error:"),
            "expected EDOM diagnostic, got {:?}",
            String::from_utf8_lossy(&c.1)
        );
    }
}

#[test]
fn range_error_matches_result_and_diagnostic() {
    let libraries = LoadedLibraries::load();
    let cases = [
        (f64::MAX, 2.0),
        (1.0e308, 2.0),
        (2.0, 1024.0),
        (-f64::MAX, 3.0),
        (f64::MIN_POSITIVE, 2.0),
        (2.0, -1075.0),
    ];

    for (base, exponent) in cases {
        let c = call_with_stderr(libraries.c_pow, base, exponent);
        let rust = call_with_stderr(libraries.rust_pow, base, exponent);
        assert_eq!(c.0, (-1.0_f64).to_bits());
        assert_eq!(rust.0, c.0);
        assert_eq!(rust.1, c.1);
        assert!(
            c.1.starts_with(b"Range error:"),
            "expected ERANGE diagnostic, got {:?}",
            String::from_utf8_lossy(&c.1)
        );
    }
}
