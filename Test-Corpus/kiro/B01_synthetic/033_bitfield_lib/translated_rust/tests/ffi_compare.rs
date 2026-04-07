use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::ffi::c_uint;
use std::io::Read;
use std::os::unix::io::FromRawFd;

mod libc_ffi {
    extern "C" {
        pub fn pipe(pipefd: *mut i32) -> i32;
        pub fn dup(oldfd: i32) -> i32;
        pub fn dup2(oldfd: i32, newfd: i32) -> i32;
        pub fn close(fd: i32) -> i32;
        pub fn fflush(stream: *mut std::ffi::c_void) -> i32;
        pub fn fcntl(fd: i32, cmd: i32, ...) -> i32;
    }
}

mod libc {
    pub use super::libc_ffi::*;
    pub const F_SETFL: i32 = 4;
    pub const O_NONBLOCK: i32 = 2048;
}

fn capture_stdout<F: FnOnce()>(f: F) -> String {
    unsafe {
        libc::fflush(std::ptr::null_mut());
        let mut pipe_fds = [0i32; 2];
        assert_eq!(libc::pipe(pipe_fds.as_mut_ptr()), 0);
        let saved = libc::dup(1);
        libc::dup2(pipe_fds[1], 1);
        libc::close(pipe_fds[1]);
        f();
        libc::fflush(std::ptr::null_mut());
        libc::dup2(saved, 1);
        libc::close(saved);
        let mut buf = String::new();
        let mut read_end = std::fs::File::from_raw_fd(pipe_fds[0]);
        libc::fcntl(pipe_fds[0], libc::F_SETFL, libc::O_NONBLOCK);
        let _ = read_end.read_to_string(&mut buf);
        buf
    }
}

type DriverFn = unsafe extern "C" fn(c_uint, c_uint, bool, c_int);

#[repr(C)]
struct FooT {
    _bitfield: c_uint,
    z: c_int,
}

type PrintFooFn = unsafe extern "C" fn(*const FooT);

fn c_lib() -> Library {
    unsafe { Library::new(concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so")).unwrap() }
}

fn rust_lib() -> Library {
    unsafe { Library::new(concat!(env!("CARGO_MANIFEST_DIR"), "/target/debug/libdriver.so")).unwrap() }
}

fn call_driver(lib: &Library, x: c_uint, y: c_uint, b: bool, z: c_int) -> String {
    capture_stdout(|| unsafe {
        let f: Symbol<DriverFn> = lib.get(b"driver").unwrap();
        f(x, y, b, z);
    })
}

fn call_print_foo(lib: &Library, bf: c_uint, z: c_int) -> String {
    capture_stdout(|| unsafe {
        let f: Symbol<PrintFooFn> = lib.get(b"print_foo").unwrap();
        let foo = FooT { _bitfield: bf, z };
        f(&foo as *const FooT);
    })
}

#[test]
fn driver_matches_c() {
    let c = c_lib();
    let r = rust_lib();

    let cases: Vec<(c_uint, c_uint, bool, c_int)> = vec![
        (0, 0, false, 0),
        (0, 0, true, 0),
        (1, 1, false, 1),
        (1, 1, true, 1),
        (2, 4, false, -1),
        (2, 4, true, -1),
        (3, 7, false, 42),
        (3, 7, true, 42),
        (4, 8, false, -100),
        (4, 8, true, -100),
        (5, 15, false, 0x7FFFFFFF),
        (5, 15, true, 0x7FFFFFFF),
        (0xFFFFFFFF, 0xFFFFFFFF, false, 0),
        (0xFFFFFFFF, 0xFFFFFFFF, true, 0),
        (0, 0, false, i32::MIN),
        (3, 7, true, i32::MAX),
        (0, 0, true, -1),
    ];

    for (x, y, b, z) in cases {
        let c_out = call_driver(&c, x, y, b, z);
        let r_out = call_driver(&r, x, y, b, z);
        assert_eq!(c_out, r_out, "MISMATCH driver({x}, {y}, {b}, {z}): C={c_out:?} Rust={r_out:?}");
    }
}

#[test]
fn print_foo_matches_c() {
    let c = c_lib();
    let r = rust_lib();

    let cases: Vec<(c_uint, c_int)> = vec![
        (0x00, 0),
        (0x3F, 42),       // x=3, y=7, b=1
        (0x03, -1),        // x=3, y=0, b=0
        (0x1C, 100),       // x=0, y=7, b=0
        (0x20, i32::MIN),  // x=0, y=0, b=1
        (0x3F, i32::MAX),
    ];

    for (bf, z) in cases {
        let c_out = call_print_foo(&c, bf, z);
        let r_out = call_print_foo(&r, bf, z);
        assert_eq!(c_out, r_out, "MISMATCH print_foo(bf={bf:#x}, z={z}): C={c_out:?} Rust={r_out:?}");
    }
}
