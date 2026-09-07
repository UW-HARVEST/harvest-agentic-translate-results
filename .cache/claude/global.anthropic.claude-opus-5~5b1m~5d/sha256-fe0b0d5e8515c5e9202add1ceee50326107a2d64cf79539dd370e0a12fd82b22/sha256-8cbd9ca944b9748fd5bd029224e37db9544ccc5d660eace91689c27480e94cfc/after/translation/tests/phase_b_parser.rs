//! Phase B, rows 6-23: `GetAlertData` -- the lowest-level parser entry point,
//! driven directly with an explicit `FILE*` (never through `driver`).

mod common;
use common::*;

use std::os::raw::c_int;
use std::path::PathBuf;

fn dir() -> PathBuf {
    tmp_root()
}

/// Drain both implementations over the same bytes/flag and require identical
/// result sequences.
#[track_caller]
fn cmp(tag: &str, bytes: &[u8], flag: c_int) -> Vec<Option<AlertSnap>> {
    let b = both();
    let d = dir();
    unsafe {
        let c = drain(&b.c, &d, tag, bytes, flag, 24);
        let r = drain(&b.rs, &d, tag, bytes, flag, 24);
        if c != r {
            panic!(
                "GetAlertData divergence [{tag}] flag={flag:#x}\n--- input ---\n{}\n--- C ---\n{c:#?}\n--- Rust ---\n{r:#?}",
                String::from_utf8_lossy(bytes)
            );
        }
        c
    }
}

/* ---------------- row 6 ---------------- */

#[test]
fn cfg_06_single_well_formed_alert() {
    let got = cmp("r06", &simple_alert(), 0);
    let a = got[0].as_ref().expect("C should parse the canonical alert");
    assert_eq!(a.rule, 5715);
    assert_eq!(a.level, 5);
    assert_eq!(a.date.as_deref(), Some(&b"2006 Apr 13 16:15:17"[..]));
    assert_eq!(a.location.as_deref(), Some(&b"myhost->/var/log/auth.log"[..]));
    assert_eq!(a.comment.as_deref(), Some(&b"sshd authentication success."[..]));
    assert_eq!(a.srcip.as_deref(), Some(&b"192.168.1.1"[..]));
    assert_eq!(a.srcport, 4321);
    assert_eq!(a.dstport, 22);
    assert_eq!(a.user.as_deref(), Some(&b"root"[..]));
}

/* ---------------- rows 7 + 8 + 9 ---------------- */

fn rand_spec(rng: &mut Rng, mail: bool, group_syscheck: bool) -> AlertSpec {
    let ports: [&[u8]; 10] = [
        b"0", b"22", b"65535", b"65536", b"-1", b"-2147483648", b"2147483647",
        b"99999999999999", b"notanumber", b"  12abc",
    ];
    let mut order: Vec<usize> = (0..6).collect();
    // Fisher-Yates with the harness PRNG
    for i in (1..order.len()).rev() {
        let j = rng.below(i + 1);
        order.swap(i, j);
    }
    let comment = {
        let mut t = rng.token(40);
        t.retain(|&c| c != b'\'');
        t
    };
    AlertSpec {
        id: {
            let mut v = Vec::new();
            v.extend_from_slice(format!("{}", 1_500_000_000u64 + rng.next_u64() % 1000).as_bytes());
            v.push(b'.');
            v.extend_from_slice(format!("{}", rng.below(100000)).as_bytes());
            v
        },
        tag: if mail { b"mail".to_vec() } else { b"noop".to_vec() },
        group: if group_syscheck {
            b"ossec,syscheck,".to_vec()
        } else {
            b"syslog,errors,".to_vec()
        },
        date: b"2006 Apr 13 16:15:17".to_vec(),
        location: {
            let mut v = b"host->".to_vec();
            let mut t = rng.token(20);
            t.retain(|&c| c != b' ');
            v.extend_from_slice(&t);
            v
        },
        rule: if rng.below(8) != 0 {
            let mut v = Vec::new();
            v.extend_from_slice(format!("{}", rng.below(100000)).as_bytes());
            v.extend_from_slice(b" (level ");
            v.extend_from_slice(format!("{}", rng.below(20)).as_bytes());
            v.extend_from_slice(b") -> '");
            v.extend_from_slice(&comment);
            v.push(b'\'');
            Some(v)
        } else {
            None
        },
        srcip: if rng.bool() { Some(rng.token(24)) } else { None },
        srcport: if rng.bool() { Some(rng.pick(&ports).to_vec()) } else { None },
        dstip: if rng.bool() { Some(rng.token(24)) } else { None },
        dstport: if rng.bool() { Some(rng.pick(&ports).to_vec()) } else { None },
        user: if rng.bool() { Some(rng.token(24)) } else { None },
        logs: (0..rng.below(4))
            .map(|_| {
                let mut l = rng.token(60);
                // avoid accidentally forming a recognized prefix or a new header
                if l.starts_with(b"*") {
                    l[0] = b'x';
                }
                l
            })
            .collect(),
        order,
    }
}

#[test]
fn cfg_07_08_all_fields_random_values_order_and_subsets() {
    let mut rng = Rng::new(0x0708_0708_0708_0708);
    for i in 0..300 {
        let spec = rand_spec(&mut rng, false, false);
        let mut buf = Vec::new();
        spec.render(&mut buf);
        buf.push(b'\n');
        cmp(&format!("r0708_{i}"), &buf, 0);
    }
}

#[test]
fn cfg_09_duplicate_field_lines_last_wins() {
    let mut rng = Rng::new(0x0909_0909_0909_0909);
    for i in 0..120 {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"** Alert 1500000000.1: noop - syslog,\n");
        buf.extend_from_slice(b"2006 Apr 13 16:15:17 host->/var/log/x\n");
        // several duplicates of each recognized line
        for k in 0..3 {
            buf.extend_from_slice(b"Rule: ");
            buf.extend_from_slice(format!("{}", 100 + k).as_bytes());
            buf.extend_from_slice(b" (level ");
            buf.extend_from_slice(format!("{}", k).as_bytes());
            buf.extend_from_slice(b") -> 'c");
            buf.extend_from_slice(format!("{k}").as_bytes());
            buf.extend_from_slice(b"'\n");
            buf.extend_from_slice(b"Src IP: ");
            buf.extend_from_slice(&rng.token(12));
            buf.push(b'\n');
            buf.extend_from_slice(b"Src Port: ");
            buf.extend_from_slice(format!("{}", rng.below(70000)).as_bytes());
            buf.push(b'\n');
            buf.extend_from_slice(b"Dst IP: ");
            buf.extend_from_slice(&rng.token(12));
            buf.push(b'\n');
            buf.extend_from_slice(b"Dst Port: ");
            buf.extend_from_slice(format!("{}", rng.below(70000)).as_bytes());
            buf.push(b'\n');
            buf.extend_from_slice(b"User: ");
            buf.extend_from_slice(&rng.token(12));
            buf.push(b'\n');
        }
        buf.extend_from_slice(b"\n");
        cmp(&format!("r09_{i}"), &buf, 0);
    }
}

/* ---------------- row 10: multi-alert push-back ---------------- */

#[test]
fn cfg_10_multi_alert_sequence() {
    let mut rng = Rng::new(0x1010_1010_1010_1010);
    for i in 0..120 {
        let n = 2 + rng.below(5);
        let mut buf = Vec::new();
        for _ in 0..n {
            rand_spec(&mut rng, false, false).render(&mut buf);
        }
        let got = cmp(&format!("r10_{i}"), &buf, 0);
        // sanity: several alerts really came back before the NULL terminator
        let parsed = got.iter().filter(|x| x.is_some()).count();
        assert!(parsed >= 1, "expected >=1 parsed alert, got {got:#?}");
    }
}

/* ---------------- rows 11-13: CRALERT_MAIL_SET ---------------- */

#[test]
fn cfg_11_mail_set_accepts_mail() {
    let mut rng = Rng::new(0x1111_1111_1111_1111);
    for i in 0..80 {
        let mut buf = Vec::new();
        rand_spec(&mut rng, true, false).render(&mut buf);
        buf.push(b'\n');
        cmp(&format!("r11_{i}"), &buf, CRALERT_MAIL_SET);
    }
}

#[test]
fn cfg_12_mail_set_rejects_non_mail() {
    let mut rng = Rng::new(0x1212_1212_1212_1212);
    let tags: [&[u8]; 6] = [b"noop", b"mai", b"maill", b"MAIL", b"", b"exec"];
    for i in 0..80 {
        let mut spec = rand_spec(&mut rng, false, false);
        spec.tag = rng.pick(&tags).to_vec();
        let mut buf = Vec::new();
        spec.render(&mut buf);
        buf.push(b'\n');
        // note: "maill" starts with "mail" so strncmp(...,4)==0 -> accepted.
        cmp(&format!("r12_{i}"), &buf, CRALERT_MAIL_SET);
    }
}

#[test]
fn cfg_13_mail_set_mixed_file() {
    let mut rng = Rng::new(0x1313_1313_1313_1313);
    for i in 0..120 {
        let mut buf = Vec::new();
        for _ in 0..(2 + rng.below(4)) {
            let is_mail = rng.bool();
            rand_spec(&mut rng, is_mail, false).render(&mut buf);
        }
        cmp(&format!("r13_{i}"), &buf, CRALERT_MAIL_SET);
    }
}

/* ---------------- rows 14-17: syscheck / integrity line ---------------- */

fn syscheck_alert(group: &[u8], first_log: Option<&[u8]>, rest: &[&[u8]]) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(b"** Alert 1500000000.7: noop - ");
    v.extend_from_slice(group);
    v.push(b'\n');
    v.extend_from_slice(b"2006 Apr 13 16:15:17 host->syscheck\n");
    v.extend_from_slice(b"Rule: 550 (level 7) -> 'Integrity checksum changed.'\n");
    if let Some(l) = first_log {
        v.extend_from_slice(l);
        v.push(b'\n');
    }
    for l in rest {
        v.extend_from_slice(l);
        v.push(b'\n');
    }
    v.push(b'\n');
    v
}

#[test]
fn cfg_14_syscheck_group_with_integrity_line() {
    let mut rng = Rng::new(0x1414_1414_1414_1414);
    let mut paths: Vec<Vec<u8>> = vec![
        b"/etc/passwd".to_vec(),
        b"x".to_vec(),
        b"/".to_vec(),
        vec![b'p'; 300],
        b"/path with spaces/and'quote".to_vec(),
    ];
    for _ in 0..60 {
        let mut p = b"/".to_vec();
        p.extend_from_slice(&rng.token(80));
        paths.push(p);
    }
    for (i, p) in paths.iter().enumerate() {
        let mut line = b"Integrity checksum changed for: '".to_vec();
        line.extend_from_slice(p);
        line.push(b'\'');
        let buf = syscheck_alert(b"ossec,syscheck,", Some(&line), &[b"Old md5sum was: aaa"]);
        let got = cmp(&format!("r14_{i}"), &buf, 0);
        let a = got[0].as_ref().expect("should parse");
        assert!(a.filename.is_some(), "filename not captured for {p:?}");
    }
}

#[test]
fn cfg_15_syscheck_group_without_integrity_line() {
    let buf = syscheck_alert(b"ossec,syscheck,", Some(b"some other log line"), &[]);
    let got = cmp("r15", &buf, 0);
    assert_eq!(got[0].as_ref().unwrap().filename, None);
}

#[test]
fn cfg_16_integrity_line_without_syscheck_group() {
    let buf = syscheck_alert(
        b"syslog,errors,",
        Some(b"Integrity checksum changed for: '/etc/shadow'"),
        &[],
    );
    let got = cmp("r16", &buf, 0);
    assert_eq!(got[0].as_ref().unwrap().filename, None);
}

#[test]
fn cfg_17_integrity_line_not_first_log_line() {
    let buf = syscheck_alert(
        b"ossec,syscheck,",
        Some(b"a preceding log line"),
        &[b"Integrity checksum changed for: '/etc/shadow'"],
    );
    let got = cmp("r17", &buf, 0);
    assert_eq!(got[0].as_ref().unwrap().filename, None);
}

/* ---------------- row 18: alert-id / header shapes ---------------- */

#[test]
fn cfg_18_header_shapes() {
    let headers: [&[u8]; 16] = [
        b"** Alert 1500000000.1: noop - syslog,",
        b"** Alert 1500000000.1: noop",                 // no '-' -> group stays NULL
        b"** Alert 1500000000.1: noop - a - b - c,",    // several '-'
        b"** Alert 1500000000.1: noop -      spaced,",  // many leading spaces
        b"** Alert 1500-000-000.1: noop - g,",          // '-' before the ':'
        b"** Alert -: x - g,",
        b"** Alert :: noop - g,",
        b"** Alert 1: noop - g,",
        b"** Alerts 1500000000.1: noop - g,",           // 9th char is 's'
        b"** Alert1500000000.1: noop - g,",
        b"** Alert  1500000000.1:  noop  -  g,",
        b"** Alert 1500000000.1:noop-g,",
        b"** Alert 1500000000.1: - g,",
        b"** Alert 1500000000.1: mail - syscheck,",
        b"** Alert 0:0 0-0",
        b"** Alert                                  x: y - z,",
    ];
    for (i, h) in headers.iter().enumerate() {
        let mut buf = h.to_vec();
        buf.push(b'\n');
        buf.extend_from_slice(b"2006 Apr 13 16:15:17 host->/var/log/x\n");
        buf.extend_from_slice(b"Rule: 1 (level 2) -> 'c'\n");
        buf.extend_from_slice(b"log line\n\n");
        for flag in [0, CRALERT_MAIL_SET] {
            cmp(&format!("r18_{i}_{flag}"), &buf, flag);
        }
    }
}

/* ---------------- row 19: OS_MAXSTR line-length boundaries ---------------- */

#[test]
fn cfg_19_line_length_boundaries() {
    for &n in &[
        1usize, 100, 1020, 1021, 1022, 1023, 1024, 1025, 1026, 2045, 2046, 2047, 2048, 4096,
    ] {
        // long log line
        let mut buf = Vec::new();
        buf.extend_from_slice(b"** Alert 1500000000.1: noop - syslog,\n");
        buf.extend_from_slice(b"2006 Apr 13 16:15:17 host->/var/log/x\n");
        buf.extend_from_slice(b"Rule: 1 (level 2) -> 'c'\n");
        buf.extend_from_slice(&vec![b'L'; n]);
        buf.push(b'\n');
        buf.push(b'\n');
        cmp(&format!("r19_log_{n}"), &buf, 0);

        // long comment
        let mut buf = Vec::new();
        buf.extend_from_slice(b"** Alert 1500000000.1: noop - syslog,\n");
        buf.extend_from_slice(b"2006 Apr 13 16:15:17 host->/var/log/x\n");
        buf.extend_from_slice(b"Rule: 1 (level 2) -> '");
        buf.extend_from_slice(&vec![b'C'; n]);
        buf.extend_from_slice(b"'\n\n");
        cmp(&format!("r19_cmt_{n}"), &buf, 0);

        // long header line
        let mut buf = Vec::new();
        buf.extend_from_slice(b"** Alert 1500000000.1: noop - ");
        buf.extend_from_slice(&vec![b'G'; n]);
        buf.extend_from_slice(b",\n");
        buf.extend_from_slice(b"2006 Apr 13 16:15:17 host->/var/log/x\n");
        buf.extend_from_slice(b"Rule: 1 (level 2) -> 'c'\n\n");
        cmp(&format!("r19_hdr_{n}"), &buf, 0);

        // long location line
        let mut buf = Vec::new();
        buf.extend_from_slice(b"** Alert 1500000000.1: noop - syslog,\n");
        buf.extend_from_slice(b"2006 Apr 13 16:15:17 host->");
        buf.extend_from_slice(&vec![b'P'; n]);
        buf.push(b'\n');
        buf.extend_from_slice(b"Rule: 1 (level 2) -> 'c'\n\n");
        cmp(&format!("r19_loc_{n}"), &buf, 0);

        // long Src IP / User values
        let mut buf = Vec::new();
        buf.extend_from_slice(b"** Alert 1500000000.1: noop - syslog,\n");
        buf.extend_from_slice(b"2006 Apr 13 16:15:17 host->/var/log/x\n");
        buf.extend_from_slice(b"Src IP: ");
        buf.extend_from_slice(&vec![b'I'; n]);
        buf.push(b'\n');
        buf.extend_from_slice(b"User: ");
        buf.extend_from_slice(&vec![b'U'; n]);
        buf.extend_from_slice(b"\n\n");
        cmp(&format!("r19_ipu_{n}"), &buf, 0);
    }
}

/* ---------------- row 20: line-ending / blank-line shapes ---------------- */

#[test]
fn cfg_20_line_ending_shapes() {
    let base = simple_alert();

    // no trailing newline
    let mut a = base.clone();
    while a.last() == Some(&b'\n') {
        a.pop();
    }
    cmp("r20_nonl", &a, 0);

    // CRLF everywhere
    let crlf: Vec<u8> = String::from_utf8_lossy(&base).replace('\n', "\r\n").into_bytes();
    cmp("r20_crlf", &crlf, 0);

    // blank lines interleaved
    let mut b2 = Vec::new();
    for line in base.split(|&c| c == b'\n') {
        b2.extend_from_slice(line);
        b2.extend_from_slice(b"\n\n");
    }
    cmp("r20_blank", &b2, 0);

    // leading blank lines / only newlines
    cmp("r20_nl_only", b"\n\n\n\n", 0);
    cmp("r20_lead_nl", &[b"\n\n\n".to_vec(), base.clone()].concat(), 0);

    // header truncated exactly at "** Alert" with and without a newline
    cmp("r20_hdr_only_nl", b"** Alert\n", 0);
    cmp("r20_hdr_only", b"** Alert", 0);
    cmp("r20_hdr_sp", b"** Alert \n", 0);
    cmp("r20_hdr_colon", b"** Alert x:\n", 0);
}

/* ---------------- row 21: degenerate files ---------------- */

#[test]
fn cfg_21_degenerate_files() {
    cmp("r21_empty", b"", 0);
    cmp("r21_ws", b"   \t  \n \t\n", 0);
    cmp("r21_logs", b"just\nsome\nlog\nlines\n", 0);
    cmp("r21_rule_only", b"Rule: 1 (level 2) -> 'c'\n", 0);
    cmp("r21_nul", b"a\0b\nRule: 1\n", 0);
    cmp("r21_hi", &[0x80u8, 0xff, b'\n', 0xfe, b'\n'], 0);
}

/* ---------------- row 22: full-int `flag` values ---------------- */

#[test]
fn cfg_22_arbitrary_flag_ints() {
    let mut rng = Rng::new(0x2222_2222_2222_2222);
    let mut flags: Vec<c_int> = vec![
        0,
        CRALERT_MAIL_SET,
        CRALERT_EXEC_SET,
        CRALERT_READ_ALL,
        CRALERT_READ_FAILED,
        CRALERT_FP_SET,
        CRALERT_MAIL_SET | CRALERT_EXEC_SET | CRALERT_READ_ALL | CRALERT_READ_FAILED
            | CRALERT_FP_SET,
        0x20,
        0x40,
        -1,
        i32::MIN,
        i32::MAX,
        0x5555_5555,
        -0x5555_5555,
    ];
    for _ in 0..48 {
        flags.push(rng.i32());
    }

    let mut mixed = Vec::new();
    let mut r2 = Rng::new(0xdead_beef_cafe_1234);
    for _ in 0..3 {
        rand_spec(&mut r2, true, false).render(&mut mixed);
        rand_spec(&mut r2, false, true).render(&mut mixed);
    }

    for (i, f) in flags.iter().enumerate() {
        cmp(&format!("r22_{i}"), &mixed, *f);
        cmp(&format!("r22s_{i}"), &simple_alert(), *f);
    }
}

/* ---------------- row 23: non-zero start offset ---------------- */

#[test]
fn cfg_23_non_zero_start_offset() {
    let b = both();
    let d = dir();
    let mut rng = Rng::new(0x2323_2323_2323_2323);
    let mut buf = Vec::new();
    for _ in 0..4 {
        rand_spec(&mut rng, false, false).render(&mut buf);
    }
    let len = buf.len() as i64;
    let offsets: Vec<i64> = {
        let mut v = vec![0, 1, 2, 8, 9, 10, len - 1, len, len + 1, len + 100];
        for _ in 0..40 {
            v.push(rng.below(buf.len() + 8) as i64);
        }
        v
    };
    unsafe {
        for (i, off) in offsets.iter().enumerate() {
            for flag in [0, CRALERT_MAIL_SET] {
                let mut run = |imp: &Impl| {
                    let fp = open_bytes(&d, &format!("r23_{}_{i}.log", imp.name), &buf);
                    fseek(fp, *off as _, SEEK_SET);
                    let mut out = Vec::new();
                    for _ in 0..12 {
                        let a = (imp.GetAlertData)(flag, fp);
                        let s = snap(a);
                        let done = s.is_none();
                        if !a.is_null() {
                            (imp.FreeAlertData)(a);
                        }
                        out.push(s);
                        if done {
                            break;
                        }
                    }
                    fclose(fp);
                    out
                };
                let rc = run(&b.c);
                let rr = run(&b.rs);
                assert_eq!(rc, rr, "offset {off} flag {flag:#x} diverged");
            }
        }
    }
}
