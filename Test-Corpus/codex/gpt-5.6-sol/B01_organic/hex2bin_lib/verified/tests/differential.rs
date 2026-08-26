use libloading::Library;
use std::ffi::{c_char, c_int};
use std::path::{Path, PathBuf};

type Hex2Bin = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const c_char,
    usize,
    *const c_char,
    *mut *const c_char,
) -> c_int;

const CASES: usize = 128;
const INVALID: &[u8] = b"/:@G`g? \t\x80\xff";

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    ret: c_int,
    output: Vec<u8>,
    end_offset: Option<usize>,
}

struct Implementations {
    c: Library,
    rust: Library,
}

impl Implementations {
    fn load() -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("c_src/build/libtranslated_rust.so");
        let rust_path = rust_library_path();
        assert!(
            c_path.is_file(),
            "missing C library {}; build c_src first",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "missing Rust library {}",
            rust_path.display()
        );

        // The paths are fixed build artifacts under this workspace.
        let c = unsafe { Library::new(&c_path) }.expect("load C shared library");
        let rust = unsafe { Library::new(&rust_path) }.expect("load Rust shared library");
        Self { c, rust }
    }

    fn compare(&self, case: &Case<'_>) -> Outcome {
        let c = invoke(&self.c, case);
        let rust = invoke(&self.rust, case);
        assert_eq!(c, rust, "case: {case:?}");
        c
    }
}

#[derive(Debug)]
struct Case<'a> {
    input: &'a [u8],
    max_len: usize,
    ignore: Option<&'a [u8]>,
    end_pointer: bool,
    null_bin: bool,
    null_hex: bool,
    fill: u8,
}

impl<'a> Case<'a> {
    fn new(input: &'a [u8], max_len: usize) -> Self {
        Self {
            input,
            max_len,
            ignore: None,
            end_pointer: false,
            null_bin: false,
            null_hex: false,
            fill: 0xa5,
        }
    }
}

fn rust_library_path() -> PathBuf {
    if let Some(path) = std::env::var_os("HEX2BIN_RUST_SO") {
        return path.into();
    }

    let exe = std::env::current_exe().expect("resolve test executable");
    let profile_dir = exe
        .parent()
        .and_then(Path::parent)
        .expect("test executable should be under target/<profile>/deps");
    let direct = profile_dir.join("libhex2bin_lib.so");
    if direct.is_file() {
        return direct;
    }

    let deps = profile_dir.join("deps");
    let mut candidates: Vec<_> = std::fs::read_dir(&deps)
        .unwrap_or_else(|error| panic!("read {}: {error}", deps.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("libhex2bin_lib") && name.ends_with(".so"))
        })
        .collect();
    candidates.sort();
    candidates
        .pop()
        .unwrap_or_else(|| panic!("no Rust cdylib found under {}", profile_dir.display()))
}

fn invoke(library: &Library, case: &Case<'_>) -> Outcome {
    let function = unsafe {
        library
            .get::<Hex2Bin>(b"hex2bin\0")
            .expect("resolve hex2bin")
    };

    let mut output = vec![case.fill; case.max_len.max(8)];
    let bin = if case.null_bin {
        std::ptr::null_mut()
    } else {
        output.as_mut_ptr()
    };
    let hex = if case.null_hex {
        std::ptr::null()
    } else {
        case.input.as_ptr().cast::<c_char>()
    };
    let ignore_storage = case.ignore.map(|bytes| {
        assert!(!bytes.contains(&0), "ignore must be a C string");
        let mut storage = bytes.to_vec();
        storage.push(0);
        storage
    });
    let ignore = ignore_storage
        .as_ref()
        .map_or(std::ptr::null(), |bytes| bytes.as_ptr().cast::<c_char>());
    let mut end = std::ptr::without_provenance::<c_char>(1);
    let end_pointer = if case.end_pointer {
        &mut end
    } else {
        std::ptr::null_mut()
    };

    let ret = unsafe {
        function(
            bin,
            case.max_len,
            hex,
            case.input.len(),
            ignore,
            end_pointer,
        )
    };
    let end_offset = case.end_pointer.then(|| {
        if end.is_null() {
            usize::MAX
        } else {
            end.addr().wrapping_sub(hex.addr())
        }
    });

    Outcome {
        ret,
        output,
        end_offset,
    }
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn usize(&mut self, upper: usize) -> usize {
        (self.next() as usize) % upper
    }

    fn byte(&mut self) -> u8 {
        self.next() as u8
    }
}

fn digit(nibble: u8, rng: &mut Rng) -> u8 {
    match nibble {
        0..=9 => b'0' + nibble,
        _ if rng.usize(2) == 0 => b'a' + nibble - 10,
        _ => b'A' + nibble - 10,
    }
}

fn encode(bytes: &[u8], rng: &mut Rng) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(digit(byte >> 4, rng));
        encoded.push(digit(byte & 0x0f, rng));
    }
    encoded
}

fn random_bytes(rng: &mut Rng, len: usize) -> Vec<u8> {
    (0..len).map(|_| rng.byte()).collect()
}

fn random_bytes_in(rng: &mut Rng, minimum: usize, width: usize) -> Vec<u8> {
    let len = minimum + rng.usize(width);
    random_bytes(rng, len)
}

#[test]
fn config_c01_empty_zero_capacity_without_end_pointer() {
    let implementations = Implementations::load();
    let mut rng = Rng::new(0xc01);
    for _ in 0..CASES {
        let mut case = Case::new(&[], 0);
        case.fill = rng.byte();
        let result = implementations.compare(&case);
        assert_eq!(result.ret, 0);
    }
}

#[test]
fn config_c02_empty_nonzero_capacity_with_end_pointer() {
    let implementations = Implementations::load();
    let mut rng = Rng::new(0xc02);
    for _ in 0..CASES {
        let mut case = Case::new(&[], 1 + rng.usize(64));
        case.end_pointer = true;
        case.fill = rng.byte();
        let result = implementations.compare(&case);
        assert_eq!((result.ret, result.end_offset), (0, Some(0)));
    }
}

#[test]
fn config_c03_decimal_byte_exact_capacity() {
    let implementations = Implementations::load();
    let mut rng = Rng::new(0xc03);
    for _ in 0..CASES {
        let input = [b'0' + rng.usize(10) as u8, b'0' + rng.usize(10) as u8];
        let result = implementations.compare(&Case::new(&input, 1));
        assert_eq!(result.ret, 1);
    }
}

#[test]
fn config_c04_uppercase_byte_exact_capacity_with_end_pointer() {
    let implementations = Implementations::load();
    let mut rng = Rng::new(0xc04);
    for _ in 0..CASES {
        let input = [b'A' + rng.usize(6) as u8, b'A' + rng.usize(6) as u8];
        let mut case = Case::new(&input, 1);
        case.end_pointer = true;
        let result = implementations.compare(&case);
        assert_eq!((result.ret, result.end_offset), (1, Some(2)));
    }
}

#[test]
fn config_c05_lowercase_byte_excess_capacity_both_end_modes() {
    let implementations = Implementations::load();
    let mut rng = Rng::new(0xc05);
    for index in 0..CASES {
        let input = [b'a' + rng.usize(6) as u8, b'a' + rng.usize(6) as u8];
        let mut case = Case::new(&input, 2 + rng.usize(63));
        case.end_pointer = index % 2 == 0;
        assert_eq!(implementations.compare(&case).ret, 1);
    }
}

#[test]
fn config_c06_many_mixed_bytes_exact_capacity() {
    let implementations = Implementations::load();
    let mut rng = Rng::new(0xc06);
    for _ in 0..CASES {
        let bytes = random_bytes_in(&mut rng, 2, 63);
        let input = encode(&bytes, &mut rng);
        let result = implementations.compare(&Case::new(&input, bytes.len()));
        assert_eq!(result.ret, bytes.len() as c_int);
        assert_eq!(&result.output[..bytes.len()], bytes);
    }
}

#[test]
fn config_c07_many_mixed_bytes_excess_capacity_with_end_pointer() {
    let implementations = Implementations::load();
    let mut rng = Rng::new(0xc07);
    for _ in 0..CASES {
        let bytes = random_bytes_in(&mut rng, 2, 63);
        let input = encode(&bytes, &mut rng);
        let mut case = Case::new(&input, bytes.len() + 1 + rng.usize(32));
        case.end_pointer = true;
        let result = implementations.compare(&case);
        assert_eq!(
            (result.ret, result.end_offset),
            (bytes.len() as c_int, Some(input.len()))
        );
    }
}

#[test]
fn config_c08_one_ignored_separator_without_end_pointer() {
    let implementations = Implementations::load();
    let mut rng = Rng::new(0xc08);
    for _ in 0..CASES {
        let bytes = random_bytes_in(&mut rng, 2, 31);
        let encoded = encode(&bytes, &mut rng);
        let separator = b"-:_ "[rng.usize(4)];
        let split = 2 * rng.usize(bytes.len() + 1);
        let mut input = encoded[..split].to_vec();
        input.push(separator);
        input.extend_from_slice(&encoded[split..]);
        let mut case = Case::new(&input, bytes.len());
        case.ignore = Some(std::slice::from_ref(&separator));
        assert_eq!(implementations.compare(&case).ret, bytes.len() as c_int);
    }
}

#[test]
fn config_c09_many_ignored_separators_with_end_pointer() {
    let implementations = Implementations::load();
    let mut rng = Rng::new(0xc09);
    for _ in 0..CASES {
        let bytes = random_bytes_in(&mut rng, 1, 32);
        let encoded = encode(&bytes, &mut rng);
        let mut input = vec![b' ', b'-'];
        for pair in encoded.chunks_exact(2) {
            input.extend_from_slice(pair);
            for _ in 0..rng.usize(4) {
                input.push(b"-:_ "[rng.usize(4)]);
            }
        }
        input.extend_from_slice(b":: ");
        let mut case = Case::new(&input, bytes.len());
        case.ignore = Some(b"-:_ ");
        case.end_pointer = true;
        let result = implementations.compare(&case);
        assert_eq!(
            (result.ret, result.end_offset),
            (bytes.len() as c_int, Some(input.len()))
        );
    }
}

#[test]
fn config_c10_valid_hex_characters_are_not_ignored() {
    let implementations = Implementations::load();
    let mut rng = Rng::new(0xc10);
    for _ in 0..CASES {
        let bytes = random_bytes_in(&mut rng, 1, 32);
        let input = encode(&bytes, &mut rng);
        let mut case = Case::new(&input, bytes.len());
        case.ignore = Some(b"0123456789ABCDEFabcdef");
        let result = implementations.compare(&case);
        assert_eq!(&result.output[..bytes.len()], bytes);
    }
}

#[test]
fn config_c11_nul_matches_ignore_terminator() {
    let implementations = Implementations::load();
    let mut rng = Rng::new(0xc11);
    for _ in 0..CASES {
        let bytes = random_bytes_in(&mut rng, 1, 32);
        let encoded = encode(&bytes, &mut rng);
        let mut input = vec![0];
        for pair in encoded.chunks_exact(2) {
            input.extend_from_slice(pair);
            for _ in 0..rng.usize(3) {
                input.push(0);
            }
        }
        let mut case = Case::new(&input, bytes.len());
        case.ignore = Some(b"");
        case.end_pointer = true;
        let result = implementations.compare(&case);
        assert_eq!(
            (result.ret, result.end_offset),
            (bytes.len() as c_int, Some(input.len()))
        );
    }
}

#[test]
fn config_c12_invalid_at_start_returns_empty_prefix() {
    let implementations = Implementations::load();
    let mut rng = Rng::new(0xc12);
    for _ in 0..CASES {
        let input = [INVALID[rng.usize(INVALID.len())], rng.byte()];
        let mut case = Case::new(&input, rng.usize(16));
        case.end_pointer = true;
        let result = implementations.compare(&case);
        assert_eq!((result.ret, result.end_offset), (0, Some(0)));
    }
}

#[test]
fn config_c13_invalid_after_complete_bytes_returns_prefix() {
    let implementations = Implementations::load();
    let mut rng = Rng::new(0xc13);
    for _ in 0..CASES {
        let bytes = random_bytes_in(&mut rng, 1, 32);
        let mut input = encode(&bytes, &mut rng);
        input.push(INVALID[rng.usize(INVALID.len())]);
        let tail = random_bytes_in(&mut rng, 0, 8);
        input.extend_from_slice(&tail);
        let mut case = Case::new(&input, bytes.len() + 8);
        case.end_pointer = true;
        let result = implementations.compare(&case);
        assert_eq!(
            (result.ret, result.end_offset),
            (bytes.len() as c_int, Some(bytes.len() * 2))
        );
    }
}

#[test]
fn config_c14_nonmatching_ignore_returns_prefix() {
    let implementations = Implementations::load();
    let mut rng = Rng::new(0xc14);
    for _ in 0..CASES {
        let bytes = random_bytes_in(&mut rng, 1, 32);
        let mut input = encode(&bytes, &mut rng);
        input.push(b'?');
        let mut case = Case::new(&input, bytes.len() + 1);
        case.ignore = Some(b"-:_ ");
        case.end_pointer = true;
        let result = implementations.compare(&case);
        assert_eq!(result.end_offset, Some(bytes.len() * 2));
    }
}

#[test]
fn config_c15_character_class_boundaries_are_invalid() {
    let implementations = Implementations::load();
    let mut rng = Rng::new(0xc15);
    for index in 0..CASES {
        let input = [INVALID[index % INVALID.len()], rng.byte()];
        let mut case = Case::new(&input, 1 + rng.usize(16));
        case.end_pointer = true;
        let result = implementations.compare(&case);
        assert_eq!((result.ret, result.end_offset), (0, Some(0)));
    }
}

#[test]
fn config_c16_safe_null_pointers() {
    let implementations = Implementations::load();
    let mut rng = Rng::new(0xc16);
    for index in 0..CASES {
        let mut case = Case::new(&[], rng.usize(64));
        case.fill = rng.byte();
        if index % 2 == 0 {
            case.null_hex = true;
        } else {
            case.null_bin = true;
        }
        assert_eq!(implementations.compare(&case).ret, 0);
    }
}

#[test]
fn config_c17_zero_capacity_invalid_input_never_checks_capacity() {
    let implementations = Implementations::load();
    let mut rng = Rng::new(0xc17);
    for _ in 0..CASES {
        let input = [INVALID[rng.usize(INVALID.len())], rng.byte()];
        let mut case = Case::new(&input, 0);
        case.end_pointer = true;
        let result = implementations.compare(&case);
        assert_eq!((result.ret, result.end_offset), (0, Some(0)));
    }
}

#[test]
fn config_c18_large_allocated_inputs() {
    let implementations = Implementations::load();
    let mut rng = Rng::new(0xc18);
    for index in 0..32 {
        let bytes = random_bytes_in(&mut rng, 2048, 2048);
        let input = encode(&bytes, &mut rng);
        let capacity = bytes.len() + if index % 2 == 0 { 0 } else { 64 };
        let mut case = Case::new(&input, capacity);
        case.end_pointer = index % 3 == 0;
        assert_eq!(implementations.compare(&case).ret, bytes.len() as c_int);
    }
}

#[test]
fn error_e01_output_capacity_exhausted() {
    let implementations = Implementations::load();
    let mut rng = Rng::new(0xe01);
    for index in 0..CASES {
        let bytes = random_bytes_in(&mut rng, 1, 64);
        let input = encode(&bytes, &mut rng);
        let capacity = if index % 2 == 0 {
            0
        } else {
            rng.usize(bytes.len())
        };
        let mut case = Case::new(&input, capacity);
        case.end_pointer = true;
        let result = implementations.compare(&case);
        assert_eq!((result.ret, result.end_offset), (-1, Some(capacity * 2)));
    }
}

#[test]
fn error_e02_unmatched_high_nibble() {
    let implementations = Implementations::load();
    let mut rng = Rng::new(0xe02);
    for index in 0..CASES {
        let bytes = random_bytes_in(&mut rng, 0, 32);
        let mut input = encode(&bytes, &mut rng);
        let nibble = rng.usize(16) as u8;
        input.push(digit(nibble, &mut rng));
        if index % 2 == 0 {
            input.push(b'-');
        }
        let mut case = Case::new(&input, bytes.len() + 1);
        case.ignore = Some(b"-");
        case.end_pointer = true;
        let result = implementations.compare(&case);
        assert_eq!((result.ret, result.end_offset), (-1, Some(bytes.len() * 2)));
    }
}

#[test]
fn error_e03_unconsumed_invalid_input_without_end_pointer() {
    let implementations = Implementations::load();
    let mut rng = Rng::new(0xe03);
    for index in 0..CASES {
        let bytes = random_bytes_in(&mut rng, 0, 32);
        let mut input = encode(&bytes, &mut rng);
        input.push(b'?');
        let mut case = Case::new(&input, bytes.len() + 1);
        if index % 2 == 0 {
            case.ignore = Some(b"-:_ ");
        }
        let result = implementations.compare(&case);
        assert_eq!(result.ret, -1);
    }
}
