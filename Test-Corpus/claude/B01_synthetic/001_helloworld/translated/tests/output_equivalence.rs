// Equivalence test: the C `driver` and the Rust `driver` are both
// stand-alone executables that print to stdout. There is no library
// API — `c_src/CMakeLists.txt` only defines `add_executable(driver ...)`
// and the Rust crate only defines a `[[bin]]` target. There are no
// `[features]` in Cargo.toml, so there is exactly one build
// configuration to verify.
//
// The only externally-observable behavior is the bytes written to
// stdout (and the exit status). This test compiles both binaries and
// asserts byte-for-byte equality of their stdout, plus matching exit
// status. This is the most faithful equivalence check possible without
// modifying the C build (which the task forbids) to produce a shared
// library.

use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn build_c_driver() -> PathBuf {
    let c_src = workspace_root().join("c_src");
    let build = c_src.join("build");
    std::fs::create_dir_all(&build).expect("create c build dir");

    let cmake_status = Command::new("cmake")
        .current_dir(&build)
        .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
        .status()
        .expect("run cmake configure");
    assert!(cmake_status.success(), "cmake configure failed");

    let build_status = Command::new("cmake")
        .current_dir(&build)
        .args(["--build", "."])
        .status()
        .expect("run cmake build");
    assert!(build_status.success(), "cmake build failed");

    let driver = build.join("driver");
    assert!(driver.exists(), "expected C driver binary at {driver:?}");
    driver
}

fn build_rust_driver() -> PathBuf {
    let status = Command::new(env!("CARGO"))
        .current_dir(workspace_root())
        .args(["build", "--bin", "driver"])
        .status()
        .expect("run cargo build");
    assert!(status.success(), "cargo build failed");

    let exe = workspace_root().join("target").join("debug").join("driver");
    assert!(exe.exists(), "expected Rust driver binary at {exe:?}");
    exe
}

#[test]
fn c_and_rust_drivers_produce_identical_output() {
    let c = build_c_driver();
    let r = build_rust_driver();

    let c_out = Command::new(&c).output().expect("run C driver");
    let r_out = Command::new(&r).output().expect("run Rust driver");

    assert_eq!(
        c_out.stdout, r_out.stdout,
        "stdout differs:\n  C   = {:?}\n  Rust= {:?}",
        String::from_utf8_lossy(&c_out.stdout),
        String::from_utf8_lossy(&r_out.stdout),
    );
    assert_eq!(
        c_out.stderr, r_out.stderr,
        "stderr differs:\n  C   = {:?}\n  Rust= {:?}",
        String::from_utf8_lossy(&c_out.stderr),
        String::from_utf8_lossy(&r_out.stderr),
    );
    assert_eq!(
        c_out.status.code(),
        r_out.status.code(),
        "exit code differs",
    );
}
