use std::process::Command;

/// Compare C and Rust binaries with the same stdin input.
/// Both `run(x)` calls twice with the value read from stdin.
#[test]
fn test_binary_output_matches() {
    let base = env!("CARGO_MANIFEST_DIR");

    // Build Rust binary
    let status = Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(base)
        .status()
        .expect("cargo build failed");
    assert!(status.success(), "cargo build failed");

    let c_bin = format!("{}/c_src/build/driver", base);
    let rust_bin = format!("{}/target/release/driver", base);

    // Test with several inputs
    for input in &["3\n", "0\n", "10\n", "-1\n"] {
        let c_out = Command::new(&c_bin)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
                child.wait_with_output()
            })
            .expect("failed to run C binary");

        let rust_out = Command::new(&rust_bin)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
                child.wait_with_output()
            })
            .expect("failed to run Rust binary");

        let c_stdout = String::from_utf8_lossy(&c_out.stdout);
        let rust_stdout = String::from_utf8_lossy(&rust_out.stdout);

        assert_eq!(
            c_out.stdout, rust_out.stdout,
            "Output mismatch for input {:?}:\nC:\n{}\nRust:\n{}",
            input, c_stdout, rust_stdout
        );
    }
}
