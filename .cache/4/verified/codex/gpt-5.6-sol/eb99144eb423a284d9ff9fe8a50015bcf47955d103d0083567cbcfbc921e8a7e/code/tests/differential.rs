use libloading::Library;
use std::ffi::{c_int, c_void};
use std::path::{Path, PathBuf};

type DriverFn = unsafe extern "C" fn(c_int);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
}

const STDOUT_FILENO: c_int = 1;

fn shared_object_path(env_name: &str, relative_path: &str) -> PathBuf {
    std::env::var_os(env_name)
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_path))
}

fn os_error(operation: &str) -> String {
    format!("{operation}: {}", std::io::Error::last_os_error())
}

unsafe fn capture_stdout(call: DriverFn, input: c_int) -> Result<Vec<u8>, String> {
    let mut pipe_fds = [-1; 2];
    if unsafe { pipe(pipe_fds.as_mut_ptr()) } != 0 {
        return Err(os_error("pipe"));
    }

    if unsafe { fflush(std::ptr::null_mut()) } != 0 {
        return Err(os_error("fflush before redirect"));
    }

    let saved_stdout = unsafe { dup(STDOUT_FILENO) };
    if saved_stdout < 0 {
        return Err(os_error("dup stdout"));
    }
    if unsafe { dup2(pipe_fds[1], STDOUT_FILENO) } < 0 {
        return Err(os_error("redirect stdout"));
    }

    unsafe { call(input) };

    if unsafe { fflush(std::ptr::null_mut()) } != 0 {
        return Err(os_error("fflush after call"));
    }
    if unsafe { dup2(saved_stdout, STDOUT_FILENO) } < 0 {
        return Err(os_error("restore stdout"));
    }
    unsafe {
        close(saved_stdout);
        close(pipe_fds[1]);
    }

    let mut output = Vec::new();
    let mut buffer = [0_u8; 64];
    loop {
        let bytes_read = unsafe { read(pipe_fds[0], buffer.as_mut_ptr().cast(), buffer.len()) };
        if bytes_read < 0 {
            unsafe { close(pipe_fds[0]) };
            return Err(os_error("read captured stdout"));
        }
        if bytes_read == 0 {
            break;
        }
        output.extend_from_slice(&buffer[..bytes_read as usize]);
    }
    unsafe { close(pipe_fds[0]) };
    Ok(output)
}

#[test]
fn config_1_driver_matches_for_full_int_domain_samples() {
    let c_path = shared_object_path("DRIVER_C_SO", "c_src/build/libdriver.so");
    let rust_path = shared_object_path("DRIVER_RUST_SO", "target/release/libdriver.so");

    assert!(
        c_path.is_file(),
        "missing C shared object: {}",
        c_path.display()
    );
    assert!(
        rust_path.is_file(),
        "missing Rust shared object: {}",
        rust_path.display()
    );

    let c_library = unsafe { Library::new(&c_path) }.expect("load C shared object");
    let rust_library = unsafe { Library::new(&rust_path) }.expect("load Rust shared object");
    let c_driver: DriverFn = *unsafe { c_library.get(b"driver\0") }.expect("load C driver");
    let rust_driver: DriverFn =
        *unsafe { rust_library.get(b"driver\0") }.expect("load Rust driver");

    let mut inputs = vec![
        c_int::MIN,
        c_int::MIN + 1,
        -151,
        -150,
        -149,
        -1,
        0,
        1,
        149,
        150,
        151,
        c_int::MAX - 1,
        c_int::MAX,
    ];

    let mut state = 0x6a09_e667_f3bc_c909_u64;
    for _ in 0..4096 {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        inputs.push((state >> 32) as c_int);
    }

    for input in inputs {
        let c_output = unsafe { capture_stdout(c_driver, input) }
            .unwrap_or_else(|error| panic!("capture C output for {input}: {error}"));
        let rust_output = unsafe { capture_stdout(rust_driver, input) }
            .unwrap_or_else(|error| panic!("capture Rust output for {input}: {error}"));
        assert_eq!(rust_output, c_output, "output mismatch for x={input}");
    }
}
