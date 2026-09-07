//! Harness self-checks.
//!
//! Guards against a *vacuously green* suite: proves the stdout capture really
//! captures the libraries' `printf` output (rather than always returning an
//! empty buffer) and that both `.so`s are distinct objects actually loaded from
//! disk.

mod harness;

use harness::*;

#[test]
fn capture_returns_the_actual_printf_bytes() {
    let l = libs();
    let _g = lock();
    // The `DEBUG_VAR` prefix is fully determined by the four arguments.
    let expected: &[u8] =
        b"Debug: param1 = 1\nDebug: param2 = 2\nDebug: param3 = 3\nDebug: param4 = 4\n";
    let prefix_len = expected.len();

    let (c_ret, c_out) = capture_stdout(|| unsafe { l.c.confusion()(1, 2, 3, 4) });
    let (r_ret, r_out) = capture_stdout(|| unsafe { l.rust.confusion()(1, 2, 3, 4) });

    assert!(
        !c_out.is_empty(),
        "capture_stdout returned nothing for the C library — the capture is broken"
    );
    assert!(
        !r_out.is_empty(),
        "capture_stdout returned nothing for the Rust library — the capture is broken"
    );
    assert_eq!(
        &c_out[..prefix_len],
        &expected[..prefix_len],
        "captured C stdout does not start with the expected DEBUG_VAR lines: {}",
        show(&c_out)
    );
    assert!(
        c_out.ends_with(format!("Final result: {c_ret}\n").as_bytes()),
        "captured C stdout must end with the final-result line: {}",
        show(&c_out)
    );
    assert_eq!(show(&c_out), show(&r_out));
    assert_eq!(c_ret, r_ret);
    // sanity: the pipeline really printed several lines
    assert!(
        c_out.iter().filter(|&&b| b == b'\n').count() >= 7,
        "expected many output lines, got: {}",
        show(&c_out)
    );
}

#[test]
fn capture_is_sensitive_to_differences() {
    // If the two captures were always equal, every stdout assertion would be
    // vacuous. Prove different inputs give different captures.
    let l = libs();
    let _g = lock();
    let (_, a) = capture_stdout(|| unsafe { l.c.confusion()(1, 2, 3, 0) });
    let (_, b) = capture_stdout(|| unsafe { l.c.confusion()(1, 2, 3, 1) });
    assert_ne!(a, b, "capture cannot distinguish two different C runs");
    let (_, ra) = capture_stdout(|| unsafe { l.rust.confusion()(1, 2, 3, 0) });
    assert_eq!(a, ra, "same input must give the same capture across libs");
}

#[test]
fn the_two_libraries_are_distinct_objects() {
    let c = find_c_so();
    let r = find_rust_so();
    assert_ne!(c, r);
    assert!(c.exists() && r.exists());
    let cb = std::fs::read(&c).unwrap();
    let rb = std::fs::read(&r).unwrap();
    assert_ne!(cb, rb, "the two .so files are byte-identical?!");
    assert!(
        c.to_string_lossy().contains("c_src/build"),
        "C .so must come from the CMake build: {}",
        c.display()
    );
}

#[test]
fn no_driver_binary_exists_in_either_project() {
    // Documents the CONFIGS.md row 26 claim mechanically.
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let cmake = std::fs::read_to_string(root.join("c_src/CMakeLists.txt")).unwrap();
    assert!(
        !cmake.contains("add_executable"),
        "CMakeLists.txt now builds an executable — add a stdout-vs-stdout binary test"
    );
    let cargo = std::fs::read_to_string(root.join("translation/Cargo.toml")).unwrap();
    assert!(
        !cargo.contains("[[bin]]"),
        "Cargo.toml now declares a binary — add a stdout-vs-stdout binary test"
    );
    assert!(!root.join("translation/src/main.rs").exists());
}
