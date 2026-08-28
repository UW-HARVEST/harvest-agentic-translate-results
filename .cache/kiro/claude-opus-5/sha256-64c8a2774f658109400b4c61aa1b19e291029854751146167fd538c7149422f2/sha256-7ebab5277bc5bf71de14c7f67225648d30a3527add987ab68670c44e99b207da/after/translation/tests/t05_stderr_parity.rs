//! The `w_regexec` failure path writes to stderr. Comparing those bytes needs
//! exclusive control of file descriptor 2, so this lives in its own test binary
//! (cargo runs test binaries one at a time) with a single `#[test]` inside it.

mod common;

use common::*;

use std::ffi::{c_char, c_int, c_void};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::fd::FromRawFd;

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn unlink(path: *const c_char) -> c_int;
}

const O_RDWR: c_int = 2;
const O_CREAT: c_int = 64;
const O_TRUNC: c_int = 512;

/// Capture everything written to file descriptor 2 while `f` runs.
fn capture_stderr(tag: &str, f: impl FnOnce()) -> Vec<u8> {
    let path = std::env::temp_dir().join(format!("driver_stderr_{}_{tag}.txt", std::process::id()));
    let cpath = std::ffi::CString::new(path.to_str().unwrap()).unwrap();

    unsafe {
        let tmp_fd = open(cpath.as_ptr(), O_RDWR | O_CREAT | O_TRUNC, 0o600 as c_int);
        assert!(tmp_fd >= 0, "could not open a temp file for stderr capture");
        let saved = dup(2);
        assert!(saved >= 0);
        assert!(dup2(tmp_fd, 2) >= 0);

        f();

        fflush(std::ptr::null_mut()); // flush every C stream
        let _ = std::io::stderr().flush();

        assert!(dup2(saved, 2) >= 0);
        close(saved);

        let mut file = std::fs::File::from_raw_fd(tmp_fd);
        file.seek(SeekFrom::Start(0)).unwrap();
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).unwrap();
        drop(file);
        unlink(cpath.as_ptr());
        buf
    }
}

#[test]
fn w_regexec_stderr_is_byte_identical() {
    let (c, rust) = load_both();

    // Patterns glibc's regcomp actually rejects. (Note `)` and `a**+` are
    // *accepted* by glibc as extended regexes, so they are not listed here.)
    let invalid: Vec<&[u8]> = vec![
        b"(",
        b"[",
        b"[z-a]",
        b"*",
        b"a{2,1}",
        b"\\",
        b"(|",
        // A pattern with bytes that are not valid UTF-8: the C uses %s, so the
        // Rust must copy raw bytes rather than go through a str conversion.
        b"(\xff",
        b"[\x80\x81",
    ];

    for (i, pat) in invalid.iter().enumerate() {
        let out_c = capture_stderr(&format!("c{i}"), || {
            let _ = c.w_regexec(Some(pat), Some(b"subject"), 2, 4);
        });
        let out_rust = capture_stderr(&format!("r{i}"), || {
            let _ = rust.w_regexec(Some(pat), Some(b"subject"), 2, 4);
        });
        assert!(
            !out_c.is_empty(),
            "expected the C to report pattern {:?} as invalid",
            String::from_utf8_lossy(pat)
        );
        assert_eq!(
            out_c,
            out_rust,
            "stderr differs for invalid pattern {:?}\nC    = {:?}\nRust = {:?}",
            String::from_utf8_lossy(pat),
            String::from_utf8_lossy(&out_c),
            String::from_utf8_lossy(&out_rust),
        );
    }

    // Nothing is printed when the pattern compiles, matched or not.
    for (i, (pat, subj)) in [
        (&b"^([0-9]+)"[..], &b"12"[..]),
        (&b"^([0-9]+)"[..], &b"ab"[..]),
    ]
    .iter()
    .enumerate()
    {
        let out_c = capture_stderr(&format!("ok_c{i}"), || {
            let _ = c.w_regexec(Some(pat), Some(subj), 2, 4);
        });
        let out_rust = capture_stderr(&format!("ok_r{i}"), || {
            let _ = rust.w_regexec(Some(pat), Some(subj), 2, 4);
        });
        assert!(out_c.is_empty(), "C printed unexpectedly: {out_c:?}");
        assert_eq!(out_c, out_rust);
    }

    // The NULL early-out prints nothing either.
    let out_c = capture_stderr("null_c", || {
        let _ = c.w_regexec(None, None, 2, 4);
    });
    let out_rust = capture_stderr("null_r", || {
        let _ = rust.w_regexec(None, None, 2, 4);
    });
    assert!(out_c.is_empty());
    assert_eq!(out_c, out_rust);
}
