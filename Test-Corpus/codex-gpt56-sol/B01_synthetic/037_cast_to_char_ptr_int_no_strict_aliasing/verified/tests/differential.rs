use libloading::{Library, Symbol};
use std::env;
use std::ffi::{c_int, c_void};
use std::fs;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

unsafe extern "C" {
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn _exit(status: c_int) -> !;
}

const C_LIBRARY: &str = "c_src/build/libdriver_c.so";
static OUTPUT_ID: AtomicU64 = AtomicU64::new(0);

struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_i32(&mut self) -> i32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 32) as u32 as i32
    }
}

#[derive(Debug, PartialEq, Eq)]
struct RunResult {
    stdout: Vec<u8>,
    status: Option<i32>,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library() -> PathBuf {
    manifest_dir().join(C_LIBRARY)
}

fn rust_library() -> PathBuf {
    let executable = env::current_exe().expect("current test executable");
    let deps = executable.parent().expect("target profile deps directory");
    let in_deps = deps.join("libdriver.so");
    if in_deps.exists() {
        in_deps
    } else {
        deps.parent()
            .expect("target profile directory")
            .join("libdriver.so")
    }
}

fn output_path() -> PathBuf {
    let id = OUTPUT_ID.fetch_add(1, Ordering::Relaxed);
    env::temp_dir().join(format!(
        "driver-differential-{}-{id}.out",
        std::process::id()
    ))
}

fn invoke(library: &Path, symbol: &str, values: &[i32], stdin: &[u8]) -> RunResult {
    let output_path = output_path();
    let values = values
        .iter()
        .map(i32::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let mut child = Command::new(env::current_exe().expect("current test executable"))
        .args(["--exact", "ffi_child", "--nocapture", "--test-threads=1"])
        .env("FFI_CHILD", "1")
        .env("FFI_LIBRARY", library)
        .env("FFI_SYMBOL", symbol)
        .env("FFI_VALUES", values)
        .env("FFI_OUTPUT", &output_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn isolated FFI caller");

    child
        .stdin
        .take()
        .expect("child stdin")
        .write_all(stdin)
        .expect("write child stdin");
    let output = child.wait_with_output().expect("wait for FFI caller");
    let stdout = fs::read(&output_path).expect("read captured library stdout");
    fs::remove_file(&output_path).expect("remove captured library stdout");

    assert!(
        output.status.success(),
        "FFI child failed for {}::{symbol}: {}",
        library.display(),
        String::from_utf8_lossy(&output.stderr)
    );

    RunResult {
        stdout,
        status: output.status.code(),
    }
}

fn invoke_both(symbol: &str, values: &[i32], stdin: &[u8]) -> (RunResult, RunResult) {
    let c = invoke(&c_library(), symbol, values, stdin);
    let rust = invoke(&rust_library(), symbol, values, stdin);
    assert_eq!(rust, c, "differential mismatch for {symbol}");
    (c, rust)
}

fn invoke_main_both(stdin: &[u8]) -> RunResult {
    let (c, _) = invoke_both("main", &[], stdin);
    assert_eq!(c.status, Some(0));
    assert_eq!(c.stdout.len(), 2 * size_of::<c_int>() + 1);
    c
}

#[test]
fn ffi_child() {
    if env::var_os("FFI_CHILD").is_none() {
        return;
    }

    let output = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(env::var_os("FFI_OUTPUT").expect("FFI_OUTPUT"))
        .expect("open child output");
    let library_path = env::var_os("FFI_LIBRARY").expect("FFI_LIBRARY");
    let symbol = env::var("FFI_SYMBOL").expect("FFI_SYMBOL");

    unsafe {
        assert_eq!(dup2(output.as_raw_fd(), 1), 1);
        let library = Library::new(library_path).expect("load shared library");
        let status = match symbol.as_str() {
            "driver" => {
                let function: Symbol<unsafe extern "C" fn(c_int)> =
                    library.get(b"driver\0").expect("load driver");
                for value in env::var("FFI_VALUES")
                    .expect("FFI_VALUES")
                    .split(',')
                    .filter(|value| !value.is_empty())
                {
                    function(value.parse().expect("parse driver value"));
                }
                0
            }
            "main" => {
                let function: Symbol<unsafe extern "C" fn() -> c_int> =
                    library.get(b"main\0").expect("load main");
                function()
            }
            _ => panic!("unknown FFI symbol {symbol}"),
        };
        std::io::stdout().flush().expect("flush Rust stdout");
        assert_eq!(fflush(std::ptr::null_mut()), 0);
        _exit(status);
    }
}

#[test]
fn phase_b_driver_all_int_shapes() {
    let mut rng = Lcg::new(0x6e3d_4a91_5bf0_27c8);
    let mut values = vec![0, 1, -1, i32::MIN, i32::MAX];
    values.extend((0..512).map(|_| rng.next_i32()));

    let (c, _) = invoke_both("driver", &values, &[]);
    assert_eq!(c.status, Some(0));
    assert_eq!(c.stdout.len(), values.len() * (2 * size_of::<c_int>() + 1));
}

#[test]
fn phase_b_main_canonical_decimal() {
    let mut rng = Lcg::new(0xad47_81c0_35ee_92b1);
    let mut values = vec![0, 1, -1, i32::MIN, i32::MAX];
    values.extend((0..64).map(|_| rng.next_i32()));

    for value in values {
        invoke_main_both(value.to_string().as_bytes());
    }
}

#[test]
fn phase_b_main_accepted_syntax() {
    let mut rng = Lcg::new(0x893e_29a4_f122_750d);
    for index in 0..96 {
        let value = rng.next_i32() % 1_000_000;
        let input = match index % 4 {
            0 => format!(" \t {value}\n"),
            1 if value >= 0 => format!("+{value}"),
            1 => value.to_string(),
            2 if value < 0 => format!("-000{}", value.unsigned_abs()),
            2 => format!("000{value}"),
            _ => format!("{value}not-a-number"),
        };
        invoke_main_both(input.as_bytes());
    }
}

#[test]
fn phase_b_main_matching_failure() {
    let mut rng = Lcg::new(0xe562_8b40_91d7_3aac);
    for index in 0..48 {
        let first = b'a' + (rng.next_i32().unsigned_abs() % 26) as u8;
        let input = format!(
            "{}invalid-{index}-{}",
            char::from(first),
            rng.next_i32().unsigned_abs()
        );
        let result = invoke_main_both(input.as_bytes());
        assert_eq!(result.stdout, b"00000000\n");
    }
}

#[test]
fn phase_b_main_input_failure_at_eof() {
    let mut rng = Lcg::new(0x079a_bc34_8f10_6d22);
    for _ in 0..32 {
        let whitespace = [b' ', b'\t', b'\n', b'\r'];
        let length = (rng.next_i32().unsigned_abs() % 24) as usize;
        let input = (0..length)
            .map(|_| whitespace[(rng.next_i32().unsigned_abs() as usize) % whitespace.len()])
            .collect::<Vec<_>>();
        let result = invoke_main_both(&input);
        assert_eq!(result.stdout, b"00000000\n");
    }
}

#[test]
fn phase_c_generic_invalid_and_range_boundaries() {
    let cases: &[&[u8]] = &[
        b"\0",
        b"+",
        b"-",
        b"2147483648",
        b"-2147483649",
        b"999999999999999999999999999999999999999999999999",
        b"-999999999999999999999999999999999999999999999999",
    ];

    for input in cases {
        invoke_main_both(input);
    }
}
