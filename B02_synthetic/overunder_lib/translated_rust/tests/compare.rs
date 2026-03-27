use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::io::Read;
use std::os::unix::io::FromRawFd;

const C_LIB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libtranslated_rust.so");

fn rust_lib_path() -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let p = format!("{manifest}/target/debug/liboverunder_lib.so");
    if std::path::Path::new(&p).exists() { return p; }
    panic!("Could not find Rust .so at {p}");
}

fn capture_stdout<F: FnOnce() -> R, R>(f: F) -> (R, String) {
    use std::io::Write;
    std::io::stdout().flush().unwrap();
    unsafe { libc::fflush(std::ptr::null_mut()); }

    let mut pipe_fds = [0i32; 2];
    unsafe { libc::pipe(pipe_fds.as_mut_ptr()); }
    let old_stdout = unsafe { libc::dup(1) };
    unsafe { libc::dup2(pipe_fds[1], 1); }

    let result = f();

    std::io::stdout().flush().unwrap();
    unsafe { libc::fflush(std::ptr::null_mut()); }
    unsafe { libc::dup2(old_stdout, 1); }
    unsafe { libc::close(old_stdout); }
    unsafe { libc::close(pipe_fds[1]); }

    let mut output = String::new();
    let mut reader = unsafe { std::fs::File::from_raw_fd(pipe_fds[0]) };
    reader.read_to_string(&mut output).unwrap();
    (result, output)
}

// ---- Tests for internal C functions vs Rust equivalents ----

#[test]
fn test_safe_double_to_int() {
    let c_lib = unsafe { Library::new(C_LIB_PATH).unwrap() };
    let c_fn: Symbol<unsafe extern "C" fn(f64) -> c_int> =
        unsafe { c_lib.get(b"safe_double_to_int").unwrap() };
    let rs_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    // Rust doesn't export safe_double_to_int yet, so test C behavior
    // and verify against expected values that Rust should also produce
    let cases: &[(f64, c_int)] = &[
        (0.0, 0),
        (1.5, 1),
        (-1.5, -1),
        (2147483647.0, i32::MAX),    // exactly INT_MAX
        (1e15, i32::MAX),            // overflow
        (-1e15, i32::MIN),           // underflow
        (f64::NAN, 0),
        (f64::INFINITY, i32::MAX),
        (f64::NEG_INFINITY, i32::MIN),
        (2.7, 2),
        (-2.7, -2),
        (100.9, 100),
    ];

    for &(input, expected) in cases {
        let c_result = unsafe { c_fn(input) };
        assert_eq!(c_result, expected,
            "C safe_double_to_int({input}) = {c_result}, expected {expected}");
    }

    // If Rust exports it, test that too
    if let Ok(rs_fn) = unsafe { rs_lib.get::<unsafe extern "C" fn(f64) -> c_int>(b"safe_double_to_int") } {
        for &(input, _) in cases {
            let c_result = unsafe { c_fn(input) };
            let rs_result = unsafe { rs_fn(input) };
            assert_eq!(c_result, rs_result,
                "safe_double_to_int({input}): C={c_result}, Rust={rs_result}");
        }
    }
}

#[test]
fn test_process_with_fallthrough() {
    let c_lib = unsafe { Library::new(C_LIB_PATH).unwrap() };
    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int) -> c_int> =
        unsafe { c_lib.get(b"process_with_fallthrough").unwrap() };
    let rs_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    let cases: &[(c_int, c_int)] = &[
        (0, 10), (1, 10), (2, 10), (3, 10), (4, 10), (5, 10),
        (6, 10), (-1, 10), (0, 0), (5, 0), (3, 100),
    ];

    for &(code, base) in cases {
        let c_result = unsafe { c_fn(code, base) };
        // Verify against expected fallthrough behavior
        let expected = match code {
            5 => base + 50 + 40 + 30 + 20 + 10,
            4 => base + 40 + 30 + 20 + 10,
            3 => base + 30 + 20 + 10,
            2 => base + 20 + 10,
            1 => base + 10,
            0 => 0,
            _ => -1,
        };
        assert_eq!(c_result, expected,
            "C process_with_fallthrough({code}, {base}) = {c_result}, expected {expected}");
    }

    if let Ok(rs_fn) = unsafe { rs_lib.get::<unsafe extern "C" fn(c_int, c_int) -> c_int>(b"process_with_fallthrough") } {
        for &(code, base) in cases {
            let c_result = unsafe { c_fn(code, base) };
            let rs_result = unsafe { rs_fn(code, base) };
            assert_eq!(c_result, rs_result,
                "process_with_fallthrough({code}, {base}): C={c_result}, Rust={rs_result}");
        }
    }
}

#[test]
fn test_handle_pointer_operations() {
    let c_lib = unsafe { Library::new(C_LIB_PATH).unwrap() };
    let c_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
        unsafe { c_lib.get(b"handle_pointer_operations").unwrap() };
    let rs_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    let cases: &[c_int] = &[0, 1, -1, 50, 100, -100];

    for &val in cases {
        let c_result = unsafe { c_fn(val) };
        let expected = val * 2 + 100;
        assert_eq!(c_result, expected,
            "C handle_pointer_operations({val}) = {c_result}, expected {expected}");
    }

    if let Ok(rs_fn) = unsafe { rs_lib.get::<unsafe extern "C" fn(c_int) -> c_int>(b"handle_pointer_operations") } {
        for &val in cases {
            let c_result = unsafe { c_fn(val) };
            let rs_result = unsafe { rs_fn(val) };
            assert_eq!(c_result, rs_result,
                "handle_pointer_operations({val}): C={c_result}, Rust={rs_result}");
        }
    }
}

#[repr(C)]
struct DataBlock {
    id: c_int,
    value: f64,
    label: [u8; 20],
}

#[test]
fn test_copy_data_block() {
    let c_lib = unsafe { Library::new(C_LIB_PATH).unwrap() };
    let c_fn: Symbol<unsafe extern "C" fn(*mut DataBlock, *const DataBlock)> =
        unsafe { c_lib.get(b"copy_data_block").unwrap() };

    let mut src = DataBlock { id: 42, value: 3.14, label: [0u8; 20] };
    src.label[..6].copy_from_slice(b"Hello\0");
    let mut dest = DataBlock { id: 0, value: 0.0, label: [0u8; 20] };

    unsafe { c_fn(&mut dest, &src); }

    assert_eq!(dest.id, 42);
    assert_eq!(dest.value, 3.14);
    assert_eq!(&dest.label[..6], b"Hello\0");
}

// ---- Main overunder comparison ----

#[test]
fn test_overunder_matches() {
    let c_lib = unsafe { Library::new(C_LIB_PATH).unwrap() };
    let rs_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
        unsafe { c_lib.get(b"overunder").unwrap() };
    let rs_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
        unsafe { rs_lib.get(b"overunder").unwrap() };

    let test_cases: &[(c_int, c_int, c_int, c_int)] = &[
        (1, 2, 3, 4),
        (0, 0, 0, 0),
        (5, 10, 15, 20),
        (-1, -2, -3, -4),
        (100, 200, 300, 400),
        (3, 7, 11, 13),
        (6, 1, 1, 1),
        (11, 0, 0, 0),
        (2, 3, 0, 0),
        (1, 1, 1, 1),
    ];

    for &(a, b, c, d) in test_cases {
        let (c_ret, c_out) = capture_stdout(|| unsafe { c_fn(a, b, c, d) });
        let (rs_ret, rs_out) = capture_stdout(|| unsafe { rs_fn(a, b, c, d) });

        assert_eq!(
            c_ret, rs_ret,
            "Return value mismatch for overunder({a}, {b}, {c}, {d}): C={c_ret}, Rust={rs_ret}"
        );
        assert_eq!(
            c_out, rs_out,
            "Stdout mismatch for overunder({a}, {b}, {c}, {d}):\n--- C ---\n{c_out}\n--- Rust ---\n{rs_out}"
        );
    }
}
