use libloading::Library;
use std::env;
use std::ffi::{CString, c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::slice;

type Decode = unsafe extern "C" fn(*const c_char) -> *mut c_char;
type SetFailMode = unsafe extern "C" fn(c_int);

unsafe extern "C" {
    fn free(ptr: *mut c_void);
}

struct Libraries {
    _c_library: Library,
    _rust_library: Library,
    c_decode: Decode,
    rust_decode: Decode,
}

impl Libraries {
    fn load() -> Self {
        let c_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libdriver.so");
        let rust_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libdriver.so");

        assert!(c_path.is_file(), "missing C library: {}", c_path.display());
        assert!(
            rust_path.is_file(),
            "missing Rust library: {}",
            rust_path.display()
        );

        unsafe {
            let c_library = Library::new(&c_path).expect("load C library");
            let rust_library = Library::new(&rust_path).expect("load Rust library");
            let c_decode = *c_library
                .get::<Decode>(b"decode_base64\0")
                .expect("load C decode_base64");
            let rust_decode = *rust_library
                .get::<Decode>(b"decode_base64\0")
                .expect("load Rust decode_base64");

            Self {
                _c_library: c_library,
                _rust_library: rust_library,
                c_decode,
                rust_decode,
            }
        }
    }
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    fn usize(&mut self, upper: usize) -> usize {
        (self.next_u64() as usize) % upper
    }

    fn pick(&mut self, bytes: &[u8]) -> u8 {
        bytes[self.usize(bytes.len())]
    }
}

const DATA: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const ACCEPTED: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=";
const IGNORED_ASCII: &[u8] = b"!\"#$%&'()*,-.:;<>?@[\\]^_`{|}~ \t\r\n";

fn is_base64(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'=')
}

fn output_len(input: &[u8]) -> usize {
    let filtered: Vec<u8> = input.iter().copied().filter(|b| is_base64(*b)).collect();
    filtered
        .chunks(4)
        .map(|chunk| {
            let c3 = chunk.get(2).copied().unwrap_or(b'A');
            let c4 = chunk.get(3).copied().unwrap_or(b'A');
            1 + usize::from(c3 != b'=') + usize::from(c4 != b'=')
        })
        .sum()
}

fn compare_valid(libraries: &Libraries, input: &[u8]) {
    assert!(!input.is_empty());
    assert!(!input.contains(&0));
    let source = CString::new(input).expect("NUL-free input");
    let length = output_len(input);

    unsafe {
        let c_result = (libraries.c_decode)(source.as_ptr());
        let rust_result = (libraries.rust_decode)(source.as_ptr());
        assert!(!c_result.is_null(), "C returned NULL for {input:?}");
        assert!(!rust_result.is_null(), "Rust returned NULL for {input:?}");

        let c_bytes = slice::from_raw_parts(c_result.cast::<u8>(), length + 1);
        let rust_bytes = slice::from_raw_parts(rust_result.cast::<u8>(), length + 1);
        assert_eq!(c_bytes, rust_bytes, "input: {input:?}");
        assert_eq!(c_bytes[length], 0, "C result was not NUL terminated");
        assert_eq!(rust_bytes[length], 0, "Rust result was not NUL terminated");

        free(c_result.cast());
        free(rust_result.cast());
    }
}

fn random_accepted(rng: &mut Rng, length: usize, alphabet: &[u8]) -> Vec<u8> {
    (0..length).map(|_| rng.pick(alphabet)).collect()
}

fn insert_ignored(rng: &mut Rng, accepted: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(accepted.len() * 3 + 2);
    result.push(rng.pick(IGNORED_ASCII));
    for &byte in accepted {
        if rng.usize(3) != 0 {
            result.push(rng.pick(IGNORED_ASCII));
        }
        result.push(byte);
        if rng.usize(2) == 0 {
            result.push(rng.pick(IGNORED_ASCII));
        }
    }
    result.push(rng.pick(IGNORED_ASCII));
    result
}

#[test]
fn config_01_all_characters_filtered_out() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x01a1_0001);
    for _ in 0..256 {
        let length = 1 + rng.usize(256);
        let input: Vec<u8> = (0..length).map(|_| rng.pick(IGNORED_ASCII)).collect();
        compare_valid(&libraries, &input);
    }
}

fn check_remainder(seed: u64, remainder: usize) {
    let libraries = Libraries::load();
    let mut rng = Rng::new(seed);
    for _ in 0..256 {
        let filtered_len = 4 * rng.usize(24) + remainder;
        let accepted = random_accepted(&mut rng, filtered_len, ACCEPTED);
        let input = insert_ignored(&mut rng, &accepted);
        compare_valid(&libraries, &input);
    }
}

#[test]
fn config_02_filtered_length_one_mod_four() {
    check_remainder(0x02a2_0002, 1);
}

#[test]
fn config_03_filtered_length_two_mod_four() {
    check_remainder(0x03a3_0003, 2);
}

#[test]
fn config_04_filtered_length_three_mod_four() {
    check_remainder(0x04a4_0004, 3);
}

#[test]
fn config_05_complete_unpadded_quartet() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x05a5_0005);
    for _ in 0..512 {
        let input = random_accepted(&mut rng, 4, DATA);
        compare_valid(&libraries, &input);
    }
}

#[test]
fn config_06_double_padding() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x06a6_0006);
    for _ in 0..512 {
        let input = vec![rng.pick(DATA), rng.pick(DATA), b'=', b'='];
        compare_valid(&libraries, &input);
    }
}

#[test]
fn config_07_single_padding() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x07a7_0007);
    for _ in 0..512 {
        let input = vec![rng.pick(DATA), rng.pick(DATA), rng.pick(DATA), b'='];
        compare_valid(&libraries, &input);
    }
}

#[test]
fn config_08_c3_padding_with_c4_data() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x08a8_0008);
    for _ in 0..512 {
        let input = vec![rng.pick(DATA), rng.pick(DATA), b'=', rng.pick(DATA)];
        compare_valid(&libraries, &input);
    }
}

#[test]
fn config_09_padding_in_c1() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x09a9_0009);
    for _ in 0..512 {
        let input = vec![b'=', rng.pick(DATA), rng.pick(DATA), rng.pick(DATA)];
        compare_valid(&libraries, &input);
    }
}

#[test]
fn config_10_padding_in_c2() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x10aa_0010);
    for _ in 0..512 {
        let input = vec![rng.pick(DATA), b'=', rng.pick(DATA), rng.pick(DATA)];
        compare_valid(&libraries, &input);
    }
}

#[test]
fn config_11_multiple_mixed_quartets() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x11ab_0011);
    for _ in 0..256 {
        let quartets = 2 + rng.usize(32);
        let mut input = Vec::with_capacity(quartets * 4);
        for _ in 0..quartets {
            match rng.usize(4) {
                0 => input.extend(random_accepted(&mut rng, 4, DATA)),
                1 => input.extend([rng.pick(DATA), rng.pick(DATA), b'=', b'=']),
                2 => input.extend([rng.pick(DATA), rng.pick(DATA), rng.pick(DATA), b'=']),
                _ => input.extend([rng.pick(DATA), rng.pick(DATA), b'=', rng.pick(DATA)]),
            }
        }
        compare_valid(&libraries, &input);
    }
}

#[test]
fn config_12_ignored_ascii_interspersed() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x12ac_0012);
    for _ in 0..256 {
        let length = 1 + rng.usize(256);
        let accepted = random_accepted(&mut rng, length, ACCEPTED);
        let input = insert_ignored(&mut rng, &accepted);
        compare_valid(&libraries, &input);
    }
}

#[test]
fn config_13_all_decode_classes_and_boundaries() {
    let libraries = Libraries::load();
    let boundaries = b"AZaz09+/=";
    let mut rng = Rng::new(0x13ad_0013);
    for &boundary in boundaries {
        for position in 0..4 {
            for _ in 0..64 {
                let mut input = random_accepted(&mut rng, 4, DATA);
                input[position] = boundary;
                compare_valid(&libraries, &input);
            }
        }
    }
}

#[test]
fn config_14_non_ascii_bytes_are_ignored() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x14ae_0014);
    for _ in 0..256 {
        let length = 1 + rng.usize(128);
        let accepted = random_accepted(&mut rng, length, ACCEPTED);
        let mut input = Vec::with_capacity(accepted.len() * 3);
        input.push(0x80 + rng.usize(128) as u8);
        for byte in accepted {
            input.push(byte);
            input.push(0x80 + rng.usize(128) as u8);
        }
        compare_valid(&libraries, &input);
    }
}

#[test]
fn config_15_long_stress_inputs() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x15af_0015);
    for _ in 0..32 {
        let length = 32_768 + rng.usize(32_768);
        let mut input = Vec::with_capacity(length);
        for _ in 0..length {
            input.push(if rng.usize(5) == 0 {
                rng.pick(IGNORED_ASCII)
            } else {
                rng.pick(ACCEPTED)
            });
        }
        compare_valid(&libraries, &input);
    }
}

#[test]
fn error_01_null_source() {
    let libraries = Libraries::load();
    unsafe {
        let c_result = (libraries.c_decode)(ptr::null());
        let rust_result = (libraries.rust_decode)(ptr::null());
        assert!(c_result.is_null());
        assert!(rust_result.is_null());
    }
}

#[test]
fn error_02_empty_source() {
    let libraries = Libraries::load();
    let empty = CString::new(Vec::<u8>::new()).unwrap();
    unsafe {
        let c_result = (libraries.c_decode)(empty.as_ptr());
        let rust_result = (libraries.rust_decode)(empty.as_ptr());
        assert!(c_result.is_null());
        assert!(rust_result.is_null());
    }
}

fn compile_interposer(output: &Path) {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/support/fail_alloc.c");
    let status = Command::new("cc")
        .args(["-shared", "-fPIC", "-O2"])
        .arg(&source)
        .arg("-o")
        .arg(output)
        .status()
        .expect("run cc for allocation interposer");
    assert!(status.success(), "failed to compile allocation interposer");
    assert!(output.is_file(), "interposer was not created");
}

fn allocation_failure_test(mode: c_int, marker: &str, test_name: &str) {
    if env::var_os(marker).is_none() {
        let test_exe = env::current_exe().expect("current test executable");
        let output = test_exe
            .parent()
            .expect("test executable directory")
            .join(format!("libfail_alloc_{mode}.so"));
        compile_interposer(&output);

        let status = Command::new(&test_exe)
            .env(marker, "1")
            .env("FAIL_ALLOC_PRELOAD", &output)
            .env("LD_PRELOAD", &output)
            .args(["--exact", test_name, "--nocapture"])
            .status()
            .expect("run preloaded child test");
        assert!(status.success(), "preloaded child test failed");
        return;
    }

    let preload_path =
        PathBuf::from(env::var_os("FAIL_ALLOC_PRELOAD").expect("FAIL_ALLOC_PRELOAD in child"));
    unsafe {
        let interposer = Library::new(&preload_path).expect("open allocation interposer");
        let set_fail_mode = *interposer
            .get::<SetFailMode>(b"set_fail_mode\0")
            .expect("load set_fail_mode");
        let libraries = Libraries::load();
        let input = CString::new(b"QUJDRA==".as_slice()).unwrap();

        set_fail_mode(mode);
        let c_result = (libraries.c_decode)(input.as_ptr());
        assert!(c_result.is_null(), "C allocation failure was not observed");

        set_fail_mode(mode);
        let rust_result = (libraries.rust_decode)(input.as_ptr());
        assert!(
            rust_result.is_null(),
            "Rust allocation failure was not observed"
        );
    }
}

#[test]
fn error_03_destination_calloc_failure() {
    allocation_failure_test(
        1,
        "DIFF_CHILD_CALLOC",
        "error_03_destination_calloc_failure",
    );
}

#[test]
fn error_04_temporary_malloc_failure() {
    allocation_failure_test(2, "DIFF_CHILD_MALLOC", "error_04_temporary_malloc_failure");
}
