use libloading::{Library, Symbol};

const C_LIB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libstatic_loop.so");

fn c_lib() -> Library {
    unsafe { Library::new(C_LIB_PATH).expect("Failed to load C .so") }
}

fn rust_lib() -> Library {
    // Find the Rust cdylib built by cargo
    let manifest = env!("CARGO_MANIFEST_DIR");
    let path = format!("{}/target/debug/libstatic_loop.so", manifest);
    unsafe { Library::new(&path).expect("Failed to load Rust .so") }
}

/// Test static_sum: call both C and Rust versions in the same sequence
/// and compare results. Each .so has its own static state.
#[test]
fn test_static_sum_sequence_stride_3() {
    let c = c_lib();
    let r = rust_lib();
    let c_fn: Symbol<unsafe extern "C" fn(i32) -> i32> = unsafe { c.get(b"static_sum").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(i32) -> i32> = unsafe { r.get(b"static_sum").unwrap() };

    let stride = 3;
    for i in 0..10i32 {
        let input = i * stride;
        let c_val = unsafe { c_fn(input) };
        let r_val = unsafe { r_fn(input) };
        assert_eq!(c_val, r_val, "static_sum mismatch at i={input}: C={c_val}, Rust={r_val}");
    }
}

#[test]
fn test_static_sum_sequence_stride_neg5() {
    let c = c_lib();
    let r = rust_lib();
    let c_fn: Symbol<unsafe extern "C" fn(i32) -> i32> = unsafe { c.get(b"static_sum").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(i32) -> i32> = unsafe { r.get(b"static_sum").unwrap() };

    let stride = -5;
    for i in 0..10i32 {
        let input = i * stride;
        let c_val = unsafe { c_fn(input) };
        let r_val = unsafe { r_fn(input) };
        assert_eq!(c_val, r_val, "static_sum mismatch at i={input}: C={c_val}, Rust={r_val}");
    }
}

/// Binary output comparison tests
#[test]
fn test_binary_output_stride_3() {
    let c_bin = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/driver");
    let rust_bin = env!("CARGO_BIN_EXE_driver");

    let c_out = std::process::Command::new(c_bin).arg("3").output().unwrap();
    let r_out = std::process::Command::new(rust_bin).arg("3").output().unwrap();

    assert_eq!(c_out.stdout, r_out.stdout,
        "stdout mismatch stride=3:\nC: {:?}\nR: {:?}",
        String::from_utf8_lossy(&c_out.stdout), String::from_utf8_lossy(&r_out.stdout));
    assert_eq!(c_out.status.code(), r_out.status.code());
}

#[test]
fn test_binary_output_stride_neg2() {
    let c_bin = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/driver");
    let rust_bin = env!("CARGO_BIN_EXE_driver");

    let c_out = std::process::Command::new(c_bin).arg("-2").output().unwrap();
    let r_out = std::process::Command::new(rust_bin).arg("-2").output().unwrap();

    assert_eq!(c_out.stdout, r_out.stdout,
        "stdout mismatch stride=-2:\nC: {:?}\nR: {:?}",
        String::from_utf8_lossy(&c_out.stdout), String::from_utf8_lossy(&r_out.stdout));
    assert_eq!(c_out.status.code(), r_out.status.code());
}

#[test]
fn test_binary_output_stride_0() {
    let c_bin = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/driver");
    let rust_bin = env!("CARGO_BIN_EXE_driver");

    let c_out = std::process::Command::new(c_bin).arg("0").output().unwrap();
    let r_out = std::process::Command::new(rust_bin).arg("0").output().unwrap();

    assert_eq!(c_out.stdout, r_out.stdout,
        "stdout mismatch stride=0:\nC: {:?}\nR: {:?}",
        String::from_utf8_lossy(&c_out.stdout), String::from_utf8_lossy(&r_out.stdout));
}

#[test]
fn test_binary_error_no_args() {
    let c_bin = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/driver");
    let rust_bin = env!("CARGO_BIN_EXE_driver");

    let c_out = std::process::Command::new(c_bin).output().unwrap();
    let r_out = std::process::Command::new(rust_bin).output().unwrap();

    assert_eq!(c_out.stdout, r_out.stdout,
        "error msg mismatch (no args):\nC: {:?}\nR: {:?}",
        String::from_utf8_lossy(&c_out.stdout), String::from_utf8_lossy(&r_out.stdout));
}

#[test]
fn test_binary_error_non_integer() {
    let c_bin = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/driver");
    let rust_bin = env!("CARGO_BIN_EXE_driver");

    let c_out = std::process::Command::new(c_bin).arg("abc").output().unwrap();
    let r_out = std::process::Command::new(rust_bin).arg("abc").output().unwrap();

    assert_eq!(c_out.stdout, r_out.stdout,
        "error msg mismatch (non-integer):\nC: {:?}\nR: {:?}",
        String::from_utf8_lossy(&c_out.stdout), String::from_utf8_lossy(&r_out.stdout));
}
