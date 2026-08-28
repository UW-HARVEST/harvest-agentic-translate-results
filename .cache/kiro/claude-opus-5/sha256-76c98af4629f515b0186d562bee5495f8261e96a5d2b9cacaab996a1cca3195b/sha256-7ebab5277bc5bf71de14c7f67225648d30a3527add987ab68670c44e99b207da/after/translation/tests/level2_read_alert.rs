//! Level 2: `GetAlertData` / `FreeAlertData` from `src/read-alert.c`.
//!
//! Every case is driven through the exported symbols of both shared objects and
//! compared field-by-field, together with the resulting `FILE*` state (position,
//! EOF and error indicators) and everything written to stderr.

mod common;

use common::*;
use std::io::{Read, Seek, SeekFrom};
use std::os::raw::c_int;
use std::path::PathBuf;

/// One `GetAlertData` call: the alert it produced plus the stream state left
/// behind.
#[derive(Debug, PartialEq, Eq, Clone)]
struct Step {
    alert: Option<AlertSnap>,
    stream: StreamSnap,
}

/// Drains a file with repeated `GetAlertData` calls, exactly like a consumer
/// looping over a log, and records every observable result.
unsafe fn drain(imp: &Impl, path: &PathBuf, flag: c_int, max_calls: usize) -> (Vec<Step>, Vec<u8>) {
    let mut steps = Vec::new();
    let out = capture_stderr(|| {
        let fp = fopen(path, b"r");
        for _ in 0..max_calls {
            // Deterministic errno so any `perror` output is reproducible.
            *libc::__errno_location() = 0;
            let al = (imp.GetAlertData)(flag, fp);
            let snap = snap_alert(al);
            let done = snap.is_none();
            steps.push(Step {
                alert: snap,
                stream: snap_stream(fp),
            });
            if !al.is_null() {
                (imp.FreeAlertData)(al);
            }
            if done {
                break;
            }
        }
        libc::fclose(fp);
    });
    (steps, out)
}

/// Captures whatever is written to fd 2 while `f` runs.
fn capture_stderr<F: FnOnce()>(f: F) -> Vec<u8> {
    unsafe {
        let path = std::env::temp_dir().join(format!(
            "c2rust-stderr2-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let mut tmp = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .expect("open capture file");
        use std::os::fd::AsRawFd;
        let cap_fd = tmp.as_raw_fd();

        libc::fflush(std::ptr::null_mut());
        let saved = libc::dup(2);
        assert!(saved >= 0);
        assert!(libc::dup2(cap_fd, 2) >= 0);

        f();

        libc::fflush(std::ptr::null_mut());
        libc::dup2(saved, 2);
        libc::close(saved);

        tmp.seek(SeekFrom::Start(0)).expect("seek");
        let mut out = Vec::new();
        tmp.read_to_end(&mut out).expect("read");
        let _ = std::fs::remove_file(&path);
        out
    }
}

fn compare_case(tag: &str, content: &[u8], flag: c_int) {
    let p = pair();
    let dir = TempDir::new("getalert");
    let path = dir.file("alerts.log", content);

    let _g = lock();
    let (steps_c, err_c) = unsafe { drain(&p.c, &path, flag, 12) };
    let (steps_rs, err_rs) = unsafe { drain(&p.rs, &path, flag, 12) };

    assert_eq!(
        steps_c.len(),
        steps_rs.len(),
        "[{tag} flag={flag:#x}] different number of GetAlertData results\nC:    {steps_c:#?}\nRust: {steps_rs:#?}"
    );
    for (i, (a, b)) in steps_c.iter().zip(steps_rs.iter()).enumerate() {
        assert_eq!(
            a, b,
            "[{tag} flag={flag:#x}] call #{i} differs\nC:    {a:#?}\nRust: {b:#?}"
        );
    }
    assert_eq!(
        String::from_utf8_lossy(&err_c),
        String::from_utf8_lossy(&err_rs),
        "[{tag} flag={flag:#x}] stderr differs"
    );
}

/// Runs a case under every flag combination that `GetAlertData` can observe.
fn compare_all_flags(tag: &str, content: &[u8]) {
    for flag in [
        0,
        CRALERT_MAIL_SET,
        CRALERT_EXEC_SET,
        CRALERT_READ_ALL,
        CRALERT_READ_FAILED,
        CRALERT_FP_SET,
        CRALERT_MAIL_SET | CRALERT_READ_ALL | CRALERT_FP_SET,
        0x1f,
    ] {
        compare_case(tag, content, flag);
    }
}

const FULL_ALERT: &[u8] = b"** Alert 1755787624.1234: mail - syscheck,pci_dss_11.5,\n\
2025 Aug 21 13:27:04 (agent-01) 10.0.0.1->syscheck\n\
Rule: 550 (level 7) -> 'Integrity checksum changed.'\n\
Src IP: 192.168.1.10\n\
Src Port: 4242\n\
Dst IP: 10.1.2.3\n\
Dst Port: 80\n\
User: root\n\
Integrity checksum changed for: '/etc/passwd'\n\
Old md5sum was: aaaaaaaa\n\
New md5sum is : bbbbbbbb\n";

#[test]
fn empty_file() {
    compare_all_flags("empty", b"");
}

#[test]
fn no_alert_header() {
    compare_all_flags("no-header", b"just some text\nand another line\n");
    compare_all_flags("no-header-no-nl", b"trailing line without newline");
}

#[test]
fn canonical_full_alert() {
    compare_all_flags("full", FULL_ALERT);
}

#[test]
fn full_alert_without_trailing_newline() {
    let mut v = FULL_ALERT.to_vec();
    assert_eq!(v.pop(), Some(b'\n'));
    compare_all_flags("full-no-nl", &v);
}

#[test]
fn two_alerts_back_to_back() {
    // Exercises the `fseek(fp, -strlen(str), SEEK_CUR)` rewind that terminates
    // an alert when the next header shows up.
    let mut v = FULL_ALERT.to_vec();
    v.extend_from_slice(
        b"** Alert 1755787625.9999: mail - authentication_failed,pci_dss_10.2.4,\n\
2025 Aug 21 13:27:05 (agent-02) 10.0.0.2->/var/log/secure\n\
Rule: 5710 (level 5) -> 'Attempt to login using a non-existent user'\n\
Src IP: 203.0.113.7\n\
User: nobody\n\
Aug 21 13:27:05 host sshd[1]: Invalid user foo\n",
    );
    compare_all_flags("two-alerts", &v);
}

#[test]
fn three_alerts_mixed_validity() {
    let mut v: Vec<u8> = Vec::new();
    v.extend_from_slice(FULL_ALERT);
    // Second alert is a bare header + location, no rule.
    v.extend_from_slice(b"** Alert 2.2: mail - group_two,\n2025 Aug 21 13:30:00 somewhere\n");
    // Third alert has a broken rule line.
    v.extend_from_slice(
        b"** Alert 3.3: mail - group_three,\n\
2025 Aug 21 13:31:00 elsewhere\n\
Rule: 999\n",
    );
    compare_all_flags("three-alerts", &v);
}

#[test]
fn mail_flag_filtering() {
    // No `mail` keyword: skipped entirely when CRALERT_MAIL_SET is set.
    let content: &[u8] = b"** Alert 1755787624.1: - group1,\n\
2025 Aug 21 13:27:04 loc/one\n\
Rule: 1 (level 2) -> 'hello'\n";
    compare_all_flags("no-mail-keyword", content);

    // `mailx` starts with `mail`, so the prefix compare still succeeds.
    let content2: &[u8] = b"** Alert 1755787624.2: mailx - group2,\n\
2025 Aug 21 13:27:04 loc/two\n\
Rule: 2 (level 3) -> 'world'\n";
    compare_all_flags("mail-prefix", content2);
}

#[test]
fn malformed_headers() {
    // No ':' after the id => header skipped, _r stays 0.
    compare_all_flags("hdr-no-colon", b"** Alert nocolonhere at all\nfollow up line\n");
    // ':' present but no space afterwards => header skipped.
    compare_all_flags("hdr-no-space", b"** Alert 1234:xyz\nfollow up line\n");
    // No '-' => group left NULL but _r becomes 1.
    compare_all_flags(
        "hdr-no-dash",
        b"** Alert 1234: mail\n2025 Aug 21 13:27:04 loc\nRule: 5 (level 6) -> 'x'\n",
    );
    // Several spaces after the '-' must all be skipped.
    compare_all_flags(
        "hdr-spaces-after-dash",
        b"** Alert 1234: mail -      syscheck,extra,\n2025 Aug 21 13:27:04 loc\n",
    );
    // Empty id (':' immediately follows the space).
    compare_all_flags(
        "hdr-empty-id",
        b"** Alert : mail - grp,\n2025 Aug 21 13:27:04 loc\n",
    );
    // Two headers in a row: exercises os_realloc on a non-NULL alertid and
    // os_free on an already-set group.
    compare_all_flags(
        "hdr-twice",
        b"** Alert 1: mail - g1,\n** Alert 22: mail - g2,\n2025 Aug 21 13:27:04 loc\n",
    );
}

#[test]
fn malformed_date_location_line() {
    // No ':' anywhere => p stays NULL => error path.
    compare_all_flags(
        "date-no-colon",
        b"** Alert 1234: mail - g,\nno colon in this line\nRule: 1 (level 2) -> 'x'\n",
    );
    // ':' present but no space after it => perror + error path.
    compare_all_flags(
        "date-colon-no-space",
        b"** Alert 1234: mail - g,\n2025Aug21T13:27:04\nRule: 1 (level 2) -> 'x'\n",
    );
    // Colon is the last character.
    compare_all_flags(
        "date-trailing-colon",
        b"** Alert 1234: mail - g,\nabc:\nRule: 1 (level 2) -> 'x'\n",
    );
    // Line consisting only of a newline.
    compare_all_flags("date-blank", b"** Alert 1234: mail - g,\n\nRule: 1\n");
}

#[test]
fn malformed_rule_lines() {
    let hdr: &[u8] = b"** Alert 1234: mail - g,\n2025 Aug 21 13:27:04 loc/x\n";
    for (tag, rule) in [
        ("rule-only-number", &b"Rule: 550\n"[..]),
        ("rule-one-space", &b"Rule: 550 x\n"[..]),
        ("rule-no-quote", &b"Rule: 550 (level 7)\n"[..]),
        ("rule-open-quote-only", &b"Rule: 550 (level 7) -> 'unterminated\n"[..]),
        ("rule-empty-comment", &b"Rule: 550 (level 7) -> ''\n"[..]),
        ("rule-nonnumeric", &b"Rule: abc (level xyz) -> 'c'\n"[..]),
        ("rule-negative", &b"Rule: -5 (level -3) -> 'neg'\n"[..]),
        ("rule-huge", &b"Rule: 99999999999 (level 88888888888) -> 'big'\n"[..]),
        (
            "rule-many-quotes",
            &b"Rule: 550 (level 7) -> 'it's a 'quoted' thing'\n"[..],
        ),
        ("rule-exact-prefix", &b"Rule: \n"[..]),
    ] {
        let mut v = hdr.to_vec();
        v.extend_from_slice(rule);
        compare_all_flags(tag, &v);
    }
}

#[test]
fn repeated_fields_are_replaced() {
    let content: &[u8] = b"** Alert 1: mail - g,\n\
2025 Aug 21 13:27:04 loc\n\
Rule: 1 (level 1) -> 'first'\n\
Src IP: 1.1.1.1\n\
Src IP: 2.2.2.2\n\
Src Port: 1\n\
Src Port: 65535\n\
Dst IP: 3.3.3.3\n\
Dst IP: 4.4.4.4\n\
Dst Port: 7\n\
Dst Port: 8\n\
User: alice\n\
User: bob\n\
Rule: 2 (level 2) -> 'second'\n";
    compare_all_flags("repeated-fields", content);
}

#[test]
fn field_edge_cases() {
    let hdr: &[u8] = b"** Alert 1: mail - g,\n2025 Aug 21 13:27:04 loc\n";
    for (tag, body) in [
        ("empty-srcip", &b"Src IP: \n"[..]),
        ("empty-user", &b"User: \n"[..]),
        ("port-empty", &b"Src Port: \nDst Port: \n"[..]),
        ("port-junk", &b"Src Port: abc\nDst Port: 12abc\n"[..]),
        ("port-negative", &b"Src Port: -1\nDst Port: -32768\n"[..]),
        ("port-overflow", &b"Src Port: 99999999999\nDst Port: 4294967296\n"[..]),
        ("port-spaces", &b"Src Port:   42\n"[..]),
        // Near misses of the prefixes: must fall into the log branch.
        ("near-miss", &b"Src IP:x\nSrcPort: 1\nUser:x\nRule:1\n"[..]),
        // Case sensitivity.
        ("wrong-case", &b"src ip: 1.2.3.4\nuser: root\n"[..]),
    ] {
        let mut v = hdr.to_vec();
        v.extend_from_slice(body);
        compare_all_flags(tag, &v);
    }
}

#[test]
fn syscheck_filename_extraction() {
    let sys: &[u8] = b"** Alert 1: mail - syscheck,\n2025 Aug 21 13:27:04 loc\n";
    // Only the first log line after a syscheck header is inspected.
    compare_all_flags(
        "syscheck-match",
        b"** Alert 1: mail - syscheck,\n\
2025 Aug 21 13:27:04 loc\n\
Integrity checksum changed for: '/etc/hosts'\n\
Old md5sum was: 1\n",
    );
    // First log line is not the integrity line => issyscheck is cleared and the
    // later integrity line is ignored.
    compare_all_flags(
        "syscheck-late",
        b"** Alert 1: mail - syscheck,\n\
2025 Aug 21 13:27:04 loc\n\
some other log line\n\
Integrity checksum changed for: '/etc/hosts'\n",
    );
    // Group without `syscheck` => filename never populated.
    compare_all_flags(
        "syscheck-absent",
        b"** Alert 1: mail - authentication,\n\
2025 Aug 21 13:27:04 loc\n\
Integrity checksum changed for: '/etc/hosts'\n",
    );
    // Prefix present but only one character of payload.
    let mut v = sys.to_vec();
    v.extend_from_slice(b"Integrity checksum changed for: 'x\n");
    compare_all_flags("syscheck-one-char", &v);
    // Substring match anywhere in the group string counts.
    compare_all_flags(
        "syscheck-substring",
        b"** Alert 1: mail - pre_syscheck_post,\n\
2025 Aug 21 13:27:04 loc\n\
Integrity checksum changed for: '/a'\n",
    );
    // Prefix is one byte short of matching.
    let mut v = sys.to_vec();
    v.extend_from_slice(b"Integrity checksum changed for:'/etc/hosts'\n");
    compare_all_flags("syscheck-near-prefix", &v);
}

#[test]
fn log_limit_boundary() {
    // More than LOG_LIMIT (100) log lines: the branch stops being taken, which
    // must happen at the same line in both implementations.
    let mut v: Vec<u8> = b"** Alert 1: mail - syscheck,\n2025 Aug 21 13:27:04 loc\n".to_vec();
    for i in 0..130 {
        v.extend_from_slice(format!("log line number {i}\n").as_bytes());
    }
    compare_all_flags("log-limit", &v);
}

#[test]
fn long_lines_are_truncated_identically() {
    // fgets reads at most OS_MAXSTR-1 = 1023 bytes, so an over-long line is
    // split and the remainder is re-parsed as a fresh line.
    let mut v: Vec<u8> = b"** Alert 1: mail - g,\n2025 Aug 21 13:27:04 loc\n".to_vec();
    v.extend_from_slice(b"Rule: 42 (level 9) -> '");
    v.extend_from_slice(&vec![b'C'; 2000]);
    v.extend_from_slice(b"'\n");
    v.extend_from_slice(b"Src IP: ");
    v.extend_from_slice(&vec![b'9'; 1500]);
    v.push(b'\n');
    compare_all_flags("long-lines", &v);

    // A header line longer than the buffer.
    let mut v2: Vec<u8> = b"** Alert 1: mail - ".to_vec();
    v2.extend_from_slice(&vec![b'g'; 1400]);
    v2.extend_from_slice(b",\n2025 Aug 21 13:27:04 loc\n");
    compare_all_flags("long-header", &v2);
}

#[test]
fn short_header_after_long_line_reuses_buffer() {
    // `p = str + ALERT_BEGIN_SZ + 1` reads past the NUL for a header shorter
    // than 10 bytes; the leftover bytes come from the previous fgets, so the
    // behaviour is only well defined once the buffer has been written to.
    let mut v: Vec<u8> = Vec::new();
    v.extend_from_slice(b"padding line with plenty of characters to fill the buffer\n");
    v.extend_from_slice(b"** Alert\n");
    v.extend_from_slice(b"2025 Aug 21 13:27:04 loc\n");
    compare_all_flags("short-header", &v);
}

#[test]
fn crlf_and_embedded_specials() {
    compare_all_flags(
        "crlf",
        b"** Alert 1: mail - g,\r\n2025 Aug 21 13:27:04 loc\r\nRule: 1 (level 2) -> 'x'\r\n",
    );
    compare_all_flags(
        "tabs",
        b"** Alert 1:\tmail\t-\tg,\n2025 Aug 21 13:27:04 loc\n",
    );
    // Multiple newlines inside one fgets chunk are impossible, but a lone \n as
    // the whole alert body is not.
    compare_all_flags("only-newlines", b"\n\n\n** Alert 1: mail - g,\n\n\n");
    // High-bit bytes.
    compare_all_flags(
        "high-bytes",
        b"** Alert 1: mail - g\xc3\xa9,\n2025 Aug 21 13:27:04 lo\xffc\nUser: \xe2\x82\xac\n",
    );
}

#[test]
fn alert_begin_prefix_variants() {
    // Exactly the prefix with extra text, and near misses.
    compare_all_flags("prefix-star", b"**Alert 1: mail - g,\n2025 Aug 21 1:2:3 l\n");
    compare_all_flags("prefix-lower", b"** alert 1: mail - g,\n2025 Aug 21 1:2:3 l\n");
    compare_all_flags(
        "prefix-embedded",
        b"prefix ** Alert 1: mail - g,\n2025 Aug 21 1:2:3 l\n",
    );
}

#[test]
fn free_alert_data_cross_allocated() {
    // Build an alert_data with one implementation's helpers and release it with
    // the other's FreeAlertData; both must accept it.
    let p = pair();
    unsafe {
        for (alloc, free) in [(&p.c, &p.rs), (&p.rs, &p.c)] {
            let ad = (alloc.os_calloc)(1, std::mem::size_of::<alert_data>()) as *mut alert_data;
            let s = cstring(b"value");
            (*ad).rule = 7;
            (*ad).level = 3;
            (*ad).alertid = (alloc.os_strdup)(s.as_ptr());
            (*ad).date = (alloc.os_strdup)(s.as_ptr());
            (*ad).location = (alloc.os_strdup)(s.as_ptr());
            (*ad).comment = (alloc.os_strdup)(s.as_ptr());
            (*ad).group = (alloc.os_strdup)(s.as_ptr());
            (*ad).srcip = (alloc.os_strdup)(s.as_ptr());
            (*ad).dstip = (alloc.os_strdup)(s.as_ptr());
            (*ad).user = (alloc.os_strdup)(s.as_ptr());
            (*ad).filename = (alloc.os_strdup)(s.as_ptr());
            (free.FreeAlertData)(ad);

            // All-NULL fields must also be accepted (os_free is NULL-guarded).
            let ad = (alloc.os_calloc)(1, std::mem::size_of::<alert_data>()) as *mut alert_data;
            (free.FreeAlertData)(ad);
        }
    }
}

#[test]
fn stream_state_after_error_is_cleared() {
    // On the error path GetAlertData calls clearerr(); a following read must
    // therefore behave identically for both implementations.
    let p = pair();
    let dir = TempDir::new("clearerr");
    let path = dir.file("alerts.log", b"garbage with no alert header\n");

    let _g = lock();
    let mut results = Vec::new();
    for imp in [&p.c, &p.rs] {
        unsafe {
            let fp = fopen(&path, b"r");
            let al = (imp.GetAlertData)(0, fp);
            assert!(al.is_null());
            let after = snap_stream(fp);
            // The stream must still be usable after clearerr.
            let mut buf = [0i8; 64];
            libc::rewind(fp);
            let got = libc::fgets(buf.as_mut_ptr(), 64, fp);
            let line = if got.is_null() {
                None
            } else {
                Some(std::ffi::CStr::from_ptr(buf.as_ptr()).to_bytes().to_vec())
            };
            libc::fclose(fp);
            results.push((after, line));
        }
    }
    assert_eq!(results[0], results[1], "post-error stream state differs");
}

#[test]
fn many_alerts_streamed() {
    // A realistic multi-alert log drained in a loop.
    let mut v: Vec<u8> = Vec::new();
    for i in 1..=6 {
        v.extend_from_slice(
            format!(
                "** Alert 175578762{i}.{i}00: mail - group{i},syscheck,\n\
                 2025 Aug 2{i} 13:27:0{i} (agent-{i}) 10.0.0.{i}->/var/log/x{i}\n\
                 Rule: {}00 (level {i}) -> 'message number {i}'\n\
                 Src IP: 10.{i}.{i}.{i}\n\
                 Src Port: {}\n\
                 Dst IP: 172.16.0.{i}\n\
                 Dst Port: {}\n\
                 User: user{i}\n\
                 Integrity checksum changed for: '/etc/file{i}'\n\
                 extra log line {i}\n",
                i,
                1000 + i,
                2000 + i
            )
            .as_bytes(),
        );
    }
    compare_case("many-alerts", &v, 0);
    compare_case("many-alerts", &v, CRALERT_MAIL_SET);
}
