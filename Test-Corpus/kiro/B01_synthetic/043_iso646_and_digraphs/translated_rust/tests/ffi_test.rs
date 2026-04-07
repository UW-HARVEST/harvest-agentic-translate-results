use libloading::{Library, Symbol};
use std::io::Read;
use std::os::unix::io::FromRawFd;

fn capture_stdout<F: FnOnce()>(f: F) -> String {
    unsafe { libc::fflush(std::ptr::null_mut()) };
    let mut pipes = [0i32; 2];
    unsafe { libc::pipe(pipes.as_mut_ptr()) };
    let old_stdout = unsafe { libc::dup(1) };
    unsafe { libc::dup2(pipes[1], 1) };
    f();
    unsafe { libc::fflush(std::ptr::null_mut()) };
    use std::io::Write;
    std::io::stdout().flush().ok();
    unsafe { libc::dup2(old_stdout, 1) };
    unsafe { libc::close(old_stdout) };
    unsafe { libc::close(pipes[1]) };
    let mut buf = String::new();
    let mut reader = unsafe { std::fs::File::from_raw_fd(pipes[0]) };
    reader.read_to_string(&mut buf).unwrap();
    buf
}

fn rust_so_path() -> String {
    // cargo test builds in target/debug/deps, the cdylib is in target/debug/
    let manifest = env!("CARGO_MANIFEST_DIR");
    format!("{}/target/debug/libdriver.so", manifest)
}

const C_SO: &str = "/tmp/harvest-work-AIEaSj/translated_rust/c_src/build/libdriver_c.so";

#[test]
fn test_driver_matches() {
    let test_cases: &[(i32, i32)] = &[
        (0, 0),
        (1, 1),
        (-1, -1),
        (0, -1),
        (-1, 0),
        (0x7FFF_FFFF, 0),
        (0, 0x7FFF_FFFF),
        (0x7FFF_FFFF, 0x7FFF_FFFF),
        (42, 17),
        (0xFF, 0xFF00),
    ];

    let c = unsafe { Library::new(C_SO).unwrap() };
    let r = unsafe { Library::new(rust_so_path()).unwrap() };

    for &(x, y) in test_cases {
        let c_out = {
            let f: Symbol<unsafe extern "C" fn(i32, i32)> = unsafe { c.get(b"driver").unwrap() };
            capture_stdout(|| unsafe { f(x, y) })
        };
        let r_out = {
            let f: Symbol<unsafe extern "C" fn(i32, i32)> = unsafe { r.get(b"driver").unwrap() };
            capture_stdout(|| unsafe { f(x, y) })
        };
        assert_eq!(c_out, r_out, "mismatch for driver({}, {}): C={:?} Rust={:?}", x, y, c_out, r_out);
    }
}
