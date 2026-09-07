use libloading::Library;
use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::ptr;
use std::slice;

type EncodeBase64 = unsafe extern "C" fn(c_int, *const c_char) -> *mut c_char;

unsafe extern "C" {
    fn free(ptr: *mut c_void);
}

struct Implementations {
    c_encode: EncodeBase64,
    rust_encode: EncodeBase64,
    _c_library: Library,
    _rust_library: Library,
}

impl Implementations {
    fn load() -> Self {
        let c_path = workspace_path("../c_src/build/libdriver.so");
        let rust_path = workspace_path("target/release/libdriver.so");
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
            let c_library = Library::new(&c_path).expect("load C shared library");
            let rust_library = Library::new(&rust_path).expect("load Rust shared library");
            let c_encode = *c_library
                .get::<EncodeBase64>(b"encode_base64\0")
                .expect("load C encode_base64");
            let rust_encode = *rust_library
                .get::<EncodeBase64>(b"encode_base64\0")
                .expect("load Rust encode_base64");

            Self {
                c_encode,
                rust_encode,
                _c_library: c_library,
                _rust_library: rust_library,
            }
        }
    }

    fn compare_success(&self, size: c_int, input: &[u8], encoded_len: usize) -> Vec<u8> {
        assert!(!input.is_empty(), "FFI input must have an address");
        unsafe {
            let c_ptr = (self.c_encode)(size, input.as_ptr().cast());
            let rust_ptr = (self.rust_encode)(size, input.as_ptr().cast());
            assert!(
                !c_ptr.is_null(),
                "C unexpectedly returned NULL for size {size}"
            );
            assert!(
                !rust_ptr.is_null(),
                "Rust unexpectedly returned NULL for size {size}"
            );

            let c_bytes = slice::from_raw_parts(c_ptr.cast::<u8>(), encoded_len + 1).to_vec();
            let rust_bytes = slice::from_raw_parts(rust_ptr.cast::<u8>(), encoded_len + 1).to_vec();
            free(c_ptr.cast());
            free(rust_ptr.cast());

            assert_eq!(c_bytes, rust_bytes, "output mismatch for size {size}");
            assert_eq!(c_bytes[encoded_len], 0, "C output is not NUL terminated");
            c_bytes
        }
    }

    fn compare_null(&self, size: c_int, input: *const c_char) {
        unsafe {
            let c_ptr = (self.c_encode)(size, input);
            let rust_ptr = (self.rust_encode)(size, input);
            assert!(c_ptr.is_null(), "C returned non-NULL for size {size}");
            assert!(rust_ptr.is_null(), "Rust returned non-NULL for size {size}");
        }
    }
}

fn workspace_path(relative: impl AsRef<Path>) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn encoded_len(input_len: usize) -> usize {
    input_len.div_ceil(3) * 4
}

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

    fn byte(&mut self) -> u8 {
        self.next_u64() as u8
    }

    fn nonzero_byte(&mut self) -> u8 {
        loop {
            let value = self.byte();
            if value != 0 {
                return value;
            }
        }
    }
}

fn length_with_remainder(rng: &mut FixedRng, remainder: usize) -> usize {
    let groups = (rng.next_u64() as usize % 64) + usize::from(remainder == 0);
    groups * 3 + remainder
}

#[test]
fn config_01_negative_sizes_with_successful_allocation() {
    let implementations = Implementations::load();
    let mut rng = FixedRng::new(0x4e45_4741_5449_5645);

    for _ in 0..128 {
        let input = [rng.byte()];
        implementations.compare_success(-1, &input, 0);
        implementations.compare_success(-2, &input, 0);

        unsafe {
            let c_ptr = (implementations.c_encode)(-3, input.as_ptr().cast());
            let rust_ptr = (implementations.rust_encode)(-3, input.as_ptr().cast());
            assert_eq!(c_ptr.is_null(), rust_ptr.is_null());
            if !c_ptr.is_null() {
                free(c_ptr.cast());
            }
            if !rust_ptr.is_null() {
                free(rust_ptr.cast());
            }
        }
    }
}

#[test]
fn config_02_inferred_empty_string() {
    let implementations = Implementations::load();
    for _ in 0..128 {
        implementations.compare_success(0, &[0], 0);
    }
}

fn compare_random_inferred(remainder: usize, seed: u64) {
    let implementations = Implementations::load();
    let mut rng = FixedRng::new(seed);

    for _ in 0..256 {
        let len = length_with_remainder(&mut rng, remainder);
        let mut input = Vec::with_capacity(len + 1);
        input.extend((0..len).map(|_| rng.nonzero_byte()));
        input.push(0);
        implementations.compare_success(0, &input, encoded_len(len));
    }
}

#[test]
fn config_03_inferred_length_modulo_one() {
    compare_random_inferred(1, 0x494e_4645_5252_4541);
}

#[test]
fn config_04_inferred_length_modulo_two() {
    compare_random_inferred(2, 0x494e_4645_5252_4542);
}

#[test]
fn config_05_inferred_length_modulo_zero() {
    compare_random_inferred(0, 0x494e_4645_5252_4543);
}

fn compare_random_explicit(remainder: usize, seed: u64) {
    let implementations = Implementations::load();
    let mut rng = FixedRng::new(seed);

    for _ in 0..256 {
        let len = length_with_remainder(&mut rng, remainder);
        let input: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
        implementations.compare_success(len as c_int, &input, encoded_len(len));
    }
}

#[test]
fn config_06_explicit_length_modulo_one() {
    compare_random_explicit(1, 0x4558_504c_4943_4951);
}

#[test]
fn config_07_explicit_length_modulo_two() {
    compare_random_explicit(2, 0x4558_504c_4943_4952);
}

#[test]
fn config_08_explicit_length_modulo_zero() {
    compare_random_explicit(0, 0x4558_504c_4943_4953);
}

#[test]
fn config_09_explicit_size_includes_embedded_nuls() {
    let implementations = Implementations::load();
    let mut rng = FixedRng::new(0x454d_4245_4444_4544);

    for _ in 0..256 {
        let len = (rng.next_u64() as usize % 190) + 3;
        let mut input: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
        let nul_index = (rng.next_u64() as usize % (len - 2)) + 1;
        input[nul_index] = 0;
        input[nul_index + 1] = rng.nonzero_byte();
        implementations.compare_success(len as c_int, &input, encoded_len(len));
    }
}

#[test]
fn config_10_explicit_size_uses_only_backing_prefix() {
    let implementations = Implementations::load();
    let mut rng = FixedRng::new(0x5052_4546_4958_4f4e);

    for _ in 0..256 {
        let backing_len = (rng.next_u64() as usize % 192) + 2;
        let explicit_len = (rng.next_u64() as usize % (backing_len - 1)) + 1;
        let input: Vec<u8> = (0..backing_len).map(|_| rng.byte()).collect();
        implementations.compare_success(explicit_len as c_int, &input, encoded_len(explicit_len));
    }
}

#[test]
fn config_11_explicit_high_bit_bytes() {
    let implementations = Implementations::load();
    let mut rng = FixedRng::new(0x4849_4748_4249_5453);

    for _ in 0..256 {
        let len = (rng.next_u64() as usize % 192) + 1;
        let input: Vec<u8> = (0..len).map(|_| rng.byte() | 0x80).collect();
        implementations.compare_success(len as c_int, &input, encoded_len(len));
    }
}

#[test]
fn config_12_all_encode_alphabet_branches() {
    let implementations = Implementations::load();
    let first_sextet_cases = [
        (0_u8, b'A'),
        (25_u8, b'Z'),
        (26_u8, b'a'),
        (51_u8, b'z'),
        (52_u8, b'0'),
        (61_u8, b'9'),
        (62_u8, b'+'),
        (63_u8, b'/'),
    ];

    for _ in 0..128 {
        for &(sextet, expected) in &first_sextet_cases {
            let input = [sextet << 2];
            let output = implementations.compare_success(1, &input, 4);
            assert_eq!(output[0], expected);
        }
    }
}

#[test]
fn error_01_null_source_returns_null_for_all_size_classes() {
    let implementations = Implementations::load();
    for size in [c_int::MIN, -4, -1, 0, 1, c_int::MAX] {
        implementations.compare_null(size, ptr::null());
    }
}

#[test]
fn error_02_oversized_allocation_returns_null() {
    let implementations = Implementations::load();
    let mut rng = FixedRng::new(0x4f56_4552_5349_5a45);

    // In C, size == -4 makes size * 4 / 3 + 4 equal -1. Conversion to
    // calloc's size_t requests SIZE_MAX bytes, which must fail.
    for _ in 0..128 {
        let input = [rng.byte()];
        implementations.compare_null(-4, input.as_ptr().cast());
    }
}
