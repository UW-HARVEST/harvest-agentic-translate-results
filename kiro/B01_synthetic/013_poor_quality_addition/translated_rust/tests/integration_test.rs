use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::c_char;

const C_LIB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so");
const RUST_LIB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/target/debug/libdriver.so");

/// Capture stdout from a C function call by redirecting fd 1 to a pipe.
/// Works for C functions that use printf (writes directly to fd 1).
fn capture_c_stdout<F: FnOnce()>(f: F) -> String {
    use std::io::Read;
    use std::os::unix::io::FromRawFd;

    unsafe { libc::fflush(std::ptr::null_mut()); }
    let mut pipes = [0i32; 2];
    unsafe { libc::pipe(pipes.as_mut_ptr()); }
    let old_stdout = unsafe { libc::dup(1) };
    unsafe { libc::dup2(pipes[1], 1); }

    f();

    unsafe {
        libc::fflush(std::ptr::null_mut());
        libc::dup2(old_stdout, 1);
        libc::close(old_stdout);
        libc::close(pipes[1]);
    }

    let mut buf = String::new();
    let mut reader = unsafe { std::fs::File::from_raw_fd(pipes[0]) };
    reader.read_to_string(&mut buf).unwrap();
    buf
}

/// Run a small helper program that loads a .so and calls a function,
/// capturing its stdout. This works for both C and Rust .so files.
fn call_void_fn_via_subprocess(lib_path: &str, fn_name: &str) -> Vec<u8> {
    // Use a small Python script to dlopen and call the function
    let script = format!(
        "import ctypes; lib = ctypes.CDLL('{}'); getattr(lib, '{}')()",
        lib_path, fn_name
    );
    let out = std::process::Command::new("python3")
        .args(&["-c", &script])
        .output()
        .expect("Failed to run python3");
    out.stdout
}

fn call_int_fn_via_subprocess(lib_path: &str, fn_name: &str, arg: i32) -> Vec<u8> {
    let script = format!(
        "import ctypes; lib = ctypes.CDLL('{}'); f = getattr(lib, '{}'); f.argtypes = [ctypes.c_int]; f({})",
        lib_path, fn_name, arg
    );
    let out = std::process::Command::new("python3")
        .args(&["-c", &script])
        .output()
        .expect("Failed to run python3");
    out.stdout
}

fn call_str_fn_via_subprocess(lib_path: &str, fn_name: &str, arg: &str) -> Vec<u8> {
    let script = format!(
        "import ctypes; lib = ctypes.CDLL('{}'); f = getattr(lib, '{}'); f.argtypes = [ctypes.c_char_p]; f(b'{}')",
        lib_path, fn_name, arg
    );
    let out = std::process::Command::new("python3")
        .args(&["-c", &script])
        .output()
        .expect("Failed to run python3");
    out.stdout
}

#[test]
fn test_print_int_line() {
    for &val in &[0i32, 1, -1, 42, i32::MAX, i32::MIN] {
        let c_out = call_int_fn_via_subprocess(C_LIB_PATH, "printIntLine", val);
        let r_out = call_int_fn_via_subprocess(RUST_LIB_PATH, "printIntLine", val);
        assert_eq!(c_out, r_out, "printIntLine mismatch for {}\nC: {:?}\nRust: {:?}",
            val, String::from_utf8_lossy(&c_out), String::from_utf8_lossy(&r_out));
    }
}

#[test]
fn test_print_line() {
    for &s in &["hello", "test 123", "Calling good()..."] {
        let c_out = call_str_fn_via_subprocess(C_LIB_PATH, "printLine", s);
        let r_out = call_str_fn_via_subprocess(RUST_LIB_PATH, "printLine", s);
        assert_eq!(c_out, r_out, "printLine mismatch for {:?}\nC: {:?}\nRust: {:?}",
            s, String::from_utf8_lossy(&c_out), String::from_utf8_lossy(&r_out));
    }
}

#[test]
fn test_print_line_null() {
    // NULL test via C: pass NULL pointer
    let c_script = format!(
        "import ctypes; lib = ctypes.CDLL('{}'); f = lib.printLine; f.argtypes = [ctypes.c_char_p]; f(None)",
        C_LIB_PATH
    );
    let r_script = format!(
        "import ctypes; lib = ctypes.CDLL('{}'); f = lib.printLine; f.argtypes = [ctypes.c_char_p]; f(None)",
        RUST_LIB_PATH
    );
    let c_out = std::process::Command::new("python3").args(&["-c", &c_script]).output().unwrap().stdout;
    let r_out = std::process::Command::new("python3").args(&["-c", &r_script]).output().unwrap().stdout;
    assert_eq!(c_out, r_out, "printLine NULL mismatch");
}

#[test]
fn test_bad() {
    let c_out = call_void_fn_via_subprocess(C_LIB_PATH, "bad");
    let r_out = call_void_fn_via_subprocess(RUST_LIB_PATH, "bad");
    assert_eq!(c_out, r_out, "bad() mismatch\nC: {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c_out), String::from_utf8_lossy(&r_out));
}

#[test]
fn test_good() {
    let c_out = call_void_fn_via_subprocess(C_LIB_PATH, "good");
    let r_out = call_void_fn_via_subprocess(RUST_LIB_PATH, "good");
    assert_eq!(c_out, r_out, "good() mismatch\nC: {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c_out), String::from_utf8_lossy(&r_out));
}

#[test]
fn test_main_binary_output() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let c_bin = format!("{}/c_src/build/driver", manifest_dir);

    let c_output = std::process::Command::new(&c_bin)
        .output().expect("Failed to run C binary");

    let rust_bin = std::process::Command::new("cargo")
        .args(&["run", "--quiet", "--bin", "driver"])
        .current_dir(manifest_dir)
        .output().expect("Failed to run Rust binary");

    assert_eq!(
        c_output.stdout, rust_bin.stdout,
        "Binary stdout mismatch:\nC: {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c_output.stdout),
        String::from_utf8_lossy(&rust_bin.stdout)
    );
}

#[test]
fn test_symbol_compatibility() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let c_so = format!("{}/c_src/build/libdriver.so", manifest_dir);
    let rust_so = format!("{}/target/debug/libdriver.so", manifest_dir);

    let get_syms = |path: &str| -> std::collections::HashSet<String> {
        let out = std::process::Command::new("nm")
            .args(&["-D", path])
            .output().expect("nm failed");
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| l.contains(" T "))
            .filter_map(|l| l.split_whitespace().nth(2).map(String::from))
            .filter(|s| !s.starts_with('_'))
            .collect()
    };

    let c_syms = get_syms(&c_so);
    let rust_syms = get_syms(&rust_so);
    let missing: Vec<_> = c_syms.difference(&rust_syms).collect();
    assert!(missing.is_empty(), "Rust .so missing C symbols: {:?}", missing);
}
