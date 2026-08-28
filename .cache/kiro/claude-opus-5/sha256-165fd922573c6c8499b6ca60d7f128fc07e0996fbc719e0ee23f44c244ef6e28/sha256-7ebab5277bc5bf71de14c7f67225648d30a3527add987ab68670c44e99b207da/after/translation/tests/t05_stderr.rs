//! Compares the exact bytes each implementation writes to stderr.
//!
//! Both libraries write diagnostics to fd 2 through unbuffered streams, so
//! redirecting fd 2 to a file around each call captures them verbatim.

mod common;

use common::*;
use std::ffi::c_int;
use std::io::Read;
use std::os::fd::AsRawFd;

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

/// Captures everything written to fd 2 while `f` runs.
fn capture_stderr<F: FnOnce()>(f: F) -> Vec<u8> {
    let path = std::env::temp_dir().join(format!("driver_stderr_{}.txt", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let file = std::fs::File::create(&path).unwrap();
    let mut out = Vec::new();
    unsafe {
        let saved = dup(2);
        assert!(saved >= 0, "dup(2) failed");
        assert!(dup2(file.as_raw_fd(), 2) >= 0, "dup2 failed");
        f();
        assert!(dup2(saved, 2) >= 0, "restoring fd 2 failed");
        close(saved);
    }
    drop(file);
    std::fs::File::open(&path)
        .unwrap()
        .read_to_end(&mut out)
        .unwrap();
    let _ = std::fs::remove_file(&path);
    out
}

fn assert_same_stderr<F, G>(label: &str, c: F, r: G)
where
    F: FnOnce(),
    G: FnOnce(),
{
    let _g = fs_lock();
    let cbytes = capture_stderr(c);
    let rbytes = capture_stderr(r);
    assert_eq!(
        String::from_utf8_lossy(&cbytes),
        String::from_utf8_lossy(&rbytes),
        "stderr differs for {label}"
    );
    assert_eq!(cbytes, rbytes, "stderr bytes differ for {label}");
}

#[test]
fn allocate_matrix_perror_messages() {
    let p = pair();
    for &(w, h) in &[(-1i32, 3i32), (3, -1), (-1, -1), (-8, 2), (2, -8)] {
        assert_same_stderr(
            &format!("allocate_matrix({w},{h})"),
            || unsafe {
                let m = (p.c.allocate_matrix)(w, h);
                (p.c.free_matrix)(m);
            },
            || unsafe {
                let m = (p.rs.allocate_matrix)(w, h);
                (p.rs.free_matrix)(m);
            },
        );
    }
}

#[test]
fn init_from_string_messages() {
    let p = pair();
    let cases: &[(&str, c_int, c_int)] = &[
        ("1 2\n", 2, 2),
        ("", 1, 1),
        ("1\n2\n", 2, 2),
        ("1 2\n3\n", 2, 2),
        ("1 2 3\n4 5 6\n7 8\n", 3, 3),
        ("1 2\n3 4\n", 2, 9),
        ("1 2\n3 4\n", -1, 2),
        ("1 2\n3 4\n", 2, -1),
    ];
    for &(input, w, h) in cases {
        let s = cstr(input);
        assert_same_stderr(
            &format!("init({input:?},{w},{h})"),
            || unsafe {
                let m = (p.c.initialize_matrix_from_string)(s.as_ptr(), w, h);
                (p.c.free_matrix)(m);
            },
            || unsafe {
                let m = (p.rs.initialize_matrix_from_string)(s.as_ptr(), w, h);
                (p.rs.free_matrix)(m);
            },
        );
    }
}

#[test]
fn multiply_mismatch_message() {
    let p = pair();
    assert_same_stderr(
        "multiply mismatch",
        || unsafe {
            let a = make_matrix(&p.c, 2, 2, &[1, 2, 3, 4]);
            let b = make_matrix(&p.c, 3, 3, &[1, 2, 3, 4, 5, 6, 7, 8, 9]);
            let r = (p.c.multiply_matrices)(a, b);
            (p.c.free_matrix)(r);
            (p.c.free_matrix)(a);
            (p.c.free_matrix)(b);
        },
        || unsafe {
            let a = make_matrix(&p.rs, 2, 2, &[1, 2, 3, 4]);
            let b = make_matrix(&p.rs, 3, 3, &[1, 2, 3, 4, 5, 6, 7, 8, 9]);
            let r = (p.rs.multiply_matrices)(a, b);
            (p.rs.free_matrix)(r);
            (p.rs.free_matrix)(a);
            (p.rs.free_matrix)(b);
        },
    );
}

#[test]
fn matrix_to_string_null_message() {
    let p = pair();
    assert_same_stderr(
        "matrix_to_string(NULL)",
        || unsafe {
            let s = (p.c.matrix_to_string)(std::ptr::null_mut());
            assert!(s.is_null());
        },
        || unsafe {
            let s = (p.rs.matrix_to_string)(std::ptr::null_mut());
            assert!(s.is_null());
        },
    );
}

#[test]
fn write_to_file_messages() {
    let p = pair();

    // NULL content.
    let name = cstr("/tmp/driver_stderr_target.txt");
    assert_same_stderr(
        "write_to_file NULL content",
        || unsafe {
            (p.c.write_to_file)(name.as_ptr(), std::ptr::null());
        },
        || unsafe {
            (p.rs.write_to_file)(name.as_ptr(), std::ptr::null());
        },
    );

    // fopen failures: the message embeds the filename and strerror(errno).
    let payload = cstr("data");
    for bad in [
        "/nonexistent_dir_xyz/out.txt",
        "",
        "/",
        "/proc/version",
        "/root/forbidden.txt",
    ] {
        let n = cstr(bad);
        assert_same_stderr(
            &format!("write_to_file({bad:?})"),
            || unsafe {
                (p.c.write_to_file)(n.as_ptr(), payload.as_ptr());
            },
            || unsafe {
                (p.rs.write_to_file)(n.as_ptr(), payload.as_ptr());
            },
        );
    }
}

#[test]
fn driver_messages() {
    let p = pair();
    let root = std::env::temp_dir().join(format!("driver_stderr_cwd_{}", std::process::id()));
    let cdir = root.join("c");
    let rdir = root.join("r");
    std::fs::create_dir_all(&cdir).unwrap();
    std::fs::create_dir_all(&rdir).unwrap();

    let cases: &[(c_int, c_int, &str, c_int, c_int, &str)] = &[
        (2, 2, "1 2\n", 2, 2, "5 6\n7 8\n"),
        (2, 2, "1 2\n3 4\n", 2, 2, "5 6\n"),
        (2, 2, "1 2\n3 4\n", 3, 3, "1 2 3\n4 5 6\n7 8 9\n"),
        (-1, 2, "1 2\n3 4\n", 2, 2, "1 2\n3 4\n"),
        (2, 2, "1 2\n3 4\n", 2, -1, "1 2\n3 4\n"),
        (1, 1, "", 1, 1, "1\n"),
    ];

    for &(wa, ha, a, wb, hb, b) in cases {
        let sa = cstr(a);
        let sb = cstr(b);
        let prev = std::env::current_dir().unwrap();
        let cbytes = {
            std::env::set_current_dir(&cdir).unwrap();
            let out = capture_stderr(|| unsafe {
                (p.c.driver)(wa, ha, sa.as_ptr(), wb, hb, sb.as_ptr());
            });
            std::env::set_current_dir(&prev).unwrap();
            out
        };
        let rbytes = {
            std::env::set_current_dir(&rdir).unwrap();
            let out = capture_stderr(|| unsafe {
                (p.rs.driver)(wa, ha, sa.as_ptr(), wb, hb, sb.as_ptr());
            });
            std::env::set_current_dir(&prev).unwrap();
            out
        };
        assert_eq!(
            String::from_utf8_lossy(&cbytes),
            String::from_utf8_lossy(&rbytes),
            "driver stderr differs for ({wa},{ha},{a:?},{wb},{hb},{b:?})"
        );
    }
}
