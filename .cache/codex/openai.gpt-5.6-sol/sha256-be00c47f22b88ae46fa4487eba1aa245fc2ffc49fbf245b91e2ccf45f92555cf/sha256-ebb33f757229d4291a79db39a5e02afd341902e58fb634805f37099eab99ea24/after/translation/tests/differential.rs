use libloading::Library;
use std::ffi::{c_char, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs, ptr, slice};

type CustomStrdup = unsafe extern "C" fn(*const c_char) -> *mut c_char;
type ArmMallocFailure = unsafe extern "C" fn(usize);

unsafe extern "C" {
    fn free(pointer: *mut c_void);
}

struct Api {
    _library: Library,
    custom_strdup: CustomStrdup,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let custom_strdup = unsafe {
            *library
                .get::<CustomStrdup>(b"custom_strdup\0")
                .unwrap_or_else(|error| {
                    panic!(
                        "failed to load custom_strdup from {}: {error}",
                        path.display()
                    )
                })
        };
        Self {
            _library: library,
            custom_strdup,
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

fn load_apis() -> (Api, Api) {
    assert!(
        c_library_path().is_file(),
        "C library is missing; build ../c_src/build/libdriver.so first"
    );
    assert!(
        rust_library_path().is_file(),
        "Rust release library is missing; run cargo build --release first"
    );
    unsafe {
        (
            Api::load(&c_library_path()),
            Api::load(&rust_library_path()),
        )
    }
}

fn first_nul_length(input: &[u8]) -> usize {
    input
        .iter()
        .position(|byte| *byte == 0)
        .expect("test input must contain a NUL byte")
        + 1
}

unsafe fn call_and_copy(function: CustomStrdup, input: &[u8]) -> Option<Vec<u8>> {
    let output = unsafe { function(input.as_ptr().cast()) };
    if output.is_null() {
        return None;
    }

    let length = first_nul_length(input);
    let bytes = unsafe { slice::from_raw_parts(output.cast::<u8>(), length) }.to_vec();
    unsafe { free(output.cast()) };
    Some(bytes)
}

fn assert_match(c: &Api, rust: &Api, input: &[u8]) {
    let expected_length = first_nul_length(input);
    let expected = &input[..expected_length];
    let c_output = unsafe { call_and_copy(c.custom_strdup, input) };
    let rust_output = unsafe { call_and_copy(rust.custom_strdup, input) };

    assert_eq!(c_output.as_deref(), Some(expected), "C result differed");
    assert_eq!(
        rust_output.as_deref(),
        Some(expected),
        "Rust result differed"
    );
    assert_eq!(rust_output, c_output, "C and Rust results differed");
}

#[derive(Clone, Copy)]
struct FixedRng(u64);

impl FixedRng {
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

    fn range(&mut self, start: usize, end: usize) -> usize {
        assert!(start < end);
        start + (self.next_u64() as usize % (end - start))
    }

    fn nonzero_byte(&mut self) -> u8 {
        self.range(1, 256) as u8
    }
}

#[test]
fn config_1_empty_string() {
    let (c, rust) = load_apis();
    for _ in 0..256 {
        assert_match(&c, &rust, &[0]);
    }
}

#[test]
fn config_2_one_non_nul_byte() {
    let (c, rust) = load_apis();
    let mut rng = FixedRng::new(0x4b1d_5eed_0000_0002);
    for _ in 0..1024 {
        assert_match(&c, &rust, &[rng.nonzero_byte(), 0]);
    }
}

#[test]
fn config_3_many_ascii_bytes() {
    let (c, rust) = load_apis();
    let mut rng = FixedRng::new(0x4b1d_5eed_0000_0003);
    for _ in 0..512 {
        let length = rng.range(2, 4097);
        let mut input = Vec::with_capacity(length + 1);
        input.extend((0..length).map(|_| rng.range(1, 128) as u8));
        input.push(0);
        assert_match(&c, &rust, &input);
    }
}

#[test]
fn config_4_many_arbitrary_non_nul_bytes() {
    let (c, rust) = load_apis();
    let mut rng = FixedRng::new(0x4b1d_5eed_0000_0004);
    for _ in 0..512 {
        let length = rng.range(2, 4097);
        let mut input = Vec::with_capacity(length + 1);
        input.extend((0..length).map(|_| rng.nonzero_byte()));
        input[0] |= 0x80;
        input.push(0);
        assert_match(&c, &rust, &input);
    }
}

#[test]
fn config_5_first_nul_terminates_input() {
    let (c, rust) = load_apis();
    let mut rng = FixedRng::new(0x4b1d_5eed_0000_0005);
    for _ in 0..512 {
        let prefix_length = rng.range(0, 1025);
        let suffix_length = rng.range(1, 1025);
        let mut input = Vec::with_capacity(prefix_length + 1 + suffix_length);
        input.extend((0..prefix_length).map(|_| rng.nonzero_byte()));
        input.push(0);
        input.extend((0..suffix_length).map(|_| rng.next_u64() as u8));
        assert_match(&c, &rust, &input);
    }
}

#[test]
fn error_1_null_pointer() {
    let (c, rust) = load_apis();
    let c_output = unsafe { (c.custom_strdup)(ptr::null()) };
    let rust_output = unsafe { (rust.custom_strdup)(ptr::null()) };
    assert!(c_output.is_null(), "C must reject NULL with NULL");
    assert!(rust_output.is_null(), "Rust must reject NULL with NULL");
    assert_eq!(rust_output, c_output);
}

fn malloc_shim_path() -> PathBuf {
    let profile_dir = env::current_exe()
        .expect("test executable path")
        .parent()
        .expect("test executable directory")
        .to_path_buf();
    profile_dir.join("libfail_malloc.so")
}

fn compile_malloc_shim(output: &Path) {
    let source = manifest_dir().join("tests/fail_malloc.c");
    let status = Command::new("cc")
        .args(["-shared", "-fPIC", "-std=c11", "-O2"])
        .arg(&source)
        .arg("-o")
        .arg(output)
        .status()
        .expect("failed to execute cc for malloc shim");
    assert!(status.success(), "failed to compile malloc shim");
    assert!(output.is_file(), "malloc shim output was not created");
}

fn run_malloc_failure_child(shim_path: &Path) {
    let shim = unsafe { Library::new(shim_path) }
        .unwrap_or_else(|error| panic!("failed to open malloc shim: {error}"));
    let arm = unsafe {
        *shim
            .get::<ArmMallocFailure>(b"fail_malloc_arm\0")
            .unwrap_or_else(|error| panic!("failed to load fail_malloc_arm: {error}"))
    };
    let (c, rust) = load_apis();
    let input = b"forced-allocation-failure\0";

    unsafe { arm(input.len()) };
    let c_output = unsafe { (c.custom_strdup)(input.as_ptr().cast()) };
    assert!(
        c_output.is_null(),
        "C did not return NULL on malloc failure"
    );

    unsafe { arm(input.len()) };
    let rust_output = unsafe { (rust.custom_strdup)(input.as_ptr().cast()) };
    assert!(
        rust_output.is_null(),
        "Rust did not return NULL on malloc failure"
    );
    assert_eq!(rust_output, c_output);
}

#[test]
fn error_2_malloc_failure() {
    const CHILD_ENV: &str = "DRIVER_MALLOC_FAILURE_CHILD";
    if env::var_os(CHILD_ENV).is_some() {
        let shim_path = env::var_os("DRIVER_MALLOC_SHIM")
            .map(PathBuf::from)
            .expect("child malloc shim path");
        run_malloc_failure_child(&shim_path);
        return;
    }

    let shim_path = malloc_shim_path();
    if shim_path.exists() {
        fs::remove_file(&shim_path).expect("remove stale malloc shim");
    }
    compile_malloc_shim(&shim_path);

    let output = Command::new(env::current_exe().expect("test executable path"))
        .args(["--exact", "error_2_malloc_failure", "--nocapture"])
        .env(CHILD_ENV, "1")
        .env("DRIVER_MALLOC_SHIM", &shim_path)
        .env("LD_PRELOAD", &shim_path)
        .output()
        .expect("failed to execute allocation-failure child");

    assert!(
        output.status.success(),
        "allocation-failure child failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
