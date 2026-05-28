// Integration test: binary equivalence between the C reference implementation
// and the Rust translation.
//
// The project has no library API (the C `CMakeLists.txt` only declares
// `add_executable`, and the Rust crate is `[[bin]]`-only). There are also no
// Cargo `[features]`, so there is exactly one configuration to verify.
//
// We treat the binary stdout (the printed XOR of the array) as the
// observable output and require byte-for-byte equality between the C and
// Rust executables for every seed we exercise.

use std::path::PathBuf;
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_binary() -> PathBuf {
    manifest_dir().join("c_src").join("build").join("driver")
}

fn rust_binary() -> PathBuf {
    // Cargo sets CARGO_BIN_EXE_<name> for [[bin]] targets when running tests.
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn run_with_seed(bin: &PathBuf, seed: &str) -> (i32, Vec<u8>, Vec<u8>) {
    let out = Command::new(bin)
        .arg(seed)
        .output()
        .unwrap_or_else(|e| panic!("failed to run {}: {}", bin.display(), e));
    (out.status.code().unwrap_or(-1), out.stdout, out.stderr)
}

fn assert_match(seed: &str) {
    let c = c_binary();
    let r = rust_binary();
    assert!(c.exists(), "C binary missing at {} — build it first via cmake", c.display());
    assert!(r.exists(), "Rust binary missing at {}", r.display());

    let (c_code, c_out, c_err) = run_with_seed(&c, seed);
    let (r_code, r_out, r_err) = run_with_seed(&r, seed);

    assert_eq!(
        c_code, r_code,
        "exit code mismatch for seed {seed}: C={c_code} Rust={r_code}\nC stderr: {}\nRust stderr: {}",
        String::from_utf8_lossy(&c_err),
        String::from_utf8_lossy(&r_err),
    );
    assert_eq!(
        c_out, r_out,
        "stdout mismatch for seed {seed}: C={:?} Rust={:?}",
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
    );
}

#[test]
fn seed_zero() {
    assert_match("0");
}

#[test]
fn seed_one() {
    assert_match("1");
}

#[test]
fn seed_forty_two() {
    assert_match("42");
}

#[test]
fn seed_uint_max() {
    // UINT_MAX -- largest valid seed value for the program.
    assert_match("4294967295");
}

#[test]
fn invalid_seed_trailing_garbage() {
    let (c_code, c_out, c_err) = run_with_seed(&c_binary(), "12abc");
    let (r_code, r_out, r_err) = run_with_seed(&rust_binary(), "12abc");
    assert_eq!(c_code, r_code, "exit code mismatch on invalid seed");
    assert_eq!(c_out, r_out, "stdout mismatch on invalid seed");
    // Both should print an error to stderr; we don't require byte-identical
    // stderr because the C program embeds argv[0] (the binary path) in the
    // usage message, which differs between the two executables.
    assert!(!c_err.is_empty() && !r_err.is_empty());
}

#[test]
fn invalid_seed_too_large() {
    // > UINT_MAX
    let (c_code, _, _) = run_with_seed(&c_binary(), "9999999999");
    let (r_code, _, _) = run_with_seed(&rust_binary(), "9999999999");
    assert_eq!(c_code, r_code);
    assert_ne!(c_code, 0, "expected nonzero exit for too-large seed");
}
