//! ERRORS.md rows E15–E27: every rejection path inside `GetAlertData`
//! (`c_src/src/read-alert.c`), differentially tested C vs Rust.

mod common;

use common::*;
use std::ffi::c_int;

const MAIL: c_int = 0x001;

/// E15 — `_r == 2`, a second `** Alert` header, but the stream cannot seek back.
#[test]
fn e15_second_alert_unseekable() {
    let mut content = Vec::new();
    content.extend_from_slice(MINIMAL.as_bytes());
    content.extend_from_slice(b"** Alert 1461102541.9999: mail - syslog,\n");
    content.extend_from_slice(b"2016 Apr 19 20:30:00 myhost->/var/log/messages\n");

    // Sanity: on a seekable stream the same input returns the FIRST alert.
    let seekable = {
        let (c, _) = libs();
        gad_on_file(c, 0, &content, 0)
    };
    assert!(
        seekable.alert.is_some(),
        "precondition: seekable stream must yield the first alert"
    );

    // On a pipe, fseek(-strlen, SEEK_CUR) fails => l_error => NULL.
    let mk = {
        let content = content.clone();
        move || unseekable_stream(&content)
    };
    let out = assert_stream_eq(0, &mk, "E15 second alert on unseekable stream");
    assert!(
        out.alert.is_none(),
        "E15 must reject: fseek back failed, so GetAlertData returns NULL (got {:?})",
        out.alert
    );
}

/// E16 — `** Alert` line with no `:` after `str + 9` → `continue`, `_r` stays 0.
#[test]
fn e16_alert_no_colon() {
    let cases: &[&[u8]] = &[
        b"** Alert 12345 mail - grp\n2016 Apr 19 20:29:00 h->/l\nRule: 1 (level 2) -> 'x'\n",
        b"** Alert\n",
        b"** Alert \n",
        b"** Alert nocolonatall\n",
        b"** Alert 999 mail - syscheck\n",
    ];
    for (i, c) in cases.iter().enumerate() {
        assert_gad_eq(0, c, 0, &format!("E16 no colon #{i}"));
        assert_gad_eq(MAIL, c, 0, &format!("E16 no colon #{i} mail"));
        // The C drops the header entirely: _r never leaves 0, so EOF => NULL.
        let (cl, _) = libs();
        assert!(
            gad_on_file(cl, 0, c, 0).alert.is_none(),
            "E16 #{i}: header without ':' must be dropped"
        );
    }
}

/// E17 — `** Alert` line with a `:` but no space → `continue` after alertid was
/// already written.
#[test]
fn e17_alert_no_space() {
    let cases: &[&[u8]] = &[
        b"** Alert 12345:foo\n",
        b"** Alert :\n",
        b"** Alert a:b:c\n",
        b"** Alert 12345:foo\n2016 Apr 19 20:29:00 h->/l\nRule: 1 (level 2) -> 'x'\n",
    ];
    for (i, c) in cases.iter().enumerate() {
        assert_gad_eq(0, c, 0, &format!("E17 no space #{i}"));
        assert_gad_eq(MAIL, c, 0, &format!("E17 no space #{i} mail"));
        let (cl, _) = libs();
        assert!(
            gad_on_file(cl, 0, c, 0).alert.is_none(),
            "E17 #{i}: header without a space must be dropped"
        );
    }
}

/// E18 — `CRALERT_MAIL_SET` and the token after the first space is not `mail`.
#[test]
fn e18_mail_filter_rejects() {
    let mut content = Vec::new();
    content.extend_from_slice(b"** Alert 1461102540.1234: no-mail - syslog,errors,\n");
    content.extend_from_slice(b"2016 Apr 19 20:29:00 myhost->/var/log/messages\n");
    content.extend_from_slice(b"Rule: 1002 (level 7) -> 'Something.'\n");

    // Without the flag the alert parses; with the flag the header is dropped.
    let (c, _) = libs();
    assert!(gad_on_file(c, 0, &content, 0).alert.is_some());
    assert!(
        gad_on_file(c, MAIL, &content, 0).alert.is_none(),
        "E18: CRALERT_MAIL_SET must reject a non-'mail' header"
    );

    for tok in [
        "no-mail", "MAIL", "mai", "maild", "e-mail", "", "mailx", "m",
    ] {
        let mut v = Vec::new();
        v.extend_from_slice(format!("** Alert 1461102540.1234: {tok} - syslog,\n").as_bytes());
        v.extend_from_slice(b"2016 Apr 19 20:29:00 myhost->/var/log/messages\n");
        v.extend_from_slice(b"Rule: 1002 (level 7) -> 'Something.'\n");
        assert_gad_eq(MAIL, &v, 0, &format!("E18 token {tok:?} with MAIL_SET"));
        assert_gad_eq(0, &v, 0, &format!("E18 token {tok:?} without MAIL_SET"));
    }
}

/// E19 — lines before the first `** Alert` are silently skipped (`_r < 1`).
#[test]
fn e19_leading_garbage() {
    let mut content = Vec::new();
    content.extend_from_slice(b"garbage line one\n");
    content.extend_from_slice(b"Rule: 1 (level 2) -> 'not in an alert'\n");
    content.extend_from_slice(b"Src IP: 1.2.3.4\n");
    content.extend_from_slice(b"\n");
    content.extend_from_slice(MINIMAL.as_bytes());
    assert_gad_eq(0, &content, 0, "E19 leading garbage then a real alert");

    // Garbage only => NULL.
    let only = b"garbage\nmore garbage\nRule: 5 (level 9) -> 'x'\n";
    assert_gad_eq(0, only, 0, "E19 garbage only");
    let (c, _) = libs();
    assert!(gad_on_file(c, 0, only, 0).alert.is_none());
}

/// E20 — `_r == 1` date/location line: contains `:` but no space at/after it.
///
/// The C reports this with `perror`, whose output includes `strerror(errno)`.
/// Neither implementation sets `errno` on this path, so the harness pins the
/// ambient value and checks several of them: that is exactly what proves the
/// Rust does not clobber `errno` on the way to the `perror` call.
#[test]
fn e20_colon_without_space() {
    let cases: &[&[u8]] = &[
        b"** Alert 1.2: mail - g\n2016:04\n",
        b"** Alert 1.2: mail - g\nabc:def\n",
        b"** Alert 1.2: mail - g\n:\n",
        b"** Alert 1.2: mail - g\ntrailing:colon\nRule: 1 (level 2) -> 'x'\n",
    ];
    // 0 = "Success", 2 = ENOENT, 22 = EINVAL
    for preset in [0, 2, 22] {
        for (i, c) in cases.iter().enumerate() {
            let g = world();
            set_preset_errno(preset);
            let (cl, rl) = libs();
            let (ca, cerr) = capture_stderr(|| gad_on_file(cl, 0, c, 0));
            let (ra, rerr) = capture_stderr(|| gad_on_file(rl, 0, c, 0));
            set_preset_errno(0);
            drop(g);
            assert_eq!(ca, ra, "E20 #{i} errno={preset} outcome differs");
            assert_eq!(
                String::from_utf8_lossy(&cerr),
                String::from_utf8_lossy(&rerr),
                "E20 #{i} errno={preset} stderr differs"
            );
            assert!(ca.alert.is_none(), "E20 #{i} must reject");
            assert!(
                cerr.starts_with(b"date of location not NULL"),
                "E20 #{i} expected perror message, got {:?}",
                String::from_utf8_lossy(&cerr)
            );
        }
    }
}

/// E21 — `_r == 1` date/location line with no `:` at all → `p == NULL`.
#[test]
fn e21_no_colon_in_dateline() {
    let cases: &[&[u8]] = &[
        b"** Alert 1.2: mail - g\nno colon here\n",
        b"** Alert 1.2: mail - g\n\n",
        b"** Alert 1.2: mail - g\nx\nRule: 1 (level 2) -> 'y'\n",
    ];
    for preset in [0, 2, 22] {
        for (i, c) in cases.iter().enumerate() {
            let g = world();
            set_preset_errno(preset);
            let (cl, rl) = libs();
            let (ca, cerr) = capture_stderr(|| gad_on_file(cl, 0, c, 0));
            let (ra, rerr) = capture_stderr(|| gad_on_file(rl, 0, c, 0));
            set_preset_errno(0);
            drop(g);
            assert_eq!(ca, ra, "E21 #{i} errno={preset} outcome differs");
            assert_eq!(
                String::from_utf8_lossy(&cerr),
                String::from_utf8_lossy(&rerr),
                "E21 #{i} errno={preset} stderr differs"
            );
            assert!(ca.alert.is_none(), "E21 #{i} must reject");
            assert!(
                cerr.starts_with(b"date or location not NULL or p is NULL"),
                "E21 #{i} expected perror message, got {:?}",
                String::from_utf8_lossy(&cerr)
            );
        }
    }
}

/// E22 — `Rule: ` line with fewer than two spaces after `str + 6`.
#[test]
fn e22_rule_too_few_spaces() {
    let head = "** Alert 1.2: mail - g\n2016 Apr 19 20:29:00 h->/l\n";
    for rule in [
        "Rule: 1002\n",
        "Rule: \n",
        "Rule: x\n",
        "Rule: 1002 x\n",
        "Rule: 1002 (level\n",
        "Rule: 99999999999999999999 a\n",
    ] {
        let content = format!("{head}{rule}");
        assert_gad_eq(0, content.as_bytes(), 0, &format!("E22 {rule:?}"));
        let (c, _) = libs();
        assert!(
            gad_on_file(c, 0, content.as_bytes(), 0).alert.is_none(),
            "E22 {rule:?} must reject"
        );
    }
}

/// E23 — `Rule: ` line with two spaces but no `'`.
#[test]
fn e23_rule_no_quote() {
    let head = "** Alert 1.2: mail - g\n2016 Apr 19 20:29:00 h->/l\n";
    for rule in [
        "Rule: 1002 a b\n",
        "Rule: 1002 (level 7) -> no quote here\n",
        "Rule: 0 0 0\n",
    ] {
        let content = format!("{head}{rule}");
        assert_gad_eq(0, content.as_bytes(), 0, &format!("E23 {rule:?}"));
        let (c, _) = libs();
        assert!(
            gad_on_file(c, 0, content.as_bytes(), 0).alert.is_none(),
            "E23 {rule:?} must reject"
        );
    }
}

/// E24 — opening `'` but no closing `'` (`strrchr(comment, '\'')` NULL).
#[test]
fn e24_rule_unclosed_quote() {
    let head = "** Alert 1.2: mail - g\n2016 Apr 19 20:29:00 h->/l\n";
    for rule in [
        "Rule: 1002 a b 'unclosed\n",
        "Rule: 1002 (level 7) -> 'no end\n",
        // opening quote is the LAST character => comment == "" => strrchr NULL
        "Rule: 1002 a b '\n",
    ] {
        let content = format!("{head}{rule}");
        assert_gad_eq(0, content.as_bytes(), 0, &format!("E24 {rule:?}"));
        let (c, _) = libs();
        assert!(
            gad_on_file(c, 0, content.as_bytes(), 0).alert.is_none(),
            "E24 {rule:?} must reject"
        );
    }
    // Two quotes: accepted (the closing one is found).
    let ok = format!("{head}Rule: 1002 a b 'x'\n");
    let (c, _) = libs();
    assert!(gad_on_file(c, 0, ok.as_bytes(), 0).alert.is_some());
    assert_gad_eq(0, ok.as_bytes(), 0, "E24 control: closed quote accepted");
}

/// E25 — EOF reached while `_r != 2`.
#[test]
fn e25_eof_r_not_two() {
    let cases: &[(&str, &[u8])] = &[
        ("empty file (G3)", b""),
        ("newline only", b"\n"),
        ("header only, _r == 1", b"** Alert 1.2: mail - g\n"),
        ("header, no newline", b"** Alert 1.2: mail - g"),
        ("no alert at all", b"hello\nworld\n"),
    ];
    for (what, c) in cases {
        assert_gad_eq(0, c, 0, &format!("E25 {what}"));
        assert_gad_eq(MAIL, c, 0, &format!("E25 {what} mail"));
        let (cl, _) = libs();
        let out = gad_on_file(cl, 0, c, 0);
        assert!(out.alert.is_none(), "E25 {what} must reject");
        // The C's `l_error:` block calls clearerr(fp), so BOTH the EOF and the
        // error indicator are wiped before returning. assert_gad_eq already
        // compared these; pin the expected value explicitly.
        assert_eq!(out.feof, 0, "E25 {what}: clearerr() must have wiped EOF");
        assert_eq!(out.ferror, 0, "E25 {what}: clearerr() must have wiped error");
    }
}

/// E26 — `fgets` returns NULL because of a read *error* while `_r == 2`:
/// a complete alert was parsed, yet `feof()` is false so the C still errors out.
#[test]
fn e26_read_error_not_eof() {
    let mut content = Vec::new();
    content.extend_from_slice(b"x\n"); // consumed to prime the stdio buffer
    content.extend_from_slice(MINIMAL.as_bytes());

    // Independently confirm the stream really fails with an ERROR (not EOF)
    // once the pre-filled stdio buffer is exhausted.
    unsafe {
        let fp = error_stream(&content);
        let mut buf = [0u8; 1025];
        let mut lines = 0;
        while !fgets_raw(buf.as_mut_ptr() as *mut std::ffi::c_char, 1024, fp).is_null() {
            lines += 1;
            assert!(lines < 100);
        }
        assert!(lines >= 4, "buffered lines should be readable, got {lines}");
        assert_eq!(feof(fp), 0, "must NOT be at EOF");
        assert_ne!(ferror(fp), 0, "must be in error state");
        fclose(fp);
    }

    let mk = {
        let content = content.clone();
        move || error_stream(&content)
    };
    let out = assert_stream_eq(0, &mk, "E26 read error with _r == 2");
    assert!(
        out.alert.is_none(),
        "E26: !feof means the parsed alert is discarded even though _r == 2"
    );
    // clearerr() ran, so both indicators are clear on return.
    assert_eq!(out.feof, 0);
    assert_eq!(out.ferror, 0);

    // Control: the same bytes on a normal file DO produce an alert.
    let (c, _) = libs();
    assert!(gad_on_file(c, 0, &content, 0).alert.is_some());
}

/// E27 — the stream is already positioned at EOF (what `Handle_Queue` leaves
/// behind when `CRALERT_READ_ALL` is clear).
#[test]
fn e27_stream_at_eof() {
    for content in [MINIMAL.as_bytes(), b"", b"\n\n\n"] {
        let len = content.len() as i64;
        assert_gad_eq(0, content, len, "E27 start at EOF");
        assert_gad_eq(MAIL, content, len, "E27 start at EOF mail");
        let (c, _) = libs();
        assert!(
            gad_on_file(c, 0, content, len).alert.is_none(),
            "E27: starting at EOF must yield NULL"
        );
    }
}

/// Control for E15: on a *seekable* stream, a second `** Alert` makes the C
/// seek back so the next call resumes exactly there.
#[test]
fn e15_control_seek_back_offset() {
    let mut content = Vec::new();
    content.extend_from_slice(MINIMAL.as_bytes());
    let second = b"** Alert 1461102541.9999: mail - syslog,\n";
    let boundary = content.len() as i64;
    content.extend_from_slice(second);
    content.extend_from_slice(b"2016 Apr 19 20:30:00 myhost->/var/log/messages\n");
    content.extend_from_slice(b"Rule: 5 (level 9) -> 'Second.'\n");

    let (c, r) = libs();
    let a = gad_on_file(c, 0, &content, 0);
    let b = gad_on_file(r, 0, &content, 0);
    assert_eq!(a, b);
    assert_eq!(
        a.ftell, boundary,
        "the C seeks back to the start of the second header"
    );
    assert_drain_eq(0, &content, 8, "E15 control: full drain of two alerts");
}
