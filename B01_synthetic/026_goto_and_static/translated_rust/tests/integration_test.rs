use std::process::Command;

fn run_c_binary(input: &str) -> String {
    let c_bin = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/driver");
    let out = Command::new(c_bin)
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
    String::from_utf8(out.stdout).unwrap()
}

fn run_rust_binary(input: &str) -> String {
    let rust_bin = env!("CARGO_BIN_EXE_driver");
    let out = Command::new(rust_bin)
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
    String::from_utf8(out.stdout).unwrap()
}

// Test case: x=1, y=2, z=3 -> all checks pass -> "Ok!\nResult: 0\n"
#[test]
fn test_all_pass() {
    let input = "1 2 3\n";
    let c_out = run_c_binary(input);
    let r_out = run_rust_binary(input);
    assert_eq!(c_out, r_out, "Mismatch for input '1 2 3':\nC:    {:?}\nRust: {:?}", c_out, r_out);
}

// Test case: x=0 -> first check fails -> "Error: x != 1\nOperation failed\nResult: 1\n"
#[test]
fn test_x_fail() {
    let input = "0 2 3\n";
    let c_out = run_c_binary(input);
    let r_out = run_rust_binary(input);
    assert_eq!(c_out, r_out, "Mismatch for input '0 2 3':\nC:    {:?}\nRust: {:?}", c_out, r_out);
}

// Test case: x=1, y=0, z=3 -> second check fails
#[test]
fn test_y_fail() {
    let input = "1 0 3\n";
    let c_out = run_c_binary(input);
    let r_out = run_rust_binary(input);
    assert_eq!(c_out, r_out, "Mismatch for input '1 0 3':\nC:    {:?}\nRust: {:?}", c_out, r_out);
}

// Test case: x=1, y=2, z=0 -> third check fails
#[test]
fn test_z_fail() {
    let input = "1 2 0\n";
    let c_out = run_c_binary(input);
    let r_out = run_rust_binary(input);
    assert_eq!(c_out, r_out, "Mismatch for input '1 2 0':\nC:    {:?}\nRust: {:?}", c_out, r_out);
}

// Test nm -D symbols: C .so exports must be present in Rust .so
#[test]
fn test_nm_symbols() {
    let c_so = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so");

    // Find the Rust cdylib
    let target_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    let rust_so = find_rust_so(&target_dir);

    let c_syms = get_dynamic_symbols(c_so.to_str().unwrap());
    let r_syms = get_dynamic_symbols(rust_so.to_str().unwrap());

    for sym in &c_syms {
        assert!(r_syms.contains(sym), "C exports symbol '{}' but Rust .so does not", sym);
    }
}

fn find_rust_so(target_dir: &std::path::Path) -> std::path::PathBuf {
    // Look in debug directory for libgoto_and_static.so
    let debug_so = target_dir.join("debug/libgoto_and_static.so");
    if debug_so.exists() {
        return debug_so;
    }
    // Fallback: search recursively
    for entry in walkdir(target_dir) {
        if entry.ends_with("libgoto_and_static.so") {
            return entry;
        }
    }
    panic!("Could not find Rust .so file");
}

fn walkdir(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut results = vec![];
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                results.extend(walkdir(&path));
            } else {
                results.push(path);
            }
        }
    }
    results
}

fn get_dynamic_symbols(path: &str) -> Vec<String> {
    let output = Command::new("nm")
        .args(&["-D", path])
        .output()
        .expect("failed to run nm");
    let stdout = String::from_utf8(output.stdout).unwrap();
    stdout.lines()
        .filter(|l| l.contains(" T "))
        .filter_map(|l| {
            let sym = l.split_whitespace().last()?;
            // Skip linker-generated symbols
            if sym.starts_with('_') { return None; }
            Some(sym.to_string())
        })
        .collect()
}
