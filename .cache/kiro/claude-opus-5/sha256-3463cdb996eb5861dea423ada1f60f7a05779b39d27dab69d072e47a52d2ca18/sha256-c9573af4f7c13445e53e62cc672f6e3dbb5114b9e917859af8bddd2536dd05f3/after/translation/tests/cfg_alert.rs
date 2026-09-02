//! CONFIGS.md rows C15–C28 — `GetAlertData`, the lowest-level parsing entry
//! point, under every configuration the C branches on.

mod common;

use common::*;
use std::ffi::c_int;

const MAIL: c_int = 0x001;

// ---------------------------------------------------------------------------
// C15 / C16 — hand-built canonical shapes
// ---------------------------------------------------------------------------

/// C15 — minimal well-formed alert. Also pins the exact expected field values,
/// so the test would catch both libraries drifting together.
#[test]
fn c15_minimal_alert() {
    assert_gad_eq(0, MINIMAL.as_bytes(), 0, "C15 minimal");
    let (c, _) = libs();
    let a = gad_on_file(c, 0, MINIMAL.as_bytes(), 0)
        .alert
        .expect("must parse");
    assert_eq!(a.rule, 1002);
    assert_eq!(a.level, 7);
    assert_eq!(a.alertid.as_deref(), Some(&b"1461102540.1234"[..]));
    assert_eq!(a.date.as_deref(), Some(&b"2016 Apr 19 20:29:00"[..]));
    assert_eq!(a.location.as_deref(), Some(&b"myhost->/var/log/messages"[..]));
    assert_eq!(
        a.comment.as_deref(),
        Some(&b"Unknown problem somewhere in the system."[..])
    );
    assert_eq!(a.group.as_deref(), Some(&b"syslog,errors,"[..]));
    assert_eq!(a.srcip, None);
    assert_eq!(a.srcport, 0);
    assert_eq!(a.dstip, None);
    assert_eq!(a.dstport, 0);
    assert_eq!(a.user, None);
    assert_eq!(a.filename, None);
}

/// C16 — every recognised field prefix in one alert.
#[test]
fn c16_all_fields() {
    let content = concat!(
        "** Alert 1461102540.1234: mail - syslog,authentication_failed,\n",
        "2016 Apr 19 20:29:00 myhost->/var/log/auth.log\n",
        "Rule: 5716 (level 5) -> 'SSHD authentication failed.'\n",
        "Src IP: 192.168.1.100\n",
        "Src Port: 54321\n",
        "Dst IP: 10.20.30.40\n",
        "Dst Port: 22\n",
        "User: admin\n",
        "Apr 19 20:28:59 myhost sshd[1234]: Failed password for admin\n",
        "another free-form log line\n",
    );
    assert_gad_eq(0, content.as_bytes(), 0, "C16 all fields");
    assert_gad_eq(MAIL, content.as_bytes(), 0, "C16 all fields, MAIL_SET");
    let (c, _) = libs();
    let a = gad_on_file(c, 0, content.as_bytes(), 0).alert.unwrap();
    assert_eq!(a.rule, 5716);
    assert_eq!(a.level, 5);
    assert_eq!(a.srcip.as_deref(), Some(&b"192.168.1.100"[..]));
    assert_eq!(a.srcport, 54321);
    assert_eq!(a.dstip.as_deref(), Some(&b"10.20.30.40"[..]));
    assert_eq!(a.dstport, 22);
    assert_eq!(a.user.as_deref(), Some(&b"admin"[..]));
    assert_eq!(
        a.comment.as_deref(),
        Some(&b"SSHD authentication failed."[..])
    );
}

// ---------------------------------------------------------------------------
// C17 — randomized field order / presence / duplicates
// ---------------------------------------------------------------------------

const FIELD_LINES: &[&str] = &[
    "Rule: 5716 (level 5) -> 'SSHD authentication failed.'",
    "Rule: 1002 (level 7) -> 'Unknown problem.'",
    "Rule: 0 (level 0) -> ''",
    "Src IP: 192.168.1.100",
    "Src IP: ",
    "Src Port: 54321",
    "Src Port: 0",
    "Dst IP: 10.20.30.40",
    "Dst Port: 22",
    "User: admin",
    "User: ",
    "a free-form log line",
    "",
    "Integrity checksum changed for: '/etc/passwd'",
    "Old md5sum was: 0123456789abcdef0123456789abcdef",
    "New sha256sum is : deadbeef",
    "Size changed from '10' to '20'",
    "Permissions changed from 'rw-r--r--' to 'rw-------'",
];

/// C17 — the field prefixes in randomized order, with randomized presence and
/// duplicates (each duplicate `os_free`s and replaces the previous value).
#[test]
fn c17_random_field_orders() {
    let mut rng = Rng::new(0xC17);
    for n in 0..500 {
        let nlines = rng.below(9);
        let mut body: Vec<&str> = Vec::new();
        for _ in 0..nlines {
            body.push(rng.pick(FIELD_LINES));
        }
        let content = alert_block(
            "1461102540.1234",
            "syslog,errors,",
            "2016 Apr 19 20:29:00 myhost->/var/log/messages",
            &body,
        );
        let flag = if rng.bool() { MAIL } else { 0 };
        assert_gad_eq(flag, &content, 0, &format!("C17 random order #{n}"));
    }
}

// ---------------------------------------------------------------------------
// C18 — atoi() input shapes for rule / level / srcport / dstport
// ---------------------------------------------------------------------------

const NUMS: &[&str] = &[
    "",
    " ",
    "0",
    "1",
    "-1",
    "+7",
    "  42",
    "\t9",
    "007",
    "2147483647",
    "2147483648",
    "4294967295",
    "4294967296",
    "-2147483648",
    "-2147483649",
    "999999999999999999999",
    "0x1F",
    "12abc",
    "abc12",
    "3.9",
    "1e3",
    "-",
    "+",
    "--5",
];

/// C18 — `atoi` shapes in every numeric position.
#[test]
fn c18_atoi_shapes() {
    // rule + level come from the same "Rule: " line.
    for a in NUMS {
        for b in NUMS {
            let content = format!(
                "** Alert 1.2: mail - g,\n\
                 2016 Apr 19 20:29:00 h->/l\n\
                 Rule: {a} (level {b}) -> 'c'\n"
            );
            assert_gad_eq(
                0,
                content.as_bytes(),
                0,
                &format!("C18 rule={a:?} level={b:?}"),
            );
        }
    }
    // Ports.
    for n in NUMS {
        let content = format!(
            "** Alert 1.2: mail - g,\n\
             2016 Apr 19 20:29:00 h->/l\n\
             Rule: 1 (level 2) -> 'c'\n\
             Src Port: {n}\n\
             Dst Port: {n}\n"
        );
        assert_gad_eq(0, content.as_bytes(), 0, &format!("C18 port={n:?}"));
    }
    // Randomized digit soup.
    let mut rng = Rng::new(0xC18);
    for i in 0..300 {
        let mut s = String::new();
        let n = 1 + rng.below(24);
        for _ in 0..n {
            s.push(*rng.pick(&['0', '1', '5', '9', '-', '+', ' ', 'x', '.']));
        }
        let content = format!(
            "** Alert 1.2: mail - g,\n\
             2016 Apr 19 20:29:00 h->/l\n\
             Rule: {s} (level {s}) -> 'c'\n\
             Src Port: {s}\n\
             Dst Port: {s}\n"
        );
        assert_gad_eq(0, content.as_bytes(), 0, &format!("C18 random#{i} {s:?}"));
    }
}

// ---------------------------------------------------------------------------
// C19 — alertid extraction shapes
// ---------------------------------------------------------------------------

/// C19 — `strstr(p, ":")` at every interesting offset, including offset 0
/// (`z == 0` ⇒ empty alertid) and a trailing colon.
#[test]
fn c19_alertid_shapes() {
    let heads: &[&str] = &[
        "** Alert : mail - g,",         // colon at offset 0 => z == 0
        "** Alert :mail - g,",          // colon at offset 0, no space before mail
        "** Alert 1: mail - g,",
        "** Alert 1461102540.1234: mail - g,",
        "** Alert a:b:c: mail - g,",    // first colon wins
        "** Alert xxxxx: mail - g,",
        "** Alert  : mail - g,",        // space then colon
        "** Alert 1.2:: mail - g,",
        "** Alert 1.2 3: mail - g,",    // space BEFORE the colon
    ];
    for h in heads {
        let content = format!("{h}\n2016 Apr 19 20:29:00 h->/l\nRule: 1 (level 2) -> 'c'\n");
        assert_gad_eq(0, content.as_bytes(), 0, &format!("C19 {h:?}"));
        assert_gad_eq(MAIL, content.as_bytes(), 0, &format!("C19 {h:?} mail"));
    }

    // Randomized headers: keep them >= 9 bytes and newline-terminated so the
    // C never reads past the fgets-written region (see ERRORS.md note on the
    // 8-byte "** Alert" at EOF).
    let mut rng = Rng::new(0xC19);
    for i in 0..400 {
        let tail: Vec<u8> = rng
            .token(40)
            .into_iter()
            .filter(|&b| b != 0 && b != b'\n')
            .collect();
        let mut content = b"** Alert".to_vec();
        content.extend_from_slice(&tail);
        content.push(b'\n');
        content.extend_from_slice(b"2016 Apr 19 20:29:00 h->/l\nRule: 1 (level 2) -> 'c'\n");
        let flag = if rng.bool() { MAIL } else { 0 };
        assert_gad_eq(flag, &content, 0, &format!("C19 random header #{i}"));
    }
}

// ---------------------------------------------------------------------------
// C20 / C21 / C22 — group parsing and the syscheck sub-mode
// ---------------------------------------------------------------------------

/// C20 — group extraction: `-` present/absent, variable leading spaces,
/// `syscheck` as a substring or not, newline-terminated or at EOF.
#[test]
fn c20_group_shapes() {
    let heads: &[&str] = &[
        "** Alert 1.2: mail - g,",
        "** Alert 1.2: mail -g,",
        "** Alert 1.2: mail -    g,",
        "** Alert 1.2: mail -",
        "** Alert 1.2: mail - ",
        "** Alert 1.2: mail no dash here",
        "** Alert 1.2: mail - syscheck,",
        "** Alert 1.2: mail - ossec,syscheck,pci_dss_11.5,",
        "** Alert 1.2: mail - not_syscheck_either",
        "** Alert 1.2: mail - SYSCHECK,",
        "** Alert 1.2: mail - a-b-c-d",
        "** Alert 1.2: mail - -leading-dash",
    ];
    for h in heads {
        // newline-terminated
        let content = format!("{h}\n2016 Apr 19 20:29:00 h->/l\nRule: 1 (level 2) -> 'c'\n");
        assert_gad_eq(0, content.as_bytes(), 0, &format!("C20 {h:?}"));
        assert_gad_eq(MAIL, content.as_bytes(), 0, &format!("C20 {h:?} mail"));
        // header at EOF (>= 9 bytes so the read stays inside the buffer)
        assert_gad_eq(0, h.as_bytes(), 0, &format!("C20 {h:?} at EOF"));
    }
}

/// C21 — `syscheck` group plus the integrity-checksum line: `filename` is set
/// from `str + 33` and its last byte is stripped.
#[test]
fn c21_syscheck_filename() {
    let paths: &[&str] = &[
        "/etc/passwd",
        "/a",
        "ab",
        "/very/long/path/with/many/components/and/a/file.conf",
        "/path with spaces/x",
        "/path'with'quotes'",
        "/tmp/x'",
    ];
    for p in paths {
        let content = format!(
            "** Alert 1.2: mail - ossec,syscheck,\n\
             2016 Apr 19 20:29:00 h->/l\n\
             Rule: 550 (level 7) -> 'Integrity checksum changed.'\n\
             Integrity checksum changed for: '{p}'\n"
        );
        assert_gad_eq(0, content.as_bytes(), 0, &format!("C21 {p:?}"));
        let (c, _) = libs();
        let a = gad_on_file(c, 0, content.as_bytes(), 0).alert.unwrap();
        // strdup(str+33) then filename[strlen-1] = '\0' strips the final char.
        let expect = format!("{p}'");
        let expect = &expect[..expect.len() - 1];
        assert_eq!(
            a.filename.as_deref(),
            Some(expect.as_bytes()),
            "C21 {p:?}: unexpected filename"
        );
    }

    // Randomized paths (always at least one byte after the prefix, so the
    // `filename[strlen-1]` write never goes negative).
    let mut rng = Rng::new(0xC21);
    for i in 0..300 {
        let tail: Vec<u8> = rng
            .token(60)
            .into_iter()
            .filter(|&b| b != 0 && b != b'\n')
            .collect();
        let mut content =
            b"** Alert 1.2: mail - ossec,syscheck,\n2016 Apr 19 20:29:00 h->/l\n".to_vec();
        content.extend_from_slice(b"Integrity checksum changed for: '");
        content.extend_from_slice(&tail);
        content.push(b'\n');
        assert_gad_eq(0, &content, 0, &format!("C21 random path #{i}"));
    }
}

/// C22 — `issyscheck` is a ONE-SHOT flag: it is cleared by the first log line
/// regardless of whether that line matched, so a later integrity line is
/// ignored.
#[test]
fn c22_syscheck_one_shot() {
    let content = concat!(
        "** Alert 1.2: mail - ossec,syscheck,\n",
        "2016 Apr 19 20:29:00 h->/l\n",
        "some unrelated log line first\n",
        "Integrity checksum changed for: '/etc/shadow'\n",
    );
    assert_gad_eq(0, content.as_bytes(), 0, "C22 syscheck one-shot");
    let (c, _) = libs();
    let a = gad_on_file(c, 0, content.as_bytes(), 0).alert.unwrap();
    assert_eq!(
        a.filename, None,
        "C22: issyscheck was already cleared by the first log line"
    );

    // Recognised field prefixes do NOT clear it (they take other branches).
    let content2 = concat!(
        "** Alert 1.2: mail - ossec,syscheck,\n",
        "2016 Apr 19 20:29:00 h->/l\n",
        "Rule: 550 (level 7) -> 'x'\n",
        "Src IP: 1.2.3.4\n",
        "User: root\n",
        "Integrity checksum changed for: '/etc/shadow'\n",
    );
    assert_gad_eq(0, content2.as_bytes(), 0, "C22 prefixes do not clear it");
    let a2 = gad_on_file(c, 0, content2.as_bytes(), 0).alert.unwrap();
    assert_eq!(
        a2.filename.as_deref(),
        Some(&b"/etc/shadow"[..]),
        "C22: field prefixes must not clear issyscheck"
    );

    // A group WITHOUT "syscheck" never sets it.
    let content3 = concat!(
        "** Alert 1.2: mail - syslog,\n",
        "2016 Apr 19 20:29:00 h->/l\n",
        "Integrity checksum changed for: '/etc/shadow'\n",
    );
    assert_gad_eq(0, content3.as_bytes(), 0, "C22 non-syscheck group");
    let a3 = gad_on_file(c, 0, content3.as_bytes(), 0).alert.unwrap();
    assert_eq!(a3.filename, None);
}

// ---------------------------------------------------------------------------
// C23 / C24 / C25 — the CRALERT_MAIL_SET axis
// ---------------------------------------------------------------------------

/// C23 — `CRALERT_MAIL_SET` accepts a header whose token is exactly `mail`.
#[test]
fn c23_mail_accepted() {
    for tok in ["mail", "mail ", "mailer", "mailbox", "mail-x"] {
        // strncmp(..., 4) only inspects the first four bytes, so anything
        // starting with "mail" is accepted.
        let content = format!(
            "** Alert 1.2: {tok} - g,\n2016 Apr 19 20:29:00 h->/l\nRule: 1 (level 2) -> 'c'\n"
        );
        assert_gad_eq(MAIL, content.as_bytes(), 0, &format!("C23 {tok:?}"));
        let (c, _) = libs();
        assert!(
            gad_on_file(c, MAIL, content.as_bytes(), 0).alert.is_some(),
            "C23 {tok:?} must be accepted (only 4 bytes are compared)"
        );
    }
}

/// C24 — a file mixing `mail` and non-`mail` headers under `CRALERT_MAIL_SET`:
/// the skipped headers must leave the `_r` state machine exactly where the C
/// leaves it.
#[test]
fn c24_mail_mixed() {
    let content = concat!(
        "** Alert 1.1: nomail - g1,\n",
        "2016 Apr 19 20:29:01 h->/l1\n",
        "Rule: 1 (level 1) -> 'one'\n",
        "** Alert 2.2: mail - g2,\n",
        "2016 Apr 19 20:29:02 h->/l2\n",
        "Rule: 2 (level 2) -> 'two'\n",
        "** Alert 3.3: nomail - g3,\n",
        "2016 Apr 19 20:29:03 h->/l3\n",
        "Rule: 3 (level 3) -> 'three'\n",
        "** Alert 4.4: mail - g4,\n",
        "2016 Apr 19 20:29:04 h->/l4\n",
        "Rule: 4 (level 4) -> 'four'\n",
    );
    assert_gad_eq(MAIL, content.as_bytes(), 0, "C24 mixed, single call");
    assert_gad_eq(0, content.as_bytes(), 0, "C24 mixed, no flag");
    assert_drain_eq(MAIL, content.as_bytes(), 12, "C24 mixed, full drain");
    assert_drain_eq(0, content.as_bytes(), 12, "C24 mixed, full drain no flag");
}

/// C25 — every flag value in `0x00..0x1F` against six representative shapes.
#[test]
fn c25_flag_cross_product() {
    let shapes: Vec<Vec<u8>> = vec![
        MINIMAL.as_bytes().to_vec(),
        alert_block(
            "1.2",
            "ossec,syscheck,",
            "2016 Apr 19 20:29:00 h->/l",
            &[
                "Rule: 550 (level 7) -> 'Integrity checksum changed.'",
                "Integrity checksum changed for: '/etc/passwd'",
            ],
        ),
        b"** Alert 1.2: notmail - g,\n2016 Apr 19 20:29:00 h->/l\nRule: 1 (level 2) -> 'c'\n"
            .to_vec(),
        b"".to_vec(),
        b"garbage only\n".to_vec(),
        {
            let mut v = MINIMAL.as_bytes().to_vec();
            v.extend_from_slice(MINIMAL.as_bytes());
            v.extend_from_slice(MINIMAL.as_bytes());
            v
        },
    ];
    for flag in 0..32 {
        for (i, s) in shapes.iter().enumerate() {
            assert_gad_eq(flag, s, 0, &format!("C25 flag={flag:#x} shape#{i}"));
            assert_drain_eq(flag, s, 8, &format!("C25 drain flag={flag:#x} shape#{i}"));
        }
    }
}

// ---------------------------------------------------------------------------
// C26 / C27 — stream positioning
// ---------------------------------------------------------------------------

/// C26 — repeated calls on one `FILE*` until NULL, over randomized multi-alert
/// files: exercises the whole `fseek`-back sequence.
#[test]
fn c26_sequential_drain() {
    let mut rng = Rng::new(0xC26);
    for n in 0..200 {
        let nalerts = 1 + rng.below(5);
        let mut content = Vec::new();
        if rng.bool() {
            content.extend_from_slice(b"leading noise line\n");
        }
        for k in 0..nalerts {
            let group = *rng.pick(&["syslog,", "ossec,syscheck,", "authentication_failed,", ""]);
            let nbody = rng.below(5);
            let body: Vec<&str> = (0..nbody).map(|_| *rng.pick(FIELD_LINES)).collect();
            content.extend_from_slice(&alert_block(
                &format!("146110254{k}.{n}"),
                group,
                &format!("2016 Apr 19 20:29:0{} h{k}->/var/log/m{k}", k % 10),
                &body,
            ));
        }
        let flag = if rng.bool() { MAIL } else { 0 };
        assert_drain_eq(flag, &content, 16, &format!("C26 drain #{n}"));
    }
}

/// C27 — start the stream at a randomized (possibly mid-line) offset.
#[test]
fn c27_random_start_offset() {
    let mut content = Vec::new();
    for k in 0..4 {
        content.extend_from_slice(&alert_block(
            &format!("146110254{k}.9"),
            "ossec,syscheck,",
            "2016 Apr 19 20:29:00 h->/var/log/messages",
            &[
                "Rule: 550 (level 7) -> 'Integrity checksum changed.'",
                "Integrity checksum changed for: '/etc/passwd'",
                "Src IP: 1.2.3.4",
                "Src Port: 99",
            ],
        ));
    }
    let mut rng = Rng::new(0xC27);
    // every offset, exhaustively, plus a few past EOF
    for off in 0..content.len() {
        assert_gad_eq(0, &content, off as i64, &format!("C27 offset {off}"));
    }
    for _ in 0..40 {
        let off = content.len() + rng.below(64);
        assert_gad_eq(0, &content, off as i64, &format!("C27 past EOF {off}"));
    }
}

// ---------------------------------------------------------------------------
// C28 — fuzz corpus
// ---------------------------------------------------------------------------

/// C28 — fully randomized corpus drawn from a weighted alphabet of every
/// recognised prefix, malformed variants, blank lines and random bytes.
#[test]
fn c28_fuzz_corpus() {
    const ALPHABET: &[&str] = &[
        "** Alert 1461102540.1234: mail - syslog,errors,",
        "** Alert 1461102540.1234: nomail - ossec,syscheck,",
        "** Alert 1.2: mail -",
        "** Alert nocolon here",
        "** Alert 1.2:nospace",
        "** Alert",
        "2016 Apr 19 20:29:00 myhost->/var/log/messages",
        "2016:04",
        "no colon at all",
        "Rule: 5716 (level 5) -> 'SSHD authentication failed.'",
        "Rule: 1002",
        "Rule: 1002 a b",
        "Rule: 1002 a b 'unclosed",
        "Src IP: 192.168.1.100",
        "Src Port: 54321",
        "Dst IP: 10.20.30.40",
        "Dst Port: 22",
        "User: admin",
        "Integrity checksum changed for: '/etc/passwd'",
        "Integrity checksum changed for: 'x'",
        "Old md5sum was: abc",
        "New sha256sum is : def",
        "Size changed from '1' to '2'",
        "Ownership was 'root'",
        "Group ownership was 'wheel'",
        "Permissions changed from 'a' to 'b'",
        "",
        "   ",
        "\t\t",
        "*",
        "**",
        "** Aler",
    ];

    let mut rng = Rng::new(0xC28);
    for n in 0..400 {
        let nlines = rng.below(24);
        let mut content = Vec::new();
        for _ in 0..nlines {
            if rng.below(10) == 0 {
                // random bytes (no NUL, no newline)
                let t: Vec<u8> = rng
                    .token(50)
                    .into_iter()
                    .filter(|&b| b != 0 && b != b'\n')
                    .collect();
                content.extend_from_slice(&t);
            } else {
                content.extend_from_slice(rng.pick(ALPHABET).as_bytes());
            }
            content.push(b'\n');
        }
        // Optionally drop the final newline, but only if the last line is not
        // exactly 8 bytes long (see ERRORS.md UB note E29).
        if rng.bool() && !content.is_empty() {
            let last_nl = content[..content.len() - 1]
                .iter()
                .rposition(|&b| b == b'\n')
                .map(|i| i + 1)
                .unwrap_or(0);
            if content.len() - 1 - last_nl != 8 {
                content.pop();
            }
        }
        let flag = rng.i32();
        assert_gad_eq(flag, &content, 0, &format!("C28 fuzz #{n} flag={flag}"));
        assert_drain_eq(flag, &content, 12, &format!("C28 fuzz drain #{n}"));
    }
}
