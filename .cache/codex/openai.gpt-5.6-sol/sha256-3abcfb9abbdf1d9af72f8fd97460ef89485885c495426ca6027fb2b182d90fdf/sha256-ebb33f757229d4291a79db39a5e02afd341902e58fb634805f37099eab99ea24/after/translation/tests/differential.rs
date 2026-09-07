use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::ptr;

type Hex2Bin = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const c_char,
    usize,
    *const c_char,
    *mut *const c_char,
) -> c_int;

struct Libraries {
    _c: Library,
    _rust: Library,
    c: Hex2Bin,
    rust: Hex2Bin,
}

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    ret: c_int,
    output: Vec<u8>,
    end_offset: Option<isize>,
}

#[derive(Clone, Copy)]
struct CallOptions {
    bin_maxlen: usize,
    hex_len: usize,
    want_end: bool,
    null_bin: bool,
    null_hex: bool,
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn usize(&mut self, upper: usize) -> usize {
        (self.next() as usize) % upper
    }

    fn byte(&mut self) -> u8 {
        self.next() as u8
    }
}

impl Libraries {
    fn load() -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = find_c_library(&manifest.join("../c_src/build"));
        let rust_path = manifest.join("target/release/libhex2bin_lib.so");
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
            let c_lib = Library::new(&c_path).unwrap();
            let rust_lib = Library::new(&rust_path).unwrap();
            let c_symbol: Symbol<'_, Hex2Bin> = c_lib.get(b"hex2bin\0").unwrap();
            let rust_symbol: Symbol<'_, Hex2Bin> = rust_lib.get(b"hex2bin\0").unwrap();
            let c = *c_symbol;
            let rust = *rust_symbol;
            Self {
                _c: c_lib,
                _rust: rust_lib,
                c,
                rust,
            }
        }
    }
}

fn find_c_library(build_dir: &Path) -> PathBuf {
    let mut candidates: Vec<_> = std::fs::read_dir(build_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("lib") && name.ends_with(".so"))
        })
        .collect();
    candidates.sort();
    assert_eq!(
        candidates.len(),
        1,
        "unexpected C libraries: {candidates:?}"
    );
    candidates.remove(0)
}

fn invoke(function: Hex2Bin, input: &[u8], ignore: Option<&[u8]>, options: CallOptions) -> Outcome {
    let storage_len = options.bin_maxlen.clamp(1, 4096) + 8;
    let mut output = vec![0xa5; storage_len];
    let bin_ptr = if options.null_bin {
        ptr::null_mut()
    } else {
        output.as_mut_ptr()
    };
    let hex_ptr = if options.null_hex {
        ptr::null()
    } else {
        input.as_ptr().cast::<c_char>()
    };
    let ignore_storage = ignore.map(|bytes| {
        let mut storage = bytes.to_vec();
        storage.push(0);
        storage
    });
    let ignore_ptr = ignore_storage
        .as_ref()
        .map_or(ptr::null(), |bytes| bytes.as_ptr().cast::<c_char>());
    let mut end = ptr::null();
    let end_ptr = if options.want_end {
        &mut end
    } else {
        ptr::null_mut()
    };

    let ret = unsafe {
        function(
            bin_ptr,
            options.bin_maxlen,
            hex_ptr,
            options.hex_len,
            ignore_ptr,
            end_ptr,
        )
    };
    let end_offset = options.want_end.then(|| {
        assert!(!end.is_null(), "function did not initialize hex_end_p");
        unsafe { end.offset_from(hex_ptr) }
    });
    Outcome {
        ret,
        output,
        end_offset,
    }
}

fn assert_differential(
    libs: &Libraries,
    input: &[u8],
    ignore: Option<&[u8]>,
    options: CallOptions,
) -> Outcome {
    let c = invoke(libs.c, input, ignore, options);
    let rust = invoke(libs.rust, input, ignore, options);
    assert_eq!(rust, c, "input={input:?}, ignore={ignore:?}");
    c
}

fn options(bin_maxlen: usize, hex_len: usize, want_end: bool) -> CallOptions {
    CallOptions {
        bin_maxlen,
        hex_len,
        want_end,
        null_bin: false,
        null_hex: false,
    }
}

fn hex_digit(nibble: u8, upper: bool) -> u8 {
    match nibble {
        0..=9 => b'0' + nibble,
        10..=15 if upper => b'A' + nibble - 10,
        10..=15 => b'a' + nibble - 10,
        _ => unreachable!(),
    }
}

fn encode(bytes: &[u8], rng: &mut Rng) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        encoded.push(hex_digit(byte >> 4, rng.usize(2) == 0));
        encoded.push(hex_digit(byte & 0x0f, rng.usize(2) == 0));
    }
    encoded
}

fn expected_success(outcome: &Outcome, expected: &[u8], end: Option<isize>) {
    assert_eq!(outcome.ret, expected.len() as c_int);
    assert_eq!(&outcome.output[..expected.len()], expected);
    assert_eq!(outcome.end_offset, end);
}

#[test]
fn config_01_empty_zero_capacity_null_options() {
    let libs = Libraries::load();
    for _ in 0..64 {
        let result = assert_differential(&libs, &[], None, options(0, 0, false));
        expected_success(&result, &[], None);
    }
}

#[test]
fn config_02_empty_spare_capacity_ignore_and_end() {
    let libs = Libraries::load();
    let mut rng = Rng::new(0x0202_0202);
    for _ in 0..128 {
        let cap = 1 + rng.usize(64);
        let ignored = [b'!', b'_', 1 + rng.byte() % 0x7e];
        let result = assert_differential(&libs, &[], Some(&ignored), options(cap, 0, true));
        expected_success(&result, &[], Some(0));
    }
}

#[test]
fn config_03_one_decimal_byte_exact_capacity() {
    let libs = Libraries::load();
    let mut rng = Rng::new(0x0303_0303);
    for _ in 0..256 {
        let hi = rng.usize(10) as u8;
        let lo = rng.usize(10) as u8;
        let input = [b'0' + hi, b'0' + lo];
        let result = assert_differential(&libs, &input, None, options(1, 2, false));
        expected_success(&result, &[hi * 16 + lo], None);
    }
}

#[test]
fn config_04_one_lowercase_alpha_byte() {
    let libs = Libraries::load();
    let mut rng = Rng::new(0x0404_0404);
    for _ in 0..256 {
        let hi = 10 + rng.usize(6) as u8;
        let lo = 10 + rng.usize(6) as u8;
        let input = [hex_digit(hi, false), hex_digit(lo, false)];
        let result = assert_differential(&libs, &input, None, options(1, 2, true));
        expected_success(&result, &[hi * 16 + lo], Some(2));
    }
}

#[test]
fn config_05_one_uppercase_alpha_byte_spare_capacity() {
    let libs = Libraries::load();
    let mut rng = Rng::new(0x0505_0505);
    for _ in 0..256 {
        let hi = 10 + rng.usize(6) as u8;
        let lo = 10 + rng.usize(6) as u8;
        let input = [hex_digit(hi, true), hex_digit(lo, true)];
        let result = assert_differential(&libs, &input, None, options(8, 2, true));
        expected_success(&result, &[hi * 16 + lo], Some(2));
    }
}

#[test]
fn config_06_mixed_numeric_and_alpha_classes() {
    let libs = Libraries::load();
    let mut rng = Rng::new(0x0606_0606);
    for i in 0..256 {
        let num = rng.usize(10) as u8;
        let alpha = 10 + rng.usize(6) as u8;
        let (input, expected) = if i % 2 == 0 {
            (
                [hex_digit(num, false), hex_digit(alpha, i % 4 == 0)],
                num * 16 + alpha,
            )
        } else {
            (
                [hex_digit(alpha, i % 3 == 0), hex_digit(num, false)],
                alpha * 16 + num,
            )
        };
        let result = assert_differential(&libs, &input, None, options(1, 2, i % 5 == 0));
        expected_success(&result, &[expected], (i % 5 == 0).then_some(2));
    }
}

#[test]
fn config_07_many_bytes_exact_capacity() {
    let libs = Libraries::load();
    let mut rng = Rng::new(0x0707_0707);
    for _ in 0..256 {
        let mut expected = vec![0; 2 + rng.usize(63)];
        for byte in &mut expected {
            *byte = rng.byte();
        }
        let input = encode(&expected, &mut rng);
        let result = assert_differential(
            &libs,
            &input,
            None,
            options(expected.len(), input.len(), false),
        );
        expected_success(&result, &expected, None);
    }
}

#[test]
fn config_08_many_bytes_spare_capacity_and_end() {
    let libs = Libraries::load();
    let mut rng = Rng::new(0x0808_0808);
    for _ in 0..256 {
        let mut expected = vec![0; 2 + rng.usize(63)];
        for byte in &mut expected {
            *byte = rng.byte();
        }
        let input = encode(&expected, &mut rng);
        let cap = expected.len() + 1 + rng.usize(32);
        let result = assert_differential(&libs, &input, None, options(cap, input.len(), true));
        expected_success(&result, &expected, Some(input.len() as isize));
    }
}

#[test]
fn config_09_nonmatching_ignore_string() {
    let libs = Libraries::load();
    let mut rng = Rng::new(0x0909_0909);
    for _ in 0..256 {
        let mut expected = vec![0; 1 + rng.usize(48)];
        for byte in &mut expected {
            *byte = rng.byte();
        }
        let input = encode(&expected, &mut rng);
        let result =
            assert_differential(&libs, &input, Some(b"!_:"), options(64, input.len(), true));
        expected_success(&result, &expected, Some(input.len() as isize));
    }
}

#[test]
fn config_10_valid_digits_are_not_ignored() {
    let libs = Libraries::load();
    let mut rng = Rng::new(0x1010_1010);
    for _ in 0..256 {
        let mut expected = vec![0; 1 + rng.usize(48)];
        for byte in &mut expected {
            *byte = rng.byte();
        }
        let input = encode(&expected, &mut rng);
        let result = assert_differential(
            &libs,
            &input,
            Some(b"0123456789abcdefABCDEF"),
            options(expected.len(), input.len(), false),
        );
        expected_success(&result, &expected, None);
    }
}

#[test]
fn config_11_single_separator_at_byte_boundaries() {
    let libs = Libraries::load();
    let mut rng = Rng::new(0x1111_1111);
    for _ in 0..256 {
        let mut expected = vec![0; 2 + rng.usize(48)];
        for byte in &mut expected {
            *byte = rng.byte();
        }
        let plain = encode(&expected, &mut rng);
        let mut input = Vec::new();
        for (index, pair) in plain.chunks_exact(2).enumerate() {
            if index != 0 {
                input.push(b':');
            }
            input.extend_from_slice(pair);
        }
        let result = assert_differential(&libs, &input, Some(b":"), options(64, input.len(), true));
        expected_success(&result, &expected, Some(input.len() as isize));
    }
}

#[test]
fn config_12_multiple_leading_internal_trailing_separators() {
    let libs = Libraries::load();
    let mut rng = Rng::new(0x1212_1212);
    for _ in 0..256 {
        let mut expected = vec![0; 1 + rng.usize(40)];
        for byte in &mut expected {
            *byte = rng.byte();
        }
        let plain = encode(&expected, &mut rng);
        let separators = [b' ', b':', b'_', b'-'];
        let mut input = vec![b' ', b'_'];
        for pair in plain.chunks_exact(2) {
            input.extend_from_slice(pair);
            for _ in 0..rng.usize(4) {
                input.push(separators[rng.usize(separators.len())]);
            }
        }
        input.extend_from_slice(b"--:");
        let result = assert_differential(
            &libs,
            &input,
            Some(&separators),
            options(64, input.len(), true),
        );
        expected_success(&result, &expected, Some(input.len() as isize));
    }
}

#[test]
fn config_13_embedded_nul_is_ignored_with_nonnull_ignore() {
    let libs = Libraries::load();
    let mut rng = Rng::new(0x1313_1313);
    for _ in 0..256 {
        let mut expected = vec![0; 1 + rng.usize(40)];
        for byte in &mut expected {
            *byte = rng.byte();
        }
        let plain = encode(&expected, &mut rng);
        let mut input = Vec::new();
        for pair in plain.chunks_exact(2) {
            input.extend_from_slice(pair);
            input.push(0);
        }
        let result = assert_differential(&libs, &input, Some(b"!"), options(64, input.len(), true));
        expected_success(&result, &expected, Some(input.len() as isize));
    }
}

#[test]
fn config_14_invalid_first_byte_returns_empty_prefix() {
    let libs = Libraries::load();
    let invalid = [b'!', b'g', b'G', b'/', b':', 0x7f, 0x80, 0xff];
    let mut rng = Rng::new(0x1414_1414);
    for _ in 0..256 {
        let input = [invalid[rng.usize(invalid.len())], rng.byte()];
        let result = assert_differential(&libs, &input, None, options(8, input.len(), true));
        expected_success(&result, &[], Some(0));
    }
}

#[test]
fn config_15_invalid_ascii_after_complete_prefix() {
    let libs = Libraries::load();
    let invalid = [b'!', b'g', b'G', b'/', b':', b'@', b'`', b'{'];
    let mut rng = Rng::new(0x1515_1515);
    for _ in 0..256 {
        let mut expected = vec![0; 1 + rng.usize(48)];
        for byte in &mut expected {
            *byte = rng.byte();
        }
        let mut input = encode(&expected, &mut rng);
        input.push(invalid[rng.usize(invalid.len())]);
        input.extend_from_slice(b"00");
        let end = (input.len() - 3) as isize;
        let result = assert_differential(&libs, &input, None, options(64, input.len(), true));
        expected_success(&result, &expected, Some(end));
    }
}

#[test]
fn config_16_invalid_high_bit_byte_after_prefix() {
    let libs = Libraries::load();
    let mut rng = Rng::new(0x1616_1616);
    for _ in 0..256 {
        let mut expected = vec![0; 1 + rng.usize(48)];
        for byte in &mut expected {
            *byte = rng.byte();
        }
        let mut input = encode(&expected, &mut rng);
        let end = input.len() as isize;
        input.push(0x80 + rng.usize(128) as u8);
        let result = assert_differential(&libs, &input, None, options(64, input.len(), true));
        expected_success(&result, &expected, Some(end));
    }
}

#[test]
fn config_17_bytes_beyond_hex_len_are_not_read() {
    let libs = Libraries::load();
    let mut rng = Rng::new(0x1717_1717);
    for _ in 0..256 {
        let mut expected = vec![0; 1 + rng.usize(48)];
        for byte in &mut expected {
            *byte = rng.byte();
        }
        let mut input = encode(&expected, &mut rng);
        let selected_len = input.len();
        input.extend_from_slice(&[b'!', 0, 0xff, b'f']);
        let result = assert_differential(&libs, &input, None, options(64, selected_len, true));
        expected_success(&result, &expected, Some(selected_len as isize));
    }
}

#[test]
fn config_18_null_output_when_no_bytes_are_written() {
    let libs = Libraries::load();
    for want_end in [false, true] {
        for cap in [0, 1, 17, 256] {
            let mut call = options(cap, 0, want_end);
            call.null_bin = true;
            let result = assert_differential(&libs, &[], None, call);
            expected_success(&result, &[], want_end.then_some(0));
        }
    }
}

#[test]
fn config_19_null_hex_with_zero_length() {
    let libs = Libraries::load();
    let mut rng = Rng::new(0x1919_1919);
    for _ in 0..128 {
        let mut call = options(rng.usize(64), 0, false);
        call.null_hex = true;
        let result = assert_differential(&libs, &[], None, call);
        expected_success(&result, &[], None);
    }
}

#[test]
fn error_01_output_capacity_exhausted() {
    let libs = Libraries::load();
    let mut rng = Rng::new(0xe101_e101);
    for _ in 0..512 {
        let mut expected = vec![0; 1 + rng.usize(64)];
        for byte in &mut expected {
            *byte = rng.byte();
        }
        let input = encode(&expected, &mut rng);
        let cap = rng.usize(expected.len());
        let result = assert_differential(&libs, &input, None, options(cap, input.len(), true));
        assert_eq!(result.ret, -1);
        assert_eq!(result.end_offset, Some((cap * 2) as isize));
        assert_eq!(&result.output[..cap], &expected[..cap]);
    }
}

#[test]
fn error_02_unmatched_high_nibble() {
    let libs = Libraries::load();
    let mut rng = Rng::new(0xe202_e202);
    for iteration in 0..512 {
        let mut prefix = vec![0; rng.usize(48)];
        for byte in &mut prefix {
            *byte = rng.byte();
        }
        let mut input = encode(&prefix, &mut rng);
        input.push(hex_digit(rng.usize(16) as u8, rng.usize(2) == 0));
        if iteration % 2 == 0 {
            input.push(b'!');
        }
        let odd_offset = (prefix.len() * 2) as isize;
        let result = assert_differential(
            &libs,
            &input,
            (iteration % 3 == 0).then_some(&b"!"[..]),
            options(64, input.len(), true),
        );
        assert_eq!(result.ret, -1);
        assert_eq!(result.end_offset, Some(odd_offset));
        assert_eq!(&result.output[..prefix.len()], &prefix);
    }
}

#[test]
fn error_03_unconsumed_input_without_end_pointer() {
    let libs = Libraries::load();
    let mut rng = Rng::new(0xe303_e303);
    let invalid = [b'!', b'g', b'G', b'/', b':', 0x80, 0xff];
    for _ in 0..512 {
        let mut prefix = vec![0; rng.usize(48)];
        for byte in &mut prefix {
            *byte = rng.byte();
        }
        let mut input = encode(&prefix, &mut rng);
        input.push(invalid[rng.usize(invalid.len())]);
        input.extend_from_slice(b"00");
        let result = assert_differential(&libs, &input, None, options(64, input.len(), false));
        assert_eq!(result.ret, -1);
        assert_eq!(result.end_offset, None);
        assert_eq!(&result.output[..prefix.len()], &prefix);
    }
}

#[test]
fn generic_boundaries_zero_null_and_oversized_lengths() {
    let libs = Libraries::load();

    let empty = assert_differential(&libs, &[], None, options(0, 0, false));
    expected_success(&empty, &[], None);

    let mut null_bin = options(0, 0, false);
    null_bin.null_bin = true;
    expected_success(&assert_differential(&libs, &[], None, null_bin), &[], None);

    let mut null_hex = options(0, 0, false);
    null_hex.null_hex = true;
    expected_success(&assert_differential(&libs, &[], None, null_hex), &[], None);

    let oversized = assert_differential(&libs, b"!", None, options(0, usize::MAX, true));
    expected_success(&oversized, &[], Some(0));

    let oversized_capacity = assert_differential(&libs, b"!", None, options(usize::MAX, 1, true));
    expected_success(&oversized_capacity, &[], Some(0));

    let null_ignore = assert_differential(&libs, b"00", None, options(1, 2, false));
    expected_success(&null_ignore, &[0], None);
}

fn run_crash_probe(library: &str, pointer: &str) -> ExitStatus {
    Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "null_pointer_crash_probe", "--nocapture"])
        .env("HEX2BIN_CRASH_LIBRARY", library)
        .env("HEX2BIN_CRASH_POINTER", pointer)
        .status()
        .unwrap()
}

#[test]
fn null_pointer_dereference_behavior_matches() {
    for pointer in ["bin", "hex"] {
        let c = run_crash_probe("c", pointer);
        let rust = run_crash_probe("rust", pointer);
        assert!(!c.success(), "C unexpectedly accepted null {pointer}");
        assert!(!rust.success(), "Rust unexpectedly accepted null {pointer}");
        assert_eq!(
            rust.signal(),
            c.signal(),
            "different process signal for null {pointer}: C={c:?}, Rust={rust:?}"
        );
    }
}

#[test]
fn null_pointer_crash_probe() {
    let Ok(library) = std::env::var("HEX2BIN_CRASH_LIBRARY") else {
        return;
    };
    let pointer = std::env::var("HEX2BIN_CRASH_POINTER").unwrap();
    let libs = Libraries::load();
    let function = match library.as_str() {
        "c" => libs.c,
        "rust" => libs.rust,
        _ => panic!("unknown library selector"),
    };

    unsafe {
        match pointer.as_str() {
            "bin" => {
                function(
                    ptr::null_mut(),
                    1,
                    b"00".as_ptr().cast(),
                    2,
                    ptr::null(),
                    ptr::null_mut(),
                );
            }
            "hex" => {
                let mut output = [0u8; 1];
                function(
                    output.as_mut_ptr(),
                    1,
                    ptr::null(),
                    1,
                    ptr::null(),
                    ptr::null_mut(),
                );
            }
            _ => panic!("unknown pointer selector"),
        }
    }
    panic!("{library} unexpectedly returned after dereferencing null {pointer}");
}

#[test]
fn broad_randomized_differential_fuzz() {
    let libs = Libraries::load();
    let mut rng = Rng::new(0xd1ff_e2e1_5eed);
    for _ in 0..20_000 {
        let len = rng.usize(65);
        let mut input = vec![0; len];
        for byte in &mut input {
            *byte = rng.byte();
        }
        let ignore_len = rng.usize(9);
        let mut ignore = vec![0; ignore_len];
        for byte in &mut ignore {
            *byte = 1 + rng.byte() % 255;
        }
        let ignore = (rng.usize(3) != 0).then_some(ignore.as_slice());
        let cap = rng.usize(34);
        let want_end = rng.usize(2) == 0;
        assert_differential(&libs, &input, ignore, options(cap, input.len(), want_end));
    }
}
