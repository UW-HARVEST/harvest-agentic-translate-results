// Integration test: run both the C reference binary and the Rust binary
// with the same stdin and assert their stdout matches byte-for-byte.
//
// Note: this project is a pure CLI binary (the C CMakeLists.txt builds an
// executable, not a shared library, and the Rust crate has only a [[bin]]
// target with no #[no_mangle] FFI exports). There are no library symbols to
// load via libloading. The only externally observable behavior is the
// executable's stdout for given stdin, so that is what the tests compare.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_binary() -> PathBuf {
    workspace_root().join("c_src").join("build").join("driver")
}

fn rust_binary() -> PathBuf {
    // Use the binary in the same target profile as the test binary itself.
    // Tests under `cargo test` run in profile `test` but binary deps are
    // built into target/debug or target/release. We try debug then release.
    let root = workspace_root();
    let candidates = [
        root.join("target").join("debug").join("driver"),
        root.join("target").join("release").join("driver"),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    candidates[0].clone()
}

fn ensure_c_binary_built() {
    let bin = c_binary();
    if bin.exists() {
        return;
    }
    let c_src = workspace_root().join("c_src");
    let build = c_src.join("build");
    std::fs::create_dir_all(&build).unwrap();
    let status = Command::new("cmake")
        .arg("..")
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .current_dir(&build)
        .status()
        .expect("cmake configure failed (is cmake installed?)");
    assert!(status.success(), "cmake configure failed");
    let status = Command::new("cmake")
        .arg("--build")
        .arg(".")
        .current_dir(&build)
        .status()
        .expect("cmake build failed");
    assert!(status.success(), "cmake build failed");
    assert!(bin.exists(), "C binary not present after build");
}

fn ensure_rust_binary_built() {
    let bin = rust_binary();
    if bin.exists() {
        return;
    }
    let status = Command::new(env!("CARGO"))
        .arg("build")
        .arg("--bin")
        .arg("driver")
        .current_dir(workspace_root())
        .status()
        .expect("cargo build failed");
    assert!(status.success(), "cargo build failed");
}

fn run(bin: &Path, stdin_input: &str) -> Vec<u8> {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn failed");
    {
        let stdin = child.stdin.as_mut().expect("get stdin");
        stdin.write_all(stdin_input.as_bytes()).unwrap();
    }
    let out = child.wait_with_output().expect("wait failed");
    out.stdout
}

fn compare(input: &str) {
    ensure_c_binary_built();
    ensure_rust_binary_built();
    let c_out = run(&c_binary(), input);
    let r_out = run(&rust_binary(), input);
    if c_out != r_out {
        let c_str = String::from_utf8_lossy(&c_out);
        let r_str = String::from_utf8_lossy(&r_out);
        // Print a useful diff-ish view for failures
        eprintln!("--- C STDOUT ({} bytes) ---", c_out.len());
        eprintln!("{}", c_str);
        eprintln!("--- RUST STDOUT ({} bytes) ---", r_out.len());
        eprintln!("{}", r_str);
        // Find first differing offset to highlight
        let min = c_out.len().min(r_out.len());
        let mut first = min;
        for i in 0..min {
            if c_out[i] != r_out[i] {
                first = i;
                break;
            }
        }
        eprintln!("First diff at byte {}", first);
        let start = first.saturating_sub(40);
        eprintln!(
            "C  ...{:?}",
            String::from_utf8_lossy(&c_out[start..(first + 40).min(c_out.len())])
        );
        eprintln!(
            "R  ...{:?}",
            String::from_utf8_lossy(&r_out[start..(first + 40).min(r_out.len())])
        );
        panic!("C and Rust outputs differ for input {:?}", input);
    }
}

#[test]
fn exit_immediately() {
    compare("7\n");
}

#[test]
fn integer_containers_then_exit() {
    compare("1\n7\n");
}

#[test]
fn double_containers_then_exit() {
    compare("2\n7\n");
}

#[test]
fn inventory_array_then_exit() {
    compare("3\n7\n");
}

#[test]
fn order_list_then_exit() {
    compare("4\n7\n");
}

#[test]
fn mixed_operations_then_exit() {
    compare("5\n7\n");
}

#[test]
fn run_all_demos_then_exit() {
    compare("6\n7\n");
}

#[test]
fn invalid_choice_then_exit() {
    compare("99\n7\n");
}

#[test]
fn invalid_input_then_exit() {
    compare("hello\n7\n");
}

#[test]
fn empty_line_then_exit() {
    compare("\n7\n");
}

#[test]
fn whitespace_then_choice_then_exit() {
    compare("   3\n7\n");
}

#[test]
fn negative_choice_then_exit() {
    compare("-1\n7\n");
}

#[test]
fn zero_choice_then_exit() {
    compare("0\n7\n");
}

#[test]
fn multiple_demos_in_sequence() {
    compare("1\n2\n3\n4\n5\n7\n");
}

#[test]
fn each_demo_individually_with_invalid_inputs() {
    compare("abc\n1\nfoo\n2\n  3\n4\n5\n6\n7\n");
}

#[test]
fn eof_after_partial_input() {
    // No trailing newline, no exit selection -- C reads via fgets which
    // returns NULL on EOF and the loop breaks. Rust does the same.
    compare("6");
}

#[test]
fn eof_immediately() {
    compare("");
}

#[test]
fn extra_data_after_choice_on_same_line() {
    // sscanf("%d") parses leading integer, ignores trailing junk.
    compare("3 garbage trailing text\n7\n");
}

#[test]
fn choice_with_plus_sign() {
    compare("+3\n7\n");
}
