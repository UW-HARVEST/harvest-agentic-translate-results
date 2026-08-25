use libloading::{Library, Symbol};
use std::arch::asm;
use std::env;
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

type NoArgs = unsafe extern "C" fn();
type Driver = unsafe extern "C" fn(c_int);
type PrintIntPtrLine = unsafe extern "C" fn(*const c_int);

fn c_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libdriver.so")
}

fn rust_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/debug/libdriver.so")
}

fn probe(library: &Path, operation: &str, arguments: &str) -> Output {
    Command::new(env::current_exe().expect("current integration-test executable"))
        .args(["--exact", "child_probe", "--nocapture"])
        .env("DRIVER_PROBE_LIBRARY", library)
        .env("DRIVER_PROBE_OPERATION", operation)
        .env("DRIVER_PROBE_ARGUMENTS", arguments)
        .output()
        .expect("run differential probe")
}

fn assert_same_process_result(operation: &str, arguments: &str) {
    let c = probe(&c_library(), operation, arguments);
    let rust = probe(&rust_library(), operation, arguments);

    assert_eq!(rust.status, c.status, "{operation}: process status differs");
    assert_eq!(rust.stdout, c.stdout, "{operation}: stdout differs");
    assert_eq!(rust.stderr, c.stderr, "{operation}: stderr differs");
}

fn seeded_values() -> Vec<c_int> {
    let mut values = vec![c_int::MIN, -1, 0, 1, c_int::MAX];
    let mut state = 0x4d59_5df4_d0f3_3173_u64;
    for _ in 0..1024 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        values.push(state as c_int);
    }
    values
}

fn encoded(values: impl IntoIterator<Item = c_int>) -> String {
    values
        .into_iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

#[test]
fn exported_symbols_are_loadable_from_both_shared_libraries() {
    for path in [c_library(), rust_library()] {
        let library = unsafe { Library::new(&path) }.expect("load shared library");
        unsafe {
            let _: Symbol<NoArgs> = library.get(b"bad").expect("bad");
            let _: Symbol<Driver> = library.get(b"driver").expect("driver");
            let _: Symbol<NoArgs> = library.get(b"good").expect("good");
            let _: Symbol<PrintIntPtrLine> =
                library.get(b"printIntPtrLine").expect("printIntPtrLine");
        }
    }
}

#[test]
fn config_c1_print_int_ptr_line_randomized() {
    assert_same_process_result("print", &encoded(seeded_values()));
}

#[test]
fn config_c2_good_direct() {
    assert_same_process_result("good", "128");
}

#[test]
fn config_c3_bad_direct() {
    assert_same_process_result("bad-seeded", "23");
}

#[test]
fn config_c4_driver_zero() {
    assert_same_process_result("driver-zero-seeded", "23");
}

#[test]
fn config_c5_driver_nonzero_randomized() {
    assert_same_process_result(
        "driver",
        &encoded(seeded_values().into_iter().filter(|value| *value != 0)),
    );
}

#[test]
fn generic_g1_print_int_ptr_line_null() {
    assert_same_process_result("print-null", "");
}

#[test]
fn child_probe() {
    let Ok(path) = env::var("DRIVER_PROBE_LIBRARY") else {
        return;
    };
    let operation = env::var("DRIVER_PROBE_OPERATION").expect("probe operation");
    let arguments = env::var("DRIVER_PROBE_ARGUMENTS").expect("probe arguments");
    let library = unsafe { Library::new(path) }.expect("load probe library");

    unsafe {
        match operation.as_str() {
            "print" => {
                let function: Symbol<PrintIntPtrLine> =
                    library.get(b"printIntPtrLine").expect("printIntPtrLine");
                for argument in arguments.split(',') {
                    let value = argument.parse::<c_int>().expect("integer argument");
                    function(&value);
                }
            }
            "print-null" => {
                let function: Symbol<PrintIntPtrLine> =
                    library.get(b"printIntPtrLine").expect("printIntPtrLine");
                function(std::ptr::null());
            }
            "good" => {
                let function: Symbol<NoArgs> = library.get(b"good").expect("good");
                for _ in 0..arguments.parse::<usize>().expect("iteration count") {
                    function();
                }
            }
            "bad-seeded" => {
                let function: Symbol<NoArgs> = library.get(b"bad").expect("bad");
                let seed = arguments.parse::<c_int>().expect("seed value");
                call_bad_seeded(*function, &seed);
            }
            "driver-zero-seeded" => {
                let function: Symbol<Driver> = library.get(b"driver").expect("driver");
                let seed = arguments.parse::<c_int>().expect("seed value");
                call_driver_zero_seeded(*function, &seed);
            }
            "driver" => {
                let function: Symbol<Driver> = library.get(b"driver").expect("driver");
                for argument in arguments.split(',') {
                    function(argument.parse::<c_int>().expect("driver argument"));
                }
            }
            _ => panic!("unknown probe operation: {operation}"),
        }
    }
}

unsafe fn call_bad_seeded(function: NoArgs, seed: *const c_int) {
    unsafe {
        asm!(
            "mov qword ptr [rsp - 24], {seed}",
            "call {function}",
            seed = in(reg) seed,
            function = in(reg) function,
            clobber_abi("C"),
        );
    }
}

unsafe fn call_driver_zero_seeded(function: Driver, seed: *const c_int) {
    unsafe {
        asm!(
            "mov qword ptr [rsp - 56], {seed}",
            "xor edi, edi",
            "call {function}",
            seed = in(reg) seed,
            function = in(reg) function,
            clobber_abi("C"),
        );
    }
}
