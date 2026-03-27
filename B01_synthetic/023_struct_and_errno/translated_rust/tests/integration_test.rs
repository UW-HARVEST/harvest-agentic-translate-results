use libloading::{Library, Symbol};
use std::process::{Command, Stdio};
use std::io::Write;
use driver::House;

fn c_lib_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so")
}

/// Test `run` by comparing struct state (not stdout) between C and Rust
#[test]
fn test_run_struct_state_matches_c() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("Failed to load C .so") };

    for extra in &[0, 1, 3, 10, -1] {
        let mut c_house = House { floors: 2, bedrooms: 5, bathrooms: 2.5 };
        let mut rust_house = House { floors: 2, bedrooms: 5, bathrooms: 2.5 };

        unsafe {
            let c_run: Symbol<unsafe extern "C" fn(*mut House, i32)> =
                c_lib.get(b"run").expect("Failed to find 'run' in C .so");
            c_run(&mut c_house, *extra);
        }

        // Redirect Rust stdout to /dev/null to suppress output during test
        driver::run(&mut rust_house, *extra);

        assert_eq!(c_house.floors, rust_house.floors, "floors mismatch for extra={extra}");
        assert_eq!(c_house.bedrooms, rust_house.bedrooms, "bedrooms mismatch for extra={extra}");
        assert!(
            (c_house.bathrooms - rust_house.bathrooms).abs() < 1e-10,
            "bathrooms mismatch for extra={extra}: C={} Rust={}",
            c_house.bathrooms, rust_house.bathrooms
        );
    }
}

/// Test double-run struct state
#[test]
fn test_double_run_struct_state_matches_c() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("Failed to load C .so") };

    let mut c_house = House { floors: 2, bedrooms: 5, bathrooms: 2.5 };
    let mut rust_house = House { floors: 2, bedrooms: 5, bathrooms: 2.5 };

    unsafe {
        let c_run: Symbol<unsafe extern "C" fn(*mut House, i32)> =
            c_lib.get(b"run").expect("Failed to find 'run'");
        c_run(&mut c_house, 3);
        c_run(&mut c_house, 3);
    }

    driver::run(&mut rust_house, 3);
    driver::run(&mut rust_house, 3);

    assert_eq!(c_house.floors, rust_house.floors);
    assert_eq!(c_house.bedrooms, rust_house.bedrooms);
    assert!((c_house.bathrooms - rust_house.bathrooms).abs() < 1e-10);
}

/// Test full binary output comparison (C executable vs Rust executable)
#[test]
fn test_binary_output_matches() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let c_binary = format!("{}/c_src/build/driver", manifest_dir);

    let build = Command::new("timeout")
        .args(["60", "cargo", "build", "--bin", "driver"])
        .current_dir(manifest_dir)
        .output()
        .expect("Failed to build Rust binary");
    assert!(build.status.success(), "Rust binary build failed: {}", String::from_utf8_lossy(&build.stderr));

    let rust_binary = format!("{}/target/debug/driver", manifest_dir);

    for input in &["3\n", "0\n", "-5\n", "abc\n", "\n"] {
        let c_out = run_binary(&c_binary, input);
        let rust_out = run_binary(&rust_binary, input);

        assert_eq!(
            c_out, rust_out,
            "Binary stdout mismatch for input {:?}\nC:    {:?}\nRust: {:?}",
            input, c_out, rust_out
        );
    }
}

fn run_binary(path: &str, input: &str) -> String {
    let mut child = Command::new(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("Failed to run {}: {}", path, e));
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    String::from_utf8_lossy(&out.stdout).to_string()
}
