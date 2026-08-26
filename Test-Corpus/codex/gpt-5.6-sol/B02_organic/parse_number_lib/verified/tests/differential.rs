#![cfg(unix)]

use libloading::Library;
use std::ffi::{c_int, c_uchar};
use std::mem::size_of;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs, ptr, slice};

const CHILD_MODE: &str = "DRIVER_DIFF_CHILD_MODE";
const CHILD_LIBRARY: &str = "DRIVER_DIFF_CHILD_LIBRARY";

#[repr(C)]
#[derive(Clone, Copy)]
struct ParseBuffer {
    content: *const c_uchar,
    length: usize,
    offset: usize,
    depth: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CJson {
    type_: c_int,
    valueint: c_int,
    valuedouble: f64,
}

type ParseNumber = unsafe extern "C" fn(*mut CJson, *mut ParseBuffer) -> c_int;

struct Api {
    _library: Library,
    parse_number: ParseNumber,
}

impl Api {
    fn load(path: &Path) -> Self {
        assert!(path.is_file(), "missing shared library: {}", path.display());
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

struct Harness {
    c: Api,
    rust: Api,
}

impl Harness {
    fn new() -> Self {
        Self {
            c: Api::load(&c_library_path()),
            rust: Api::load(&rust_library_path()),
        }
    }

    fn compare(&self, row: &str, input: &[u8], length: usize, offset: usize) -> Snapshot {
        let c = run_case(&self.c, input, length, offset);
        let rust = run_case(&self.rust, input, length, offset);
        assert_eq!(
            c, rust,
            "{row}: input={input:?}, length={length}, offset={offset}"
        );
        c
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Snapshot {
    result: c_int,
    item_bytes: Vec<u8>,
    item_type: c_int,
    valueint: c_int,
    valuedouble_bits: u64,
    content_unchanged: bool,
    length: usize,
    offset: usize,
    depth: usize,
    input: Vec<u8>,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("c_src/build/libdriver.so")
}

fn rust_library_path() -> PathBuf {
    let target = env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir().join("target"));
    let target = if target.is_absolute() {
        target
    } else {
        manifest_dir().join(target)
    };
    target.join("debug/libdriver.so")
}

fn seeded_item() -> CJson {
    CJson {
        type_: 0x1357_2468,
        valueint: -0x1234_567,
        valuedouble: f64::from_bits(0x4009_21fb_5444_2d18),
    }
}

fn as_bytes<T>(value: &T) -> Vec<u8> {
    unsafe { slice::from_raw_parts(ptr::from_ref(value).cast::<u8>(), size_of::<T>()).to_vec() }
}

fn run_case(api: &Api, input: &[u8], length: usize, offset: usize) -> Snapshot {
    let mut input = input.to_vec();
    let content = input.as_ptr();
    let mut item = seeded_item();
    let mut buffer = ParseBuffer {
        content,
        length,
        offset,
        depth: 0x1020_3040,
    };
    let result = unsafe { (api.parse_number)(&mut item, &mut buffer) };

    Snapshot {
        result,
        item_bytes: as_bytes(&item),
        item_type: item.type_,
        valueint: item.valueint,
        valuedouble_bits: item.valuedouble.to_bits(),
        content_unchanged: buffer.content == content,
        length: buffer.length,
        offset: buffer.offset,
        depth: buffer.depth,
        input: std::mem::take(&mut input),
    }
}

fn assert_success(snapshot: &Snapshot, expected_offset: usize) {
    assert_eq!(snapshot.result, 1);
    assert_eq!(snapshot.item_type, 1 << 3);
    assert_eq!(snapshot.offset, expected_offset);
    assert!(snapshot.content_unchanged);
}

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

    fn below(&mut self, upper: u64) -> u64 {
        self.next() % upper
    }

    fn digit(&mut self) -> u8 {
        b'0' + self.below(10) as u8
    }
}

#[test]
fn c01_one_byte_unsigned_integer() {
    let harness = Harness::new();
    let mut rng = Rng::new(0xc01);
    for _ in 0..128 {
        let input = [rng.digit()];
        assert_success(&harness.compare("C01", &input, 1, 0), 1);
    }
}

#[test]
fn c02_multi_byte_unsigned_integer() {
    let harness = Harness::new();
    let mut rng = Rng::new(0xc02);
    for _ in 0..128 {
        let input = (10 + rng.below(1_000_000)).to_string().into_bytes();
        assert_success(&harness.compare("C02", &input, input.len(), 0), input.len());
    }
}

#[test]
fn c03_explicit_sign() {
    let harness = Harness::new();
    let mut rng = Rng::new(0xc03);
    for iteration in 0..128 {
        let sign = if iteration % 2 == 0 { '+' } else { '-' };
        let input = format!("{sign}{}", 1 + rng.below(1_000_000)).into_bytes();
        assert_success(&harness.compare("C03", &input, input.len(), 0), input.len());
    }
}

#[test]
fn c04_decimal_digits_on_both_sides() {
    let harness = Harness::new();
    let mut rng = Rng::new(0xc04);
    for _ in 0..128 {
        let input = format!("{}.{}", rng.below(100_000), rng.below(100_000)).into_bytes();
        assert_success(&harness.compare("C04", &input, input.len(), 0), input.len());
    }
}

#[test]
fn c05_decimal_digits_on_one_side() {
    let harness = Harness::new();
    let mut rng = Rng::new(0xc05);
    for iteration in 0..128 {
        let input = if iteration % 2 == 0 {
            format!(".{}", 1 + rng.below(100_000))
        } else {
            format!("{}.", rng.below(100_000))
        }
        .into_bytes();
        assert_success(&harness.compare("C05", &input, input.len(), 0), input.len());
    }
}

#[test]
fn c06_lowercase_exponent() {
    let harness = Harness::new();
    let mut rng = Rng::new(0xc06);
    for iteration in 0..128 {
        let exponent = rng.below(41) as i64 - 20;
        let marker = if iteration % 2 == 0 { "e" } else { "e+" };
        let exponent = if marker == "e+" {
            exponent.unsigned_abs().to_string()
        } else {
            exponent.to_string()
        };
        let input = format!("{}{}{}", 1 + rng.below(999), marker, exponent).into_bytes();
        assert_success(&harness.compare("C06", &input, input.len(), 0), input.len());
    }
}

#[test]
fn c07_uppercase_exponent() {
    let harness = Harness::new();
    let mut rng = Rng::new(0xc07);
    for iteration in 0..128 {
        let sign = if iteration % 2 == 0 { "" } else { "-" };
        let input = format!("{}E{sign}{}", 1 + rng.below(999), rng.below(21)).into_bytes();
        assert_success(&harness.compare("C07", &input, input.len(), 0), input.len());
    }
}

#[test]
fn c08_delimiter_terminates_scan() {
    let harness = Harness::new();
    let mut rng = Rng::new(0xc08);
    let delimiters = [b'x', b' ', b',', b']', 0];
    for _ in 0..128 {
        let prefix = (1 + rng.below(1_000_000)).to_string();
        let mut input = prefix.as_bytes().to_vec();
        input.push(delimiters[rng.below(delimiters.len() as u64) as usize]);
        input.extend_from_slice(b"987");
        let snapshot = harness.compare("C08", &input, input.len(), 0);
        assert_success(&snapshot, prefix.len());
    }
}

#[test]
fn c09_length_truncates_non_nul_backing_array() {
    let harness = Harness::new();
    let mut rng = Rng::new(0xc09);
    for _ in 0..128 {
        let prefix = (1 + rng.below(1_000_000)).to_string();
        let mut input = prefix.as_bytes().to_vec();
        for _ in 0..8 {
            input.push(rng.digit());
        }
        let snapshot = harness.compare("C09", &input, prefix.len(), 0);
        assert_success(&snapshot, prefix.len());
    }
}

#[test]
fn c10_nonzero_offset() {
    let harness = Harness::new();
    let mut rng = Rng::new(0xc10);
    for _ in 0..128 {
        let start = 1 + rng.below(8) as usize;
        let token = (1 + rng.below(1_000_000)).to_string();
        let mut input = vec![b'x'; start];
        input.extend_from_slice(token.as_bytes());
        input.push(b',');
        let snapshot = harness.compare("C10", &input, input.len(), start);
        assert_success(&snapshot, start + token.len());
    }
}

#[test]
fn c11_strtod_consumes_only_valid_prefix() {
    let harness = Harness::new();
    let mut rng = Rng::new(0xc11);
    for iteration in 0..128 {
        let number = 1 + rng.below(1_000_000);
        let input = match iteration % 5 {
            0 => format!("{number}e+"),
            1 => format!("{number}E-"),
            2 => format!("{number}-{}", rng.below(100)),
            3 => format!("{number}+{}", rng.below(100)),
            _ => format!("{number}..{}", rng.below(100)),
        }
        .into_bytes();
        let snapshot = harness.compare("C11", &input, input.len(), 0);
        assert_eq!(snapshot.result, 1);
        assert!(snapshot.offset < input.len());
    }
}

#[test]
fn c12_zero_and_leading_zeroes() {
    let harness = Harness::new();
    let mut rng = Rng::new(0xc12);
    for iteration in 0..128 {
        let zeroes = 1 + rng.below(20) as usize;
        let mut input = vec![b'0'; zeroes];
        if iteration % 2 == 0 {
            input.extend_from_slice((1 + rng.below(999)).to_string().as_bytes());
        }
        assert_success(&harness.compare("C12", &input, input.len(), 0), input.len());
    }
}

#[test]
fn c13_positive_int_saturation() {
    let harness = Harness::new();
    let mut rng = Rng::new(0xc13);
    for iteration in 0..128 {
        let value = if iteration == 0 {
            i32::MAX as u64
        } else if iteration == 1 {
            i32::MAX as u64 + 1
        } else {
            i32::MAX as u64 + rng.below(1_000_000_000)
        };
        let input = value.to_string().into_bytes();
        let snapshot = harness.compare("C13", &input, input.len(), 0);
        assert_success(&snapshot, input.len());
        assert_eq!(snapshot.valueint, i32::MAX);
    }
}

#[test]
fn c14_negative_int_saturation() {
    let harness = Harness::new();
    let mut rng = Rng::new(0xc14);
    for iteration in 0..128 {
        let magnitude = if iteration == 0 {
            2_147_483_648_u64
        } else if iteration == 1 {
            2_147_483_649_u64
        } else {
            2_147_483_648_u64 + rng.below(1_000_000_000)
        };
        let input = format!("-{magnitude}").into_bytes();
        let snapshot = harness.compare("C14", &input, input.len(), 0);
        assert_success(&snapshot, input.len());
        assert_eq!(snapshot.valueint, i32::MIN);
    }
}

#[test]
fn c15_infinite_exponent_results() {
    let harness = Harness::new();
    let mut rng = Rng::new(0xc15);
    for iteration in 0..128 {
        let sign = if iteration % 2 == 0 { "" } else { "-" };
        let input = format!("{sign}{}e{}", 1 + rng.below(9), 309 + rng.below(5000)).into_bytes();
        let snapshot = harness.compare("C15", &input, input.len(), 0);
        assert_success(&snapshot, input.len());
        assert_eq!(
            f64::from_bits(snapshot.valuedouble_bits).is_infinite(),
            true
        );
    }
}

#[test]
fn c16_subnormal_and_underflow_results() {
    let harness = Harness::new();
    let mut rng = Rng::new(0xc16);
    for _ in 0..128 {
        let input = format!("{}e-{}", 1 + rng.below(9), 308 + rng.below(500)).into_bytes();
        assert_success(&harness.compare("C16", &input, input.len(), 0), input.len());
    }
}

#[test]
fn c17_values_adjacent_to_int_boundaries() {
    let harness = Harness::new();
    let mut rng = Rng::new(0xc17);
    for iteration in 0..128 {
        let value = if iteration % 2 == 0 {
            i32::MAX as i64 - 1 - rng.below(100_000) as i64
        } else {
            i32::MIN as i64 + 1 + rng.below(100_000) as i64
        };
        let input = value.to_string().into_bytes();
        let snapshot = harness.compare("C17", &input, input.len(), 0);
        assert_success(&snapshot, input.len());
        assert_eq!(snapshot.valueint, value as i32);
    }
}

#[test]
fn c18_long_numeric_tokens() {
    let harness = Harness::new();
    let mut rng = Rng::new(0xc18);
    for _ in 0..128 {
        let length = 64 + rng.below(449) as usize;
        let mut input = Vec::with_capacity(length);
        input.push(b'1' + rng.below(9) as u8);
        while input.len() < length {
            input.push(rng.digit());
        }
        assert_success(&harness.compare("C18", &input, input.len(), 0), input.len());
    }
}

#[test]
fn e01_null_input_buffer() {
    let c = Api::load(&c_library_path());
    let rust = Api::load(&rust_library_path());
    let call = |api: &Api| {
        let mut item = seeded_item();
        let result = unsafe { (api.parse_number)(&mut item, ptr::null_mut()) };
        (result, as_bytes(&item))
    };
    assert_eq!(call(&c), call(&rust));
    assert_eq!(call(&c).0, 0);
}

#[test]
fn e02_null_content() {
    let c = Api::load(&c_library_path());
    let rust = Api::load(&rust_library_path());
    let call = |api: &Api| {
        let mut item = seeded_item();
        let mut buffer = ParseBuffer {
            content: ptr::null(),
            length: 123,
            offset: 7,
            depth: 11,
        };
        let result = unsafe { (api.parse_number)(&mut item, &mut buffer) };
        (
            result,
            as_bytes(&item),
            buffer.length,
            buffer.offset,
            buffer.depth,
        )
    };
    assert_eq!(call(&c), call(&rust));
    assert_eq!(call(&c).0, 0);
}

#[test]
fn e03_allocation_failure() {
    let interposer = build_malloc_interposer();
    let c = allocation_failure_result(&c_library_path(), &interposer);
    let rust = allocation_failure_result(&rust_library_path(), &interposer);
    assert_eq!(c, rust);
    assert_eq!(c, "result=0 item_unchanged=true offset=0");
}

#[test]
fn e03_allocation_failure_child() {
    if env::var(CHILD_MODE).as_deref() != Ok("allocation") {
        return;
    }

    let api = Api::load(Path::new(&env::var_os(CHILD_LIBRARY).unwrap()));
    let preload = libloading::os::unix::Library::this();
    let arm = unsafe {
        *preload
            .get::<unsafe extern "C" fn()>(b"fail_malloc_arm\0")
            .expect("fail_malloc_arm is not preloaded")
    };
    let input = [b'1'];
    let mut item = seeded_item();
    let before = as_bytes(&item);
    let mut buffer = ParseBuffer {
        content: input.as_ptr(),
        length: input.len(),
        offset: 0,
        depth: 0,
    };
    unsafe {
        arm();
    }
    let result = unsafe { (api.parse_number)(&mut item, &mut buffer) };
    println!(
        "DRIVER_DIFF result={result} item_unchanged={} offset={}",
        as_bytes(&item) == before,
        buffer.offset
    );
}

#[test]
fn e04_empty_zero_length_and_offset_at_end() {
    let harness = Harness::new();
    let mut rng = Rng::new(0xe04);
    for iteration in 0..128 {
        let length = rng.below(32) as usize;
        let input = vec![b'7'; length];
        let (visible_length, offset) = match iteration % 3 {
            0 => (0, 0),
            1 => (length, length),
            _ => (0, 0),
        };
        let snapshot = harness.compare("E04", &input, visible_length, offset);
        assert_eq!(snapshot.result, 0);
        assert_eq!(snapshot.offset, offset);
        assert_eq!(snapshot.item_bytes, as_bytes(&seeded_item()));
    }
}

#[test]
fn e05_scanned_token_has_no_conversion() {
    let harness = Harness::new();
    let mut rng = Rng::new(0xe05);
    let alphabet = b".+-eE";
    for _ in 0..128 {
        let length = 1 + rng.below(32) as usize;
        let input: Vec<u8> = (0..length)
            .map(|_| alphabet[rng.below(alphabet.len() as u64) as usize])
            .collect();
        let snapshot = harness.compare("E05", &input, input.len(), 0);
        assert_eq!(snapshot.result, 0);
        assert_eq!(snapshot.offset, 0);
        assert_eq!(snapshot.item_bytes, as_bytes(&seeded_item()));
    }
}

#[test]
fn e06_null_item_has_matching_termination() {
    let c_signal = null_item_signal(&c_library_path());
    let rust_signal = null_item_signal(&rust_library_path());
    assert_eq!(
        c_signal,
        Some(11),
        "C oracle did not terminate with SIGSEGV"
    );
    assert_eq!(rust_signal, c_signal);
}

#[test]
fn e06_null_item_child() {
    if env::var(CHILD_MODE).as_deref() != Ok("null-item") {
        return;
    }

    let api = Api::load(Path::new(&env::var_os(CHILD_LIBRARY).unwrap()));
    let input = [b'1'];
    let mut buffer = ParseBuffer {
        content: input.as_ptr(),
        length: input.len(),
        offset: 0,
        depth: 0,
    };
    unsafe {
        (api.parse_number)(ptr::null_mut(), &mut buffer);
    }
    panic!("parse_number unexpectedly returned for a null output item");
}

#[test]
fn e07_oversized_length_with_immediate_delimiter() {
    let harness = Harness::new();
    for delimiter in [b'x', b' ', b',', b']', 0] {
        let snapshot = harness.compare("E07", &[delimiter], usize::MAX, 0);
        assert_eq!(snapshot.result, 0);
        assert_eq!(snapshot.offset, 0);
        assert_eq!(snapshot.length, usize::MAX);
    }
}

fn build_malloc_interposer() -> PathBuf {
    let output_dir = manifest_dir().join("target/test-support");
    fs::create_dir_all(&output_dir).expect("create test-support directory");
    let output = output_dir.join("libfail_malloc.so");
    let status = Command::new("cc")
        .args(["-shared", "-fPIC", "-std=c11", "-o"])
        .arg(&output)
        .arg(manifest_dir().join("tests/support/fail_malloc.c"))
        .status()
        .expect("run cc for malloc interposer");
    assert!(status.success(), "failed to build malloc interposer");
    output
}

fn child_command(library: &Path, mode: &str, exact_test: &str) -> Command {
    let mut command = Command::new(env::current_exe().expect("current test executable"));
    command
        .args(["--exact", exact_test, "--nocapture"])
        .env(CHILD_MODE, mode)
        .env(CHILD_LIBRARY, library);
    command
}

fn allocation_failure_result(library: &Path, interposer: &Path) -> String {
    let output = child_command(library, "allocation", "e03_allocation_failure_child")
        .env("LD_PRELOAD", interposer)
        .output()
        .expect("run allocation-failure child");
    assert!(
        output.status.success(),
        "allocation child failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("DRIVER_DIFF "))
        .expect("allocation child result marker")
        .to_owned()
}

fn null_item_signal(library: &Path) -> Option<i32> {
    child_command(library, "null-item", "e06_null_item_child")
        .output()
        .expect("run null-item child")
        .status
        .signal()
}
