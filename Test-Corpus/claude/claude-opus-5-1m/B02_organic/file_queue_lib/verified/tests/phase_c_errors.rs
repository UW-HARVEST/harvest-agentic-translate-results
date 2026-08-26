//! Phase C — one differential test per row of `ERRORS.md`.
//!
//! Every test constructs the exact invalid input / condition the C rejects on,
//! calls BOTH `.so`s, and asserts they produce the SAME rejection: the same
//! return sentinel, the same `errno`-derived message on stderr, and — for the
//! `exit(EXIT_FAILURE)` paths — the same process exit status.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void, CString};

const MAIL: c_int = 0x001;
const READ_ALL: c_int = 0x004;
const FP_SET: c_int = 0x010;

const ALERTS: &str = "alerts.log";
const STDIN_NAME: &str = "<stdin>";

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Run `f` against each library in a forked child and require identical
/// termination status *and* identical stderr.
fn diff_child(tag: &str, f: impl Fn(&'static Api)) {
    let (c, r) = apis();
    let (oc, ec) = run_in_child(|| f(c));
    let (or, er) = run_in_child(|| f(r));
    assert_eq!(
        oc, or,
        "[{tag}] termination differs: C={oc:?} RUST={or:?}\n  C stderr={:?}\n  R stderr={:?}",
        String::from_utf8_lossy(&ec),
        String::from_utf8_lossy(&er)
    );
    assert_eq!(
        ec,
        er,
        "[{tag}] stderr differs\n  C   ={:?}\n  RUST={:?}",
        String::from_utf8_lossy(&ec),
        String::from_utf8_lossy(&er)
    );
}

fn tm_of(mday: c_int, mon: c_int, year: c_int) -> Tm {
    let mut t = Tm::default();
    t.tm_mday = mday;
    t.tm_mon = mon;
    t.tm_year = year;
    t
}

/// `Init_FileQueue` differential that also compares whatever `merror` wrote.
struct InitOut {
    rc: c_int,
    q: QueueSnap,
    err: Vec<u8>,
}

unsafe fn init_capture(
    api: &Api,
    mut q: FileQueue,
    tm: &Tm,
    flags: c_int,
    preset: Option<*mut FILE>,
) -> InitOut {
    if let Some(f) = preset {
        q.fp = f;
    }
    let mut rc = 0;
    let err = capture_stderr(|| {
        rc = (api.Init_FileQueue)(&mut q, tm, flags);
    });
    let snap = snap_queue(&q);
    if !q.fp.is_null() {
        fclose(q.fp);
    }
    InitOut { rc, q: snap, err }
}

fn diff_init_err(
    tag: &str,
    start: FileQueue,
    tm: &Tm,
    flags: c_int,
    mk: impl Fn() -> Option<*mut FILE>,
    expect_rc: Option<c_int>,
) {
    let (c, r) = apis();
    unsafe {
        let a = init_capture(c, start, tm, flags, mk());
        let b = init_capture(r, start, tm, flags, mk());
        assert_eq!(a.rc, b.rc, "[{tag}] flags={flags:#x} rc differs");
        if let Some(e) = expect_rc {
            assert_eq!(a.rc, e, "[{tag}] C rc is not the documented {e}");
        }
        assert_eq!(a.q, b.q, "[{tag}] flags={flags:#x} file_queue differs");
        assert_eq!(
            a.err,
            b.err,
            "[{tag}] merror output differs\n  C   ={:?}\n  RUST={:?}",
            String::from_utf8_lossy(&a.err),
            String::from_utf8_lossy(&b.err)
        );
    }
}

/// Assert both libraries return NULL for this input (an `l_error` / rejection).
fn assert_both_reject(tag: &str, bytes: &[u8], flag: c_int) {
    let (c, r) = apis();
    unsafe {
        let dir = tmp_dir();
        let fc = file_stream(dir, "rej_c.log", bytes);
        let pc = (c.GetAlertData)(flag, fc);
        let sc = take_alert(c, pc);
        let stc = stream_state(fc);
        fclose(fc);

        let fr = file_stream(dir, "rej_r.log", bytes);
        let pr = (r.GetAlertData)(flag, fr);
        let sr = take_alert(r, pr);
        let str_ = stream_state(fr);
        fclose(fr);

        assert_eq!(
            sc,
            sr,
            "[{tag}] result differs for {:?}",
            String::from_utf8_lossy(bytes)
        );
        assert_eq!(stc, str_, "[{tag}] stream state differs");
        assert!(
            sc.is_none(),
            "[{tag}] expected the C to REJECT {:?} but it returned {sc:?}",
            String::from_utf8_lossy(bytes)
        );
        // `clearerr` must have run on the error path.
        assert_eq!(
            stc.as_ref().map(|s| s.err),
            Some(false),
            "[{tag}] clearerr did not run"
        );
    }
}

// ===========================================================================
// Row 1 — os_calloc: calloc returns NULL -> exit(1)
// ===========================================================================

#[test]
fn err_01_os_calloc_alloc_failure_exits_1() {
    for (num, size) in [
        (usize::MAX, 1usize),
        (1, usize::MAX),
        (usize::MAX, usize::MAX), // multiplication overflow
        (usize::MAX / 2, 4),
        (1 << 62, 8),
    ] {
        diff_child(&format!("os_calloc({num},{size})"), move |api| unsafe {
            (api.os_calloc)(num, size);
        });
    }
}

// ===========================================================================
// Row 2 — os_realloc: realloc returns NULL -> exit(1)
// ===========================================================================

#[test]
fn err_02_os_realloc_alloc_failure_exits_1() {
    for n in [usize::MAX, usize::MAX - 8, 1usize << 62, (1usize << 63) + 1] {
        diff_child(&format!("os_realloc(NULL,{n})"), move |api| unsafe {
            (api.os_realloc)(std::ptr::null_mut(), n);
        });
    }
}

/// Row 2b — `realloc(non-NULL, 0)` frees and returns NULL on glibc, which the C
/// then treats as an allocation failure.
#[test]
fn err_02b_os_realloc_shrink_to_zero() {
    diff_child("os_realloc(p,0)", |api| unsafe {
        let p = (api.os_realloc)(std::ptr::null_mut(), 64);
        (api.os_realloc)(p, 0);
    });
}

// ===========================================================================
// Row 3 — os_strdup(NULL) -> exit(1)
// ===========================================================================

#[test]
fn err_03_os_strdup_null_exits_1() {
    diff_child("os_strdup(NULL)", |api| unsafe {
        (api.os_strdup)(std::ptr::null());
    });
}

// ===========================================================================
// Row 5 — second `** Alert` header while _r == 2 on a NON-SEEKABLE stream:
//         fseek fails (ESPIPE) -> l_error -> NULL
// ===========================================================================

#[test]
fn err_05_second_header_on_pipe_fseek_fails() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x0505_2024);
    for i in 0..60 {
        let a = rand_alert(&mut rng, false);
        let b = rand_alert(&mut rng, false);
        let mut bytes = a.bytes();
        b.render(&mut bytes);
        unsafe {
            let fc = pipe_stream(&bytes);
            let pc = (c.GetAlertData)(0, fc);
            let sc = take_alert(c, pc);
            fclose(fc);
            let fr = pipe_stream(&bytes);
            let pr = (r.GetAlertData)(0, fr);
            let sr = take_alert(r, pr);
            fclose(fr);
            assert_eq!(sc, sr, "[pipe#{i}] fseek-failure result differs");
            assert!(
                sc.is_none(),
                "[pipe#{i}] expected NULL from the fseek error path, got {sc:?}"
            );
        }
        // Same bytes on a seekable file must instead SUCCEED — proves the row
        // is really exercising the fseek failure and not a parse failure.
        diff_get_alert_data(&format!("pipe-control#{i}"), &bytes, 0, &[Kind::File], 3);
    }
}

// ===========================================================================
// Row 6 — header line with no ':' at/after str+9 -> `continue`
// ===========================================================================

#[test]
fn err_06_header_without_colon() {
    let mut rng = Rng::new(0x0606_2024);
    for _ in 0..80 {
        let mut b = b"** Alert ".to_vec();
        let mut t = rng.token_len(0, 30);
        t.retain(|&x| x != b':');
        b.extend_from_slice(&t);
        b.push(b'\n');
        b.extend_from_slice(b"2006 Apr 13 16:15:17 /loc\n");
        b.extend_from_slice(b"Rule: 1 (level) 2 -> 'c'\n");
        assert_both_reject("no-colon", &b, 0);
        assert_both_reject("no-colon", &b, MAIL);
    }
    // `strstr` starts at str+9, so a colon BEFORE that offset does not count.
    for raw in [
        &b"** Alert:\n2006 Apr 13 16:15:17 /loc\n"[..],
        &b"** Alert \n2006 Apr 13 16:15:17 /loc\n"[..],
        &b"** Alertx:y\n2006 Apr 13 16:15:17 /loc\n"[..],
    ] {
        // (the third does not even match `** Alert` + offset-9 colon rules)
        assert_both_reject("colon-before-9", raw, 0);
    }
}

// ===========================================================================
// Row 7 — header with a colon but no ' ' at/after str+9 -> `continue`
//         (alertid IS assigned first)
// ===========================================================================

#[test]
fn err_07_header_colon_no_space() {
    for raw in [
        &b"** Alert:x\n2006 Apr 13 16:15:17 /loc\n"[..],
        &b"** Alert12:34\n2006 Apr 13 16:15:17 /loc\n"[..],
        &b"** Alert_abc:def\n2006 Apr 13 16:15:17 /loc\n"[..],
        &b"** Alertzz:\n2006 Apr 13 16:15:17 /loc\n"[..],
    ] {
        assert_both_reject("colon-no-space", raw, 0);
        assert_both_reject("colon-no-space", raw, MAIL);
    }
}

// ===========================================================================
// Row 8 — CRALERT_MAIL_SET but the word after the first space is not "mail"
// ===========================================================================

#[test]
fn err_08_mail_set_but_not_mail() {
    let mut rng = Rng::new(0x0808_2024);
    let words: [&[u8]; 9] = [
        b"mai", b"mailx", b"MAIL", b"nomail", b"", b"m", b"maiL", b"xmail", b"ail",
    ];
    for w in words {
        for _ in 0..12 {
            let id = rng.token_len(1, 5);
            let mut b = b"** Alert ".to_vec();
            b.extend_from_slice(&id);
            b.extend_from_slice(b": ");
            b.extend_from_slice(w);
            b.extend_from_slice(b" - grp\n");
            b.extend_from_slice(b"2006 Apr 13 16:15:17 /loc\n");
            b.extend_from_slice(b"Rule: 5 (level) 6 -> 'c'\n");
            // The C compares only the first ALERT_MAIL_SZ == 4 bytes, so any
            // word with a `mail` PREFIX (e.g. `mailx`) is still accepted; only
            // the others are the row-8 rejection.
            if w.starts_with(b"mail") {
                diff_get_alert_data("mail-prefix", &b, MAIL, &[Kind::File], 3);
            } else {
                assert_both_reject("mail-mismatch", &b, MAIL);
            }
            // Without the flag the very same input always parses.
            diff_get_alert_data("mail-off", &b, 0, &[Kind::File], 3);
        }
    }
}

// ===========================================================================
// Row 9 — body lines arriving while _r < 1 are ignored
// ===========================================================================

#[test]
fn err_09_body_before_header() {
    let mut rng = Rng::new(0x0909_2024);
    for _ in 0..80 {
        let mut b = Vec::new();
        for _ in 0..(1 + rng.below(5)) {
            b.extend_from_slice(&rng.token_len(0, 30));
            b.push(b'\n');
        }
        b.extend_from_slice(b"Rule: 1 (level) 2 -> 'c'\n");
        b.extend_from_slice(b"Src IP: 1.2.3.4\n");
        assert_both_reject("body-before-header", &b, 0);
    }
    // and the same lines AFTER a header do get consumed (control)
    diff_get_alert_data(
        "body-after-header",
        b"** Alert 1.2: mail - g\n2006 Apr 13 16:15:17 /loc\nRule: 1 (level) 2 -> 'c'\n",
        0,
        &[Kind::File],
        3,
    );
}

// ===========================================================================
// Row 10 — date line with ':' but no ' ' after it
//          -> perror + l_error -> NULL
// ===========================================================================

#[test]
fn err_10_dateline_colon_without_space() {
    for line in [
        &b"2006 Apr 13 16:15:17"[..],
        &b"a:b"[..],
        &b":"[..],
        &b"x:"[..],
        &b"aaa:bbb:ccc"[..],
        &b"16:15:17"[..],
    ] {
        let mut b = b"** Alert 1.2: mail - g\n".to_vec();
        b.extend_from_slice(line);
        b.push(b'\n');
        b.extend_from_slice(b"Rule: 1 (level) 2 -> 'c'\n");
        assert_both_reject("dateline-no-space", &b, 0);
    }
}

// ===========================================================================
// Row 11 — date line with NO colon at all -> p == NULL -> l_error -> NULL
// ===========================================================================

#[test]
fn err_11_dateline_no_colon() {
    let mut rng = Rng::new(0x1111_2024);
    for _ in 0..80 {
        let mut line = rng.token_len(0, 40);
        line.retain(|&b| b != b':');
        let mut b = b"** Alert 1.2: mail - g\n".to_vec();
        b.extend_from_slice(&line);
        b.push(b'\n');
        b.extend_from_slice(b"Rule: 1 (level) 2 -> 'c'\n");
        assert_both_reject("dateline-no-colon", &b, 0);
    }
    for line in [&b""[..], &b" "[..], &b"no colon here"[..], &b"    "[..]] {
        let mut b = b"** Alert 1.2: mail - g\n".to_vec();
        b.extend_from_slice(line);
        b.push(b'\n');
        assert_both_reject("dateline-no-colon", &b, 0);
    }
}

// ===========================================================================
// Row 12 — `Rule: ` line missing one of the two spaces -> l_error -> NULL
// ===========================================================================

#[test]
fn err_12_rule_missing_second_space() {
    for line in [
        &b"Rule: 1000"[..],
        &b"Rule: 1000 x"[..],
        &b"Rule: "[..],
        &b"Rule: x"[..],
        &b"Rule: 1 2"[..],
        &b"Rule:  "[..],
    ] {
        let mut b = b"** Alert 1.2: mail - g\n2006 Apr 13 16:15:17 /loc\n".to_vec();
        b.extend_from_slice(line);
        b.push(b'\n');
        assert_both_reject("rule-spaces", &b, 0);
    }
}

// ===========================================================================
// Row 13 — `Rule: ` line with both spaces but no opening quote
// ===========================================================================

#[test]
fn err_13_rule_missing_open_quote() {
    for line in [
        &b"Rule: 1000 level 7 no quote"[..],
        &b"Rule: 1 2 3"[..],
        &b"Rule: 1 (level) 7 -> no quote here"[..],
        &b"Rule: 0 0 0"[..],
    ] {
        let mut b = b"** Alert 1.2: mail - g\n2006 Apr 13 16:15:17 /loc\n".to_vec();
        b.extend_from_slice(line);
        b.push(b'\n');
        assert_both_reject("rule-no-quote", &b, 0);
    }
}

// ===========================================================================
// Row 14 — comment with no closing quote (strrchr fails) -> l_error -> NULL
// ===========================================================================

#[test]
fn err_14_rule_missing_close_quote() {
    let mut rng = Rng::new(0x1414_2024);
    for _ in 0..60 {
        let mut c = rng.token_len(0, 25);
        c.retain(|&b| b != b'\'');
        let mut b = b"** Alert 1.2: mail - g\n2006 Apr 13 16:15:17 /loc\n".to_vec();
        b.extend_from_slice(b"Rule: 42 (level) 9 -> '");
        b.extend_from_slice(&c);
        b.push(b'\n');
        assert_both_reject("rule-unterminated", &b, 0);
    }
}

// ===========================================================================
// Row 15 — EOF reached with _r != 2
// ===========================================================================

#[test]
fn err_15_eof_with_r_not_2() {
    for raw in [
        &b""[..],                                    // empty file
        &b"\n"[..],                                  // one blank line
        &b"nothing interesting here\n"[..],          // no header
        &b"** Alert 1.2: mail - g\n"[..],            // header only (_r == 1)
        &b"** Alert 1.2: mail - g"[..],              // header only, no newline
        &b"** Alert 1.2: mail - g\n** Alert 3.4: mail - h\n"[..], // header, header
        &b"Rule: 1 (level) 2 -> 'c'\n"[..],          // body only
    ] {
        assert_both_reject("eof-not-r2", raw, 0);
        assert_both_reject("eof-not-r2", raw, MAIL);
    }
}

// ===========================================================================
// Row 16 — fgets fails WITHOUT eof (write-only stream) -> l_error -> NULL
// ===========================================================================

#[test]
fn err_16_read_error_not_eof() {
    let (c, r) = apis();
    let s = Scratch::new("err16");
    s.write("wo_c.log", b"** Alert 1.2: mail - g\n2006 Apr 13 16:15:17 /loc\n");
    s.write("wo_r.log", b"** Alert 1.2: mail - g\n2006 Apr 13 16:15:17 /loc\n");
    unsafe {
        // "a" (append) opens for writing only: fgets fails with EBADF and the
        // eof indicator is NOT set.
        let fc = fopen_str("wo_c.log", "a");
        assert!(!fc.is_null());
        let eof_before = feof(fc);
        let pc = (c.GetAlertData)(0, fc);
        let sc = take_alert(c, pc);
        let stc = stream_state(fc);
        fclose(fc);

        let fr = fopen_str("wo_r.log", "a");
        assert!(!fr.is_null());
        let pr = (r.GetAlertData)(0, fr);
        let sr = take_alert(r, pr);
        let str_ = stream_state(fr);
        fclose(fr);

        assert_eq!(eof_before, 0);
        assert_eq!(sc, sr, "write-only stream result differs");
        assert!(sc.is_none(), "expected NULL from the read-error path");
        assert_eq!(stc, str_, "write-only stream state differs");
        // clearerr() ran, so the error indicator is down again.
        assert_eq!(stc.map(|x| x.err), Some(false));
    }
}

// ===========================================================================
// Row 19 — fopen fails: Handle_Queue returns 0, Init_FileQueue still returns 0
// ===========================================================================

#[test]
fn err_19_init_missing_alerts_log_returns_0() {
    let s = Scratch::new("err19");
    s.remove(ALERTS);
    assert!(!s.exists(ALERTS));
    for flags in [0, MAIL, READ_ALL, MAIL | READ_ALL] {
        diff_init_err(
            "no-alerts-log",
            FileQueue::zeroed(),
            &tm_of(1, 0, 100),
            flags,
            || None,
            Some(0),
        );
    }
}

// ===========================================================================
// Row 20 — CRALERT_FP_SET with a NULL fp: Handle_Queue returns 0
// ===========================================================================

#[test]
fn err_20_fp_set_with_null_fp() {
    let s = Scratch::new("err20");
    s.write(ALERTS, b"content that must not be opened\n");
    s.write(STDIN_NAME, b"nor this\n");
    for flags in [FP_SET, FP_SET | MAIL] {
        diff_init_err(
            "fpset-null-fp",
            FileQueue::zeroed(),
            &tm_of(2, 1, 101),
            flags,
            || None,
            Some(0),
        );
    }
}

// ===========================================================================
// Row 21 — fseek(SEEK_END) fails on a pipe -> merror + fclose -> -1
// ===========================================================================

#[test]
fn err_21_fseek_error_returns_minus1() {
    let s = Scratch::new("err21");
    s.remove(ALERTS);
    diff_init_err(
        "fseek-espipe",
        FileQueue::zeroed(),
        &tm_of(3, 2, 102),
        FP_SET,
        || Some(unsafe { pipe_stream(b"** Alert 1.2: mail - g\n") }),
        Some(-1),
    );
    diff_init_err(
        "fseek-espipe-mail",
        FileQueue::zeroed(),
        &tm_of(3, 2, 102),
        FP_SET | MAIL,
        || Some(unsafe { pipe_stream(b"") }),
        Some(-1),
    );
}

// ===========================================================================
// Row 22 — fstat(fileno(fp)) fails on an fmemopen stream -> merror -> -1
// ===========================================================================

#[test]
fn err_22_fstat_error_returns_minus1() {
    let s = Scratch::new("err22");
    s.remove(ALERTS);
    // fileno() on an fmemopen stream is -1, so fstat fails with EBADF. Both
    // with and without CRALERT_READ_ALL (fmemopen IS seekable, so the fseek
    // succeeds and we always reach the fstat).
    for flags in [FP_SET, FP_SET | READ_ALL, FP_SET | READ_ALL | MAIL] {
        diff_init_err(
            "fstat-ebadf",
            FileQueue::zeroed(),
            &tm_of(4, 3, 103),
            flags,
            || Some(unsafe { mem_stream(b"** Alert 1.2: mail - g\n") }),
            Some(-1),
        );
    }
}

// ===========================================================================
// Row 23 — tm_mon outside 0..=11 indexes s_month out of bounds
// ===========================================================================

#[test]
fn err_23_tm_mon_out_of_range() {
    let s = Scratch::new("err23");
    s.write(ALERTS, b"** Alert 1.2: mail - g\n2006 Apr 13 16:15:17 /loc\n");

    // In range: full struct parity, including `mon`.
    for mon in 0..12 {
        diff_init_err(
            "mon-in-range",
            FileQueue::zeroed(),
            &tm_of(5, mon, 104),
            READ_ALL,
            || None,
            Some(0),
        );
    }

    // Out of range: the C reads `s_month` out of bounds (undefined behaviour)
    // and traps for essentially every value a caller could realistically pass.
    // Assert the *termination* agrees for the large-magnitude values, which is
    // the only part of the behaviour that is reproducible at all.
    let (c, r) = apis();
    for mon in [
        i32::MIN,
        i32::MIN + 1,
        -2_000_000_000,
        -1_000_000,
        -100_000,
        -1_000,
        -100,
        25,
        100,
        100_000,
        1_000_000,
        2_000_000_000,
        i32::MAX - 1,
        i32::MAX,
    ] {
        let run = |api: &'static Api| {
            move || unsafe {
                let mut q = FileQueue::zeroed();
                let tm = tm_of(5, mon, 104);
                (api.Init_FileQueue)(&mut q, &tm, READ_ALL);
            }
        };
        let (oc, _) = run_in_child(run(c));
        let (or, _) = run_in_child(run(r));
        assert_eq!(
            oc, or,
            "tm_mon={mon}: C terminated {oc:?} but Rust terminated {or:?} \
             (both must fault on the out-of-bounds s_month read)"
        );
        assert_eq!(
            oc,
            ChildOutcome::Signalled(11),
            "tm_mon={mon}: expected the C to fault on the OOB read"
        );
    }
}

// ===========================================================================
// Row 24 — tm_year + 1900 signed overflow
// ===========================================================================

#[test]
fn err_24_tm_year_overflow() {
    let s = Scratch::new("err24");
    s.write(ALERTS, b"** Alert 1.2: mail - g\n2006 Apr 13 16:15:17 /loc\n");
    for y in [
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 1899,
        i32::MIN + 1900,
        -1901,
        -1900,
        -1899,
        -1,
        0,
        i32::MAX - 1900,
        i32::MAX - 1899,
        i32::MAX - 1,
        i32::MAX,
    ] {
        diff_init_err(
            &format!("year={y}"),
            FileQueue::zeroed(),
            &tm_of(6, 4, y),
            READ_ALL,
            || None,
            Some(0),
        );
    }
    // and the same through `driver`, where tm_year is the `year` parameter
    let (c, r) = apis();
    for y in [i32::MIN, -1900, 0, i32::MAX] {
        unsafe {
            let pc = (c.driver)(6, 4, y, 0, READ_ALL);
            let sc = take_alert(c, pc);
            let pr = (r.driver)(6, 4, y, 0, READ_ALL);
            let sr = take_alert(r, pr);
            assert_eq!(sc, sr, "driver year={y} differs");
        }
    }
}

// ===========================================================================
// Row 25 — driver: Init_FileQueue failure branch
// ===========================================================================

#[test]
fn err_25_driver_init_failure() {
    let s = Scratch::new("err25");
    // `driver` always hands Init_FileQueue a zeroed fq, so CRALERT_FP_SET can
    // only reach the `return 0` path (row 20) and Init never returns < 0.
    // Assert that for EVERY flag value both libraries agree, and that the
    // "File queue initialization failed" branch is never taken by either.
    s.write(ALERTS, b"** Alert 1.2: mail - g\n2006 Apr 13 16:15:17 /loc\n");
    s.write(STDIN_NAME, b"** Alert 9.9: mail - h\n2007 May 14 01:02:03 /l2\n");
    let (c, r) = apis();
    for flags in 0..32 {
        unsafe {
            let mut pc = std::ptr::null_mut();
            let ec = capture_stderr(|| pc = (c.driver)(1, 0, 100, 0, flags));
            let sc = take_alert(c, pc);
            let mut pr = std::ptr::null_mut();
            let er = capture_stderr(|| pr = (r.driver)(1, 0, 100, 0, flags));
            let sr = take_alert(r, pr);
            assert_eq!(sc, sr, "driver flags={flags:#x} result differs");
            assert_eq!(
                ec,
                er,
                "driver flags={flags:#x} stderr differs\n C={:?}\n R={:?}",
                String::from_utf8_lossy(&ec),
                String::from_utf8_lossy(&er)
            );
        }
    }
}

// ===========================================================================
// Row 26 — Read_FileMon: queue unavailable -> file_sleep + NULL  (~5 s each)
// ===========================================================================

#[test]
fn err_26_readfilemon_no_queue() {
    let s = Scratch::new("err26");
    s.remove(ALERTS);
    let (c, r) = apis();
    let tm = tm_of(7, 5, 105);
    unsafe {
        let run = |api: &Api| {
            let mut q = FileQueue::zeroed();
            let rc = (api.Init_FileQueue)(&mut q, &tm, 0);
            let p = (api.Read_FileMon)(&mut q, &tm, 0);
            let a = take_alert(api, p);
            let snap = snap_queue(&q);
            if !q.fp.is_null() {
                fclose(q.fp);
            }
            (rc, a, snap)
        };
        let a = run(c);
        let b = run(r);
        assert_eq!(a.0, b.0, "Init rc differs");
        assert_eq!(a.1, b.1, "Read_FileMon result differs");
        assert_eq!(a.2, b.2, "file_queue differs");
        assert!(a.1.is_none(), "expected NULL when the queue is unavailable");
    }
}

// ===========================================================================
// Row 28 — alerts.log disappears between the two Handle_Queue calls (~5 s each)
// ===========================================================================

#[test]
fn err_28_readfilemon_file_vanishes() {
    let s = Scratch::new("err28");
    let (c, r) = apis();
    let tm = tm_of(8, 6, 106);
    // Written ONCE so both implementations fstat the identical inode/mtime;
    // it is renamed out of the way (not rewritten) to make it "vanish".
    s.write(ALERTS, b"not an alert at all\n");
    let live = s.dir.join(ALERTS);
    let hidden = s.dir.join("hidden.log");
    unsafe {
        let run = |api: &Api| {
            // present at Init (so fp is opened) but with nothing parseable
            if hidden.exists() {
                std::fs::rename(&hidden, &live).expect("restore alerts.log");
            }
            let mut q = FileQueue::zeroed();
            let rc = (api.Init_FileQueue)(&mut q, &tm, READ_ALL);
            // ... and gone by the time the re-open happens
            std::fs::rename(&live, &hidden).expect("hide alerts.log");
            let p = (api.Read_FileMon)(&mut q, &tm, 0);
            let a = take_alert(api, p);
            let mut snap = snap_queue(&q);
            // The *test* renames the file between the two runs to make it
            // vanish, and `rename` bumps the inode's ctime. That is an artifact
            // of the fixture, not of the library, so normalise just this field;
            // inode, size, mtime and last_change are all still compared.
            snap.st_ctim_sec = 0;
            if !q.fp.is_null() {
                fclose(q.fp);
            }
            (rc, a, snap)
        };
        let a = run(c);
        let b = run(r);
        assert_eq!(a.0, b.0);
        assert_eq!(a.1, b.1, "Read_FileMon result differs");
        assert_eq!(a.2, b.2, "file_queue differs");
        assert!(a.1.is_none());
        assert!(a.2.fp_null, "fp must have been left NULL");
        assert_ne!(a.2.st_mtim_sec, 0, "the fstat from Init must have happened");
    }
}

// ===========================================================================
// Row 29 — the timeout loop expires (~5 s each)
// ===========================================================================

#[test]
fn err_29_readfilemon_timeout_expires() {
    let s = Scratch::new("err29");
    s.write(ALERTS, b"garbage that never parses\n");
    let (c, r) = apis();
    let tm = tm_of(9, 7, 107);
    unsafe {
        let run = |api: &Api| {
            let mut q = FileQueue::zeroed();
            let rc = (api.Init_FileQueue)(&mut q, &tm, READ_ALL);
            let p = (api.Read_FileMon)(&mut q, &tm, 1);
            let a = take_alert(api, p);
            let snap = snap_queue(&q);
            if !q.fp.is_null() {
                fclose(q.fp);
            }
            (rc, a, snap)
        };
        let a = run(c);
        let b = run(r);
        assert_eq!(a.0, b.0);
        assert_eq!(a.1, b.1, "Read_FileMon result differs");
        assert_eq!(a.2, b.2, "file_queue differs");
        assert!(a.1.is_none(), "timeout must expire with NULL");
    }
}

// ===========================================================================
// Row 30 — syscheck integrity line with an EMPTY filename tail
//          (`filename[strlen-1]` writes one byte before the block)
// ===========================================================================

#[test]
fn err_30_syscheck_empty_filename() {
    // exactly the 33-byte prefix and nothing else
    let b = b"** Alert 1.2: mail - syscheck\n\
              2006 Apr 13 16:15:17 /loc\n\
              Integrity checksum changed for: '\n"
        .to_vec();
    diff_get_alert_data("syscheck-empty", &b, 0, &[Kind::File], 3);
    diff_get_alert_data("syscheck-empty", &b, MAIL, &[Kind::File], 3);

    // one-character tail: filename becomes "" after dropping the last byte
    for tail in [&b"x"[..], &b"'"[..], &b" "[..]] {
        let mut v = b"** Alert 1.2: mail - syscheck\n2006 Apr 13 16:15:17 /loc\n".to_vec();
        v.extend_from_slice(b"Integrity checksum changed for: '");
        v.extend_from_slice(tail);
        v.push(b'\n');
        diff_get_alert_data("syscheck-1char", &v, 0, &[Kind::File], 3);
    }
}

// ===========================================================================
// Row 31 — merror with a NULL template
// ===========================================================================

#[test]
fn err_31_merror_null_template() {
    let name = CString::new("some/file").unwrap();
    let msg = CString::new("Illegal seek").unwrap();
    let np = name.as_ptr() as usize;
    let mp = msg.as_ptr() as usize;
    diff_child("merror(NULL,...)", move |api| unsafe {
        (api.merror)(
            std::ptr::null(),
            np as *const c_char,
            29,
            mp as *const c_char,
        );
        // if snprintf survived a NULL format we still want a distinct status
        _exit(77);
    });
}

// ===========================================================================
// Row 32 — merror output longer than the 256-byte buffer is truncated
// ===========================================================================

#[test]
fn err_32_merror_truncation() {
    let (c, r) = apis();
    let templates = [
        "(1116): Could not set position in file '%s' due to [(%d)-(%s)].",
        "(1118): Could not retrieve information of file '%s' due to [(%d)-(%s)].",
    ];
    let mut rng = Rng::new(0x3232_2024);
    for t in templates {
        let tc = CString::new(t).unwrap();
        for len in [200usize, 240, 250, 255, 256, 257, 300, 1024, 4096] {
            let mut name = vec![b'N'; len];
            name.push(0);
            let mut msg = rng.token(len.min(600));
            msg.push(0);
            let call = |api: &Api| {
                capture_stderr(|| unsafe {
                    (api.merror)(
                        tc.as_ptr(),
                        name.as_ptr() as *const c_char,
                        -12345,
                        msg.as_ptr() as *const c_char,
                    );
                })
            };
            let ec = call(c);
            let er = call(r);
            assert_eq!(
                ec,
                er,
                "merror truncation differs at len={len}\n C={:?}\n R={:?}",
                String::from_utf8_lossy(&ec),
                String::from_utf8_lossy(&er)
            );
            // 255 chars of payload + the '\n' fprintf adds
            assert!(ec.len() <= 256, "buffer was not truncated: {}", ec.len());
        }
    }
}

// ===========================================================================
// Row 33 — out-of-range "enum" ints for GetAlertData's `flag`
// ===========================================================================

#[test]
fn err_33_getalertdata_out_of_range_flags() {
    let mut rng = Rng::new(0x3333_2024);
    let flags = [
        -1,
        i32::MIN,
        i32::MAX,
        0x20,
        0x40,
        0xFFFF,
        0x7FFF_FFFF,
        -0x8000_0000,
        0x1F,
        0xFFFF_FFF0u32 as i32,
        1 << 30,
        (1u32 << 31).wrapping_sub(1) as i32,
    ];
    for i in 0..30 {
        let mut bytes = Vec::new();
        rand_alert(&mut rng, i % 2 == 0).render(&mut bytes);
        rand_alert(&mut rng, i % 2 == 1).render(&mut bytes);
        for f in flags {
            diff_get_alert_data(&format!("oor-flag{f:#x}"), &bytes, f, &[Kind::File], 4);
        }
        // fully random ints too
        for _ in 0..8 {
            let f = rng.i32_any();
            diff_get_alert_data(&format!("rand-flag{f:#x}"), &bytes, f, &[Kind::File], 4);
        }
    }
}

// ===========================================================================
// Row 34 — out-of-range ints for Init_FileQueue's / driver's `flags`
// ===========================================================================

#[test]
fn err_34_init_out_of_range_flags() {
    let s = Scratch::new("err34");
    let mut rng = Rng::new(0x3434_2024);
    s.write(ALERTS, b"** Alert 1.2: mail - g\n2006 Apr 13 16:15:17 /loc\n");
    s.write(STDIN_NAME, b"** Alert 9.9: mail - h\n2007 May 14 01:02:03 /l2\n");
    s.write("input.log", b"** Alert 5.5: mail - i\n2008 Jun 15 02:03:04 /l3\n");
    let flags = [
        -1,
        i32::MIN,
        i32::MAX,
        0x20,
        0x40,
        0xFFFF,
        0xFFFF_FFF0u32 as i32,
        0x1F,
        1 << 30,
        -2,
        -16,
        -17,
    ];
    for f in flags {
        // Init_FileQueue: the raw value must land in fileq->flags verbatim and
        // only bits 2 and 4 may change behaviour.
        let preset = if f & FP_SET != 0 {
            Some("input.log")
        } else {
            None
        };
        diff_init_err(
            &format!("init-oor{f:#x}"),
            FileQueue::zeroed(),
            &tm_of(11, 8, 108),
            f,
            || {
                preset.map(|p| {
                    let h = unsafe { fopen_str(p, "r") };
                    assert!(!h.is_null());
                    h
                })
            },
            None,
        );
    }
    // driver with the same values (timeout 0, both candidate files present)
    let (c, r) = apis();
    for f in flags.iter().copied().chain((0..12).map(|_| rng.i32_any())) {
        unsafe {
            let mut pc = std::ptr::null_mut();
            let ec = capture_stderr(|| pc = (c.driver)(11, 8, 108, 0, f));
            let sc = take_alert(c, pc);
            let mut pr = std::ptr::null_mut();
            let er = capture_stderr(|| pr = (r.driver)(11, 8, 108, 0, f));
            let sr = take_alert(r, pr);
            assert_eq!(sc, sr, "driver flags={f:#x} differs");
            assert_eq!(ec, er, "driver flags={f:#x} stderr differs");
        }
    }
}

// ===========================================================================
// Row 35 — oversized line (past the OS_MAXSTR fgets bound)
// ===========================================================================

#[test]
fn err_35_oversized_line() {
    let mut rng = Rng::new(0x3535_2024);
    for &n in &[1023usize, 1024, 1025, 2048, 8192, 65536] {
        // a single gigantic header line
        let mut b = b"** Alert 1.2: mail - g".to_vec();
        while b.len() < n {
            b.push(b'q');
        }
        b.push(b'\n');
        b.extend_from_slice(b"2006 Apr 13 16:15:17 /loc\n");
        diff_get_alert_data(&format!("huge-header{n}"), &b, 0, &[Kind::File], 4);

        // a gigantic body line inside a valid alert
        let mut v = b"** Alert 1.2: mail - g\n2006 Apr 13 16:15:17 /loc\n".to_vec();
        v.extend_from_slice(&rng.token(n));
        v.push(b'\n');
        v.extend_from_slice(b"Src IP: 1.2.3.4\n");
        diff_get_alert_data(&format!("huge-body{n}"), &v, 0, &[Kind::File], 4);

        // a gigantic date/location line
        let mut w = b"** Alert 1.2: mail - g\n".to_vec();
        let mut d = b"2006 Apr 13 16:15:17 ".to_vec();
        while d.len() < n {
            d.push(b'L');
        }
        w.extend_from_slice(&d);
        w.push(b'\n');
        diff_get_alert_data(&format!("huge-date{n}"), &w, 0, &[Kind::File], 4);
    }
}

// ===========================================================================
// Row 38 — z == 0: alertid becomes "" (not NULL)
// ===========================================================================

#[test]
fn err_38_zero_length_alertid() {
    for raw in [
        &b"** Alert : mail - g\n2006 Apr 13 16:15:17 /loc\n"[..],
        &b"** Alert :: mail - g\n2006 Apr 13 16:15:17 /loc\n"[..],
        &b"** Alert : - g\n2006 Apr 13 16:15:17 /loc\n"[..],
        &b"** Alert :x y - g\n2006 Apr 13 16:15:17 /loc\n"[..],
    ] {
        for flag in [0, MAIL] {
            diff_get_alert_data("zero-alertid", raw, flag, &[Kind::File], 3);
        }
    }
    // and prove the C really produces "" rather than NULL
    let (c, _) = apis();
    unsafe {
        let f = file_stream(
            tmp_dir(),
            "z.log",
            b"** Alert : mail - g\n2006 Apr 13 16:15:17 /loc\n",
        );
        let p = (c.GetAlertData)(0, f);
        let snap = take_alert(c, p).expect("C must accept this alert");
        fclose(f);
        assert_eq!(
            snap.alertid.as_deref(),
            Some(&b""[..]),
            "expected an empty (not NULL) alertid"
        );
    }
}

// ===========================================================================
// Row 39 — atoi extremes reinterpreted through `unsigned int` fields
// ===========================================================================

#[test]
fn err_39_atoi_extremes() {
    let nums: [&str; 14] = [
        "-1",
        "-2147483648",
        "-2147483649",
        "2147483648",
        "4294967295",
        "4294967296",
        "9223372036854775807",
        "9223372036854775808",
        "-9223372036854775808",
        "-9223372036854775809",
        "99999999999999999999",
        "-99999999999999999999",
        "0000000000000000000005",
        "  -0",
    ];
    for r in nums {
        for l in nums {
            let mut b = b"** Alert 1.2: mail - g\n2006 Apr 13 16:15:17 /loc\n".to_vec();
            b.extend_from_slice(b"Rule: ");
            b.extend_from_slice(r.as_bytes());
            b.extend_from_slice(b" (level) ");
            b.extend_from_slice(l.as_bytes());
            b.extend_from_slice(b" -> 'c'\n");
            b.extend_from_slice(b"Src Port: ");
            b.extend_from_slice(r.as_bytes());
            b.push(b'\n');
            b.extend_from_slice(b"Dst Port: ");
            b.extend_from_slice(l.as_bytes());
            b.push(b'\n');
            diff_get_alert_data("atoi-extreme", &b, 0, &[Kind::File], 2);
        }
    }
}

// ===========================================================================
// Row 40 — FP_SET | READ_ALL with a NULL fp: Handle_Queue returns 1 and copies
//          the STALE f_status.st_mtime into last_change
// ===========================================================================

#[test]
fn err_40_fp_set_read_all_null_fp() {
    let s = Scratch::new("err40");
    s.write(ALERTS, b"must not be opened\n");
    s.write(STDIN_NAME, b"nor this\n");
    // A zeroed queue gives last_change == 0; a dirtied one proves the stale
    // f_status really is what gets copied.
    for fill in [0x00u8, 0x01, 0x41, 0xAB, 0xFF] {
        for flags in [FP_SET | READ_ALL, FP_SET | READ_ALL | MAIL] {
            diff_init_err(
                &format!("stale-fstatus{fill:#02x}"),
                FileQueue::dirty(fill),
                &tm_of(12, 9, 109),
                flags,
                || None,
                Some(0),
            );
        }
    }
}

// ===========================================================================
// Extra generic boundaries required by the task, beyond the table
// ===========================================================================

/// Zero and oversized lengths for the allocator wrappers.
#[test]
fn err_boundary_alloc_zero_and_huge() {
    let (c, r) = apis();
    unsafe {
        for (n, sz) in [(0usize, 0usize), (0, 1), (1, 0), (1, 1)] {
            let pc = (c.os_calloc)(n, sz);
            let pr = (r.os_calloc)(n, sz);
            assert_eq!(pc.is_null(), pr.is_null());
            if !pc.is_null() {
                free(pc);
            }
            if !pr.is_null() {
                free(pr);
            }
        }
        // os_strdup("")
        let empty = CString::new("").unwrap();
        let pc = (c.os_strdup)(empty.as_ptr());
        let pr = (r.os_strdup)(empty.as_ptr());
        assert_eq!(strlen(pc), 0);
        assert_eq!(strlen(pr), 0);
        free(pc as *mut c_void);
        free(pr as *mut c_void);
    }
}

/// `GetAlertData` on a stream already positioned at EOF, and on a zero-length
/// stream of each kind.
#[test]
fn err_boundary_empty_streams() {
    let (c, r) = apis();
    unsafe {
        for kind in [0, 1, 2] {
            let mk = || match kind {
                0 => file_stream(tmp_dir(), "empty.log", b""),
                1 => pipe_stream(b""),
                _ => mem_stream(b"\0"),
            };
            let fc = mk();
            let pc = (c.GetAlertData)(0, fc);
            let sc = take_alert(c, pc);
            let stc = stream_state(fc);
            fclose(fc);
            let fr = mk();
            let pr = (r.GetAlertData)(0, fr);
            let sr = take_alert(r, pr);
            let str_ = stream_state(fr);
            fclose(fr);
            assert_eq!(sc, sr, "empty stream kind={kind} differs");
            assert!(sc.is_none());
            if kind != 1 {
                assert_eq!(stc, str_, "empty stream kind={kind} state differs");
            }
        }
    }
}

/// `timeout` boundary values for `Read_FileMon` / `driver` on a file that DOES
/// yield an alert, so no `file_sleep` is reachable.
#[test]
fn err_boundary_timeout_values() {
    let s = Scratch::new("errtimeout");
    let mut rng = Rng::new(0x7777_2024);
    let mut v = Vec::new();
    for i in 0..3 {
        rand_alert(&mut rng, i % 2 == 0).render(&mut v);
    }
    s.write(ALERTS, &v);
    let (c, r) = apis();
    for t in [0u32, 1, 2, 1000, u32::MAX] {
        unsafe {
            let pc = (c.driver)(1, 0, 100, t, READ_ALL);
            let sc = take_alert(c, pc);
            let pr = (r.driver)(1, 0, 100, t, READ_ALL);
            let sr = take_alert(r, pr);
            assert_eq!(sc, sr, "driver timeout={t} differs");
            assert!(sc.is_some(), "the first alert must be returned immediately");
        }
    }
}

/// `tm_mday` extremes (never used as an index, so simply pass-through).
#[test]
fn err_boundary_tm_mday_extremes() {
    let s = Scratch::new("errmday");
    s.write(ALERTS, b"** Alert 1.2: mail - g\n2006 Apr 13 16:15:17 /loc\n");
    for d in [i32::MIN, i32::MIN + 1, -1, 0, 1, 31, 32, i32::MAX - 1, i32::MAX] {
        diff_init_err(
            &format!("mday={d}"),
            FileQueue::zeroed(),
            &tm_of(d, 5, 110),
            READ_ALL,
            || None,
            Some(0),
        );
    }
}
