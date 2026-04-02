use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::io::Read;
use std::os::unix::io::FromRawFd;

fn capture_stdout<F: FnOnce()>(f: F) -> String {
    unsafe { libc::fflush(std::ptr::null_mut()); }
    let mut pipe_fds = [0 as libc::c_int; 2];
    unsafe { libc::pipe(pipe_fds.as_mut_ptr()); }
    let saved = unsafe { libc::dup(1) };
    unsafe { libc::dup2(pipe_fds[1], 1); }

    f();

    unsafe { libc::fflush(std::ptr::null_mut()); }
    unsafe { libc::dup2(saved, 1); }
    unsafe { libc::close(saved); }
    unsafe { libc::close(pipe_fds[1]); }

    let mut buf = String::new();
    let mut r = unsafe { std::fs::File::from_raw_fd(pipe_fds[0]) };
    r.read_to_string(&mut buf).unwrap();
    buf
}

fn c_lib_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> std::path::PathBuf {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.join("target/debug/libdriver.so")
}

type DriverFn = unsafe extern "C" fn(c_int, c_int, c_int);

fn call_with_lib(lib: &Library, x: c_int, y: c_int, z: c_int) -> String {
    capture_stdout(|| unsafe {
        let f: Symbol<DriverFn> = lib.get(b"driver").unwrap();
        f(x, y, z);
    })
}

struct TestCase {
    x: c_int,
    y: c_int,
    z: c_int,
    label: &'static str,
}

const CASES: &[TestCase] = &[
    TestCase { x: 0, y: 2, z: 3, label: "x!=1" },
    TestCase { x: 1, y: 5, z: 3, label: "y!=2" },
    TestCase { x: 1, y: 2, z: 0, label: "z!=3" },
    TestCase { x: 1, y: 2, z: 3, label: "all_ok" },
];

#[test]
fn test_driver_outputs_match() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C .so") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust .so") };

    for tc in CASES {
        let c_out = call_with_lib(&c_lib, tc.x, tc.y, tc.z);
        let r_out = call_with_lib(&r_lib, tc.x, tc.y, tc.z);
        assert_eq!(
            c_out, r_out,
            "Mismatch [{}] driver({},{},{})\nC:    {c_out:?}\nRust: {r_out:?}",
            tc.label, tc.x, tc.y, tc.z
        );
    }
}
