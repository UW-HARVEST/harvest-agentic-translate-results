use libloading::{Library, Symbol};
use std::env;
use std::ffi::{CString, c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

const CHILD_ENV: &str = "DRIVER_DIFFERENTIAL_CHILD";
const LIB_ENV: &str = "DRIVER_DIFFERENTIAL_LIBRARY";
const CASE_ENV: &str = "DRIVER_DIFFERENTIAL_CASE";
const PAYLOAD_ENV: &str = "DRIVER_DIFFERENTIAL_PAYLOAD";

#[derive(Debug)]
struct RunResult {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn c_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libdriver.so")
}

fn rust_library() -> PathBuf {
    let test_exe = env::current_exe().expect("current test executable");
    let profile_library = test_exe
        .parent()
        .and_then(Path::parent)
        .expect("target profile directory")
        .join("libdriver.so");
    if profile_library.exists() {
        return profile_library;
    }

    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libdriver.so")
}

fn run_library(library: &Path, case: &str, payload: &str) -> RunResult {
    let output = Command::new(env::current_exe().expect("current test executable"))
        .arg("--exact")
        .arg("ffi_child")
        .arg("--nocapture")
        .arg("--test-threads=1")
        .env(CHILD_ENV, "1")
        .env(LIB_ENV, library)
        .env(CASE_ENV, case)
        .env(PAYLOAD_ENV, payload)
        .output()
        .expect("run isolated FFI child");

    RunResult {
        status: output.status,
        stdout: output.stdout,
        stderr: output.stderr,
    }
}

fn assert_differential(case: &str, payload: &str) {
    let c = run_library(&c_library(), case, payload);
    let rust = run_library(&rust_library(), case, payload);

    assert_eq!(
        c.status, rust.status,
        "exit status differs for {case}\nC: {c:?}\nRust: {rust:?}"
    );
    assert_eq!(
        c.stdout, rust.stdout,
        "stdout differs for {case}\nC: {c:?}\nRust: {rust:?}"
    );
    assert_eq!(
        c.stderr, rust.stderr,
        "stderr differs for {case}\nC: {c:?}\nRust: {rust:?}"
    );
    assert!(
        c.status.success(),
        "both implementations failed for {case}\nC: {c:?}\nRust: {rust:?}"
    );
}

fn encode_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0xf) as usize] as char);
    }
    encoded
}

fn decode_bytes(encoded: &str) -> Vec<u8> {
    assert_eq!(encoded.len() % 2, 0, "hex payload has odd length");
    encoded
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = (pair[0] as char).to_digit(16).expect("hex digit");
            let low = (pair[1] as char).to_digit(16).expect("hex digit");
            ((high << 4) | low) as u8
        })
        .collect()
}

fn next_random(state: &mut u64) -> u64 {
    let mut value = *state;
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    *state = value;
    value
}

#[test]
fn ffi_child() {
    if env::var_os(CHILD_ENV).is_none() {
        return;
    }

    let library_path = env::var_os(LIB_ENV).expect("child library path");
    let case = env::var(CASE_ENV).expect("child case");
    let payload = env::var(PAYLOAD_ENV).expect("child payload");

    unsafe {
        let library = Library::new(library_path).expect("load shared library");
        match case.as_str() {
            "print-line-batch" => {
                let function: Symbol<unsafe extern "C" fn(*const c_char)> =
                    library.get(b"printLine").expect("load printLine");
                for encoded in payload.split(',') {
                    let value = CString::new(decode_bytes(encoded)).expect("NUL-free input");
                    function(value.as_ptr());
                }
            }
            "print-line-null" => {
                let function: Symbol<unsafe extern "C" fn(*const c_char)> =
                    library.get(b"printLine").expect("load printLine");
                let count: usize = payload.parse().expect("null call count");
                for _ in 0..count {
                    function(std::ptr::null());
                }
            }
            "print-int-batch" => {
                let function: Symbol<unsafe extern "C" fn(c_int)> =
                    library.get(b"printIntLine").expect("load printIntLine");
                for value in payload.split(',') {
                    function(value.parse::<c_int>().expect("C int"));
                }
            }
            "bad" => {
                let function: Symbol<unsafe extern "C" fn()> =
                    library.get(b"bad").expect("load bad");
                let count: usize = payload.parse().expect("bad call count");
                for _ in 0..count {
                    function();
                }
            }
            "good" => {
                let function: Symbol<unsafe extern "C" fn()> =
                    library.get(b"good").expect("load good");
                let count: usize = payload.parse().expect("good call count");
                for _ in 0..count {
                    function();
                }
            }
            "driver-batch" => {
                let function: Symbol<unsafe extern "C" fn(c_int)> =
                    library.get(b"driver").expect("load driver");
                for value in payload.split(',') {
                    function(value.parse::<c_int>().expect("C int"));
                }
            }
            other => panic!("unknown child case: {other}"),
        }

        unsafe extern "C" {
            fn fflush(stream: *mut c_void) -> c_int;
        }
        fflush(std::ptr::null_mut());
    }
}

#[test]
fn config_01_print_line_empty() {
    assert_differential("print-line-batch", "");
}

#[test]
fn config_02_print_line_nonempty_randomized() {
    let mut state = 0x8f91_3c27_d4a6_b5e1;
    let mut values = Vec::new();
    for _ in 0..256 {
        let length = (next_random(&mut state) % 128 + 1) as usize;
        let mut bytes = Vec::with_capacity(length);
        for _ in 0..length {
            let mut byte = next_random(&mut state) as u8;
            if byte == 0 {
                byte = 1;
            }
            bytes.push(byte);
        }
        values.push(encode_bytes(&bytes));
    }
    assert_differential("print-line-batch", &values.join(","));
}

#[test]
fn config_03_print_int_randomized_and_boundaries() {
    let mut state = 0x63b7_201d_f90e_a45c;
    let mut values = vec![c_int::MIN, -1, 0, 1, c_int::MAX];
    for _ in 0..1024 {
        values.push(next_random(&mut state) as c_int);
    }
    let payload = values
        .iter()
        .map(c_int::to_string)
        .collect::<Vec<_>>()
        .join(",");
    assert_differential("print-int-batch", &payload);
}

#[test]
fn config_04_bad() {
    assert_differential("bad", "32");
}

#[test]
fn config_05_good() {
    assert_differential("good", "32");
}

#[test]
fn config_06_driver_zero() {
    assert_differential("driver-batch", &vec!["0"; 32].join(","));
}

#[test]
fn config_07_driver_nonzero_randomized() {
    let mut state = 0xd9a4_c61f_207b_835e;
    let mut values = vec![c_int::MIN, -1, 1, c_int::MAX];
    for _ in 0..1024 {
        let mut value = next_random(&mut state) as c_int;
        if value == 0 {
            value = 1;
        }
        values.push(value);
    }
    let payload = values
        .iter()
        .map(c_int::to_string)
        .collect::<Vec<_>>()
        .join(",");
    assert_differential("driver-batch", &payload);
}

#[test]
fn error_01_print_line_null() {
    assert_differential("print-line-null", "256");
}
