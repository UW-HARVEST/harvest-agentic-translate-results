use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

const PROBE_LIB_ENV: &str = "DRIVER_PROBE_LIB";
const PROBE_SCRIPT_ENV: &str = "DRIVER_PROBE_SCRIPT";
const RANDOM_CASES: usize = 64;

type DriverFn = unsafe extern "C" fn(*const c_char);
type RunFn = unsafe extern "C" fn(c_int);

unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
}

#[derive(Clone, Debug)]
enum Operation {
    Driver(Vec<u8>),
    DriverNull,
    Run(i32),
}

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

    fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
}

fn c_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../c_src/build/libdriver.so")
}

fn rust_library() -> PathBuf {
    std::env::var_os("DRIVER_RUST_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libdriver.so")
        })
}

fn hex_encode(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(DIGITS[(byte >> 4) as usize] as char);
        encoded.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn hex_decode(encoded: &str) -> Vec<u8> {
    assert_eq!(encoded.len() % 2, 0, "invalid probe hex string");
    encoded
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = (pair[0] as char).to_digit(16).expect("invalid hex digit");
            let low = (pair[1] as char).to_digit(16).expect("invalid hex digit");
            ((high << 4) | low) as u8
        })
        .collect()
}

fn serialize(operations: &[Operation]) -> String {
    let mut script = String::new();
    for operation in operations {
        match operation {
            Operation::Driver(input) => {
                script.push_str("D ");
                script.push_str(&hex_encode(input));
                script.push('\n');
            }
            Operation::DriverNull => script.push_str("N\n"),
            Operation::Run(value) => {
                script.push_str("R ");
                script.push_str(&value.to_string());
                script.push('\n');
            }
        }
    }
    script
}

fn script_path() -> PathBuf {
    static NEXT_ID: AtomicU64 = AtomicU64::new(0);
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("driver-differential-{}-{id}.txt", std::process::id()))
}

fn probe(library: &Path, operations: &[Operation]) -> Output {
    assert!(
        library.is_file(),
        "shared library does not exist: {}",
        library.display()
    );

    let script = script_path();
    fs::write(&script, serialize(operations)).expect("failed to write probe script");
    let output = Command::new(std::env::current_exe().expect("test executable path"))
        .args([
            "--exact",
            "ffi_probe",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(PROBE_LIB_ENV, library)
        .env(PROBE_SCRIPT_ENV, &script)
        .output()
        .expect("failed to run FFI probe");
    fs::remove_file(script).expect("failed to remove probe script");
    output
}

fn assert_same(operations: &[Operation]) {
    let c = probe(&c_library(), operations);
    let rust = probe(&rust_library(), operations);

    assert_eq!(
        rust.status.code(),
        c.status.code(),
        "exit codes differ\nC stderr: {}\nRust stderr: {}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&rust.stderr)
    );
    assert_eq!(
        rust.stdout,
        c.stdout,
        "stdout differs\nC:\n{}\nRust:\n{}",
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&rust.stdout)
    );
    assert_eq!(
        rust.stderr,
        c.stderr,
        "stderr differs\nC:\n{}\nRust:\n{}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&rust.stderr)
    );
}

#[cfg(unix)]
fn assert_same_signal(operations: &[Operation]) {
    use std::os::unix::process::ExitStatusExt;

    let c = probe(&c_library(), operations);
    let rust = probe(&rust_library(), operations);
    assert!(
        !c.status.success(),
        "C unexpectedly accepted boundary input"
    );
    assert_eq!(
        rust.status.signal(),
        c.status.signal(),
        "termination signals differ: C={:?}, Rust={:?}",
        c.status,
        rust.status
    );
    assert_eq!(rust.stdout, c.stdout, "stdout before termination differs");
    assert_eq!(rust.stderr, c.stderr, "stderr before termination differs");
}

#[test]
fn ffi_probe() {
    let Some(library_path) = std::env::var_os(PROBE_LIB_ENV) else {
        return;
    };
    let script_path = std::env::var_os(PROBE_SCRIPT_ENV).expect("probe script path");
    let script = fs::read_to_string(script_path).expect("failed to read probe script");

    unsafe {
        let library = Library::new(library_path).expect("failed to load shared library");
        let driver: Symbol<'_, DriverFn> =
            library.get(b"driver\0").expect("missing driver export");
        let run: Symbol<'_, RunFn> = library.get(b"run\0").expect("missing run export");

        for line in script.lines() {
            let (kind, argument) = line.split_once(' ').unwrap_or((line, ""));
            match kind {
                "D" => {
                    let mut input = hex_decode(argument);
                    input.push(0);
                    driver(input.as_ptr().cast());
                }
                "N" => driver(std::ptr::null()),
                "R" => run(argument.parse().expect("invalid run argument")),
                _ => panic!("unknown probe operation: {kind}"),
            }
        }
        fflush(std::ptr::null_mut());
    }
}

#[test]
fn config_1_run_single_randomized() {
    let mut rng = Rng::new(0xa10c_0001_5eed);
    let fixed = [i32::MIN, -1, 0, 1, i32::MAX];
    for value in fixed
        .into_iter()
        .chain((0..RANDOM_CASES).map(|_| rng.next_i32()))
    {
        assert_same(&[Operation::Run(value)]);
    }
}

#[test]
fn config_2_run_many_randomized() {
    let mut rng = Rng::new(0xa10c_0002_5eed);
    let operations = (0..RANDOM_CASES)
        .map(|_| Operation::Run(rng.next_i32()))
        .collect::<Vec<_>>();
    assert_same(&operations);
}

#[test]
fn config_3_driver_canonical_randomized() {
    let mut rng = Rng::new(0xa10c_0003_5eed);
    let operations = (0..RANDOM_CASES)
        .map(|_| Operation::Driver(rng.next_i32().to_string().into_bytes()))
        .collect::<Vec<_>>();
    assert_same(&operations);
}

#[test]
fn config_4_driver_whitespace_and_sign_randomized() {
    let mut rng = Rng::new(0xa10c_0004_5eed);
    let whitespace = [" ", "\t", "\n", "\r\n"];
    let operations = (0..RANDOM_CASES)
        .map(|index| {
            let value = rng.next_i32();
            let sign = if value >= 0 && index % 2 == 0 { "+" } else { "" };
            Operation::Driver(
                format!("{}{sign}{value}", whitespace[index % whitespace.len()]).into_bytes(),
            )
        })
        .collect::<Vec<_>>();
    assert_same(&operations);
}

#[test]
fn config_5_driver_numeric_prefix_randomized() {
    let mut rng = Rng::new(0xa10c_0005_5eed);
    let suffixes = ["x", " bedrooms", ".75", "e10", "\ttrailing"];
    let operations = (0..RANDOM_CASES)
        .map(|index| {
            Operation::Driver(
                format!("{}{}", rng.next_i32(), suffixes[index % suffixes.len()]).into_bytes(),
            )
        })
        .collect::<Vec<_>>();
    assert_same(&operations);
}

#[test]
fn config_6_driver_int_boundaries() {
    let mut rng = Rng::new(0xa10c_0006_5eed);
    let mut values = vec![i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    for _ in 0..RANDOM_CASES {
        let offset = (rng.next_u64() % 4096) as i32;
        values.push(if rng.next_u64() & 1 == 0 {
            i32::MIN + offset
        } else {
            i32::MAX - offset
        });
    }
    let operations = values
        .into_iter()
        .map(|value| Operation::Driver(value.to_string().into_bytes()))
        .collect::<Vec<_>>();
    assert_same(&operations);
}

#[test]
fn config_7_driver_embedded_nul_randomized() {
    let mut rng = Rng::new(0xa10c_0007_5eed);
    let operations = (0..RANDOM_CASES)
        .map(|_| {
            let mut input = rng.next_i32().to_string().into_bytes();
            input.extend_from_slice(b"\0ignored-2147483648");
            Operation::Driver(input)
        })
        .collect::<Vec<_>>();
    assert_same(&operations);
}

#[test]
fn config_8_mixed_entry_points_randomized() {
    let mut rng = Rng::new(0xa10c_0008_5eed);
    let operations = (0..RANDOM_CASES)
        .map(|index| {
            let value = rng.next_i32();
            if index % 3 == 0 {
                Operation::Run(value)
            } else {
                Operation::Driver(value.to_string().into_bytes())
            }
        })
        .collect::<Vec<_>>();
    assert_same(&operations);
}

#[test]
fn error_1_no_characters_consumed() {
    let operations = ["", " ", "\t\r\n", "abc", "+", "-", " x123"]
        .into_iter()
        .map(|input| Operation::Driver(input.as_bytes().to_vec()))
        .collect::<Vec<_>>();
    assert_same(&operations);
}

#[test]
fn error_2_strtol_errno() {
    let operations = vec![
        Operation::Driver(b"999999999999999999999999999999999999999".to_vec()),
        Operation::Driver(b"-999999999999999999999999999999999999999".to_vec()),
        Operation::Driver(vec![b'9'; 4096]),
    ];
    assert_same(&operations);
}

#[test]
fn error_3_below_int_min() {
    let operations = ["-2147483649", "-2147483650", "-3000000000"]
        .into_iter()
        .map(|input| Operation::Driver(input.as_bytes().to_vec()))
        .collect::<Vec<_>>();
    assert_same(&operations);
}

#[test]
fn error_4_above_int_max() {
    let operations = ["2147483648", "2147483649", "3000000000"]
        .into_iter()
        .map(|input| Operation::Driver(input.as_bytes().to_vec()))
        .collect::<Vec<_>>();
    assert_same(&operations);
}

#[test]
#[cfg(unix)]
fn error_5_null_input() {
    assert_same_signal(&[Operation::DriverNull]);
}
