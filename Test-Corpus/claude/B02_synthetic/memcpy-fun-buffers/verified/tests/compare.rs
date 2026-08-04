// Integration tests that run the C and Rust binaries as subprocesses with
// identical stdin and assert their stdout is byte-identical.
//
// The C source compiles to an executable (not a shared library), so there
// are no exported FFI symbols to load via `libloading`. The dependency is
// still declared in dev-dependencies per harness instructions, and used
// below to demonstrate the load path even though there is no library here.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rust_binary() -> PathBuf {
    // CARGO_BIN_EXE_<name> is set by Cargo for integration tests.
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn c_binary() -> PathBuf {
    workspace_root().join("c_src").join("build").join("driver")
}

fn ensure_c_built() {
    let bin = c_binary();
    if bin.exists() {
        return;
    }
    let c_src = workspace_root().join("c_src");
    let build_dir = c_src.join("build");
    std::fs::create_dir_all(&build_dir).unwrap();
    let cmake = Command::new("cmake")
        .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
        .current_dir(&build_dir)
        .output()
        .expect("cmake configure failed");
    assert!(cmake.status.success(), "cmake configure failed: {}", String::from_utf8_lossy(&cmake.stderr));
    let build = Command::new("cmake")
        .args(["--build", "."])
        .current_dir(&build_dir)
        .output()
        .expect("cmake build failed");
    assert!(build.status.success(), "cmake build failed: {}", String::from_utf8_lossy(&build.stderr));
}

fn run(bin: &PathBuf, input: &str) -> (Vec<u8>, i32) {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn");
    {
        let stdin = child.stdin.as_mut().expect("no stdin");
        stdin.write_all(input.as_bytes()).expect("write stdin");
    }
    let out = child.wait_with_output().expect("wait failed");
    let code = out.status.code().unwrap_or(-1);
    (out.stdout, code)
}

fn compare(input: &str) {
    ensure_c_built();
    let c = run(&c_binary(), input);
    let r = run(&rust_binary(), input);
    assert_eq!(
        String::from_utf8_lossy(&c.0),
        String::from_utf8_lossy(&r.0),
        "stdout differs for input:\n{}\nC stdout: {:?}\nR stdout: {:?}",
        input,
        c.0,
        r.0
    );
    assert_eq!(c.1, r.1, "exit code differs for input:\n{}", input);
}

// ==================== libloading no-op (no library to load) ====================
//
// The C code does not produce a shared library, only an executable, so there
// is nothing for libloading to load. We still link against it to satisfy the
// harness dependency requirement and exercise the fact that no .so exists.
#[test]
fn no_shared_library_to_load() {
    let so = workspace_root().join("c_src").join("build").join("libdriver.so");
    assert!(
        !so.exists(),
        "expected no shared library; this project's C target is an executable",
    );
    // Use libloading so the dev-dep is referenced.
    let _ = unsafe { libloading::Library::new(&so) }.err();
}

// ==================== OP_COPY (operation 0) ====================
// Requires >=2 buffers. Outputs a single line: copy of buffer[0].

#[test]
fn op_copy_basic() {
    compare("0\n2\n5 1 2 3 4 5\n3 9 8 7\n");
}

#[test]
fn op_copy_empty_first_buffer() {
    compare("0\n2\n0\n3 9 8 7\n");
}

#[test]
fn op_copy_one_buffer_is_error() {
    // C prints to stderr and returns 1. We compare stdout (empty) and exit code.
    compare("0\n1\n5 1 2 3 4 5\n");
}

#[test]
fn op_copy_max_length() {
    let mut input = String::from("0\n2\n256");
    for i in 0..256 {
        input.push(' ');
        input.push_str(&(i & 0xff).to_string());
    }
    input.push_str("\n0\n");
    compare(&input);
}

// ==================== OP_REVERSE (operation 1) ====================

#[test]
fn op_reverse_basic() {
    compare("1\n3\n3 1 2 3\n2 4 5\n3 6 7 8\n");
}

#[test]
fn op_reverse_empty_buffer() {
    compare("1\n2\n0\n3 1 2 3\n");
}

#[test]
fn op_reverse_single_byte() {
    compare("1\n1\n1 42\n");
}

// ==================== OP_MERGE (operation 2) ====================

#[test]
fn op_merge_basic() {
    compare("2\n2\n3 1 2 3\n2 4 5\n");
}

#[test]
fn op_merge_overflow_is_error() {
    // 200 + 200 > 256 should be an error per the C code.
    let mut a = String::from("200");
    for i in 0..200 {
        a.push(' ');
        a.push_str(&((i + 1) % 256).to_string());
    }
    let mut b = String::from("200");
    for i in 0..200 {
        b.push(' ');
        b.push_str(&((i + 100) % 256).to_string());
    }
    let input = format!("2\n2\n{}\n{}\n", a, b);
    compare(&input);
}

#[test]
fn op_merge_one_buffer_error() {
    compare("2\n1\n3 1 2 3\n");
}

#[test]
fn op_merge_empty_first() {
    compare("2\n2\n0\n3 9 8 7\n");
}

#[test]
fn op_merge_empty_second() {
    compare("2\n2\n3 1 2 3\n0\n");
}

// ==================== OP_SPLIT (operation 3) ====================

#[test]
fn op_split_basic() {
    compare("3\n1\n5 10 20 30 40 50\n2\n");
}

#[test]
fn op_split_at_zero() {
    compare("3\n1\n5 10 20 30 40 50\n0\n");
}

#[test]
fn op_split_at_end() {
    compare("3\n1\n5 10 20 30 40 50\n5\n");
}

#[test]
fn op_split_invalid_position() {
    // split_pos > length triggers error
    compare("3\n1\n3 10 20 30\n10\n");
}

// ==================== OP_INTERLEAVE (operation 4) ====================

#[test]
fn op_interleave_equal_lengths() {
    compare("4\n2\n3 1 2 3\n3 4 5 6\n");
}

#[test]
fn op_interleave_unequal_lengths() {
    compare("4\n2\n5 1 2 3 4 5\n2 9 8\n");
}

#[test]
fn op_interleave_empty_one_side() {
    compare("4\n2\n0\n3 9 8 7\n");
}

#[test]
fn op_interleave_overflow() {
    // 130 + 130 > 256
    let mut a = String::from("130");
    for i in 0..130 {
        a.push(' ');
        a.push_str(&(i % 256).to_string());
    }
    let mut b = String::from("130");
    for i in 0..130 {
        b.push(' ');
        b.push_str(&((i + 50) % 256).to_string());
    }
    let input = format!("4\n2\n{}\n{}\n", a, b);
    compare(&input);
}

#[test]
fn op_interleave_one_buffer_error() {
    compare("4\n1\n3 1 2 3\n");
}

// ==================== OP_ROTATE (operation 5) ====================

#[test]
fn op_rotate_positive() {
    compare("5\n2\n5 1 2 3 4 5\n4 10 20 30 40\n2\n");
}

#[test]
fn op_rotate_negative() {
    compare("5\n2\n5 1 2 3 4 5\n4 10 20 30 40\n-1\n");
}

#[test]
fn op_rotate_zero() {
    compare("5\n1\n5 1 2 3 4 5\n0\n");
}

#[test]
fn op_rotate_modulo_wraps() {
    // Rotation by length is identity
    compare("5\n1\n5 1 2 3 4 5\n5\n");
}

#[test]
fn op_rotate_large_positive() {
    compare("5\n1\n5 1 2 3 4 5\n7\n");
}

#[test]
fn op_rotate_large_negative() {
    compare("5\n1\n5 1 2 3 4 5\n-7\n");
}

#[test]
fn op_rotate_empty_buffer() {
    compare("5\n1\n0\n3\n");
}

// ==================== OP_CHECKSUM (operation 6) ====================

#[test]
fn op_checksum_basic() {
    compare("6\n3\n3 1 2 3\n2 4 5\n5 100 200 50 150 25\n");
}

#[test]
fn op_checksum_empty_buffer() {
    compare("6\n2\n0\n3 1 2 3\n");
}

#[test]
fn op_checksum_max_size() {
    let mut input = String::from("6\n1\n256");
    for i in 0..256 {
        input.push(' ');
        input.push_str(&(i & 0xff).to_string());
    }
    input.push('\n');
    compare(&input);
}

// ==================== Boundary / error conditions ====================

#[test]
fn unknown_operation() {
    compare("99\n1\n3 1 2 3\n");
}

#[test]
fn invalid_buffer_count_zero() {
    compare("1\n0\n");
}

#[test]
fn invalid_buffer_count_too_large() {
    compare("1\n101\n");
}

#[test]
fn invalid_buffer_length_negative() {
    compare("1\n1\n-1\n");
}

#[test]
fn invalid_buffer_length_too_large() {
    compare("1\n1\n257\n");
}

// ==================== Larger combined patterns ====================

#[test]
fn combined_max_buffers_reverse() {
    // 100 buffers of varying lengths
    let mut input = String::from("1\n100\n");
    for i in 0..100 {
        let len = (i % 20) + 1;
        input.push_str(&len.to_string());
        for j in 0..len {
            input.push(' ');
            input.push_str(&(((i * 7 + j * 3) & 0xff)).to_string());
        }
        input.push('\n');
    }
    compare(&input);
}

#[test]
fn combined_full_size_rotate() {
    let mut input = String::from("5\n1\n256");
    for i in 0..256 {
        input.push(' ');
        input.push_str(&(i & 0xff).to_string());
    }
    input.push_str("\n100\n");
    compare(&input);
}
