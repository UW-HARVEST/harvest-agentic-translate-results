// Integration test that loads both the C .so and the Rust .so via libloading
// and compares the output of sh_puts. Because sh_puts writes to stdout via
// printf, we redirect stdout to a pipe to capture the output.

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::os::raw::c_void;

const C_LIB: &str = "c_src/build/libtranslated_rust.so";
const RUST_LIB: &str = "target/release/libsh_puts_lib.so";

unsafe fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    // Flush stdout first
    extern "C" {
        fn fflush(stream: *mut c_void) -> c_int;
        fn dup(fd: c_int) -> c_int;
        fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
        fn close(fd: c_int) -> c_int;
        fn pipe(pipefd: *mut c_int) -> c_int;
    }
    fflush(std::ptr::null_mut());
    let saved = dup(1);
    let mut fds: [c_int; 2] = [0; 2];
    pipe(fds.as_mut_ptr());
    dup2(fds[1], 1);
    close(fds[1]);

    f();

    fflush(std::ptr::null_mut());
    dup2(saved, 1);
    close(saved);

    let mut file = File::from_raw_fd(fds[0]);
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).unwrap();
    buf
}

fn run_sh_puts(lib_path: &str, num: c_int) -> Vec<u8> {
    unsafe {
        let lib = Library::new(lib_path).expect("library loads");
        let func: Symbol<unsafe extern "C" fn(c_int)> =
            lib.get(b"sh_puts").expect("sh_puts symbol");
        capture_stdout(|| func(num))
    }
}

#[test]
fn sh_puts_matches() {
    for &num in &[0i32, 1, 5, 10, 42, -1, 100] {
        let c_out = run_sh_puts(C_LIB, num);
        let rust_out = run_sh_puts(RUST_LIB, num);
        assert_eq!(
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&rust_out),
            "sh_puts mismatch for num={}",
            num
        );
    }
}
