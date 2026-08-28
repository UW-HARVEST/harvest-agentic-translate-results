use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::slice;

type EncodeBase64 = unsafe extern "C" fn(c_int, *const c_char) -> *mut c_char;

unsafe extern "C" {
    fn free(ptr: *mut c_void);
}

struct Implementations {
    c: Library,
    rust: Library,
}

impl Implementations {
    fn load() -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest.join("../c_src/build/libdriver.so");
        let rust_path = rust_library_path(&manifest);

        assert!(
            c_path.is_file(),
            "C shared library missing: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "Rust shared library missing: {}",
            rust_path.display()
        );

        Self {
            c: unsafe { Library::new(c_path).expect("load C shared library") },
            rust: unsafe { Library::new(rust_path).expect("load Rust shared library") },
        }
    }

    fn compare(&self, size: c_int, input: &[u8], output_len: Option<usize>) {
        let input_ptr = input.as_ptr().cast::<c_char>();
        let c_result = unsafe { self.call(&self.c, size, input_ptr) };
        let rust_result = unsafe { self.call(&self.rust, size, input_ptr) };

        assert_eq!(
            c_result.is_null(),
            rust_result.is_null(),
            "return sentinel differs for size {size}"
        );

        match output_len {
            Some(output_len) => {
                assert!(!c_result.is_null(), "C rejected valid size {size}");
                let c_bytes = unsafe { slice::from_raw_parts(c_result.cast::<u8>(), output_len) };
                let rust_bytes =
                    unsafe { slice::from_raw_parts(rust_result.cast::<u8>(), output_len) };
                assert_eq!(c_bytes, rust_bytes, "output differs for size {size}");

                unsafe {
                    free(c_result.cast::<c_void>());
                    free(rust_result.cast::<c_void>());
                }
            }
            None => {
                assert!(c_result.is_null(), "C did not reject invalid size {size}");
                assert!(
                    rust_result.is_null(),
                    "Rust did not reject invalid size {size}"
                );
            }
        }
    }

    fn compare_null_input(&self, size: c_int) {
        let c_result = unsafe { self.call(&self.c, size, std::ptr::null()) };
        let rust_result = unsafe { self.call(&self.rust, size, std::ptr::null()) };
        assert!(
            c_result.is_null(),
            "C accepted a null input for size {size}"
        );
        assert!(
            rust_result.is_null(),
            "Rust accepted a null input for size {size}"
        );
    }

    unsafe fn call(&self, library: &Library, size: c_int, input: *const c_char) -> *mut c_char {
        let function: Symbol<EncodeBase64> =
            unsafe { library.get(b"encode_base64\0").expect("load encode_base64") };
        unsafe { function(size, input) }
    }
}

fn rust_library_path(manifest: &Path) -> PathBuf {
    manifest.join("target/release/libdriver.so")
}

fn encoded_buffer_len(input_len: usize) -> usize {
    input_len.div_ceil(3) * 4 + 1
}

fn bytes(seed: &mut u64, len: usize, allow_nul: bool) -> Vec<u8> {
    (0..len)
        .map(|_| {
            let mut value = next(seed) as u8;
            if !allow_nul && value == 0 {
                value = 1;
            }
            value
        })
        .collect()
}

fn next(seed: &mut u64) -> u64 {
    let mut value = *seed;
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    *seed = value;
    value
}

fn random_len_with_remainder(seed: &mut u64, remainder: usize) -> usize {
    let groups = (next(seed) as usize % 128) + usize::from(remainder == 0);
    groups * 3 + remainder
}

#[test]
fn config_01_string_mode_empty() {
    let implementations = Implementations::load();
    let mut seed = 0x4d59_5df4_d0f3_3173;

    for _ in 0..256 {
        let mut input = vec![0];
        let ignored_len = next(&mut seed) as usize % 64;
        input.extend(bytes(&mut seed, ignored_len, true));
        implementations.compare(0, &input, Some(1));
    }
}

#[test]
fn config_02_to_04_string_mode_remainders() {
    let implementations = Implementations::load();
    let mut seed = 0xd1b5_4a32_d192_ed03;

    for remainder in 0..=2 {
        for _ in 0..256 {
            let len = random_len_with_remainder(&mut seed, remainder);
            let mut input = bytes(&mut seed, len, false);
            input.push(0);
            implementations.compare(0, &input, Some(encoded_buffer_len(len)));
        }
    }
}

#[test]
fn config_05_to_07_explicit_binary_remainders() {
    let implementations = Implementations::load();
    let mut seed = 0x94d0_49bb_1331_11eb;

    for remainder in 0..=2 {
        for _ in 0..512 {
            let len = random_len_with_remainder(&mut seed, remainder);
            let input = bytes(&mut seed, len, true);
            implementations.compare(
                c_int::try_from(len).unwrap(),
                &input,
                Some(encoded_buffer_len(len)),
            );
        }
    }
}

#[test]
fn config_08_negative_sizes_accepted_without_encoding() {
    let implementations = Implementations::load();
    let mut seed = 0x853c_49e6_748f_ea9b;

    for size in -3..=-1 {
        for _ in 0..128 {
            let input = bytes(&mut seed, 16, true);
            let readable_bytes = usize::try_from(size * 4 / 3 + 4).unwrap();
            implementations.compare(size, &input, Some(readable_bytes));
        }
    }
}

#[test]
fn config_09_to_13_all_alphabet_branches() {
    let implementations = Implementations::load();
    let mut seed = 0xda94_2042_e4dd_58b5;
    let classes = [(0, 25), (26, 51), (52, 61), (62, 62), (63, 63)];

    for (minimum, maximum) in classes {
        for _ in 0..256 {
            let six_bits = minimum + (next(&mut seed) as u8 % (maximum - minimum + 1));
            let input = [
                (six_bits << 2) | (next(&mut seed) as u8 & 0x03),
                next(&mut seed) as u8,
                next(&mut seed) as u8,
            ];
            implementations.compare(3, &input, Some(encoded_buffer_len(input.len())));
        }
    }
}

#[test]
fn boundary_large_explicit_length() {
    let implementations = Implementations::load();
    let mut seed = 0x9e37_79b9_7f4a_7c15;
    let input = bytes(&mut seed, 1_000_003, true);

    implementations.compare(
        c_int::try_from(input.len()).unwrap(),
        &input,
        Some(encoded_buffer_len(input.len())),
    );
}

#[test]
fn error_01_null_input_returns_null() {
    let implementations = Implementations::load();

    for size in [c_int::MIN, -1, 0, 1, c_int::MAX] {
        implementations.compare_null_input(size);
    }
}

#[test]
fn error_02_oversized_allocation_returns_null() {
    let implementations = Implementations::load();
    let input = [0_u8];

    // For these negative sizes the C allocation expression is negative, then
    // converts to an impossibly large size_t. calloc rejects it before input
    // is read, exercising the allocation-failure branch without overcommit.
    for size in [-4, -1024, -1_000_000] {
        implementations.compare(size, &input, None);
    }
}
