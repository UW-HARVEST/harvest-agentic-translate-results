//! Compares the `errno` state each implementation leaves behind.
//!
//! `write_to_file` returns `errno` directly, and `perror` renders it, so any
//! divergence in the errno side effects of the lower-level helpers (notably
//! `atoi`, which glibc implements via `strtol` and which sets `ERANGE`) would be
//! observable. Both libraries share this process's glibc, hence the same errno.

mod common;

use common::*;
use std::ffi::c_int;

unsafe extern "C" {
    fn __errno_location() -> *mut c_int;
}

fn get_errno() -> c_int {
    unsafe { *__errno_location() }
}

fn set_errno(v: c_int) {
    unsafe { *__errno_location() = v }
}

/// Runs `f` with errno pre-set to a marker and returns the resulting errno.
fn errno_after<F: FnOnce()>(f: F) -> c_int {
    set_errno(0);
    f();
    get_errno()
}

#[test]
fn errno_after_initialize_matrix_from_string() {
    let p = pair();
    let cases: &[(&str, c_int, c_int)] = &[
        ("1 2\n3 4\n", 2, 2),
        // In-range values: no errno change expected.
        ("2147483647 -2147483648\n", 2, 1),
        // Out of range for long: glibc strtol sets ERANGE.
        ("99999999999999999999\n", 1, 1),
        ("-99999999999999999999\n", 1, 1),
        ("9223372036854775808\n", 1, 1),
        ("9223372036854775807\n", 1, 1),
        ("-9223372036854775808\n", 1, 1),
        ("-9223372036854775809\n", 1, 1),
        ("1 99999999999999999999\n", 2, 1),
        ("abc\n", 1, 1),
        ("", 1, 1),
    ];
    for &(input, w, h) in cases {
        let s = cstr(input);
        let ce = errno_after(|| unsafe {
            let m = (p.c.initialize_matrix_from_string)(s.as_ptr(), w, h);
            (p.c.free_matrix)(m);
        });
        let re = errno_after(|| unsafe {
            let m = (p.rs.initialize_matrix_from_string)(s.as_ptr(), w, h);
            (p.rs.free_matrix)(m);
        });
        assert_eq!(
            ce, re,
            "errno differs after init({input:?},{w},{h}): c={ce}, rust={re}"
        );
    }
    set_errno(0);
}

#[test]
fn errno_after_write_to_file() {
    let p = pair();
    let _g = fs_lock();
    let d = std::env::temp_dir().join(format!("driver_errno_{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let ok_c = cstr(d.join("c.txt").to_str().unwrap());
    let ok_r = cstr(d.join("r.txt").to_str().unwrap());
    let payload = cstr("hello\n");

    let ce = errno_after(|| unsafe {
        (p.c.write_to_file)(ok_c.as_ptr(), payload.as_ptr());
    });
    let re = errno_after(|| unsafe {
        (p.rs.write_to_file)(ok_r.as_ptr(), payload.as_ptr());
    });
    assert_eq!(ce, re, "errno differs after a successful write");

    for bad in ["/nonexistent_dir_xyz/o.txt", "", "/"] {
        let n = cstr(bad);
        let ce = errno_after(|| unsafe {
            (p.c.write_to_file)(n.as_ptr(), payload.as_ptr());
        });
        let re = errno_after(|| unsafe {
            (p.rs.write_to_file)(n.as_ptr(), payload.as_ptr());
        });
        assert_eq!(ce, re, "errno differs after write_to_file({bad:?})");
    }
    set_errno(0);
}

#[test]
fn errno_after_driver() {
    let p = pair();
    let _g = fs_lock();
    let root = std::env::temp_dir().join(format!("driver_errno_d_{}", std::process::id()));
    let cdir = root.join("c");
    let rdir = root.join("r");
    std::fs::create_dir_all(&cdir).unwrap();
    std::fs::create_dir_all(&rdir).unwrap();
    let prev = std::env::current_dir().unwrap();

    let cases: &[(c_int, c_int, &str, c_int, c_int, &str)] = &[
        (2, 2, "1 2\n3 4\n", 2, 2, "5 6\n7 8\n"),
        // Overflowing tokens set ERANGE inside the parse.
        (1, 1, "99999999999999999999\n", 1, 1, "2\n"),
        (2, 1, "99999999999999999999 1\n", 1, 2, "2\n3\n"),
        (2, 2, "1 2\n", 2, 2, "5 6\n7 8\n"),
        (2, 2, "1 2\n3 4\n", 3, 3, "1 2 3\n4 5 6\n7 8 9\n"),
    ];

    for &(wa, ha, a, wb, hb, b) in cases {
        let sa = cstr(a);
        let sb = cstr(b);

        std::env::set_current_dir(&cdir).unwrap();
        let (crc, ce) = {
            set_errno(0);
            let rc = unsafe { (p.c.driver)(wa, ha, sa.as_ptr(), wb, hb, sb.as_ptr()) };
            (rc, get_errno())
        };
        std::env::set_current_dir(&rdir).unwrap();
        let (rrc, re) = {
            set_errno(0);
            let rc = unsafe { (p.rs.driver)(wa, ha, sa.as_ptr(), wb, hb, sb.as_ptr()) };
            (rc, get_errno())
        };
        std::env::set_current_dir(&prev).unwrap();

        assert_eq!(crc, rrc, "driver rc differs for ({wa},{ha},{a:?})");
        assert_eq!(
            ce, re,
            "errno differs after driver({wa},{ha},{a:?},{wb},{hb},{b:?}): c={ce}, rust={re}"
        );
    }
    set_errno(0);
}
