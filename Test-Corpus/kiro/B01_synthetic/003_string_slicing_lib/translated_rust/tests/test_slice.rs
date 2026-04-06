use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, CString};
use std::io::Read;
use std::os::unix::io::FromRawFd;

type SliceFn = unsafe extern "C" fn(*mut c_char, *const c_int, *const c_int) -> c_int;

/// Capture stdout from a closure that calls printf-based functions.
fn capture_stdout<F: FnOnce() -> c_int>(f: F) -> (c_int, Vec<u8>) {
    unsafe {
        libc::fflush(std::ptr::null_mut()); // flush all streams

        let mut pipefd = [0i32; 2];
        assert_eq!(libc::pipe(pipefd.as_mut_ptr()), 0);

        let saved_stdout = libc::dup(1);
        assert!(saved_stdout >= 0);
        libc::dup2(pipefd[1], 1);
        libc::close(pipefd[1]);

        let ret = f();

        libc::fflush(std::ptr::null_mut());

        libc::dup2(saved_stdout, 1);
        libc::close(saved_stdout);

        let mut buf = Vec::new();
        let mut file = std::fs::File::from_raw_fd(pipefd[0]);
        file.read_to_end(&mut buf).unwrap();

        (ret, buf)
    }
}

fn c_lib_path() -> String {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    format!("{}/c_src/build/libString_Slice.so", manifest)
}

fn rust_lib_path() -> String {
    // cargo puts cdylib in target/debug/
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    format!("{}/target/debug/libString_Slice.so", manifest)
}

fn run_test(input: &str, start: Option<c_int>, stop: Option<c_int>) {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C .so") };
    let c_slice: Symbol<SliceFn> = unsafe { c_lib.get(b"slice").expect("C slice") };

    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust .so") };
    let r_slice: Symbol<SliceFn> = unsafe { r_lib.get(b"slice").expect("Rust slice") };

    let (c_ret, c_out) = {
        let mut s = CString::new(input).unwrap().into_bytes_with_nul();
        let st = start;
        let sp = stop;
        capture_stdout(|| unsafe {
            c_slice(
                s.as_mut_ptr() as *mut c_char,
                st.as_ref().map_or(std::ptr::null(), |v| v as *const c_int),
                sp.as_ref().map_or(std::ptr::null(), |v| v as *const c_int),
            )
        })
    };

    let (r_ret, r_out) = {
        let mut s = CString::new(input).unwrap().into_bytes_with_nul();
        let st = start;
        let sp = stop;
        capture_stdout(|| unsafe {
            r_slice(
                s.as_mut_ptr() as *mut c_char,
                st.as_ref().map_or(std::ptr::null(), |v| v as *const c_int),
                sp.as_ref().map_or(std::ptr::null(), |v| v as *const c_int),
            )
        })
    };

    assert_eq!(
        c_ret, r_ret,
        "Return mismatch for input={:?} start={:?} stop={:?}: C={} Rust={}",
        input, start, stop, c_ret, r_ret
    );
    assert_eq!(
        c_out, r_out,
        "Stdout mismatch for input={:?} start={:?} stop={:?}:\n  C:    {:?}\n  Rust: {:?}",
        input, start, stop,
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out)
    );
}

#[test] fn test_full_string_no_bounds()  { run_test("Hello, World!", None, None); }
#[test] fn test_with_start_only()        { run_test("Hello, World!", Some(7), None); }
#[test] fn test_with_stop_only()         { run_test("Hello, World!", None, Some(5)); }
#[test] fn test_with_start_and_stop()    { run_test("Hello, World!", Some(0), Some(5)); }
#[test] fn test_start_equals_stop()      { run_test("Hello", Some(2), Some(2)); }
#[test] fn test_start_after_stop()       { run_test("Hello", Some(3), Some(1)); }
#[test] fn test_start_off_end()          { run_test("Hi", Some(100), None); }
#[test] fn test_stop_off_end()           { run_test("Hi", None, Some(100)); }
#[test] fn test_empty_string_no_bounds() { run_test("", None, None); }
#[test] fn test_single_char()            { run_test("A", Some(0), Some(1)); }
#[test] fn test_middle_slice()           { run_test("abcdefgh", Some(2), Some(6)); }
