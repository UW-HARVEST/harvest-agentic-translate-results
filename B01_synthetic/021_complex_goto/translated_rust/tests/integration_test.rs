use std::process::Command;

fn run_c_binary(x: i32, y: i32) -> String {
    let out = Command::new("./c_src/build/driver")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(format!("{} {}", x, y).as_bytes()).unwrap();
            child.wait_with_output()
        })
        .expect("failed to run C binary");
    String::from_utf8(out.stdout).unwrap()
}

fn run_rust_binary(x: i32, y: i32) -> String {
    let out = Command::new("./target/debug/driver")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(format!("{} {}", x, y).as_bytes()).unwrap();
            child.wait_with_output()
        })
        .expect("failed to run Rust binary");
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn test_binary_output_matches() {
    for x in 0..=10 {
        for y in 0..=10 {
            let c_out = run_c_binary(x, y);
            let r_out = run_rust_binary(x, y);
            assert_eq!(
                c_out.as_bytes(),
                r_out.as_bytes(),
                "Mismatch for x={}, y={}\nC:  {:?}\nRust: {:?}",
                x, y, c_out, r_out
            );
        }
    }
}
