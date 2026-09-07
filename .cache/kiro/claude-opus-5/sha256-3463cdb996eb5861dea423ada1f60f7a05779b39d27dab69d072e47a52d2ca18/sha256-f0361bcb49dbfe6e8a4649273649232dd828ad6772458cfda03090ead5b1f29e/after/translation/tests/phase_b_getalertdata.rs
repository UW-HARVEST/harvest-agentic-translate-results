//! Phase B — CONFIGS.md rows 6..24: `GetAlertData`, the lowest-level parsing
//! entry point, driven directly through both `.so`s.
//!
//! Every test drains the stream with repeated `GetAlertData` calls and compares
//! the whole sequence of results *and* the stream offset / EOF state after each
//! call, so the `fseek` push-back behaviour is covered too.

#![allow(non_snake_case)]

mod harness;
use harness::*;

use std::ffi::c_int;

const ALL_FLAGS: c_int =
    CRALERT_MAIL_SET | CRALERT_EXEC_SET | CRALERT_READ_ALL | CRALERT_READ_FAILED | CRALERT_FP_SET;

/// Row 6 — minimal complete alert, `flag = 0`.
#[test]
fn row06_minimal_alert() {
    let text = AlertSpec::minimal().render();
    diff_get_alert_data("minimal", text.as_bytes(), 0);
}

/// Row 7 — an alert carrying every recognised field plus free-form log lines.
#[test]
fn row07_all_fields() {
    let mut a = AlertSpec::minimal();
    a.srcip = Some("Src IP: 192.168.10.44".into());
    a.srcport = Some("Src Port: 51234".into());
    a.dstip = Some("Dst IP: 10.1.2.3".into());
    a.dstport = Some("Dst Port: 22".into());
    a.user = Some("User: root".into());
    a.body = vec![
        "Jan 12 03:04:05 host sshd[1]: Failed password for root".into(),
        "extra context line".into(),
    ];
    diff_get_alert_data("all-fields", a.render().as_bytes(), 0);

    // Field order permutations: the parser dispatches per line, so order is a
    // real axis.
    let mut rng = Rng::new(0xB0B0_0007);
    for n in 0..40 {
        let mut lines = vec![
            format!("Rule: {} (level {}) -> 'c{}'", 1000 + n, n % 16, n),
            format!("Src IP: 10.0.{}.{}", n % 255, (n * 7) % 255),
            format!("Src Port: {}", n * 111),
            format!("Dst IP: 172.16.{}.1", n % 255),
            format!("Dst Port: {}", n * 3),
            format!("User: u{}", n),
        ];
        // Fisher-Yates with the deterministic RNG.
        for i in (1..lines.len()).rev() {
            let j = rng.below(i + 1);
            lines.swap(i, j);
        }
        let text = format!(
            "** Alert 1.{n}: mail - syslog\n2006 Apr 13 16:15:17 /var/log/x\n{}\n",
            lines.join("\n")
        );
        diff_get_alert_data(&format!("perm{n}"), text.as_bytes(), 0);
    }
}

/// Row 8 — many alerts in one file: exercises `_r == 2` +
/// `fseek(fp, -strlen(str), SEEK_CUR)` push-back over and over.
#[test]
fn row08_multiple_alerts() {
    for count in [2usize, 3, 5, 17] {
        let specs: Vec<AlertSpec> = (0..count)
            .map(|i| {
                let mut a = AlertSpec::minimal();
                a.id = format!("15000000{:02}.{}", i, i * 13);
                a.group = format!("grp{i},syslog");
                a.date = format!("2006 Apr {:02} 16:15:17", 1 + i % 28);
                a.location = format!("/var/log/f{i}.log");
                a.rule = Some(format!(
                    "Rule: {} (level {}) -> 'comment number {i}'",
                    1000 + i,
                    i % 16
                ));
                a.user = Some(format!("User: user{i}"));
                a
            })
            .collect();
        diff_get_alert_data(
            &format!("multi{count}"),
            render_all(&specs).as_bytes(),
            0,
        );
    }
}

/// Row 9 — repeated field lines inside one alert (`os_free` + re-`strdup`).
#[test]
fn row09_repeated_fields() {
    let text = "\
** Alert 1500000000.1: mail - syslog,errors
2006 Apr 13 16:15:17 /var/log/auth.log
Rule: 1001 (level 3) -> 'first comment'
Src IP: 1.1.1.1
Src IP: 2.2.2.2
Src Port: 11
Src Port: 22
Dst IP: 3.3.3.3
Dst IP: 4.4.4.4
Dst Port: 33
Dst Port: 44
User: alice
User: bob
Rule: 2002 (level 9) -> 'second comment'
";
    diff_get_alert_data("repeated", text.as_bytes(), 0);

    // A second `Rule:` whose parse FAILS after the first succeeded: the
    // partially-populated struct must be freed identically.
    let text2 = "\
** Alert 1500000000.1: mail - syslog
2006 Apr 13 16:15:17 /var/log/auth.log
Rule: 1001 (level 3) -> 'ok'
Rule: 2002
";
    diff_get_alert_data("repeated-then-bad", text2.as_bytes(), 0);
}

/// Row 10 — syscheck group + `Integrity checksum changed for: '<path>'`.
#[test]
fn row10_syscheck_filename() {
    for path in [
        "/etc/passwd'",
        "/etc/shadow'",
        "/a'",
        "'",
        "/very/long/path/with spaces/and-dashes/file.conf'",
    ] {
        let text = format!(
            "** Alert 1500000000.7: mail - ossec,syscheck,\n\
             2006 Apr 13 16:15:17 /var/log/syscheck\n\
             Rule: 550 (level 7) -> 'Integrity checksum changed'\n\
             Integrity checksum changed for: '{path}\n\
             Old md5sum was: aaa\n\
             New md5sum is : bbb\n"
        );
        diff_get_alert_data("syscheck", text.as_bytes(), 0);
    }

    // ERRORS.md row 31: the 33-byte prefix with an EMPTY remainder makes the C
    // write `filename[(size_t)-1]`. Reproduced, not fixed.
    let text = "** Alert 1500000000.7: mail - syscheck\n\
                2006 Apr 13 16:15:17 /var/log/syscheck\n\
                Rule: 550 (level 7) -> 'x'\n\
                Integrity checksum changed for: '\n";
    diff_get_alert_data("syscheck-empty-filename", text.as_bytes(), 0);

    // The syscheck line arriving as the FIRST body line vs. later.
    let text = "** Alert 1.1: mail - syscheck\n\
                2006 Apr 13 16:15:17 /x\n\
                Integrity checksum changed for: '/etc/hosts'\n\
                Rule: 550 (level 7) -> 'x'\n";
    diff_get_alert_data("syscheck-first", text.as_bytes(), 0);
}

/// Row 11 — syscheck group but the next body line does not match the prefix,
/// so `issyscheck` is consumed and `filename` stays NULL.
#[test]
fn row11_syscheck_reset() {
    let text = "** Alert 1.1: mail - syscheck\n\
                2006 Apr 13 16:15:17 /x\n\
                Rule: 550 (level 7) -> 'x'\n\
                something else entirely\n\
                Integrity checksum changed for: '/etc/hosts'\n";
    diff_get_alert_data("syscheck-reset", text.as_bytes(), 0);

    // Prefix present but one byte short / one byte off.
    for line in [
        "Integrity checksum changed for: ",
        "Integrity checksum changed for:",
        "Integrity checksum changed FOR: '/etc/hosts'",
        "ntegrity checksum changed for: '/etc/hosts'",
    ] {
        let text = format!(
            "** Alert 1.1: mail - syscheck\n2006 Apr 13 16:15:17 /x\n{line}\n"
        );
        diff_get_alert_data("syscheck-nearmiss", text.as_bytes(), 0);
    }
}

/// Row 12 — group without `syscheck`: the integrity line must not populate
/// `filename`.
#[test]
fn row12_no_syscheck_group() {
    let text = "** Alert 1.1: mail - syslog,errors\n\
                2006 Apr 13 16:15:17 /x\n\
                Rule: 1 (level 1) -> 'x'\n\
                Integrity checksum changed for: '/etc/hosts'\n";
    diff_get_alert_data("no-syscheck", text.as_bytes(), 0);

    // No `-` in the header at all -> group stays NULL entirely.
    let text = "** Alert 1.1: mail syslog\n\
                2006 Apr 13 16:15:17 /x\n\
                Rule: 1 (level 1) -> 'x'\n\
                Integrity checksum changed for: '/etc/hosts'\n";
    diff_get_alert_data("no-dash", text.as_bytes(), 0);
}

/// Rows 13 & 14 — `CRALERT_MAIL_SET` accept / reject.
#[test]
fn row13_14_mail_flag() {
    let mut mail = AlertSpec::minimal();
    mail.kind = "mail".into();
    let mut nomail = AlertSpec::minimal();
    nomail.kind = "exec".into();

    for flag in [0, CRALERT_MAIL_SET] {
        diff_get_alert_data("mail-yes", mail.render().as_bytes(), flag);
        diff_get_alert_data("mail-no", nomail.render().as_bytes(), flag);
        // Mixed stream: only some alerts are mail.
        let mixed = render_all(&[
            nomail.clone(),
            mail.clone(),
            nomail.clone(),
            mail.clone(),
        ]);
        diff_get_alert_data("mail-mixed", mixed.as_bytes(), flag);
    }

    // `mail` as a strict 4-byte prefix: `mailx` also matches `strncmp(...,4)`.
    for kind in ["mail", "mailx", "mai", "mail1", "MAIL", "", "-"] {
        let mut a = AlertSpec::minimal();
        a.kind = kind.into();
        diff_get_alert_data(
            &format!("mail-prefix-{kind}"),
            a.render().as_bytes(),
            CRALERT_MAIL_SET,
        );
    }
}

/// Row 15 — each defined flag bit alone, and all together.
#[test]
fn row15_each_flag_bit() {
    let mut a = AlertSpec::minimal();
    a.srcip = Some("Src IP: 8.8.8.8".into());
    a.user = Some("User: nobody".into());
    let text = a.render();

    for flag in [
        0,
        CRALERT_MAIL_SET,
        CRALERT_EXEC_SET,
        CRALERT_READ_ALL,
        CRALERT_READ_FAILED,
        CRALERT_FP_SET,
        CRALERT_EXEC_SET | CRALERT_READ_ALL | CRALERT_READ_FAILED | CRALERT_FP_SET,
        ALL_FLAGS,
    ] {
        diff_get_alert_data(&format!("flag{flag:#x}"), text.as_bytes(), flag);
    }
}

/// Row 16 — flags with undefined bits (`int` accepts anything).
#[test]
fn row16_undefined_flag_bits() {
    let text = AlertSpec::minimal().render();
    let mut rng = Rng::new(0xB0B0_0016);

    let mut flags: Vec<c_int> = vec![
        -1,
        i32::MIN,
        i32::MAX,
        0xFFFF,
        0x7FFF_FFFE,
        !CRALERT_MAIL_SET,
        0x20,
        0x1000_0000,
    ];
    for _ in 0..80 {
        flags.push(rng.i32());
    }
    for flag in flags {
        diff_get_alert_data("undef-flag", text.as_bytes(), flag);
    }
}

/// Row 17 — no trailing newline before EOF (the `feof && _r == 2` path).
#[test]
fn row17_no_trailing_newline() {
    let text = AlertSpec::minimal().render();
    let trimmed = text.trim_end_matches('\n');
    diff_get_alert_data("no-final-nl", trimmed.as_bytes(), 0);

    // Truncated at every interesting boundary near the end.
    let b = text.as_bytes();
    for cut in [b.len() - 1, b.len() - 2, b.len() / 2, 10, 9, 8, 1] {
        diff_get_alert_data(&format!("cut{cut}"), &b[..cut], 0);
    }
}

/// Row 18 — CRLF line endings. `os_clearnl` strips only `\n`, so the `\r`
/// survives inside every field.
#[test]
fn row18_crlf() {
    let mut a = AlertSpec::minimal();
    a.srcip = Some("Src IP: 5.5.5.5".into());
    a.user = Some("User: crlf".into());
    let text = a.render().replace('\n', "\r\n");
    diff_get_alert_data("crlf", text.as_bytes(), 0);

    // Multiple CRLF alerts -> push-back offsets must match too.
    let text = render_all(&[a.clone(), a.clone(), a]).replace('\n', "\r\n");
    diff_get_alert_data("crlf-multi", text.as_bytes(), 0);
}

/// Row 19 — lines longer than `OS_MAXSTR - 1` (1023), so `fgets` splits them.
#[test]
fn row19_oversized_lines() {
    for len in [1020usize, 1022, 1023, 1024, 1025, 2050, 3000] {
        // Long comment inside the Rule line.
        let mut a = AlertSpec::minimal();
        a.rule = Some(format!(
            "Rule: 1002 (level 6) -> '{}'",
            "C".repeat(len)
        ));
        diff_get_alert_data(&format!("long-rule{len}"), a.render().as_bytes(), 0);

        // Long header line.
        let mut b = AlertSpec::minimal();
        b.group = "g".repeat(len);
        diff_get_alert_data(&format!("long-hdr{len}"), b.render().as_bytes(), 0);

        // Long location.
        let mut c = AlertSpec::minimal();
        c.location = format!("/{}", "p".repeat(len));
        diff_get_alert_data(&format!("long-loc{len}"), c.render().as_bytes(), 0);

        // Long free-form body line.
        let mut d = AlertSpec::minimal();
        d.body = vec!["L".repeat(len)];
        diff_get_alert_data(&format!("long-body{len}"), d.render().as_bytes(), 0);

        // Long Src IP / User values.
        let mut e = AlertSpec::minimal();
        e.srcip = Some(format!("Src IP: {}", "1".repeat(len)));
        e.user = Some(format!("User: {}", "u".repeat(len)));
        diff_get_alert_data(&format!("long-fields{len}"), e.render().as_bytes(), 0);
    }
}

/// Row 20 — blank and whitespace-only lines interleaved.
#[test]
fn row20_blank_lines() {
    let text = "\n\n** Alert 1.1: mail - syslog\n\n2006 Apr 13 16:15:17 /x\n\n \n\t\n\
                Rule: 1 (level 1) -> 'c'\n\n";
    diff_get_alert_data("blank", text.as_bytes(), 0);

    // A blank line right where the date line is expected (`_r == 1`).
    diff_get_alert_data("blank-date", b"** Alert 1.1: mail - g\n\n", 0);
    diff_get_alert_data("blank-date2", b"** Alert 1.1: mail - g\n \n", 0);
    diff_get_alert_data("only-nl", b"\n", 0);
    diff_get_alert_data("empty", b"", 0);
}

/// Row 21 — `atoi` numeric shapes for rule / level / srcport / dstport.
#[test]
fn row21_numeric_shapes() {
    let nums = [
        "0",
        "1",
        "-1",
        "+7",
        "007",
        "2147483647",
        "2147483648",
        "4294967295",
        "4294967296",
        "-2147483648",
        "-2147483649",
        "99999999999999999999",
        "abc",
        "",
        " 42",
        "12abc",
        "0x1F",
        "-0",
    ];
    for n in nums {
        let mut a = AlertSpec::minimal();
        a.rule = Some(format!("Rule: {n} (level {n}) -> 'c'"));
        a.srcport = Some(format!("Src Port: {n}"));
        a.dstport = Some(format!("Dst Port: {n}"));
        diff_get_alert_data(&format!("num-{n}"), a.render().as_bytes(), 0);
    }

    // Randomized numerics.
    let mut rng = Rng::new(0xB0B0_0021);
    for _ in 0..120 {
        let r = rng.i32();
        let l = rng.i32();
        let sp = rng.i32();
        let dp = rng.i32();
        let mut a = AlertSpec::minimal();
        a.rule = Some(format!("Rule: {r} (level {l}) -> 'c{r}'"));
        a.srcport = Some(format!("Src Port: {sp}"));
        a.dstport = Some(format!("Dst Port: {dp}"));
        diff_get_alert_data("num-rand", a.render().as_bytes(), 0);
    }
}

/// Row 22 — fully randomized alert streams (property test, fixed seed).
#[test]
fn row22_random_streams() {
    let mut rng = Rng::new(0xB0B0_0022);

    let kinds = ["mail", "exec", "mailx", "-", "noop"];
    let groups = [
        "syslog",
        "syscheck",
        "ossec,syscheck,",
        "pci_dss_10.2.4,gdpr",
        "syscheck-but-not-really",
        "",
    ];

    for iter in 0..500 {
        let n = 1 + rng.below(4);
        let mut text = String::new();
        for i in 0..n {
            let mut a = AlertSpec::minimal();
            a.id = format!("{}.{}", 1500000000u64 + rng.below(9999) as u64, rng.below(9999));
            a.kind = (*rng.choice(&kinds)).into();
            a.group = (*rng.choice(&groups)).into();
            a.date = format!(
                "20{:02} {} {:02} {:02}:{:02}:{:02}",
                rng.below(100),
                rng.choice(&["Jan", "Feb", "Mar", "Dec"]),
                rng.below(32),
                rng.below(24),
                rng.below(60),
                rng.below(60)
            );
            a.location = format!("/var/log/{}", String::from_utf8_lossy(&rng.token_upto(13)).replace([' ', '\t'], "_"));
            if rng.bool() {
                a.rule = Some(format!(
                    "Rule: {} (level {}) -> '{}'",
                    rng.i32(),
                    rng.below(17),
                    String::from_utf8_lossy(&rng.token_upto(20)).replace('\'', "q")
                ));
            }
            if rng.bool() {
                a.srcip = Some(format!("Src IP: {}", String::from_utf8_lossy(&rng.token_upto(18))));
            }
            if rng.bool() {
                a.srcport = Some(format!("Src Port: {}", rng.i32()));
            }
            if rng.bool() {
                a.dstip = Some(format!("Dst IP: {}", String::from_utf8_lossy(&rng.token_upto(18))));
            }
            if rng.bool() {
                a.dstport = Some(format!("Dst Port: {}", rng.i32()));
            }
            if rng.bool() {
                a.user = Some(format!("User: {}", String::from_utf8_lossy(&rng.token_upto(12))));
            }
            for _ in 0..rng.below(4) {
                let mut line = String::from_utf8_lossy(&rng.token_upto(60)).into_owned();
                if rng.below(6) == 0 {
                    line = format!("Integrity checksum changed for: '{line}'");
                }
                a.body.push(line);
            }
            text.push_str(&a.render());
            let _ = i;
        }
        if rng.below(4) == 0 {
            // Drop the final newline.
            while text.ends_with('\n') {
                text.pop();
            }
        }
        let flag = *rng.choice(&[0, CRALERT_MAIL_SET, ALL_FLAGS, -1, CRALERT_READ_ALL]);
        diff_get_alert_data(&format!("rand{iter}"), text.as_bytes(), flag);
    }
}

/// Row 23 — stream pre-positioned mid-file before the call.
#[test]
fn row23_preseeked_stream() {
    let p = libs();
    let specs: Vec<AlertSpec> = (0..4)
        .map(|i| {
            let mut a = AlertSpec::minimal();
            a.id = format!("1.{i}");
            a.location = format!("/l{i}");
            a
        })
        .collect();
    let text = render_all(&specs);
    let sc = Scratch::new("preseek");
    let path = sc.write("alerts.log", text.as_bytes());

    let len = text.len() as i64;
    let mut offsets: Vec<i64> = vec![0, 1, 5, 8, 9, len / 2, len - 1, len, len + 10];
    let mut rng = Rng::new(0xB0B0_0023);
    for _ in 0..60 {
        offsets.push(rng.below(text.len() + 8) as i64);
    }

    for off in offsets {
        unsafe {
            let fc = open_ro(&path);
            fseek(fc, off, SEEK_SET);
            let c = drain(&p.c, 0, fc);
            fclose(fc);

            let fr = open_ro(&path);
            fseek(fr, off, SEEK_SET);
            let r = drain(&p.rs, 0, fr);
            fclose(fr);

            assert!(
                c == r,
                "DIVERGENCE in GetAlertData at offset {off}\n  C   : {c:#?}\n  Rust: {r:#?}"
            );
        }
    }
}

/// Row 24 — non-seekable pipe carrying one complete alert. The alert ends at
/// EOF, so the `fseek` push-back is never reached and both must succeed.
#[test]
fn row24_pipe_single_alert() {
    let p = libs();
    let mut a = AlertSpec::minimal();
    a.srcip = Some("Src IP: 9.9.9.9".into());
    a.user = Some("User: pipe".into());
    let text = a.render();

    unsafe {
        let pc = PipeFile::new(text.as_bytes());
        let cres = snap_and_free(&p.c, (p.c.GetAlertData)(0, pc.fp));

        let pr = PipeFile::new(text.as_bytes());
        let rres = snap_and_free(&p.rs, (p.rs.GetAlertData)(0, pr.fp));

        assert_same!("GetAlertData on pipe", cres.clone(), rres);
        assert!(matches!(cres, AlertSnap::Some { .. }));
    }
}
