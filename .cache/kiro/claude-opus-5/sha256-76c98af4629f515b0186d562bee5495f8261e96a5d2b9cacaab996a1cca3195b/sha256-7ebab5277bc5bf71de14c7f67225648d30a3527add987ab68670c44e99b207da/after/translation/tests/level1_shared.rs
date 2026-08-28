//! Level 1: the allocator helpers from `include/shared.h` and `merror` from
//! `src/file-queue.c`. Both are ordinary exported symbols of the C library.

mod common;

use common::*;
use std::io::{Read, Seek, SeekFrom};
use std::os::raw::{c_char, c_int, c_void};

#[test]
fn os_calloc_matches() {
    let p = pair();
    for (num, size) in [(1usize, 1usize), (1, 96), (7, 13), (0, 0), (0, 16), (1024, 4)] {
        unsafe {
            let a = (p.c.os_calloc)(num, size);
            let b = (p.rs.os_calloc)(num, size);
            assert!(!a.is_null(), "C os_calloc({num},{size}) returned NULL");
            assert!(!b.is_null(), "Rust os_calloc({num},{size}) returned NULL");

            let n = num * size;
            let sa = std::slice::from_raw_parts(a as *const u8, n);
            let sb = std::slice::from_raw_parts(b as *const u8, n);
            assert_eq!(sa, sb, "os_calloc({num},{size}) contents differ");
            assert!(
                sa.iter().all(|b| *b == 0),
                "os_calloc({num},{size}) did not zero the block"
            );

            libc::free(a);
            libc::free(b);
        }
    }
}

#[test]
fn os_realloc_matches() {
    let p = pair();
    unsafe {
        // NULL pointer behaves like malloc.
        let a = (p.c.os_realloc)(std::ptr::null_mut(), 32);
        let b = (p.rs.os_realloc)(std::ptr::null_mut(), 32);
        assert!(!a.is_null() && !b.is_null());

        // Fill and grow: contents must be preserved identically.
        let seed: Vec<u8> = (0..32u8).collect();
        std::ptr::copy_nonoverlapping(seed.as_ptr(), a as *mut u8, 32);
        std::ptr::copy_nonoverlapping(seed.as_ptr(), b as *mut u8, 32);

        let a = (p.c.os_realloc)(a, 4096);
        let b = (p.rs.os_realloc)(b, 4096);
        assert!(!a.is_null() && !b.is_null());
        assert_eq!(
            std::slice::from_raw_parts(a as *const u8, 32),
            std::slice::from_raw_parts(b as *const u8, 32),
            "os_realloc did not preserve contents identically"
        );

        // Shrink.
        let a = (p.c.os_realloc)(a, 8);
        let b = (p.rs.os_realloc)(b, 8);
        assert_eq!(
            std::slice::from_raw_parts(a as *const u8, 8),
            std::slice::from_raw_parts(b as *const u8, 8),
        );

        libc::free(a);
        libc::free(b);
    }
}

#[test]
fn os_strdup_matches() {
    let p = pair();
    for s in [
        &b""[..],
        &b"a"[..],
        &b"alerts.log"[..],
        &b"** Alert 1234.5678: mail - syscheck"[..],
        &[0xffu8, 0xfe, 0x41, 0x42][..],
        &vec![b'x'; 4096][..],
    ] {
        let cs = cstring(s);
        unsafe {
            let a = (p.c.os_strdup)(cs.as_ptr());
            let b = (p.rs.os_strdup)(cs.as_ptr());
            assert!(!a.is_null() && !b.is_null());
            assert_ne!(a, cs.as_ptr() as *mut c_char, "os_strdup must copy");
            let ba = std::ffi::CStr::from_ptr(a).to_bytes().to_vec();
            let bb = std::ffi::CStr::from_ptr(b).to_bytes().to_vec();
            assert_eq!(ba, bb);
            assert_eq!(ba.as_slice(), s);
            libc::free(a as *mut c_void);
            libc::free(b as *mut c_void);
        }
    }
}

/// Captures whatever the given closure writes to file descriptor 2.
fn capture_stderr<F: FnOnce()>(f: F) -> Vec<u8> {
    unsafe {
        let mut tmp = std::fs::File::from(
            std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(std::env::temp_dir().join(format!(
                    "c2rust-stderr-{}-{:?}",
                    std::process::id(),
                    std::thread::current().id()
                )))
                .expect("open capture file"),
        );
        use std::os::fd::AsRawFd;
        let cap_fd = tmp.as_raw_fd();

        libc::fflush(std::ptr::null_mut());
        let saved = libc::dup(2);
        assert!(saved >= 0);
        assert!(libc::dup2(cap_fd, 2) >= 0);

        f();

        // `stderr` is fully buffered when redirected to a file.
        libc::fflush(std::ptr::null_mut());
        libc::dup2(saved, 2);
        libc::close(saved);

        tmp.seek(SeekFrom::Start(0)).expect("seek");
        let mut out = Vec::new();
        tmp.read_to_end(&mut out).expect("read");
        out
    }
}

#[test]
fn merror_matches() {
    let p = pair();
    let _g = lock();

    let long_name = vec![b'N'; 400];
    let cases: Vec<(&[u8], Vec<u8>, c_int, Vec<u8>)> = vec![
        (
            b"(1118): Could not retrieve information of file '%s' due to [(%d)-(%s)].",
            b"alerts.log".to_vec(),
            2,
            b"No such file or directory".to_vec(),
        ),
        (
            b"(1116): Could not set position in file '%s' due to [(%d)-(%s)].",
            b"/tmp/x".to_vec(),
            0,
            b"Success".to_vec(),
        ),
        // Truncation at the 256 byte internal buffer.
        (
            b"(1118): Could not retrieve information of file '%s' due to [(%d)-(%s)].",
            long_name,
            -1,
            b"custom message".to_vec(),
        ),
        // Empty pieces.
        (b"%s%d%s", b"".to_vec(), 0, b"".to_vec()),
    ];

    for (tmpl, name, err, msg) in cases {
        let t = cstring(tmpl);
        let n = cstring(&name);
        let m = cstring(&msg);

        let out_c = capture_stderr(|| unsafe {
            (p.c.merror)(t.as_ptr(), n.as_ptr(), err, m.as_ptr());
        });
        let out_rs = capture_stderr(|| unsafe {
            (p.rs.merror)(t.as_ptr(), n.as_ptr(), err, m.as_ptr());
        });

        assert_eq!(
            String::from_utf8_lossy(&out_c),
            String::from_utf8_lossy(&out_rs),
            "merror output differs for template {:?}",
            String::from_utf8_lossy(tmpl)
        );
        assert!(!out_c.is_empty(), "expected merror to write something");
    }
}
