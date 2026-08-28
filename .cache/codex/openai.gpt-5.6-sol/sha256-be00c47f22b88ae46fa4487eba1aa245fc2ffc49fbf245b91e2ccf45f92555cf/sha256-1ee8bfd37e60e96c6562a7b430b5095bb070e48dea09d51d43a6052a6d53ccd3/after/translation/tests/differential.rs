use libloading::Library;
use std::env;
use std::ffi::{c_char, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::slice;

type CustomStrdup = unsafe extern "C" fn(*const c_char) -> *mut c_char;
type FailNextMalloc = unsafe extern "C" fn();

unsafe extern "C" {
    fn free(pointer: *mut c_void);
}

struct Libraries {
    _c: Library,
    _rust: Library,
    c_strdup: CustomStrdup,
    rust_strdup: CustomStrdup,
}

impl Libraries {
    fn load() -> Self {
        let c_path = c_library_path();
        let rust_path = rust_library_path();
        assert!(
            c_path.is_file(),
            "C shared library not found: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "Rust shared library not found: {}",
            rust_path.display()
        );

        unsafe {
            let c = Library::new(&c_path)
                .unwrap_or_else(|error| panic!("load {}: {error}", c_path.display()));
            let rust = Library::new(&rust_path)
                .unwrap_or_else(|error| panic!("load {}: {error}", rust_path.display()));
            let c_strdup = *c
                .get::<CustomStrdup>(b"custom_strdup\0")
                .expect("load custom_strdup from C shared library");
            let rust_strdup = *rust
                .get::<CustomStrdup>(b"custom_strdup\0")
                .expect("load custom_strdup from Rust shared library");
            assert_ne!(
                c_strdup as usize, rust_strdup as usize,
                "C and Rust symbol lookups resolved to the same implementation"
            );

            Self {
                _c: c,
                _rust: rust,
                c_strdup,
                rust_strdup,
            }
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("../c_src/build/libdriver.so")
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/release/libdriver.so")
}

fn malloc_fail_library_path() -> PathBuf {
    manifest_dir().join("target/malloc-fail/libmalloc_fail.so")
}

fn expected_c_string(storage: &[u8]) -> &[u8] {
    let nul = storage
        .iter()
        .position(|byte| *byte == 0)
        .expect("test input must contain a NUL terminator");
    &storage[..=nul]
}

fn compare_success(libraries: &Libraries, storage: &[u8]) {
    let expected = expected_c_string(storage);
    let input = storage.as_ptr().cast::<c_char>();

    unsafe {
        let c_result = (libraries.c_strdup)(input);
        let rust_result = (libraries.rust_strdup)(input);
        assert!(!c_result.is_null(), "C unexpectedly returned NULL");
        assert!(!rust_result.is_null(), "Rust unexpectedly returned NULL");
        assert_ne!(c_result.cast_const(), input, "C returned the input pointer");
        assert_ne!(
            rust_result.cast_const(),
            input,
            "Rust returned the input pointer"
        );

        let c_bytes = slice::from_raw_parts(c_result.cast::<u8>(), expected.len());
        let rust_bytes = slice::from_raw_parts(rust_result.cast::<u8>(), expected.len());
        assert_eq!(c_bytes, expected, "C output differs from the source bytes");
        assert_eq!(
            rust_bytes, c_bytes,
            "Rust output differs byte-for-byte from C"
        );

        free(c_result.cast());
        free(rust_result.cast());
    }
}

#[derive(Clone, Copy)]
struct FixedRng(u64);

impl FixedRng {
    fn new() -> Self {
        Self(0xd1ff_e2e7_5eed_c0de)
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn usize_below(&mut self, upper: usize) -> usize {
        (self.next_u64() as usize) % upper
    }

    fn nonzero_byte(&mut self) -> u8 {
        (self.usize_below(255) + 1) as u8
    }

    fn byte(&mut self) -> u8 {
        self.next_u64() as u8
    }
}

#[test]
fn config_1_empty_strings() {
    let libraries = Libraries::load();
    let mut rng = FixedRng::new();

    for _ in 0..512 {
        let trailing_len = rng.usize_below(257);
        let mut storage = Vec::with_capacity(trailing_len + 1);
        storage.push(0);
        storage.extend((0..trailing_len).map(|_| rng.byte()));
        compare_success(&libraries, &storage);
    }
}

#[test]
fn config_2_one_byte_strings() {
    let libraries = Libraries::load();
    let mut rng = FixedRng::new();

    for _ in 0..512 {
        compare_success(&libraries, &[rng.nonzero_byte(), 0]);
    }
}

#[test]
fn config_3_multi_byte_and_long_strings() {
    let libraries = Libraries::load();
    let mut rng = FixedRng::new();

    for case in 0..512 {
        let length = if case % 64 == 0 {
            65_536 + rng.usize_below(65_537)
        } else {
            2 + rng.usize_below(8_191)
        };
        let mut storage: Vec<u8> = (0..length).map(|_| rng.nonzero_byte()).collect();
        storage.push(0);
        compare_success(&libraries, &storage);
    }

    let mut oversized = vec![0xff; 1_048_576];
    oversized.push(0);
    compare_success(&libraries, &oversized);
}

#[test]
fn config_4_bytes_after_first_nul_are_not_copied() {
    let libraries = Libraries::load();
    let mut rng = FixedRng::new();

    for _ in 0..512 {
        let prefix_len = 1 + rng.usize_below(256);
        let suffix_len = 1 + rng.usize_below(256);
        let mut storage: Vec<u8> = (0..prefix_len).map(|_| rng.nonzero_byte()).collect();
        storage.push(0);
        storage.extend((0..suffix_len).map(|_| rng.byte()));
        compare_success(&libraries, &storage);
    }
}

#[test]
fn error_1_null_input() {
    let libraries = Libraries::load();

    unsafe {
        let c_result = (libraries.c_strdup)(ptr::null());
        let rust_result = (libraries.rust_strdup)(ptr::null());
        assert!(c_result.is_null(), "C did not return NULL for NULL input");
        assert_eq!(
            rust_result, c_result,
            "Rust rejection sentinel differs from C"
        );
    }
}

fn build_malloc_fail_shim(output: &Path) {
    let source = manifest_dir().join("tests/malloc_fail.c");
    let parent = output.parent().expect("shim output has a parent");
    std::fs::create_dir_all(parent).expect("create malloc-fail output directory");

    let status = Command::new("cc")
        .args(["-shared", "-fPIC", "-o"])
        .arg(output)
        .arg(source)
        .status()
        .expect("run C compiler for malloc-failure shim");
    assert!(status.success(), "failed to compile malloc-failure shim");
}

#[test]
fn error_2_allocation_failure() {
    if env::var_os("DRIVER_MALLOC_FAILURE_CHILD").is_some() {
        return;
    }

    let shim = malloc_fail_library_path();
    build_malloc_fail_shim(&shim);
    let output = Command::new(env::current_exe().expect("get current test executable"))
        .arg("--exact")
        .arg("allocation_failure_child")
        .arg("--nocapture")
        .arg("--test-threads=1")
        .env("DRIVER_MALLOC_FAILURE_CHILD", "1")
        .env("LD_PRELOAD", &shim)
        .output()
        .expect("run allocation-failure child test");

    assert!(
        output.status.success(),
        "allocation-failure child failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn allocation_failure_child() {
    if env::var_os("DRIVER_MALLOC_FAILURE_CHILD").is_none() {
        return;
    }

    let shim_path = malloc_fail_library_path();
    let shim = unsafe { Library::new(&shim_path) }
        .unwrap_or_else(|error| panic!("load {}: {error}", shim_path.display()));
    let fail_next = unsafe {
        *shim
            .get::<FailNextMalloc>(b"fail_next_malloc\0")
            .expect("load fail_next_malloc from preload shim")
    };
    let libraries = Libraries::load();
    let input = b"allocation must fail\0";

    unsafe {
        fail_next();
        let c_result = (libraries.c_strdup)(input.as_ptr().cast());
        assert!(
            c_result.is_null(),
            "C did not return NULL when malloc failed"
        );

        fail_next();
        let rust_result = (libraries.rust_strdup)(input.as_ptr().cast());
        assert_eq!(
            rust_result, c_result,
            "Rust allocation-failure sentinel differs from C"
        );
    }
}
