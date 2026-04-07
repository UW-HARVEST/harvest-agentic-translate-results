use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void, CString};
use std::fs;
use std::io::Read;
use std::os::unix::io::FromRawFd;
use std::path::PathBuf;

extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn fclose(stream: *mut c_void) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn pipe(pipefd: *mut c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/libdriver.so");
    if p.exists() { p } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/libdriver.so")
    }
}

fn capture_outputs<F: FnOnce() -> c_int>(f: F) -> (c_int, Vec<u8>, Vec<u8>) {
    unsafe {
        fflush(std::ptr::null_mut());
        let orig_out = dup(1);
        let orig_err = dup(2);
        let mut out_pipe = [0i32; 2];
        let mut err_pipe = [0i32; 2];
        pipe(out_pipe.as_mut_ptr());
        pipe(err_pipe.as_mut_ptr());
        dup2(out_pipe[1], 1);
        dup2(err_pipe[1], 2);
        close(out_pipe[1]);
        close(err_pipe[1]);

        let ret = f();

        fflush(std::ptr::null_mut());
        dup2(orig_out, 1);
        dup2(orig_err, 2);
        close(orig_out);
        close(orig_err);

        let mut stdout_buf = Vec::new();
        let mut stderr_buf = Vec::new();
        std::fs::File::from_raw_fd(out_pipe[0]).read_to_end(&mut stdout_buf).unwrap();
        std::fs::File::from_raw_fd(err_pipe[0]).read_to_end(&mut stderr_buf).unwrap();
        (ret, stdout_buf, stderr_buf)
    }
}

#[test]
fn test_forward_goto_example() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    type Fn = unsafe extern "C" fn(c_int) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"forward_goto_example").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"forward_goto_example").unwrap() };

    for x in [-100, -1, 0, 1, 5, 100] {
        let (cr, co, ce) = capture_outputs(|| unsafe { c_fn(x) });
        let (rr, ro, re) = capture_outputs(|| unsafe { r_fn(x) });
        assert_eq!(cr, rr, "forward_goto_example({x}): return");
        assert_eq!(co, ro, "forward_goto_example({x}): stdout\nC:  {:?}\nRs: {:?}", String::from_utf8_lossy(&co), String::from_utf8_lossy(&ro));
        assert_eq!(ce, re, "forward_goto_example({x}): stderr");
    }
}

#[test]
fn test_open_with_cleanup() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    type Fn = unsafe extern "C" fn(*const c_char) -> *mut c_void;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"open_with_cleanup").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"open_with_cleanup").unwrap() };

    // Nonexistent file
    let bad = CString::new("/tmp/_no_such_file_goto_test_").unwrap();
    let (cr, co, ce) = capture_outputs(|| { let p = unsafe { c_fn(bad.as_ptr()) }; if p.is_null() { 0 } else { unsafe { fclose(p); } 1 } });
    let (rr, ro, re) = capture_outputs(|| { let p = unsafe { r_fn(bad.as_ptr()) }; if p.is_null() { 0 } else { unsafe { fclose(p); } 1 } });
    assert_eq!(cr, rr, "open_with_cleanup(bad): return");
    assert_eq!(co, ro, "open_with_cleanup(bad): stdout");
    assert_eq!(ce, re, "open_with_cleanup(bad): stderr");

    // Real file
    let tmp = "/tmp/_goto_test_file_.txt";
    fs::write(tmp, "hello\nworld\n").unwrap();
    let good = CString::new(tmp).unwrap();
    let (cr, co, ce) = capture_outputs(|| { let p = unsafe { c_fn(good.as_ptr()) }; if p.is_null() { 0 } else { unsafe { fclose(p); } 1 } });
    let (rr, ro, re) = capture_outputs(|| { let p = unsafe { r_fn(good.as_ptr()) }; if p.is_null() { 0 } else { unsafe { fclose(p); } 1 } });
    assert_eq!(cr, rr, "open_with_cleanup(good): return");
    assert_eq!(co, ro, "open_with_cleanup(good): stdout");
    assert_eq!(ce, re, "open_with_cleanup(good): stderr");
    fs::remove_file(tmp).ok();
}

#[test]
fn test_driver() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    type Fn = unsafe extern "C" fn(c_int, *const c_char) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"driver").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"driver").unwrap() };

    // Negative num -> return -1
    let dummy = CString::new("/tmp/_dummy_").unwrap();
    let (cr, co, ce) = capture_outputs(|| unsafe { c_fn(-5, dummy.as_ptr()) });
    let (rr, ro, re) = capture_outputs(|| unsafe { r_fn(-5, dummy.as_ptr()) });
    assert_eq!(cr, rr, "driver(-5): return");
    assert_eq!(co, ro, "driver(-5): stdout");
    assert_eq!(ce, re, "driver(-5): stderr");

    // Positive num, bad file -> return -2
    let bad = CString::new("/tmp/_no_such_driver_").unwrap();
    let (cr, co, ce) = capture_outputs(|| unsafe { c_fn(3, bad.as_ptr()) });
    let (rr, ro, re) = capture_outputs(|| unsafe { r_fn(3, bad.as_ptr()) });
    assert_eq!(cr, rr, "driver(3,bad): return");
    assert_eq!(co, ro, "driver(3,bad): stdout");
    assert_eq!(ce, re, "driver(3,bad): stderr");

    // Positive num, good file -> return 0
    let tmp = "/tmp/_goto_driver_test_.txt";
    fs::write(tmp, "test content\n").unwrap();
    let good = CString::new(tmp).unwrap();
    let (cr, co, ce) = capture_outputs(|| unsafe { c_fn(7, good.as_ptr()) });
    let (rr, ro, re) = capture_outputs(|| unsafe { r_fn(7, good.as_ptr()) });
    assert_eq!(cr, rr, "driver(7,good): return");
    assert_eq!(co, ro, "driver(7,good): stdout");
    assert_eq!(ce, re, "driver(7,good): stderr");
    fs::remove_file(tmp).ok();
}
