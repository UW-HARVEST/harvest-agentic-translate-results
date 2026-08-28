use libloading::Library;
use std::ffi::{c_int, c_uchar, c_void};
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct ParseBuffer {
    content: *const c_uchar,
    length: usize,
    offset: usize,
    depth: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct CJson {
    kind: c_int,
    valueint: c_int,
    valuedouble: f64,
}

type ParseNumber = unsafe extern "C" fn(item: *mut CJson, input_buffer: *mut ParseBuffer) -> c_int;

struct Drivers {
    _c_library: Library,
    _rust_library: Library,
    c_parse_number: ParseNumber,
    rust_parse_number: ParseNumber,
}

// The loaded function pointers remain valid because both libraries are retained.
unsafe impl Send for Drivers {}
unsafe impl Sync for Drivers {}

impl Drivers {
    fn load() -> Self {
        let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = crate_dir.join("../c_src/build/libdriver.so");
        let rust_path = crate_dir.join("target/release/libdriver.so");
        assert!(c_path.is_file(), "missing C library: {}", c_path.display());
        assert!(
            rust_path.is_file(),
            "missing Rust release library: {}",
            rust_path.display()
        );

        unsafe {
            let c_library = Library::new(&c_path)
                .unwrap_or_else(|error| panic!("load {}: {error}", c_path.display()));
            let rust_library = Library::new(&rust_path)
                .unwrap_or_else(|error| panic!("load {}: {error}", rust_path.display()));
            let c_parse_number = *c_library
                .get::<ParseNumber>(b"parse_number\0")
                .expect("resolve C parse_number");
            let rust_parse_number = *rust_library
                .get::<ParseNumber>(b"parse_number\0")
                .expect("resolve Rust parse_number");

            Self {
                _c_library: c_library,
                _rust_library: rust_library,
                c_parse_number,
                rust_parse_number,
            }
        }
    }
}

fn drivers() -> &'static Drivers {
    static DRIVERS: OnceLock<Drivers> = OnceLock::new();
    DRIVERS.get_or_init(Drivers::load)
}

#[derive(Clone, Copy, Debug)]
struct Outcome {
    result: c_int,
    item: CJson,
    buffer: ParseBuffer,
}

fn bytes_of<T>(value: &T) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            std::ptr::from_ref(value).cast::<u8>(),
            std::mem::size_of::<T>(),
        )
    }
}

fn compare_case(
    data: &[u8],
    length: usize,
    offset: usize,
    depth: usize,
    initial_item: CJson,
) -> Outcome {
    let mut c_item = initial_item;
    let mut rust_item = initial_item;
    let initial_buffer = ParseBuffer {
        content: data.as_ptr(),
        length,
        offset,
        depth,
    };
    let mut c_buffer = initial_buffer;
    let mut rust_buffer = initial_buffer;
    let drivers = drivers();

    let c_result = unsafe { (drivers.c_parse_number)(&mut c_item, &mut c_buffer) };
    let rust_result = unsafe { (drivers.rust_parse_number)(&mut rust_item, &mut rust_buffer) };

    assert_eq!(rust_result, c_result, "return mismatch for {data:?}");
    assert_eq!(
        bytes_of(&rust_item),
        bytes_of(&c_item),
        "item byte mismatch for {data:?}"
    );
    assert_eq!(
        bytes_of(&rust_buffer),
        bytes_of(&c_buffer),
        "buffer byte mismatch for {data:?}"
    );

    Outcome {
        result: c_result,
        item: c_item,
        buffer: c_buffer,
    }
}

fn default_item() -> CJson {
    CJson {
        kind: 0x1234_5678,
        valueint: 0x2345_6789,
        valuedouble: f64::from_bits(0x4009_21fb_5444_2d18),
    }
}

#[derive(Clone, Copy)]
struct Lcg(u64);

impl Lcg {
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

    fn range(&mut self, start: u64, end: u64) -> u64 {
        start + self.next_u64() % (end - start)
    }
}

fn assert_success(outcome: Outcome) {
    assert_eq!(outcome.result, 1);
    assert_eq!(outcome.item.kind, 1 << 3);
}

fn run_at_zero(token: &str) -> Outcome {
    compare_case(token.as_bytes(), token.len(), 0, 0, default_item())
}

#[test]
fn config_01_unsigned_integer_exact_length() {
    let mut rng = Lcg::new(0x0101);
    for _ in 0..64 {
        let token = rng.range(0, 2_000_000_000).to_string();
        let outcome = run_at_zero(&token);
        assert_success(outcome);
        assert_eq!(outcome.buffer.offset, token.len());
    }
}

#[test]
fn config_02_plus_integer_with_terminator() {
    let mut rng = Lcg::new(0x0202);
    for _ in 0..64 {
        let token = format!("+{}x", rng.range(0, 2_000_000_000));
        let outcome = compare_case(token.as_bytes(), token.len(), 0, 7, default_item());
        assert_success(outcome);
        assert_eq!(outcome.buffer.offset, token.len() - 1);
    }
}

#[test]
fn config_03_negative_integer_nonzero_offset_exact_length() {
    let mut rng = Lcg::new(0x0303);
    for _ in 0..64 {
        let token = format!("-{}", rng.range(0, 2_000_000_000));
        let data = format!("zz{token}");
        let outcome = compare_case(data.as_bytes(), data.len(), 2, 11, default_item());
        assert_success(outcome);
        assert_eq!(outcome.buffer.offset, data.len());
    }
}

#[test]
fn config_04_negative_integer_nonzero_offset_with_terminator() {
    let mut rng = Lcg::new(0x0404);
    for _ in 0..64 {
        let token = format!("-{}", rng.range(0, 2_000_000_000));
        let data = format!("p{token}/tail");
        let outcome = compare_case(data.as_bytes(), data.len(), 1, 13, default_item());
        assert_success(outcome);
        assert_eq!(outcome.buffer.offset, 1 + token.len());
    }
}

#[test]
fn config_05_decimal_digits_on_both_sides() {
    let mut rng = Lcg::new(0x0505);
    for _ in 0..64 {
        let token = format!(
            "{}.{}",
            rng.range(0, 1_000_000),
            rng.range(100_000, 1_000_000)
        );
        let outcome = run_at_zero(&token);
        assert_success(outcome);
        assert_eq!(outcome.buffer.offset, token.len());
    }
}

#[test]
fn config_06_decimal_without_leading_digit() {
    let mut rng = Lcg::new(0x0606);
    for _ in 0..64 {
        let token = format!(".{}x", rng.range(1, 1_000_000));
        let outcome = compare_case(token.as_bytes(), token.len(), 0, 0, default_item());
        assert_success(outcome);
        assert_eq!(outcome.buffer.offset, token.len() - 1);
    }
}

#[test]
fn config_07_decimal_without_trailing_digit() {
    let mut rng = Lcg::new(0x0707);
    for _ in 0..64 {
        let token = format!("{}.", rng.range(0, 1_000_000));
        let outcome = run_at_zero(&token);
        assert_success(outcome);
        assert_eq!(outcome.buffer.offset, token.len());
    }
}

#[test]
fn config_08_multiple_decimal_points_consumes_prefix() {
    let mut rng = Lcg::new(0x0808);
    for _ in 0..64 {
        let prefix = format!("{}.{}", rng.range(1, 10_000), rng.range(1, 10_000));
        let token = format!("{prefix}.{}", rng.range(1, 10_000));
        let outcome = run_at_zero(&token);
        assert_success(outcome);
        assert_eq!(outcome.buffer.offset, prefix.len());
    }
}

#[test]
fn config_09_lowercase_exponent_unsigned() {
    let mut rng = Lcg::new(0x0909);
    for _ in 0..64 {
        let token = format!("{}e{}", rng.range(1, 10), rng.range(0, 20));
        let outcome = run_at_zero(&token);
        assert_success(outcome);
        assert_eq!(outcome.buffer.offset, token.len());
    }
}

#[test]
fn config_10_uppercase_exponent_plus_with_terminator() {
    let mut rng = Lcg::new(0x1010);
    for _ in 0..64 {
        let token = format!(
            "{}.{}E+{}",
            rng.range(1, 10),
            rng.range(1, 10),
            rng.range(0, 10)
        );
        let data = format!("{token},");
        let outcome = compare_case(data.as_bytes(), data.len(), 0, 0, default_item());
        assert_success(outcome);
        assert_eq!(outcome.buffer.offset, token.len());
    }
}

#[test]
fn config_11_negative_exponent_nonzero_offset() {
    let mut rng = Lcg::new(0x1111);
    for _ in 0..64 {
        let token = format!("{}e-{}", rng.range(1, 10_000), rng.range(0, 20));
        let data = format!("##{token}");
        let outcome = compare_case(data.as_bytes(), data.len(), 2, 19, default_item());
        assert_success(outcome);
        assert_eq!(outcome.buffer.offset, data.len());
    }
}

#[test]
fn config_12_incomplete_exponent_consumes_mantissa() {
    let mut rng = Lcg::new(0x1212);
    let suffixes = ["e", "e+", "e-", "E", "E+", "E-"];
    for index in 0..64 {
        let mantissa = rng.range(1, 2_000_000_000).to_string();
        let token = format!("{mantissa}{}", suffixes[index % suffixes.len()]);
        let outcome = run_at_zero(&token);
        assert_success(outcome);
        assert_eq!(outcome.buffer.offset, mantissa.len());
    }
}

#[test]
fn config_13_extra_sign_consumes_numeric_prefix() {
    let mut rng = Lcg::new(0x1313);
    for index in 0..64 {
        let prefix = rng.range(1, 2_000_000_000).to_string();
        let sign = if index % 2 == 0 { '+' } else { '-' };
        let token = format!("{prefix}{sign}{}", rng.range(1, 1_000));
        let outcome = run_at_zero(&token);
        assert_success(outcome);
        assert_eq!(outcome.buffer.offset, prefix.len());
    }
}

#[test]
fn config_14_in_range_valueint_truncates() {
    let mut rng = Lcg::new(0x1414);
    for index in 0..64 {
        let whole = rng.range(0, 2_000_000_000) as i32;
        let fraction = rng.range(1, 1_000);
        let (token, expected) = if index % 2 == 0 {
            (format!("{whole}.{fraction:03}"), whole)
        } else {
            (format!("-{whole}.{fraction:03}"), -whole)
        };
        let outcome = run_at_zero(&token);
        assert_success(outcome);
        assert_eq!(outcome.item.valueint, expected);
    }
}

#[test]
fn config_15_int_max_saturation() {
    let mut rng = Lcg::new(0x1515);
    for _ in 0..64 {
        let number = i32::MAX as u64 + rng.range(0, 1_000_000);
        let outcome = run_at_zero(&number.to_string());
        assert_success(outcome);
        assert_eq!(outcome.item.valueint, i32::MAX);
    }
}

#[test]
fn config_16_int_min_saturation() {
    let mut rng = Lcg::new(0x1616);
    for _ in 0..64 {
        let magnitude = 2_147_483_648_u64 + rng.range(0, 1_000_000);
        let outcome = run_at_zero(&format!("-{magnitude}"));
        assert_success(outcome);
        assert_eq!(outcome.item.valueint, i32::MIN);
    }
}

#[test]
fn config_17_positive_infinity_saturates() {
    let mut rng = Lcg::new(0x1717);
    for _ in 0..64 {
        let token = format!("{}e{}", rng.range(1, 10), rng.range(400, 1_000));
        let outcome = run_at_zero(&token);
        assert_success(outcome);
        assert!(outcome.item.valuedouble.is_infinite());
        assert!(outcome.item.valuedouble.is_sign_positive());
        assert_eq!(outcome.item.valueint, i32::MAX);
    }
}

#[test]
fn config_18_negative_infinity_saturates() {
    let mut rng = Lcg::new(0x1818);
    for _ in 0..64 {
        let token = format!("-{}e{}", rng.range(1, 10), rng.range(400, 1_000));
        let outcome = run_at_zero(&token);
        assert_success(outcome);
        assert!(outcome.item.valuedouble.is_infinite());
        assert!(outcome.item.valuedouble.is_sign_negative());
        assert_eq!(outcome.item.valueint, i32::MIN);
    }
}

#[test]
fn config_19_exponent_underflow() {
    let mut rng = Lcg::new(0x1919);
    for _ in 0..64 {
        let token = format!("{}e-{}", rng.range(1, 10), rng.range(400, 1_000));
        let outcome = run_at_zero(&token);
        assert_success(outcome);
        assert_eq!(outcome.item.valueint, 0);
        assert_eq!(outcome.item.valuedouble, 0.0);
    }
}

#[test]
fn config_20_non_token_byte_stops_scan() {
    let mut rng = Lcg::new(0x2020);
    let terminators = [b'x', b' ', b'/', b',', b'\0'];
    for index in 0..64 {
        let token = rng.range(1, 2_000_000_000).to_string();
        let mut data = token.as_bytes().to_vec();
        data.push(terminators[index % terminators.len()]);
        data.extend_from_slice(b"tail");
        let outcome = compare_case(&data, data.len(), 0, 0, default_item());
        assert_success(outcome);
        assert_eq!(outcome.buffer.offset, token.len());
    }
}

#[test]
fn config_21_item_overwrite_and_depth_preservation() {
    let mut rng = Lcg::new(0x2121);
    for _ in 0..64 {
        let value = rng.range(0, 2_000_000_000) as i32;
        let token = value.to_string();
        let depth = rng.next_u64() as usize;
        let initial_item = CJson {
            kind: rng.next_u64() as i32,
            valueint: rng.next_u64() as i32,
            valuedouble: f64::from_bits(rng.next_u64()),
        };
        let outcome = compare_case(token.as_bytes(), token.len(), 0, depth, initial_item);
        assert_success(outcome);
        assert_eq!(outcome.item.valueint, value);
        assert_eq!(outcome.buffer.depth, depth);
    }
}

#[test]
fn error_01_null_input_buffer() {
    let drivers = drivers();
    let initial = default_item();
    let mut c_item = initial;
    let mut rust_item = initial;

    let c_result = unsafe { (drivers.c_parse_number)(&mut c_item, std::ptr::null_mut()) };
    let rust_result = unsafe { (drivers.rust_parse_number)(&mut rust_item, std::ptr::null_mut()) };

    assert_eq!(c_result, 0);
    assert_eq!(rust_result, c_result);
    assert_eq!(bytes_of(&c_item), bytes_of(&initial));
    assert_eq!(bytes_of(&rust_item), bytes_of(&initial));
}

#[test]
fn error_02_null_content() {
    let drivers = drivers();
    let initial_item = default_item();
    let initial_buffer = ParseBuffer {
        content: std::ptr::null(),
        length: 123,
        offset: 7,
        depth: 99,
    };
    let mut c_item = initial_item;
    let mut rust_item = initial_item;
    let mut c_buffer = initial_buffer;
    let mut rust_buffer = initial_buffer;

    let c_result = unsafe { (drivers.c_parse_number)(&mut c_item, &mut c_buffer) };
    let rust_result = unsafe { (drivers.rust_parse_number)(&mut rust_item, &mut rust_buffer) };

    assert_eq!(c_result, 0);
    assert_eq!(rust_result, c_result);
    assert_eq!(bytes_of(&c_item), bytes_of(&initial_item));
    assert_eq!(bytes_of(&rust_item), bytes_of(&initial_item));
    assert_eq!(bytes_of(&c_buffer), bytes_of(&initial_buffer));
    assert_eq!(bytes_of(&rust_buffer), bytes_of(&initial_buffer));
}

#[repr(C)]
#[derive(Clone, Copy)]
struct RLimit {
    current: u64,
    maximum: u64,
}

unsafe extern "C" {
    fn getrlimit(resource: c_int, limit: *mut RLimit) -> c_int;
    fn setrlimit(resource: c_int, limit: *const RLimit) -> c_int;
    fn malloc(size: usize) -> *mut c_void;
    fn free(pointer: *mut c_void);
}

const RLIMIT_AS: c_int = 9;

#[test]
fn error_03_allocation_failure_child() {
    if std::env::var_os("DRIVER_ALLOC_FAILURE_CHILD").is_none() {
        return;
    }

    let drivers = drivers();
    let data = vec![b'1'; 16 * 1024 * 1024];
    let initial_item = default_item();
    let initial_buffer = ParseBuffer {
        content: data.as_ptr(),
        length: data.len(),
        offset: 0,
        depth: 17,
    };
    let mut c_item = initial_item;
    let mut rust_item = initial_item;
    let mut c_buffer = initial_buffer;
    let mut rust_buffer = initial_buffer;
    let mut old_limit = RLimit {
        current: 0,
        maximum: 0,
    };
    assert_eq!(unsafe { getrlimit(RLIMIT_AS, &mut old_limit) }, 0);
    let constrained_limit = RLimit {
        current: 0,
        maximum: old_limit.maximum,
    };
    assert_eq!(unsafe { setrlimit(RLIMIT_AS, &constrained_limit) }, 0);

    let allocation_size = data.len() + 1;
    let mut held_allocations = [std::ptr::null_mut(); 64];
    let mut held_count = 0;
    while held_count < held_allocations.len() {
        let allocation = unsafe { malloc(allocation_size) };
        if allocation.is_null() {
            break;
        }
        held_allocations[held_count] = allocation;
        held_count += 1;
    }
    let probe_failed = held_count < held_allocations.len();

    let (c_result, rust_result) = if probe_failed {
        let c_result = unsafe { (drivers.c_parse_number)(&mut c_item, &mut c_buffer) };
        let rust_result = unsafe { (drivers.rust_parse_number)(&mut rust_item, &mut rust_buffer) };
        (c_result, rust_result)
    } else {
        (-1, -1)
    };

    assert_eq!(unsafe { setrlimit(RLIMIT_AS, &old_limit) }, 0);
    for allocation in held_allocations.into_iter().take(held_count) {
        unsafe {
            free(allocation);
        }
    }
    assert!(probe_failed, "could not force an exact-size malloc failure");
    assert_eq!(c_result, 0, "C allocation unexpectedly succeeded");
    assert_eq!(rust_result, c_result);
    assert_eq!(bytes_of(&c_item), bytes_of(&initial_item));
    assert_eq!(bytes_of(&rust_item), bytes_of(&initial_item));
    assert_eq!(bytes_of(&c_buffer), bytes_of(&initial_buffer));
    assert_eq!(bytes_of(&rust_buffer), bytes_of(&initial_buffer));
}

#[test]
fn error_03_allocation_failure() {
    let output = Command::new(std::env::current_exe().expect("current test executable"))
        .args([
            "--exact",
            "error_03_allocation_failure_child",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("DRIVER_ALLOC_FAILURE_CHILD", "1")
        .output()
        .expect("run allocation-failure child");
    assert!(
        output.status.success(),
        "allocation child failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn error_04_no_conversion_and_length_boundaries() {
    let invalid_tokens: &[&[u8]] = &[
        b"+", b"-", b".", b"+.", b"-.", b"e", b"E", b"e+", b"E-", b"--", b"++",
    ];
    for token in invalid_tokens {
        let outcome = compare_case(token, token.len(), 0, 23, default_item());
        assert_eq!(outcome.result, 0, "unexpected conversion for {token:?}");
        assert_eq!(outcome.buffer.offset, 0);
    }

    let backing = [b'x', b'y', b'z', 0];
    for (length, offset) in [(0, 0), (1, 1), (1, 2)] {
        let outcome = compare_case(&backing, length, offset, 29, default_item());
        assert_eq!(outcome.result, 0);
        assert_eq!(outcome.buffer.offset, offset);
    }

    let outcome = compare_case(&backing, usize::MAX, 0, 31, default_item());
    assert_eq!(outcome.result, 0);
    assert_eq!(outcome.buffer.offset, 0);
}

#[test]
fn generic_integer_values_one_past_range() {
    let above = run_at_zero("2147483648");
    let below = run_at_zero("-2147483649");
    assert_success(above);
    assert_success(below);
    assert_eq!(above.item.valueint, i32::MAX);
    assert_eq!(below.item.valueint, i32::MIN);
}

#[test]
fn generic_null_item_child() {
    let Some(library) = std::env::var_os("DRIVER_NULL_ITEM_LIBRARY") else {
        return;
    };
    let token = b"123";
    let mut buffer = ParseBuffer {
        content: token.as_ptr(),
        length: token.len(),
        offset: 0,
        depth: 0,
    };
    let drivers = drivers();
    let function = if library == "c" {
        drivers.c_parse_number
    } else {
        drivers.rust_parse_number
    };
    unsafe {
        function(std::ptr::null_mut(), &mut buffer);
    }
}

#[cfg(unix)]
#[test]
fn generic_null_item_matches_c_crash() {
    use std::os::unix::process::ExitStatusExt;

    for library in ["c", "rust"] {
        let output = Command::new(std::env::current_exe().expect("current test executable"))
            .args([
                "--exact",
                "generic_null_item_child",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("DRIVER_NULL_ITEM_LIBRARY", library)
            .output()
            .unwrap_or_else(|error| panic!("run null-item {library} child: {error}"));
        assert_eq!(
            output.status.signal(),
            Some(11),
            "{library} null-item status was {:?}\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
