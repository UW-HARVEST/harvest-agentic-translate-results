// Integration tests: load both the C and Rust .so libraries via `libloading`
// and compare their stdout output byte-for-byte for every public function.

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, CString};
use std::fs::File;
use std::io::Read;
use std::os::unix::io::AsRawFd;

const C_SO: &str = "c_src/build/libdriver.so";
const RUST_SO: &str = "target/release/libdriver.so";

/// Capture everything written to stdout by `f`. Uses dup/dup2 against a
/// temporary file so this also works for C printf output.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    unsafe {
        // Flush any pending stdout buffering.
        libc::fflush(std::ptr::null_mut());

        // Save the original stdout fd.
        let saved = libc::dup(1);
        assert!(saved >= 0, "dup failed");

        // Open a tempfile and redirect stdout to it.
        let path = std::env::temp_dir().join(format!(
            "driver_capture_{}_{}.tmp",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let file = File::create(&path).expect("create tmp");
        let fd = file.as_raw_fd();
        assert!(libc::dup2(fd, 1) >= 0, "dup2 failed");

        f();

        // Flush, restore.
        libc::fflush(std::ptr::null_mut());
        assert!(libc::dup2(saved, 1) >= 0, "dup2 restore failed");
        libc::close(saved);
        drop(file);

        // Read the captured contents.
        let mut buf = Vec::new();
        File::open(&path)
            .expect("open tmp")
            .read_to_end(&mut buf)
            .expect("read tmp");
        let _ = std::fs::remove_file(&path);
        buf
    }
}

fn load_libs() -> (Library, Library) {
    unsafe {
        let c = Library::new(C_SO).expect("load C .so");
        let r = Library::new(RUST_SO).expect("load Rust .so");
        (c, r)
    }
}

#[test]
fn print_line_matches() {
    let (c, r) = load_libs();
    unsafe {
        let c_print: Symbol<unsafe extern "C" fn(*const c_char)> =
            c.get(b"printLine").unwrap();
        let r_print: Symbol<unsafe extern "C" fn(*const c_char)> =
            r.get(b"printLine").unwrap();

        for input in &["", "hello", "Calling good()...", "0123456789", "with spaces and !@#$%^"] {
            let s = CString::new(*input).unwrap();
            let c_out = capture_stdout(|| c_print(s.as_ptr()));
            let r_out = capture_stdout(|| r_print(s.as_ptr()));
            assert_eq!(c_out, r_out, "mismatch for input {:?}", input);
        }

        // NULL pointer: function must do nothing.
        let c_out = capture_stdout(|| c_print(std::ptr::null()));
        let r_out = capture_stdout(|| r_print(std::ptr::null()));
        assert_eq!(c_out, r_out);
        assert!(c_out.is_empty());
    }
}

#[test]
fn print_int_line_matches() {
    let (c, r) = load_libs();
    unsafe {
        let c_print: Symbol<unsafe extern "C" fn(c_int)> = c.get(b"printIntLine").unwrap();
        let r_print: Symbol<unsafe extern "C" fn(c_int)> = r.get(b"printIntLine").unwrap();

        for &v in &[0, 1, -1, 42, -42, i32::MAX, i32::MIN, 12345] {
            let c_out = capture_stdout(|| c_print(v));
            let r_out = capture_stdout(|| r_print(v));
            assert_eq!(c_out, r_out, "mismatch for input {}", v);
        }
    }
}

#[test]
fn good_matches() {
    let (c, r) = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn()> = c.get(b"good").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn()> = r.get(b"good").unwrap();
        let c_out = capture_stdout(|| c_fn());
        let r_out = capture_stdout(|| r_fn());
        assert_eq!(c_out, r_out);
    }
}

#[test]
fn bad_matches() {
    let (c, r) = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn()> = c.get(b"bad").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn()> = r.get(b"bad").unwrap();
        let c_out = capture_stdout(|| c_fn());
        let r_out = capture_stdout(|| r_fn());
        assert_eq!(c_out, r_out);
    }
}

#[test]
fn driver_matches() {
    let (c, r) = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn()> = c.get(b"driver").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn()> = r.get(b"driver").unwrap();
        let c_out = capture_stdout(|| c_fn());
        let r_out = capture_stdout(|| r_fn());
        assert_eq!(c_out, r_out);
    }
}

#[test]
fn exported_symbols_match() {
    // Sanity: every application-level C symbol must exist in the Rust .so.
    let (c, r) = load_libs();
    let needed = [b"printLine".as_slice(), b"printIntLine", b"good", b"bad", b"driver"];
    unsafe {
        for name in &needed {
            let _: Symbol<unsafe extern "C" fn()> = c
                .get(name)
                .unwrap_or_else(|_| panic!("C missing {:?}", std::str::from_utf8(name).unwrap()));
            let _: Symbol<unsafe extern "C" fn()> = r
                .get(name)
                .unwrap_or_else(|_| panic!("Rust missing {:?}", std::str::from_utf8(name).unwrap()));
        }
    }
}
