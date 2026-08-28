//! Phase B rows 6-19 and Phase C rows 5-16 / 30 / 33 / 34: `GetAlertData`,
//! the parsing core of the library, driven through both `.so`s.

mod common;

use common::*;
use std::ffi::c_int;

const F0: c_int = 0;

/* =================== CONFIGS row 6 =================== */

#[test]
fn cfg06_single_full_alert() {
    unsafe {
        let s = full_alert("1163181400.1", false, "ossec,authentication_success,");
        diff_get_alert_data(s.as_bytes(), F0, "full alert");
        diff_get_alert_data(s.as_bytes(), CRALERT_MAIL_SET, "full alert / mail flag");
        let m = full_alert("1163181400.2", true, "ossec,syscheck,");
        diff_get_alert_data(m.as_bytes(), F0, "mail alert");
        diff_get_alert_data(m.as_bytes(), CRALERT_MAIL_SET, "mail alert / mail flag");
    }
}

/* =================== CONFIGS row 7 =================== */

#[test]
fn cfg07_random_field_subsets() {
    let mut rng = Rng::new(0x0707);
    let lines: Vec<String> = vec![
        "Rule: 1002 (level 7) -> 'Unknown problem somewhere in the system.'".into(),
        "Rule: 0 (level 0) -> ''".into(),
        "Src IP: 10.11.12.13".into(),
        "Src IP: (none)".into(),
        "Src Port: 65535".into(),
        "Src Port: -1".into(),
        "Src Port: notanumber".into(),
        "Dst IP: ::1".into(),
        "Dst Port: 0".into(),
        "Dst Port: 99999999999999999999".into(),
        "User: (none)".into(),
        "User: ".into(),
        "Integrity checksum changed for: '/etc/passwd'".into(),
        "some arbitrary log line".into(),
        "".into(),
        "Rule".into(),
        "Src IP:".into(),
    ];
    unsafe {
        for case in 0..300 {
            let mut s = String::new();
            s.push_str(&header(
                &format!("116318{}.{}", 1000 + rng.below(9000), rng.below(100)),
                if rng.bool() { "mail" } else { "exec" },
                if rng.bool() { "syscheck," } else { "ossec,pci," },
            ));
            s.push_str("2006 Apr 13 16:15:17 host->/var/log/x\n");
            let n = rng.below(8);
            for _ in 0..n {
                s.push_str(rng.pick(&lines).as_str());
                s.push('\n');
            }
            let flag = if rng.bool() { CRALERT_MAIL_SET } else { 0 };
            diff_get_alert_data(s.as_bytes(), flag, &format!("subset case {case}"));
        }
    }
}

/* =================== CONFIGS row 8 =================== */

#[test]
fn cfg08_duplicate_fields() {
    unsafe {
        let mut s = String::new();
        s.push_str(&header("1.1", "mail", "ossec,"));
        s.push_str("2006 Apr 13 16:15:17 host->/x\n");
        s.push_str("Rule: 1 (level 1) -> 'first'\n");
        s.push_str("Rule: 2 (level 2) -> 'second'\n");
        s.push_str("Rule: 3 (level 3) -> 'third'\n");
        s.push_str("Src IP: 1.1.1.1\n");
        s.push_str("Src IP: 2.2.2.2\n");
        s.push_str("Src Port: 11\nSrc Port: 22\n");
        s.push_str("Dst IP: 3.3.3.3\nDst IP: 4.4.4.4\n");
        s.push_str("Dst Port: 33\nDst Port: 44\n");
        s.push_str("User: a\nUser: b\nUser: c\n");
        s.push_str("Integrity checksum changed for: '/one'\n");
        s.push_str("Integrity checksum changed for: '/two'\n");
        diff_get_alert_data(s.as_bytes(), F0, "duplicate fields");

        // duplicated group headers while _r == 1 (header seen twice in a row)
        let mut t = String::new();
        t.push_str(&header("2.1", "mail", "first_group,"));
        t.push_str(&header("2.2", "mail", "second_group,syscheck,"));
        t.push_str("2006 Apr 13 16:15:17 host->/x\n");
        t.push_str("Integrity checksum changed for: '/three'\n");
        diff_get_alert_data(t.as_bytes(), F0, "duplicate headers");
    }
}

/* =================== CONFIGS row 9 =================== */

#[test]
fn cfg09_multi_alert_sequence() {
    let mut rng = Rng::new(0x0909);
    unsafe {
        for n in 1..=6 {
            let mut s = String::new();
            for i in 0..n {
                s.push_str(&full_alert(
                    &format!("11631814{:02}.{}", i, i * 7),
                    i % 2 == 0,
                    if i % 3 == 0 { "syscheck," } else { "ossec," },
                ));
            }
            diff_get_alert_data(s.as_bytes(), F0, &format!("{n} alerts"));
            diff_get_alert_data(s.as_bytes(), CRALERT_MAIL_SET, &format!("{n} alerts / mail"));
        }
        for case in 0..120 {
            let n = 1 + rng.below(4);
            let mut s = String::new();
            for i in 0..n {
                s.push_str(&full_alert(
                    &format!("{}.{}", 1163181400u64 + i, rng.below(1000)),
                    rng.bool(),
                    if rng.bool() { "syscheck," } else { "ossec," },
                ));
                for _ in 0..rng.below(3) {
                    s.push_str(&String::from_utf8_lossy(&rng.token(30)));
                    s.push('\n');
                }
            }
            diff_get_alert_data(s.as_bytes(), F0, &format!("rand multi {case}"));
        }
    }
}

/* =================== CONFIGS row 10 =================== */

#[test]
fn cfg10_mail_flag_filtering() {
    unsafe {
        let mut s = String::new();
        s.push_str(&full_alert("1.1", false, "ossec,"));
        s.push_str(&full_alert("1.2", true, "ossec,"));
        s.push_str(&full_alert("1.3", false, "syscheck,"));
        s.push_str(&full_alert("1.4", true, "syscheck,"));
        for flag in [0, CRALERT_MAIL_SET, CRALERT_MAIL_SET | CRALERT_READ_ALL] {
            diff_get_alert_data(s.as_bytes(), flag, &format!("mail filter flag={flag:#x}"));
        }
        // token that merely *starts* with "mail"
        for tag in ["mail", "mailx", "mai", "maill", "MAIL", "", " mail"] {
            let mut t = String::new();
            t.push_str(&format!("** Alert 9.9: {} - ossec,\n", tag));
            t.push_str("2006 Apr 13 16:15:17 host->/x\n");
            t.push_str("Rule: 7 (level 3) -> 'c'\n");
            diff_get_alert_data(t.as_bytes(), CRALERT_MAIL_SET, &format!("tag {tag:?}"));
            diff_get_alert_data(t.as_bytes(), 0, &format!("tag {tag:?} no flag"));
        }
    }
}

/* =================== CONFIGS row 11 =================== */

#[test]
fn cfg11_random_flag_words() {
    let mut rng = Rng::new(0x1111);
    unsafe {
        let mut s = String::new();
        s.push_str(&full_alert("3.1", false, "ossec,"));
        s.push_str(&full_alert("3.2", true, "syscheck,"));
        for _ in 0..200 {
            let flag = rng.i32();
            diff_get_alert_data(s.as_bytes(), flag, &format!("flag {flag:#x}"));
        }
        for flag in [
            i32::MIN,
            i32::MAX,
            -1,
            0x20,
            0x1f,
            !CRALERT_MAIL_SET,
            i32::MIN | CRALERT_MAIL_SET,
        ] {
            diff_get_alert_data(s.as_bytes(), flag, &format!("boundary flag {flag:#x}"));
        }
    }
}

/* =================== CONFIGS row 12 =================== */

#[test]
fn cfg12_group_and_syscheck_detection() {
    unsafe {
        let variants: [&str; 14] = [
            "** Alert 1.1: mail - syscheck,\n",
            "** Alert 1.1: mail -syscheck,\n",
            "** Alert 1.1: mail -    syscheck,\n",
            "** Alert 1.1: mail -\n",
            "** Alert 1.1: mail - \n",
            "** Alert 1.1: mail\n",
            "** Alert 1.1: mail - a,b,c,\n",
            "** Alert 1.1: mail - a,syscheck\n",
            "** Alert 1.1: mail - xsyscheckx\n",
            "** Alert 1.1: mail - SYSCHECK\n",
            "** Alert 1.1: mail - -- double dash\n",
            "** Alert 1.1:-immediately\n",
            "** Alert 1.1: - nospacetag\n",
            "** Alert 1.1: mail - syscheck, - second dash\n",
        ];
        for (i, h) in variants.iter().enumerate() {
            let mut s = String::new();
            s.push_str(h);
            s.push_str("2006 Apr 13 16:15:17 host->/x\n");
            s.push_str("Integrity checksum changed for: '/etc/hosts'\n");
            s.push_str("Rule: 550 (level 7) -> 'Integrity checksum changed.'\n");
            diff_get_alert_data(s.as_bytes(), F0, &format!("group variant {i}"));
            diff_get_alert_data(s.as_bytes(), CRALERT_MAIL_SET, &format!("group variant {i} mail"));
        }
    }
}

/* =================== CONFIGS row 13 =================== */

#[test]
fn cfg13_syscheck_filename() {
    unsafe {
        let names: [&str; 10] = [
            "Integrity checksum changed for: '/etc/passwd'",
            "Integrity checksum changed for: '/etc/passwd",
            "Integrity checksum changed for: ''",
            "Integrity checksum changed for: '",
            "Integrity checksum changed for: ",
            "Integrity checksum changed for:",
            "Integrity checksum changed for: 'a'",
            "Integrity checksum changed fOr: 'x'",
            "integrity checksum changed for: 'x'",
            "Integrity checksum changed for: '/very/long/path/with spaces/and'quotes'",
        ];
        for (i, line) in names.iter().enumerate() {
            for group in ["syscheck,", "ossec,"] {
                let mut s = String::new();
                s.push_str(&format!("** Alert 5.{}: mail - {}\n", i, group));
                s.push_str("2006 Apr 13 16:15:17 host->/x\n");
                s.push_str(line);
                s.push('\n');
                s.push_str("Integrity checksum changed for: '/second'\n");
                diff_get_alert_data(s.as_bytes(), F0, &format!("integrity {i} {group}"));
            }
        }
        // issyscheck is consumed by the FIRST plain log line, so a leading
        // non-matching log line disables the filename extraction entirely.
        let mut s = String::new();
        s.push_str("** Alert 6.1: mail - syscheck,\n");
        s.push_str("2006 Apr 13 16:15:17 host->/x\n");
        s.push_str("unrelated first log line\n");
        s.push_str("Integrity checksum changed for: '/etc/shadow'\n");
        diff_get_alert_data(s.as_bytes(), F0, "integrity after other log line");

        // ... but Rule:/Src IP:/... lines do NOT consume it.
        let mut t = String::new();
        t.push_str("** Alert 6.2: mail - syscheck,\n");
        t.push_str("2006 Apr 13 16:15:17 host->/x\n");
        t.push_str("Rule: 550 (level 7) -> 'x'\n");
        t.push_str("Src IP: 1.2.3.4\n");
        t.push_str("Integrity checksum changed for: '/etc/shadow'\n");
        diff_get_alert_data(t.as_bytes(), F0, "integrity after field lines");
    }
}

/* =================== CONFIGS row 14 =================== */

#[test]
fn cfg14_no_trailing_newline() {
    unsafe {
        let base = full_alert("7.1", false, "ossec,");
        let no_nl = base.trim_end_matches('\n').to_string();
        diff_get_alert_data(no_nl.as_bytes(), F0, "no trailing newline");
        // header only, no newline
        diff_get_alert_data(b"** Alert 7.2: mail - ossec,", F0, "header only, no nl");
        // header + date, no newline
        diff_get_alert_data(
            b"** Alert 7.3: mail - ossec,\n2006 Apr 13 16:15:17 host->/x",
            F0,
            "header+date, no nl",
        );
        // Exactly "** Alert" with no newline. `p = str + ALERT_BEGIN_SZ + 1`
        // then points one past the NUL that `fgets` wrote, so the C inspects
        // uninitialised stack. It is unobservable: such a line can only be the
        // *last* one (fgets stops at '\n'), so whatever `strstr`/`strchr` find
        // inside the 1025-byte buffer, `_r` can never reach 2 and the function
        // always returns NULL with the stream at EOF. Both must agree.
        diff_get_alert_data(b"** Alert", F0, "bare header, no nl");
        diff_get_alert_data(b"** Alert\n", F0, "bare header");
        diff_get_alert_data(b"** Aler", F0, "truncated marker");
        diff_get_alert_data(b"** Alertx", F0, "9-byte marker");
        // ... and the same tail after a complete alert (the _r==2 fseek branch
        // runs before `p = str + 9`, so the first record is still returned)
        let mut u = full_alert("7.4", true, "syscheck,");
        u.push_str("** Alert");
        diff_get_alert_data(u.as_bytes(), F0, "alert then bare header");
        for tail in ["** Alert", "** Alert\n", "** Alert:", "** Alert :"] {
            let mut v = full_alert("7.5", true, "ossec,");
            v.push_str(tail);
            diff_get_alert_data(v.as_bytes(), F0, &format!("alert then {tail:?}"));
        }
        diff_get_alert_data(b"", F0, "empty");
        diff_get_alert_data(b"\n", F0, "single newline");
        diff_get_alert_data(b"\n\n\n", F0, "blank lines");
    }
}

/* =================== CONFIGS row 15 =================== */

#[test]
fn cfg15_crlf_line_endings() {
    unsafe {
        let s = full_alert("8.1", true, "syscheck,").replace('\n', "\r\n");
        diff_get_alert_data(s.as_bytes(), F0, "crlf");
        diff_get_alert_data(s.as_bytes(), CRALERT_MAIL_SET, "crlf mail");
        // lone \r
        let t = full_alert("8.2", true, "ossec,").replace('\n', "\r");
        diff_get_alert_data(t.as_bytes(), F0, "cr only");
        // mixed
        let mut u = String::new();
        u.push_str("** Alert 8.3: mail - syscheck,\r\n");
        u.push_str("2006 Apr 13 16:15:17 host->/x\n");
        u.push_str("Rule: 1 (level 1) -> 'x'\r\n");
        u.push_str("Src IP: 1.2.3.4\r\n");
        diff_get_alert_data(u.as_bytes(), F0, "mixed eol");
    }
}

/* =================== CONFIGS row 16 / ERRORS row 30 =================== */

#[test]
fn cfg16_os_maxstr_boundaries() {
    unsafe {
        for len in [
            1usize, 2, 100, 1020, 1021, 1022, 1023, 1024, 1025, 1026, 2046, 2047, 2048, 2049, 2050,
            3000,
        ] {
            // long comment
            let mut s = String::new();
            s.push_str("** Alert 9.1: mail - ossec,\n");
            s.push_str("2006 Apr 13 16:15:17 host->/x\n");
            s.push_str("Rule: 42 (level 9) -> '");
            s.push_str(&"C".repeat(len));
            s.push_str("'\n");
            diff_get_alert_data(s.as_bytes(), F0, &format!("long comment {len}"));

            // long Src IP value
            let mut t = String::new();
            t.push_str("** Alert 9.2: mail - ossec,\n");
            t.push_str("2006 Apr 13 16:15:17 host->/x\n");
            t.push_str("Src IP: ");
            t.push_str(&"S".repeat(len));
            t.push('\n');
            diff_get_alert_data(t.as_bytes(), F0, &format!("long srcip {len}"));

            // long header line
            let mut u = String::new();
            u.push_str("** Alert 9.3: mail - ");
            u.push_str(&"G".repeat(len));
            u.push('\n');
            u.push_str("2006 Apr 13 16:15:17 host->/x\n");
            diff_get_alert_data(u.as_bytes(), F0, &format!("long header {len}"));

            // long date/location line
            let mut v = String::new();
            v.push_str("** Alert 9.4: mail - ossec,\n");
            v.push_str("2006 Apr 13 16:15:17 ");
            v.push_str(&"L".repeat(len));
            v.push('\n');
            v.push_str("Rule: 1 (level 1) -> 'x'\n");
            diff_get_alert_data(v.as_bytes(), F0, &format!("long location {len}"));

            // long plain log line
            let mut w = String::new();
            w.push_str("** Alert 9.5: mail - syscheck,\n");
            w.push_str("2006 Apr 13 16:15:17 host->/x\n");
            w.push_str(&"Z".repeat(len));
            w.push('\n');
            diff_get_alert_data(w.as_bytes(), F0, &format!("long log {len}"));

            // long integrity line
            let mut x = String::new();
            x.push_str("** Alert 9.6: mail - syscheck,\n");
            x.push_str("2006 Apr 13 16:15:17 host->/x\n");
            x.push_str("Integrity checksum changed for: '");
            x.push_str(&"P".repeat(len));
            x.push_str("'\n");
            diff_get_alert_data(x.as_bytes(), F0, &format!("long integrity {len}"));
        }
    }
}

/* =================== CONFIGS row 17 =================== */

#[test]
fn cfg17_fuzz_random_streams() {
    let mut rng = Rng::new(0xF00D);
    unsafe {
        for case in 0..400 {
            let content = random_stream(&mut rng);
            let flag = match case % 4 {
                0 => 0,
                1 => CRALERT_MAIL_SET,
                2 => rng.i32(),
                _ => CRALERT_MAIL_SET | CRALERT_READ_ALL | CRALERT_FP_SET,
            };
            diff_get_alert_data(&content, flag, &format!("fuzz stream {case}"));
        }
    }
}

/* =================== CONFIGS row 18 =================== */

#[test]
fn cfg18_fuzz_random_bytes() {
    let mut rng = Rng::new(0xBEEF);
    unsafe {
        for case in 0..200 {
            let nlines = rng.below(12);
            let mut content: Vec<u8> = Vec::new();
            for _ in 0..nlines {
                if rng.below(5) == 0 {
                    content.extend_from_slice(b"** Alert ");
                }
                content.extend_from_slice(&rng.raw_line(60));
                content.push(b'\n');
            }
            diff_get_alert_data(&content, if rng.bool() { CRALERT_MAIL_SET } else { 0 }, &format!("fuzz bytes {case}"));
        }
        // pure random blobs
        for case in 0..100 {
            let n = rng.below(400) as usize;
            let content: Vec<u8> = (0..n).map(|_| {
                let b = rng.below(256) as u8;
                if b == 0 { b'0' } else { b }
            }).collect();
            diff_get_alert_data(&content, 0, &format!("blob {case}"));
        }
    }
}

/* =================== CONFIGS row 19 =================== */

#[test]
fn cfg19_preseeked_stream() {
    let _s = shared();
    let mut rng = Rng::new(0x1919);
    unsafe {
        let mut s = String::new();
        for i in 0..3 {
            s.push_str(&full_alert(&format!("10.{i}"), i % 2 == 0, "syscheck,"));
        }
        let bytes = s.as_bytes();
        for _ in 0..120 {
            let pos = rng.below(bytes.len() as u64 + 4) as i64;
            let c = drain_get_alert_data(cc(), "c", bytes, 0, 24, Some(pos));
            let r = drain_get_alert_data(rs(), "r", bytes, 0, 24, Some(pos));
            assert_eq!(c.len(), r.len(), "preseek {pos}: count");
            for (i, (a, b)) in c.iter().zip(r.iter()).enumerate() {
                assert_eq!(a.0, b.0, "preseek {pos} record {i}");
                assert_eq!(a.1, b.1, "preseek {pos} stream {i}");
            }
        }
    }
}

/* =================== CONFIGS row 41 =================== */

/// `GetAlertData` on a non-seekable stream (a pipe): with a single alert the
/// push-back `fseek` is never reached, so the record comes back normally.
#[test]
fn cfg41_nonseekable_stream() {
    let _s = shared();
    let mut rng = Rng::new(0x4141);
    unsafe {
        let mut cases: Vec<Vec<u8>> = vec![
            full_alert("22.1", true, "syscheck,").into_bytes(),
            full_alert("22.2", false, "ossec,").into_bytes(),
            b"nothing\n".to_vec(),
            b"".to_vec(),
        ];
        for _ in 0..60 {
            cases.push(random_stream(&mut rng));
        }
        for (i, content) in cases.iter().enumerate() {
            let mut per_api = Vec::new();
            for api in [cc(), rs()] {
                let mut fds = [0i32; 2];
                assert_eq!(pipe(fds.as_mut_ptr()), 0);
                if !content.is_empty() {
                    write(fds[1], content.as_ptr() as *const _, content.len());
                }
                close(fds[1]);
                let m = cpath("r");
                let fp = fdopen(fds[0], m.as_ptr());
                let mut recs = Vec::new();
                for _ in 0..4 {
                    set_errno(0);
                    let a = (api.GetAlertData)(0, fp);
                    let s = snap_alert(a);
                    if !a.is_null() {
                        (api.FreeAlertData)(a);
                    }
                    let done = s.is_none();
                    recs.push((s, feof(fp) != 0, ferror(fp) != 0));
                    if done {
                        break;
                    }
                }
                fclose(fp);
                per_api.push(recs);
            }
            assert_eq!(per_api[0], per_api[1], "pipe case {i} differs");
        }
    }
}

/* =================== CONFIGS row 42 =================== */

/// `GetAlertData` on a stream where `fgets` fails outright: a directory stream
/// (`open` succeeds on Linux, `read` fails with `EISDIR`).
#[test]
fn cfg42_directory_stream() {
    let _g = guard();
    unsafe {
        let dirname = format!("dirstream-{}", std::process::id());
        let _ = std::fs::remove_dir_all(&dirname);
        std::fs::create_dir(&dirname).unwrap();
        let mut per_api = Vec::new();
        for api in [cc(), rs()] {
            let fp = open_ro(&dirname);
            set_errno(0);
            let (a, err) = capture_stderr(|| {
                let a = (api.GetAlertData)(0, fp);
                let s = snap_alert(a);
                if !a.is_null() {
                    (api.FreeAlertData)(a);
                }
                s
            });
            per_api.push((
                a,
                feof(fp) != 0,
                ferror(fp) != 0,
                String::from_utf8_lossy(&err).to_string(),
            ));
            fclose(fp);
        }
        assert_eq!(per_api[0], per_api[1], "directory stream differs");
        assert_eq!(per_api[0].0, None);
        std::fs::remove_dir_all(&dirname).unwrap();
    }
}

/* =================== ERRORS row 5 =================== */

#[test]
fn err05_getalertdata_fseek_fails_on_pipe() {
    let _s = shared();
    unsafe {
        // Two alerts: the second "** Alert" line triggers the fseek push-back,
        // which fails with ESPIPE on a pipe -> l_error -> NULL.
        let mut s = String::new();
        s.push_str(&full_alert("11.1", false, "ossec,"));
        s.push_str(&full_alert("11.2", false, "ossec,"));
        let bytes = s.as_bytes();

        for api in [cc(), rs()] {
            let mut fds = [0i32; 2];
            assert_eq!(pipe(fds.as_mut_ptr()), 0);
            let n = write(fds[1], bytes.as_ptr() as *const _, bytes.len());
            assert_eq!(n as usize, bytes.len());
            close(fds[1]);
            let m = cpath("r");
            let fp = fdopen(fds[0], m.as_ptr());
            assert!(!fp.is_null());
            set_errno(0);
            let a = (api.GetAlertData)(0, fp);
            let snap = snap_alert(a);
            if !a.is_null() {
                (api.FreeAlertData)(a);
            }
            assert!(
                snap.is_none(),
                "{}: expected NULL when fseek push-back fails, got {:#?}",
                api.name,
                snap
            );
            fclose(fp);
        }

        // Sanity: on a *seekable* stream the same input yields two records.
        let c = drain_get_alert_data(cc(), "c", bytes, 0, 8, None);
        assert_eq!(c.len(), 3, "expected 2 records + terminating NULL");
    }
}

/* =================== ERRORS rows 6-8 =================== */

#[test]
fn err06_header_without_colon() {
    unsafe {
        for s in [
            &b"** Alert 1163181400 no colon\n2006 Apr 13 16:15:17 host->/x\nRule: 1 (level 1) -> 'x'\n"[..],
            &b"** Alert no-colon-here\n"[..],
            &b"** Alert\n2006 Apr 13 16:15:17 host->/x\n"[..],
        ] {
            diff_get_alert_data(s, F0, "header without colon");
        }
    }
}

#[test]
fn err07_header_without_space() {
    unsafe {
        for s in [
            &b"** Alert 1163181400.1:nospace\n2006 Apr 13 16:15:17 host->/x\n"[..],
            &b"** Alert:\n2006 Apr 13 16:15:17 host->/x\n"[..],
            &b"** Alert 1:2:3\n"[..],
        ] {
            diff_get_alert_data(s, F0, "header without space");
        }
    }
}

#[test]
fn err08_mail_filter_rejects() {
    let _s = shared();
    unsafe {
        let s = full_alert("12.1", false, "ossec,");
        // With CRALERT_MAIL_SET the "no-mail" tag makes the header be skipped,
        // so parsing never reaches _r=1 and the final result is NULL.
        let c = drain_get_alert_data(cc(), "c", s.as_bytes(), CRALERT_MAIL_SET, 4, None);
        let r = drain_get_alert_data(rs(), "r", s.as_bytes(), CRALERT_MAIL_SET, 4, None);
        assert_eq!(c.len(), r.len());
        assert!(c[0].0.is_none(), "C should reject non-mail alert");
        assert!(r[0].0.is_none(), "RUST should reject non-mail alert");
        for (a, b) in c.iter().zip(r.iter()) {
            assert_eq!(a.0, b.0);
            assert_eq!(a.1, b.1);
        }
    }
}

/* =================== ERRORS rows 9-10 (perror paths) =================== */

fn stderr_diff_get_alert_data(content: &[u8], flag: c_int, label: &str) {
    let _g = guard();
    unsafe {
        let (c, cerr) = capture_stderr(|| drain_get_alert_data(cc(), "c", content, flag, 8, None));
        let (r, rerr) = capture_stderr(|| drain_get_alert_data(rs(), "r", content, flag, 8, None));
        assert_eq!(c.len(), r.len(), "{label}: count");
        for (i, (a, b)) in c.iter().zip(r.iter()).enumerate() {
            assert_eq!(a.0, b.0, "{label}: record {i}");
            assert_eq!(a.1, b.1, "{label}: stream {i}");
        }
        assert_eq!(
            String::from_utf8_lossy(&cerr),
            String::from_utf8_lossy(&rerr),
            "{label}: stderr (perror) differs"
        );
    }
}

#[test]
fn err09_date_line_colon_no_space() {
    for s in [
        &b"** Alert 13.1: mail - ossec,\ncolon:butnospace\n"[..],
        &b"** Alert 13.2: mail - ossec,\n:\n"[..],
        &b"** Alert 13.3: mail - ossec,\nabc:def:ghi\n"[..],
        &b"** Alert 13.4: mail - ossec,\n2006 Apr 13 16:15:17\n"[..],
    ] {
        stderr_diff_get_alert_data(s, F0, "date line colon without space");
    }
}

#[test]
fn err10_date_line_no_colon() {
    for s in [
        &b"** Alert 14.1: mail - ossec,\nno colon at all\n"[..],
        &b"** Alert 14.2: mail - ossec,\n\n"[..],
        &b"** Alert 14.3: mail - ossec,\nplain\nRule: 1 (level 1) -> 'x'\n"[..],
    ] {
        stderr_diff_get_alert_data(s, F0, "date line without colon");
    }
}

/* =================== ERRORS rows 11-13 =================== */

#[test]
fn err11_rule_missing_second_space() {
    unsafe {
        for line in [
            "Rule: 123",
            "Rule: ",
            "Rule:",
            "Rule: 123 ",
            "Rule: 123 (level",
            "Rule: nonnumeric",
        ] {
            let s = format!(
                "** Alert 15.1: mail - ossec,\n2006 Apr 13 16:15:17 host->/x\n{}\n",
                line
            );
            diff_get_alert_data(s.as_bytes(), F0, &format!("rule {line:?}"));
        }
    }
}

#[test]
fn err12_rule_missing_open_quote() {
    unsafe {
        for line in [
            "Rule: 12 (level 7) -> no quotes here",
            "Rule: 12 (level 7)",
            "Rule: 1 2 3 4",
        ] {
            let s = format!(
                "** Alert 16.1: mail - ossec,\n2006 Apr 13 16:15:17 host->/x\n{}\n",
                line
            );
            diff_get_alert_data(s.as_bytes(), F0, &format!("rule {line:?}"));
        }
    }
}

#[test]
fn err13_rule_missing_close_quote() {
    unsafe {
        for line in [
            "Rule: 12 (level 7) -> 'unterminated",
            "Rule: 12 (level 7) -> '",
            "Rule: 1 2 'x",
        ] {
            let s = format!(
                "** Alert 17.1: mail - ossec,\n2006 Apr 13 16:15:17 host->/x\n{}\n",
                line
            );
            diff_get_alert_data(s.as_bytes(), F0, &format!("rule {line:?}"));
        }
        // a comment consisting only of the closing quote
        diff_get_alert_data(
            b"** Alert 17.2: mail - ossec,\n2006 Apr 13 16:15:17 host->/x\nRule: 1 (level 1) -> ''\n",
            F0,
            "empty comment",
        );
    }
}

/* =================== ERRORS rows 14-15 =================== */

#[test]
fn err14_eof_without_complete_alert() {
    unsafe {
        for s in [
            &b""[..],
            &b"\n"[..],
            &b"nothing to see here\n"[..],
            &b"** Alert 18.1: mail - ossec,\n"[..],
            &b"Rule: 1 (level 1) -> 'x'\n"[..],
            &b"Src IP: 1.2.3.4\nUser: root\n"[..],
        ] {
            diff_get_alert_data(s, F0, "eof without complete alert");
        }
    }
}

#[test]
fn err15_eof_flag_cleared() {
    let _s = shared();
    unsafe {
        // After a NULL return the stream's EOF *and* error indicators must be
        // cleared (`clearerr` in the C), so a subsequent read starts fresh.
        let content = b"junk only\n";
        for api in [cc(), rs()] {
            let name = scratch("eofclr", content);
            let fp = open_ro(&name);
            let a = (api.GetAlertData)(0, fp);
            assert!(a.is_null(), "{}", api.name);
            assert_eq!(feof(fp), 0, "{}: EOF flag not cleared", api.name);
            assert_eq!(ferror(fp), 0, "{}: error flag not cleared", api.name);
            assert_eq!(ftell(fp), content.len() as i64, "{}", api.name);
            fclose(fp);
            let _ = std::fs::remove_file(&name);
        }
        // Calling again on an exhausted stream also returns NULL and leaves the
        // flags clear.
        for api in [cc(), rs()] {
            let name = scratch("eofclr2", content);
            let fp = open_ro(&name);
            for _ in 0..3 {
                let a = (api.GetAlertData)(0, fp);
                assert!(a.is_null());
                assert_eq!(feof(fp), 0, "{}", api.name);
            }
            fclose(fp);
            let _ = std::fs::remove_file(&name);
        }
    }
}

/* =================== ERRORS row 16 =================== */

#[test]
fn err16_integrity_empty_filename() {
    unsafe {
        // `strdup("")` then `filename[strlen(filename)-1] = 0` writes at
        // `filename[-1]`. On glibc x86-64 that byte is the (already zero) MSB
        // of the chunk header, so the behaviour is the same in both builds.
        let s = "** Alert 19.1: mail - syscheck,\n2006 Apr 13 16:15:17 host->/x\nIntegrity checksum changed for: '\n";
        diff_get_alert_data(s.as_bytes(), F0, "integrity empty filename");
        // Same, without the trailing newline on the integrity line.
        let t = "** Alert 19.2: mail - syscheck,\n2006 Apr 13 16:15:17 host->/x\nIntegrity checksum changed for: '";
        diff_get_alert_data(t.as_bytes(), F0, "integrity empty filename, no nl");
    }
}

/* =================== ERRORS row 17 (null args) =================== */

#[test]
fn err17_null_pointer_args_both_segv() {
    for action in [
        "null_getalertdata",
        "null_freealertdata",
        "null_initfilequeue_q",
        "null_initfilequeue_tm",
        "null_readfilemon_q",
        "null_readfilemon_tm",
    ] {
        diff_worker_fatal(action);
        // the C side must specifically SIGSEGV
        let c = run_worker(&format!("c:{action}"));
        assert_eq!(c.signal, Some(11), "C {action} should SIGSEGV: {c:#?}");
    }
}

/* =================== ERRORS row 33 =================== */

#[test]
fn err33_body_before_header() {
    unsafe {
        for s in [
            &b"Rule: 1 (level 1) -> 'x'\nSrc IP: 1.2.3.4\n** Alert 20.1: mail - ossec,\n2006 Apr 13 16:15:17 host->/x\nUser: root\n"[..],
            &b"leading junk\n\n\n** Alert 20.2: mail - syscheck,\n2006 Apr 13 16:15:17 host->/x\nIntegrity checksum changed for: '/a'\n"[..],
        ] {
            diff_get_alert_data(s, F0, "body before header");
        }
    }
}

/* =================== ERRORS row 34 =================== */

#[test]
fn err34_atoi_garbage_and_overflow() {
    unsafe {
        let nums: [&str; 22] = [
            "0",
            "-0",
            "1",
            "-1",
            "2147483647",
            "2147483648",
            "-2147483648",
            "-2147483649",
            "4294967295",
            "4294967296",
            "9223372036854775807",
            "9223372036854775808",
            "-9223372036854775808",
            "-9223372036854775809",
            "99999999999999999999999",
            "  42",
            "+42",
            "0x1f",
            "012",
            "abc",
            "",
            "1abc",
        ];
        for n in nums {
            let s = format!(
                "** Alert 21.1: mail - ossec,\n2006 Apr 13 16:15:17 host->/x\nRule: {n} (level {n}) -> 'c'\nSrc Port: {n}\nDst Port: {n}\n"
            );
            diff_get_alert_data(s.as_bytes(), F0, &format!("atoi {n:?}"));
        }
    }
}

/* =================== sub-process worker =================== */

#[test]
fn zz_subprocess_worker() {
    let Some(action) = worker_action() else {
        return;
    };
    let (api, rest) = worker_api(&action);
    unsafe {
        match rest {
            "null_getalertdata" => {
                let p = (api.GetAlertData)(0, std::ptr::null_mut());
                emit(&format!("{:?}", p));
            }
            "null_freealertdata" => {
                (api.FreeAlertData)(std::ptr::null_mut());
                emit("survived");
            }
            "null_initfilequeue_q" => {
                let t = tm::new(1, 0, 100);
                let rc = (api.Init_FileQueue)(std::ptr::null_mut(), &t, 0);
                emit(&format!("{rc}"));
            }
            "null_initfilequeue_tm" => {
                let mut q = file_queue::zeroed();
                let rc = (api.Init_FileQueue)(&mut q, std::ptr::null(), 0);
                emit(&format!("{rc}"));
            }
            "null_readfilemon_q" => {
                let t = tm::new(1, 0, 100);
                let p = (api.Read_FileMon)(std::ptr::null_mut(), &t, 0);
                emit(&format!("{:?}", p));
            }
            "null_readfilemon_tm" => {
                // fp must be non-NULL and yield no alert, so that the code
                // reaches `fileq->day = p->tm_mday` with p == NULL.
                let name = scratch("nulltm", b"nothing here\n");
                let mut q = file_queue::zeroed();
                q.fp = open_ro(&name);
                let p = (api.Read_FileMon)(&mut q, std::ptr::null(), 0);
                emit(&format!("{:?}", p));
            }
            other => panic!("unknown worker action {other:?}"),
        }
    }
    std::process::exit(0);
}
