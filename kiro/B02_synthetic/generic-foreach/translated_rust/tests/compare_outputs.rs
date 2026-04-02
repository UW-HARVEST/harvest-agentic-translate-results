use std::io::Write;

fn manifest_dir() -> String {
    env!("CARGO_MANIFEST_DIR").to_string()
}

fn run_binary(cmd: &str, args: &[&str], input: &str) -> String {
    let output = std::process::Command::new(cmd)
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            if let Some(ref mut stdin) = child.stdin {
                stdin.write_all(input.as_bytes()).ok();
            }
            child.wait_with_output()
        })
        .expect("Failed to run binary");
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn c_binary() -> String {
    format!("{}/c_build/driver", manifest_dir())
}

fn rust_binary() -> String {
    // Build first, then return path to binary
    let status = std::process::Command::new("cargo")
        .args(["build", "--bin", "driver", "--manifest-path",
            &format!("{}/Cargo.toml", manifest_dir())])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to build");
    assert!(status.success(), "cargo build failed");
    format!("{}/target/debug/driver", manifest_dir())
}

fn assert_match(test_name: &str, c_out: &str, r_out: &str) {
    if c_out != r_out {
        let c_lines: Vec<&str> = c_out.lines().collect();
        let r_lines: Vec<&str> = r_out.lines().collect();
        let max = c_lines.len().max(r_lines.len());
        for i in 0..max {
            let cl = c_lines.get(i).unwrap_or(&"<MISSING>");
            let rl = r_lines.get(i).unwrap_or(&"<MISSING>");
            if cl != rl {
                panic!(
                    "{} mismatch at line {}:\n  C:    {:?}\n  Rust: {:?}",
                    test_name, i + 1, cl, rl
                );
            }
        }
        panic!("{} length differs: C={}, Rust={}", test_name, c_lines.len(), r_lines.len());
    }
}

// Run each demo by sending the menu choice, then exit with 7
fn run_demo(choice: &str) -> (String, String) {
    let input = format!("{}\n7\n", choice);
    let c_out = run_binary(&c_binary(), &[], &input);
    let r_out = run_binary(&rust_binary(), &[], &input);

    // Strip the banner (first 3 lines) and menu/exit from both outputs
    // to isolate just the demo output. Actually, let's compare full output.
    (c_out, r_out)
}

#[test]
fn test_01_demo_integer_containers() {
    let (c_out, r_out) = run_demo("1");
    assert_match("demo_integer_containers", &c_out, &r_out);
}

#[test]
fn test_02_demo_double_containers() {
    let (c_out, r_out) = run_demo("2");
    assert_match("demo_double_containers", &c_out, &r_out);
}

#[test]
fn test_03_demo_inventory_array() {
    let (c_out, r_out) = run_demo("3");
    assert_match("demo_inventory_array", &c_out, &r_out);
}

#[test]
fn test_04_demo_order_list() {
    let (c_out, r_out) = run_demo("4");
    assert_match("demo_order_list", &c_out, &r_out);
}

#[test]
fn test_05_demo_mixed_operations() {
    let (c_out, r_out) = run_demo("5");
    assert_match("demo_mixed_operations", &c_out, &r_out);
}

#[test]
fn test_06_run_all_demos() {
    let (c_out, r_out) = run_demo("6");
    assert_match("run_all_demos", &c_out, &r_out);
}

#[test]
fn test_07_exit() {
    let c_out = run_binary(&c_binary(), &[], "7\n");
    let r_out = run_binary(&rust_binary(), &[], "7\n");
    assert_match("exit", &c_out, &r_out);
}

#[test]
fn test_08_invalid_input() {
    let c_out = run_binary(&c_binary(), &[], "abc\n7\n");
    let r_out = run_binary(&rust_binary(), &[], "abc\n7\n");
    assert_match("invalid_input", &c_out, &r_out);
}

#[test]
fn test_09_invalid_choice() {
    let c_out = run_binary(&c_binary(), &[], "99\n7\n");
    let r_out = run_binary(&rust_binary(), &[], "99\n7\n");
    assert_match("invalid_choice", &c_out, &r_out);
}

#[test]
fn test_10_multiple_demos() {
    let c_out = run_binary(&c_binary(), &[], "1\n2\n3\n4\n5\n7\n");
    let r_out = run_binary(&rust_binary(), &[], "1\n2\n3\n4\n5\n7\n");
    assert_match("multiple_demos", &c_out, &r_out);
}
