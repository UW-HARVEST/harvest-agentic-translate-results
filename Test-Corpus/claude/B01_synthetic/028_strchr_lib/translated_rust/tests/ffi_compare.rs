use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        manifest.join("target/debug/libdriver.so"),
        manifest.join("target/release/libdriver.so"),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!("Could not locate Rust .so; expected one of {:?}", candidates);
}

unsafe fn load_foo(
    lib: &Library,
) -> Symbol<'_, unsafe extern "C" fn(*const c_char, c_char) -> c_int> {
    lib.get(b"foo\0").expect("symbol foo missing")
}

unsafe fn load_driver(lib: &Library) -> Symbol<'_, unsafe extern "C" fn(*const c_char)> {
    lib.get(b"driver\0").expect("symbol driver missing")
}

fn run_foo_pair(input: &str, ch: c_char) -> (c_int, c_int) {
    let cinput = CString::new(input).unwrap();
    unsafe {
        let clib = Library::new(c_lib_path()).expect("load C lib");
        let rlib = Library::new(rust_lib_path()).expect("load Rust lib");
        let cf = load_foo(&clib);
        let rf = load_foo(&rlib);
        let cv = cf(cinput.as_ptr(), ch);
        let rv = rf(cinput.as_ptr(), ch);
        (cv, rv)
    }
}

// Capture stdout by redirecting FD 1 through a pipe. Caller must guarantee no
// other thread writes to stdout during the closure (we only call this from a
// single combined test function so the test harness can't interleave its own
// progress output).
unsafe fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::os::unix::io::FromRawFd;
    extern "C" {
        fn dup(fd: c_int) -> c_int;
        fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
        fn close(fd: c_int) -> c_int;
        fn pipe(fds: *mut c_int) -> c_int;
        fn fflush(stream: *mut std::ffi::c_void) -> c_int;
    }
    extern "C" {
        static stdout: *mut std::ffi::c_void;
    }

    fflush(stdout);
    let saved = dup(1);
    assert!(saved >= 0);

    let mut fds = [0_i32; 2];
    assert!(pipe(fds.as_mut_ptr()) == 0);
    let read_fd = fds[0];
    let write_fd = fds[1];

    assert!(dup2(write_fd, 1) >= 0);
    close(write_fd);

    f();
    fflush(stdout);

    assert!(dup2(saved, 1) >= 0);
    close(saved);

    let mut buf = Vec::new();
    use std::io::Read;
    let mut reader = std::fs::File::from_raw_fd(read_fd);
    reader.read_to_end(&mut buf).expect("read pipe");
    buf
}

fn run_driver_capture(input: &str, lib_path: PathBuf) -> Vec<u8> {
    let cinput = CString::new(input).unwrap();
    unsafe {
        let lib = Library::new(lib_path).expect("load lib");
        let f = load_driver(&lib);
        capture_stdout(|| {
            f(cinput.as_ptr());
        })
    }
}

#[test]
fn ffi_compare_all() {
    // ---- foo: basic inputs ----
    for (input, ch) in [
        ("hello world", b'o' as c_char),
        ("AAA bbb ccc", b'A' as c_char),
        ("xxxxx", b'x' as c_char),
        ("no matches here", b'Z' as c_char),
        ("", b'A' as c_char),
        ("AxAxAxAx", b'A' as c_char),
        ("AxAxAxAx", b'x' as c_char),
        ("The quick brown fox jumps over the lazy dog", b' ' as c_char),
        ("AaAaAa", b'a' as c_char),
    ] {
        let (c_val, r_val) = run_foo_pair(input, ch);
        assert_eq!(
            c_val, r_val,
            "foo mismatch for input={:?} ch={:?}: C={} Rust={}",
            input, ch as u8 as char, c_val, r_val
        );
    }

    // ---- foo: long input ----
    let big = "A".repeat(10_000) + "B" + &"x".repeat(5_000) + "A";
    let (c_a, r_a) = run_foo_pair(&big, b'A' as c_char);
    assert_eq!(c_a, r_a, "long-input foo for 'A' differs");
    assert_eq!(c_a, 10_001);
    let (c_x, r_x) = run_foo_pair(&big, b'x' as c_char);
    assert_eq!(c_x, r_x, "long-input foo for 'x' differs");
    assert_eq!(c_x, 5_000);

    // ---- driver: stdout output must match byte-for-byte ----
    for input in [
        "hello world",
        "AAAxxxAAAxxx",
        "no matches",
        "",
        "A",
        "x",
        "AxAxAxAx",
        "The quick brown fox jumps over the lazy A and x",
    ] {
        let c_out = run_driver_capture(input, c_lib_path());
        let r_out = run_driver_capture(input, rust_lib_path());
        assert_eq!(
            c_out, r_out,
            "driver output mismatch for input={:?}\nC:    {:?}\nRust: {:?}",
            input,
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
    }
}
