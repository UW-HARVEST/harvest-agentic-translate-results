use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::c_char;

fn capture_driver(lib: &Library, s1: &[u8], s2: &[u8]) -> String {
    use std::io::Read;
    use std::os::unix::io::FromRawFd;

    let mut fds = [0i32; 2];
    unsafe { libc_pipe(&mut fds) };
    let (read_fd, write_fd) = (fds[0], fds[1]);

    // Redirect stdout to pipe
    let saved = unsafe { libc_dup(1) };
    unsafe { libc_dup2(write_fd, 1) };

    let func: Symbol<unsafe extern "C" fn(*const c_char, *const c_char)> =
        unsafe { lib.get(b"driver").unwrap() };
    let c1 = CString::new(s1).unwrap();
    let c2 = CString::new(s2).unwrap();
    unsafe { func(c1.as_ptr(), c2.as_ptr()) };

    // Flush and restore stdout
    unsafe { libc_fflush(std::ptr::null_mut()) };
    unsafe { libc_dup2(saved, 1) };
    unsafe { libc_close(saved) };
    unsafe { libc_close(write_fd) };

    let mut buf = String::new();
    let mut f = unsafe { std::fs::File::from_raw_fd(read_fd) };
    f.read_to_string(&mut buf).unwrap();
    buf
}

extern "C" {
    fn pipe(fds: *mut i32) -> i32;
    fn dup(fd: i32) -> i32;
    fn dup2(old: i32, new: i32) -> i32;
    fn close(fd: i32) -> i32;
    fn fflush(stream: *mut std::ffi::c_void) -> i32;
}
unsafe fn libc_pipe(fds: &mut [i32; 2]) -> i32 { unsafe { pipe(fds.as_mut_ptr()) } }
unsafe fn libc_dup(fd: i32) -> i32 { unsafe { dup(fd) } }
unsafe fn libc_dup2(old: i32, new: i32) -> i32 { unsafe { dup2(old, new) } }
unsafe fn libc_close(fd: i32) -> i32 { unsafe { close(fd) } }
unsafe fn libc_fflush(s: *mut std::ffi::c_void) -> i32 { unsafe { fflush(s) } }

fn rust_so_path() -> std::path::PathBuf {
    let mut p = std::env::current_exe().unwrap();
    p.pop(); // test binary
    p.pop(); // deps
    p.push("libdriver.so");
    p
}

fn c_so_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so")
}

#[test]
fn test_driver_matches() {
    let c_lib = unsafe { Library::new(c_so_path()).expect("load C .so") };
    let rust_lib = unsafe { Library::new(rust_so_path()).expect("load Rust .so") };

    let cases: &[(&[u8], &[u8])] = &[
        (b"hello", b"world"),
        (b"abcdef", b"dc"),
        (b"", b"abc"),
        (b"abc", b""),
        (b"", b""),
        (b"aaaa", b"b"),
        (b"abcdef", b"a"),
        (b"abcdef", b"f"),
        (b"test string", b" "),
        (b"no match here", b"xyz"),
    ];

    for (s1, s2) in cases {
        let c_out = capture_driver(&c_lib, s1, s2);
        let r_out = capture_driver(&rust_lib, s1, s2);
        assert_eq!(
            c_out, r_out,
            "mismatch for s1={:?} s2={:?}: C={:?} Rust={:?}",
            std::str::from_utf8(s1).unwrap_or("<bin>"),
            std::str::from_utf8(s2).unwrap_or("<bin>"),
            c_out, r_out
        );
    }
}
