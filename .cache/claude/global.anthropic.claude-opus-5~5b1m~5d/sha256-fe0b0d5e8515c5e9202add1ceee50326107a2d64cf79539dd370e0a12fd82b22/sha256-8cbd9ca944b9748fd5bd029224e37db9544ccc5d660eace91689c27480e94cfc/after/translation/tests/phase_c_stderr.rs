//! Phase C, the rows whose expected result is a message on **stderr**
//! (`ERRORS.md` rows 9-12 and 35).
//!
//! These live in their own test binary on purpose: fd 2 is process-global, so a
//! test that redirects it cannot tolerate an unrelated test writing to stderr at
//! the same time (`GetAlertData` calls `perror` on several rejection paths).
//! Cargo runs test binaries one at a time, and no test in this file emits
//! anything to stderr other than the call under test, so the captures are clean.
//! Within the file, `capture_stderr` additionally serialises on a mutex.

mod common;
use common::*;

use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};
use std::path::PathBuf;

fn dir() -> PathBuf {
    tmp_root()
}


#[test]
fn err_09_11_fseek_error_unseekable() {
    let b = both();
    let d = dir();
    unsafe {
        let t = tm::new(3, 3, 106);
        let mut qc = file_queue::zeroed();
        let mut qr = file_queue::zeroed();
        qc.fp = pipe_stream(b"** Alert 1: mail - g,\n");
        qr.fp = pipe_stream(b"** Alert 1: mail - g,\n");

        let mut rc = 0;
        let ec = capture_stderr(&d.join("e09_c.txt"), || {
            rc = (b.c.Init_FileQueue)(&mut qc, &t, CRALERT_FP_SET);
        });
        let mut rr = 0;
        let er = capture_stderr(&d.join("e09_r.txt"), || {
            rr = (b.rs.Init_FileQueue)(&mut qr, &t, CRALERT_FP_SET);
        });

        assert_eq!(rc, -1, "unseekable stream must make Init_FileQueue fail");
        assert_eq!(rc, rr, "Init_FileQueue return differs");
        assert!(qc.fp.is_null(), "C must fclose and NULL out fp");
        assert!(qr.fp.is_null(), "Rust must fclose and NULL out fp");
        assert_eq!(qsnap(&qc, true), qsnap(&qr, true));
        assert_eq!(
            ec, er,
            "merror(FSEEK_ERROR) output differs:\nC   ={}\nRust={}",
            String::from_utf8_lossy(&ec),
            String::from_utf8_lossy(&er)
        );
        assert!(
            ec.starts_with(b"(1116): Could not set position in file '<stdin>'"),
            "unexpected C message: {}",
            String::from_utf8_lossy(&ec)
        );
    }
}


/* ===================== rows 10, 11: fstat failure ===================== */

#[test]
fn err_10_11_fstat_error_fmemopen() {
    let b = both();
    let d = dir();
    unsafe {
        let t = tm::new(3, 3, 106);
        let mut bufc = *b"** Alert 1: mail - g,\n";
        let mut bufr = bufc;
        let mut qc = file_queue::zeroed();
        let mut qr = file_queue::zeroed();
        // fmemopen streams have no file descriptor -> fileno() == -1 -> fstat EBADF
        qc.fp = fmemopen(
            bufc.as_mut_ptr() as *mut c_void,
            bufc.len(),
            b"r\0".as_ptr() as *const c_char,
        );
        qr.fp = fmemopen(
            bufr.as_mut_ptr() as *mut c_void,
            bufr.len(),
            b"r\0".as_ptr() as *const c_char,
        );
        assert!(!qc.fp.is_null() && !qr.fp.is_null());

        let flags = CRALERT_FP_SET | CRALERT_READ_ALL; // skip the fseek, reach the fstat
        let mut rc = 0;
        let ec = capture_stderr(&d.join("e10_c.txt"), || {
            rc = (b.c.Init_FileQueue)(&mut qc, &t, flags);
        });
        let mut rr = 0;
        let er = capture_stderr(&d.join("e10_r.txt"), || {
            rr = (b.rs.Init_FileQueue)(&mut qr, &t, flags);
        });

        assert_eq!(rc, -1, "fstat failure must make Init_FileQueue fail");
        assert_eq!(rc, rr);
        assert!(qc.fp.is_null() && qr.fp.is_null());
        assert_eq!(qsnap(&qc, true), qsnap(&qr, true));
        assert_eq!(
            ec, er,
            "merror(FSTAT_ERROR) output differs:\nC   ={}\nRust={}",
            String::from_utf8_lossy(&ec),
            String::from_utf8_lossy(&er)
        );
        assert!(
            ec.starts_with(b"(1118): Could not retrieve information of file '<stdin>'"),
            "unexpected C message: {}",
            String::from_utf8_lossy(&ec)
        );
    }
}


/* ===================== row 12: driver's Init failure branch ===================== */

#[test]
fn err_12_driver_init_never_fails() {
    let w = workdir("e12");
    let b = both();
    let d = dir();
    unsafe {
        // `driver` memsets its file_queue, so fp is always NULL on entry and
        // Handle_Queue can never reach its two `return -1` branches.
        // Assert C and Rust agree that the message is never printed, for both a
        // present and an absent alerts.log and across the whole flag space.
        for present in [true, false] {
            if present {
                w.write("alerts.log", b"");
                w.write("<stdin>", b"");
            } else {
                w.remove("alerts.log");
                w.write("<stdin>", b"");
            }
            for f in 0..32 {
                if !present && f & CRALERT_FP_SET == 0 {
                    continue; // would cost a 5 s file_sleep
                }
                let ec = capture_stderr(&d.join("e12_c.txt"), || {
                    let p = (b.c.driver)(1, 0, 100, 0, f);
                    if !p.is_null() {
                        (b.c.FreeAlertData)(p);
                    }
                });
                let er = capture_stderr(&d.join("e12_r.txt"), || {
                    let p = (b.rs.driver)(1, 0, 100, 0, f);
                    if !p.is_null() {
                        (b.rs.FreeAlertData)(p);
                    }
                });
                assert_eq!(ec, er, "driver stderr differs for flags={f:#x}");
                assert!(
                    !ec.windows(9).any(|w| w == b"File queu"),
                    "unexpectedly reachable: {}",
                    String::from_utf8_lossy(&ec)
                );
            }
        }
    }
}


/* ===================== row 35: merror truncation ===================== */

#[test]
fn err_35_merror_truncation() {
    let b = both();
    let d = dir();
    unsafe {
        // NOTE: `merror` is a plain varargs forwarder, so a template demanding
        // MORE conversions than the three arguments it is given reads past the
        // argument list -- undefined in C and not a testable behaviour. Only
        // templates consuming at most (const char*, int, const char*) are used.
        let templates: [&[u8]; 6] = [
            b"(1118): Could not retrieve information of file '%s' due to [(%d)-(%s)].\0",
            b"(1116): Could not set position in file '%s' due to [(%d)-(%s)].\0",
            b"no conversions\0",
            b"%s\0",
            b"%s|%d\0",
            b"[%s][%d][%s]\0",
        ];
        for (i, tpl) in templates.iter().enumerate() {
            for &n in &[0usize, 1, 100, 180, 200, 254, 255, 256, 257, 1000, 5000] {
                let name = vec![b'N'; n];
                let msg = vec![b'M'; n];
                let cname = CString::new(name).unwrap();
                let cmsg = CString::new(msg).unwrap();
                for &err in &[0i32, -1, i32::MIN, i32::MAX, 1118] {
                    let oc = capture_stderr(&d.join("e35_c.txt"), || {
                        (b.c.merror)(
                            tpl.as_ptr() as *const c_char,
                            cname.as_ptr(),
                            err,
                            cmsg.as_ptr(),
                        )
                    });
                    let or = capture_stderr(&d.join("e35_r.txt"), || {
                        (b.rs.merror)(
                            tpl.as_ptr() as *const c_char,
                            cname.as_ptr(),
                            err,
                            cmsg.as_ptr(),
                        )
                    });
                    assert_eq!(
                        oc, or,
                        "merror differs (tpl#{i}, n={n}, err={err})\nC   ={}\nRust={}",
                        String::from_utf8_lossy(&oc),
                        String::from_utf8_lossy(&or)
                    );
                    assert!(oc.len() <= 256, "buffer bound violated: {}", oc.len());
                }
            }
        }
    }
}
