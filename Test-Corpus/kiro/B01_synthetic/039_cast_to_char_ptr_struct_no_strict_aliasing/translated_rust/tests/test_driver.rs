use std::io::Read;
use std::os::unix::io::FromRawFd;

fn c_lib_path() -> String {
    std::env::current_dir()
        .unwrap()
        .join("c_src/build/libdriver.so")
        .to_str()
        .unwrap()
        .to_string()
}

fn rust_lib_path() -> String {
    // Find the Rust cdylib in target/debug/
    let dir = std::env::current_dir().unwrap().join("target/debug");
    for entry in std::fs::read_dir(&dir).expect("target/debug not found") {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("libdriver") && name.ends_with(".so") {
            return entry.path().to_str().unwrap().to_string();
        }
    }
    panic!("Rust libdriver.so not found in {:?}", dir);
}

fn capture_driver_from_lib(lib_path: &str, floors: i32) -> String {
    unsafe {
        let lib = libloading::Library::new(lib_path).expect("Failed to load library");
        let driver: libloading::Symbol<unsafe extern "C" fn(i32)> =
            lib.get(b"driver").expect("Failed to find driver symbol");

        capture_stdout(|| driver(floors))
    }
}

fn capture_stdout<F: FnOnce()>(f: F) -> String {
    unsafe {
        libc::fflush(std::ptr::null_mut());
        let mut fds = [0i32; 2];
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0);

        let old_stdout = libc::dup(1);
        libc::dup2(fds[1], 1);

        f();

        libc::fflush(std::ptr::null_mut());
        libc::dup2(old_stdout, 1);
        libc::close(old_stdout);
        libc::close(fds[1]);

        let mut result = String::new();
        let mut reader = std::fs::File::from_raw_fd(fds[0]);
        reader.read_to_string(&mut result).unwrap();
        result
    }
}

fn test_driver_value(floors: i32) {
    let c_out = capture_driver_from_lib(&c_lib_path(), floors);
    let r_out = capture_driver_from_lib(&rust_lib_path(), floors);
    assert_eq!(c_out, r_out, "driver({}) mismatch:\n  C:    {:?}\n  Rust: {:?}", floors, c_out, r_out);
}

#[test]
fn test_driver_floors_0() { test_driver_value(0); }

#[test]
fn test_driver_floors_5() { test_driver_value(5); }

#[test]
fn test_driver_floors_neg1() { test_driver_value(-1); }

#[test]
fn test_driver_floors_max() { test_driver_value(i32::MAX); }
