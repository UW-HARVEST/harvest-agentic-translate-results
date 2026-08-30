use libloading::Library;
use std::env;
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

type EntryPoint = unsafe extern "C" fn(c_int);

const SAMPLE_COUNT: usize = 32;
const OUTPUT_PREFIX: &[u8] = b"The house has ";

#[derive(Clone, Copy)]
enum Operation {
    Run(i32),
    Driver(i32),
}

impl Operation {
    fn encode(self) -> String {
        match self {
            Self::Run(value) => format!("run:{value}"),
            Self::Driver(value) => format!("driver:{value}"),
        }
    }

    fn output_lines(self) -> usize {
        match self {
            Self::Run(_) => 4,
            Self::Driver(_) => 8,
        }
    }
}

fn manifest_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("../c_src/build/libdriver.so")
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/release/libdriver.so")
}

fn encode_operations(operations: &[Operation]) -> String {
    operations
        .iter()
        .copied()
        .map(Operation::encode)
        .collect::<Vec<_>>()
        .join(";")
}

fn run_worker(selected: &str, operations: &[Operation]) -> Output {
    Command::new(env::current_exe().expect("integration test executable"))
        .args([
            "--exact",
            "ffi_worker",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("DRIVER_C_LIBRARY", c_library_path())
        .env("DRIVER_RUST_LIBRARY", rust_library_path())
        .env("DRIVER_SELECTED_LIBRARY", selected)
        .env("DRIVER_OPERATIONS", encode_operations(operations))
        .output()
        .expect("run isolated FFI worker")
}

fn occurrence_count(haystack: &[u8], needle: &[u8]) -> usize {
    haystack
        .windows(needle.len())
        .filter(|window| *window == needle)
        .count()
}

fn compare_case(row: usize, sample: usize, operations: &[Operation]) {
    let c_output = run_worker("c", operations);
    let rust_output = run_worker("rust", operations);

    assert!(
        c_output.status.success(),
        "CONFIGS.md row {row}, sample {sample}: C worker failed: {}",
        String::from_utf8_lossy(&c_output.stderr)
    );
    assert!(
        rust_output.status.success(),
        "CONFIGS.md row {row}, sample {sample}: Rust worker failed: {}",
        String::from_utf8_lossy(&rust_output.stderr)
    );

    let expected_lines = operations
        .iter()
        .copied()
        .map(Operation::output_lines)
        .sum();
    assert_eq!(
        occurrence_count(&c_output.stdout, OUTPUT_PREFIX),
        expected_lines,
        "CONFIGS.md row {row}, sample {sample}: C emitted an unexpected number of lines"
    );
    assert_eq!(
        c_output.stdout, rust_output.stdout,
        "CONFIGS.md row {row}, sample {sample}: stdout differs byte-for-byte"
    );
}

fn randomized_values(seed: u64, low: i32, high: i32) -> Vec<i32> {
    assert!(low <= high);
    let span = i64::from(high) - i64::from(low) + 1;
    let mut state = seed;
    let mut values = Vec::with_capacity(SAMPLE_COUNT);

    while values.len() < SAMPLE_COUNT {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let offset = ((state >> 16) % span as u64) as i64;
        values.push((i64::from(low) + offset) as i32);
    }

    values
}

#[test]
fn config_1_run_from_fresh_state() {
    let mut values = randomized_values(0x52a7_11f0, -1_000_000, 1_000_000);
    values[..5].copy_from_slice(&[i32::MIN, i32::MAX - 5, -1, 0, 1]);

    for (sample, value) in values.into_iter().enumerate() {
        compare_case(1, sample, &[Operation::Run(value)]);
    }
}

#[test]
fn config_2_run_from_accumulated_state() {
    let values = randomized_values(0x7b31_d4e9, -1_000_000, 1_000_000);

    for (sample, value) in values.into_iter().enumerate() {
        let prelude = match sample % 3 {
            0 => vec![Operation::Run(0)],
            1 => vec![Operation::Driver(-17), Operation::Run(23)],
            _ => vec![
                Operation::Run(-101),
                Operation::Driver(211),
                Operation::Run(-307),
            ],
        };
        let mut operations = prelude;
        operations.push(Operation::Run(value));
        compare_case(2, sample, &operations);
    }
}

#[test]
fn config_3_driver_from_fresh_state() {
    let mut values = randomized_values(0x1a88_c031, -1_000_000, 1_000_000);
    values[..5].copy_from_slice(&[-1_073_741_826, 1_073_741_821, -1, 0, 1]);

    for (sample, value) in values.into_iter().enumerate() {
        compare_case(3, sample, &[Operation::Driver(value)]);
    }
}

#[test]
fn config_4_driver_from_accumulated_state() {
    let values = randomized_values(0xe90c_442d, -1_000_000, 1_000_000);

    for (sample, value) in values.into_iter().enumerate() {
        let prelude = match sample % 3 {
            0 => vec![Operation::Run(0)],
            1 => vec![Operation::Driver(29), Operation::Run(-31)],
            _ => vec![
                Operation::Run(401),
                Operation::Driver(-503),
                Operation::Run(607),
            ],
        };
        let mut operations = prelude;
        operations.push(Operation::Driver(value));
        compare_case(4, sample, &operations);
    }
}

#[test]
#[ignore = "invoked by the differential parent tests in an isolated process"]
fn ffi_worker() {
    let c_path = env::var_os("DRIVER_C_LIBRARY").expect("C library path");
    let rust_path = env::var_os("DRIVER_RUST_LIBRARY").expect("Rust library path");
    let selected = env::var("DRIVER_SELECTED_LIBRARY").expect("selected library");
    let encoded = env::var("DRIVER_OPERATIONS").expect("encoded operations");

    let c_library = unsafe { Library::new(c_path) }.expect("load C shared library");
    let rust_library = unsafe { Library::new(rust_path) }.expect("load Rust shared library");
    let library = match selected.as_str() {
        "c" => &c_library,
        "rust" => &rust_library,
        other => panic!("unknown selected library: {other}"),
    };

    for operation in encoded.split(';') {
        let (symbol, value) = operation.split_once(':').expect("operation separator");
        let value: c_int = value.parse().expect("C int argument");
        let symbol_name: &[u8] = match symbol {
            "run" => b"run\0",
            "driver" => b"driver\0",
            other => panic!("unknown operation: {other}"),
        };
        let function = unsafe { library.get::<EntryPoint>(symbol_name) }
            .expect("resolve exported entry point");
        unsafe { function(value) };
    }
}
