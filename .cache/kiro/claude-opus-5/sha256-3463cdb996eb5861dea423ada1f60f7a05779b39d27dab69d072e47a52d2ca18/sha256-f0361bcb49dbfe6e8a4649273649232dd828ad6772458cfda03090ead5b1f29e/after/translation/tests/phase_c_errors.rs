//! Phase C — error-path differential tests, one per ERRORS.md row.
//!
//! Each test constructs the exact invalid input/condition, calls BOTH `.so`s,
//! and asserts the SAME error/rejection: the same return sentinel (NULL / -1 /
//! 0), the same process exit status where the C `exit`s, and the same stderr
//! bytes where the C reports.

#![allow(non_snake_case)]

mod harness;
use harness::*;

use std::ffi::{c_char, c_int, c_void};

const ALL_FLAGS: c_int =
    CRALERT_MAIL_SET | CRALERT_EXEC_SET | CRALERT_READ_ALL | CRALERT_READ_FAILED | CRALERT_FP_SET;

// ---------------------------------------------------------------------------
// Rows 1-3: shared.h allocation failures -> stderr message + exit(1).
// ---------------------------------------------------------------------------

/// ERRORS.md row 1 — `os_strdup(NULL)`.
#[test]
fn err01_os_strdup_null() {
    let p = libs();
    unsafe {
        let (co, ce) = run_in_child(|| {
            (p.c.os_strdup)(std::ptr::null());
        });
        let (ro, re) = run_in_child(|| {
            (p.rs.os_strdup)(std::ptr::null());
        });
        assert_same!("os_strdup(NULL) outcome", co.clone(), ro);
        assert_same!(
            "os_strdup(NULL) stderr",
            String::from_utf8_lossy(&ce).into_owned(),
            String::from_utf8_lossy(&re).into_owned()
        );
        assert_eq!(co, ChildOutcome::Exited(1));
        assert_eq!(ce, b"NULL string passed to os_strdup");
    }
}

/// ERRORS.md row 2 — `os_calloc` when `calloc` fails.
#[test]
fn err02_os_calloc_oom() {
    let p = libs();
    for (num, size) in [
        (usize::MAX, usize::MAX),
        (usize::MAX, 2),
        (2, usize::MAX),
        (usize::MAX / 2, 4),
    ] {
        unsafe {
            let (co, ce) = run_in_child(|| {
                let q = (p.c.os_calloc)(num, size);
                // Reaching here means calloc unexpectedly succeeded.
                free(q);
            });
            let (ro, re) = run_in_child(|| {
                let q = (p.rs.os_calloc)(num, size);
                free(q);
            });
            assert_same!(
                format!("os_calloc({num},{size}) outcome"),
                co.clone(),
                ro
            );
            assert_same!(
                format!("os_calloc({num},{size}) stderr"),
                String::from_utf8_lossy(&ce).into_owned(),
                String::from_utf8_lossy(&re).into_owned()
            );
            assert_eq!(co, ChildOutcome::Exited(1));
            assert_eq!(ce, b"Memory allocation failed in os_calloc");
        }
    }
}

/// ERRORS.md row 3 — `os_realloc` when `realloc` fails.
#[test]
fn err03_os_realloc_oom() {
    let p = libs();
    for size in [usize::MAX, usize::MAX - 1, usize::MAX / 2] {
        unsafe {
            let (co, ce) = run_in_child(|| {
                let q = (p.c.os_realloc)(std::ptr::null_mut(), size);
                free(q);
            });
            let (ro, re) = run_in_child(|| {
                let q = (p.rs.os_realloc)(std::ptr::null_mut(), size);
                free(q);
            });
            assert_same!(format!("os_realloc(NULL,{size}) outcome"), co.clone(), ro);
            assert_same!(
                format!("os_realloc(NULL,{size}) stderr"),
                String::from_utf8_lossy(&ce).into_owned(),
                String::from_utf8_lossy(&re).into_owned()
            );
            assert_eq!(co, ChildOutcome::Exited(1));
            assert_eq!(ce, b"Memory allocation failed in os_realloc");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 4: the `fseek(-strlen)` push-back failing on a non-seekable stream.
// ---------------------------------------------------------------------------

/// ERRORS.md row 4 — `_r == 2` plus a second `** Alert` header on a pipe, so
/// the push-back `fseek` fails and `GetAlertData` must return NULL.
#[test]
fn err04_pushback_fseek_fails_on_pipe() {
    let p = libs();
    let mut a = AlertSpec::minimal();
    a.srcip = Some("Src IP: 1.2.3.4".into());
    let two = render_all(&[a.clone(), a.clone()]);
    let five = render_all(&[a.clone(), a.clone(), a.clone(), a.clone(), a]);

    for (tag, text) in [("two", &two), ("five", &five)] {
        unsafe {
            let pc = PipeFile::new(text.as_bytes());
            let c = snap_and_free(&p.c, (p.c.GetAlertData)(0, pc.fp));
            let c_err = ferror(pc.fp) != 0;

            let pr = PipeFile::new(text.as_bytes());
            let r = snap_and_free(&p.rs, (p.rs.GetAlertData)(0, pr.fp));
            let r_err = ferror(pr.fp) != 0;

            assert_same!(
                format!("GetAlertData pipe push-back [{tag}]"),
                (c.clone(), c_err),
                (r, r_err)
            );
            assert_eq!(c, AlertSnap::Null, "the push-back must fail on a pipe");
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 5-18: GetAlertData rejection branches. `diff_get_alert_data` compares
// the result sequence AND the stream state; these tests additionally assert the
// first result is the NULL/rejection the C produces.
// ---------------------------------------------------------------------------

/// Assert both libraries reject `text` (first `GetAlertData` yields NULL) and
/// agree on the stream state and stderr bytes.
fn assert_rejected(tag: &str, text: &[u8], flag: c_int) {
    let p = libs();
    let sc = Scratch::new("rej");
    let path = sc.write("alerts.log", text);
    unsafe {
        let fc = open_ro(&path);
        let (c, c_err) = capture_stderr(|| {
            let a = snap_and_free(&p.c, (p.c.GetAlertData)(flag, fc));
            (a, snap_stream(fc))
        });
        fclose(fc);

        let fr = open_ro(&path);
        let (r, r_err) = capture_stderr(|| {
            let a = snap_and_free(&p.rs, (p.rs.GetAlertData)(flag, fr));
            (a, snap_stream(fr))
        });
        fclose(fr);

        assert!(
            c == r,
            "DIVERGENCE in GetAlertData rejection [{tag}] flag={flag:#x}\n  input={:?}\n  C   : {c:#?}\n  Rust: {r:#?}",
            String::from_utf8_lossy(text)
        );
        assert!(
            c_err == r_err,
            "DIVERGENCE in GetAlertData stderr [{tag}]\n  C   : {:?}\n  Rust: {:?}",
            String::from_utf8_lossy(&c_err),
            String::from_utf8_lossy(&r_err)
        );
        assert_eq!(
            c.0,
            AlertSnap::Null,
            "[{tag}] the C is expected to reject this input: {:?}",
            String::from_utf8_lossy(text)
        );
    }
}

/// ERRORS.md row 5 — date line has `':'` but no `' '` at/after it.
#[test]
fn err05_date_colon_without_space() {
    for date in [
        "2006Apr13:16:15:17/var/log/auth.log",
        ":",
        "a:b",
        "16:15:17",
        "x:",
        ":::",
    ] {
        assert_rejected(
            "date-colon-nospace",
            format!("** Alert 1.1: mail - g\n{date}\n").as_bytes(),
            0,
        );
    }
}

/// ERRORS.md row 6 — date line with no `':'` at all, so `p` stays NULL.
#[test]
fn err06_date_without_colon() {
    for date in ["2006 Apr 13 /var/log/auth.log", "", " ", "no-colon-here", "\t"] {
        assert_rejected(
            "date-nocolon",
            format!("** Alert 1.1: mail - g\n{date}\n").as_bytes(),
            0,
        );
    }
}

/// ERRORS.md row 7 — the `al_data->date || al_data->location` disjunct.
///
/// Unreachable through the public API: `_r` only becomes 1 from the header
/// branch, and once `_r == 2` a header triggers the push-back `return` instead.
/// The test pins that unreachability: a stream with a header, a date line, and
/// then a second header returns the first alert rather than re-entering
/// `_r == 1`, identically in both libraries.
#[test]
fn err07_date_already_set_unreachable() {
    let a = AlertSpec::minimal();
    let text = render_all(&[a.clone(), a.clone(), a]);
    diff_get_alert_data("date-already-set", text.as_bytes(), 0);

    // Also with the date line repeated verbatim inside one alert: the second
    // copy is treated as a body line (_r == 2), never as a date line.
    let text = "** Alert 1.1: mail - g\n\
                2006 Apr 13 16:15:17 /var/log/a\n\
                2006 Apr 13 16:15:17 /var/log/a\n\
                Rule: 1 (level 1) -> 'c'\n";
    diff_get_alert_data("date-repeated", text.as_bytes(), 0);
}

/// ERRORS.md row 8 — `Rule:` line with no space after the rule id.
#[test]
fn err08_rule_no_first_space() {
    for rule in ["Rule: 1234", "Rule: ", "Rule: abc", "Rule: 1234\t(level 5)"] {
        assert_rejected(
            "rule-no-space",
            format!("** Alert 1.1: mail - g\n2006 Apr 13 16:15:17 /x\n{rule}\n").as_bytes(),
            0,
        );
    }
}

/// ERRORS.md row 9 — `Rule:` line where the SECOND `strchr(p,' ')` is NULL.
#[test]
fn err09_rule_no_second_space() {
    for rule in ["Rule: 1234 level", "Rule: 1234 ", "Rule: 1 (level"] {
        assert_rejected(
            "rule-no-2nd-space",
            format!("** Alert 1.1: mail - g\n2006 Apr 13 16:15:17 /x\n{rule}\n").as_bytes(),
            0,
        );
    }
}

/// ERRORS.md row 10 — `Rule:` line with no `'` after the level.
#[test]
fn err10_rule_no_quote() {
    for rule in [
        "Rule: 1 level 5 -> no quote",
        "Rule: 1 (level 5) -> plain text",
        "Rule: 1 2 3",
    ] {
        assert_rejected(
            "rule-no-quote",
            format!("** Alert 1.1: mail - g\n2006 Apr 13 16:15:17 /x\n{rule}\n").as_bytes(),
            0,
        );
    }
}

/// ERRORS.md row 11 — comment opened with `'` but never closed.
#[test]
fn err11_rule_unclosed_comment() {
    for rule in [
        "Rule: 1 (level 5) -> 'unterminated",
        "Rule: 1 (level 5) -> '",
    ] {
        assert_rejected(
            "rule-unclosed",
            format!("** Alert 1.1: mail - g\n2006 Apr 13 16:15:17 /x\n{rule}\n").as_bytes(),
            0,
        );
    }
    // Exactly one `'`, which `strrchr` finds -> this one SUCCEEDS in C with an
    // empty comment. Verified as a valid path, not a rejection.
    diff_get_alert_data(
        "rule-single-quote-at-end",
        b"** Alert 1.1: mail - g\n2006 Apr 13 16:15:17 /x\nRule: 1 (level 5) -> ''\n",
        0,
    );
}

/// ERRORS.md row 12 — EOF with `_r == 0` (no `** Alert` header anywhere).
#[test]
fn err12_eof_r0() {
    for text in [
        &b"just some text\n"[..],
        &b"** Alertx 1.1: mail - g\n"[..],
        &b"* Alert 1.1: mail - g\n"[..],
        &b"Rule: 1 (level 1) -> 'c'\n"[..],
        &b"Src IP: 1.1.1.1\nUser: root\n"[..],
        &b"\n\n\n"[..],
    ] {
        assert_rejected("eof-r0", text, 0);
    }
}

/// ERRORS.md row 13 — EOF with `_r == 1` (header, then nothing).
#[test]
fn err13_eof_r1() {
    for text in [
        &b"** Alert 1.1: mail - g\n"[..],
        &b"** Alert 1.1: mail - g"[..],
        &b"** Alert 1.1: mail\n"[..],
    ] {
        assert_rejected("eof-r1", text, 0);
    }
}

/// ERRORS.md row 14 — completely empty file.
#[test]
fn err14_empty_file() {
    assert_rejected("empty", b"", 0);
    assert_rejected("empty-mail", b"", CRALERT_MAIL_SET);
    assert_rejected("empty-allflags", b"", ALL_FLAGS);
    assert_rejected("empty-negflags", b"", -1);
}

/// ERRORS.md row 15 — the `FILE*` is already at EOF, or already in error state.
#[test]
fn err15_stream_already_exhausted() {
    let p = libs();
    let sc = Scratch::new("exhausted");
    let text = AlertSpec::minimal().render();
    let path = sc.write("alerts.log", text.as_bytes());

    unsafe {
        // Position past the end, and force the EOF indicator by reading.
        for prime in [false, true] {
            let run = |lib: &Lib| -> (AlertSnap, StreamSnap, AlertSnap, StreamSnap) {
                let fp = open_ro(&path);
                fseek(fp, text.len() as i64 + 100, SEEK_SET);
                if prime {
                    // Trigger the EOF flag with a real read attempt.
                    let mut buf = [0u8; 8];
                    let _ = fwrite(buf.as_mut_ptr() as *const c_void, 0, 0, fp);
                    let a = (lib.GetAlertData)(0, fp);
                    let s1 = snap_stream(fp);
                    let r1 = snap_and_free(lib, a);
                    let b = (lib.GetAlertData)(0, fp);
                    let s2 = snap_stream(fp);
                    let r2 = snap_and_free(lib, b);
                    fclose(fp);
                    (r1, s1, r2, s2)
                } else {
                    let a = (lib.GetAlertData)(0, fp);
                    let s1 = snap_stream(fp);
                    let r1 = snap_and_free(lib, a);
                    fclose(fp);
                    (r1.clone(), s1.clone(), r1, s1)
                }
            };
            let c = run(&p.c);
            let r = run(&p.rs);
            assert!(c == r, "DIVERGENCE on exhausted stream\n  C   : {c:#?}\n  Rust: {r:#?}");
            assert_eq!(c.0, AlertSnap::Null);
            // `clearerr` must have been called: the EOF flag is reset.
            assert!(!c.1.eof, "GetAlertData must clearerr before returning NULL");
        }
    }
}

/// ERRORS.md row 16 — header with no `':'` after `str + 9` -> `continue`.
#[test]
fn err16_header_no_colon() {
    for hdr in [
        "** Alert 1500000000 mail - g",
        "** Alert",
        "** Alert ",
        "** Alert  ",
        "** Alertzzz",
    ] {
        assert_rejected(
            "hdr-nocolon",
            format!("{hdr}\n2006 Apr 13 16:15:17 /x\nRule: 1 (level 1) -> 'c'\n").as_bytes(),
            0,
        );
    }
}

/// ERRORS.md row 17 — header where `strchr(p, ' ')` after the id is NULL.
#[test]
fn err17_header_no_space_after_id() {
    for hdr in ["** Alert 1.1:mail-g", "** Alert 1.1:", "** Alert :"] {
        assert_rejected(
            "hdr-nospace",
            format!("{hdr}\n2006 Apr 13 16:15:17 /x\nRule: 1 (level 1) -> 'c'\n").as_bytes(),
            0,
        );
    }
}

/// ERRORS.md row 18 — `CRALERT_MAIL_SET` with a non-`mail` token.
#[test]
fn err18_mail_flag_rejects() {
    for kind in ["exec", "mai", "MAIL", "", "-", "m", "xmail"] {
        let mut a = AlertSpec::minimal();
        a.kind = kind.into();
        assert_rejected(
            &format!("mail-reject-{kind}"),
            a.render().as_bytes(),
            CRALERT_MAIL_SET,
        );
    }
}

/// ERRORS.md row 19 — flags with no valid `CRALERT_*` variant. C `int` accepts
/// any value; only bit 0 may change the outcome.
#[test]
fn err19_out_of_range_flags() {
    let mut rng = Rng::new(0xE44_0019);
    let a = AlertSpec::minimal();
    let good = a.render();
    let mut bad = a.clone();
    bad.kind = "exec".into();
    let bad = bad.render();

    let mut flags: Vec<c_int> = vec![
        -1,
        i32::MIN,
        i32::MAX,
        0x20,
        0x40,
        0x80,
        0x1_0000,
        0x7FFF_FFFE,
        !0x1F,
        0x0000_FFFE,
    ];
    for _ in 0..100 {
        flags.push(rng.i32());
    }

    for flag in flags {
        // Valid alert under an arbitrary flag word.
        diff_get_alert_data("oob-flag-good", good.as_bytes(), flag);
        // Non-mail alert: rejected iff bit 0 is set.
        if flag & CRALERT_MAIL_SET != 0 {
            assert_rejected("oob-flag-bad", bad.as_bytes(), flag);
        } else {
            diff_get_alert_data("oob-flag-bad", bad.as_bytes(), flag);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 20-29: file-queue rejection branches.
// ---------------------------------------------------------------------------

/// ERRORS.md row 20 — `fopen` fails because `alerts.log` does not exist.
/// `Handle_Queue` returns 0, which `Init_FileQueue` reports as SUCCESS (0).
#[test]
fn err20_init_missing_file() {
    let p = libs();
    let sc = Scratch::new("err20"); // deliberately empty: no alerts.log
    let _cwd = enter_dir(&sc.dir);
    let t = tm {
        tm_mday: 1,
        tm_mon: 0,
        tm_year: 100,
        ..Default::default()
    };
    for flags in [0, CRALERT_READ_ALL, CRALERT_MAIL_SET, ALL_FLAGS & !CRALERT_FP_SET] {
        unsafe {
            let run = |lib: &Lib| {
                let mut fq = file_queue::default();
                let (rc, err) = capture_stderr(|| (lib.Init_FileQueue)(&mut fq, &t, flags));
                (rc, snap_fq(&fq), err)
            };
            let c = run(&p.c);
            let r = run(&p.rs);
            assert!(
                c == r,
                "DIVERGENCE in Init_FileQueue (missing file) flags={flags:#x}\n  C   : {c:#?}\n  Rust: {r:#?}"
            );
            assert_eq!(c.0, 0, "fopen failure is reported as success (0)");
            assert!(c.1.fp_is_null);
            assert!(c.2.is_empty(), "no stderr for a missing queue file");
        }
    }
}

/// ERRORS.md row 21 — `CRALERT_FP_SET` with `fp == NULL` and `CRALERT_READ_ALL`
/// clear: `Handle_Queue` hits `if (!fileq->fp) return (0)`.
#[test]
fn err21_fp_set_null_fp() {
    let p = libs();
    let sc = Scratch::new("err21");
    sc.write("alerts.log", AlertSpec::minimal().render().as_bytes());
    let _cwd = enter_dir(&sc.dir);
    let t = tm {
        tm_mday: 9,
        tm_mon: 9,
        tm_year: 109,
        ..Default::default()
    };
    for flags in [
        CRALERT_FP_SET,
        CRALERT_FP_SET | CRALERT_MAIL_SET,
        CRALERT_FP_SET | CRALERT_READ_ALL,
    ] {
        unsafe {
            let run = |lib: &Lib| {
                let mut fq = file_queue::default();
                let (rc, err) = capture_stderr(|| (lib.Init_FileQueue)(&mut fq, &t, flags));
                (rc, snap_fq(&fq), err)
            };
            let c = run(&p.c);
            let r = run(&p.rs);
            assert!(
                c == r,
                "DIVERGENCE in Init_FileQueue (FP_SET, NULL fp) flags={flags:#x}\n  C   : {c:#?}\n  Rust: {r:#?}"
            );
            assert_eq!(c.0, 0);
            assert!(c.1.fp_is_null);
            assert_eq!(c.1.file_name, b"<stdin>");
            assert!(c.2.is_empty());
        }
    }
}

/// ERRORS.md row 22 — `fseek(fp, 0, SEEK_END)` fails: `merror(FSEEK_ERROR)` on
/// stderr, `fp` closed and NULLed, `Handle_Queue` returns -1 so
/// `Init_FileQueue` returns -1.
#[test]
fn err22_fseek_error() {
    let p = libs();
    let sc = Scratch::new("err22");
    sc.write("alerts.log", b"");
    let _cwd = enter_dir(&sc.dir);
    let t = tm {
        tm_mday: 2,
        tm_mon: 2,
        tm_year: 102,
        ..Default::default()
    };

    unsafe {
        // A non-seekable caller-supplied stream with CRALERT_FP_SET and
        // CRALERT_READ_ALL clear -> the fseek branch is taken.
        let run = |lib: &Lib| {
            let pf = PipeFile::new(b"whatever\n");
            let mut fq = file_queue::default();
            fq.fp = pf.fp;
            let (rc, err) =
                capture_stderr(|| (lib.Init_FileQueue)(&mut fq, &t, CRALERT_FP_SET));
            // Handle_Queue already fclose'd it; don't double-close.
            std::mem::forget(pf);
            (rc, fq.fp.is_null(), fq.flags, err)
        };
        let c = run(&p.c);
        let r = run(&p.rs);
        assert!(
            c == r,
            "DIVERGENCE in Init_FileQueue fseek-error\n  C   : {c:#?}\n  Rust: {r:#?}"
        );
        assert_eq!(c.0, -1, "fseek failure must yield -1");
        assert!(c.1, "fp must be NULLed");
        assert!(
            String::from_utf8_lossy(&c.3).contains("(1116)"),
            "expected FSEEK_ERROR on stderr, got {:?}",
            String::from_utf8_lossy(&c.3)
        );
    }
}

/// ERRORS.md row 23 — `fstat(fileno(fp))` fails: `merror(FSTAT_ERROR)`, `fp`
/// closed and NULLed, -1 returned.
#[test]
fn err23_fstat_error() {
    let p = libs();
    let sc = Scratch::new("err23");
    sc.write("alerts.log", b"");
    let _cwd = enter_dir(&sc.dir);
    let t = tm {
        tm_mday: 3,
        tm_mon: 3,
        tm_year: 103,
        ..Default::default()
    };

    unsafe {
        // CRALERT_READ_ALL skips the fseek so the fstat branch is reached with
        // a stream whose descriptor has been closed behind its back.
        let run = |lib: &Lib| {
            let df = DeadFile::new();
            let mut fq = file_queue::default();
            fq.fp = df.fp;
            let (rc, err) = capture_stderr(|| {
                (lib.Init_FileQueue)(&mut fq, &t, CRALERT_FP_SET | CRALERT_READ_ALL)
            });
            (rc, fq.fp.is_null(), err)
        };
        let c = run(&p.c);
        let r = run(&p.rs);
        assert!(
            c == r,
            "DIVERGENCE in Init_FileQueue fstat-error\n  C   : {c:#?}\n  Rust: {r:#?}"
        );
        assert_eq!(c.0, -1, "fstat failure must yield -1");
        assert!(c.1, "fp must be NULLed");
        assert!(
            String::from_utf8_lossy(&c.2).contains("(1118)"),
            "expected FSTAT_ERROR on stderr, got {:?}",
            String::from_utf8_lossy(&c.2)
        );
    }
}

/// ERRORS.md rows 24 & 25 — `Init_FileQueue` returning -1 propagates through
/// `driver`, which prints `File queue initialization failed` and returns NULL.
///
/// Reached without `CRALERT_FP_SET` by making `alerts.log` a FIFO: `fopen`
/// succeeds (a writer is held open) but `fseek` fails with `ESPIPE`.
#[test]
fn err24_25_driver_init_failure() {
    let p = libs();
    let sc = Scratch::new("err25");
    let _fifo = Fifo::new(sc.path("alerts.log"));
    let _cwd = enter_dir(&sc.dir);

    unsafe {
        for flags in [0, CRALERT_MAIL_SET, CRALERT_EXEC_SET | CRALERT_READ_FAILED] {
            let (c, c_err) = capture_stderr(|| {
                snap_and_free(&p.c, (p.c.driver)(13, 3, 106, 0, flags))
            });
            let (r, r_err) = capture_stderr(|| {
                snap_and_free(&p.rs, (p.rs.driver)(13, 3, 106, 0, flags))
            });
            assert_same!(
                format!("driver init-failure flags={flags:#x}"),
                c.clone(),
                r
            );
            assert_same!(
                format!("driver init-failure stderr flags={flags:#x}"),
                String::from_utf8_lossy(&c_err).into_owned(),
                String::from_utf8_lossy(&r_err).into_owned()
            );
            assert_eq!(c, AlertSnap::Null);
            let text = String::from_utf8_lossy(&c_err).into_owned();
            assert!(
                text.contains("(1116)") && text.contains("File queue initialization failed"),
                "expected FSEEK_ERROR + the driver message, got {text:?}"
            );
        }
    }
}

/// ERRORS.md row 26 — `Read_FileMon` with `fp == NULL` and a queue that cannot
/// be opened: `file_sleep()` then NULL.
#[test]
fn err26_read_filemon_queue_unavailable() {
    let p = libs();
    let sc = Scratch::new("err26"); // no alerts.log at all
    let _cwd = enter_dir(&sc.dir);
    let t = tm {
        tm_mday: 6,
        tm_mon: 6,
        tm_year: 106,
        ..Default::default()
    };
    unsafe {
        let run = |lib: &Lib| {
            let mut fq = file_queue::default();
            let rc = (lib.Init_FileQueue)(&mut fq, &t, CRALERT_READ_ALL);
            let start = std::time::Instant::now();
            let a = (lib.Read_FileMon)(&mut fq, &t, 0);
            let slept = start.elapsed();
            let snap = snap_and_free(lib, a);
            (rc, snap, snap_fq(&fq), slept)
        };
        let c = run(&p.c);
        let r = run(&p.rs);
        assert_same!(
            "Read_FileMon queue-unavailable",
            (c.0, c.1.clone(), c.2.clone()),
            (r.0, r.1, r.2)
        );
        assert_eq!(c.1, AlertSnap::Null);
        // Both must have taken the 5 s `file_sleep` path.
        for (name, d) in [("C", c.3), ("Rust", r.3)] {
            assert!(
                d.as_secs_f64() > 4.0,
                "{name} did not take the FQ_TIMEOUT sleep ({d:?})"
            );
        }
    }
}

/// ERRORS.md row 27 — the second `if (!fileq->fp) return (NULL)` in
/// `Read_FileMon`.
///
/// Unreachable: `Handle_Queue(fileq, 0)` can only return 1 with a non-NULL
/// `fp`, because without `CRALERT_FP_SET` it returns 0 whenever `fopen` fails.
/// The test pins the reachable neighbourhood: entering `Read_FileMon` with a
/// NULL `fp` under every flag word ends in the row-26 sleep-then-NULL path,
/// never in a `fp`-NULL success.
#[test]
fn err27_second_null_fp_unreachable() {
    let p = libs();
    let sc = Scratch::new("err27");
    sc.write("alerts.log", AlertSpec::minimal().render().as_bytes());
    let _cwd = enter_dir(&sc.dir);
    let t = tm {
        tm_mday: 7,
        tm_mon: 7,
        tm_year: 107,
        ..Default::default()
    };
    unsafe {
        // Init with CRALERT_FP_SET and a NULL fp leaves fp NULL and the queue
        // name at "<stdin>"; Read_FileMon then re-opens with flags = 0.
        let run = |lib: &Lib| {
            let mut fq = file_queue::default();
            let rc = (lib.Init_FileQueue)(&mut fq, &t, CRALERT_FP_SET | CRALERT_READ_ALL);
            let had_fp = !fq.fp.is_null();
            let a = (lib.Read_FileMon)(&mut fq, &t, 0);
            let snap = snap_and_free(lib, a);
            let fqs = snap_fq(&fq);
            if !fq.fp.is_null() {
                fclose(fq.fp);
            }
            (rc, had_fp, snap, fqs)
        };
        let c = run(&p.c);
        let r = run(&p.rs);
        assert!(
            c == r,
            "DIVERGENCE in Read_FileMon NULL-fp path\n  C   : {c:#?}\n  Rust: {r:#?}"
        );
        assert_eq!(c.0, 0);
        assert!(!c.1, "Init must leave fp NULL here");
        assert_eq!(c.2, AlertSnap::Null);
    }
}

/// ERRORS.md row 28 — the queue file disappears after `Init_FileQueue`, so the
/// re-`Handle_Queue(fileq, 0)` inside `Read_FileMon` fails: sleep then NULL.
#[test]
fn err28_requeue_fails_midway() {
    let p = libs();
    let t = tm {
        tm_mday: 8,
        tm_mon: 8,
        tm_year: 108,
        ..Default::default()
    };
    unsafe {
        let run = |lib: &Lib| {
            let sc = Scratch::new("err28");
            sc.write("alerts.log", b"nothing parseable here\n");
            let _cwd = enter_dir(&sc.dir);
            let mut fq = file_queue::default();
            let rc = (lib.Init_FileQueue)(&mut fq, &t, CRALERT_READ_ALL);
            // Remove the file so the re-open inside Read_FileMon fails.
            std::fs::remove_file(sc.path("alerts.log")).unwrap();
            let start = std::time::Instant::now();
            let a = (lib.Read_FileMon)(&mut fq, &t, 0);
            let slept = start.elapsed();
            let snap = snap_and_free(lib, a);
            let fqs = snap_fq(&fq);
            if !fq.fp.is_null() {
                fclose(fq.fp);
            }
            (rc, snap, fqs, slept)
        };
        let c = run(&p.c);
        let r = run(&p.rs);
        // The two runs necessarily see two separately created `alerts.log`
        // files, so inode and mtime differ for reasons unrelated to the code.
        assert_same!(
            "Read_FileMon requeue-failure",
            (c.0, c.1.clone(), c.2.clone().masked()),
            (r.0, r.1, r.2.masked())
        );
        assert_eq!(c.1, AlertSnap::Null);
        for (name, d) in [("C", c.3), ("Rust", r.3)] {
            assert!(
                d.as_secs_f64() > 4.0,
                "{name} did not take the FQ_TIMEOUT sleep ({d:?})"
            );
        }
    }
}

/// ERRORS.md rows 29 & 34 — `timeout` exhaustion, including the `timeout == 0`
/// boundary where the retry loop never runs.
#[test]
fn err29_34_timeout_exhausted() {
    let p = libs();
    let sc = Scratch::new("err29");
    sc.write("alerts.log", b"no alerts\n");
    let _cwd = enter_dir(&sc.dir);
    let t = tm {
        tm_mday: 9,
        tm_mon: 1,
        tm_year: 99,
        ..Default::default()
    };

    unsafe {
        for timeout in [0u32, 1] {
            let run = |lib: &Lib| {
                let mut fq = file_queue::default();
                let rc = (lib.Init_FileQueue)(&mut fq, &t, CRALERT_READ_ALL);
                let start = std::time::Instant::now();
                let a = (lib.Read_FileMon)(&mut fq, &t, timeout);
                let elapsed = start.elapsed().as_secs_f64();
                let snap = snap_and_free(lib, a);
                let fqs = snap_fq(&fq);
                if !fq.fp.is_null() {
                    fclose(fq.fp);
                }
                (rc, snap, fqs, elapsed)
            };
            let c = run(&p.c);
            let r = run(&p.rs);
            assert_same!(
                format!("Read_FileMon timeout={timeout}"),
                (c.0, c.1.clone(), c.2.clone()),
                (r.0, r.1, r.2)
            );
            assert_eq!(c.1, AlertSnap::Null);
            // timeout == 0 must not sleep; timeout == 1 sleeps once.
            let expect_sleep = timeout > 0;
            for (name, e) in [("C", c.3), ("Rust", r.3)] {
                assert_eq!(
                    e > 4.0,
                    expect_sleep,
                    "{name} sleep behaviour wrong for timeout={timeout} ({e}s)"
                );
            }
        }
    }
}

/// ERRORS.md row 30 — `merror` `snprintf` truncation at 256 bytes.
#[test]
fn err30_merror_truncation() {
    let p = libs();
    const FSTAT_ERROR: &str =
        "(1118): Could not retrieve information of file '%s' due to [(%d)-(%s)].";
    const FSEEK_ERROR: &str = "(1116): Could not set position in file '%s' due to [(%d)-(%s)].";

    let mut rng = Rng::new(0xE44_0030);
    for tmpl in [FSTAT_ERROR, FSEEK_ERROR] {
        let t = cs(tmpl);
        // Lengths that straddle the 256-byte buffer boundary.
        let mut lens: Vec<usize> = (150..=260).step_by(1).collect();
        lens.extend([0, 1, 500, 1024, 4096]);
        for len in lens {
            let name = vec![b'N'; len];
            let n = cbytes(&name);
            let msg = rng.token_upto(40);
            let m = cbytes(&msg);
            let err = rng.i32();
            unsafe {
                let (_, ce) =
                    capture_stderr(|| (p.c.merror)(t.as_ptr(), n.as_ptr(), err, m.as_ptr()));
                let (_, re) =
                    capture_stderr(|| (p.rs.merror)(t.as_ptr(), n.as_ptr(), err, m.as_ptr()));
                assert_same!(
                    format!("merror truncation len={len}"),
                    String::from_utf8_lossy(&ce).into_owned(),
                    String::from_utf8_lossy(&re).into_owned()
                );
                // buffer[256] holds at most 255 chars + NUL, then a '\n'.
                assert!(ce.len() <= 256, "merror emitted {} bytes", ce.len());
            }
        }
    }
}

/// ERRORS.md row 31 — the empty-`filename` underflow write.
#[test]
fn err31_filename_underflow() {
    for tail in ["", " ", "'", "x"] {
        let text = format!(
            "** Alert 1.1: mail - syscheck\n\
             2006 Apr 13 16:15:17 /x\n\
             Rule: 1 (level 1) -> 'c'\n\
             Integrity checksum changed for: '{tail}\n"
        );
        diff_get_alert_data("filename-underflow", text.as_bytes(), 0);
    }
    // With no trailing newline either, so `os_clearnl` finds nothing.
    diff_get_alert_data(
        "filename-underflow-noeol",
        b"** Alert 1.1: mail - syscheck\n2006 Apr 13 16:15:17 /x\nIntegrity checksum changed for: '",
        0,
    );
}

/// ERRORS.md row 32 — lines exceeding `OS_MAXSTR - 1`, at and around the exact
/// `fgets` boundary.
#[test]
fn err32_oversized_line_boundary() {
    for len in [1021usize, 1022, 1023, 1024, 1025, 1026, 2046, 2047, 2048] {
        // Oversized header line: the split tail becomes its own logical line.
        let text = format!(
            "** Alert 1.1: mail - {}\n2006 Apr 13 16:15:17 /x\nRule: 1 (level 1) -> 'c'\n",
            "g".repeat(len)
        );
        diff_get_alert_data(&format!("oversz-hdr{len}"), text.as_bytes(), 0);

        // Oversized date line.
        let text = format!(
            "** Alert 1.1: mail - g\n2006 Apr 13 16:15:17 /{}\nRule: 1 (level 1) -> 'c'\n",
            "d".repeat(len)
        );
        diff_get_alert_data(&format!("oversz-date{len}"), text.as_bytes(), 0);

        // Oversized `Rule:` line, so the closing quote lands in the tail.
        let text = format!(
            "** Alert 1.1: mail - g\n2006 Apr 13 16:15:17 /x\nRule: 1 (level 1) -> '{}'\n",
            "c".repeat(len)
        );
        diff_get_alert_data(&format!("oversz-rule{len}"), text.as_bytes(), 0);

        // Oversized integrity line under a syscheck group.
        let text = format!(
            "** Alert 1.1: mail - syscheck\n2006 Apr 13 16:15:17 /x\nIntegrity checksum changed for: '{}'\n",
            "p".repeat(len)
        );
        diff_get_alert_data(&format!("oversz-integ{len}"), text.as_bytes(), 0);
    }
}

/// ERRORS.md row 33 — `tm_mday` / `tm_year` extremes with no range check.
#[test]
fn err33_tm_extremes() {
    let p = libs();
    let sc = Scratch::new("err33");
    sc.write("alerts.log", b"");
    let _cwd = enter_dir(&sc.dir);

    let mut rng = Rng::new(0xE44_0033);
    let mut cases: Vec<(c_int, c_int)> = vec![
        (0, 0),
        (-1, -1),
        (1, -1900),
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN),
        (i32::MAX, i32::MAX - 1899),
        (i32::MIN, i32::MIN + 1),
    ];
    for _ in 0..80 {
        cases.push((rng.i32(), rng.i32()));
    }

    unsafe {
        for (mday, year) in cases {
            let t = tm {
                tm_mday: mday,
                tm_mon: rng.below(12) as c_int,
                tm_year: year,
                ..Default::default()
            };
            let run = |lib: &Lib| {
                let mut fq = file_queue::default();
                let rc = (lib.Init_FileQueue)(&mut fq, &t, CRALERT_READ_ALL);
                let s = snap_fq(&fq);
                if !fq.fp.is_null() {
                    fclose(fq.fp);
                }
                (rc, s)
            };
            let c = run(&p.c);
            let r = run(&p.rs);
            assert!(
                c == r,
                "DIVERGENCE for tm_mday={mday} tm_year={year}\n  C   : {c:#?}\n  Rust: {r:#?}"
            );
            assert_eq!(c.1.year, year.wrapping_add(1900));
        }
    }
}

/// ERRORS.md row 35 — the `__attribute__((nonnull))` annotations are a compiler
/// hint; no runtime null check exists in either implementation, so passing NULL
/// is UB. Documented here, deliberately not exercised.
#[test]
fn err35_nonnull_is_a_hint_only() {
    // Confirm the annotations really are the only "check": there is no
    // `if (!fileq)` / `if (!fp)` guard anywhere in the C sources.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("c_src/src");
    let mut src = String::new();
    for f in ["file-queue.c", "read-alert.c", "driver.c"] {
        src.push_str(&std::fs::read_to_string(root.join(f)).unwrap());
    }
    assert!(!src.contains("if (!fileq)"));
    assert!(!src.contains("if (!fp)"));
    assert!(!src.contains("if (!al_data)"));
    assert!(src.contains("nonnull"));
}

/// ERRORS.md row 36 — `s_month[tm_mon]` performs no bounds check.
///
/// Out-of-range `tm_mon` reads past a `static const char *[12]`, which is UB
/// whose result depends on whatever follows the array in each `.so` image, so it
/// cannot be byte-matched between two independently linked libraries. Instead
/// this pins that (a) the C really has no check and (b) all twelve in-range
/// values agree.
#[test]
fn err36_month_index_unchecked() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("c_src/src/file-queue.c");
    let src = std::fs::read_to_string(root).unwrap();
    assert!(src.contains("s_month[p->tm_mon]"));
    assert!(
        !src.contains("tm_mon < 0") && !src.contains("tm_mon > 11") && !src.contains("tm_mon >= 12"),
        "the C has no tm_mon range check; the Rust must not add one"
    );

    let p = libs();
    let sc = Scratch::new("err36");
    sc.write("alerts.log", b"");
    let _cwd = enter_dir(&sc.dir);
    unsafe {
        for mon in 0..12 {
            let t = tm {
                tm_mday: 1,
                tm_mon: mon,
                tm_year: 100,
                ..Default::default()
            };
            let run = |lib: &Lib| {
                let mut fq = file_queue::default();
                let rc = (lib.Init_FileQueue)(&mut fq, &t, CRALERT_READ_ALL);
                let s = snap_fq(&fq);
                if !fq.fp.is_null() {
                    fclose(fq.fp);
                }
                (rc, s)
            };
            assert_same!(format!("tm_mon={mon}"), run(&p.c), run(&p.rs));
        }
    }
    let _ = std::mem::size_of::<*const c_char>();
}
