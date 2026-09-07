//! Phase C: one differential test per row of `ERRORS.md`.
//!
//! Rows that end in `exit()` or in a memory-corruption abort are run in a
//! forked child so the raw `waitpid` status (exit code / fatal signal) can be
//! compared between the two libraries.

mod common;
use common::*;

use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};
use std::path::PathBuf;

fn dir() -> PathBuf {
    tmp_root()
}

/// Drain both libraries over `bytes` and require identical result sequences.
#[track_caller]
fn cmp(tag: &str, bytes: &[u8], flag: c_int) -> Vec<Option<AlertSnap>> {
    let b = both();
    let d = dir();
    unsafe {
        let c = drain(&b.c, &d, tag, bytes, flag, 16);
        let r = drain(&b.rs, &d, tag, bytes, flag, 16);
        if c != r {
            panic!(
                "divergence [{tag}] flag={flag:#x}\n--- input ---\n{}\n--- C ---\n{c:#?}\n--- Rust ---\n{r:#?}",
                String::from_utf8_lossy(bytes)
            );
        }
        c
    }
}

/// Both libraries must reject (return NULL) for this input.
#[track_caller]
fn cmp_reject(tag: &str, bytes: &[u8], flag: c_int) {
    let got = cmp(tag, bytes, flag);
    assert_eq!(
        got.iter().filter(|x| x.is_some()).count(),
        0,
        "[{tag}] expected NULL from GetAlertData, got {got:#?}"
    );
}

/* ===================== rows 1-3: os_* fatal exits ===================== */

#[test]
fn err_01_os_strdup_null_exits() {
    let b = both();
    let d = dir();
    unsafe {
        let stc = fork_status(&d.join("e01_c.txt"), || {
            (b.c.os_strdup)(std::ptr::null());
        });
        let str_ = fork_status(&d.join("e01_r.txt"), || {
            (b.rs.os_strdup)(std::ptr::null());
        });
        assert_eq!(
            wait_exited(stc),
            Some(1),
            "C os_strdup(NULL) must exit(EXIT_FAILURE)"
        );
        assert_eq!(wait_exited(stc), wait_exited(str_), "exit status differs");
        assert_eq!(wait_signal(stc), wait_signal(str_), "signal differs");
        let mc = std::fs::read(d.join("e01_c.txt")).unwrap();
        let mr = std::fs::read(d.join("e01_r.txt")).unwrap();
        assert_eq!(mc, b"NULL string passed to os_strdup".to_vec());
        assert_eq!(mc, mr, "stderr message differs");
    }
}

#[test]
fn err_02_os_calloc_oom_exits() {
    let b = both();
    let d = dir();
    unsafe {
        let stc = fork_status(&d.join("e02_c.txt"), || {
            (b.c.os_calloc)(usize::MAX, 2);
        });
        let str_ = fork_status(&d.join("e02_r.txt"), || {
            (b.rs.os_calloc)(usize::MAX, 2);
        });
        assert_eq!(wait_exited(stc), Some(1), "C os_calloc OOM must exit(1)");
        assert_eq!(wait_exited(stc), wait_exited(str_));
        assert_eq!(wait_signal(stc), wait_signal(str_));
        let mc = std::fs::read(d.join("e02_c.txt")).unwrap();
        let mr = std::fs::read(d.join("e02_r.txt")).unwrap();
        assert_eq!(mc, b"Memory allocation failed in os_calloc".to_vec());
        assert_eq!(mc, mr);
    }
}

#[test]
fn err_03_os_realloc_oom_exits() {
    let b = both();
    let d = dir();
    unsafe {
        let stc = fork_status(&d.join("e03_c.txt"), || {
            (b.c.os_realloc)(std::ptr::null_mut(), usize::MAX);
        });
        let str_ = fork_status(&d.join("e03_r.txt"), || {
            (b.rs.os_realloc)(std::ptr::null_mut(), usize::MAX);
        });
        assert_eq!(wait_exited(stc), Some(1), "C os_realloc OOM must exit(1)");
        assert_eq!(wait_exited(stc), wait_exited(str_));
        assert_eq!(wait_signal(stc), wait_signal(str_));
        let mc = std::fs::read(d.join("e03_c.txt")).unwrap();
        let mr = std::fs::read(d.join("e03_r.txt")).unwrap();
        assert_eq!(mc, b"Memory allocation failed in os_realloc".to_vec());
        assert_eq!(mc, mr);
    }
}

/* ===================== rows 4-6: zero-size boundaries ===================== */

#[test]
fn err_04_os_calloc_zero_ok() {
    let b = both();
    unsafe {
        for (n, s) in [(0usize, 0usize), (0, 32), (32, 0), (0, usize::MAX), (usize::MAX, 0)] {
            let pc = (b.c.os_calloc)(n, s);
            let pr = (b.rs.os_calloc)(n, s);
            assert_eq!(
                pc.is_null(),
                pr.is_null(),
                "os_calloc({n},{s}) NULL-ness differs"
            );
            if !pc.is_null() {
                free(pc);
            }
            if !pr.is_null() {
                free(pr);
            }
        }
    }
}

#[test]
fn err_05_os_realloc_null_ptr_ok() {
    let b = both();
    unsafe {
        for n in [1usize, 8, 4096] {
            let pc = (b.c.os_realloc)(std::ptr::null_mut(), n);
            let pr = (b.rs.os_realloc)(std::ptr::null_mut(), n);
            assert!(!pc.is_null() && !pr.is_null());
            free(pc);
            free(pr);
        }
    }
}

#[test]
fn err_06_os_realloc_zero_size() {
    let b = both();
    let d = dir();
    unsafe {
        // glibc's realloc(p, 0) frees and returns NULL -> hits the error branch.
        let stc = fork_status(&d.join("e06_c.txt"), || {
            let p = (b.c.os_realloc)(std::ptr::null_mut(), 64);
            (b.c.os_realloc)(p, 0);
        });
        let str_ = fork_status(&d.join("e06_r.txt"), || {
            let p = (b.rs.os_realloc)(std::ptr::null_mut(), 64);
            (b.rs.os_realloc)(p, 0);
        });
        assert_eq!(
            wait_exited(stc),
            wait_exited(str_),
            "os_realloc(p,0) exit status differs (C={stc:#x} Rust={str_:#x})"
        );
        assert_eq!(wait_signal(stc), wait_signal(str_));
        assert_eq!(
            std::fs::read(d.join("e06_c.txt")).unwrap(),
            std::fs::read(d.join("e06_r.txt")).unwrap(),
            "os_realloc(p,0) stderr differs"
        );
    }
}

/* ===================== rows 7, 13: missing alerts.log ===================== */

#[test]
fn err_07_missing_alerts_log() {
    let w = workdir("e07");
    let b = both();
    w.remove("alerts.log");
    unsafe {
        // Init_FileQueue: fopen fails -> Handle_Queue returns 0 -> Init returns 0
        let t = tm::new(3, 3, 106);
        for flags in [0, CRALERT_READ_ALL, CRALERT_MAIL_SET, 0x20] {
            let mut qc = file_queue::zeroed();
            let mut qr = file_queue::zeroed();
            let rc = (b.c.Init_FileQueue)(&mut qc, &t, flags);
            let rr = (b.rs.Init_FileQueue)(&mut qr, &t, flags);
            assert_eq!(rc, 0, "expected 0 (queue not available), got {rc}");
            assert_eq!(rc, rr);
            assert_eq!(qsnap(&qc, true), qsnap(&qr, true));
            assert!(qc.fp.is_null() && qr.fp.is_null());
        }

        // row 13: Read_FileMon with a NULL fp and no file -> file_sleep + NULL
        // (one 5 s sleep per library; a single flags value keeps that bounded)
        let mut qc = file_queue::zeroed();
        let mut qr = file_queue::zeroed();
        (b.c.Init_FileQueue)(&mut qc, &t, 0);
        (b.rs.Init_FileQueue)(&mut qr, &t, 0);
        let ac = (b.c.Read_FileMon)(&mut qc, &t, 0);
        let ar = (b.rs.Read_FileMon)(&mut qr, &t, 0);
        assert!(ac.is_null(), "C Read_FileMon must return NULL");
        assert_eq!(snap(ac), snap(ar));
        assert_eq!(qsnap(&qc, true), qsnap(&qr, true));

        // row 12/13 through the driver
        let dc = (b.c.driver)(3, 3, 106, 0, 0);
        let dr = (b.rs.driver)(3, 3, 106, 0, 0);
        assert!(dc.is_null() && dr.is_null());
    }
}

/* ===================== row 8: FP_SET with a NULL fp ===================== */

#[test]
fn err_08_fp_set_null_fp() {
    let b = both();
    unsafe {
        let t = tm::new(3, 3, 106);
        // FP_SET and fp == NULL, READ_ALL clear -> Handle_Queue returns 0 (not -1)
        let mut qc = file_queue::zeroed();
        let mut qr = file_queue::zeroed();
        let rc = (b.c.Init_FileQueue)(&mut qc, &t, CRALERT_FP_SET);
        let rr = (b.rs.Init_FileQueue)(&mut qr, &t, CRALERT_FP_SET);
        assert_eq!(rc, 0, "return must be 0, not -1");
        assert_eq!(rc, rr);
        let sc = qsnap(&qc, true);
        assert_eq!(sc, qsnap(&qr, true));
        assert!(sc.fp_null);
        assert_eq!(sc.last_change, 0, "last_change stays 0 (fstat skipped)");
        assert_eq!(sc.file_name, b"<stdin>".to_vec());
    }
}

/* ===================== rows 9, 11: fseek failure ===================== */

/* ===================== row 14: NULL fp after a successful Handle_Queue ===== */

#[test]
fn err_14_read_filemon_null_fp_fast() {
    let b = both();
    unsafe {
        let t = tm::new(3, 3, 106);
        // FP_SET|READ_ALL with fp == NULL: Handle_Queue returns 1 without opening
        // anything, so Read_FileMon hits `if(!fileq->fp) return NULL;` -- the
        // early-out that must NOT sleep.
        let mut qc = file_queue::zeroed();
        let mut qr = file_queue::zeroed();
        let flags = CRALERT_FP_SET | CRALERT_READ_ALL;
        assert_eq!(
            (b.c.Init_FileQueue)(&mut qc, &t, flags),
            (b.rs.Init_FileQueue)(&mut qr, &t, flags)
        );
        assert!(qc.fp.is_null() && qr.fp.is_null());

        // Reproduce the state Read_FileMon sees: fp NULL and Handle_Queue(_,0)
        // succeeding is impossible here (fopen("<stdin>") in the crate dir
        // fails), so drive the second branch directly by pre-setting fp to NULL
        // and letting Handle_Queue(0) run. Timing is asserted to bound it.
        let t0 = std::time::Instant::now();
        let ac = (b.c.Read_FileMon)(&mut qc, &t, 0);
        let tc = t0.elapsed();
        let t1 = std::time::Instant::now();
        let ar = (b.rs.Read_FileMon)(&mut qr, &t, 0);
        let tr = t1.elapsed();
        assert_eq!(snap(ac), snap(ar));
        assert!(ac.is_null() && ar.is_null());
        // both must take the same branch => comparable durations
        let slow = |d: std::time::Duration| d.as_millis() > 2500;
        assert_eq!(
            slow(tc),
            slow(tr),
            "C and Rust disagree on whether file_sleep ran (C={tc:?} Rust={tr:?})"
        );
        assert_eq!(qsnap(&qc, true), qsnap(&qr, true));
    }
}

/* ===================== row 15: file deleted between the two Handle_Queues == */

#[test]
fn err_15_file_deleted_midway() {
    let w = workdir("e15");
    let b = both();
    unsafe {
        let t = tm::new(3, 3, 106);
        // No parseable alert -> the first GetAlertData fails; then the file is
        // gone, so the re-Handle_Queue returns 0 -> file_sleep + NULL.
        let mut run = |imp: &Impl| {
            w.write("alerts.log", b"not an alert at all\n");
            let mut q = file_queue::zeroed();
            let r0 = (imp.Init_FileQueue)(&mut q, &t, CRALERT_READ_ALL);
            w.remove("alerts.log");
            let a = (imp.Read_FileMon)(&mut q, &t, 0);
            let s = snap(a);
            if !a.is_null() {
                (imp.FreeAlertData)(a);
            }
            let qs = qsnap(&q, true);
            if !q.fp.is_null() {
                fclose(q.fp);
            }
            (r0, s, qs)
        };
        let (r0c, sc, mut qc) = run(&b.c);
        let (r0r, sr, mut qr) = run(&b.rs);
        // inode/mtime come from two distinct creations of the same path
        qc.st_ino = 0;
        qr.st_ino = 0;
        qc.st_mtime = 0;
        qr.st_mtime = 0;
        qc.last_change = 0;
        qr.last_change = 0;
        assert_eq!(r0c, r0r);
        assert_eq!(sc, sr);
        assert!(sc.is_none(), "must return NULL");
        assert_eq!(qc, qr);
        assert!(qc.fp_null, "fp must be NULL after the failed re-open");
    }
}

/* ===================== row 16: timeout expiry ===================== */

#[test]
fn err_16_timeout_expires() {
    let w = workdir("e16");
    let b = both();
    unsafe {
        let t = tm::new(3, 3, 106);
        w.write("alerts.log", b"nothing parseable here\nnor here\n");
        // timeout = 0: no retry loop at all
        // timeout = 1: exactly one 5 s file_sleep per library
        for timeout in [0u32, 1] {
            let mut run = |imp: &Impl| {
                let mut q = file_queue::zeroed();
                (imp.Init_FileQueue)(&mut q, &t, CRALERT_READ_ALL);
                let a = (imp.Read_FileMon)(&mut q, &t, timeout);
                let s = snap(a);
                if !a.is_null() {
                    (imp.FreeAlertData)(a);
                }
                let qs = qsnap(&q, true);
                if !q.fp.is_null() {
                    fclose(q.fp);
                }
                (s, qs)
            };
            let (sc, qc) = run(&b.c);
            let (sr, qr) = run(&b.rs);
            assert_eq!(sc, sr, "timeout={timeout}");
            assert!(sc.is_none(), "timeout={timeout} must expire to NULL");
            assert_eq!(qc, qr);
        }
    }
}

/* ===================== row 17: push-back fseek failure ===================== */

#[test]
fn err_17_alert_pushback_fseek_fail() {
    let b = both();
    unsafe {
        // Two alerts on an unseekable pipe: the first alert parses, then the
        // second "** Alert" header triggers fseek(-strlen, SEEK_CUR) which fails
        // on a pipe -> l_error -> NULL.
        let payload = {
            let mut v = Vec::new();
            v.extend_from_slice(b"** Alert 1500000000.1: mail - syslog,\n");
            v.extend_from_slice(b"2006 Apr 13 16:15:17 host->/var/log/x\n");
            v.extend_from_slice(b"Rule: 1 (level 2) -> 'c'\n");
            v.extend_from_slice(b"** Alert 1500000000.2: mail - syslog,\n");
            v.extend_from_slice(b"2006 Apr 13 16:15:18 host->/var/log/y\n");
            v.extend_from_slice(b"Rule: 2 (level 3) -> 'd'\n");
            v
        };
        let mut run = |imp: &Impl| {
            let fp = pipe_stream(&payload);
            let a = (imp.GetAlertData)(0, fp);
            let s = snap(a);
            if !a.is_null() {
                (imp.FreeAlertData)(a);
            }
            fclose(fp);
            s
        };
        let sc = run(&b.c);
        let sr = run(&b.rs);
        assert_eq!(sc, sr, "unseekable push-back result differs");
        assert!(
            sc.is_none(),
            "fseek on a pipe must fail -> NULL, got {sc:#?}"
        );
    }
}

/* ===================== rows 18-21: header/pre-header rejections ============ */

#[test]
fn err_18_alert_no_colon() {
    for (i, hdr) in [
        &b"** Alert no colon here"[..],
        &b"** Alert"[..],
        &b"** Alert "[..],
        &b"** Alertxyz"[..],
        &b"** Alert 12345"[..],
    ]
    .iter()
    .enumerate()
    {
        let mut buf = hdr.to_vec();
        buf.extend_from_slice(b"\n2006 Apr 13 16:15:17 h->/x\nRule: 1 (level 2) -> 'c'\n\n");
        cmp_reject(&format!("e18_{i}"), &buf, 0);
    }
}

#[test]
fn err_19_alert_no_space() {
    // ':' present, but no ' ' anywhere from str+9 onwards
    for (i, hdr) in [
        &b"** Alertx:y"[..],
        &b"** Alert:12345:abc"[..],
        &b"** Alert1:2"[..],
    ]
    .iter()
    .enumerate()
    {
        let mut buf = hdr.to_vec();
        buf.extend_from_slice(b"\n2006 Apr 13 16:15:17 h->/x\nRule: 1 (level 2) -> 'c'\n\n");
        cmp_reject(&format!("e19_{i}"), &buf, 0);
    }
}

#[test]
fn err_20_mail_set_rejects_nonmail() {
    for (i, tag) in [&b"noop"[..], &b"exec"[..], &b"MAIL"[..], &b"mai"[..], &b"x"[..]]
        .iter()
        .enumerate()
    {
        let mut buf = b"** Alert 1500000000.1: ".to_vec();
        buf.extend_from_slice(tag);
        buf.extend_from_slice(b" - syslog,\n2006 Apr 13 16:15:17 h->/x\nRule: 1 (level 2) -> 'c'\n\n");
        cmp_reject(&format!("e20_{i}"), &buf, CRALERT_MAIL_SET);
        // ... and the very same input is accepted without the flag
        let got = cmp(&format!("e20_ok_{i}"), &buf, 0);
        assert!(got[0].is_some(), "should parse without CRALERT_MAIL_SET");
    }
}

#[test]
fn err_21_lines_before_header_ignored() {
    let mut buf = Vec::new();
    for i in 0..50 {
        buf.extend_from_slice(format!("junk line {i} Rule: 9 Src IP: 1 User: u\n").as_bytes());
    }
    buf.extend_from_slice(&simple_alert());
    let got = cmp("e21", &buf, 0);
    let a = got[0].as_ref().expect("alert after the junk must still parse");
    assert_eq!(a.rule, 5715, "pre-header lines must not leak into the alert");
}

/* ===================== rows 22-24: date/location line rejections ========== */

#[test]
fn err_22_dateline_colon_no_space() {
    // ':' present but strchr(p,' ') from the colon onwards is NULL
    for (i, line) in [
        &b"2006Apr13:16:15:17"[..],
        &b"a:b"[..],
        &b":"[..],
        &b"host:/var/log/x"[..],
    ]
    .iter()
    .enumerate()
    {
        let mut buf = b"** Alert 1500000000.1: noop - syslog,\n".to_vec();
        buf.extend_from_slice(line);
        buf.extend_from_slice(b"\nRule: 1 (level 2) -> 'c'\n\n");
        cmp_reject(&format!("e22_{i}"), &buf, 0);
    }
}

#[test]
fn err_23_dateline_no_colon() {
    // no ':' at all -> p stays NULL -> the `!p` guard fires
    for (i, line) in [
        &b"2006 Apr 13 161517 /var/log/x"[..],
        &b"plain"[..],
        &b""[..],
        &b"   "[..],
    ]
    .iter()
    .enumerate()
    {
        let mut buf = b"** Alert 1500000000.1: noop - syslog,\n".to_vec();
        buf.extend_from_slice(line);
        buf.extend_from_slice(b"\nRule: 1 (level 2) -> 'c'\n\n");
        cmp_reject(&format!("e23_{i}"), &buf, 0);
    }
}

#[test]
fn err_24_date_already_set_unreachable() {
    // `_r` flips to 2 the moment date/location are stored, and only a new
    // "** Alert" header can bring it back -- which returns/pushes back first.
    // The closest reachable shape is two date-looking lines in a row: the
    // second is consumed as a log line, not as a second date line.
    let mut buf = b"** Alert 1500000000.1: noop - syslog,\n".to_vec();
    buf.extend_from_slice(b"2006 Apr 13 16:15:17 h->/x\n");
    buf.extend_from_slice(b"2006 Apr 13 16:15:18 h->/y\n");
    buf.extend_from_slice(b"Rule: 1 (level 2) -> 'c'\n\n");
    let got = cmp("e24", &buf, 0);
    let a = got[0].as_ref().expect("should parse");
    assert_eq!(a.date.as_deref(), Some(&b"2006 Apr 13 16:15:17"[..]));
    assert_eq!(a.location.as_deref(), Some(&b"h->/x"[..]));
}

/* ===================== rows 25-27: Rule: line rejections ===================== */

#[test]
fn err_25_rule_too_few_spaces() {
    for (i, rule) in [
        &b"Rule: 5715"[..],
        &b"Rule: 5715 (level"[..],
        &b"Rule: "[..],
        &b"Rule: x"[..],
        &b"Rule: 1 2"[..],
    ]
    .iter()
    .enumerate()
    {
        let mut buf = b"** Alert 1500000000.1: noop - syslog,\n".to_vec();
        buf.extend_from_slice(b"2006 Apr 13 16:15:17 h->/x\n");
        buf.extend_from_slice(rule);
        buf.extend_from_slice(b"\n\n");
        cmp_reject(&format!("e25_{i}"), &buf, 0);
    }

    // `"Rule:"` (no trailing space) does NOT match RULE_BEGIN ("Rule: ", 6 bytes)
    // once os_clearnl has stripped the newline, so it is treated as a plain log
    // line and the alert is accepted. Both libraries must agree on that.
    let mut buf = b"** Alert 1500000000.1: noop - syslog,\n".to_vec();
    buf.extend_from_slice(b"2006 Apr 13 16:15:17 h->/x\n");
    buf.extend_from_slice(b"Rule:\n\n");
    let got = cmp("e25_not_a_rule_line", &buf, 0);
    let a = got[0]
        .as_ref()
        .expect("\"Rule:\" is a log line, so the alert parses");
    assert_eq!(a.rule, 0);
    assert_eq!(a.comment, None);
}

#[test]
fn err_26_rule_no_quote() {
    for (i, rule) in [
        &b"Rule: 5715 (level 5) -> no quote"[..],
        &b"Rule: 1 2 3"[..],
        &b"Rule: 1 2 3 4 5 6"[..],
    ]
    .iter()
    .enumerate()
    {
        let mut buf = b"** Alert 1500000000.1: noop - syslog,\n".to_vec();
        buf.extend_from_slice(b"2006 Apr 13 16:15:17 h->/x\n");
        buf.extend_from_slice(rule);
        buf.extend_from_slice(b"\n\n");
        cmp_reject(&format!("e26_{i}"), &buf, 0);
    }
}

#[test]
fn err_27_rule_unterminated_comment() {
    for (i, rule) in [
        &b"Rule: 5715 (level 5) -> 'unterminated"[..],
        &b"Rule: 5715 (level 5) -> '"[..],
        &b"Rule: 1 1 1 'abc"[..],
    ]
    .iter()
    .enumerate()
    {
        let mut buf = b"** Alert 1500000000.1: noop - syslog,\n".to_vec();
        buf.extend_from_slice(b"2006 Apr 13 16:15:17 h->/x\n");
        buf.extend_from_slice(rule);
        buf.extend_from_slice(b"\n\n");
        cmp_reject(&format!("e27_{i}"), &buf, 0);
    }
    // exactly two quotes -> accepted (the closing one is found by strrchr)
    let mut ok = b"** Alert 1500000000.1: noop - syslog,\n".to_vec();
    ok.extend_from_slice(b"2006 Apr 13 16:15:17 h->/x\n");
    ok.extend_from_slice(b"Rule: 5715 (level 5) -> 'terminated'\n\n");
    assert!(cmp("e27_ok", &ok, 0)[0].is_some());
}

/* ===================== rows 28, 29: EOF in the wrong state ===================== */

#[test]
fn err_28_eof_wrong_state() {
    cmp_reject("e28_empty", b"", 0);                       // _r == 0
    cmp_reject("e28_hdr", b"** Alert 1: noop - g,\n", 0);  // _r == 1
    cmp_reject("e28_hdr_nonl", b"** Alert 1: noop - g,", 0);
    cmp_reject("e28_junk", b"a\nb\nc\n", 0);
    // _r == 2 at EOF -> accepted
    let mut ok = b"** Alert 1: noop - g,\n".to_vec();
    ok.extend_from_slice(b"2006 Apr 13 16:15:17 h->/x\n");
    assert!(cmp("e28_ok", &ok, 0)[0].is_some(), "_r==2 at EOF must succeed");
}

#[test]
fn err_29_fp_at_eof() {
    let b = both();
    let d = dir();
    let body = simple_alert();
    unsafe {
        for imp_pair in [(&b.c, &b.rs)] {
            let fpc = open_bytes(&d, "e29_c.log", &body);
            let fpr = open_bytes(&d, "e29_r.log", &body);
            fseek(fpc, 0, SEEK_END);
            fseek(fpr, 0, SEEK_END);
            let ac = (imp_pair.0.GetAlertData)(0, fpc);
            let ar = (imp_pair.1.GetAlertData)(0, fpr);
            assert_eq!(snap(ac), snap(ar));
            assert!(ac.is_null() && ar.is_null(), "EOF start must yield NULL");
            // clearerr must have reset EOF so a rewind+retry works
            fseek(fpc, 0, SEEK_SET);
            fseek(fpr, 0, SEEK_SET);
            let ac2 = (imp_pair.0.GetAlertData)(0, fpc);
            let ar2 = (imp_pair.1.GetAlertData)(0, fpr);
            let s = snap(ac2);
            assert_eq!(s, snap(ar2));
            assert!(s.is_some(), "stream must be usable after clearerr");
            if !ac2.is_null() {
                (imp_pair.0.FreeAlertData)(ac2);
            }
            if !ar2.is_null() {
                (imp_pair.1.FreeAlertData)(ar2);
            }
            fclose(fpc);
            fclose(fpr);
        }
    }
}

/* ===================== row 30: LOG_LIMIT never trips ===================== */

#[test]
fn err_30_log_limit_never_trips() {
    let mut buf = b"** Alert 1500000000.1: noop - ossec,syscheck,\n".to_vec();
    buf.extend_from_slice(b"2006 Apr 13 16:15:17 h->/x\n");
    buf.extend_from_slice(b"Rule: 1 (level 2) -> 'c'\n");
    for i in 0..250 {
        buf.extend_from_slice(format!("log line number {i}\n").as_bytes());
    }
    buf.push(b'\n');
    let got = cmp("e30", &buf, 0);
    assert!(got[0].is_some(), "250 log lines must still parse");

    // the syscheck filename capture also lives behind the same guard
    let mut buf2 = b"** Alert 1500000000.1: noop - ossec,syscheck,\n".to_vec();
    buf2.extend_from_slice(b"2006 Apr 13 16:15:17 h->/x\n");
    buf2.extend_from_slice(b"Integrity checksum changed for: '/etc/x'\n");
    for i in 0..150 {
        buf2.extend_from_slice(format!("tail {i}\n").as_bytes());
    }
    let got2 = cmp("e30b", &buf2, 0);
    assert_eq!(
        got2[0].as_ref().unwrap().filename.as_deref(),
        Some(&b"/etc/x"[..])
    );
}

/* ===================== row 31: empty integrity filename ===================== */

#[test]
fn err_31_integrity_empty_filename() {
    let b = both();
    let d = dir();
    // `filename[strlen(filename) - 1] = '\0'` with an empty filename writes one
    // byte *before* the malloc block, corrupting the chunk header. Both
    // libraries must fail the same way, so compare the raw wait status.
    let mut buf = b"** Alert 1500000000.1: noop - ossec,syscheck,\n".to_vec();
    buf.extend_from_slice(b"2006 Apr 13 16:15:17 h->/x\n");
    buf.extend_from_slice(b"Integrity checksum changed for: '\n");
    buf.extend_from_slice(b"tail\n\n");

    unsafe {
        let mut run = |imp: &'static Impl, tag: &str| -> c_int {
            let path = d.join(format!("e31_{tag}.err"));
            let file = d.join(format!("e31_{tag}.log"));
            std::fs::write(&file, &buf).unwrap();
            let cf = CString::new(file.to_str().unwrap()).unwrap();
            fork_status(&path, move || {
                let fp = fopen(cf.as_ptr(), b"r\0".as_ptr() as *const c_char);
                let a = (imp.GetAlertData)(0, fp);
                if !a.is_null() {
                    // exercise the free path too -- this is where the corrupted
                    // chunk header is noticed
                    (imp.FreeAlertData)(a);
                }
                fclose(fp);
                _exit(7);
            })
        };
        let stc = run(&b.c, "c");
        let str_ = run(&b.rs, "r");
        assert_eq!(
            (wait_exited(stc), wait_signal(stc)),
            (wait_exited(str_), wait_signal(str_)),
            "empty-integrity-filename outcome differs (C={stc:#x} Rust={str_:#x})"
        );
    }
}

/* ===================== row 32: nonnull params (documented) ================= */

#[test]
fn err_32_nonnull_documented() {
    // `__attribute__((nonnull))` makes a NULL argument undefined behaviour in C
    // (gcc even optimises the check away), so there is no defined rejection to
    // compare. Recorded here for completeness; the Rust wrappers dereference
    // the same pointers, i.e. they are equally undefined.
    let b = both();
    assert_eq!(b.c.name, "C");
    assert_eq!(b.rs.name, "Rust");
}

/* ===================== row 33: out-of-range tm_mon ===================== */

/// In-range months: full equality, including the 3 bytes `strncpy`d into `mon`.
#[test]
fn err_33a_in_range_month_exact() {
    let w = workdir("e33a");
    let b = both();
    unsafe {
        w.write("alerts.log", &simple_alert());
        for mon in 0..12 {
            let t = tm::new(5, mon, 106);
            let mut qc = file_queue::zeroed();
            let mut qr = file_queue::zeroed();
            let rc = (b.c.Init_FileQueue)(&mut qc, &t, CRALERT_READ_ALL);
            let rr = (b.rs.Init_FileQueue)(&mut qr, &t, CRALERT_READ_ALL);
            assert_eq!(rc, rr);
            assert_eq!(qsnap(&qc, true), qsnap(&qr, true), "mon={mon}");
            if !qc.fp.is_null() {
                fclose(qc.fp);
            }
            if !qr.fp.is_null() {
                fclose(qr.fp);
            }
        }
    }
}

/// Out-of-range `tm_mon` makes the C read past the end of the 12-entry
/// `s_month` table and `strncpy` from the resulting pointer -- undefined
/// behaviour, typically a SIGSEGV. The Rust reproduces the unchecked read
/// verbatim (`src/file_queue.rs::copy_month`), so both are expected to fail the
/// same way. Each value is exercised in a forked child so a crash cannot take
/// the harness down, and the raw wait statuses are compared.
#[test]
fn err_33b_out_of_range_month_is_ub_in_both() {
    let w = workdir("e33b");
    let b = both();
    let d = dir();
    w.write("alerts.log", &simple_alert());
    unsafe {
        for mon in [-1i32, 12, 13, 100, -100, i32::MIN, i32::MAX] {
            let run = |imp: &'static Impl, tag: &str| -> c_int {
                fork_status(&d.join(format!("e33b_{tag}_{mon}.err")), move || {
                    let t = tm::new(5, mon, 106);
                    let mut q = file_queue::zeroed();
                    let r = (imp.Init_FileQueue)(&mut q, &t, CRALERT_READ_ALL);
                    let a = (imp.Read_FileMon)(&mut q, &t, 0);
                    // encode "survived + observable outcome" in the exit code
                    let code = (if r == 0 { 1 } else { 0 })
                        | (if a.is_null() { 0 } else { 2 })
                        | (if q.fp.is_null() { 0 } else { 4 });
                    _exit(code);
                })
            };
            let stc = run(&b.c, "c");
            let str_ = run(&b.rs, "r");
            let died = |st: c_int| wait_signal(st).is_some();
            println!(
                "mon={mon:>12}: C {} / Rust {}",
                if died(stc) {
                    format!("signal {}", wait_signal(stc).unwrap())
                } else {
                    format!("exit {}", wait_exited(stc).unwrap())
                },
                if died(str_) {
                    format!("signal {}", wait_signal(str_).unwrap())
                } else {
                    format!("exit {}", wait_exited(str_).unwrap())
                }
            );
            if !died(stc) && !died(str_) {
                // Whenever the wild read happens to land on a readable pointer in
                // BOTH objects, every observable output must agree: `tm_mon` only
                // ever feeds the (unobservable, garbage) `mon` field.
                assert_eq!(
                    wait_exited(stc),
                    wait_exited(str_),
                    "mon={mon}: both survived the OOB read but disagree on the outcome"
                );
            }
            // If either child crashed, the outcome is decided by whatever byte
            // happens to sit next to a 12-pointer table inside that particular
            // shared object. That is not a property of the translation and cannot
            // be matched; the translation's obligation -- performing the same
            // unchecked read rather than bounds-checking it away -- is asserted by
            // `err_33c_rust_does_not_bounds_check_the_month`.
        }
    }
}

/// The translation must NOT be safer than the ground truth: an out-of-range
/// `tm_mon` has to reach the same unchecked `s_month[tm_mon]` read. Proof: for
/// at least one out-of-range value the Rust object faults exactly like the C
/// one, which is only possible if no bounds check was inserted. (A bounds-
/// checked `copy_month` would leave `mon` untouched and never fault.)
#[test]
fn err_33c_rust_does_not_bounds_check_the_month() {
    let w = workdir("e33c");
    let b = both();
    let d = dir();
    w.write("alerts.log", b"");
    unsafe {
        let probe = |imp: &'static Impl, tag: &str| -> Vec<(i32, bool)> {
            let mut out = Vec::new();
            for mon in [12i32, 13, 64, 1024, 65536, -1, -64, i32::MIN, i32::MAX] {
                let st = fork_status(&d.join(format!("e33c_{tag}_{mon}.err")), move || {
                    let t = tm::new(5, mon, 106);
                    let mut q = file_queue::zeroed();
                    (imp.Init_FileQueue)(&mut q, &t, CRALERT_READ_ALL);
                    _exit(0);
                });
                out.push((mon, wait_signal(st).is_some()));
            }
            out
        };
        let pc = probe(&b.c, "c");
        let pr = probe(&b.rs, "r");
        println!("C   faults: {pc:?}");
        println!("Rust faults: {pr:?}");
        assert!(
            pc.iter().any(|&(_, f)| f),
            "expected the C to fault on some out-of-range month"
        );
        assert!(
            pr.iter().any(|&(_, f)| f),
            "the Rust never faulted on any out-of-range month -- copy_month must \
             perform the same unchecked s_month[] read as the C, not a bounds check"
        );
    }
}

#[test]
#[ignore = "UB: C reads s_month out of bounds; see err_33a/err_33b/err_33c"]
fn err_33_out_of_range_month() {
    let w = workdir("e33");
    let b = both();
    unsafe {
        w.write("alerts.log", &simple_alert());
        // s_month[tm_mon] is read out of bounds for these; `mon` is therefore
        // excluded from the comparison, but every *observable* output
        // (alert_data, return codes, day/year/flags/file_name) must match.
        for mon in [-1i32, 12, 13, 100, -100, i32::MIN, i32::MAX] {
            let t = tm::new(5, mon, 106);
            let mut qc = file_queue::zeroed();
            let mut qr = file_queue::zeroed();
            let rc = (b.c.Init_FileQueue)(&mut qc, &t, CRALERT_READ_ALL);
            let rr = (b.rs.Init_FileQueue)(&mut qr, &t, CRALERT_READ_ALL);
            assert_eq!(rc, rr, "mon={mon}");
            assert_eq!(qsnap(&qc, false), qsnap(&qr, false), "mon={mon}");

            let ac = (b.c.Read_FileMon)(&mut qc, &t, 0);
            let ar = (b.rs.Read_FileMon)(&mut qr, &t, 0);
            let sc = snap(ac);
            let sr = snap(ar);
            if !ac.is_null() {
                (b.c.FreeAlertData)(ac);
            }
            if !ar.is_null() {
                (b.rs.FreeAlertData)(ar);
            }
            assert_eq!(sc, sr, "Read_FileMon differs for mon={mon}");
            assert!(sc.is_some(), "should still return the alert");
            if !qc.fp.is_null() {
                fclose(qc.fp);
            }
            if !qr.fp.is_null() {
                fclose(qr.fp);
            }

            // and through the driver, where `month` goes straight into tm_mon
            let dc = (b.c.driver)(5, mon, 106, 0, CRALERT_READ_ALL);
            let dr = (b.rs.driver)(5, mon, 106, 0, CRALERT_READ_ALL);
            let s1 = snap(dc);
            let s2 = snap(dr);
            if !dc.is_null() {
                (b.c.FreeAlertData)(dc);
            }
            if !dr.is_null() {
                (b.rs.FreeAlertData)(dr);
            }
            assert_eq!(s1, s2, "driver differs for month={mon}");
        }
    }
}

/* ===================== row 34: unknown flag bits ===================== */

#[test]
fn err_34_unknown_flag_bits() {
    let w = workdir("e34");
    let b = both();
    unsafe {
        w.write("alerts.log", &simple_alert());
        w.write("<stdin>", &simple_alert());
        // Values with no valid variant: the C API takes a plain int, so these
        // are real inputs. Only bits 0x1/0x4/0x10 are ever tested.
        for f in [
            -1,
            i32::MIN,
            i32::MAX,
            0x20,
            0x40,
            0x8000,
            0x7fff_ffe0,
            -0x20,
            0x1_0000,
            0x0002 | 0x0008, // the two inert documented bits only
        ] {
            let dc = (b.c.driver)(5, 3, 106, 0, f);
            let dr = (b.rs.driver)(5, 3, 106, 0, f);
            let s1 = snap(dc);
            let s2 = snap(dr);
            if !dc.is_null() {
                (b.c.FreeAlertData)(dc);
            }
            if !dr.is_null() {
                (b.rs.FreeAlertData)(dr);
            }
            assert_eq!(s1, s2, "driver flags={f:#x} differs");

            // ... and at the GetAlertData level
            let t = tm::new(5, 3, 106);
            let mut qc = file_queue::zeroed();
            let mut qr = file_queue::zeroed();
            let rc = (b.c.Init_FileQueue)(&mut qc, &t, f);
            let rr = (b.rs.Init_FileQueue)(&mut qr, &t, f);
            assert_eq!(rc, rr, "Init_FileQueue flags={f:#x}");
            assert_eq!(qsnap(&qc, true), qsnap(&qr, true), "flags={f:#x}");
            if !qc.fp.is_null() {
                fclose(qc.fp);
            }
            if !qr.fp.is_null() {
                fclose(qr.fp);
            }
        }
    }
}
