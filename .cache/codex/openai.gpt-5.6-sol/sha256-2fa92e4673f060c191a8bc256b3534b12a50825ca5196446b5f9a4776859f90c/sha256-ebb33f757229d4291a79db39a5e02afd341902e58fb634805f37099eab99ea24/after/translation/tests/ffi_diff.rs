use libloading::{Library, Symbol};
use std::env;
use std::ffi::c_int;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

type PrintIntPtrLine = unsafe extern "C" fn(*const c_int);
type NoArgs = unsafe extern "C" fn();
type Driver = unsafe extern "C" fn(c_int);

#[cfg(target_arch = "x86_64")]
unsafe fn call_bad_with_seeded_stack(function: NoArgs) {
    let seed = 0x1357_9bdf_i32;
    unsafe {
        core::arch::asm!(
            "sub rsp, 128",
            "mov [rsp - 24], {seed}",
            "call {function}",
            "add rsp, 128",
            seed = in(reg) &seed,
            function = in(reg) function,
            clobber_abi("C"),
        );
    }
}

#[cfg(target_arch = "x86_64")]
unsafe fn call_driver_zero_with_seeded_stack(function: Driver) {
    let seed = 0x2468_ace0_i32;
    unsafe {
        core::arch::asm!(
            "sub rsp, 128",
            "mov [rsp - 56], {seed}",
            "mov r11, {function}",
            "xor edi, edi",
            "call r11",
            "add rsp, 128",
            seed = in(reg) &seed,
            function = in(reg) function,
            out("r11") _,
            clobber_abi("C"),
        );
    }
}

fn c_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../c_src/build/libdriver.so")
        .canonicalize()
        .expect("C shared library is not built")
}

fn rust_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/release/libdriver.so")
        .canonicalize()
        .expect("Rust release shared library is not built")
}

fn values_payload(values: &[i32]) -> String {
    values
        .iter()
        .map(i32::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn parse_values() -> Vec<i32> {
    let payload = env::var("FFI_DIFF_PAYLOAD").unwrap_or_default();
    if payload.is_empty() {
        Vec::new()
    } else {
        payload
            .split(',')
            .map(|value| value.parse().expect("invalid child payload"))
            .collect()
    }
}

fn run_child(library: &Path, operation: &str, payload: &str) -> Output {
    Command::new(env::current_exe().expect("integration-test executable"))
        .args([
            "--exact",
            "ffi_child_entry",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("FFI_DIFF_CHILD", "1")
        .env("FFI_DIFF_LIBRARY", library)
        .env("FFI_DIFF_OPERATION", operation)
        .env("FFI_DIFF_PAYLOAD", payload)
        .env("LD_BIND_NOW", "1")
        .output()
        .expect("run isolated FFI child")
}

fn assert_success_equivalent(operation: &str, payload: &str) {
    let c = run_child(&c_library(), operation, payload);
    let rust = run_child(&rust_library(), operation, payload);

    assert!(
        c.status.success(),
        "C child failed for {operation}: status={:?}, stderr={}",
        c.status,
        String::from_utf8_lossy(&c.stderr)
    );
    assert!(
        rust.status.success(),
        "Rust child failed for {operation}: status={:?}, stderr={}",
        rust.status,
        String::from_utf8_lossy(&rust.stderr)
    );
    assert_eq!(c.stdout, rust.stdout, "stdout mismatch for {operation}");
    assert_eq!(c.stderr, rust.stderr, "stderr mismatch for {operation}");
}

fn assert_failure_path_equivalent(operation: &str) {
    let c = run_child(&c_library(), operation, "");
    let rust = run_child(&rust_library(), operation, "");

    assert_eq!(
        (c.status.code(), c.status.signal()),
        (rust.status.code(), rust.status.signal()),
        "termination mismatch for {operation}: C={:?}, Rust={:?}\nC stdout={}\nRust stdout={}\nC stderr={}\nRust stderr={}",
        c.status,
        rust.status,
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&rust.stdout),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&rust.stderr)
    );
    assert_eq!(c.stdout, rust.stdout, "stdout mismatch for {operation}");
    assert_eq!(c.stderr, rust.stderr, "stderr mismatch for {operation}");
}

fn randomized_i32s() -> Vec<i32> {
    let mut values = vec![i32::MIN, -1, 0, 1, i32::MAX];
    let mut state = 0x4d59_5df4_d0f3_3173_u64;
    for _ in 0..512 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        values.push(state as i32);
    }
    values
}

fn randomized_nonzero_i32s() -> Vec<i32> {
    let mut values = randomized_i32s();
    values.retain(|value| *value != 0);
    values
}

#[test]
fn ffi_child_entry() {
    if env::var_os("FFI_DIFF_CHILD").is_none() {
        return;
    }

    let library_path = env::var_os("FFI_DIFF_LIBRARY").expect("child library path");
    let operation = env::var("FFI_DIFF_OPERATION").expect("child operation");
    let values = parse_values();

    unsafe {
        let library = Library::new(library_path).expect("load shared library");
        match operation.as_str() {
            "print_many" => {
                let function: Symbol<PrintIntPtrLine> =
                    library.get(b"printIntPtrLine\0").expect("printIntPtrLine");
                for value in values {
                    function(&value);
                }
            }
            "good_many" => {
                let function: Symbol<NoArgs> = library.get(b"good\0").expect("good");
                for _ in 0..values[0] {
                    function();
                }
            }
            "driver_many" => {
                let function: Symbol<Driver> = library.get(b"driver\0").expect("driver");
                for value in values {
                    function(value);
                }
            }
            "null" => {
                let function: Symbol<PrintIntPtrLine> =
                    library.get(b"printIntPtrLine\0").expect("printIntPtrLine");
                function(std::ptr::null());
            }
            "bad" => {
                let function: Symbol<NoArgs> = library.get(b"bad\0").expect("bad");
                call_bad_with_seeded_stack(*function);
            }
            "driver_zero" => {
                let function: Symbol<Driver> = library.get(b"driver\0").expect("driver");
                call_driver_zero_with_seeded_stack(*function);
            }
            _ => panic!("unknown child operation {operation}"),
        }
    }
}

#[test]
fn config_1_print_int_ptr_line_randomized() {
    assert_success_equivalent("print_many", &values_payload(&randomized_i32s()));
}

#[test]
fn config_2_good() {
    assert_success_equivalent("good_many", "257");
}

#[test]
fn config_3_driver_nonzero_randomized() {
    assert_success_equivalent("driver_many", &values_payload(&randomized_nonzero_i32s()));
}

#[test]
fn error_1_null_pointer() {
    assert_failure_path_equivalent("null");
}

#[test]
fn config_4_error_2_bad() {
    assert_failure_path_equivalent("bad");
}

#[test]
fn config_5_error_3_driver_zero() {
    assert_failure_path_equivalent("driver_zero");
}
