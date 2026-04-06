use std::process::Command;

#[test]
fn test_driver_outputs_match() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let c_bin = format!("{}/c_src/build/driver", manifest_dir);

    // Build the Rust binary
    let status = Command::new("cargo")
        .args(["build", "--bin", "driver", "--features", "_bin"])
        .current_dir(manifest_dir)
        .status()
        .expect("Failed to build Rust binary");
    assert!(status.success(), "Rust binary build failed");

    let rust_bin = format!("{}/target/debug/driver", manifest_dir);

    for x in [0i32, 1, 5, 10] {
        let c_out = Command::new(&c_bin)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child.stdin.take().unwrap().write_all(format!("{}", x).as_bytes()).unwrap();
                child.wait_with_output()
            })
            .expect("Failed to run C binary");

        let rust_out = Command::new(&rust_bin)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child.stdin.take().unwrap().write_all(format!("{}", x).as_bytes()).unwrap();
                child.wait_with_output()
            })
            .expect("Failed to run Rust binary");

        assert_eq!(
            c_out.stdout, rust_out.stdout,
            "Mismatch for driver({})\nC:    {:?}\nRust: {:?}",
            x,
            String::from_utf8_lossy(&c_out.stdout),
            String::from_utf8_lossy(&rust_out.stdout)
        );
    }
}
