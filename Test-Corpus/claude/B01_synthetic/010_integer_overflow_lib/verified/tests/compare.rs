use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::io::{Seek, SeekFrom};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::Mutex;

// Serialize tests because they manipulate global stdout fd.
static GLOBAL_LOCK: Mutex<()> = Mutex::new(());

fn c_lib_path() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let candidates = [
        "target/debug/libdriver.so",
        "target/release/libdriver.so",
    ];
    for c in candidates.iter() {
        let p = PathBuf::from(manifest_dir).join(c);
        if p.exists() {
            return p;
        }
    }
    panic!("Rust libdriver.so not found; build with `cargo build` first");
}

/// Run `f` with stdout redirected to a temp file. Returns the captured bytes.
/// The closure runs with the global stdout fd swapped to a tempfile.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    unsafe {
        // Flush any pending stdout from before the redirect.
        libc::fflush(std::ptr::null_mut());

        let tmp = tempfile::tempfile().expect("tempfile");
        let tmp_fd = tmp.as_raw_fd();

        let saved_stdout = libc::dup(1);
        if saved_stdout < 0 {
            panic!("dup failed");
        }
        // Redirect fd 1 to the temp file.
        if libc::dup2(tmp_fd, 1) < 0 {
            libc::close(saved_stdout);
            panic!("dup2 failed");
        }

        // Run the user's function.
        f();

        // Flush all C stdio streams (in particular, the C printf going to stdout fd 1).
        libc::fflush(std::ptr::null_mut());

        // Restore stdout.
        if libc::dup2(saved_stdout, 1) < 0 {
            libc::close(saved_stdout);
            panic!("dup2 restore failed");
        }
        libc::close(saved_stdout);

        // Read back what was written.
        let mut tmp = tmp;
        tmp.seek(SeekFrom::Start(0)).expect("seek");
        let mut buf = Vec::new();
        let _ = std::io::Read::read_to_end(&mut tmp, &mut buf);
        buf
    }
}

#[test]
fn driver_outputs_match_for_all_chars() {
    let _g = GLOBAL_LOCK.lock().unwrap();

    let c_lib = unsafe { Library::new(c_lib_path()) }.expect("load C lib");
    let r_lib = unsafe { Library::new(rust_lib_path()) }.expect("load Rust lib");

    let c_driver: Symbol<unsafe extern "C" fn(c_char)> =
        unsafe { c_lib.get(b"driver\0") }.expect("driver in C");
    let r_driver: Symbol<unsafe extern "C" fn(c_char)> =
        unsafe { r_lib.get(b"driver\0") }.expect("driver in Rust");

    for i in -128i32..=127 {
        let ch = i as c_char;
        let c_out = capture_stdout(|| unsafe { c_driver(ch) });
        let r_out = capture_stdout(|| unsafe { r_driver(ch) });
        assert_eq!(
            c_out, r_out,
            "driver mismatch for input {}: C={:?} Rust={:?}",
            i,
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
    }
}

#[test]
fn print_hex_char_line_outputs_match_for_all_chars() {
    let _g = GLOBAL_LOCK.lock().unwrap();

    let c_lib = unsafe { Library::new(c_lib_path()) }.expect("load C lib");
    let r_lib = unsafe { Library::new(rust_lib_path()) }.expect("load Rust lib");

    let c_fn: Symbol<unsafe extern "C" fn(c_char)> =
        unsafe { c_lib.get(b"printHexCharLine\0") }.expect("printHexCharLine in C");
    let r_fn: Symbol<unsafe extern "C" fn(c_char)> =
        unsafe { r_lib.get(b"printHexCharLine\0") }.expect("printHexCharLine in Rust");

    for i in -128i32..=127 {
        let ch = i as c_char;
        let c_out = capture_stdout(|| unsafe { c_fn(ch) });
        let r_out = capture_stdout(|| unsafe { r_fn(ch) });
        assert_eq!(
            c_out, r_out,
            "printHexCharLine mismatch for input {}: C={:?} Rust={:?}",
            i,
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
    }
}
