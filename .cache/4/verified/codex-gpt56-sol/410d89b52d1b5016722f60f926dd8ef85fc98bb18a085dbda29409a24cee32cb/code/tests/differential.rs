use libloading::Library;
use std::ffi::{CStr, c_char, c_int, c_void};
use std::path::{Path, PathBuf};

type EncodeBase64 = unsafe extern "C" fn(c_int, *const c_char) -> *mut c_char;

unsafe extern "C" {
    fn free(ptr: *mut c_void);
}

struct Api {
    _library: Library,
    encode_base64: EncodeBase64,
}

impl Api {
    fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let encode_base64 = unsafe {
            *library
                .get::<EncodeBase64>(b"encode_base64\0")
                .unwrap_or_else(|error| {
                    panic!(
                        "failed to resolve encode_base64 in {}: {error}",
                        path.display()
                    )
                })
        };
        Self {
            _library: library,
            encode_base64,
        }
    }
}

struct Libraries {
    c: Api,
    rust: Api,
}

impl Libraries {
    fn load() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("c_src/build/libdriver.so");
        let test_executable = std::env::current_exe().expect("test executable path");
        let rust_path = test_executable
            .parent()
            .and_then(Path::parent)
            .expect("target profile directory")
            .join("libdriver.so");

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

        Self {
            c: Api::load(&c_path),
            rust: Api::load(&rust_path),
        }
    }

    fn assert_match(&self, size: c_int, input: &[u8]) {
        let c = call(&self.c, size, input.as_ptr().cast());
        let rust = call(&self.rust, size, input.as_ptr().cast());
        assert_eq!(c, rust, "differential mismatch for size {size}");
    }
}

fn call(api: &Api, size: c_int, input: *const c_char) -> Option<Vec<u8>> {
    let output = unsafe { (api.encode_base64)(size, input) };
    if output.is_null() {
        return None;
    }

    let bytes = unsafe { CStr::from_ptr(output) }
        .to_bytes_with_nul()
        .to_vec();
    unsafe { free(output.cast()) };
    Some(bytes)
}

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

    fn usize(&mut self, upper_exclusive: usize) -> usize {
        (self.next_u64() as usize) % upper_exclusive
    }

    fn bytes(&mut self, length: usize) -> Vec<u8> {
        (0..length).map(|_| self.next_u64() as u8).collect()
    }

    fn nonzero_bytes(&mut self, length: usize) -> Vec<u8> {
        (0..length)
            .map(|_| (self.next_u64() % 255 + 1) as u8)
            .collect()
    }
}

fn check_explicit_fixed_length(length: usize, seed: u64) {
    let libraries = Libraries::load();
    let mut rng = Rng::new(seed);
    for _ in 0..256 {
        let input = rng.bytes(length);
        libraries.assert_match(length as c_int, &input);
    }
}

fn check_explicit_remainder(remainder: usize, seed: u64) {
    let libraries = Libraries::load();
    let mut rng = Rng::new(seed);
    for _ in 0..256 {
        let blocks = 2 + rng.usize(63);
        let length = blocks * 3 + remainder;
        let input = rng.bytes(length);
        libraries.assert_match(length as c_int, &input);
    }
}

fn check_strlen_fixed_length(length: usize, seed: u64) {
    let libraries = Libraries::load();
    let mut rng = Rng::new(seed);
    for _ in 0..256 {
        let mut input = rng.nonzero_bytes(length);
        input.push(0);
        libraries.assert_match(0, &input);
    }
}

fn check_strlen_remainder(remainder: usize, seed: u64) {
    let libraries = Libraries::load();
    let mut rng = Rng::new(seed);
    for _ in 0..256 {
        let blocks = 2 + rng.usize(63);
        let length = blocks * 3 + remainder;
        let mut input = rng.nonzero_bytes(length);
        input.push(0);
        libraries.assert_match(0, &input);
    }
}

#[test]
fn config_01_explicit_one_byte() {
    check_explicit_fixed_length(1, 0x0101_9e37_79b9_7f4a);
}

#[test]
fn config_02_explicit_two_bytes() {
    check_explicit_fixed_length(2, 0x0202_9e37_79b9_7f4a);
}

#[test]
fn config_03_explicit_complete_block() {
    check_explicit_fixed_length(3, 0x0303_9e37_79b9_7f4a);
}

#[test]
fn config_04_explicit_many_remainder_one() {
    check_explicit_remainder(1, 0x0404_9e37_79b9_7f4a);
}

#[test]
fn config_05_explicit_many_remainder_two() {
    check_explicit_remainder(2, 0x0505_9e37_79b9_7f4a);
}

#[test]
fn config_06_explicit_many_complete_blocks() {
    check_explicit_remainder(0, 0x0606_9e37_79b9_7f4a);
}

#[test]
fn config_07_strlen_empty() {
    let libraries = Libraries::load();
    for _ in 0..256 {
        libraries.assert_match(0, &[0]);
    }
}

#[test]
fn config_08_strlen_one_byte() {
    check_strlen_fixed_length(1, 0x0808_9e37_79b9_7f4a);
}

#[test]
fn config_09_strlen_two_bytes() {
    check_strlen_fixed_length(2, 0x0909_9e37_79b9_7f4a);
}

#[test]
fn config_10_strlen_many_remainder_one() {
    check_strlen_remainder(1, 0x1010_9e37_79b9_7f4a);
}

#[test]
fn config_11_strlen_many_remainder_two() {
    check_strlen_remainder(2, 0x1111_9e37_79b9_7f4a);
}

#[test]
fn config_12_strlen_many_complete_blocks() {
    check_strlen_remainder(0, 0x1212_9e37_79b9_7f4a);
}

#[test]
fn config_13_explicit_binary_full_byte_range() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x1313_9e37_79b9_7f4a);
    let mut input: Vec<u8> = (0..=u8::MAX).collect();

    for _ in 0..128 {
        for index in (1..input.len()).rev() {
            let other = rng.usize(index + 1);
            input.swap(index, other);
        }
        libraries.assert_match(input.len() as c_int, &input);
    }
}

#[test]
fn config_14_explicit_large_readable_lengths() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x1414_9e37_79b9_7f4a);

    for length in [65_535, 65_536, 65_537] {
        for _ in 0..16 {
            let input = rng.bytes(length);
            libraries.assert_match(length as c_int, &input);
        }
    }
}

#[test]
fn config_15_negative_size_with_successful_allocation() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x1515_9e37_79b9_7f4a);

    for _ in 0..256 {
        let length = 1 + rng.usize(64);
        let input = rng.bytes(length);
        libraries.assert_match(-1, &input);
        libraries.assert_match(-2, &input);
    }
}

#[test]
fn error_01_null_source_returns_null() {
    let libraries = Libraries::load();
    let c = call(&libraries.c, 1, std::ptr::null());
    let rust = call(&libraries.rust, 1, std::ptr::null());
    assert_eq!(c, None);
    assert_eq!(rust, c);
}

#[test]
fn error_02_allocation_failure_returns_null() {
    let libraries = Libraries::load();
    let input = [0_u8];
    let size = -536_870_912;
    let c = call(&libraries.c, size, input.as_ptr().cast());
    let rust = call(&libraries.rust, size, input.as_ptr().cast());
    assert_eq!(c, None);
    assert_eq!(rust, c);
}
