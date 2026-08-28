use libloading::Library;
use std::ffi::{CString, c_char, c_int, c_long, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs, ptr, slice};

type DecodeBase64 = unsafe extern "C" fn(*const c_char) -> *mut c_char;

unsafe extern "C" {
    fn free(ptr: *mut c_void);
}

struct Drivers {
    c: Library,
    rust: Library,
}

impl Drivers {
    fn load() -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("../c_src/build/libdriver.so");
        let rust_path = root.join("target/release/libdriver.so");

        assert!(c_path.is_file(), "missing C library: {}", c_path.display());
        assert!(
            rust_path.is_file(),
            "missing Rust library: {}; run cargo build --release",
            rust_path.display()
        );

        unsafe {
            Self {
                c: Library::new(c_path).expect("load C driver"),
                rust: Library::new(rust_path).expect("load Rust driver"),
            }
        }
    }

    unsafe fn decode_with(library: &Library, src: *const c_char) -> *mut c_char {
        let decode = unsafe {
            library
                .get::<DecodeBase64>(b"decode_base64\0")
                .expect("load decode_base64")
        };
        unsafe { decode(src) }
    }

    fn assert_matches(&self, row: usize, input: &[u8]) {
        let input = CString::new(input).expect("test input must not contain NUL");
        let allocation_len = input.as_bytes().len() + 14;

        unsafe {
            let c_result = Self::decode_with(&self.c, input.as_ptr());
            let rust_result = Self::decode_with(&self.rust, input.as_ptr());

            assert!(!c_result.is_null(), "CONFIGS.md row {row}: C returned NULL");
            assert!(
                !rust_result.is_null(),
                "CONFIGS.md row {row}: Rust returned NULL"
            );

            let c_bytes = slice::from_raw_parts(c_result.cast::<u8>(), allocation_len);
            let rust_bytes = slice::from_raw_parts(rust_result.cast::<u8>(), allocation_len);
            assert_eq!(
                rust_bytes, c_bytes,
                "CONFIGS.md row {row}, input bytes {input:?}"
            );

            free(c_result.cast());
            free(rust_result.cast());
        }
    }

    fn assert_both_null(&self, row: usize, src: *const c_char) {
        unsafe {
            let c_result = Self::decode_with(&self.c, src);
            let rust_result = Self::decode_with(&self.rust, src);
            assert!(c_result.is_null(), "ERRORS.md row {row}: C was non-NULL");
            assert!(
                rust_result.is_null(),
                "ERRORS.md row {row}: Rust was non-NULL"
            );
        }
    }
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn for_row(row: usize) -> Self {
        Self(0x9e37_79b9_7f4a_7c15 ^ row as u64)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    fn index(&mut self, len: usize) -> usize {
        (self.next_u64() as usize) % len
    }

    fn choose(&mut self, values: &[u8]) -> u8 {
        values[self.index(values.len())]
    }

    fn shuffle(&mut self, values: &mut [u8]) {
        for i in (1..values.len()).rev() {
            values.swap(i, self.index(i + 1));
        }
    }
}

const UPPER: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const LOWER: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
const DIGITS: &[u8] = b"0123456789";
const BASE64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const BASE64_WITH_PAD: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=";
const INVALID: &[u8] = b"!\"#$%&'()*,-.:;<>?@[\\]^_`{|}~ \t\n\r";
const ITERATIONS: usize = 128;

fn random_bytes(rng: &mut Rng, alphabet: &[u8], len: usize) -> Vec<u8> {
    (0..len).map(|_| rng.choose(alphabet)).collect()
}

#[test]
fn config_01_uppercase_quartet() {
    let drivers = Drivers::load();
    let mut rng = Rng::for_row(1);
    for _ in 0..ITERATIONS {
        drivers.assert_matches(1, &random_bytes(&mut rng, UPPER, 4));
    }
}

#[test]
fn config_02_all_decode_classes() {
    let drivers = Drivers::load();
    let mut rng = Rng::for_row(2);
    for _ in 0..ITERATIONS {
        let mut input = [rng.choose(LOWER), rng.choose(DIGITS), b'+', b'/'];
        rng.shuffle(&mut input);
        drivers.assert_matches(2, &input);
    }
}

#[test]
fn config_03_one_retained_character() {
    let drivers = Drivers::load();
    let mut rng = Rng::for_row(3);
    for _ in 0..ITERATIONS {
        drivers.assert_matches(3, &[rng.choose(BASE64)]);
    }
}

#[test]
fn config_04_two_retained_characters() {
    let drivers = Drivers::load();
    let mut rng = Rng::for_row(4);
    for _ in 0..ITERATIONS {
        drivers.assert_matches(4, &random_bytes(&mut rng, BASE64, 2));
    }
}

#[test]
fn config_05_three_retained_characters() {
    let drivers = Drivers::load();
    let mut rng = Rng::for_row(5);
    for _ in 0..ITERATIONS {
        drivers.assert_matches(5, &random_bytes(&mut rng, BASE64, 3));
    }
}

#[test]
fn config_06_third_position_padding_only() {
    let drivers = Drivers::load();
    let mut rng = Rng::for_row(6);
    for _ in 0..ITERATIONS {
        let input = [
            rng.choose(BASE64),
            rng.choose(BASE64),
            b'=',
            rng.choose(BASE64),
        ];
        drivers.assert_matches(6, &input);
    }
}

#[test]
fn config_07_fourth_position_padding_only() {
    let drivers = Drivers::load();
    let mut rng = Rng::for_row(7);
    for _ in 0..ITERATIONS {
        let input = [
            rng.choose(BASE64),
            rng.choose(BASE64),
            rng.choose(BASE64),
            b'=',
        ];
        drivers.assert_matches(7, &input);
    }
}

#[test]
fn config_08_both_trailing_positions_padded() {
    let drivers = Drivers::load();
    let mut rng = Rng::for_row(8);
    for _ in 0..ITERATIONS {
        let input = [rng.choose(BASE64), rng.choose(BASE64), b'=', b'='];
        drivers.assert_matches(8, &input);
    }
}

#[test]
fn config_09_leading_padding() {
    let drivers = Drivers::load();
    let mut rng = Rng::for_row(9);
    for iteration in 0..ITERATIONS {
        let mut input = random_bytes(&mut rng, BASE64, 4);
        input[iteration % 2] = b'=';
        drivers.assert_matches(9, &input);
    }
}

#[test]
fn config_10_ignored_bytes_interspersed() {
    let drivers = Drivers::load();
    let mut rng = Rng::for_row(10);
    for _ in 0..ITERATIONS {
        let retained_len = 1 + rng.index(16);
        let retained = random_bytes(&mut rng, BASE64_WITH_PAD, retained_len);
        let mut input = Vec::with_capacity(retained.len() * 3 + 2);
        input.push(rng.choose(INVALID));
        for byte in retained {
            input.push(byte);
            input.push(rng.choose(INVALID));
            if rng.index(2) == 0 {
                input.push(0x80 + rng.index(0x80) as u8);
            }
        }
        input.push(rng.choose(INVALID));
        drivers.assert_matches(10, &input);
    }
}

#[test]
fn config_11_all_bytes_ignored() {
    let drivers = Drivers::load();
    let mut rng = Rng::for_row(11);
    for _ in 0..ITERATIONS {
        let len = 1 + rng.index(64);
        let mut input = random_bytes(&mut rng, INVALID, len);
        if rng.index(2) == 0 {
            input[rng.index(len)] = 0xff;
        }
        drivers.assert_matches(11, &input);
    }
}

#[test]
fn config_12_multiple_quartets() {
    let drivers = Drivers::load();
    let mut rng = Rng::for_row(12);
    for _ in 0..ITERATIONS {
        let len = 5 + rng.index(92);
        let mut input = random_bytes(&mut rng, BASE64_WITH_PAD, len);
        input[len - 1] = if rng.index(2) == 0 {
            b'='
        } else {
            rng.choose(BASE64)
        };
        drivers.assert_matches(12, &input);
    }
}

#[test]
fn config_13_embedded_nul_output() {
    let drivers = Drivers::load();
    let mut rng = Rng::for_row(13);
    for _ in 0..ITERATIONS {
        let input = [
            b'A',
            b'A' + rng.index(16) as u8,
            rng.choose(BASE64),
            rng.choose(BASE64),
        ];
        drivers.assert_matches(13, &input);
    }
}

#[test]
fn config_14_long_input() {
    let drivers = Drivers::load();
    let mut rng = Rng::for_row(14);
    for _ in 0..32 {
        let len = 4096 + rng.index(4096);
        let mut alphabet = Vec::from(BASE64_WITH_PAD);
        alphabet.extend_from_slice(INVALID);
        drivers.assert_matches(14, &random_bytes(&mut rng, &alphabet, len));
    }
}

#[test]
fn error_01_null_input() {
    Drivers::load().assert_both_null(1, ptr::null());
}

#[test]
fn error_02_and_05_empty_input() {
    let empty = b"\0";
    Drivers::load().assert_both_null(2, empty.as_ptr().cast());
}

#[test]
fn error_06_long_input_is_accepted() {
    let drivers = Drivers::load();
    let mut rng = Rng::for_row(14);
    for _ in 0..16 {
        let len = 16_384 + rng.index(16_384);
        drivers.assert_matches(14, &random_bytes(&mut rng, BASE64, len));
    }
}

fn interposer_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("fail_alloc.so")
}

fn build_interposer() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = root.join("tests/fail_alloc.c");
    let output = interposer_path();
    fs::create_dir_all(output.parent().expect("interposer parent")).unwrap();

    let status = Command::new("cc")
        .args(["-std=c11", "-shared", "-fPIC", "-O2", "-o"])
        .arg(&output)
        .arg(&source)
        .status()
        .expect("run cc for allocator interposer");
    assert!(status.success(), "allocator interposer build failed");
    assert!(output.is_file(), "allocator interposer was not produced");
    output
}

#[test]
fn error_03_and_04_allocation_failures() {
    if env::var_os("FAIL_ALLOC_CHILD").is_some() {
        return;
    }

    let interposer = build_interposer();
    let output = Command::new(env::current_exe().expect("current test executable"))
        .args([
            "--exact",
            "error_03_and_04_allocation_failures_child",
            "--nocapture",
        ])
        .env("FAIL_ALLOC_CHILD", "1")
        .env("LD_PRELOAD", &interposer)
        .output()
        .expect("run allocation-failure child");

    assert!(
        output.status.success(),
        "allocation-failure child failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn error_03_and_04_allocation_failures_child() {
    if env::var_os("FAIL_ALLOC_CHILD").is_none() {
        return;
    }

    type Arm = unsafe extern "C" fn(c_long);
    type Disable = unsafe extern "C" fn();
    type WasFreed = unsafe extern "C" fn() -> c_int;

    let process = libloading::os::unix::Library::this();
    let drivers = Drivers::load();
    let input = CString::new("TWFu").unwrap();

    unsafe {
        let arm = process
            .get::<Arm>(b"fail_alloc_arm\0")
            .expect("find fail_alloc_arm");
        let disable = process
            .get::<Disable>(b"fail_alloc_disable\0")
            .expect("find fail_alloc_disable");
        let was_freed = process
            .get::<WasFreed>(b"fail_alloc_tracked_calloc_was_freed\0")
            .expect("find allocation free tracker");

        for (name, library) in [("C", &drivers.c), ("Rust", &drivers.rust)] {
            arm(1);
            let result = Drivers::decode_with(library, input.as_ptr());
            disable();
            assert!(result.is_null(), "ERRORS.md row 3: {name} was non-NULL");

            arm(2);
            let result = Drivers::decode_with(library, input.as_ptr());
            let freed = was_freed();
            disable();
            assert!(result.is_null(), "ERRORS.md row 4: {name} was non-NULL");
            assert_eq!(freed, 1, "ERRORS.md row 4: {name} leaked destination");
        }
    }
}
