use libloading::Library;
use std::env;
use std::ffi::{c_int, c_uchar};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const CJSON_NUMBER: c_int = 1 << 3;
const RANDOM_CASES: usize = 64;

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
    r#type: c_int,
    valueint: c_int,
    valuedouble: f64,
}

type ParseNumber = unsafe extern "C" fn(*mut CJson, *mut ParseBuffer) -> c_int;

struct Api {
    _library: Library,
    parse_number: ParseNumber,
}

#[derive(Clone, Copy, Debug)]
struct Outcome {
    result: c_int,
    item: CJson,
    buffer: ParseBuffer,
}

#[derive(Clone, Copy)]
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

    fn bool(&mut self) -> bool {
        self.next_u64() & 1 != 0
    }
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let parse_number = unsafe {
            *library
                .get::<ParseNumber>(b"parse_number\0")
                .unwrap_or_else(|error| {
                    panic!(
                        "failed to load parse_number from {}: {error}",
                        path.display()
                    )
                })
        };

        Self {
            _library: library,
            parse_number,
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

fn load_pair() -> (Api, Api) {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(
        c_path.is_file(),
        "C library is missing at {}; build it first",
        c_path.display()
    );
    assert!(
        rust_path.is_file(),
        "Rust release library is missing at {}; run cargo build --release first",
        rust_path.display()
    );

    unsafe { (Api::load(&c_path), Api::load(&rust_path)) }
}

fn sentinel_item(seed: u64) -> CJson {
    CJson {
        r#type: (seed as c_int) ^ 0x1357_2468,
        valueint: (!(seed as u32)) as c_int,
        valuedouble: f64::from_bits(0x7ff8_0000_0000_0000 | (seed & 0x0007_ffff_ffff_ffff)),
    }
}

unsafe fn call(
    api: &Api,
    item: CJson,
    content: *const c_uchar,
    length: usize,
    offset: usize,
    depth: usize,
) -> Outcome {
    let mut item = item;
    let mut buffer = ParseBuffer {
        content,
        length,
        offset,
        depth,
    };
    let result = unsafe { (api.parse_number)(&mut item, &mut buffer) };

    Outcome {
        result,
        item,
        buffer,
    }
}

fn assert_outcomes_equal(c: Outcome, rust: Outcome, context: &str) {
    assert_eq!(c.result, rust.result, "{context}: return value");
    assert_eq!(c.item.r#type, rust.item.r#type, "{context}: item.type");
    assert_eq!(
        c.item.valueint, rust.item.valueint,
        "{context}: item.valueint"
    );
    assert_eq!(
        c.item.valuedouble.to_bits(),
        rust.item.valuedouble.to_bits(),
        "{context}: item.valuedouble bits; C={:?}, Rust={:?}",
        c.item.valuedouble,
        rust.item.valuedouble
    );
    assert_eq!(
        c.buffer.content, rust.buffer.content,
        "{context}: buffer.content"
    );
    assert_eq!(
        c.buffer.length, rust.buffer.length,
        "{context}: buffer.length"
    );
    assert_eq!(
        c.buffer.offset, rust.buffer.offset,
        "{context}: buffer.offset"
    );
    assert_eq!(c.buffer.depth, rust.buffer.depth, "{context}: buffer.depth");
}

fn differential_case(
    apis: (&Api, &Api),
    bytes: &[u8],
    length: usize,
    offset: usize,
    depth: usize,
    item: CJson,
    context: &str,
) -> Outcome {
    assert!(length <= bytes.len(), "{context}: invalid test length");
    assert!(offset <= bytes.len(), "{context}: invalid backing offset");
    let before = bytes.to_vec();
    let content = bytes.as_ptr();
    let c = unsafe { call(apis.0, item, content, length, offset, depth) };
    let rust = unsafe { call(apis.1, item, content, length, offset, depth) };
    assert_outcomes_equal(c, rust, context);
    assert_eq!(bytes, before, "{context}: input bytes were modified");
    c
}

fn assert_success(outcome: Outcome, expected_offset: usize, context: &str) {
    assert_eq!(outcome.result, 1, "{context}: C should accept the input");
    assert_eq!(outcome.item.r#type, CJSON_NUMBER, "{context}: C item type");
    assert_eq!(
        outcome.buffer.offset, expected_offset,
        "{context}: C consumed offset"
    );
}

#[test]
fn config_01_digit_only_integer_to_length() {
    let (c, rust) = load_pair();
    let mut rng = Rng::new(0x0101_5eed_cafe_f00d);
    for case in 0..RANDOM_CASES {
        let value = rng.next_u64() % 2_000_000_001;
        let bytes = value.to_string().into_bytes();
        let context = format!("config 1 case {case}: {value}");
        let outcome = differential_case(
            (&c, &rust),
            &bytes,
            bytes.len(),
            0,
            rng.next_u64() as usize,
            sentinel_item(rng.next_u64()),
            &context,
        );
        assert_success(outcome, bytes.len(), &context);
        assert_eq!(outcome.item.valueint, value as c_int, "{context}");
    }
}

#[test]
fn config_02_leading_sign() {
    let (c, rust) = load_pair();
    let mut rng = Rng::new(0x0202_5eed_cafe_f00d);
    for case in 0..RANDOM_CASES {
        let magnitude = (rng.next_u64() % 2_000_000_001) as i64;
        let signed = if rng.bool() { magnitude } else { -magnitude };
        let sign = if signed < 0 { '-' } else { '+' };
        let bytes = format!("{sign}{}", signed.unsigned_abs()).into_bytes();
        let context = format!("config 2 case {case}: {}", String::from_utf8_lossy(&bytes));
        let outcome = differential_case(
            (&c, &rust),
            &bytes,
            bytes.len(),
            0,
            rng.next_u64() as usize,
            sentinel_item(rng.next_u64()),
            &context,
        );
        assert_success(outcome, bytes.len(), &context);
        assert_eq!(outcome.item.valueint, signed as c_int, "{context}");
    }
}

#[test]
fn config_03_decimal_point() {
    let (c, rust) = load_pair();
    let mut rng = Rng::new(0x0303_5eed_cafe_f00d);
    for case in 0..RANDOM_CASES {
        let whole = rng.next_u64() % 1_000_000;
        let fraction = 1 + rng.next_u64() % 999_999;
        let sign = if rng.bool() { "" } else { "-" };
        let bytes = format!("{sign}{whole}.{fraction:06}").into_bytes();
        let context = format!("config 3 case {case}: {}", String::from_utf8_lossy(&bytes));
        let outcome = differential_case(
            (&c, &rust),
            &bytes,
            bytes.len(),
            0,
            rng.next_u64() as usize,
            sentinel_item(rng.next_u64()),
            &context,
        );
        assert_success(outcome, bytes.len(), &context);
    }
}

#[test]
fn config_04_complete_exponent() {
    let (c, rust) = load_pair();
    let mut rng = Rng::new(0x0404_5eed_cafe_f00d);
    for case in 0..RANDOM_CASES {
        let sign = if rng.bool() { "" } else { "-" };
        let marker = if rng.bool() { 'e' } else { 'E' };
        let exponent_sign = if rng.bool() { '+' } else { '-' };
        let exponent = rng.usize(21);
        let bytes = format!(
            "{sign}{}.{:03}{marker}{exponent_sign}{exponent}",
            1 + rng.usize(8),
            rng.usize(1000)
        )
        .into_bytes();
        let context = format!("config 4 case {case}: {}", String::from_utf8_lossy(&bytes));
        let outcome = differential_case(
            (&c, &rust),
            &bytes,
            bytes.len(),
            0,
            rng.next_u64() as usize,
            sentinel_item(rng.next_u64()),
            &context,
        );
        assert_success(outcome, bytes.len(), &context);
    }
}

#[test]
fn config_05_successful_partial_strtod_consumption() {
    let (c, rust) = load_pair();
    let mut rng = Rng::new(0x0505_5eed_cafe_f00d);
    for case in 0..RANDOM_CASES {
        let number = 1 + rng.next_u64() % 1_000_000;
        let tail = match rng.usize(8) {
            0 => "e".to_owned(),
            1 => "e+".to_owned(),
            2 => "E-".to_owned(),
            3 => format!("+{}", rng.usize(100)),
            4 => format!("-{}", rng.usize(100)),
            5 => format!(".{}.", rng.usize(100)),
            6 => format!("e{}e", rng.usize(20)),
            _ => format!("..{}", rng.usize(100)),
        };
        let bytes = format!("{number}{tail}").into_bytes();
        let context = format!("config 5 case {case}: {}", String::from_utf8_lossy(&bytes));
        let outcome = differential_case(
            (&c, &rust),
            &bytes,
            bytes.len(),
            0,
            rng.next_u64() as usize,
            sentinel_item(rng.next_u64()),
            &context,
        );
        assert_eq!(outcome.result, 1, "{context}");
        assert!(
            outcome.buffer.offset > 0 && outcome.buffer.offset < bytes.len(),
            "{context}: expected a successful partial parse, got offset {}",
            outcome.buffer.offset
        );
    }
}

#[test]
fn config_06_non_number_terminator() {
    let (c, rust) = load_pair();
    let mut rng = Rng::new(0x0606_5eed_cafe_f00d);
    let terminators = [b'x', b' ', b',', b'/', 0, b'[', 0x80, 0xff];
    for case in 0..RANDOM_CASES {
        let prefix = (rng.next_u64() % 2_000_000_001).to_string();
        let mut bytes = prefix.as_bytes().to_vec();
        bytes.push(terminators[rng.usize(terminators.len())]);
        bytes.extend_from_slice(b"999");
        let context = format!("config 6 case {case}: {bytes:?}");
        let outcome = differential_case(
            (&c, &rust),
            &bytes,
            bytes.len(),
            0,
            rng.next_u64() as usize,
            sentinel_item(rng.next_u64()),
            &context,
        );
        assert_success(outcome, prefix.len(), &context);
    }
}

#[test]
fn config_07_declared_length_truncates_backing_bytes() {
    let (c, rust) = load_pair();
    let mut rng = Rng::new(0x0707_5eed_cafe_f00d);
    for case in 0..RANDOM_CASES {
        let prefix = (1 + rng.next_u64() % 2_000_000_000).to_string();
        let mut bytes = prefix.as_bytes().to_vec();
        bytes.extend_from_slice((rng.next_u64() % 1_000_000).to_string().as_bytes());
        let context = format!("config 7 case {case}: {bytes:?}, length {}", prefix.len());
        let outcome = differential_case(
            (&c, &rust),
            &bytes,
            prefix.len(),
            0,
            rng.next_u64() as usize,
            sentinel_item(rng.next_u64()),
            &context,
        );
        assert_success(outcome, prefix.len(), &context);
    }
}

#[test]
fn config_08_nonzero_offset() {
    let (c, rust) = load_pair();
    let mut rng = Rng::new(0x0808_5eed_cafe_f00d);
    for case in 0..RANDOM_CASES {
        let offset = 1 + rng.usize(8);
        let prefix: Vec<u8> = (0..offset).map(|_| b'0' + rng.usize(10) as u8).collect();
        let token = (rng.next_u64() % 2_000_000_001).to_string();
        let mut bytes = prefix;
        bytes.extend_from_slice(token.as_bytes());
        bytes.push(b',');
        let context = format!("config 8 case {case}: {bytes:?}, offset {offset}");
        let outcome = differential_case(
            (&c, &rust),
            &bytes,
            bytes.len(),
            offset,
            rng.next_u64() as usize,
            sentinel_item(rng.next_u64()),
            &context,
        );
        assert_success(outcome, offset + token.len(), &context);
    }
}

#[test]
fn config_09_depth_and_preinitialized_output_are_overwritten_selectively() {
    let (c, rust) = load_pair();
    let mut rng = Rng::new(0x0909_5eed_cafe_f00d);
    for case in 0..RANDOM_CASES {
        let depth = rng.next_u64() as usize;
        let item = sentinel_item(rng.next_u64());
        let value = (rng.next_u64() % 2_000_000_001) as c_int;
        let bytes = value.to_string().into_bytes();
        let context = format!("config 9 case {case}: value {value}, depth {depth}");
        let outcome = differential_case((&c, &rust), &bytes, bytes.len(), 0, depth, item, &context);
        assert_success(outcome, bytes.len(), &context);
        assert_eq!(outcome.buffer.depth, depth, "{context}: depth changed");
        assert_eq!(
            outcome.buffer.length,
            bytes.len(),
            "{context}: length changed"
        );
        assert_eq!(outcome.item.valueint, value, "{context}");
        assert_eq!(
            outcome.item.valuedouble.to_bits(),
            (value as f64).to_bits(),
            "{context}"
        );
    }
}

#[test]
fn config_10_fraction_truncates_toward_zero() {
    let (c, rust) = load_pair();
    let mut rng = Rng::new(0x1010_5eed_cafe_f00d);
    for case in 0..RANDOM_CASES {
        let whole = rng.usize(2_000_000_000);
        let fraction = 1 + rng.usize(999_999);
        let negative = rng.bool();
        let sign = if negative { "-" } else { "" };
        let bytes = format!("{sign}{whole}.{fraction:06}").into_bytes();
        let expected = if negative {
            -(whole as c_int)
        } else {
            whole as c_int
        };
        let context = format!("config 10 case {case}: {}", String::from_utf8_lossy(&bytes));
        let outcome = differential_case(
            (&c, &rust),
            &bytes,
            bytes.len(),
            0,
            rng.next_u64() as usize,
            sentinel_item(rng.next_u64()),
            &context,
        );
        assert_success(outcome, bytes.len(), &context);
        assert_eq!(outcome.item.valueint, expected, "{context}");
    }
}

#[test]
fn config_11_saturates_at_int_max() {
    let (c, rust) = load_pair();
    let mut rng = Rng::new(0x1111_5eed_cafe_f00d);
    for case in 0..RANDOM_CASES {
        let bytes = match case % 4 {
            0 => i32::MAX.to_string().into_bytes(),
            1 => (i32::MAX as u64 + 1 + rng.next_u64() % 1_000_000_000)
                .to_string()
                .into_bytes(),
            2 => format!("{}e{}", 1 + rng.usize(9), 10 + rng.usize(290)).into_bytes(),
            _ => format!("2147483647.{:06}", rng.usize(1_000_000)).into_bytes(),
        };
        let context = format!("config 11 case {case}: {}", String::from_utf8_lossy(&bytes));
        let outcome = differential_case(
            (&c, &rust),
            &bytes,
            bytes.len(),
            0,
            rng.next_u64() as usize,
            sentinel_item(rng.next_u64()),
            &context,
        );
        assert_success(outcome, bytes.len(), &context);
        assert_eq!(outcome.item.valueint, i32::MAX, "{context}");
    }
}

#[test]
fn config_12_saturates_at_int_min() {
    let (c, rust) = load_pair();
    let mut rng = Rng::new(0x1212_5eed_cafe_f00d);
    for case in 0..RANDOM_CASES {
        let bytes = match case % 4 {
            0 => i32::MIN.to_string().into_bytes(),
            1 => format!("-{}", i32::MAX as u64 + 2 + rng.next_u64() % 1_000_000_000).into_bytes(),
            2 => format!("-{}e{}", 1 + rng.usize(9), 10 + rng.usize(290)).into_bytes(),
            _ => format!("-2147483648.{:06}", rng.usize(1_000_000)).into_bytes(),
        };
        let context = format!("config 12 case {case}: {}", String::from_utf8_lossy(&bytes));
        let outcome = differential_case(
            (&c, &rust),
            &bytes,
            bytes.len(),
            0,
            rng.next_u64() as usize,
            sentinel_item(rng.next_u64()),
            &context,
        );
        assert_success(outcome, bytes.len(), &context);
        assert_eq!(outcome.item.valueint, i32::MIN, "{context}");
    }
}

#[test]
fn config_13_double_overflow_to_infinity() {
    let (c, rust) = load_pair();
    let mut rng = Rng::new(0x1313_5eed_cafe_f00d);
    for case in 0..RANDOM_CASES {
        let negative = rng.bool();
        let sign = if negative { "-" } else { "" };
        let bytes = format!("{sign}{}e{}", 1 + rng.usize(9), 309 + rng.usize(1000)).into_bytes();
        let context = format!("config 13 case {case}: {}", String::from_utf8_lossy(&bytes));
        let outcome = differential_case(
            (&c, &rust),
            &bytes,
            bytes.len(),
            0,
            rng.next_u64() as usize,
            sentinel_item(rng.next_u64()),
            &context,
        );
        assert_success(outcome, bytes.len(), &context);
        assert!(outcome.item.valuedouble.is_infinite(), "{context}");
        assert_eq!(
            outcome.item.valueint,
            if negative { i32::MIN } else { i32::MAX },
            "{context}"
        );
    }
}

#[test]
fn config_14_subnormal_underflow_and_signed_zero() {
    let (c, rust) = load_pair();
    let mut rng = Rng::new(0x1414_5eed_cafe_f00d);
    for case in 0..RANDOM_CASES {
        let negative = rng.bool();
        let sign = if negative { "-" } else { "" };
        let bytes = match case % 4 {
            0 => format!("{sign}0").into_bytes(),
            1 => format!("{sign}0e-{}", 1 + rng.usize(1000)).into_bytes(),
            2 => format!("{sign}{}e-{}", 1 + rng.usize(9), 309 + rng.usize(15)).into_bytes(),
            _ => format!("{sign}{}e-{}", 1 + rng.usize(9), 324 + rng.usize(1000)).into_bytes(),
        };
        let context = format!("config 14 case {case}: {}", String::from_utf8_lossy(&bytes));
        let outcome = differential_case(
            (&c, &rust),
            &bytes,
            bytes.len(),
            0,
            rng.next_u64() as usize,
            sentinel_item(rng.next_u64()),
            &context,
        );
        assert_success(outcome, bytes.len(), &context);
        assert_eq!(outcome.item.valueint, 0, "{context}");
        assert!(
            outcome.item.valuedouble == 0.0 || outcome.item.valuedouble.is_subnormal(),
            "{context}: expected zero or subnormal, got {:?}",
            outcome.item.valuedouble
        );
    }
}

#[test]
fn error_01_null_input_buffer() {
    let (c, rust) = load_pair();
    let initial = sentinel_item(0xe001);
    let mut c_item = initial;
    let mut rust_item = initial;
    let c_result = unsafe { (c.parse_number)(&mut c_item, std::ptr::null_mut()) };
    let rust_result = unsafe { (rust.parse_number)(&mut rust_item, std::ptr::null_mut()) };
    assert_eq!(c_result, 0);
    assert_eq!(c_result, rust_result);
    assert_eq!(c_item.r#type, rust_item.r#type);
    assert_eq!(c_item.valueint, rust_item.valueint);
    assert_eq!(
        c_item.valuedouble.to_bits(),
        rust_item.valuedouble.to_bits()
    );
    assert_eq!(c_item.r#type, initial.r#type);
    assert_eq!(c_item.valueint, initial.valueint);
    assert_eq!(c_item.valuedouble.to_bits(), initial.valuedouble.to_bits());
}

#[test]
fn error_02_null_content() {
    let (c, rust) = load_pair();
    let initial = sentinel_item(0xe002);
    let c_outcome = unsafe { call(&c, initial, std::ptr::null(), 123, 77, 55) };
    let rust_outcome = unsafe { call(&rust, initial, std::ptr::null(), 123, 77, 55) };
    assert_outcomes_equal(c_outcome, rust_outcome, "error 2");
    assert_eq!(c_outcome.result, 0);
    assert_eq!(c_outcome.buffer.offset, 77);
    assert_eq!(c_outcome.item.r#type, initial.r#type);
    assert_eq!(c_outcome.item.valueint, initial.valueint);
    assert_eq!(
        c_outcome.item.valuedouble.to_bits(),
        initial.valuedouble.to_bits()
    );
}

fn fail_malloc_source() -> PathBuf {
    manifest_dir().join("tests/fail_malloc.c")
}

fn build_fail_malloc_preload() -> PathBuf {
    let output_dir = manifest_dir().join("target/differential-fixtures");
    std::fs::create_dir_all(&output_dir).expect("create differential fixture directory");
    let output = output_dir.join("libfail_malloc.so");
    let status = Command::new("cc")
        .args(["-shared", "-fPIC", "-std=c11"])
        .arg(fail_malloc_source())
        .arg("-o")
        .arg(&output)
        .status()
        .expect("run cc for malloc interposer");
    assert!(status.success(), "building malloc interposer failed");
    assert!(output.is_file(), "malloc interposer output is missing");
    output
}

fn run_named_child(test_name: &str, target: &Path, preload: Option<&Path>) -> Output {
    let executable = env::current_exe().expect("current integration test executable");
    let mut command = Command::new(executable);
    command
        .args(["--exact", test_name, "--nocapture", "--test-threads=1"])
        .env("DIFF_CHILD_TARGET", target);
    if let Some(preload) = preload {
        command.env("LD_PRELOAD", preload);
    }
    command.output().expect("run differential child process")
}

fn result_line(output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .find_map(|line| {
            line.find("DIFF_RESULT ")
                .map(|position| line[position..].to_owned())
        })
        .unwrap_or_else(|| {
            panic!(
                "child result line missing\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
                output.status,
                stdout,
                String::from_utf8_lossy(&output.stderr)
            )
        })
}

#[test]
fn error_03_allocation_failure() {
    let preload = build_fail_malloc_preload();
    let c_output = run_named_child(
        "allocation_failure_child",
        &c_library_path(),
        Some(&preload),
    );
    let rust_output = run_named_child(
        "allocation_failure_child",
        &rust_library_path(),
        Some(&preload),
    );
    assert!(
        c_output.status.success(),
        "C allocation child failed: {:?}",
        c_output
    );
    assert!(
        rust_output.status.success(),
        "Rust allocation child failed: {:?}",
        rust_output
    );
    assert_eq!(result_line(&c_output), result_line(&rust_output));
    assert!(result_line(&c_output).starts_with("DIFF_RESULT 0 "));
}

#[test]
fn allocation_failure_child() {
    let Ok(target) = env::var("DIFF_CHILD_TARGET") else {
        return;
    };

    let api = unsafe { Api::load(Path::new(&target)) };
    let process = libloading::os::unix::Library::this();
    let fail_next_malloc = unsafe {
        *process
            .get::<unsafe extern "C" fn()>(b"fail_next_malloc\0")
            .expect("load fail_next_malloc from LD_PRELOAD")
    };
    let bytes = b"12345";
    let initial = sentinel_item(0xe003);
    let mut item = initial;
    let mut buffer = ParseBuffer {
        content: bytes.as_ptr(),
        length: bytes.len(),
        offset: 0,
        depth: 91,
    };

    unsafe {
        fail_next_malloc();
    }
    let result = unsafe { (api.parse_number)(&mut item, &mut buffer) };
    println!(
        "DIFF_RESULT {result} {} {} {:016x} {} {} {}",
        item.r#type,
        item.valueint,
        item.valuedouble.to_bits(),
        buffer.length,
        buffer.offset,
        buffer.depth
    );
}

#[test]
fn error_04_no_strtod_conversion() {
    let (c, rust) = load_pair();
    let mut rng = Rng::new(0xe004_5eed_cafe_f00d);
    let invalid = [
        b"".as_slice(),
        b"x",
        b".",
        b"+",
        b"-",
        b"e",
        b"E",
        b"++",
        b"--",
        b"e+",
        b"..",
        b"+-eE.",
    ];
    for case in 0..RANDOM_CASES {
        let bytes = invalid[rng.usize(invalid.len())];
        let initial = sentinel_item(rng.next_u64());
        let context = format!("error 4 case {case}: {bytes:?}");
        let outcome = differential_case(
            (&c, &rust),
            bytes,
            bytes.len(),
            0,
            rng.next_u64() as usize,
            initial,
            &context,
        );
        assert_eq!(outcome.result, 0, "{context}");
        assert_eq!(outcome.buffer.offset, 0, "{context}");
        assert_eq!(outcome.item.r#type, initial.r#type, "{context}");
        assert_eq!(outcome.item.valueint, initial.valueint, "{context}");
        assert_eq!(
            outcome.item.valuedouble.to_bits(),
            initial.valuedouble.to_bits(),
            "{context}"
        );
    }
}

#[test]
fn generic_zero_and_out_of_range_logical_lengths() {
    let (c, rust) = load_pair();
    let bytes = b"12345678";
    for (case, length, offset) in [(0, 0, 0), (1, 3, 3), (2, 2, 3)] {
        let initial = sentinel_item(0xb000 + case);
        let context = format!("generic boundary case {case}: length {length}, offset {offset}");
        let outcome = differential_case((&c, &rust), bytes, length, offset, 17, initial, &context);
        assert_eq!(outcome.result, 0, "{context}");
        assert_eq!(outcome.buffer.offset, offset, "{context}");
        assert_eq!(outcome.item.r#type, initial.r#type, "{context}");
    }
}

#[test]
fn generic_large_safe_length() {
    let (c, rust) = load_pair();
    let bytes = vec![b'7'; 1024 * 1024];
    let context = "generic 1 MiB numeric input";
    let outcome = differential_case(
        (&c, &rust),
        &bytes,
        bytes.len(),
        0,
        usize::MAX,
        sentinel_item(0xb100),
        context,
    );
    assert_success(outcome, bytes.len(), context);
    assert!(outcome.item.valuedouble.is_infinite());
    assert_eq!(outcome.item.valueint, i32::MAX);
}

#[test]
fn generic_null_item_is_safe_when_conversion_rejects() {
    let (c, rust) = load_pair();
    let bytes = b"x";
    let mut c_buffer = ParseBuffer {
        content: bytes.as_ptr(),
        length: bytes.len(),
        offset: 0,
        depth: 3,
    };
    let mut rust_buffer = c_buffer;
    let c_result =
        unsafe { (c.parse_number)(std::ptr::null_mut(), &mut c_buffer as *mut ParseBuffer) };
    let rust_result =
        unsafe { (rust.parse_number)(std::ptr::null_mut(), &mut rust_buffer as *mut ParseBuffer) };
    assert_eq!(c_result, 0);
    assert_eq!(c_result, rust_result);
    assert_eq!(c_buffer.offset, rust_buffer.offset);
}

#[test]
fn generic_null_item_on_success_has_matching_process_result() {
    use std::os::unix::process::ExitStatusExt;

    let c_output = run_named_child("null_item_child", &c_library_path(), None);
    let rust_output = run_named_child("null_item_child", &rust_library_path(), None);
    assert_eq!(
        c_output.status.signal(),
        Some(11),
        "C null-item call should terminate with SIGSEGV; stdout={}, stderr={}",
        String::from_utf8_lossy(&c_output.stdout),
        String::from_utf8_lossy(&c_output.stderr)
    );
    assert_eq!(
        rust_output.status.signal(),
        c_output.status.signal(),
        "Rust null-item process result differs; stdout={}, stderr={}",
        String::from_utf8_lossy(&rust_output.stdout),
        String::from_utf8_lossy(&rust_output.stderr)
    );
}

#[test]
fn null_item_child() {
    let Ok(target) = env::var("DIFF_CHILD_TARGET") else {
        return;
    };
    let api = unsafe { Api::load(Path::new(&target)) };
    let bytes = b"1";
    let mut buffer = ParseBuffer {
        content: bytes.as_ptr(),
        length: bytes.len(),
        offset: 0,
        depth: 0,
    };
    let result = unsafe { (api.parse_number)(std::ptr::null_mut(), &mut buffer) };
    let mut message = String::new();
    let _ = write!(&mut message, "unexpected return {result}");
    panic!("{message}");
}
